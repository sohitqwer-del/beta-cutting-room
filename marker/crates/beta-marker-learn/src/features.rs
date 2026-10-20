use anyhow::{Result, bail, ensure};
use beta_marker_core::rules::{allowed_rotations, effective_job, flip_allowed, multiplier};
use beta_marker_core::{Job, Layout, Nap, Pt};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Versioned, fixed-order feature names. Units are fractions, millimetres and counts.
pub fn feature_names() -> Vec<String> {
    let mut names: Vec<String> = [
        "log_piece_count",
        "log_shape_groups",
        "shape_group_fraction",
        "shape_entropy",
        "log_width_mm",
        "log_area_over_width_squared",
        "largest_area_share",
        "sheet_code_2",
        "sheet_code_3",
        "extra_constraints_per_piece",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    for name in [
        "area_over_width_squared",
        "convexity",
        "rectangularity",
        "compactness",
        "aspect",
        "short_side_over_width",
        "long_side_over_width",
        "rotation_components",
        "rotation_coverage",
        "optional_flip",
    ] {
        for stat in ["mean", "std", "min", "max"] {
            names.push(format!("{name}_{stat}"));
        }
    }
    names
}

/// Rotation-invariant geometric measurements in millimetres.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Shape {
    /// Polygon area, mmÂ².
    pub area: f64,
    /// Polygon area divided by convex hull area.
    pub convexity: f64,
    /// Polygon area divided by minimum enclosing rectangle area.
    pub rectangularity: f64,
    /// 4Ï€ area / perimeterÂ².
    pub compactness: f64,
    /// Short side of the minimum-area rectangle, mm.
    pub short: f64,
    /// Long side of the minimum-area rectangle, mm.
    pub long: f64,
}

fn cross(a: Pt, b: Pt, c: Pt) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
fn area(points: &[Pt]) -> f64 {
    let origin = points[0];
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(&a, &b)| cross(origin, a, b))
        .sum::<f64>()
        .abs()
        * 0.5
}

impl Shape {
    /// Measure one simple outline. Holes must not be silently removed by the caller.
    pub fn from_outline(points: &[Pt]) -> Result<Self> {
        ensure!(
            points.len() >= 3 && points.len() <= 100_000,
            "outline needs 3..100000 vertices"
        );
        ensure!(
            points
                .iter()
                .flatten()
                .all(|v| v.is_finite() && v.abs() <= 1e9),
            "invalid coordinates"
        );
        let a = area(points);
        ensure!(
            a.is_finite() && a > 1e-12,
            "outline has no positive finite area"
        );
        let perimeter = points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .take(points.len())
            .map(|(a, b)| (b[0] - a[0]).hypot(b[1] - a[1]))
            .sum::<f64>();
        let mut sorted = points.to_vec();
        sorted.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
        sorted.dedup();
        let mut hull: Vec<Pt> = Vec::new();
        for &p in &sorted {
            while hull.len() >= 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0 {
                hull.pop();
            }
            hull.push(p);
        }
        let lower = hull.len();
        for &p in sorted.iter().rev().skip(1) {
            while hull.len() > lower && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0.0
            {
                hull.pop();
            }
            hull.push(p);
        }
        hull.pop();
        ensure!(hull.len() >= 3, "degenerate convex hull");
        // Bound quadratic work on untrusted input; real garment hulls are small.
        ensure!(
            hull.len() <= 4096,
            "outline convex hull is too complex for this estimator"
        );
        let hull_area = area(&hull);
        let mut best = (f64::INFINITY, 0.0_f64, 0.0_f64);
        for (&p, &q) in hull
            .iter()
            .zip(hull.iter().cycle().skip(1))
            .take(hull.len())
        {
            let dx = q[0] - p[0];
            let dy = q[1] - p[1];
            let norm = dx.hypot(dy);
            let (cos, sin) = (dx / norm, dy / norm);
            let mut bounds = [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ];
            for v in &hull {
                let (x, y) = (v[0] - p[0], v[1] - p[1]);
                let (u, v) = (x * cos + y * sin, -x * sin + y * cos);
                bounds[0] = bounds[0].min(u);
                bounds[1] = bounds[1].min(v);
                bounds[2] = bounds[2].max(u);
                bounds[3] = bounds[3].max(v);
            }
            let (w, h) = (bounds[2] - bounds[0], bounds[3] - bounds[1]);
            let candidate = (w * h, w.min(h), w.max(h));
            if candidate.0 < best.0 * (1.0 - 1e-12)
                || ((candidate.0 - best.0).abs() <= best.0 * 1e-12 && candidate.1 > best.1)
            {
                best = candidate;
            }
        }
        let shape = Self {
            area: a,
            convexity: (a / hull_area).min(1.0),
            rectangularity: (a / best.0).min(1.0),
            compactness: 4.0 * std::f64::consts::PI * a / perimeter.powi(2),
            short: best.1,
            long: best.2,
        };
        shape.validate()?;
        Ok(shape)
    }
    fn validate(&self) -> Result<()> {
        ensure!(
            [
                self.area,
                self.convexity,
                self.rectangularity,
                self.compactness,
                self.short,
                self.long
            ]
            .iter()
            .all(|v| v.is_finite() && *v > 0.0),
            "invalid shape summary"
        );
        ensure!(
            self.short <= self.long
                && self.convexity <= 1.000001
                && self.rectangularity <= 1.000001
                && self.compactness <= 1.000001,
            "inconsistent shape summary"
        );
        Ok(())
    }
    fn signature(self) -> [i64; 4] {
        // A geometric diversity proxy, independent of piece labels or dataset hashes.
        [
            self.area.ln(),
            self.short.ln(),
            self.long.ln(),
            self.convexity,
        ]
        .map(|v| (v * 1e7).round() as i64)
    }
}

