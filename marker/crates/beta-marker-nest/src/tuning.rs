//! Optional controls for measured search experiments. Existing callers keep their defaults.
use crate::Instant;
use crate::{Job, MarkerResult, NestOptions, NestProgress, v02};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

/// How independent seeds share a search budget.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PortfolioStrategy {
    /// Preserve a long baseline, probe a job-class alternative, then focus on the winner.
    Adaptive,
    /// Existing concurrent portfolio, dividing workers between runs.
    #[default]
    Parallel,
    /// Full-worker restarts; every later run starts from the best valid marker.
    Sequential,
    /// Spend 30% on independent probes, then refine the best with all workers.
    RaceThenFocus,
}

/// Change item enumeration before Sparrow's initial construction.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstructionOrder {
    /// Preserve source order.
    #[default]
    Input,
    /// Reverse source order.
    Reverse,
    /// Shuffle deterministically using the run seed.
    Shuffle,
}

/// Sparrow parameters from its public API; defaults preserve the previous search.
/// These options are separate from NestOptions to preserve existing Rust callers.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SearchTuning {
    /// Allow obstacle-free proposals followed by exact obstacle repair.
    pub obstacle_proposals: bool,
    /// Fraction of time used for compression, between zero and one inclusive.
    pub compression_share: f64,
    /// Exploration length reduction per feasibility attempt.
    pub exploration_shrink: f64,
    /// Spread of the exploration solution-pool selection distribution.
    pub pool_stddev: f64,
    /// Uniform samples over the whole container for each placement search.
    pub container_samples: usize,
    /// Samples near the current placement.
    pub focused_samples: usize,
    /// Number of sampled positions refined by coordinate descent.
    pub coordinate_descents: usize,
    /// Portfolio scheduling policy; uses NestOptions.runs.
    pub portfolio: PortfolioStrategy,
    /// Initial item enumeration variant.
    pub construction_order: ConstructionOrder,
    /// For >=150 placements, use at least 40% compression. Experimental.
    pub adaptive_compression: bool,
    /// Initial handedness allocation (0..4); per-piece permissions still apply.
    pub mirror_variant: usize,
}

impl Default for SearchTuning {
    fn default() -> Self {
        Self {
            obstacle_proposals: true,
            compression_share: 0.2,
            exploration_shrink: 0.001,
            pool_stddev: 0.25,
            container_samples: 50,
            focused_samples: 25,
            coordinate_descents: 3,
            portfolio: PortfolioStrategy::Parallel,
            construction_order: ConstructionOrder::Input,
            adaptive_compression: false,
            mirror_variant: 0,
        }
    }
}

/// Nest using explicit experimental search controls. Each accepted result is independently checked.
pub fn nest_with_tuning(
    job: &Job,
    opts: &NestOptions,
    tuning: &SearchTuning,
) -> Result<MarkerResult> {
    nest_with_tuning_and_progress(job, opts, tuning, &mut |_| {})
}

