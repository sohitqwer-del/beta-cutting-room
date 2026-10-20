//! CAD cleanup, engine precision guards and independent validation repair.
use anyhow::{Result, ensure};
use beta_marker_core::geometry::{bbox, place, signed_area};
use beta_marker_core::{Job, MarkerResult, Pt};
use geo::{Buffer, Covers, Polygon, Validation};
use std::collections::BTreeSet;

pub(super) fn polygon(points: &[Pt]) -> Polygon<f64> {
    Polygon::new(points.iter().map(|p| (p[0], p[1])).collect(), vec![])
}

pub(super) fn check_input(job: &Job) -> Result<()> {
    ensure!(
        job.fabric.width.is_finite() && job.fabric.width <= f64::from(f32::MAX),
        "fabric width must be finite and fit engine precision"
    );
    ensure!(
        job.fabric.tilt_deg.is_finite() && (0.0..=180.0).contains(&job.fabric.tilt_deg),
        "tilt must be finite and between 0 and 180 degrees"
    );
    ensure!(
        job.fabric
            .max_length
            .is_none_or(|v| v.is_finite() && v > 0.0),
        "max_length must be finite and positive"
    );
    ensure!(
        job.spacing.edge_gap.is_finite() && job.spacing.piece_gap.is_finite(),
        "gaps must be finite"
    );
    for piece in &job.pieces {
        ensure!(
            piece
                .tilt_deg
                .is_none_or(|tilt| tilt.is_finite() && (0.0..=180.0).contains(&tilt)),
            "piece tilt must be finite and between 0 and 180 degrees for '{}'",
            piece.id
        );
        ensure!(
            piece.buffer.is_finite(),
            "buffer must be finite for {}",
            piece.id
        );
        ensure!(
            piece
                .grain
                .is_none_or(|g| g.iter().flatten().all(|v| v.is_finite())),
            "grain coordinates must be finite for {}",
            piece.id
        );
        ensure!(
            piece
                .outline
                .iter()
                .flatten()
                .all(|v| (*v as f32).is_finite()),
            "outline exceeds engine precision for {}",
            piece.id
        );
        clean_outline(&piece.outline)
            .map_err(|e| anyhow::anyhow!("invalid outline for piece '{}': {e}", piece.id))?;
        let multiplier = beta_marker_core::rules::multiplier(job, piece);
        ensure!(
            piece.quantity.checked_mul(multiplier).is_some()
                && piece.mirror_quantity.checked_mul(multiplier).is_some(),
            "quantity overflows for {}",
            piece.id
        );
    }
    Ok(())
}

/// Remove redundant vertices without moving any cut-line segment. Backtracking
/// collinear edges are deliberately retained so validation rejects their spikes.
pub(super) fn clean_outline(points: &[Pt]) -> Result<Vec<Pt>> {
    let mut clean = points.to_vec();
    clean.dedup();
    if clean.len() > 1 && clean.first() == clean.last() {
        clean.pop();
    }
    loop {
        let n = clean.len();
        if n < 3 {
            break;
        }
        let redundant = (0..n).find(|&i| {
            let a = clean[(i + n - 1) % n];
            let b = clean[i];
            let c = clean[(i + 1) % n];
            let cross = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
            let dot = (b[0] - a[0]) * (b[0] - c[0]) + (b[1] - a[1]) * (b[1] - c[1]);
            cross == 0.0 && dot <= 0.0
        });
        if let Some(i) = redundant {
            clean.remove(i);
        } else {
            break;
        }
    }
    let poly = polygon(&clean);
    ensure!(
        poly.is_valid(),
        "outline must be a simple polygon (self-intersections and backtracking spikes are unsupported): {}",
        poly.validation_errors()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("; ")
    );
    ensure!(
        clean.len() >= 3 && signed_area(&clean).abs() > 0.0,
        "outline has no area after cleanup"
    );
    if signed_area(&clean) < 0.0 {
        clean.reverse();
    }
    Ok(clean)
}

/// Cover both input conversion and centring arithmetic, including zero-gap jobs.
/// Export/placement arithmetic is checked separately by the final validator.
pub(super) fn precision_guard(points: &[Pt]) -> f64 {
    let scale = points.iter().flatten().fold(1.0_f64, |s, v| s.max(v.abs()));
    0.002 + 32.0 * f64::from(f32::EPSILON) * scale
}

