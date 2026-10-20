//! Independent checks of marker counts, rules, geometry and reported metrics.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::geometry::bbox;
use crate::metrics::compute_metrics;
use crate::model::{Issue, Job, Layout, MarkerResult, ValidationReport};
use crate::rules::{
    allowed_rotations, check_job, demands, effective_job, flip_allowed, layer_copies,
};

mod polygon;
#[cfg(test)]
mod tests;
mod v02;

const TOLERANCE: f64 = 0.01;

fn issue(code: &str, message: String, ids: &[&str]) -> Issue {
    Issue {
        code: code.into(),
        message,
        piece_ids: ids.iter().map(|s| (*s).into()).collect(),
    }
}

/// Compatibility alias. New callers should use `rules::cut_outline`.
pub use crate::rules::cut_outline;

/// Validate a marker independently of the nesting engine. Distances are in mm.
/// Boundary contact is allowed; positive interior intersection is always an error.
pub fn validate(job: &Job, result: &MarkerResult) -> ValidationReport {
    let effective = effective_job(job);
    let job = &effective;
    let m = compute_metrics(job, &result.placements);
    let mut report = ValidationReport {
        length: m.length,
        piece_area: m.piece_area,
        efficiency: m.efficiency,
        errors: check_job(job),
        ..ValidationReport::default()
    };
    let by_id: HashMap<_, _> = job.pieces.iter().map(|p| (p.id.as_str(), p)).collect();
    for p in &job.pieces {
        if p.block_margin.is_some_and(|v| !v.is_finite() || v < 0.0) {
            report.errors.push(issue(
                "block",
                format!(
                    "Piece '{}' needs a finite, non-negative block margin.",
                    p.id
                ),
                &[&p.id],
            ));
        }
    }
    let mut expected: BTreeMap<_, _> =
        demands(job)
            .into_iter()
            .fold(BTreeMap::new(), |mut counts, d| {
                *counts
                    .entry((
                        d.piece_id,
                        d.mirrored && !flip_allowed(job, &job.pieces[d.piece_index]),
                    ))
                    .or_insert(0) += u64::from(d.count);
                counts
            });
    let mut actual = BTreeMap::new();
    let mut copies = BTreeSet::new();
    let mut shapes = Vec::new();
    let explicit_layers = result.placements.iter().any(|p| !p.layer_allocations.is_empty());
    for pl in &result.placements {
        let Some(&p) = by_id.get(pl.piece_id.as_str()) else {
            report.errors.push(issue(
                "unknown_piece",
                format!("Piece '{}' is not in the job.", pl.piece_id),
                &[&pl.piece_id],
            ));
            continue;
        };
        if explicit_layers && job.fabric.layout != Layout::Open && pl.layer_allocations.is_empty() {
            report.errors.push(issue("layer_allocation",
                "Every layered placement must declare its assignments when explicit allocation is used.".into(), &[&p.id]));
        }
        v02::layers(job, p, pl, &mut report);
        let (normal, mirrored) = layer_copies(job.fabric.layout, pl);
        for (mirror, count) in [(false, normal), (true, mirrored)] {
            let key = (p.id.clone(), mirror && !flip_allowed(job, p));
            *actual.entry(key.clone()).or_insert(0_u64) += u64::from(count);
            expected.entry(key).or_insert(0);
        }
        if !copies.insert((&pl.piece_id, pl.mirrored, pl.copy)) {
            report.warnings.push(issue(
                "duplicate_copy",
                format!(
                    "Piece '{}' repeats copy {} (mirrored: {}).",
                    p.id, pl.copy, pl.mirrored
                ),
                &[&p.id],
            ));
        }
        if pl.mirrored && p.mirror_quantity == 0 && !flip_allowed(job, p)
            && !(pl.on_fold && p.fold_line.is_some() && job.fabric.layout != Layout::Open)
        {
            report.errors.push(issue(
                "mirror",
                format!("Piece '{}' has no permitted mirrored copies.", p.id),
                &[&p.id],
            ));
        }
        if !pl.rotation_deg.is_finite()
            || !allowed_rotations(job, p, pl.mirrored).allows(pl.rotation_deg)
        {
            report.errors.push(issue(
                "grain",
                format!(
                    "Piece '{}' has a rotation of {} degrees that its grain rule does not allow.",
                    p.id, pl.rotation_deg
                ),
                &[&p.id],
            ));
        }
        v02::fold(job, p, pl, &mut report);
        let points = cut_outline(p, pl);
        if points.len() < 3 || points.iter().flatten().any(|v| !v.is_finite()) {
            report.errors.push(issue(
                "bad_placement",
                format!(
                    "Piece '{}' has no usable placed outline or has non-finite coordinates.",
                    p.id
                ),
                &[&p.id],
            ));
            continue;
        }
        let edge = job.spacing.edge_gap + p.buffer;
        let low = if job.fabric.layout == Layout::Tubular {
            0.0
        } else {
            edge
        };
        let high = if job.fabric.layout == Layout::Open {
            edge
        } else {
            0.0
        };
        if points
            .iter()
            .any(|q| q[1] < low - TOLERANCE || q[1] > job.fabric.width - high + TOLERANCE)
        {
            report.errors.push(issue(
                "outside_width",
                format!(
                    "Piece '{}' extends outside the usable fabric width or edge clearance.",
                    p.id
                ),
                &[&p.id],
            ));
        }
        for zone in &job.fabric.defects {
            if polygon::relation(&points, &zone.polygon, 0.0).0 {
                report.errors.push(issue(
                    "defect",
                    format!("Piece '{}' overlaps defect '{}'.", p.id, zone.id),
                    &[&p.id],
                ));
            }
        }
        if points.iter().any(|q| q[0] < edge - TOLERANCE) {
            report.errors.push(issue(
                "outside_start",
                format!(
                    "Piece '{}' extends before the marker start clearance.",
                    p.id
                ),
                &[&p.id],
            ));
        }
        if let Some(bounds) = bbox(&points) {
            let clearance = p.buffer + job.spacing.piece_gap / 2.0;
            shapes.push((bounds, clearance, p.id.as_str(), points));
        }
    }
    for ((id, mirrored), count) in expected {
        let found = actual.get(&(id.clone(), mirrored)).copied().unwrap_or(0);
        if found != count {
            let code = if found < count {
                "missing_piece"
            } else {
                "extra_piece"
            };
            let finding = issue(
                code,
                format!("Piece '{id}' needs {count} copies (mirrored: {mirrored}); found {found}."),
                &[&id],
            );
            if found > count && job.fabric.layout != Layout::Open && !explicit_layers {
                report.warnings.push(Issue {
                    code: "surplus_copy".into(),
                    ..finding
                });
            } else {
                report.errors.push(finding);
            }
        }
    }
    // Sweep expanded boxes along the fabric length. The y rejection also avoids
    // polygon work for neighbours in separate rows of a long marker.
    shapes.sort_by(|a, b| (a.0[0] - a.1).total_cmp(&(b.0[0] - b.1)));
    for (i, (a, ca, id_a, pa)) in shapes.iter().enumerate() {
        for (b, cb, id_b, pb) in &shapes[i + 1..] {
            if b[0] - cb > a[2] + ca {
                break;
            }
            if b[1] - cb > a[3] + ca || a[1] - ca > b[3] + cb {
                continue;
            }
            let required = ca + cb;
            let (overlap, distance) = polygon::relation(pa, pb, required);
            if overlap {
                report.errors.push(issue(
                    "overlap",
                    format!("Pieces '{id_a}' and '{id_b}' overlap."),
                    &[id_a, id_b],
                ));
            } else if (required > 0.0 && distance <= 1e-9) || distance < required - TOLERANCE {
                report.errors.push(issue("gap", format!("Pieces '{id_a}' and '{id_b}' are {distance:.3} mm apart; need {required:.3} mm."), &[id_a, id_b]));
            }
        }
    }
    v02::bundles(job, result, &mut report);
    let cut_length = shapes.iter().map(|s| s.0[2]).fold(m.length, f64::max);
    report.length = cut_length;
    report.efficiency = if cut_length > 0.0 && job.fabric.width > 0.0 {
        m.piece_area / (cut_length * job.fabric.width)
    } else {
        0.0
    };
    if job
        .fabric
        .max_length
        .is_some_and(|limit| cut_length > limit)
    {
        report.errors.push(issue(
            "too_long",
            format!(
                "Marker length {:.3} mm exceeds the fabric maximum.",
                cut_length
            ),
            &[],
        ));
    }
    for (code, supplied, computed, tolerance) in [
        ("length", result.length, report.length, TOLERANCE),
        ("efficiency", result.efficiency, report.efficiency, 0.0001),
    ] {
        if !supplied.is_finite() || (supplied - computed).abs() > tolerance {
            report.warnings.push(issue(
                code,
                format!("Reported {code} is {supplied}; recomputed value is {computed}."),
                &[],
            ));
        }
    }
    report.ok = report.errors.is_empty();
    report
}
