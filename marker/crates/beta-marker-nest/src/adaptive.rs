//! Measured two-arm portfolio with a protected baseline incumbent.
use crate::{
    ConstructionOrder, Instant, NestOptions, NestProgress, PortfolioStrategy, SearchTuning, v02,
};
use anyhow::{Result, ensure};
use beta_marker_core::{Job, Layout, MarkerResult};

fn arms(job: &Job, base: &SearchTuning) -> [SearchTuning; 2] {
    let mut baseline = base.clone();
    baseline.portfolio = PortfolioStrategy::Parallel;
    let mut alternative = baseline.clone();
    if job.fabric.layout != Layout::Open {
        alternative.mirror_variant = (base.mirror_variant + 1) % 5;
    } else if !job.fabric.defects.is_empty() {
        alternative.obstacle_proposals = false;
    } else {
        alternative.construction_order = ConstructionOrder::Reverse;
        // Fixed-only jobs also compare the direct obstacle packer.
        alternative.obstacle_proposals = false;
    }
    [baseline, alternative]
}

pub(crate) fn nest(
    job: &Job,
    opts: &NestOptions,
    callback: &mut dyn FnMut(NestProgress),
    tuning: &SearchTuning,
) -> Result<MarkerResult> {
    ensure!(
        opts.time_limit_s.is_finite() && opts.time_limit_s > 0.,
        "time limit must be finite and positive"
    );
    std::time::Duration::try_from_secs_f64(opts.time_limit_s)?;
    let arms = arms(job, tuning);
    // Tiny rounds should continue their existing engine, not pay three setup costs.
    if opts.time_limit_s < 12.0 {
        return v02::nest_tuned(job, opts, callback, &arms[0]);
    }
    let started = Instant::now();
    let seed = opts.seed.unwrap_or_else(rand::random);
    let mut best: Option<MarkerResult> = None;
    let mut best_arm = 0;
    let mut reported = f64::INFINITY;
    let mut stages = 0;
    let mut failures = 0;
    for stage in 0..3 {
        let remaining = opts.time_limit_s - started.elapsed().as_secs_f64();
        if remaining <= 0.05 && best.is_some() {
            break;
        }
        let arm = if stage < 2 { stage } else { best_arm };
        let mut options = opts.clone();
        options.time_limit_s = if stage == 0 {
            opts.time_limit_s * 0.70
        } else if stage == 1 {
            (opts.time_limit_s * 0.15).min(remaining)
        } else {
            remaining
        }
        .max(0.001);
        options.seed = Some(seed.wrapping_add(stage as u64));
        if stage > 0 {
            options.runs = 1;
        }
        // A fresh layered probe must be free to try different layer allocations.
        options.warm_start =
            if stage == 1 && job.fabric.layout != Layout::Open && opts.fixed.is_empty() {
                opts.warm_start.clone()
            } else {
                best.clone().or_else(|| opts.warm_start.clone())
            };
        let result = v02::nest_tuned(
            job,
            &options,
            &mut |event| {
                if event.length < reported && beta_marker_core::validate(job, &event.result).ok {
                    reported = event.length;
                    callback(NestProgress {
                        seconds: started.elapsed().as_secs_f64(),
                        ..event
                    });
                }
            },
            &arms[arm],
        );
        stages += 1;
        match result {
            Ok(result) => {
                let report = beta_marker_core::validate(job, &result);
                ensure!(report.ok, "Invalid adaptive candidate: {:?}", report.errors);
                if best.as_ref().is_none_or(|old| result.length < old.length) {
                    best_arm = arm;
                    best = Some(result);
                }
            }
            Err(error) if best.is_none() && stage == 0 => return Err(error),
            Err(_) => failures += 1,
        }
    }
    let mut best = best.ok_or_else(|| anyhow::anyhow!("No complete adaptive marker found"))?;
    best.runtime_s = started.elapsed().as_secs_f64();
    best.warnings.push(format!("Adaptive search: {stages} stages; selected arm {best_arm}; {failures} failed stages. Baseline used 70% of the requested budget; no full-budget baseline dominance is claimed."));
    Ok(best)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn invalid_adaptive_budgets_fail_before_search() {
        let job: Job = serde_json::from_str(r#"{"format":"beta-marker.job/v1","name":"Adaptive input check","fabric":{"width":100},"pieces":[{"id":"rect","name":"Rectangle","quantity":1,"outline":[[0,0],[20,0],[20,10],[0,10]]}]}"#).unwrap();
        assert!(beta_marker_core::check_job(&job).is_empty());
        for seconds in [0., -1., f64::NAN, f64::INFINITY] {
            let options = NestOptions {
                time_limit_s: seconds,
                ..Default::default()
            };
            let mut events = 0;
            assert!(crate::nest_adaptive_with_progress(&job, &options, |_| events += 1).is_err());
            assert_eq!(events, 0);
        }
    }
    #[test]
    fn obstacle_and_layer_probes_keep_independent_policies() {
        let mut job = Job::default();
        job.fabric.layout = Layout::Folded;
        let choices = arms(&job, &SearchTuning::default());
        assert_ne!(choices[0].mirror_variant, choices[1].mirror_variant);
        job.fabric.layout = Layout::Open;
        let choices = arms(&job, &SearchTuning::default());
        assert!(choices[0].obstacle_proposals);
        assert!(!choices[1].obstacle_proposals);
    }
}
