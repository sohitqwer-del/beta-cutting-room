//! Adapter checks for v0.2 rules, kept independent of the search heuristic.
use super::{bundles, safety};
use anyhow::{Context, Result, ensure};
use beta_marker_core::geometry::{
    angle_diff_deg, area, bbox, grain_alignment_deg, norm_deg, transform_points,
};
use beta_marker_core::{Job, Layout, MarkerResult, Nap, Piece, Placement, Pt};
use geo::{Distance, Euclidean, Relate};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn outline(piece: &Piece, pl: &Placement) -> Result<Vec<Pt>> {
    let points = beta_marker_core::rules::cut_outline(piece, pl);
    ensure!(
        points.iter().flatten().all(|v| v.is_finite())
            && area(&points).is_finite()
            && area(&points) > 0.0,
        "placed cut outline loses coordinate precision for '{}'",
        piece.id
    );
    if pl.on_fold {
        // Clipping through an existing vertex can repeat that vertex. A fold
        // must still produce one simple, non-empty physical half.
        safety::clean_outline(&points)
            .with_context(|| format!("invalid fold half for '{}'", piece.id))
    } else {
        Ok(points)
    }
}

pub(super) fn conflict(a: &[Pt], b: &[Pt], gap: f64) -> bool {
    let Some((ab, bb)) = bbox(a).zip(bbox(b)) else {
        return true;
    };
    if ab[0] > bb[2] + gap || bb[0] > ab[2] + gap || ab[1] > bb[3] + gap || bb[1] > ab[3] + gap {
        return false;
    }
    let a = safety::polygon(a);
    let b = safety::polygon(b);
    let relation = a.relate(&b);
    // Interior/interior intersection catches containment and coincident shapes
    // as well as proper crossings. Boundary contact is legal at zero gap.
    relation.matches("T********").unwrap_or(true)
        || (gap > 0.0 && Euclidean.distance(&a, &b) + 1e-8 < gap)
}

pub(super) fn direction(piece: &Piece, pl: &Placement) -> Result<f64> {
    let relative = norm_deg(pl.rotation_deg - grain_alignment_deg(piece.grain, pl.mirrored));
    if angle_diff_deg(relative, 0.0) <= angle_diff_deg(relative, 180.0) {
        Ok(0.0)
    } else {
        Ok(180.0)
    }
}

pub(super) fn bounds_ok(job: &Job, piece: &Piece, pl: &Placement, points: &[Pt]) -> bool {
    let Some(b) = bbox(points) else { return false };
    let edge = job.spacing.edge_gap + piece.buffer;
    let lower = if job.fabric.layout == Layout::Tubular {
        0.0
    } else {
        edge
    };
    let upper = if job.fabric.layout == Layout::Open {
        job.fabric.width - edge
    } else {
        job.fabric.width
    };
    b.iter().all(|v| v.is_finite())
        && b[0] >= edge - 1e-7
        && b[1] >= lower - 1e-7
        && b[3] <= upper + 1e-7
        && (!pl.on_fold || fold_edge(job, piece, pl).is_ok())
}

fn fold_edge(job: &Job, piece: &Piece, pl: &Placement) -> Result<()> {
    ensure!(
        job.fabric.layout != Layout::Open,
        "on-fold placement requires folded or tubular fabric"
    );
    let line = piece
        .fold_line
        .context("on-fold placement needs a fold line")?;
    let points = transform_points(&line, pl.mirrored, pl.rotation_deg, pl.translate);
    let on = |y: f64| points.iter().all(|p| (p[1] - y).abs() <= 1e-6);
    ensure!(
        on(job.fabric.width) || (job.fabric.layout == Layout::Tubular && on(0.0)),
        "fold line for '{}' must lie exactly on a fold edge",
        piece.id
    );
    Ok(())
}

