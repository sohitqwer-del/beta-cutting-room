//! Version 0.2 orchestration. The ordinary job still uses the original adapter.
mod bounded;
mod checks;
mod layers;
#[cfg(test)]
mod tests;

use super::{NestOptions, NestProgress, nest_single, safety};
use anyhow::{Context, Result, ensure};
use beta_marker_core::geometry::grain_alignment_deg;
use beta_marker_core::{Job, Layout, MarkerResult, Nap, Piece, Placement, RotationRule};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::mpsc;
use crate::Instant;

#[derive(Clone, Debug)]
struct Slot {
    piece: usize,
    placement: Placement,
}

#[cfg(test)]
fn nest(
    input: &Job,
    opts: &NestOptions,
    callback: &mut dyn FnMut(NestProgress),
) -> Result<MarkerResult> {
    super::tuning::nest_with_tuning_and_progress(
        input,
        opts,
        &super::SearchTuning::default(),
        callback,
    )
}

pub(super) fn nest_tuned(
    input: &Job,
    opts: &NestOptions,
    callback: &mut dyn FnMut(NestProgress),
    tuning: &super::SearchTuning,
) -> Result<MarkerResult> {
    nest_controlled(input, opts, callback, tuning, None)
}

pub(super) fn nest_controlled(
    input: &Job,
    opts: &NestOptions,
    callback: &mut dyn FnMut(NestProgress),
    tuning: &super::SearchTuning,
    cancel: Option<&std::sync::atomic::AtomicBool>,
) -> Result<MarkerResult> {
    let started = Instant::now();
    let input_phase = crate::profile::Phase::new("input_and_fixed_checks");
    check_input(input, opts)?;
    let job = beta_marker_core::rules::effective_job(input);
    let mut normalized = opts.clone();
    if let Some(warm) = &mut normalized.warm_start
        && warm.placements.iter().any(|p| p.bundle.is_none())
    {
        assign_bundles(&job, &mut warm.placements);
    }
    if let Some(warm) = &mut normalized.warm_start {
        checks::refresh(&job, warm)?;
    }
    let opts = &normalized;
    safety::check_input(&job)?;
    let issues = beta_marker_core::check_job(&job);
    ensure!(issues.is_empty(), "invalid job: {issues:?}");
    let legacy_layers = job.fabric.layout != Layout::Open
        && opts.fixed.iter().chain(opts.warm_start.iter().flat_map(|r| &r.placements))
            .any(|p| p.layer_allocations.is_empty());
    let slots = if legacy_layers {
        legacy_slots(&job)?
    } else if job.fabric.layout != Layout::Open {
        if let Some(warm) = &opts.warm_start {
            layers::from_placements(&job, &warm.placements)?
        } else {
            layers::slots_after(&job, false, &opts.fixed)?
        }
    } else {
        slots(&job)?
    };
    let fixed = fixed_placements(&job, &slots, &opts.fixed)?;
    checks::geometry(&job, &fixed)?;
    if let Some(warm) = &opts.warm_start {
        ensure!(
            warm.job_name == job.name && warm.width == job.fabric.width,
            "warm start belongs to a different job or fabric width"
        );
        checks::complete(&job, warm)?;
        let report = beta_marker_core::validate(input, warm);
        ensure!(report.ok, "invalid warm start: {:?}", report.errors);
    }
    drop(input_phase);
    let seed = opts.seed.unwrap_or_else(rand::random);
    let runs = opts.runs.max(1) as usize;
    let cores = std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);
    let (parallel, workers, waves) = thread_budget(cores, runs, opts.workers);
    let mut best = opts
        .warm_start
        .clone()
        .filter(|r| fixed.iter().all(|p| r.placements.contains(p)));
    let mut reported = best.as_ref().map_or(f64::INFINITY, |r| r.length);
    if reported.is_finite() {
        callback(NestProgress {
            seconds: started.elapsed().as_secs_f64(),
            length: reported,
        });
    }
    let mut failures = Vec::new();
    let mut accept = |event| match event {
        Event::Progress(length) if length < reported => {
            reported = length;
            callback(NestProgress {
                seconds: started.elapsed().as_secs_f64(),
                length,
            });
        }
        Event::Done(Ok(result)) => {
            let report = beta_marker_core::validate(input, &result);
            if !report.ok {
                failures.push(format!(
                    "independent validation failed: {:?}",
                    report.errors
                ));
            } else if best.as_ref().is_none_or(|b| result.length < b.length) {
                best = Some(result);
            }
        }
        Event::Done(Err(error)) => failures.push(error.to_string()),
        _ => {}
    };
    for wave in 0..waves {
        if wave > 0 && cancel.is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed)) {
            break;
        }
        let first = wave * parallel;
        let count = parallel.min(runs - first);
        if count == 1 {
            let options = run_options(opts, seed, first, workers, waves);
            let result = run(
                &job,
                &slots,
                &fixed,
                &options,
                first,
                tuning,
                &mut |p| accept(Event::Progress(p.length)),
                cancel,
            );
            accept(Event::Done(result));
            continue;
        }
        let (send, receive) = mpsc::channel();
        std::thread::scope(|scope| {
            for index in first..first + count {
                let send = send.clone();
                let job = &job;
                let slots = &slots;
                let fixed = &fixed;
                let options = run_options(opts, seed, index, workers, waves);
                scope.spawn(move || {
                    let result = run(
                        job,
                        slots,
                        fixed,
                        &options,
                        index,
                        tuning,
                        &mut |p| {
                            let _ = send.send(Event::Progress(p.length));
                        },
                        cancel,
                    );
                    let _ = send.send(Event::Done(result));
                });
            }
            drop(send);
            for event in receive {
                accept(event);
            }
        });
    }
    let mut result =
        best.with_context(|| format!("all nesting runs failed: {}", failures.join("; ")))?;
    if input.fabric.shrinkage.length_pct != 0.0 || input.fabric.shrinkage.width_pct != 0.0 {
        result
            .warnings
            .push("Shrinkage applied: placements refer to rules::effective_job(job).".into());
    }
    for piece in &job.pieces {
        let placed: Vec<_> = result
            .placements
            .iter()
            .filter(|p| p.piece_id == piece.id)
            .collect();
        if placed.is_empty() {
            continue;
        }
        if piece.block_margin.is_some() {
            result.warnings.push(format!(
                "Fusing block: '{}' uses its grain-aligned bounding rectangle plus margin.",
                piece.id
            ));
        }
        if job.fabric.layout != Layout::Open {
            let unassigned = placed.iter().flat_map(|p| &p.layer_allocations).filter(|a| a.is_none()).count();
            if unassigned > 0 {
                result.warnings.push(format!("Two-layer surplus for '{}': {unassigned} unassigned copies.", piece.id));
            }
            let (normal, mirrored) = placed
                .iter()
                .map(|p| beta_marker_core::rules::layer_copies(job.fabric.layout, p))
                .fold((0u64, 0u64), |(a, b), (c, d)| {
                    (a + u64::from(c), b + u64::from(d))
                });
            let multiplier = u64::from(beta_marker_core::rules::multiplier(&job, piece));
            if beta_marker_core::rules::flip_allowed(&job, piece) {
                let surplus = (normal + mirrored).saturating_sub(
                    (u64::from(piece.quantity) + u64::from(piece.mirror_quantity)) * multiplier,
                );
                if surplus > 0 {
                    result.warnings.push(format!(
                        "Two-layer surplus for '{}': {surplus} copies on faceless fabric.",
                        piece.id
                    ));
                }
                continue;
            }
            let surplus = (
                normal.saturating_sub(u64::from(piece.quantity) * multiplier),
                mirrored.saturating_sub(u64::from(piece.mirror_quantity) * multiplier),
            );
            if surplus != (0, 0) {
                result.warnings.push(format!(
                    "Two-layer surplus for '{}': {} normal and {} mirrored copies.",
                    piece.id, surplus.0, surplus.1
                ));
            }
        }
    }
    if runs > 1 {
        result.warnings.push(format!("Portfolio: {runs} seeds, at most {parallel} concurrent runs, {workers} separator workers per run."));
    }
    if !failures.is_empty() {
        result.warnings.push(format!(
            "{} portfolio runs failed; a valid run was retained.",
            failures.len()
        ));
    }
    checks::complete(&job, &result)?;
    // Do not bypass any independent validation error.
    checks::independently_validate(input, &result)?;
    if cancel.is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed))
        && !result
            .warnings
            .iter()
            .any(|w| w.contains("Search cancelled"))
    {
        result
            .warnings
            .push("Search cancelled; best independently valid complete marker retained.".into());
    }
    result.runtime_s = started.elapsed().as_secs_f64();
    if result.length < reported {
        callback(NestProgress {
            seconds: result.runtime_s,
            length: result.length,
        });
    }
    Ok(result)
}

