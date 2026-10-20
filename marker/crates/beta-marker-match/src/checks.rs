use anyhow::{Result, ensure};
use beta_marker_core::geometry::{bbox, transform_point};
use beta_marker_core::{Issue, Job, MarkerResult, MatchAxes, Placement, PointRef, ShadeAxis};
use beta_marker_nest::NestOptions;
use geo::{Polygon, Validation};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn axes(axes: MatchAxes, axis: usize) -> bool {
    matches!(axes, MatchAxes::Both) || matches!((axes, axis), (MatchAxes::X, 0) | (MatchAxes::Y, 1))
}

pub(crate) fn residual(delta: f64, period: f64) -> f64 {
    let r = delta.rem_euclid(period);
    r.min(period - r)
}

pub(crate) fn issue(code: &str, message: String, ids: Vec<String>) -> Issue {
    Issue {
        code: code.into(),
        message,
        piece_ids: ids,
    }
}

pub(crate) fn point(job: &Job, reference: &PointRef, placement: &Placement) -> Option<[f64; 2]> {
    let piece = job.pieces.iter().find(|p| p.id == reference.piece_id)?;
    let mp = piece
        .match_points
        .iter()
        .find(|p| p.id == reference.point_id)?;
    Some(transform_point(
        mp.point,
        placement.mirrored,
        placement.rotation_deg,
        placement.translate,
    ))
}

/// Independently check every active rule in every garment bundle.
///
/// Every copy of each referenced piece in that bundle must match every partner
/// copy. For `b = None`, the reference is the fabric repeat offset. Axes without
/// a period impose no constraint. Zero tolerance means the contract default,
/// 1 mm. Distance is measured around the repeat boundary (39.8 and 0.2 on a
/// 40 mm repeat differ by 0.4 mm). Missing points, partners and bundle IDs are
/// errors. Core validation remains responsible for geometry and demand counts.
pub fn match_errors(job: &Job, result: &MarkerResult) -> Vec<Issue> {
    let job = beta_marker_core::rules::effective_job(job);
    let mut out = Vec::new();
    if job.match_rules.is_empty() {
        return out;
    }
    let Some(repeat) = job.fabric.repeat else {
        return vec![issue(
            "match_rule",
            "Match rules need a fabric repeat.".into(),
            vec![],
        )];
    };
    if [repeat.period_x, repeat.period_y]
        .into_iter()
        .flatten()
        .any(|p| !p.is_finite() || p <= 0.0)
        || !repeat.offset_x.is_finite()
        || !repeat.offset_y.is_finite()
        || !repeat.tolerance_mm.is_finite()
        || repeat.tolerance_mm < 0.0
    {
        return vec![issue(
            "repeat",
            "Repeat periods, offsets or tolerance are invalid.".into(),
            vec![],
        )];
    }
    let tolerance = if repeat.tolerance_mm == 0.0 {
        1.0
    } else {
        repeat.tolerance_mm
    };
    let mut by_bundle = BTreeMap::<u32, Vec<&Placement>>::new();
    for pl in &result.placements {
        if let Some(bundle) = pl.bundle {
            by_bundle.entry(bundle).or_default().push(pl);
        } else {
            out.push(issue(
                "bundle",
                format!("Piece {} has no garment bundle.", pl.piece_id),
                vec![pl.piece_id.clone()],
            ));
        }
    }
    for (ri, rule) in job.match_rules.iter().enumerate() {
        let valid_ref = |r: &PointRef| {
            job.pieces
                .iter()
                .any(|p| p.id == r.piece_id && p.match_points.iter().any(|m| m.id == r.point_id))
        };
        if !valid_ref(&rule.a)
            || rule.b.as_ref().is_some_and(|b| !valid_ref(b))
            || rule.offset.iter().any(|x| !x.is_finite())
        {
            out.push(issue(
                "match_rule",
                format!("Rule {ri} has an unknown point or invalid offset."),
                vec![rule.a.piece_id.clone()],
            ));
            continue;
        }
        for (&bundle, placements) in &by_bundle {
            let aa: Vec<_> = placements
                .iter()
                .filter(|p| p.piece_id == rule.a.piece_id)
                .collect();
            let bb: Vec<_> = rule
                .b
                .as_ref()
                .map(|b| {
                    placements
                        .iter()
                        .filter(|p| p.piece_id == b.piece_id)
                        .collect()
                })
                .unwrap_or_default();
            if aa.is_empty() && bb.is_empty() {
                continue;
            }
            if aa.is_empty() || (rule.b.is_some() && bb.is_empty()) {
                out.push(issue(
                    "match_partner",
                    format!("Rule {ri} has no partner in bundle {bundle}."),
                    vec![rule.a.piece_id.clone()],
                ));
                continue;
            }
            for a in aa {
                let Some(ap) = point(&job, &rule.a, a) else {
                    continue;
                };
                let partners: Vec<_> = if let Some(b) = &rule.b {
                    bb.iter().filter_map(|p| point(&job, b, p)).collect()
                } else {
                    vec![[repeat.offset_x, repeat.offset_y]]
                };
                for bp in partners {
                    for (axis, period) in [repeat.period_x, repeat.period_y].into_iter().enumerate()
                    {
                        if !axes(rule.axes, axis) {
                            continue;
                        }
                        let Some(period) = period else { continue };
                        let delta = ap[axis] - bp[axis] - rule.offset[axis];
                        if !delta.is_finite() || residual(delta, period) > tolerance + 1e-7 {
                            out.push(issue(
                                "match",
                                format!(
                                    "Rule {ri}, bundle {bundle}, axis {} is out of phase.",
                                    ["x", "y"][axis]
                                ),
                                vec![rule.a.piece_id.clone()],
                            ));
                        }
                    }
                }
            }
        }
    }
    out
}

