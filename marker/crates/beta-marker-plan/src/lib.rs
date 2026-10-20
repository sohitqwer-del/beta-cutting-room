//! Cut order planning: validated markers, bounded integer search, and fabric totals.

/// Width and ratio comparisons with explicit fabric prices and stock balances.
pub mod cost;
mod integration;
mod optimizer;
mod quality;
pub use integration::{apply_roll_group, spreading_requests};
#[cfg(test)]
mod cost_tests;
#[cfg(test)]
mod tests;

use anyhow::{Result, ensure};
use beta_marker_core::{Job, MarkerResult, Order};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::Instant;

/// Application-owned nesting executor. The planner independently validates all
/// returned geometry. A queue can add bounded work, cancellation and disk saves.
pub type NestExecutor<'a> =
    dyn FnMut(&Job, &beta_marker_nest::NestOptions) -> Result<MarkerResult> + 'a;

/// Cut order JSON identifier.
pub const CUT_ORDER_FORMAT: &str = "beta-marker.cutorder/v1";
/// Cut plan JSON identifier.
pub const CUT_PLAN_FORMAT: &str = "beta-marker.cutplan/v1";

/// Pieces, quantities, and cutting-room limits. The job's original ratio is ignored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CutOrder {
    /// Must be [`CUT_ORDER_FORMAT`].
    pub format: String,
    /// Human-readable order name.
    pub name: String,
    /// Pattern pieces and fabric rules for all sizes.
    pub job: Job,
    /// Garments required per size.
    pub quantities: BTreeMap<String, u32>,
    /// Maximum fabric plies in one lay.
    pub max_plies: u32,
    /// Maximum garments in a marker ratio.
    pub max_garments_per_marker: u32,
    /// Maximum extra garments per size, rounded down to a whole garment.
    #[serde(default)]
    pub overcut_pct: f64,
}

/// A plan with measured markers and an explicit quantity balance.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CutPlan {
    /// [`CUT_PLAN_FORMAT`].
    pub format: String,
    /// Source order name.
    pub order_name: String,
    /// Search and costing controls used for this plan.
    pub options: PlanOptions,
    /// Each entry is one spreading and cutting operation.
    pub lays: Vec<Lay>,
    /// Sum of `(marker length + two end allowances) * plies`, in metres.
    pub total_fabric_m: f64,
    /// Fabric plus the configured cost per lay, in metre equivalents.
    pub objective_m: f64,
    /// Original quantities, for comparison with planned quantities.
    pub ordered: BTreeMap<String, u32>,
    /// Garments to be cut per size.
    pub planned: BTreeMap<String, u32>,
    /// Missing garments. Empty when every ordered quantity is met.
    pub shortfall: BTreeMap<String, u32>,
    /// Extra garments, within the order's limit.
    pub overcut: BTreeMap<String, u32>,
    /// True when every ordered quantity is met.
    pub complete: bool,
    /// True when a complete plan is optimal for the evaluated ratio table.
    pub optimal_for_evaluated_markers: bool,
    /// Number of real marker evaluations (one per distinct ratio).
    pub markers_evaluated: usize,
    /// Total wall-clock runtime, including nesting.
    pub runtime_s: f64,
    /// Search limits, unavailable markers, and nesting warnings.
    pub warnings: Vec<String>,
}

/// One marker spread over a number of fabric plies.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Lay {
    /// Garments per size in one ply.
    pub ratio: BTreeMap<String, u32>,
    /// Number of fabric plies.
    pub plies: u32,
    /// Validated marker length, millimetres.
    pub marker_length_mm: f64,
    /// Fraction of the marker occupied by pattern pieces.
    pub efficiency: f64,
    /// Fabric consumed including end allowances, metres.
    pub fabric_m: f64,
    /// The validated marker used by this lay.
    #[serde(default)]
    pub result: Option<MarkerResult>,
}