enum Event {
    Progress(f64),
    Done(Result<MarkerResult>),
}

fn thread_budget(cores: usize, runs: usize, requested: Option<usize>) -> (usize, usize, usize) {
    let cores = cores.max(1);
    let runs = runs.max(1);
    // Sparrow's exploration pool remains alive during compression, including
    // with one worker. Reserve two pools per run and its coordinator. The
    // outer coordinator needs a thread only when multiple runs are spawned.
    let parallel = runs.min((cores.saturating_sub(1) / 3).max(1));
    let share = if parallel == 1 {
        cores
    } else {
        (cores - 1) / parallel
    };
    let workers = requested
        .unwrap_or(3)
        .min((share.saturating_sub(1) / 2).max(1));
    (parallel, workers, runs.div_ceil(parallel))
}

fn run_options(
    opts: &NestOptions,
    seed: u64,
    index: usize,
    workers: usize,
    waves: usize,
) -> NestOptions {
    let mut options = opts.clone();
    options.runs = 1;
    options.seed = Some(seed.wrapping_add((index as u64).wrapping_mul(0x9e3779b97f4a7c15)));
    options.workers = Some(workers);
    options.time_limit_s /= waves as f64;
    options
}

fn check_input(job: &Job, opts: &NestOptions) -> Result<()> {
    safety::check_input(job)?;
    ensure!(
        opts.time_limit_s.is_finite() && opts.time_limit_s > 0.0,
        "time limit must be finite and positive"
    );
    std::time::Duration::try_from_secs_f64(opts.time_limit_s).context("time limit is too large")?;
    ensure!(opts.workers != Some(0), "workers must be greater than zero");
    ensure!(opts.runs <= 1024, "runs must be at most 1024");
    let sh = job.fabric.shrinkage;
    ensure!(
        [sh.length_pct, sh.width_pct]
            .iter()
            .all(|v| v.is_finite() && *v > -100.0),
        "shrinkage must be finite and greater than -100 percent"
    );
    if let Some(order) = &job.order {
        order
            .sizes
            .values()
            .try_fold(0u32, |sum, &n| sum.checked_add(n))
            .context("garment bundle count overflows")?;
    }
    for p in &job.pieces {
        ensure!(
            p.block_margin.is_none_or(|v| v.is_finite() && v >= 0.0),
            "block margin must be finite and non-negative for '{}'",
            p.id
        );
        ensure!(
            p.fold_line
                .is_none_or(|[a, b]| a != b && a.iter().chain(b.iter()).all(|v| v.is_finite())),
            "fold line must have two distinct finite points for '{}'",
            p.id
        );
        p.quantity
            .checked_add(p.mirror_quantity)
            .context("total quantity overflows")?
            .checked_mul(beta_marker_core::rules::multiplier(job, p))
            .context("total quantity overflows")?;
    }
    for zone in &job.fabric.defects {
        ensure!(
            zone.polygon.iter().flatten().all(|v| v.is_finite()),
            "defect coordinates must be finite"
        );
        safety::clean_outline(&zone.polygon)
            .with_context(|| format!("invalid defect '{}'", zone.id))?;
    }
    Ok(())
}