pub(crate) fn shade_errors(job: &Job, result: &MarkerResult) -> Vec<Issue> {
    let Some(shade) = job.fabric.shade else {
        return vec![];
    };
    let axis = usize::from(shade.axis == ShadeAxis::Width);
    let mut bands = BTreeMap::<u32, i64>::new();
    let mut errors = Vec::new();
    for pl in &result.placements {
        let Some(piece) = job.pieces.iter().find(|p| p.id == pl.piece_id) else {
            continue;
        };
        let Some(b) = bbox(&crate::search::outline(piece, pl)) else {
            continue;
        };
        let band = ((b[axis] + 1e-7) / shade.band_mm).floor() as i64;
        let Some(bundle) = pl.bundle else {
            errors.push(issue(
                "bundle",
                "Shade bands need garment bundle IDs.".into(),
                vec![piece.id.clone()],
            ));
            continue;
        };
        if b[axis + 2] > (band + 1) as f64 * shade.band_mm + 1e-7
            || bands.insert(bundle, band).is_some_and(|old| old != band)
        {
            errors.push(issue(
                "shade",
                format!("Bundle {bundle} crosses a shade band."),
                vec![piece.id.clone()],
            ));
        }
    }
    errors
}

pub(crate) fn check_input(job: &Job, opts: &NestOptions) -> Result<()> {
    // Core v0.2's demand helper multiplies u32 counts. Check overflow before
    // calling check_job, which calls that helper itself.
    for p in &job.pieces {
        let m = beta_marker_core::rules::multiplier(job, p);
        ensure!(
            p.quantity.checked_add(p.mirror_quantity).is_some()
                && p.quantity.checked_mul(m).is_some()
                && p.mirror_quantity.checked_mul(m).is_some(),
            "copy count overflows for {}",
            p.id
        );
    }
    let errors = beta_marker_core::check_job(job);
    ensure!(errors.is_empty(), "invalid job: {errors:?}");
    ensure!(
        opts.time_limit_s.is_finite() && opts.time_limit_s > 0.0 && opts.workers != Some(0),
        "invalid nesting controls"
    );
    ensure!(job.fabric.width.is_finite(), "fabric width must be finite");
    ensure!(
        job.fabric
            .shade
            .is_none_or(|s| s.band_mm.is_finite() && s.band_mm > 0.0),
        "shade band must be finite and positive"
    );
    if let Some(r) = job.fabric.repeat {
        ensure!(
            [r.period_x, r.period_y]
                .into_iter()
                .flatten()
                .all(|p| p.is_finite() && p > 0.0)
                && r.offset_x.is_finite()
                && r.offset_y.is_finite()
                && r.tolerance_mm.is_finite()
                && r.tolerance_mm >= 0.0,
            "invalid repeat"
        );
    }
    ensure!(
        job.match_rules
            .iter()
            .all(|r| r.offset.iter().all(|v| v.is_finite())),
        "match offsets must be finite"
    );
    for p in &job.pieces {
        ensure!(
            p.buffer.is_finite()
                && p.buffer >= 0.0
                && p.block_margin.is_none_or(|m| m.is_finite() && m >= 0.0),
            "invalid buffer or fusing-block margin on {}",
            p.id
        );
        let mut ids = BTreeSet::new();
        ensure!(
            p.match_points
                .iter()
                .all(|m| ids.insert(&m.id) && m.point.iter().all(|v| v.is_finite())),
            "invalid or duplicate match points on {}",
            p.id
        );
    }
    for defect in &job.fabric.defects {
        ensure!(
            defect.polygon.len() >= 3 && defect.polygon.iter().flatten().all(|v| v.is_finite()),
            "invalid defect polygon {}",
            defect.id
        );
        let polygon = Polygon::new(
            defect.polygon.iter().map(|p| (p[0], p[1])).collect(),
            vec![],
        );
        ensure!(
            polygon.is_valid() && beta_marker_core::geometry::area(&defect.polygon) > 0.0,
            "invalid defect polygon {}",
            defect.id
        );
    }
    if let Some(order) = &job.order {
        ensure!(
            order
                .sizes
                .values()
                .try_fold(0_u32, |s, &n| s.checked_add(n))
                .is_some(),
            "too many garment bundles"
        );
    }
    Ok(())
}