/// Search and costing controls. Defaults suit a small order with four sizes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PlanOptions {
    /// Search seconds per short nest. Construction and validation add overhead.
    pub time_per_marker_s: f64,
    /// Final search seconds per selected ratio; zero disables final re-nesting.
    pub final_time_s: f64,
    /// Fabric at each end of each ply, millimetres.
    pub end_allowance_mm: f64,
    /// Cost of starting one lay, expressed as metres of fabric.
    pub lay_cost_m: f64,
    /// Initial area-to-length estimate: piece area / (width * efficiency).
    pub estimated_efficiency: f64,
    /// Most ratios to retain after ranking. Single-size ratios are always retained.
    pub max_candidates: usize,
    /// Most ratios to refine with real nesting. Must cover all ordered sizes.
    pub max_refinements: usize,
    /// Most branch-and-bound states per optimisation pass.
    pub search_nodes: usize,
    /// Reproducible base seed for nesting (wall-clock search may still vary).
    pub seed: u64,
}

impl Default for PlanOptions {
    fn default() -> Self {
        Self {
            time_per_marker_s: 20.0,
            final_time_s: 0.0,
            end_allowance_mm: 0.0,
            lay_cost_m: 0.5,
            estimated_efficiency: 0.75,
            max_candidates: 128,
            max_refinements: 16,
            search_nodes: 20_000,
            seed: 42,
        }
    }
}

#[derive(Clone)]
pub(crate) struct Candidate {
    pub ratio: Vec<u32>,
    pub length: f64,
    pub result: Option<MarkerResult>,
}

pub(crate) fn ratio_map(sizes: &[String], ratio: &[u32]) -> BTreeMap<String, u32> {
    sizes
        .iter()
        .cloned()
        .zip(ratio.iter().copied())
        .filter(|(_, n)| *n > 0)
        .collect()
}

fn validate_input(order: &CutOrder, opts: &PlanOptions) -> Result<()> {
    ensure!(
        order.format == CUT_ORDER_FORMAT,
        "unsupported cut order format"
    );
    ensure!(
        order.max_plies > 0 && order.max_garments_per_marker > 0,
        "ply and marker limits must be positive"
    );
    ensure!(
        order.quantities.len() <= 16,
        "at most 16 sizes are supported per order"
    );
    ensure!(
        order.overcut_pct.is_finite() && order.overcut_pct >= 0.0,
        "overcut_pct must be finite and nonnegative"
    );
    ensure!(
        opts.time_per_marker_s.is_finite()
            && opts.time_per_marker_s > 0.0
            && opts.time_per_marker_s <= 86_400.0,
        "time_per_marker_s must be in (0, 86400]"
    );
    ensure!(
        opts.final_time_s.is_finite() && (0.0..=86_400.0).contains(&opts.final_time_s),
        "final_time_s must be in [0, 86400]"
    );
    ensure!(
        opts.end_allowance_mm.is_finite()
            && opts.end_allowance_mm >= 0.0
            && opts.lay_cost_m.is_finite()
            && opts.lay_cost_m >= 0.0,
        "allowance and lay cost must be finite and nonnegative"
    );
    ensure!(
        opts.estimated_efficiency.is_finite()
            && opts.estimated_efficiency > 0.0
            && opts.estimated_efficiency <= 1.0,
        "estimated_efficiency must be in (0, 1]"
    );
    ensure!(
        opts.search_nodes > 0 && opts.max_candidates > 0,
        "search and candidate limits must be positive"
    );
    let n = order.quantities.values().filter(|q| **q > 0).count();
    ensure!(
        order.job.fabric.width.is_finite() && order.job.fabric.width > 0.0,
        "fabric width must be finite and positive"
    );
    ensure!(
        n == 0 || opts.max_refinements > 0,
        "A nonempty order needs at least one real refinement."
    );
    for (size, &quantity) in &order.quantities {
        let upper = f64::from(quantity) * (1.0 + order.overcut_pct / 100.0);
        ensure!(
            upper.is_finite() && upper <= f64::from(u32::MAX),
            "quantity plus overcut overflows for {size}"
        );
        if quantity > 0 {
            ensure!(
                order
                    .job
                    .pieces
                    .iter()
                    .any(|p| p.size.as_ref() == Some(size)
                        && (p.quantity > 0 || p.mirror_quantity > 0)),
                "no pattern pieces for ordered size {size}"
            );
        }
    }
    if n == 0 {
        return Ok(());
    }
    let mut job = order.job.clone();
    job.order = Some(Order {
        sizes: order
            .quantities
            .iter()
            .filter(|(_, q)| **q > 0)
            .map(|(s, _)| (s.clone(), 1))
            .collect(),
    });
    let issues = beta_marker_core::check_job(&job);
    ensure!(
        issues.is_empty(),
        "invalid pattern job: {}",
        issues
            .iter()
            .map(|i| i.message.as_str())
            .collect::<Vec<_>>()
            .join("; ")
    );
    Ok(())
}

