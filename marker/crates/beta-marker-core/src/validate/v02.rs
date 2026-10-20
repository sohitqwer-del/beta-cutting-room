//! Independent v0.2 checks. This module does not depend on the matching engine.
use super::{TOLERANCE, issue};
use crate::geometry::{angle_diff_deg, grain_alignment_deg, norm_deg, place};
use crate::model::*;
use crate::rules::{bundle_sizes, bundle_layer_copies, flip_allowed, placement_bundles, placed_outline};
use std::collections::{BTreeMap, HashMap};

pub(super) fn layers(job: &Job, piece: &Piece, pl: &Placement, report: &mut ValidationReport) {
    if pl.layer_allocations.is_empty() {
        return;
    }
    let canonical = bundle_sizes(job);
    let count = if pl.on_fold { 1 } else { 2 };
    let mut valid = job.fabric.layout != Layout::Open && pl.layer_allocations.len() == count;
    let mut useful = 0;
    for (index, layer) in pl.layer_allocations.iter().enumerate() {
        let Some(layer) = layer else { continue };
        useful += 1;
        valid &= canonical.get(layer.bundle as usize).is_some_and(|size| *size == piece.size);
        // A fold line declares geometric symmetry, not optional face-down cutting.
        // Its two full layer copies are interchangeable; asymmetric pieces still
        // require the physical normal/mirrored split unless flipping is allowed.
        valid &= layer.mirrored == (index == 1) || flip_allowed(job, piece) || piece.fold_line.is_some();
    }
    valid &= useful > 0
        && pl.bundle == pl.layer_allocations.iter().flatten().next().map(|a| a.bundle);
    if !valid {
        report.errors.push(issue("layer_allocation",
            format!("Piece '{}' has invalid physical layer, hand or garment assignments.", piece.id), &[&piece.id]));
    }
    if useful < count {
        report.warnings.push(issue("surplus_copy",
            format!("Piece '{}' cuts {} unassigned surplus layer copies.", piece.id, count - useful), &[&piece.id]));
    }
}

pub(super) fn fold(job: &Job, piece: &Piece, pl: &Placement, report: &mut ValidationReport) {
    if !pl.on_fold {
        return;
    }
    let valid = job.fabric.layout != Layout::Open
        && piece.fold_line.is_some_and(|line| {
            let points = place(&line, pl);
            let on_edge = |edge: f64| points.iter().all(|p| (p[1] - edge).abs() <= TOLERANCE);
            points.iter().flatten().all(|v| v.is_finite())
                && (points[0][0] - points[1][0]).abs() > TOLERANCE
                && (on_edge(job.fabric.width)
                    || (job.fabric.layout == Layout::Tubular && on_edge(0.0)))
        });
    if !valid {
        report.errors.push(issue(
            "fold",
            format!(
                "Piece '{}' needs a fold line on a fabric fold edge (within 0.01 mm).",
                piece.id
            ),
            &[&piece.id],
        ));
    }
}