pub(super) fn geometry(job: &Job, placements: &[Placement]) -> Result<()> {
    let mut shapes: Vec<(Vec<Pt>, f64)> = Vec::new();
    let mut bundle_directions = BTreeMap::new();
    let mut copies = BTreeSet::new();
    for pl in placements {
        let piece = job
            .pieces
            .iter()
            .find(|p| p.id == pl.piece_id)
            .context("unknown placed piece")?;
        ensure!(
            pl.translate
                .iter()
                .chain(std::iter::once(&pl.rotation_deg))
                .all(|v| v.is_finite()),
            "non-finite placement for '{}'",
            piece.id
        );
        ensure!(
            copies.insert((&pl.piece_id, pl.mirrored, pl.copy)),
            "duplicate copy for '{}'",
            piece.id
        );
        ensure!(
            beta_marker_core::allowed_rotations(job, piece, pl.mirrored).allows(pl.rotation_deg),
            "grain rule violated for '{}'",
            piece.id
        );
        if pl.on_fold {
            fold_edge(job, piece, pl)?;
        }
        let points = outline(piece, pl)?;
        ensure!(
            bounds_ok(job, piece, pl, &points),
            "placement outside fabric or edge clearance for '{}'",
            piece.id
        );
        if job.fabric.nap == Nap::OneWayPerBundle {
            let relative =
                norm_deg(pl.rotation_deg - grain_alignment_deg(piece.grain, pl.mirrored));
            let dir = direction(piece, pl)?;
            ensure!(
                angle_diff_deg(relative, dir)
                    <= piece.tilt_deg.unwrap_or(job.fabric.tilt_deg) + 1e-3,
                "bundle grain must be 0 or 180 degrees for '{}'",
                piece.id
            );
            let bundle = pl
                .bundle
                .context("one-way-per-bundle placement needs a bundle")?;
            if let Some(previous) = bundle_directions.insert(bundle, dir) {
                ensure!(
                    previous == dir,
                    "bundle {bundle} has inconsistent directions"
                );
            }
        }
        for zone in &job.fabric.defects {
            ensure!(
                !conflict(&points, &zone.polygon, 0.0),
                "piece '{}' overlaps defect '{}'",
                piece.id,
                zone.id
            );
        }
        for (other, other_piece) in &shapes {
            let gap = job.spacing.piece_gap + piece.buffer + other_piece;
            ensure!(
                !conflict(&points, other, gap),
                "placed outlines overlap or violate the piece gap for '{}'",
                piece.id
            );
        }
        shapes.push((points, piece.buffer));
    }
    Ok(())
}

pub(super) fn complete(job: &Job, result: &MarkerResult) -> Result<()> {
    ensure!(
        result.width == job.fabric.width
            && [
                result.length,
                result.piece_area,
                result.efficiency,
                result.runtime_s
            ]
            .iter()
            .all(|v| v.is_finite()),
        "result width or metrics are invalid"
    );
    geometry(job, &result.placements)?;
    counts(job, &result.placements)?;
    let mut metrics = result.clone();
    refresh(job, &mut metrics)?;
    ensure!(
        job.fabric
            .max_length
            .is_none_or(|max| metrics.length <= max + 0.01),
        "too_long: marker exceeds max_length"
    );
    ensure!(
        (result.length - metrics.length).abs() <= 0.01,
        "result length differs from the cut outlines"
    );
    Ok(())
}