fn generate(
    order: &CutOrder,
    opts: &PlanOptions,
    sizes: &[String],
    upper: &[u32],
) -> Result<(Vec<Candidate>, bool)> {
    let effective = beta_marker_core::rules::effective_job(&order.job);
    let areas: Vec<f64> = sizes
        .iter()
        .map(|s| {
            effective
                .pieces
                .iter()
                .filter(|p| p.size.as_ref() == Some(s))
                .map(|p| {
                    beta_marker_core::geometry::area(&p.outline)
                        * (f64::from(p.quantity) + f64::from(p.mirror_quantity))
                })
                .sum()
        })
        .collect();
    ensure!(
        areas.iter().all(|a| a.is_finite() && *a > 0.0),
        "each ordered size must have positive finite piece area"
    );
    let mut ratios = Vec::new();
    let mut truncated = false;
    fn visit(
        i: usize,
        left: u32,
        upper: &[u32],
        ratio: &mut [u32],
        out: &mut Vec<Vec<u32>>,
        truncated: &mut bool,
    ) {
        if out.len() >= 200_000 {
            *truncated = true;
            return;
        }
        if i == ratio.len() {
            if ratio.iter().any(|n| *n > 0) {
                out.push(ratio.to_vec());
            }
            return;
        }
        for n in 0..=left.min(upper[i]) {
            ratio[i] = n;
            visit(i + 1, left - n, upper, ratio, out, truncated);
            if *truncated {
                break;
            }
        }
        ratio[i] = 0;
    }
    visit(
        0,
        order.max_garments_per_marker,
        upper,
        &mut vec![0; sizes.len()],
        &mut ratios,
        &mut truncated,
    );
    // Guarantee a repair marker for every size even if enumeration was capped.
    for i in 0..sizes.len() {
        let mut r = vec![0; sizes.len()];
        r[i] = 1;
        ratios.push(r);
    }
    ratios.sort();
    ratios.dedup();
    let demand: Vec<f64> = sizes
        .iter()
        .map(|s| f64::from(order.quantities[s]))
        .collect();
    let total: f64 = demand.iter().sum();
    let rank = |r: &[u32]| {
        let count: f64 = r.iter().map(|n| f64::from(*n)).sum();
        let imbalance: f64 = r
            .iter()
            .zip(&demand)
            .map(|(n, q)| (f64::from(*n) / count - q / total).abs())
            .sum();
        imbalance + 0.15 / count
    };
    ratios.sort_by(|a, b| rank(a).total_cmp(&rank(b)).then_with(|| a.cmp(b)));
    let pruned = ratios.len() > opts.max_candidates;
    let singles: Vec<_> = ratios
        .iter()
        .filter(|r| r.iter().sum::<u32>() == 1)
        .cloned()
        .collect();
    ratios.retain(|r| r.iter().sum::<u32>() != 1);
    ratios.truncate(opts.max_candidates.saturating_sub(singles.len()));
    ratios.extend(singles);
    ratios.sort_by(|a, b| rank(a).total_cmp(&rank(b)).then_with(|| a.cmp(b)));
    let candidates = ratios
        .into_iter()
        .map(|ratio| {
            let area: f64 = ratio
                .iter()
                .zip(&areas)
                .map(|(n, a)| f64::from(*n) * a)
                .sum();
            Candidate {
                ratio,
                length: area / (order.job.fabric.width * opts.estimated_efficiency),
                result: None,
            }
        })
        .collect();
    Ok((candidates, truncated || pruned))
}

/// Plan a cut order. Only independently validated real nests can appear in the result.
/// Search is exact for the evaluated table when it finishes within `search_nodes`.
/// Candidate pruning and bounded refinement make the overall planner a heuristic.
pub fn plan(order: &CutOrder, opts: &PlanOptions) -> Result<CutPlan> {
    plan_impl(
        order,
        opts,
        &[],
        |_| Ok(()),
        &mut quality::checked_nest,
        false,
    )
}

