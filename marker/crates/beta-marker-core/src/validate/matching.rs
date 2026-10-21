//! One phase measurement implementation shared by validation and overlays.
use crate::{
    geometry::place,
    model::*,
    rules::{bundle_sizes, effective_job, placement_bundles},
};

/// Report all applicable rules, even when a garment has no placed pieces yet.
/// Invalid jobs return no rows; callers should display `check_job` errors first.
pub fn match_reports(job: &Job, result: &MarkerResult) -> Result<Vec<MatchReport>, String> {
    if !crate::check_job(job).is_empty() {
        return Ok(vec![]);
    }
    evaluate(&effective_job(job), result)
}

pub(super) fn evaluate(job: &Job, result: &MarkerResult) -> Result<Vec<MatchReport>, String> {
    let Some(repeat) = job.fabric.repeat else {
        return Ok(vec![]);
    };
    let tolerance = if repeat.tolerance_mm == 0. {
        1.
    } else {
        repeat.tolerance_mm
    };
    if job.match_rules.is_empty() {
        return Ok(vec![]);
    }
    let mut groups = std::collections::BTreeMap::<u32, Vec<(usize, &Placement)>>::new();
    for (index, placement) in result.placements.iter().enumerate() {
        for bundle in placement_bundles(placement) {
            groups.entry(bundle).or_default().push((index, placement));
        }
    }
    let pieces: std::collections::HashMap<_, _> =
        job.pieces.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut rows = vec![];
    for (bundle, size) in bundle_sizes(job).iter().enumerate() {
        for (rule_index, rule) in job.match_rules.iter().enumerate() {
            let Some(piece) = pieces.get(rule.a.piece_id.as_str()) else {
                continue;
            };
            if job.order.is_some() && piece.size != *size {
                continue;
            }
            let points = |reference: &PointRef| -> Vec<(Option<usize>, Option<Pt>)> {
                let mut found = vec![];
                for &(index, pl) in groups.get(&(bundle as u32)).into_iter().flatten() {
                    if pl.piece_id != reference.piece_id {
                        continue;
                    }
                    if let Some(p) = pieces.get(reference.piece_id.as_str())
                        && let Some(m) = p.match_points.iter().find(|m| m.id == reference.point_id)
                    {
                        let point = place(&[m.point], pl)[0];
                        found.push((
                            Some(index),
                            point.iter().all(|v| v.is_finite()).then_some(point),
                        ));
                    }
                }
                if found.is_empty() {
                    found.push((None, None));
                }
                found
            };
            let a = points(&rule.a);
            let b = rule
                .b
                .as_ref()
                .map(points)
                .unwrap_or_else(|| vec![(None, Some([repeat.offset_x, repeat.offset_y]))]);
            for &(ai, ap) in &a {
                for &(bi, bp) in &b {
                    let mut offset_mm = [None; 2];
                    let mut error: f64 = 0.;
                    let mut valid = ap.is_some() && bp.is_some();
                    if let (Some(x), Some(y)) = (ap, bp) {
                        for axis in 0..2 {
                            if (axis == 0 && rule.axes == MatchAxes::Y)
                                || (axis == 1 && rule.axes == MatchAxes::X)
                            {
                                continue;
                            }
                            let period = [repeat.period_x, repeat.period_y][axis];
                            if let Some(period) = period.filter(|v| v.is_finite() && *v > 0.) {
                                let phase =
                                    (x[axis] - y[axis] - rule.offset[axis]).rem_euclid(period);
                                let residual = if phase > period / 2. {
                                    phase - period
                                } else {
                                    phase
                                };
                                valid &= residual.is_finite();
                                if residual.is_finite() {
                                    offset_mm[axis] = Some(residual);
                                    error = error.max(residual.abs());
                                }
                            } else {
                                valid = false;
                            }
                        }
                    }
                    let pass = valid && error <= tolerance + 1e-9;
                    if rows.len() >= 100_000 {
                        return Err("Matching report exceeds 100000 endpoint pairs. Reduce the marker ratio or number of rules.".into());
                    }
                    rows.push(MatchReport {
                        rule: rule_index,
                        garment: bundle as u32,
                        status: if ap.is_none() || bp.is_none() {
                            MatchStatus::NotPlaced
                        } else if pass {
                            MatchStatus::Matched
                        } else {
                            MatchStatus::Off
                        },
                        a_ref: rule.a.clone(),
                        b_ref: rule.b.clone(),
                        a: ap,
                        b: bp,
                        a_placement: ai,
                        b_placement: bi,
                        offset_mm,
                        error_mm: valid.then_some(error),
                        tolerance_mm: tolerance,
                        pass,
                    });
                }
            }
        }
    }
    Ok(rows)
}
