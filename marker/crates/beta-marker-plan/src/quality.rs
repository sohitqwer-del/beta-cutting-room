//! Compare ratios on a calibrated budget, then retain the feasible incumbent.
use crate::{Candidate, CutOrder, PlanOptions, optimizer, ratio_map};
use anyhow::{Result, ensure};
use beta_marker_core::{MarkerResult, Order};
use std::collections::BTreeSet;

// A third-party worker panic must not erase an independently validated marker.
// The complete engine invocation is discarded on unwind; no partial geometry escapes.
fn guard<T>(action: impl FnOnce() -> Result<T>) -> Result<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(action)).map_err(|payload| {
        let message = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or("worker panic");
        anyhow::anyhow!("Nesting engine panicked; discarded this trial: {message}")
    })?
}
pub(crate) fn checked_nest(
    job: &beta_marker_core::Job,
    options: &beta_marker_nest::NestOptions,
) -> Result<MarkerResult> {
    guard(|| beta_marker_match::nest_with_rules(job, options))
}

pub(crate) fn refine(
    order: &CutOrder,
    opts: &PlanOptions,
    sizes: &[String],
    c: &mut Candidate,
    seconds: f64,
    engine: &mut crate::NestExecutor<'_>,
) -> Result<()> {
    let mut job = order.job.clone();
    job.order = Some(Order {
        sizes: ratio_map(sizes, &c.ratio),
    });
    let old = c
        .result
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Cannot refine an unmeasured ratio."))?;
    let result = engine(
        &job,
        &beta_marker_nest::NestOptions {
            time_limit_s: seconds,
            seed: Some(opts.seed),
            workers: Some(2),
            warm_start: Some(old.clone()),
            ..Default::default()
        },
    )?;
    let report = beta_marker_core::validate(&job, &result);
    ensure!(
        report.ok,
        "Refined marker failed validation: {:?}",
        report.errors
    );
    if report.length < old.length {
        c.length = report.length;
        c.result = Some(MarkerResult {
            length: report.length,
            piece_area: report.piece_area,
            efficiency: report.efficiency,
            ..result
        });
    }
    Ok(())
}
fn count(c: &Candidate) -> u32 {
    c.ratio.iter().sum()
}
fn cost(candidates: &[Candidate], solution: &optimizer::Solution, opts: &PlanOptions) -> f64 {
    solution
        .actions
        .iter()
        .map(|(i, p)| {
            (candidates[*i].length + 2.0 * opts.end_allowance_mm) * f64::from(*p) / 1000.0
                + opts.lay_cost_m
        })
        .sum()
}
/// Piecewise linear correction between observed garment-count classes.
/// Factors never make a short measured marker worse; physical lengths stay separate.
fn correction(n: u32, samples: &[(u32, f64)]) -> f64 {
    let Some(&(low, a)) = samples.first() else {
        return 1.0;
    };
    if n <= low {
        return a;
    }
    for pair in samples.windows(2) {
        let [(low, a), (high, b)] = pair else {
            continue;
        };
        if n <= *high {
            let t = f64::from(n - low) / f64::from(high - low);
            return a + t * (b - a);
        }
    }
    samples.last().map_or(a, |s| s.1)
}
pub(super) fn select(
    order: &CutOrder,
    opts: &PlanOptions,
    sizes: &[String],
    measured: &mut [Candidate],
    demand: (&[u32], &[u32]),
    warnings: &mut Vec<String>,
    engine: &mut crate::NestExecutor<'_>,
) -> Result<optimizer::Solution> {
    let (required, upper) = demand;
    let baseline = optimizer::solve(measured, required, upper, order.max_plies, opts);
    if measured.is_empty() || baseline.actions.is_empty() {
        return Ok(baseline);
    }
    let baseline_cost = cost(measured, &baseline, opts);
    // A probe per garment-count class avoids extrapolating the single-garment
    // search gain to a medium or large marker. Prefer the shortest measured
    // length per garment within each class, never raw efficiency across sizes.
    let mut classes = std::collections::BTreeMap::<u32, usize>::new();
    for (i, candidate) in measured.iter().enumerate() {
        let n = count(candidate);
        let entry = classes.entry(n).or_insert(i);
        if candidate.length / f64::from(n) < measured[*entry].length / f64::from(n) {
            *entry = i;
        }
    }
    let probes: BTreeSet<_> = classes.into_values().collect();
    let mut samples = Vec::new();
    let mut calibrated = BTreeSet::new();
    let budget = (3.0 * opts.time_per_marker_s).min(60.0);
    for i in probes {
        let short = measured[i].length;
        match refine(order, opts, sizes, &mut measured[i], budget, engine) {
            Ok(()) => {
                samples.push((count(&measured[i]), measured[i].length / short));
                calibrated.insert(i);
            }
            Err(e) => warnings.push(format!(
                "Calibration for {:?} kept the short marker: {e:#}",
                ratio_map(sizes, &measured[i].ratio)
            )),
        }
    }
    samples.sort_by_key(|s| s.0);
    let mut scored = measured.to_vec();
    for (i, c) in scored.iter_mut().enumerate() {
        if !calibrated.contains(&i) {
            c.length *= correction(count(c), &samples);
        }
    }
    let nominated = optimizer::solve(&scored, required, upper, order.max_plies, opts);
    let shortlist: BTreeSet<_> = baseline
        .actions
        .iter()
        .chain(&nominated.actions)
        .map(|(i, _)| *i)
        .collect();
    if opts.final_time_s > 0.0 {
        for &i in &shortlist {
            // Failure cannot erase an independently validated feasible incumbent.
            if let Err(e) = refine(
                order,
                opts,
                sizes,
                &mut measured[i],
                opts.final_time_s,
                engine,
            ) {
                warnings.push(format!(
                    "Final nest for {:?} kept its validated incumbent: {e:#}",
                    ratio_map(sizes, &measured[i].ratio)
                ));
            }
        }
    }
    // Compare actual validated lengths for both plans after equal refinement.
    // Never publish calibrated estimates as fabric use or cutter geometry.
    let mut actual = measured.to_vec();
    if opts.final_time_s > 0.0 {
        for (i, c) in actual.iter_mut().enumerate() {
            if !shortlist.contains(&i) {
                c.length = f64::INFINITY;
            }
        }
    }
    let mut selected = optimizer::solve(&actual, required, upper, order.max_plies, opts);
    // A bounded re-search must also retain the explicit feasible incumbent.
    let complete = |solution: &optimizer::Solution| {
        required.iter().enumerate().all(|(k, q)| {
            solution
                .actions
                .iter()
                .map(|(i, p)| u64::from(measured[*i].ratio[k]) * u64::from(*p))
                .sum::<u64>()
                >= u64::from(*q)
        })
    };
    if complete(&baseline)
        && (!complete(&selected)
            || cost(measured, &baseline, opts) < cost(measured, &selected, opts))
    {
        selected = optimizer::Solution {
            actions: baseline.actions,
            exhausted: false,
        };
    }
    if opts.final_time_s > 0.0 {
        warnings.push("Final comparison covers the incumbent and calibrated shortlist; it does not prove optimality across every evaluated ratio.".into());
    }
    warnings.push(format!("Budget calibration ({} s per probe): {:?}; short-plan objective {:.3} m, selected actual objective {:.3} m. Final nests never replace a shorter valid marker.", budget, samples, baseline_cost, cost(measured, &selected, opts)));
    Ok(selected)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn engine_panic_becomes_an_error_instead_of_unwinding_the_plan() {
        let failure = guard::<()>(|| panic!("synthetic worker failure"));
        assert!(
            failure
                .expect_err("caught panic")
                .to_string()
                .contains("synthetic worker failure")
        );
        assert_eq!(guard(|| Ok(7)).expect("ordinary success"), 7);
    }
    #[test]
    fn calibration_respects_ratio_size_and_bounds() {
        let samples = [(1, 0.98), (6, 0.90)];
        assert_eq!(correction(0, &samples), 0.98);
        assert_eq!(correction(12, &samples), 0.90);
        assert!((correction(3, &samples) - 0.948).abs() < 1e-9);
        assert_eq!(correction(2, &[]), 1.0);
    }
}