/// Plan with a checked partial-plan checkpoint after every measured ratio.
/// Returning an error from the callback stops between engine calls. Callbacks
/// run synchronously and may persist a checkpoint or inspect cancellation.
pub fn plan_with_progress(
    order: &CutOrder,
    opts: &PlanOptions,
    progress: impl FnMut(&CutPlan) -> Result<()>,
) -> Result<CutPlan> {
    plan_ratios_with_progress(order, opts, &[], progress)
}

/// Plan using an explicit menu of ratios. An empty menu uses automatic ratios.
/// No single-size repair ratios are added to a nonempty menu.
pub fn plan_ratios_with_progress(
    order: &CutOrder,
    opts: &PlanOptions,
    ratios: &[BTreeMap<String, u32>],
    progress: impl FnMut(&CutPlan) -> Result<()>,
) -> Result<CutPlan> {
    plan_ratios_with_engine(order, opts, ratios, progress, &mut quality::checked_nest)
}

/// Planner with an application-owned nesting executor. A queue can slice and
/// checkpoint engine work while preserving the planner's independent validation.
pub fn plan_ratios_with_engine(
    order: &CutOrder,
    opts: &PlanOptions,
    ratios: &[BTreeMap<String, u32>],
    progress: impl FnMut(&CutPlan) -> Result<()>,
    engine: &mut NestExecutor<'_>,
) -> Result<CutPlan> {
    plan_impl(order, opts, ratios, progress, engine, true)
}

