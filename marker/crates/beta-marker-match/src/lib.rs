//! Repeat-lattice matching and whole-garment shade bands.
//!
//! See the crate README for the search limits and the matching convention.

mod checks;
mod search;
mod shade;
#[cfg(test)]
mod tests;

use anyhow::{Result, ensure};
use beta_marker_core::{Job, MarkerResult, Placement};
use beta_marker_nest::NestOptions;
use jagua_rs::Instant;
use std::collections::BTreeMap;
pub mod browser;

pub use checks::match_errors;

/// `true` if the job needs this crate (matching rules or shade bands).
pub fn needs_rules(job: &Job) -> bool {
    !job.match_rules.is_empty() || job.fabric.shade.is_some()
}

/// Nest matching rules and shade bands, then check the finished marker.
///
/// Search is bounded and heuristic. An error means this search found no feasible
/// layout; it does not prove that the job is impossible. User-fixed placements
/// are never moved. Shrinkage is applied before solving match-point phases.
pub fn nest_with_rules(job: &Job, opts: &NestOptions) -> Result<MarkerResult> {
    nest_with_shade_search(job, opts, 1)
}

/// Try alternative whole-bundle shade assignments, then refine the best checked one.
/// The probes share 35% of the budget; refinement receives the remaining wall time.
pub fn nest_with_shade_search(
    job: &Job,
    opts: &NestOptions,
    attempts: usize,
) -> Result<MarkerResult> {
    if !needs_rules(job) {
        return beta_marker_nest::nest(job, opts);
    }
    let started = Instant::now();
    checks::check_input(job, opts)?;
    let effective = beta_marker_core::rules::effective_job(job);
    let mut result = if effective.fabric.shade.is_some() {
        if attempts == 1 {
            shade::nest_bands(&effective, opts)?
        } else {
            shade::nest_bands_search(&effective, opts, attempts)?
        }
    } else {
        nest_matching(&effective, opts)?
    };
    let report = beta_marker_core::validate(job, &result);
    ensure!(
        report.ok,
        "rule marker failed validation: {:?}",
        report.errors
    );
    let errors = match_errors(job, &result);
    ensure!(errors.is_empty(), "rule marker failed matching: {errors:?}");
    let errors = checks::shade_errors(&effective, &result);
    ensure!(
        errors.is_empty(),
        "rule marker failed shade checks: {errors:?}"
    );
    result.runtime_s = started.elapsed().as_secs_f64();
    Ok(result)
}

/// Use the same garment IDs as the nesting engine and independent validator.
fn bundles(job: &Job) -> Vec<Option<String>> {
    beta_marker_core::rules::bundle_sizes(job)
}

fn bundle_for(job: &Job, pl: &Placement) -> Option<u32> {
    let piece = job.pieces.iter().find(|p| p.id == pl.piece_id)?;
    let quantity = if pl.mirrored {
        piece.mirror_quantity
    } else {
        piece.quantity
    };
    if quantity == 0 {
        return pl.bundle;
    }
    let local = pl.copy / quantity;
    bundles(job)
        .iter()
        .enumerate()
        .filter(|(_, size)| job.order.is_none() || **size == piece.size)
        .nth(local as usize)
        .map(|(index, _)| index as u32)
}

fn assign_bundles(job: &Job, placements: &mut [Placement]) {
    for pl in placements {
        if pl.bundle.is_none() {
            pl.bundle = bundle_for(job, pl);
        }
    }
}

fn identity(pl: &Placement) -> (&str, bool, u32) {
    (&pl.piece_id, pl.mirrored, pl.copy)
}

