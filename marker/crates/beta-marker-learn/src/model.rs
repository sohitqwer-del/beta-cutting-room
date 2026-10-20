use crate::{Features, feature_names, features_from_job};
use anyhow::{Result, ensure};
use beta_marker_core::Job;
use serde::{Deserialize, Serialize};

const MAX_MODEL_BYTES: usize = 5_000_000;
type Node = (i32, f64, usize, usize, f64);

/// Validated, immutable additive regression-tree model. Load once and reuse.
#[derive(Debug, Clone)]
pub struct Model {
    file: ModelFile,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelFile {
    format: String,
    feature_names: Vec<String>,
    intercept: f64,
    // [feature (-1 for leaf), threshold, left index, right index, leaf value].
    trees: Vec<Vec<Node>>,
    residual_q90: f64,
    feature_min: Vec<f64>,
    feature_max: Vec<f64>,
    provenance: String,
    license: String,
}

/// Estimated consumption, with a validation-calibrated residual interval.
/// This is not a layout, feasibility certificate, maximum efficiency or quotation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Estimate {
    /// Expected Lectra-style achieved efficiency as a fraction.
    pub efficiency: f64,
    /// Nominal 90% residual interval, calibrated on a reserved validation subset.
    pub efficiency_interval: [f64; 2],
    /// Demanded area / (fabric width × predicted efficiency), in mm.
    pub length_mm: f64,
    /// Length interval, in increasing order. Excludes ends, splices and roll loss.
    pub length_interval_mm: [f64; 2],
    /// Features outside the training min/max envelope; empty does not prove in-domain.
    pub outside_training_range: Vec<String>,
}
impl Model {
    /// Load and validate JSON. Tree topology is bounded and acyclic before inference.
    pub fn from_json(json: &str) -> Result<Self> {
        ensure!(json.len() <= MAX_MODEL_BYTES, "model exceeds 5 MB");
        let file: ModelFile = serde_json::from_str(json)?;
        ensure!(
            file.format == "beta-marker-learn/1" && file.feature_names == feature_names(),
            "unsupported model or feature schema"
        );
        let n = file.feature_names.len();
        ensure!(
            file.intercept.is_finite()
                && file.residual_q90.is_finite()
                && (0.0..=1.0).contains(&file.residual_q90),
            "invalid model calibration"
        );
        ensure!(
            file.feature_min.len() == n && file.feature_max.len() == n,
            "invalid feature ranges"
        );
        ensure!(
            file.feature_min
                .iter()
                .zip(&file.feature_max)
                .all(|(a, b)| a.is_finite() && b.is_finite() && a <= b),
            "invalid feature bounds"
        );
        ensure!(
            !file.trees.is_empty() && file.trees.len() <= 4096,
            "invalid tree count"
        );
        for tree in &file.trees {
            ensure!(!tree.is_empty() && tree.len() <= 16383, "invalid tree size");
            for (index, &(feature, threshold, left, right, value)) in tree.iter().enumerate() {
                ensure!(
                    threshold.is_finite() && value.is_finite(),
                    "non-finite tree node"
                );
                if feature == -1 {
                    ensure!(left == 0 && right == 0, "leaf has children");
                } else {
                    ensure!(
                        feature >= 0 && (feature as usize) < n,
                        "invalid feature index"
                    );
                    ensure!(
                        left > index && right > index && left < tree.len() && right < tree.len(),
                        "tree has invalid or cyclic edges"
                    );
                }
            }
        }
        Ok(Self { file })
    }
    /// Load the small bundled model, attributed to the CC BY 4.0 Lectra dataset.
    pub fn bundled() -> Result<Self> {
        Self::from_json(include_str!("../models/lectra-v1.json"))
    }
    /// Model source and fit notes retained in the artifact.
    pub fn provenance(&self) -> &str {
        &self.file.provenance
    }
    /// Predict from Rust features. All inputs must be finite; no network or Python is used.
    pub fn predict(&self, features: &Features) -> Result<Estimate> {
        ensure!(
            features.values.len() == self.file.feature_names.len()
                && features.values.iter().all(|v| v.is_finite()),
            "invalid feature vector"
        );
        ensure!(
            features.area_mm2.is_finite()
                && features.area_mm2 > 0.0
                && features.width_mm.is_finite()
                && features.width_mm > 0.0
                && features.piece_count > 0,
            "invalid physical totals"
        );
        let mut value = self.file.intercept;
        for tree in &self.file.trees {
            let mut at = 0;
            loop {
                let (f, t, l, r, v) = tree[at];
                if f == -1 {
                    value += v;
                    break;
                }
                at = if features.values[f as usize] <= t {
                    l
                } else {
                    r
                };
            }
        }
        ensure!(value.is_finite(), "prediction overflow");
        let efficiency = value.clamp(0.01, 1.0);
        let interval = [
            (efficiency - self.file.residual_q90).max(0.01),
            (efficiency + self.file.residual_q90).min(1.0),
        ];
        let ideal = features.area_mm2 / features.width_mm;
        let length_mm = ideal / efficiency;
        let length_interval_mm = [ideal / interval[1], ideal / interval[0]];
        ensure!(
            length_interval_mm.iter().all(|v| v.is_finite()),
            "length overflow"
        );
        let outside_training_range = features
            .values
            .iter()
            .enumerate()
            .filter(|&(i, v)| {
                *v < self.file.feature_min[i] - 1e-9 || *v > self.file.feature_max[i] + 1e-9
            })
            .map(|(i, _)| self.file.feature_names[i].clone())
            .collect();
        Ok(Estimate {
            efficiency,
            efficiency_interval: interval,
            length_mm,
            length_interval_mm,
            outside_training_range,
        })
    }
    /// Estimate a supported job. Unsupported garment constraints return an error.
    pub fn estimate_job(&self, job: &Job) -> Result<Estimate> {
        self.predict(&features_from_job(job)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_model_matches_independent_training_predictions() -> Result<()> {
        #[derive(Deserialize)]
        struct Golden {
            features: Features,
            prediction: f64,
        }
        let cases: Vec<Golden> = serde_json::from_str(include_str!("../models/golden.json"))?;
        let model = Model::bundled()?;
        assert_eq!(cases.len(), 12);
        assert!(model.provenance().contains("7030786"));
        for case in cases {
            assert!((model.predict(&case.features)?.efficiency - case.prediction).abs() < 1e-12);
        }
        Ok(())
    }
    fn tiny() -> ModelFile {
        ModelFile {
            format: "beta-marker-learn/1".into(),
            feature_names: feature_names(),
            intercept: 0.7,
            trees: vec![vec![
                (0, 1.0, 1, 2, 0.0),
                (-1, 0.0, 0, 0, 0.1),
                (-1, 0.0, 0, 0, -0.1),
            ]],
            residual_q90: 0.05,
            feature_min: vec![-10.0; feature_names().len()],
            feature_max: vec![10.0; feature_names().len()],
            provenance: "unit test".into(),
            license: "Apache-2.0".into(),
        }
    }
    #[test]
    fn threshold_and_interval_direction() -> Result<()> {
        let m = Model::from_json(&serde_json::to_string(&tiny())?)?;
        let mut f = Features {
            values: vec![0.0; feature_names().len()],
            area_mm2: 800.0,
            width_mm: 10.0,
            piece_count: 1,
        };
        f.values[0] = 1.0;
        let p = m.predict(&f)?;
        assert!((p.efficiency - 0.8).abs() < 1e-12);
        assert!(p.length_interval_mm[0] < p.length_mm && p.length_mm < p.length_interval_mm[1]);
        f.values[0] = 1.00001;
        assert!((m.predict(&f)?.efficiency - 0.6).abs() < 1e-12);
        f.values[1] = f64::NAN;
        assert!(m.predict(&f).is_err());
        Ok(())
    }
    #[test]
    fn corrupt_models_are_rejected() -> Result<()> {
        let mut f = tiny();
        f.trees[0][0].2 = 0;
        assert!(Model::from_json(&serde_json::to_string(&f)?).is_err());
        let mut f = tiny();
        f.feature_names.reverse();
        assert!(Model::from_json(&serde_json::to_string(&f)?).is_err());
        let mut f = tiny();
        f.trees[0][0].0 = 500;
        assert!(Model::from_json(&serde_json::to_string(&f)?).is_err());
        Ok(())
    }
}