fn counts(job: &Job, placements: &[Placement]) -> Result<()> {
    for pl in placements {
        let piece = job
            .pieces
            .iter()
            .find(|p| p.id == pl.piece_id)
            .context("unknown placed piece")?;
        ensure!(
            pl.bundle.is_some_and(|b| bundles(job, piece).contains(&b)),
            "invalid or missing bundle for '{}'",
            piece.id
        );
        ensure!(
            beta_marker_core::rules::placement_bundles(pl).iter().all(|b| bundles(job, piece).contains(b)),
            "invalid layer bundle for '{}'", piece.id
        );
    }
    for piece in &job.pieces {
        for bundle in bundles(job, piece) {
            let mut copies = (0u64, 0u64);
            for pl in placements
                .iter()
                .filter(|p| p.piece_id == piece.id)
            {
                let layers = beta_marker_core::rules::bundle_layer_copies(job.fabric.layout, pl, bundle);
                copies.0 += u64::from(layers.0);
                copies.1 += u64::from(layers.1);
            }
            let explicit = placements.iter().any(|p| !p.layer_allocations.is_empty());
            if beta_marker_core::rules::flip_allowed(job, piece) {
                let total = copies.0 + copies.1;
                let expected = u64::from(piece.quantity) + u64::from(piece.mirror_quantity);
                ensure!(
                    total >= expected
                        && total <= expected + u64::from(job.fabric.layout != Layout::Open && !explicit),
                    "wrong faceless demand for '{}' bundle {bundle}",
                    piece.id
                );
            } else if job.fabric.layout == Layout::Open || explicit {
                ensure!(
                    copies == (u64::from(piece.quantity), u64::from(piece.mirror_quantity)),
                    "wrong demand for '{}' bundle {bundle}",
                    piece.id
                );
            } else {
                ensure!(
                    copies.0 >= u64::from(piece.quantity)
                        && copies.1 >= u64::from(piece.mirror_quantity),
                    "missing layer copies for '{}' bundle {bundle}",
                    piece.id
                );
                ensure!(
                    copies.0.max(copies.1) <= u64::from(piece.quantity.max(piece.mirror_quantity)),
                    "extra placements for '{}' bundle {bundle}",
                    piece.id
                );
            }
        }
    }
    Ok(())
}

pub(super) fn refresh(job: &Job, result: &mut MarkerResult) -> Result<()> {
    let mut length = 0.0_f64;
    let mut piece_area = 0.0;
    for pl in &result.placements {
        let piece = job
            .pieces
            .iter()
            .find(|p| p.id == pl.piece_id)
            .context("unknown placed piece")?;
        let points = outline(piece, pl)?;
        length = length.max(bbox(&points).context("empty placed outline")?[2]);
        piece_area += area(&beta_marker_core::rules::placed_outline(piece, pl));
    }
    result.length = length;
    result.piece_area = piece_area;
    result.efficiency = if length > 0.0 {
        piece_area / (job.fabric.width * length)
    } else {
        0.0
    };
    Ok(())
}

/// Keep the independent gate mandatory, but distinguish a known old-contract
/// rejection from search failure while the validator owner's patch is pending.
pub(super) fn independently_validate(job: &Job, result: &MarkerResult) -> Result<()> {
    let report = beta_marker_core::validate(job, result);
    if report.ok {
        return Ok(());
    }
    let effective = beta_marker_core::rules::effective_job(job);
    let has_override = effective.pieces.iter().any(|p| {
        p.flip
            .is_some_and(|flip| flip != effective.fabric.allow_flip)
            || (effective.fabric.nap == Nap::OneWayPerBundle
                && p.tilt_deg
                    .is_some_and(|tilt| tilt != effective.fabric.tilt_deg))
    });
    let legacy_codes = report.errors.iter().all(|e| {
        matches!(
            e.code.as_str(),
            "mirror" | "missing_piece" | "extra_piece" | "bundle" | "nap_bundle"
        )
    });
    if has_override && legacy_codes && complete(&effective, result).is_ok() {
        let mut diagnostic = effective;
        let original_tilt = diagnostic.fabric.tilt_deg;
        for p in &mut diagnostic.pieces {
            p.tilt_deg = Some(p.tilt_deg.unwrap_or(original_tilt));
        }
        diagnostic.fabric.tilt_deg = diagnostic
            .pieces
            .iter()
            .filter_map(|p| p.tilt_deg)
            .fold(original_tilt, f64::max);
        diagnostic.fabric.allow_flip = true;
        // This relaxed job is used only to classify the error. It never makes
        // a result eligible for return, progress or a benchmark score.
        if beta_marker_core::validate(&diagnostic, result).ok {
            anyhow::bail!(
                "pending_validator: per-piece rules pass adapter checks but the independent validator rejects them: {:?}",
                report.errors
            );
        }
    }
    anyhow::bail!("independent validation failed: {:?}", report.errors)
}