// The shared contract defines bundle numbers, including numeric sizes.
fn bundles(job: &Job, piece: &Piece) -> Vec<u32> {
    beta_marker_core::rules::bundle_sizes(job)
        .iter()
        .enumerate()
        .filter(|(_, size)| **size == piece.size)
        .map(|(index, _)| index as u32)
        .collect()
}

fn slots(job: &Job) -> Result<Vec<Slot>> {
    if job.fabric.layout != Layout::Open {
        return layers::slots(job, false);
    }
    legacy_slots(job)
}

fn legacy_slots(job: &Job) -> Result<Vec<Slot>> {
    let mut out = Vec::new();
    for (index, piece) in job.pieces.iter().enumerate() {
        let mut copies = [0, 0];
        for bundle in bundles(job, piece) {
            let mut add = |mirrored: bool, on_fold: bool, count: u32| {
                for _ in 0..count {
                    let copy = &mut copies[usize::from(mirrored)];
                    out.push(Slot {
                        piece: index,
                        placement: Placement {
                            piece_id: piece.id.clone(),
                            copy: *copy,
                            mirrored,
                            bundle: Some(bundle),
                            on_fold,
                            ..Default::default()
                        },
                    });
                    *copy += 1;
                }
            };
            if job.fabric.layout == Layout::Open {
                add(false, false, piece.quantity);
                add(true, false, piece.mirror_quantity);
            } else if beta_marker_core::rules::flip_allowed(job, piece) {
                let total = piece.quantity + piece.mirror_quantity;
                add(false, false, total / 2);
                if !total.is_multiple_of(2) {
                    add(
                        false,
                        piece.fold_line.is_some() && piece.block_margin.is_none_or(|m| m == 0.0),
                        1,
                    );
                }
            } else {
                let paired = piece.quantity.min(piece.mirror_quantity);
                add(false, false, paired);
                if piece.fold_line.is_some()
                    && piece.block_margin.is_none_or(|m| m == 0.0)
                    && piece.quantity > paired
                {
                    add(false, true, piece.quantity - paired);
                } else {
                    add(
                        false,
                        false,
                        piece.quantity.max(piece.mirror_quantity) - paired,
                    );
                }
            }
        }
    }
    ensure!(!out.is_empty(), "no pieces to place");
    Ok(out)
}

