//! Pair pre-placement, empty-space filling and short strategy races.
//!
//! All optimization uses the public `beta_marker_nest::nest` entry point.
//! Each successful return passes the independent core validator. Unsupported
//! garment-rule combinations keep the original job and use plain nesting.
mod cluster;
mod fill;
mod geometry;
#[cfg(test)]
mod tests;

use anyhow::{Result, ensure};
use beta_marker_core::{Job, Layout, MarkerResult, Nap};
use beta_marker_nest::{NestOptions, nest};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

/// Search strategy. Experimental variants remain explicit until paired trials
/// justify changing the production default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Strategy {
    /// Use plain nesting until an experimental strategy wins confirmation trials.
    #[default]
    Auto,
    /// Use the public engine without transforming the job.
    Plain,
    /// Experimentally race plain, clusters and two-stage search; refine the best.
    Race,
    /// Nest compact pair envelopes, expand them, then freely compress members.
    Cluster,
    /// Use a conservative connected contour around each pair, then compress.
    ClusterContour,
    /// Group exact duplicate copies without fixing any relative pair placement.
    Aggregate,
    /// GapFill's free search phases without small-piece removal or reinsertion.
    Restart,
    /// Reinsert small pieces after nesting, then freely compress all pieces.
    GapFill,
    /// Reinsert small pieces, then keep the large pieces fixed during refinement.
    GapFillFixed,
    /// Nest large pieces first, insert small pieces, then freely compress all.
    TwoStage,
    /// The two-stage search with large pieces fixed during final refinement.
    TwoStageFixed,
}

/// Nest a job with an external construction strategy and independent validation.
///
/// `opts.fixed`, warm starts and `runs` retain their engine meaning. Advanced
/// rule combinations use plain nesting rather than weakening any constraint.
/// Search phases share one wall-time allowance; engine construction and final
/// checking can overrun it, as documented by `beta_marker_nest`.
pub fn nest_clustered(job: &Job, opts: &NestOptions, strategy: Strategy) -> Result<MarkerResult> {
    let started = Instant::now();
    check_transform_input(job)?;
    ensure!(
        opts.time_limit_s.is_finite() && opts.time_limit_s > 0.0,
        "time limit must be finite and positive"
    );
    let allowance = Duration::try_from_secs_f64(opts.time_limit_s)?;
    ensure!(
        started.checked_add(allowance).is_some(),
        "time limit exceeds the platform clock range"
    );
    ensure!(opts.workers != Some(0), "workers must be greater than zero");
    ensure!(opts.runs <= 1024, "runs must be at most 1024");
    let mut normalized = opts.clone();
    normalized.seed = Some(opts.seed.unwrap_or_else(rand::random));
    let opts = &normalized;
    let issues = beta_marker_core::check_job(job);
    ensure!(issues.is_empty(), "invalid job: {issues:?}");
    if let Some(warm) = &opts.warm_start {
        ensure!(
            warm.job_name == job.name && warm.width == job.fabric.width,
            "warm start belongs to a different job or fabric width"
        );
        checked(job, warm.clone())?;
    }
    let mut result = if matches!(strategy, Strategy::Plain | Strategy::Auto)
        || !supported(job, opts)
    {
        let mut result = nest(job, opts)?;
        if !matches!(strategy, Strategy::Plain | Strategy::Auto) {
            result.warnings.push("Cluster strategy used plain nesting to preserve advanced garment rules or fixed placements.".into());
        }
        result
    } else {
        let effective = beta_marker_core::rules::effective_job(job);
        if job.fabric.shrinkage.length_pct != 0.0 || job.fabric.shrinkage.width_pct != 0.0 {
            check_transform_input(&effective)?;
            let issues = beta_marker_core::check_job(&effective);
            ensure!(issues.is_empty(), "invalid job after shrinkage: {issues:?}");
        }
        let result = match strategy {
            Strategy::Race => race(&effective, opts),
            Strategy::Cluster => cluster::run(&effective, opts),
            Strategy::ClusterContour => cluster::run_mode(&effective, opts, cluster::Mode::Contour),
            Strategy::Aggregate => cluster::run_mode(&effective, opts, cluster::Mode::Aggregate),
            Strategy::Restart => restart(&effective, opts),
            Strategy::GapFill | Strategy::GapFillFixed => {
                fill::run(&effective, opts, false, strategy == Strategy::GapFillFixed)
            }
            Strategy::TwoStage | Strategy::TwoStageFixed => {
                fill::run(&effective, opts, true, strategy == Strategy::TwoStageFixed)
            }
            Strategy::Plain | Strategy::Auto => nest(&effective, opts),
        };
        match result {
            Ok(result) => result,
            Err(error) => {
                let mut fallback = opts.clone();
                fallback.time_limit_s = remaining(opts, started).max(0.001);
                let mut result = nest(job, &fallback)?;
                result.warnings.push(format!(
                    "{strategy:?} candidate failed; plain fallback: {error:#}"
                ));
                result
            }
        }
    };
    result = checked(job, result)?;
    if (job.fabric.shrinkage.length_pct != 0.0 || job.fabric.shrinkage.width_pct != 0.0)
        && !result
            .warnings
            .iter()
            .any(|w| w.starts_with("Shrinkage applied:"))
    {
        result
            .warnings
            .push("Shrinkage applied: placements refer to rules::effective_job(job).".into());
    }
    result.seed = opts.seed;
    result.runtime_s = started.elapsed().as_secs_f64();
    Ok(result)
}