pub(crate) fn nest_with_tuning_and_progress(
    job: &Job,
    opts: &NestOptions,
    tuning: &SearchTuning,
    callback: &mut dyn FnMut(NestProgress),
) -> Result<MarkerResult> {
    ensure!(
        tuning.compression_share.is_finite() && (0.0..=1.0).contains(&tuning.compression_share),
        "compression share must be between zero and one"
    );
    ensure!(
        tuning.exploration_shrink.is_finite()
            && (0.0..1.0).contains(&tuning.exploration_shrink)
            && tuning.exploration_shrink > 0.0,
        "exploration shrink must be between zero and one, excluding endpoints"
    );
    ensure!(
        tuning.pool_stddev.is_finite() && tuning.pool_stddev > 0.0,
        "pool standard deviation must be positive"
    );
    ensure!(
        (1..=10000).contains(&tuning.container_samples)
            && tuning.focused_samples <= 10000
            && (1..=100).contains(&tuning.coordinate_descents),
        "sample counts exceed supported limits"
    );
    ensure!(
        tuning.mirror_variant <= 4,
        "mirror variant must be between zero and four"
    );
    ensure!(
        (tuning.exploration_shrink as f32) > 0.0
            && (tuning.pool_stddev as f32).is_finite()
            && (tuning.pool_stddev as f32) > 0.0,
        "search parameters must fit engine precision"
    );
    let started = Instant::now();
    let mut tuning = tuning.clone();
    if tuning.adaptive_compression
        && job.pieces.iter().fold(0_u64, |sum, p| {
            sum.saturating_add(
                (u64::from(p.quantity) + u64::from(p.mirror_quantity))
                    .saturating_mul(u64::from(beta_marker_core::rules::multiplier(job, p))),
            )
        }) >= 150
    {
        tuning.compression_share = tuning.compression_share.max(0.4);
    }
    if tuning.portfolio == PortfolioStrategy::Adaptive {
        return crate::adaptive::nest(job, opts, callback, &tuning);
    }
    if tuning.portfolio == PortfolioStrategy::Parallel || opts.runs <= 1 {
        return v02::nest_tuned(job, opts, callback, &tuning);
    }
    ensure!(
        (1..=1024).contains(&opts.runs),
        "runs must be between 1 and 1024"
    );
    ensure!(
        opts.time_limit_s.is_finite() && opts.time_limit_s > 0.0,
        "time limit must be finite and positive"
    );
    let runs = opts.runs;
    let seed = opts.seed.unwrap_or_else(rand::random);
    let mut best: Option<MarkerResult> = None;
    let mut failures = Vec::new();
    let race = tuning.portfolio == PortfolioStrategy::RaceThenFocus;
    let stages = runs + u32::from(race);
    let mut best_variant = tuning.mirror_variant;
    let mut reported = f64::INFINITY;
    for stage in 0..stages {
        let mut options = opts.clone();
        options.runs = 1;
        options.seed = Some(seed.wrapping_add(u64::from(stage).wrapping_mul(0x9e3779b97f4a7c15)));
        options.time_limit_s = if race && stage < runs {
            opts.time_limit_s * 0.3 / f64::from(runs)
        } else if race {
            (opts.time_limit_s - started.elapsed().as_secs_f64()).max(0.001)
        } else {
            (opts.time_limit_s - started.elapsed().as_secs_f64()).max(0.001)
                / f64::from(stages - stage)
        };
        options.warm_start = if race && stage < runs {
            opts.warm_start.clone()
        } else {
            best.clone().or_else(|| opts.warm_start.clone())
        };
        let mut stage_tuning = tuning.clone();
        stage_tuning.mirror_variant = if race && stage == runs {
            best_variant
        } else {
            (tuning.mirror_variant + stage as usize) % 5
        };
        let result = v02::nest_tuned(
            job,
            &options,
            &mut |progress: NestProgress| {
                if progress.length < reported {
                    reported = progress.length;
                    callback(NestProgress {
                        seconds: started.elapsed().as_secs_f64(),
                        length: progress.length,
                        result: progress.result,
                    });
                }
            },
            &stage_tuning,
        );
        match result {
            Ok(result) if best.as_ref().is_none_or(|old| result.length < old.length) => {
                if result.length < reported {
                    reported = result.length;
                    callback(NestProgress {
                        seconds: started.elapsed().as_secs_f64(),
                        length: result.length,
                        result: result.clone(),
                    });
                }
                best = Some(result);
                best_variant = stage_tuning.mirror_variant;
            }
            Err(error) => failures.push(error.to_string()),
            _ => {}
        }
    }
    let mut result = best.ok_or_else(|| {
        anyhow::anyhow!("all sequential searches failed: {}", failures.join("; "))
    })?;
    // Do not return an unchecked incumbent if all runs failed before validation.
    let report = beta_marker_core::validate(job, &result);
    ensure!(
        report.ok,
        "invalid sequential incumbent: {:?}",
        report.errors
    );
    result.runtime_s = started.elapsed().as_secs_f64();
    result.warnings.push(format!(
        "Search schedule: {:?}, {runs} probes/restarts, {} failed stages.",
        tuning.portfolio,
        failures.len()
    ));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restart_progress_uses_one_clock_and_reports_improvements() -> Result<()> {
        let job = Job {
            format: beta_marker_core::JOB_FORMAT.into(),
            fabric: beta_marker_core::Fabric {
                width: 100.0,
                ..Default::default()
            },
            pieces: vec![beta_marker_core::Piece {
                id: "panel".into(),
                quantity: 3,
                outline: vec![[0.0, 0.0], [20.0, 0.0], [20.0, 10.0], [0.0, 10.0]],
                ..Default::default()
            }],
            ..Default::default()
        };
        let options = NestOptions {
            time_limit_s: 0.1,
            seed: Some(7),
            workers: Some(2),
            runs: 2,
            ..Default::default()
        };
        for portfolio in [
            PortfolioStrategy::Sequential,
            PortfolioStrategy::RaceThenFocus,
        ] {
            let tuning = SearchTuning {
                portfolio,
                ..Default::default()
            };
            let mut events = Vec::new();
            let result =
                nest_with_tuning_and_progress(&job, &options, &tuning, &mut |p| events.push(p))?;
            assert!(beta_marker_core::validate(&job, &result).ok);
            assert!(!events.is_empty());
            assert!(
                events
                    .windows(2)
                    .all(|p| p[0].seconds <= p[1].seconds && p[1].length < p[0].length)
            );
            assert!(events.iter().all(|p| p.seconds <= result.runtime_s));
        }
        Ok(())
    }

    #[test]
    fn adaptive_count_does_not_overflow_before_input_validation() {
        let job = Job {
            format: beta_marker_core::JOB_FORMAT.into(),
            fabric: beta_marker_core::Fabric {
                width: 100.0,
                ..Default::default()
            },
            order: Some(beta_marker_core::Order {
                sizes: std::collections::BTreeMap::from([("M".into(), 2)]),
            }),
            pieces: vec![beta_marker_core::Piece {
                id: "panel".into(),
                size: Some("M".into()),
                quantity: u32::MAX,
                outline: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
                ..Default::default()
            }],
            ..Default::default()
        };
        let tuning = SearchTuning {
            adaptive_compression: true,
            ..Default::default()
        };
        assert!(nest_with_tuning(&job, &NestOptions::default(), &tuning).is_err());
    }

    #[test]
    fn invalid_search_parameters_fail_before_the_engine() {
        let job = Job::default();
        for value in [f64::NAN, -0.1, 1.1] {
            let tuning = SearchTuning {
                compression_share: value,
                ..Default::default()
            };
            assert!(nest_with_tuning(&job, &NestOptions::default(), &tuning).is_err());
        }
        assert!(serde_json::from_str::<SearchTuning>(r#"{"compression_shrae":0.3}"#).is_err());
    }
}