fn fixed_placements(job: &Job, slots: &[Slot], fixed: &[Placement]) -> Result<Vec<Placement>> {
    let mut used = BTreeSet::new();
    let mut out = Vec::new();
    for pl in fixed {
        let index = slots
            .iter()
            .enumerate()
            .position(|(i, s)| {
                !used.contains(&i)
                    && s.placement.piece_id == pl.piece_id
                    // A locked full two-layer cut may replace a planned fold
                    // half. Its extra handed copy is checked/reported later.
                    && (!pl.on_fold || s.placement.on_fold)
                    && (pl.layer_allocations.is_empty() || pl.layer_allocations == s.placement.layer_allocations)
                    && pl.bundle.is_none_or(|b| s.placement.bundle == Some(b))
                    && (beta_marker_core::rules::flip_allowed(job, &job.pieces[s.piece])
                        || job.fabric.layout != Layout::Open
                        || s.placement.mirrored == pl.mirrored)
                    && (pl.bundle.is_some() || s.placement.copy == pl.copy)
            })
            .with_context(|| {
                format!(
                    "fixed placement is duplicate, unknown or exceeds demand: '{}' copy {}",
                    pl.piece_id, pl.copy
                )
            })?;
        used.insert(index);
        let mut copy = pl.clone();
        if copy.bundle.is_none() {
            copy.bundle = slots[index].placement.bundle;
        }
        if job.fabric.layout != Layout::Open && copy.layer_allocations.is_empty() {
            copy.layer_allocations.clone_from(&slots[index].placement.layer_allocations);
            if !copy.on_fold && copy.layer_allocations.len() == 1 {
                copy.layer_allocations.push(None);
            }
        }
        out.push(copy);
    }
    Ok(out)
}

pub(super) fn assign_bundles(job: &Job, placements: &mut [Placement]) {
    for pl in placements {
        if let Some(piece) = job.pieces.iter().find(|p| p.id == pl.piece_id) {
            let per_garment = if pl.mirrored {
                piece.mirror_quantity
            } else {
                piece.quantity
            };
            pl.bundle = bundles(job, piece)
                .get((pl.copy / per_garment.max(1)) as usize)
                .copied();
        }
    }
}

/// Materialize a grain-aligned fusing rectangle without changing the public job.
fn cut_piece(piece: &Piece) -> Result<Piece> {
    let mut cut = piece.clone();
    if piece.block_margin.is_some() {
        cut.outline = beta_marker_core::rules::cut_outline(piece, &Placement::default());
        cut.block_margin = None;
    }
    Ok(cut)
}