fn check_transform_input(job: &Job) -> Result<()> {
    use anyhow::Context;
    use geo::Validation;
    ensure!(
        job.fabric.width.is_finite() && job.fabric.width <= f64::from(f32::MAX),
        "fabric width exceeds engine precision"
    );
    ensure!(
        job.spacing.piece_gap.is_finite() && job.spacing.edge_gap.is_finite(),
        "gaps must be finite"
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
        [
            job.fabric.shrinkage.length_pct,
            job.fabric.shrinkage.width_pct
        ]
        .iter()
        .all(|v| v.is_finite() && *v > -100.0),
        "shrinkage must be finite and greater than -100 percent"
    );
    if let Some(order) = &job.order {
        order
            .sizes
            .values()
            .try_fold(0u32, |sum, &n| sum.checked_add(n))
            .context("bundle count overflows")?;
    }
    for piece in &job.pieces {
        piece
            .quantity
            .checked_add(piece.mirror_quantity)
            .and_then(|q| q.checked_mul(beta_marker_core::rules::multiplier(job, piece)))
            .context("piece quantity overflows")?;
        ensure!(piece.buffer.is_finite(), "piece buffer must be finite");
        ensure!(
            piece
                .grain
                .is_none_or(|g| g.iter().flatten().all(|v| v.is_finite())),
            "grain coordinates must be finite"
        );
        ensure!(
            piece
                .tilt_deg
                .is_none_or(|v| v.is_finite() && (0.0..=180.0).contains(&v)),
            "piece tilt must be finite and between 0 and 180 degrees"
        );
        ensure!(
            piece
                .outline
                .iter()
                .flatten()
                .all(|v| (*v as f32).is_finite()),
            "outline exceeds engine precision"
        );
        let mut points = piece.outline.clone();
        points.dedup();
        if points.len() > 1 && points.first() == points.last() {
            points.pop();
        }
        ensure!(
            geometry::polygon(&points).is_valid(),
            "piece '{}' needs a simple polygon without crossing edges or spikes",
            piece.id
        );
    }
    Ok(())
}

fn supported(job: &Job, opts: &NestOptions) -> bool {
    opts.fixed.is_empty()
        && job.fabric.layout == Layout::Open
        && job.fabric.nap != Nap::OneWayPerBundle
        && !job.fabric.allow_flip
        && job.fabric.defects.is_empty()
        && job.fabric.shade.is_none()
        && job.fabric.repeat.is_none()
        && job.match_rules.is_empty()
        && job
            .pieces
            .iter()
            .all(|p| p.block_margin.is_none() && p.flip != Some(true))
}

pub(crate) fn checked(job: &Job, mut result: MarkerResult) -> Result<MarkerResult> {
    let report = beta_marker_core::validate(job, &result);
    ensure!(
        report.ok,
        "cluster result failed validation: {:?}",
        report.errors
    );
    result.length = report.length;
    result.piece_area = report.piece_area;
    result.efficiency = report.efficiency;
    result.useful_piece_area = report.useful_piece_area;
    result.useful_yield = report.useful_yield;
    result.surplus_copies = report.surplus_copies;
    Ok(result)
}