/// Summary of legal orientation unions: components, angular coverage, optional hand.
/// Each input is `[start_degrees, end_degrees, hand]`; hand is 0 or 1.
/// Components are averaged across available hands; coverage is in [0,1].
pub fn summarize_rotations(intervals: &[[f64; 3]]) -> Result<[f64; 3]> {
    ensure!(
        !intervals.is_empty() && intervals.len() <= 4096,
        "missing or excessive rotations"
    );
    let mut hands: [Vec<[f64; 2]>; 2] = Default::default();
    for &[start, end, hand] in intervals {
        ensure!(
            start.is_finite() && end.is_finite() && [0.0, 1.0].contains(&hand),
            "invalid rotation interval"
        );
        let delta = end - start;
        ensure!(delta.is_finite(), "rotation overflow");
        let span = if delta.abs() >= 360.0 {
            360.0
        } else {
            delta.rem_euclid(360.0)
        };
        let start = start.rem_euclid(360.0);
        let list = &mut hands[hand as usize];
        if span >= 360.0 {
            list.push([0.0, 360.0]);
        } else if start + span > 360.0 {
            list.push([start, 360.0]);
            list.push([0.0, start + span - 360.0]);
        } else {
            list.push([start, start + span]);
        }
    }
    let hand_count = hands.iter().filter(|v| !v.is_empty()).count() as f64;
    let mut components = 0.0;
    let mut coverage = 0.0;
    for list in &mut hands {
        if list.is_empty() {
            continue;
        }
        list.sort_by(|a, b| a[0].total_cmp(&b[0]));
        let mut merged: Vec<[f64; 2]> = Vec::new();
        for &interval in list.iter() {
            if let Some(last) = merged.last_mut()
                && interval[0] <= last[1] + 1e-8
            {
                last[1] = last[1].max(interval[1]);
            } else {
                merged.push(interval);
            }
        }
        let mut n = merged.len();
        if n > 1 && merged[0][0] == 0.0 && merged[n - 1][1] == 360.0 {
            n -= 1;
        }
        components += n as f64;
        coverage += merged.iter().map(|v| v[1] - v[0]).sum::<f64>() / 360.0;
    }
    Ok([
        components / hand_count,
        coverage / hand_count,
        hand_count - 1.0,
    ])
}

