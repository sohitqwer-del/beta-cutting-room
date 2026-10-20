//! Geometry-only consumption estimates, transparent ridge fitting, and short real nests.
use anyhow::{Result, ensure};
use beta_marker_core::geometry::{area, bbox, transform_points};
use beta_marker_core::rules::{effective_job, multiplier};
use beta_marker_core::{Job, Layout, MarkerResult, Pt};
use beta_marker_nest::NestOptions;
use serde::{Deserialize, Serialize};

/// Five scaled geometry features; the regression adds an intercept.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Features {
    /// ln(1 + physical piece count) / 5.
    pub piece_count: f64,
    /// Mean outline area / fabric width squared.
    pub mean_piece_area: f64,
    /// Largest minimum grain-allowed piece width / fabric width.
    pub largest_width_ratio: f64,
    /// Copy-weighted mean of outline area / convex-hull area.
    pub convexity_ratio: f64,
    /// Copy-weighted rotation freedom: fixed 0, flip 0.25, quarter 0.75, free 1.
    pub rotation_freedom: f64,
}
impl Features {
    fn row(self) -> [f64; 6] {
        [
            1.0,
            self.piece_count,
            self.mean_piece_area,
            self.largest_width_ratio,
            self.convexity_ratio,
            self.rotation_freedom,
        ]
    }
}
/// Linear marker-efficiency model. Save coefficients and source notes together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EfficiencyModel {
    /// Intercept, then five coefficients in [`Features`] order.
    pub coefficients: [f64; 6],
    /// Number of real nests used for fitting.
    pub training_samples: usize,
    /// Sources and fit notes.
    pub provenance: String,
}
impl Default for EfficiencyModel {
    fn default() -> Self {
        Self {
            coefficients: [0.55, 0.04, -0.15, -0.12, 0.22, 0.08],
            training_samples: 0,
            provenance: "Uncalibrated heuristic; fit local observations before quotations.".into(),
        }
    }
}
impl EfficiencyModel {
    /// Predict a fraction clamped to 20–95%. This is not a feasible packing bound.
    pub fn predict(&self, f: Features) -> Result<f64> {
        let row = f.row();
        ensure!(
            self.coefficients
                .iter()
                .chain(row.iter())
                .all(|v| v.is_finite()),
            "model and features must be finite"
        );
        let value: f64 = row.iter().zip(self.coefficients).map(|(a, b)| a * b).sum();
        ensure!(value.is_finite(), "model numeric overflow");
        Ok(value.clamp(0.20, 0.95))
    }
}
/// One real nest observation; stores no pattern geometry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    /// Dataset/style id for splitting training and holdout by pattern.
    pub source: String,
    /// Features at the measured width and ratio.
    pub features: Features,
    /// Independently validated marker efficiency, fraction.
    pub efficiency: f64,
    /// Measured marker length, mm.
    pub length_mm: f64,
    /// Fabric width, mm.
    pub width_mm: f64,
    /// Garments in the job.
    pub garments: u32,
}
/// Consumption estimate excluding end allowances and roll losses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumptionEstimate {
    /// Full marker length, mm.
    pub marker_length_mm: f64,
    /// Fabric length per explicitly supplied garment, mm/garment.
    pub fabric_per_garment_mm: f64,
    /// Predicted marker efficiency, fraction.
    pub expected_efficiency: f64,
    /// Total demanded outline area after shrinkage, mm².
    pub total_piece_area_mm2: f64,
    /// Input features.
    pub features: Features,
    /// Limitations for costing decisions.
    pub warnings: Vec<String>,
}
/// Accurate mode returns a checked real marker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccurateEstimate {
    /// Validated nesting output.
    pub marker: MarkerResult,
    /// Real marker length / garments, mm/garment.
    pub fabric_per_garment_mm: f64,
}
fn check_input(job: &Job, width: f64) -> Result<()> {
    ensure!(
        width.is_finite() && width > 0.0,
        "fabric width must be finite and positive"
    );
    ensure!(
        job.fabric.layout == Layout::Open,
        "costing needs open fabric; expand folded jobs first"
    );
    let mut count = 0u64;
    for p in &job.pieces {
        for q in [p.quantity, p.mirror_quantity] {
            let n = q
                .checked_mul(multiplier(job, p))
                .ok_or_else(|| anyhow::anyhow!("piece demand overflow"))?;
            count = count
                .checked_add(u64::from(n))
                .ok_or_else(|| anyhow::anyhow!("piece count overflow"))?;
        }
    }
    ensure!(
        count > 0 && count <= 100_000,
        "costing needs 1 to 100,000 physical pieces"
    );
    let mut j = job.clone();
    j.fabric.width = width;
    ensure!(
        beta_marker_core::check_job(&j).is_empty(),
        "invalid costing job"
    );
    ensure!(
        [
            j.spacing.piece_gap,
            j.spacing.edge_gap,
            j.fabric.tilt_deg,
            j.fabric.shrinkage.length_pct,
            j.fabric.shrinkage.width_pct
        ]
        .iter()
        .all(|v| v.is_finite()),
        "non-finite fabric settings"
    );
    ensure!(
        j.fabric.shrinkage.length_pct > -100.0 && j.fabric.shrinkage.width_pct > -100.0,
        "shrinkage must preserve positive dimensions"
    );
    Ok(())
}
fn check_garments(job: &Job, garments: u32) -> Result<()> {
    ensure!(garments > 0, "garment count must be positive");
    if let Some(order) = &job.order {
        let sum: u64 = order.sizes.values().map(|n| u64::from(*n)).sum();
        ensure!(
            sum == u64::from(garments),
            "garment count differs from job ratio"
        );
    }
    Ok(())
}
fn cross(a: Pt, b: Pt, c: Pt) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
fn hull_area(points: &[Pt]) -> f64 {
    let mut p = points.to_vec();
    p.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    p.dedup();
    if p.len() < 3 {
        return 0.0;
    }
    let mut lower = Vec::new();
    for &v in &p {
        while lower.len() >= 2 && cross(lower[lower.len() - 2], lower[lower.len() - 1], v) <= 0.0 {
            lower.pop();
        }
        lower.push(v);
    }
    let mut upper = Vec::new();
    for &v in p.iter().rev() {
        while upper.len() >= 2 && cross(upper[upper.len() - 2], upper[upper.len() - 1], v) <= 0.0 {
            upper.pop();
        }
        upper.push(v);
    }
    lower.pop();
    upper.pop();
    lower.extend(upper);
    area(&lower)
}
/// Extract features and demanded area with mirror quantities, size ratios and shrinkage.
/// Freely rotated minimum width is sampled every 5 degrees for this feature only.
pub fn features(job: &Job, width: f64) -> Result<(Features, f64)> {
    check_input(job, width)?;
    let mut job = effective_job(job);
    job.fabric.width = width;
    let (mut count, mut total, mut convexity, mut freedom, mut largest) =
        (0.0_f64, 0.0, 0.0, 0.0, 0.0_f64);
    for d in beta_marker_core::demands(&job) {
        let p = &job.pieces[d.piece_index];
        let n = f64::from(d.count);
        let a = area(&p.outline);
        let hull = hull_area(&p.outline);
        ensure!(
            a.is_finite() && a > 0.0 && hull.is_finite() && hull > 0.0 && a <= hull + 0.01,
            "invalid polygon area for {}",
            p.id
        );
        let allowed = beta_marker_core::allowed_rotations(&job, p, d.mirrored);
        let angles = if allowed.free {
            (0..36).map(|i| f64::from(i) * 5.0).collect::<Vec<_>>()
        } else {
            allowed.angles.clone()
        };
        let mut min_width = f64::INFINITY;
        for angle in angles {
            if let Some(b) = bbox(&transform_points(&p.outline, d.mirrored, angle, [0.0, 0.0])) {
                min_width = min_width.min(b[3] - b[1]);
            }
        }
        ensure!(min_width.is_finite(), "piece has no usable rotation");
        largest = largest.max(min_width / width);
        let rotation = if allowed.free {
            1.0
        } else if allowed.angles.len() >= 4 {
            0.75
        } else if allowed.angles.len() >= 2 {
            0.25
        } else {
            0.0
        };
        count += n;
        total += n * a;
        convexity += n * (a / hull).min(1.0);
        freedom += n * rotation;
    }
    let f = Features {
        piece_count: count.ln_1p() / 5.0,
        mean_piece_area: total / count / width / width,
        largest_width_ratio: largest,
        convexity_ratio: convexity / count,
        rotation_freedom: freedom / count,
    };
    ensure!(
        total.is_finite() && f.row().iter().all(|v| v.is_finite()),
        "feature numeric overflow"
    );
    ensure!(
        largest <= 1.0 + 1e-9,
        "a piece cannot fit requested fabric width"
    );
    Ok((f, total))
}
/// Quick estimate = demanded area / (width × predicted efficiency) / garments.
pub fn estimate(
    job: &Job,
    width: f64,
    garments: u32,
    model: &EfficiencyModel,
) -> Result<ConsumptionEstimate> {
    check_garments(job, garments)?;
    let (f, area) = features(job, width)?;
    let efficiency = model.predict(f)?;
    let length = area / width / efficiency;
    ensure!(length.is_finite(), "consumption numeric overflow");
    let mut warnings = vec![
        "Geometry estimate excludes end allowances and roll loss; it is not a feasible marker."
            .into(),
    ];
    if model.training_samples == 0 {
        warnings.push("Model has not been calibrated on real nests.".into());
    }
    if job.fabric.repeat.is_some()
        || job.fabric.shade.is_some()
        || !job.fabric.defects.is_empty()
        || job.pieces.iter().any(|p| p.block_margin.is_some())
    {
        warnings.push("Repeat, shade bands, defects and fusing blocks are not model features; use accurate mode.".into());
    }
    Ok(ConsumptionEstimate {
        marker_length_mm: length,
        fabric_per_garment_mm: length / f64::from(garments),
        expected_efficiency: efficiency,
        total_piece_area_mm2: area,
        features: f,
        warnings,
    })
}
/// Real nest with exactly two workers and a search budget in (0,20] seconds.
/// Construction and validation add wall-clock overhead beyond the search budget.
pub fn accurate_estimate(
    job: &Job,
    width: f64,
    garments: u32,
    seconds: f64,
    seed: u64,
) -> Result<AccurateEstimate> {
    check_input(job, width)?;
    check_garments(job, garments)?;
    ensure!(
        seconds.is_finite() && seconds > 0.0 && seconds <= 20.0,
        "accurate budget must be in (0,20] seconds"
    );
    let mut job = effective_job(job);
    job.fabric.width = width;
    let marker = beta_marker_match::nest_with_rules(
        &job,
        &NestOptions {
            time_limit_s: seconds,
            seed: Some(seed),
            workers: Some(2),
            ..Default::default()
        },
    )?;
    ensure!(
        beta_marker_core::validate(&job, &marker).ok,
        "accurate estimate failed independent validation"
    );
    Ok(AccurateEstimate {
        fabric_per_garment_mm: marker.length / f64::from(garments),
        marker,
    })
}
/// Fit ridge regression with an unpenalized intercept. Hold out whole patterns for evaluation.
pub fn fit(observations: &[Observation], ridge: f64) -> Result<EfficiencyModel> {
    ensure!(
        observations.len() >= 6 && ridge.is_finite() && ridge > 0.0,
        "fit needs six observations and positive ridge"
    );
    let mut a = [[0.0; 7]; 6];
    for o in observations {
        ensure!(
            o.efficiency.is_finite() && o.efficiency > 0.0 && o.efficiency <= 1.0,
            "invalid observed efficiency"
        );
        let row = o.features.row();
        ensure!(row.iter().all(|v| v.is_finite()), "non-finite features");
        for i in 0..6 {
            for j in 0..6 {
                a[i][j] += row[i] * row[j];
            }
            a[i][6] += row[i] * o.efficiency;
        }
    }
    for (i, row) in a.iter_mut().enumerate().skip(1) {
        row[i] += ridge;
    }
    for col in 0..6 {
        let pivot = (col..6)
            .max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))
            .ok_or_else(|| anyhow::anyhow!("empty regression"))?;
        a.swap(col, pivot);
        let d = a[col][col];
        ensure!(
            d.is_finite() && d.abs() > 1e-12,
            "singular regression matrix"
        );
        for v in &mut a[col][col..=6] {
            *v /= d;
        }
        let pivot_row = a[col];
        for (i, row) in a.iter_mut().enumerate() {
            if i != col {
                let scale = row[col];
                for (value, pivot) in row[col..=6].iter_mut().zip(&pivot_row[col..=6]) {
                    *value -= scale * pivot;
                }
            }
        }
    }
    let coefficients = std::array::from_fn(|i| a[i][6]);
    ensure!(
        coefficients.iter().all(|v| v.is_finite()),
        "non-finite fitted model"
    );
    Ok(EfficiencyModel {
        coefficients,
        training_samples: observations.len(),
        provenance: format!("Own validated nests; ridge {ridge}; see held-out consumption MAPE."),
    })
}
/// Mean absolute percentage error in consumption against real nests, percent.
/// Relative length error = observed efficiency / predicted efficiency - 1.
pub fn consumption_mape(model: &EfficiencyModel, observations: &[Observation]) -> Result<f64> {
    ensure!(!observations.is_empty(), "MAPE needs observations");
    let mut sum = 0.0;
    for o in observations {
        ensure!(
            o.efficiency.is_finite()
                && o.efficiency > 0.0
                && o.efficiency <= 1.0
                && o.length_mm.is_finite()
                && o.length_mm > 0.0
                && o.width_mm.is_finite()
                && o.width_mm > 0.0
                && o.garments > 0,
            "invalid observed nest"
        );
        sum += (o.efficiency / model.predict(o.features)? - 1.0).abs();
    }
    Ok(100.0 * sum / observations.len() as f64)
}