fn plan_impl(
    order: &CutOrder,
    opts: &PlanOptions,
    ratios: &[BTreeMap<String, u32>],
    mut progress: impl FnMut(&CutPlan) -> Result<()>,
    engine: &mut NestExecutor<'_>,
    checkpoints: bool,
) -> Result<CutPlan> {
    validate_input(order, opts)?;
    if ratios.is_empty() {
        let n = order.quantities.values().filter(|q| **q > 0).count();
        ensure!(
            opts.max_refinements >= n && opts.max_candidates >= n,
            "Automatic candidate/refinement limits must cover all ordered sizes."
        );
    }
    let started = Instant::now();
    let mut sizes: Vec<_> = order
        .quantities
        .iter()
        .filter(|(_, q)| **q > 0)
        .map(|(s, _)| s.clone())
        .collect();
    let size_job = Job {
        order: Some(Order {
            sizes: sizes.iter().map(|s| (s.clone(), 1)).collect(),
        }),
        ..Default::default()
    };
    sizes = beta_marker_core::rules::bundle_sizes(&size_job)
        .into_iter()
        .flatten()
        .collect();
    let required: Vec<_> = sizes.iter().map(|s| order.quantities[s]).collect();
    let upper: Vec<_> = required
        .iter()
        .map(|q| (f64::from(*q) * (1.0 + order.overcut_pct / 100.0)).floor() as u32)
        .collect();
    let mut warnings = Vec::new();
    if sizes.is_empty() {
        return Ok(assemble(order, opts, &sizes, &[], &[], true, warnings));
    }
    let (mut candidates, pruned) = if ratios.is_empty() {
        generate(order, opts, &sizes, &upper)?
    } else {
        let mut rows = BTreeMap::new();
        for ratio in ratios {
            ensure!(
                ratio.values().map(|n| u64::from(*n)).sum::<u64>() > 0
                    && ratio.values().map(|n| u64::from(*n)).sum::<u64>()
                        <= u64::from(order.max_garments_per_marker),
                "Ratio must contain between 1 and max_garments_per_marker garments."
            );
            ensure!(
                ratio.iter().all(|(s, n)| *n == 0 || sizes.contains(s)),
                "Ratio refers to an unordered size."
            );
            let values: Vec<u32> = sizes
                .iter()
                .map(|s| ratio.get(s).copied().unwrap_or(0))
                .collect();
            ensure!(
                values.iter().zip(&upper).all(|(n, cap)| n <= cap),
                "Ratio exceeds the order quantity limit."
            );
            let mut job = order.job.clone();
            job.order = Some(Order {
                sizes: ratio.clone(),
            });
            let effective = beta_marker_core::rules::effective_job(&job);
            let area: f64 = beta_marker_core::demands(&effective)
                .iter()
                .map(|d| {
                    beta_marker_core::geometry::area(&effective.pieces[d.piece_index].outline)
                        * f64::from(d.count)
                })
                .sum();
            rows.insert(
                values.clone(),
                Candidate {
                    ratio: values,
                    length: area / (job.fabric.width * opts.estimated_efficiency),
                    result: None,
                },
            );
        }
        (rows.into_values().collect(), false)
    };
    if pruned {
        warnings.push("Candidate ratios were pruned; global optimality is not guaranteed.".into());
    }
    let initial_lengths: Vec<_> = candidates.iter().map(|c| c.length).collect();
    let mut attempted = vec![false; candidates.len()];
    // Single-size markers provide a feasible repair route before mixed ratios are tried.
    let mut pending: Vec<usize> = candidates
        .iter()
        .enumerate()
        .filter(|(_, c)| c.ratio.iter().sum::<u32>() == 1)
        .map(|(i, _)| i)
        .collect();
    // Include the ratio closest to the complete order before estimated-table repair.
    if !candidates.is_empty() && !pending.contains(&0) {
        pending.push(0);
    }
    let mut evaluations = 0;
    while evaluations < opts.max_refinements {
        if pending.is_empty() {
            let estimated = optimizer::solve(&candidates, &required, &upper, order.max_plies, opts);
            pending = estimated
                .actions
                .iter()
                .map(|(i, _)| *i)
                .filter(|i| !attempted[*i])
                .collect();
            pending.sort_unstable();
            pending.dedup();
            if pending.is_empty() {
                if let Some(i) = attempted.iter().position(|v| !v) {
                    pending.push(i);
                } else {
                    break;
                }
            }
        }
        let i = pending.remove(0);
        if attempted[i] {
            continue;
        }
        attempted[i] = true;
        evaluations += 1;
        let mut job = order.job.clone();
        job.order = Some(Order {
            sizes: ratio_map(&sizes, &candidates[i].ratio),
        });
        let outcome = (|| -> Result<MarkerResult> {
            for p in &job.pieces {
                let m = beta_marker_core::rules::multiplier(&job, p);
                ensure!(
                    p.quantity.checked_mul(m).is_some()
                        && p.mirror_quantity.checked_mul(m).is_some(),
                    "piece count overflow: {}",
                    p.id
                );
            }
            let mut result = engine(
                &job,
                &beta_marker_nest::NestOptions {
                    time_limit_s: opts.time_per_marker_s,
                    workers: Some(2),
                    seed: Some(opts.seed.wrapping_add(i as u64)),
                    ..Default::default()
                },
            )?;
            ensure!(
                result.length.is_finite() && result.length > 0.0 && result.efficiency.is_finite(),
                "invalid marker metrics"
            );
            let report = beta_marker_core::validate(&job, &result);
            ensure!(report.ok, "marker failed independent validation");
            result.length = report.length;
            result.piece_area = report.piece_area;
            result.efficiency = report.efficiency;
            result
                .warnings
                .extend(report.warnings.into_iter().map(|w| w.message));
            Ok(result)
        })();
        match outcome {
            Ok(result) => {
                candidates[i].length = result.length;
                candidates[i].result = Some(result);
                // Keep unmeasured ratios on the same efficiency scale as real short
                // nests. Otherwise an optimistic/pessimistic initial guess changes
                // which ratios receive the limited refinement budget.
                let mut factors: Vec<_> = candidates
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.result.is_some())
                    .map(|(k, c)| c.length / initial_lengths[k])
                    .collect();
                factors.sort_by(f64::total_cmp);
                let factor = factors[factors.len() / 2];
                for (k, c) in candidates
                    .iter_mut()
                    .enumerate()
                    .filter(|(k, _)| !attempted[*k])
                {
                    c.length = initial_lengths[k] * factor;
                }
            }
            Err(e) => {
                candidates[i].length = f64::INFINITY;
                warnings.push(format!(
                    "Ratio {:?} unavailable: {e:#}",
                    job.order.map(|o| o.sizes).unwrap_or_default()
                ));
            }
        }
        if checkpoints {
            let measured: Vec<_> = candidates
                .iter()
                .filter(|c| c.result.is_some())
                .cloned()
                .collect();
            let checkpoint = optimizer::solve(&measured, &required, &upper, order.max_plies, opts);
            let mut partial = assemble(
                order,
                opts,
                &sizes,
                &measured,
                &checkpoint.actions,
                false,
                warnings.clone(),
            );
            partial.markers_evaluated = evaluations;
            partial.runtime_s = started.elapsed().as_secs_f64();
            progress(&partial)?;
        }
    }
    let mut measured: Vec<_> = candidates
        .into_iter()
        .filter(|c| c.result.is_some())
        .collect();
    let solution = quality::select(
        order,
        opts,
        &sizes,
        &mut measured,
        (&required, &upper),
        &mut warnings,
        engine,
    )?;
    if !solution.exhausted {
        warnings.push("Integer search reached its state limit; this is the best plan found, not a proof of optimality.".into());
    }
    warnings.push(format!("Evaluated {evaluations} real markers. Fabric optimality across unevaluated ratios is not guaranteed."));
    let mut plan = assemble(
        order,
        opts,
        &sizes,
        &measured,
        &solution.actions,
        solution.exhausted,
        warnings,
    );
    plan.optimal_for_evaluated_markers &= opts.final_time_s == 0.0;
    plan.markers_evaluated = evaluations;
    plan.runtime_s = started.elapsed().as_secs_f64();
    progress(&plan)?;
    Ok(plan)
}

