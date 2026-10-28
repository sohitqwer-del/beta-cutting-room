//! Strong free-layout proposals followed by exact repair around real obstacles.
//! Only complete layouts passing the original job's validator are published.
use super::{Slot, bounded, checks, cut_piece, directions, mirror_state};
use crate::{Instant, NestOptions, NestProgress, SearchTuning, nest_single};
use anyhow::{Context, Result, ensure};
use beta_marker_core::{Job, Layout, MarkerResult, Nap, Placement};

pub(super) fn eligible(job: &Job, opts: &NestOptions) -> bool {
    job.fabric.layout == Layout::Open
        && opts.time_limit_s >= 2.
        && job.fabric.shade.is_none()
        && job.fabric.repeat.is_none()
        && job.match_rules.is_empty()
        && job.fabric.nap != Nap::OneWayPerBundle
        && (cfg!(target_arch = "wasm32")
            || std::thread::available_parallelism().is_ok_and(|n| n.get() >= 3))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn pack(
    job: &Job,
    slots: &[Slot],
    fixed: &[Placement],
    opts: &NestOptions,
    variant: usize,
    tuning: &SearchTuning,
    progress: &mut dyn FnMut(NestProgress),
) -> Result<MarkerResult> {
    let started = Instant::now();
    let dirs = directions(job, slots, fixed, variant)?;
    let mut options = opts.clone();
    options.time_limit_s *= 0.2;
    let mut best = bounded::pack(job, slots, fixed, &options, variant, &dirs, progress, None)
        .ok()
        .or_else(|| {
            opts.warm_start
                .as_ref()
                .filter(|warm| fixed.iter().all(|p| warm.placements.contains(p)))
                .cloned()
        });
    let mut used = vec![false; slots.len()];
    for p in fixed {
        let i = slots
            .iter()
            .enumerate()
            .position(|(i, s)| {
                !used[i]
                    && s.placement.piece_id == p.piece_id
                    && s.placement.bundle == p.bundle
                    && (s.placement.mirrored == p.mirrored
                        || beta_marker_core::rules::flip_allowed(job, &job.pieces[s.piece]))
            })
            .context("fixed placement has no proposal slot")?;
        used[i] = true;
    }
    let movable: Vec<_> = slots
        .iter()
        .enumerate()
        .filter(|(i, _)| !used[*i])
        .map(|(_, s)| s)
        .collect();
    let remaining = opts.time_limit_s - started.elapsed().as_secs_f64();
    if !movable.is_empty() && remaining > 0.1 {
        let mut free = job.clone();
        free.fabric.defects.clear();
        free.fabric.allow_flip = false;
        free.order = None;
        free.pieces.clear();
        for (i, slot) in movable.iter().enumerate() {
            let mut piece = cut_piece(&job.pieces[slot.piece])?;
            piece.id = format!("__repair_{i}");
            piece.size = None;
            piece.flip = Some(false);
            let mirrored = mirror_state(job, slot, variant);
            piece.quantity = u32::from(!mirrored);
            piece.mirror_quantity = u32::from(mirrored);
            free.pieces.push(piece);
        }
        options.time_limit_s = remaining * 0.4;
        options.warm_start = None;
        options.fixed.clear();
        if let Ok(proposal) = nest_single(&free, &options, &mut |_| {}, tuning) {
            let mut hints = vec![];
            for mut p in proposal.placements {
                let i: usize = p
                    .piece_id
                    .strip_prefix("__repair_")
                    .context("unknown repair item")?
                    .parse()?;
                let slot = movable.get(i).context("unknown repair slot")?;
                p.piece_id.clone_from(&slot.placement.piece_id);
                p.copy = slot.placement.copy;
                p.bundle = slot.placement.bundle;
                p.layer_allocations
                    .clone_from(&slot.placement.layer_allocations);
                hints.push(p);
            }
            options = opts.clone();
            options.time_limit_s = (opts.time_limit_s - started.elapsed().as_secs_f64()).max(0.05);
            options.warm_start = best.clone();
            let repaired = bounded::repair(
                job,
                slots,
                fixed,
                &options,
                variant,
                &dirs,
                &mut |event| {
                    if best.as_ref().is_none_or(|old| event.length < old.length) {
                        best = Some(event.result.clone());
                        progress(NestProgress {
                            seconds: started.elapsed().as_secs_f64(),
                            ..event
                        });
                    }
                },
                &hints,
            );
            if let Ok(candidate) = repaired
                && best
                    .as_ref()
                    .is_none_or(|old| candidate.length < old.length)
            {
                best = Some(candidate);
            }
        }
    }
    let mut result =
        best.context("obstacle search and repaired free-layout proposal both failed")?;
    checks::complete(job, &result)?;
    checks::independently_validate(job, &result)?;
    ensure!(
        fixed.iter().all(|p| result.placements.contains(p)),
        "repaired proposal moved a fixed piece"
    );
    result.runtime_s = started.elapsed().as_secs_f64();
    result.warnings.push("Compared a free-layout proposal with exact obstacle repair; fixed transforms and the complete valid incumbent were retained.".into());
    Ok(result)
}