pub(crate) fn remaining(opts: &NestOptions, started: Instant) -> f64 {
    (opts.time_limit_s - started.elapsed().as_secs_f64()).max(0.0)
}

pub(crate) fn refine(
    job: &Job,
    opts: &NestOptions,
    result: MarkerResult,
    seconds: f64,
    fixed: bool,
) -> Result<MarkerResult> {
    let result = checked(job, result)?;
    if seconds < 0.01 {
        return Ok(result);
    }
    let mut options = opts.clone();
    options.time_limit_s = seconds;
    options.warm_start = Some(result.clone());
    if fixed {
        let small = fill::small_ids(job);
        options.fixed = result
            .placements
            .iter()
            .filter(|p| !small.contains(&p.piece_id))
            .cloned()
            .collect();
    }
    match nest(job, &options) {
        Ok(candidate) if candidate.length < result.length => checked(job, candidate),
        Ok(_) => Ok(result),
        Err(error) => {
            let mut retained = result;
            retained.warnings.push(format!(
                "Retained valid incumbent after refinement failed: {error:#}"
            ));
            Ok(retained)
        }
    }
}

fn restart(job: &Job, opts: &NestOptions) -> Result<MarkerResult> {
    if fill::small_ids(job).is_empty() {
        return nest(job, opts);
    }
    let started = Instant::now();
    let mut initial = opts.clone();
    initial.time_limit_s *= 0.55;
    let candidate = nest(job, &initial)?;
    let mut result = refine(job, opts, candidate, remaining(opts, started), false)?;
    result
        .warnings
        .push("Two free warm-start phases; the same seed is used in both phases.".into());
    Ok(result)
}

fn race(job: &Job, opts: &NestOptions) -> Result<MarkerResult> {
    let started = Instant::now();
    let mut best = opts.warm_start.clone();
    let mut failures = Vec::new();
    let mut candidates = Vec::new();
    // The size rule is fixed before benchmarking. Large jobs give construction
    // more time; tiny jobs avoid paying for three engine initializations.
    let count: u64 = beta_marker_core::demands(job)
        .iter()
        .map(|d| u64::from(d.count))
        .sum();
    if count < 4 || opts.time_limit_s < 0.5 {
        return nest(job, opts);
    }
    let fraction = if count > 150 { 0.24 } else { 0.20 };
    for strategy in [Strategy::Plain, Strategy::Cluster, Strategy::TwoStage] {
        let seconds = (opts.time_limit_s * fraction).min(remaining(opts, started));
        if seconds < 0.01 {
            break;
        }
        let mut options = opts.clone();
        options.time_limit_s = seconds;
        // Candidates are independent. A plain warm start would mask whether
        // construction strategies contribute a useful starting arrangement.
        options.warm_start = None;
        let candidate = match strategy {
            Strategy::Plain => nest(job, &options),
            Strategy::Cluster => cluster::run(job, &options),
            _ => fill::run(job, &options, true, false),
        }
        .and_then(|r| checked(job, r));
        if let Ok(result) = &candidate {
            candidates.push(format!("{strategy:?}: {:.4}%", result.efficiency * 100.0));
        }
        match candidate {
            Ok(result) if best.as_ref().is_none_or(|b| result.length < b.length) => {
                best = Some(result)
            }
            Err(error) => failures.push(format!("{strategy:?}: {error:#}")),
            _ => {}
        }
    }
    let best =
        best.ok_or_else(|| anyhow::anyhow!("strategy race failed: {}", failures.join("; ")))?;
    let mut result = refine(job, opts, best, remaining(opts, started), false)?;
    result.engine = format!("cluster race / {}", result.engine);
    result.warnings.push(format!(
        "Raced plain, cluster and two-stage construction; {} failed candidates.",
        failures.len()
    ));
    result
        .warnings
        .push(format!("Race candidates: {}.", candidates.join(", ")));
    result.warnings.extend(failures);
    Ok(result)
}