/// Test full polygon coverage, including edges and interiors, after all engine
/// offsets and simplifications. Checking vertices alone misses concave cutouts.
pub(super) fn collision_covers(
    item: &jagua_rs::entities::Item,
    points: &[Pt],
    radius: f64,
) -> bool {
    let pre = item.shape_orig().pre_transform.translation();
    let original = polygon(
        &points
            .iter()
            .map(|p| [p[0] + f64::from(pre.0), p[1] + f64::from(pre.1)])
            .collect::<Vec<_>>(),
    );
    let collision = Polygon::new(
        item.shape_cd()
            .vertices()
            .iter()
            .map(|p| (f64::from(p.0), f64::from(p.1)))
            .collect(),
        vec![],
    );
    if !collision.is_valid() {
        return false;
    }
    if radius == 0.0 {
        return collision.covers(&original);
    }
    // Default round-buffer chords lie inside a disk. Inflate their radius enough
    // to cover the requested disk; the engine has a larger independent guard.
    let required = original.buffer_with_style(
        geo::algorithm::buffer::BufferStyle::new(radius * 1.002)
            .line_join(geo::algorithm::buffer::LineJoin::Round(0.1)),
    );
    required.0.iter().all(|p| collision.covers(p))
}

pub(super) fn refresh_metrics(job: &Job, result: &mut MarkerResult) {
    let metrics = beta_marker_core::compute_metrics(job, &result.placements);
    result.length = metrics.length;
    result.piece_area = metrics.piece_area;
    result.efficiency = metrics.efficiency;
}

/// Try small lengthwise nudges first. If contacts persist, move only the
/// offending piece copies to disjoint boxes at the marker end. This bounded
/// fallback sacrifices density but never relaxes validation or changes rotation.
pub(super) fn repair(job: &Job, result: &mut MarkerResult) -> Result<()> {
    let pieces: std::collections::BTreeMap<_, _> =
        job.pieces.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut repaired = false;
    for attempt in 0..=4 {
        refresh_metrics(job, result);
        let report = beta_marker_core::validate(job, result);
        if report.ok {
            if repaired {
                result.warnings.push(
                    "validation_repair: separated engine contacts along the marker length".into(),
                );
            }
            result.warnings.extend(
                report
                    .warnings
                    .iter()
                    .map(|i| format!("{}: {}", i.code, i.message)),
            );
            return Ok(());
        }
        let errors = || {
            report
                .errors
                .iter()
                .map(|i| format!("{}: {}", i.code, i.message))
                .collect::<Vec<_>>()
                .join("; ")
        };
        ensure!(
            attempt < 4,
            "independent validation failed after repair: {}",
            errors()
        );
        ensure!(
            report.errors.iter().all(|i| matches!(
                i.code.as_str(),
                "overlap" | "gap" | "outside_width" | "outside_start"
            )),
            "independent validation failed: {}",
            errors()
        );
        let mut move_ids = BTreeSet::new();
        for issue in &report.errors {
            if matches!(issue.code.as_str(), "overlap" | "gap") {
                move_ids.insert(issue.piece_ids[1].as_str());
            }
        }
        let mut end = result.length;
        for pl in &mut result.placements {
            let piece = pieces[pl.piece_id.as_str()];
            let bounds = bbox(&place(&piece.outline, pl)).expect("checked outline");
            let edge = job.spacing.edge_gap + piece.buffer;
            ensure!(
                bounds[3] - bounds[1] <= job.fabric.width - 2.0 * edge,
                "piece '{}' cannot fit the usable width during repair",
                piece.id
            );
            pl.translate[1] +=
                (edge - bounds[1]).max(0.0) - (bounds[3] - (job.fabric.width - edge)).max(0.0);
            pl.translate[0] += (edge - bounds[0]).max(0.0);
            if move_ids.contains(pl.piece_id.as_str()) {
                if attempt < 3 {
                    pl.translate[0] += [0.01, 0.1, 1.0][attempt];
                } else {
                    let bounds = bbox(&place(&piece.outline, pl)).expect("checked outline");
                    let max_buffer = job.pieces.iter().map(|p| p.buffer).fold(0.0_f64, f64::max);
                    let gap = job.spacing.piece_gap + piece.buffer + max_buffer + 0.01;
                    pl.translate[0] += end + gap - bounds[0];
                    end += gap + bounds[2] - bounds[0];
                }
            }
        }
        repaired = true;
    }
    unreachable!("bounded repair loop returns on its final check")
}