fn nest_matching(job: &Job, opts: &NestOptions) -> Result<MarkerResult> {
    let started = Instant::now();
    // A bounded setup allowance keeps sub-second interactive requests useful.
    // Optimization still receives only the remaining requested time.
    let allowance = std::time::Duration::try_from_secs_f64(opts.time_limit_s + 2.)
        .map_err(|_| anyhow::anyhow!("Matching time limit is too large."))?;
    let deadline = started
        .checked_add(allowance)
        .ok_or_else(|| anyhow::anyhow!("Matching time limit is too large."))?;
    // The engine validates its intermediate result. Matching is checked after
    // our fixed placements have been installed, not on the free baseline.
    let mut free_job = job.clone();
    free_job.match_rules.clear();
    free_job.fabric.shade = None;
    if job.match_rules.is_empty() {
        let mut result = beta_marker_nest::nest(&free_job, opts)?;
        assign_bundles(job, &mut result.placements);
        return Ok(result);
    }
    let mut baseline_opts = opts.clone();
    baseline_opts.time_limit_s = (opts.time_limit_s * 0.45).max(0.01);
    let matched = |id: &str| {
        job.match_rules
            .iter()
            .any(|r| r.a.piece_id == id || r.b.as_ref().is_some_and(|b| b.piece_id == id))
    };
    let mut buffered_job = free_job.clone();
    let use_buffer = opts.fixed.is_empty()
        && opts.warm_start.is_none()
        && job.fabric.layout == beta_marker_core::Layout::Open;
    if use_buffer {
        let repeat = job
            .fabric
            .repeat
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("matching needs repeat"))?;
        for p in &mut buffered_job.pieces {
            if matched(&p.id) {
                let active = |axis| {
                    job.match_rules.iter().any(|r| {
                        (r.a.piece_id == p.id || r.b.as_ref().is_some_and(|b| b.piece_id == p.id))
                            && checks::axes(r.axes, axis)
                    })
                };
                let dx = if active(0) {
                    repeat.period_x.unwrap_or(0.0)
                } else {
                    0.0
                };
                let dy = if active(1) {
                    repeat.period_y.unwrap_or(0.0)
                } else {
                    0.0
                };
                let radius = dx.hypot(dy) / 2.0 + 0.05;
                p.buffer += radius;
            }
        }
    }
    let mut baseline = if use_buffer {
        beta_marker_nest::nest(&buffered_job, &baseline_opts)
            .or_else(|_| beta_marker_nest::nest(&free_job, &baseline_opts))?
    } else {
        beta_marker_nest::nest(&free_job, &baseline_opts)?
    };
    assign_bundles(job, &mut baseline.placements);
    let snapped = if use_buffer {
        search::snap_baseline(job, &baseline)?
    } else {
        None
    };
    let fixed = if let Some(candidate) = &snapped {
        candidate
            .placements
            .iter()
            .filter(|p| matched(&p.piece_id))
            .cloned()
            .collect()
    } else {
        search::place_matched_until(job, &baseline, &opts.fixed, Some(deadline))?
    };
    let mut finish_opts = opts.clone();
    finish_opts.time_limit_s = (opts.time_limit_s - started.elapsed().as_secs_f64()).max(0.01);
    finish_opts.fixed = fixed;
    // The snapped layout is already complete and checked. Keep it available
    // while bounded fixed-piece compaction searches for a shorter marker.
    finish_opts.warm_start = snapped.clone();
    let mut result = beta_marker_nest::nest(&free_job, &finish_opts)?;
    assign_bundles(job, &mut result.placements);
    let expected: BTreeMap<_, _> = finish_opts.fixed.iter().map(|p| (identity(p), p)).collect();
    ensure!(
        expected
            .keys()
            .all(|key| result.placements.iter().any(|p| identity(p) == *key)),
        "nesting engine omitted a fixed placement"
    );
    for pl in &mut result.placements {
        if let Some((translate, rotation, bundle, locked)) = expected
            .get(&identity(pl))
            .map(|p| (p.translate, p.rotation_deg, p.bundle, p.locked))
        {
            ensure!(
                pl.translate == translate && pl.rotation_deg == rotation,
                "nesting engine moved fixed matched piece {}",
                pl.piece_id
            );
            pl.bundle = bundle;
            pl.locked = locked;
        }
    }
    let errors = match_errors(job, &result);
    ensure!(errors.is_empty(), "matched layout is invalid: {errors:?}");
    if let Some(candidate) = snapped
        && candidate.length < result.length
    {
        result = candidate;
    }
    result.engine = format!("repeat lattice + {}", result.engine);
    Ok(result)
}

fn refresh(job: &Job, result: &mut MarkerResult) {
    let m = beta_marker_core::compute_metrics(job, &result.placements);
    result.width = job.fabric.width;
    result.length = m.length;
    result.piece_area = m.piece_area;
    result.efficiency = m.efficiency;
}