/// One shape/orientation group. Counts represent physical demanded copies.
#[derive(Debug, Clone)]
pub struct PieceSummary {
    /// Geometry in millimetres.
    pub shape: Shape,
    /// Positive copy count.
    pub count: u32,
    /// Output of [`summarize_rotations`].
    pub rotations: [f64; 3],
}
/// Geometry-free task input, also used by the offline dataset extractor.
#[derive(Debug, Clone)]
pub struct TaskSummary {
    /// Fabric width in mm.
    pub width: f64,
    /// Original opaque dataset category (0, 2, 3). Live open jobs use 0 conditionally.
    pub sheet_code: u8,
    /// Number of non-rotation dataset constraints. Live supported jobs use zero.
    pub extra_constraints: usize,
    /// Required pieces.
    pub pieces: Vec<PieceSummary>,
}
/// Fixed-order numerical model input and physical totals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Features {
    /// Values in [`feature_names`] order.
    pub values: Vec<f64>,
    /// Total demanded piece area, mmÂ².
    pub area_mm2: f64,
    /// Fabric width, mm.
    pub width_mm: f64,
    /// Physical piece count.
    pub piece_count: u64,
}

/// Extract copy-weighted statistics without expanding repeated copies.
pub fn summarize(task: &TaskSummary) -> Result<Features> {
    ensure!(
        task.width.is_finite() && task.width > 0.0 && task.width <= 1e9,
        "invalid width"
    );
    ensure!([0, 2, 3].contains(&task.sheet_code), "unknown sheet code");
    let mut n = 0_u64;
    let mut total = 0.0;
    let mut largest: f64 = 0.0;
    let mut groups: BTreeMap<[i64; 4], u64> = BTreeMap::new();
    for p in &task.pieces {
        p.shape.validate()?;
        ensure!(
            p.count > 0
                && p.rotations.iter().all(|v| v.is_finite())
                && p.rotations[0] >= 1.0
                && (0.0..=1.0).contains(&p.rotations[1])
                && (0.0..=1.0).contains(&p.rotations[2]),
            "invalid piece summary"
        );
        n = n
            .checked_add(u64::from(p.count))
            .ok_or_else(|| anyhow::anyhow!("count overflow"))?;
        ensure!(n <= 100_000, "estimator supports at most 100000 pieces");
        total += p.shape.area * f64::from(p.count);
        largest = largest.max(p.shape.area);
        *groups.entry(p.shape.signature()).or_default() += u64::from(p.count);
    }
    ensure!(n > 0 && total.is_finite(), "empty task or area overflow");
    let count = n as f64;
    let entropy = groups
        .values()
        .map(|&v| {
            let p = v as f64 / count;
            -p * p.ln()
        })
        .sum::<f64>();
    let mut values = vec![
        count.ln_1p(),
        (groups.len() as f64).ln_1p(),
        groups.len() as f64 / count,
        entropy,
        task.width.ln(),
        (total / task.width.powi(2)).ln_1p(),
        largest / total,
        f64::from(task.sheet_code == 2),
        f64::from(task.sheet_code == 3),
        task.extra_constraints as f64 / count,
    ];
    let mut rows = Vec::with_capacity(task.pieces.len());
    for p in &task.pieces {
        let s = p.shape;
        rows.push((
            [
                s.area / task.width.powi(2),
                s.convexity,
                s.rectangularity,
                s.compactness,
                s.long / s.short,
                s.short / task.width,
                s.long / task.width,
                p.rotations[0],
                p.rotations[1],
                p.rotations[2],
            ],
            f64::from(p.count),
        ));
    }
    for col in 0..10 {
        let mean = rows.iter().map(|(r, w)| r[col] * w).sum::<f64>() / count;
        let std = (rows
            .iter()
            .map(|(r, w)| (r[col] - mean).powi(2) * w)
            .sum::<f64>()
            / count)
            .sqrt();
        let min = rows
            .iter()
            .map(|(r, _)| r[col])
            .fold(f64::INFINITY, f64::min);
        let max = rows
            .iter()
            .map(|(r, _)| r[col])
            .fold(f64::NEG_INFINITY, f64::max);
        values.extend([mean, std, min, max]);
    }
    ensure!(values.iter().all(|v| v.is_finite()), "feature overflow");
    Ok(Features {
        values,
        area_mm2: total,
        width_mm: task.width,
        piece_count: n,
    })
}