fn assemble(
    order: &CutOrder,
    opts: &PlanOptions,
    sizes: &[String],
    candidates: &[Candidate],
    actions: &[(usize, u32)],
    exhausted: bool,
    warnings: Vec<String>,
) -> CutPlan {
    let mut out = CutPlan {
        format: CUT_PLAN_FORMAT.into(),
        order_name: order.name.clone(),
        options: opts.clone(),
        ordered: order.quantities.clone(),
        planned: order.quantities.keys().map(|s| (s.clone(), 0)).collect(),
        optimal_for_evaluated_markers: exhausted,
        warnings,
        ..Default::default()
    };
    for &(i, plies) in actions {
        let c = &candidates[i];
        for (s, n) in ratio_map(sizes, &c.ratio) {
            *out.planned.entry(s).or_default() += n * plies;
        }
        let fabric_m = (c.length + 2.0 * opts.end_allowance_mm) * f64::from(plies) / 1000.0;
        out.total_fabric_m += fabric_m;
        if let Some(result) = &c.result {
            for w in &result.warnings {
                if !out.warnings.contains(w) {
                    out.warnings.push(w.clone());
                }
            }
        }
        out.lays.push(Lay {
            ratio: ratio_map(sizes, &c.ratio),
            plies,
            marker_length_mm: c.length,
            efficiency: c.result.as_ref().map_or(0.0, |r| r.efficiency),
            fabric_m,
            result: c.result.clone(),
        });
    }
    for (s, q) in &order.quantities {
        let p = out.planned[s];
        if p < *q {
            out.shortfall.insert(s.clone(), q - p);
        }
        if p > *q {
            out.overcut.insert(s.clone(), p - q);
        }
    }
    out.complete = out.shortfall.is_empty();
    out.optimal_for_evaluated_markers &= out.complete;
    if !out.complete {
        out.warnings.push("Order could not be completed with the validated markers and search budget. Review shortfall; infeasibility is not proven unless the evaluated-table search exhausted all states.".into());
    }
    out.objective_m = out.total_fabric_m + opts.lay_cost_m * out.lays.len() as f64;
    out
}

/// Re-nest the distinct ratios of a feasible short plan, keeping every valid
/// incumbent, then recompute integer lays from actual final lengths.
pub fn refine_plan_with_progress(
    order: &CutOrder,
    opts: &PlanOptions,
    incumbent: &CutPlan,
    progress: impl FnMut(&CutPlan) -> Result<()>,
) -> Result<CutPlan> {
    refine_plan_with_engine(order, opts, incumbent, progress, &mut quality::checked_nest)
}