fn directions(
    job: &Job,
    slots: &[Slot],
    fixed: &[Placement],
    variant: usize,
) -> Result<BTreeMap<u32, f64>> {
    if job.fabric.layout != Layout::Open && slots.iter().all(|s| !s.placement.layer_allocations.is_empty()) {
        return layers::directions(job, slots, fixed);
    }
    let mut dirs = BTreeMap::new();
    for slot in slots {
        let bundle = slot.placement.bundle.context("slot without bundle")?;
        dirs.entry(bundle).or_insert(if variant.is_multiple_of(3) {
            0.0
        } else if (bundle as usize + variant).is_multiple_of(2) {
            180.0
        } else {
            0.0
        });
        let p = &job.pieces[slot.piece];
        let allowed = beta_marker_core::allowed_rotations(job, p, slot.placement.mirrored);
        let base = grain_alignment_deg(p.grain, slot.placement.mirrored);
        if !allowed.allows(base + 180.0) {
            dirs.insert(bundle, 0.0);
        }
    }
    for pl in fixed {
        let p = job
            .pieces
            .iter()
            .find(|p| p.id == pl.piece_id)
            .context("unknown fixed piece")?;
        let direction = checks::direction(p, pl)?;
        dirs.insert(
            pl.bundle.context("fixed placement without bundle")?,
            direction,
        );
    }
    Ok(dirs)
}

#[allow(clippy::too_many_arguments)]
fn run(
    job: &Job,
    slots: &[Slot],
    fixed: &[Placement],
    opts: &NestOptions,
    variant: usize,
    tuning: &super::SearchTuning,
    progress: &mut dyn FnMut(NestProgress),
    cancel: Option<&std::sync::atomic::AtomicBool>,
) -> Result<MarkerResult> {
    let variant = variant.wrapping_add(tuning.mirror_variant);
    if job.fabric.layout != Layout::Open {
        return layers::pack(job, slots, fixed, opts, variant, tuning, progress, cancel);
    }
    run_allocated(job, slots, fixed, opts, variant, tuning, progress, cancel)
}