/// Extract a supported plain open job. Unsupported constraints cause an explicit error.
/// Source sheet code 0 is only conditionally interpreted as an ordinary open strip.
pub fn features_from_job(job: &Job) -> Result<Features> {
    ensure!(
        job.fabric.layout == Layout::Open && job.fabric.nap != Nap::OneWayPerBundle,
        "model supports plain open fabric only"
    );
    ensure!(
        job.fabric.defects.is_empty()
            && job.fabric.shade.is_none()
            && job.fabric.repeat.is_none()
            && job.match_rules.is_empty()
            && job.fabric.max_length.is_none(),
        "model does not support constrained or finite-sheet jobs"
    );
    ensure!(
        job.spacing.piece_gap == 0.0 && job.spacing.edge_gap == 0.0,
        "model is not calibrated for gaps"
    );
    ensure!(
        [
            job.fabric.shrinkage.length_pct,
            job.fabric.shrinkage.width_pct
        ]
        .iter()
        .all(|v| v.is_finite() && *v > -100.0),
        "invalid shrinkage"
    );
    // Guard demand overflow before calling shared checks, which use u32 products.
    for p in &job.pieces {
        for q in [p.quantity, p.mirror_quantity] {
            ensure!(
                q.checked_mul(multiplier(job, p)).is_some(),
                "demand overflow"
            );
        }
    }
    ensure!(beta_marker_core::check_job(job).is_empty(), "invalid job");
    ensure!(
        job.fabric.tilt_deg.is_finite() && job.fabric.tilt_deg >= 0.0,
        "invalid fabric tilt"
    );
    for p in &job.pieces {
        ensure!(
            p.tilt_deg.is_none_or(|v| v.is_finite() && v >= 0.0),
            "invalid piece tilt"
        );
        ensure!(
            p.grain
                .is_none_or(|g| g.iter().flatten().all(|v| v.is_finite()) && g[0] != g[1]),
            "invalid grain"
        );
    }
    let job = effective_job(job);
    let mut pieces = Vec::new();
    for p in &job.pieces {
        let count =
            (u64::from(p.quantity) + u64::from(p.mirror_quantity)) * u64::from(multiplier(&job, p));
        if count == 0 {
            continue;
        }
        ensure!(count <= 100_000, "too many demanded pieces");
        ensure!(
            p.buffer == 0.0 && p.block_margin.is_none(),
            "model is not calibrated for buffers or fusing blocks"
        );
        let shape = Shape::from_outline(&p.outline)?;
        // Geometry features are hand/rotation invariant. Freedom is per physical copy.
        let r = allowed_rotations(&job, p, false);
        let mut intervals = Vec::new();
        for hand in 0..=u8::from(flip_allowed(&job, p)) {
            if r.free {
                intervals.push([0.0, 360.0, f64::from(hand)]);
            } else {
                ensure!(r.tilt_deg.is_finite(), "invalid tilt");
                for &a in &r.angles {
                    intervals.push([a - r.tilt_deg, a + r.tilt_deg, f64::from(hand)]);
                }
            }
        }
        if intervals.is_empty() {
            bail!("piece has no legal rotations");
        }
        pieces.push(PieceSummary {
            shape,
            count: count as u32,
            rotations: summarize_rotations(&intervals)?,
        });
    }
    summarize(&TaskSummary {
        width: job.fabric.width,
        sheet_code: 0,
        extra_constraints: 0,
        pieces,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn concavity_and_rectangle_fill_measure_different_things() -> Result<()> {
        let shape = Shape::from_outline(&[
            [0.0, 0.0],
            [2.0, 0.0],
            [2.0, 1.0],
            [1.0, 1.0],
            [1.0, 2.0],
            [0.0, 2.0],
        ])?;
        assert!((shape.area - 3.0).abs() < 1e-12);
        assert!((shape.convexity - 6.0 / 7.0).abs() < 1e-12);
        assert!((shape.rectangularity - 0.75).abs() < 1e-12);
        assert!(Shape::from_outline(&[[0.0, 0.0], [1.0, 1.0], [2.0, 2.0]]).is_err());
        Ok(())
    }
    #[test]
    fn shape_is_rotation_translation_and_winding_invariant() -> Result<()> {
        let p = vec![[0.0, 0.0], [4.0, 0.0], [4.0, 2.0], [0.0, 2.0]];
        let s = Shape::from_outline(&p)?;
        assert!((s.area - 8.0).abs() < 1e-12);
        assert!((s.short - 2.0).abs() < 1e-12);
        let q: Vec<_> = p
            .iter()
            .rev()
            .map(|v| {
                [
                    100.0 + (v[0] - v[1]) * 0.5_f64.sqrt(),
                    -20.0 + (v[0] + v[1]) * 0.5_f64.sqrt(),
                ]
            })
            .collect();
        let t = Shape::from_outline(&q)?;
        assert!((s.area - t.area).abs() < 1e-10);
        assert!((s.long - t.long).abs() < 1e-10);
        Ok(())
    }
    #[test]
    fn rotation_union_handles_wrapping_overlap_and_optional_hand() -> Result<()> {
        assert_eq!(
            summarize_rotations(&[[0.0, 0.0, 0.0], [180.0, 180.0, 0.0]])?,
            [2.0, 0.0, 0.0]
        );
        let r = summarize_rotations(&[[358.0, 2.0, 0.0], [-1.0, 1.0, 0.0], [358.0, 2.0, 1.0]])?;
        assert_eq!(r[0], 1.0);
        assert!((r[1] - 4.0 / 360.0).abs() < 1e-12);
        assert_eq!(r[2], 1.0);
        assert_eq!(summarize_rotations(&[[20.0, 380.0, 0.0]])?, [1.0, 1.0, 0.0]);
        assert!(summarize_rotations(&[[0.0, f64::NAN, 0.0]]).is_err());
        Ok(())
    }
    #[test]
    fn duplicate_rows_equal_weighted_copies() -> Result<()> {
        let s = Shape::from_outline(&[[0.0, 0.0], [4.0, 0.0], [4.0, 2.0], [0.0, 2.0]])?;
        let p = PieceSummary {
            shape: s,
            count: 3,
            rotations: [2.0, 0.0, 0.0],
        };
        let a = TaskSummary {
            width: 10.0,
            sheet_code: 0,
            extra_constraints: 0,
            pieces: vec![p.clone()],
        };
        let b = TaskSummary {
            pieces: vec![PieceSummary { count: 1, ..p }; 3],
            ..a.clone()
        };
        assert_eq!(summarize(&a)?.values, summarize(&b)?.values);
        assert_eq!(summarize(&a)?.values.len(), feature_names().len());
        Ok(())
    }
}

#[cfg(test)]
mod job_tests {
    use super::*;
    use beta_marker_core::{Fabric, JOB_FORMAT, Order, Piece, RotationRule};
    fn job() -> Job {
        Job {
            format: JOB_FORMAT.into(),
            fabric: Fabric {
                width: 100.0,
                ..Default::default()
            },
            pieces: vec![Piece {
                id: "a".into(),
                outline: vec![[0.0, 0.0], [20.0, 0.0], [20.0, 10.0], [0.0, 10.0]],
                size: Some("M".into()),
                quantity: 2,
                mirror_quantity: 1,
                rotation: RotationRule::Quarter,
                ..Default::default()
            }],
            order: Some(Order {
                sizes: BTreeMap::from([("M".into(), 3)]),
            }),
            ..Default::default()
        }
    }
    #[test]
    fn job_honors_order_mirrors_shrinkage_and_rotation() -> Result<()> {
        let mut j = job();
        j.fabric.shrinkage.width_pct = 10.0;
        let f = features_from_job(&j)?;
        assert_eq!(f.piece_count, 9);
        assert!((f.area_mm2 - 1980.0).abs() < 1e-8);
        let index = feature_names()
            .iter()
            .position(|s| s == "rotation_components_mean")
            .expect("schema");
        assert_eq!(f.values[index], 4.0);
        Ok(())
    }
    #[test]
    fn unsupported_rules_and_overflow_are_not_silently_ignored() {
        let mut j = job();
        j.spacing.edge_gap = 1.0;
        assert!(features_from_job(&j).is_err());
        let mut j = job();
        j.fabric.layout = Layout::Folded;
        assert!(features_from_job(&j).is_err());
        let mut j = job();
        j.fabric.nap = Nap::OneWayPerBundle;
        assert!(features_from_job(&j).is_err());
        let mut j = job();
        j.pieces[0].quantity = u32::MAX;
        assert!(features_from_job(&j).is_err());
        let mut j = job();
        j.pieces[0].tilt_deg = Some(f64::NAN);
        assert!(features_from_job(&j).is_err());
        let mut j = job();
        j.fabric.width = f64::INFINITY;
        assert!(features_from_job(&j).is_err());
    }
}