/// Refine a feasible plan using an application-owned bounded nesting executor.
pub fn refine_plan_with_engine(
    order: &CutOrder,
    opts: &PlanOptions,
    incumbent: &CutPlan,
    mut progress: impl FnMut(&CutPlan) -> Result<()>,
    engine: &mut NestExecutor<'_>,
) -> Result<CutPlan> {
    validate_input(order, opts)?;
    ensure!(
        incumbent.complete && incumbent.ordered == order.quantities,
        "Final refinement needs a complete plan for the same order."
    );
    let started = Instant::now();
    let mut sizes: Vec<_> = order
        .quantities
        .iter()
        .filter(|(_, q)| **q > 0)
        .map(|(s, _)| s.clone())
        .collect();
    sizes.sort_by(|a, b| {
        let a = beta_marker_core::rules::size_order_key(a);
        let b = beta_marker_core::rules::size_order_key(b);
        a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)).then(a.2.cmp(&b.2))
    });
    let required: Vec<_> = sizes.iter().map(|s| order.quantities[s]).collect();
    let upper: Vec<_> = required
        .iter()
        .map(|q| (f64::from(*q) * (1.0 + order.overcut_pct / 100.0)).floor() as u32)
        .collect();
    let mut unique = BTreeMap::new();
    for lay in &incumbent.lays {
        let result = lay
            .result
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Incumbent lay has no marker."))?;
        let mut job = order.job.clone();
        job.order = Some(Order {
            sizes: lay.ratio.clone(),
        });
        let report = beta_marker_core::validate(&job, result);
        ensure!(
            report.ok,
            "Incumbent failed independent validation: {:?}",
            report.errors
        );
        let ratio: Vec<_> = sizes
            .iter()
            .map(|s| lay.ratio.get(s).copied().unwrap_or(0))
            .collect();
        unique.entry(ratio.clone()).or_insert(Candidate {
            ratio,
            length: report.length,
            result: Some(result.clone()),
        });
    }
    let mut measured: Vec<_> = unique.into_values().collect();
    let mut warnings = incumbent.warnings.clone();
    // Each selected ratio is refined once. Publish checked progress between
    // calls so cancellation never discards earlier improvements.
    for c in &mut measured {
        if opts.final_time_s > 0.0
            && let Err(e) = quality::refine(order, opts, &sizes, c, opts.final_time_s, engine)
        {
            warnings.push(format!("Final ratio kept its validated incumbent: {e:#}"));
        }
        let mut partial = incumbent.clone();
        for lay in &mut partial.lays {
            if lay.ratio == ratio_map(&sizes, &c.ratio) {
                lay.marker_length_mm = c.length;
                lay.result = c.result.clone();
                lay.efficiency = c.result.as_ref().map_or(0.0, |r| r.efficiency);
                lay.fabric_m =
                    (c.length + 2.0 * opts.end_allowance_mm) * f64::from(lay.plies) / 1000.0;
            }
        }
        partial.total_fabric_m = partial.lays.iter().map(|l| l.fabric_m).sum();
        partial.objective_m = partial.total_fabric_m + opts.lay_cost_m * partial.lays.len() as f64;
        progress(&partial)?;
    }
    let solution = optimizer::solve(&measured, &required, &upper, order.max_plies, opts);
    let mut out = assemble(
        order,
        opts,
        &sizes,
        &measured,
        &solution.actions,
        false,
        warnings,
    );
    // Preserve original feasible ply actions if a bounded integer re-search
    // misses them. Re-price their updated physical markers too.
    let original: Vec<_> = incumbent
        .lays
        .iter()
        .map(|lay| {
            let ratio: Vec<_> = sizes
                .iter()
                .map(|s| lay.ratio.get(s).copied().unwrap_or(0))
                .collect();
            measured
                .iter()
                .position(|c| c.ratio == ratio)
                .map(|i| (i, lay.plies))
                .ok_or_else(|| anyhow::anyhow!("Missing incumbent ratio"))
        })
        .collect::<Result<_>>()?;
    let retained = assemble(
        order,
        opts,
        &sizes,
        &measured,
        &original,
        false,
        out.warnings.clone(),
    );
    if retained.complete && (!out.complete || retained.objective_m < out.objective_m) {
        out = retained;
    }
    out.runtime_s = started.elapsed().as_secs_f64();
    out.markers_evaluated = measured.len();
    progress(&out)?;
    Ok(out)
}