#[allow(clippy::too_many_arguments)]
fn run_allocated(
    job: &Job, slots: &[Slot], fixed: &[Placement], opts: &NestOptions, variant: usize,
    tuning: &super::SearchTuning, progress: &mut dyn FnMut(NestProgress),
    cancel: Option<&std::sync::atomic::AtomicBool>,
) -> Result<MarkerResult> {
    let simple = fixed.is_empty()
        && job.fabric.defects.is_empty()
        && job.fabric.layout == Layout::Open
        && !job
            .pieces
            .iter()
            .any(|p| beta_marker_core::rules::flip_allowed(job, p))
        && job.fabric.nap != Nap::OneWayPerBundle
        && job.pieces.iter().all(|p| p.block_margin.is_none());
    // Sparrow uses Rayon's serial fallback on wasm32; no native workers are needed.
    let small_cpu = !cfg!(target_arch = "wasm32")
        && std::thread::available_parallelism().map_or(true, |n| n.get() < 3);
    if simple && !small_cpu && cancel.is_none() {
        let mut result = nest_single(job, opts, progress, tuning)?;
        assign_bundles(job, &mut result.placements);
        return Ok(result);
    }
    let dirs = directions(job, slots, fixed, variant)?;
    if !fixed.is_empty()
        || !job.fabric.defects.is_empty()
        || slots.iter().any(|s| s.placement.on_fold)
        || (job.fabric.layout != Layout::Open
            && (job.spacing.edge_gap > 0.0 || job.pieces.iter().any(|p| p.buffer > 0.0)))
        || small_cpu
        || cancel.is_some()
    {
        return bounded::pack(job, slots, fixed, opts, variant, &dirs, progress, cancel);
    }
    // Sparrow cannot choose reflection per item. Give each physical placement
    // its own item and select normal/mirrored allocations across runs.
    let mut engine = job.clone();
    engine.fabric.layout = Layout::Open;
    engine.fabric.allow_flip = false;
    engine.fabric.defects.clear();
    engine.fabric.shade = None;
    engine.fabric.repeat = None;
    engine.fabric.shrinkage = Default::default();
    engine.order = None;
    engine.match_rules.clear();
    engine.pieces.clear();
    for (i, slot) in slots.iter().enumerate() {
        let mut piece = cut_piece(&job.pieces[slot.piece])?;
        piece.id = format!("__slot_{i}");
        piece.flip = Some(false);
        // Every expanded engine item belongs to its private single garment.
        // Original size/bundle identity is restored after optimization.
        piece.size = None;
        let mirrored = mirror_state(job, slot, variant);
        piece.quantity = u32::from(!mirrored);
        piece.mirror_quantity = u32::from(mirrored);
        if job.fabric.nap == Nap::OneWayPerBundle {
            let base = grain_alignment_deg(piece.grain, mirrored);
            let angle =
                (base + dirs[&slot.placement.bundle.context("slot without bundle")?]).to_radians();
            // Grain reversal fixes the chosen bundle direction, while keeping
            // the original coordinate system for the exported transform.
            let dx = angle.cos() * if mirrored { -1.0 } else { 1.0 };
            piece.grain = Some([[0.0, 0.0], [dx, -angle.sin()]]);
            piece.rotation = RotationRule::Fixed;
        }
        engine.pieces.push(piece);
    }
    engine.fabric.nap = if job.fabric.nap == Nap::OneWayPerBundle {
        Nap::OneWay
    } else {
        job.fabric.nap
    };
    let mut options = opts.clone();
    options.warm_start = opts.warm_start.as_ref().and_then(|warm| {
        let mut result = warm.clone();
        for (i, slot) in slots.iter().enumerate() {
            let pl = result.placements.iter_mut().find(|p| {
                p.piece_id == slot.placement.piece_id
                    && p.bundle == slot.placement.bundle
                    && p.mirrored == mirror_state(job, slot, variant)
            })?;
            pl.piece_id = format!("__slot_{i}");
            pl.copy = 0;
            pl.bundle = Some(0);
            pl.layer_allocations.clear();
        }
        Some(result)
    });
    let mut result = nest_single(&engine, &options, &mut |_| {}, tuning)?;
    for pl in &mut result.placements {
        let i: usize = pl
            .piece_id
            .strip_prefix("__slot_")
            .context("unknown engine slot")?
            .parse()?;
        let template = &slots.get(i).context("unknown engine slot")?.placement;
        pl.piece_id.clone_from(&template.piece_id);
        pl.copy = template.copy;
        pl.bundle = template.bundle;
        pl.layer_allocations.clone_from(&template.layer_allocations);
    }
    renumber(&mut result.placements, fixed);
    checks::refresh(job, &mut result)?;
    checks::complete(job, &result)?;
    checks::independently_validate(job, &result)?;
    progress(NestProgress {
        seconds: result.runtime_s,
        length: result.length,
    });
    Ok(result)
}

fn mirror_state(job: &Job, slot: &Slot, variant: usize) -> bool {
    if job.fabric.layout != Layout::Open {
        // A full two-layer placement cuts both handed copies. A symmetric
        // fold half cuts one full normal copy whichever primary half is used.
        if !beta_marker_core::rules::flip_allowed(job, &job.pieces[slot.piece])
            && job.pieces[slot.piece].mirror_quantity == 0
        {
            return false;
        }
        return if variant.is_multiple_of(2) {
            slot.placement.mirrored
        } else {
            !slot.placement.mirrored
        };
    }
    if !beta_marker_core::rules::flip_allowed(job, &job.pieces[slot.piece]) {
        return slot.placement.mirrored;
    }
    match variant % 5 {
        0 => slot.placement.mirrored,
        1 => !slot.placement.mirrored,
        2 => (slot.placement.copy + slot.placement.bundle.unwrap_or(0)).is_multiple_of(2),
        3 => false,
        _ => true,
    }
}

fn renumber(placements: &mut [Placement], fixed: &[Placement]) {
    let mut used = BTreeSet::new();
    for pl in fixed {
        used.insert((pl.piece_id.clone(), pl.mirrored, pl.copy));
    }
    for pl in placements {
        if fixed.contains(pl) {
            continue;
        }
        let mut copy = 0;
        while used.contains(&(pl.piece_id.clone(), pl.mirrored, copy)) {
            copy += 1;
        }
        pl.copy = copy;
        used.insert((pl.piece_id.clone(), pl.mirrored, copy));
    }
}