pub(super) fn bundles(job: &Job, result: &MarkerResult, report: &mut ValidationReport) {
    let needed = job.fabric.shade.is_some()
        || job.fabric.nap == Nap::OneWayPerBundle
        || !job.match_rules.is_empty();
    if !needed && result.placements.iter().all(|p| p.bundle.is_none() && p.layer_allocations.is_empty()) {
        return;
    }
    let pieces: HashMap<_, _> = job.pieces.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut groups = BTreeMap::<u32, Vec<(&Piece, &Placement)>>::new();
    for pl in &result.placements {
        if let Some(&piece) = pieces.get(pl.piece_id.as_str()) {
            let assigned = placement_bundles(pl);
            if assigned.is_empty() && needed {
                report.errors.push(issue(
                    "bundle_missing",
                    format!("Piece '{}' has no garment bundle.", piece.id),
                    &[&piece.id],
                ));
            }
            for bundle in assigned {
                groups.entry(bundle).or_default().push((piece, pl));
            }
        }
    }
    let canonical = bundle_sizes(job);
    let mut sizes_found = BTreeMap::<Option<&str>, u64>::new();
    for (&bundle, placements) in &groups {
        let size = placements[0].0.size.as_deref();
        *sizes_found.entry(size).or_default() += 1;
        let mut counts = BTreeMap::<(&str, bool), u64>::new();
        let mut valid = placements.iter().all(|(p, _)| p.size.as_deref() == size);
        valid &= canonical
            .get(bundle as usize)
            .is_some_and(|expected| job.order.is_none() || expected.as_deref() == size);
        for (piece, pl) in placements {
            let (a, b) = bundle_layer_copies(job.fabric.layout, pl, bundle);
            for (mirror, n) in [(false, a), (true, b)] {
                *counts
                    .entry((&piece.id, mirror && !flip_allowed(job, piece)))
                    .or_default() += u64::from(n);
            }
        }
        for p in job.pieces.iter().filter(|p| p.size.as_deref() == size) {
            let expected = if flip_allowed(job, p) {
                vec![(false, u64::from(p.quantity) + u64::from(p.mirror_quantity))]
            } else {
                vec![
                    (false, u64::from(p.quantity)),
                    (true, u64::from(p.mirror_quantity)),
                ]
            };
            for (mirror, n) in expected {
                let actual = counts.get(&(p.id.as_str(), mirror)).copied().unwrap_or(0);
                let upper = if job.fabric.layout == Layout::Open
                    || result.placements.iter().any(|p| !p.layer_allocations.is_empty()) {
                    n
                } else if flip_allowed(job, p) {
                    n + 1 // A final two-layer placement may produce one surplus copy.
                } else {
                    u64::from(p.quantity.max(p.mirror_quantity))
                };
                valid &= actual >= n && actual <= upper;
            }
        }
        if !valid {
            report.errors.push(issue(
                "bundle",
                format!("Bundle {bundle} must contain one full garment of one size."),
                &[],
            ));
        }
        if job.fabric.nap == Nap::OneWayPerBundle {
            let mut direction = None;
            for (p, pl) in placements {
                let angle = norm_deg(pl.rotation_deg - grain_alignment_deg(p.grain, pl.mirrored));
                let class = if angle_diff_deg(angle, 0.0)
                    <= p.tilt_deg.unwrap_or(job.fabric.tilt_deg).max(0.0) + 0.001
                {
                    Some(false)
                } else if angle_diff_deg(angle, 180.0)
                    <= p.tilt_deg.unwrap_or(job.fabric.tilt_deg).max(0.0) + 0.001
                {
                    Some(true)
                } else {
                    None
                };
                if class.is_none() || direction.is_some_and(|d| Some(d) != class) {
                    report.errors.push(issue(
                        "nap_bundle",
                        format!("Bundle {bundle} has mixed fabric directions."),
                        &[&p.id],
                    ));
                    break;
                }
                direction = class;
            }
        }
        if let Some(shade) = job
            .fabric
            .shade
            .filter(|s| s.band_mm.is_finite() && s.band_mm > 0.0)
        {
            let axis = if shade.axis == ShadeAxis::Length {
                0
            } else {
                1
            };
            let mut lo = f64::INFINITY;
            let mut hi = f64::NEG_INFINITY;
            for (p, pl) in placements {
                for point in placed_outline(p, pl) {
                    lo = lo.min(point[axis]);
                    hi = hi.max(point[axis]);
                }
            }
            let band = ((lo + TOLERANCE) / shade.band_mm).floor();
            if !lo.is_finite() || !hi.is_finite() || hi > (band + 1.0) * shade.band_mm + TOLERANCE {
                report.errors.push(issue(
                    "shade",
                    format!("Bundle {bundle} crosses a shade band boundary."),
                    &[],
                ));
            }
        }
        matching(job, bundle, placements, report);
    }
    // `rules::bundle_sizes` defines the garments with or without an order.
    let mut expected_sizes = BTreeMap::<Option<&str>, u64>::new();
    for size in &canonical {
        *expected_sizes.entry(size.as_deref()).or_default() += 1;
    }
    expected_sizes.retain(|_, n| *n > 0);
    if sizes_found != expected_sizes {
        report.errors.push(issue(
            "bundle",
            "Garment bundle counts do not match the size ratio.".into(),
            &[],
        ));
    }
}

fn matching(
    job: &Job,
    bundle: u32,
    placements: &[(&Piece, &Placement)],
    report: &mut ValidationReport,
) {
    let Some(repeat) = job.fabric.repeat else {
        return;
    };
    let tolerance = if repeat.tolerance_mm == 0.0 {
        1.0
    } else {
        repeat.tolerance_mm
    };
    let points = |reference: &PointRef| -> Vec<Pt> {
        placements
            .iter()
            .filter(|(p, _)| p.id == reference.piece_id)
            .filter_map(|(p, pl)| {
                p.match_points
                    .iter()
                    .find(|m| m.id == reference.point_id)
                    .map(|m| place(&[m.point], pl)[0])
            })
            .collect()
    };
    for rule in &job.match_rules {
        let a = points(&rule.a);
        let b = rule
            .b
            .as_ref()
            .map(&points)
            .unwrap_or_else(|| vec![[repeat.offset_x, repeat.offset_y]]);
        // A rule for a different size has no endpoints in this bundle.
        if a.is_empty() && rule.b.as_ref().is_some_and(|_| b.is_empty()) {
            continue;
        }
        if a.is_empty() && rule.b.is_none() {
            continue;
        }
        let mut valid = !a.is_empty() && !b.is_empty() && tolerance.is_finite() && tolerance >= 0.0;
        for axis in 0..2 {
            if (axis == 0 && rule.axes == MatchAxes::Y) || (axis == 1 && rule.axes == MatchAxes::X)
            {
                continue;
            }
            let period = if axis == 0 {
                repeat.period_x
            } else {
                repeat.period_y
            };
            let Some(period) = period.filter(|v| v.is_finite() && *v > 0.0) else {
                valid = false;
                continue;
            };
            // All copies of each endpoint participate, including left/right pairs.
            for x in &a {
                for y in &b {
                    let phase = (x[axis] - y[axis] - rule.offset[axis]).rem_euclid(period);
                    valid &= phase.is_finite() && phase.min(period - phase) <= tolerance + 1e-9;
                }
            }
        }
        if !valid {
            report.errors.push(issue(
                "match",
                format!(
                    "Bundle {bundle} does not satisfy matching point {}/{}.",
                    rule.a.piece_id, rule.a.point_id
                ),
                &[&rule.a.piece_id],
            ));
        }
    }
}
