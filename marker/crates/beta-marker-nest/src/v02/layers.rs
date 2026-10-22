//! Allocate useful copies before choosing their physical cut geometry.
use super::{Slot, bounded, checks};
use crate::{NestOptions, NestProgress};
use anyhow::{Result, ensure};
use beta_marker_core::geometry::{bbox, grain_alignment_deg, transform_points};
use beta_marker_core::rules::{bundle_sizes, flip_allowed};
use beta_marker_core::{Job, LayerAllocation, Layout, MarkerResult, Placement};
use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;
use crate::Instant;

/// Pair complementary hands across garments of the same piece/size. Symmetric
/// pieces can use either physical layer. The alternative keeps those copies on
/// the fold so narrow tubes are not forced to accept full-width bodies.
pub(super) fn slots(job: &Job, pair_symmetric: bool) -> Result<Vec<Slot>> {
    slots_after(job, pair_symmetric, &[])
}

pub(super) fn from_placements(job: &Job, placements: &[Placement]) -> Result<Vec<Slot>> {
    placements
        .iter()
        .map(|placement| {
            let piece = job
                .pieces
                .iter()
                .position(|p| p.id == placement.piece_id)
                .ok_or_else(|| {
                    anyhow::anyhow!("unknown allocated piece '{}'", placement.piece_id)
                })?;
            Ok(Slot {
                piece,
                placement: placement.clone(),
            })
        })
        .collect()
}

pub(super) fn slots_after(
    job: &Job,
    pair_symmetric: bool,
    fixed: &[Placement],
) -> Result<Vec<Slot>> {
    let sizes = bundle_sizes(job);
    let mut out = from_placements(job, fixed)?;
    for (index, piece) in job.pieces.iter().enumerate() {
        let mut normal = Vec::new();
        let mut mirrored = Vec::new();
        for (bundle, _) in sizes.iter().enumerate().filter(|(_, s)| **s == piece.size) {
            normal.extend((0..piece.quantity).map(|_| LayerAllocation {
                bundle: bundle as u32,
                mirrored: false,
            }));
            mirrored.extend((0..piece.mirror_quantity).map(|_| LayerAllocation {
                bundle: bundle as u32,
                mirrored: true,
            }));
        }
        for assigned in fixed
            .iter()
            .filter(|p| p.piece_id == piece.id)
            .flat_map(|p| p.layer_allocations.iter().flatten())
        {
            let (primary, alternate) = if assigned.mirrored {
                (&mut mirrored, &mut normal)
            } else {
                (&mut normal, &mut mirrored)
            };
            if let Some(position) = primary.iter().position(|a| a == assigned) {
                primary.remove(position);
            } else if let Some(position) = alternate
                .iter()
                .position(|a| flip_allowed(job, piece) && a.bundle == assigned.bundle)
            {
                alternate.remove(position);
            } else {
                anyhow::bail!("fixed layer exceeds demand for '{}'", piece.id);
            }
        }
        normal.reverse();
        mirrored.reverse();
        let foldable = piece.fold_line.is_some() && piece.block_margin.is_none_or(|m| m == 0.0);
        let symmetric = beta_marker_core::rules::layer_hands_interchangeable(job, piece);
        let faceless = flip_allowed(job, piece);
        let points = transform_points(
            &piece.outline,
            false,
            grain_alignment_deg(piece.grain, false),
            [0.0; 2],
        );
        let full_fits = bbox(&points).is_some_and(|b| {
            b[3] - b[1]
                <= job.fabric.width
                    - if job.fabric.layout == Layout::Folded {
                        job.spacing.edge_gap + piece.buffer
                    } else {
                        0.0
                    }
        });
        let mut copy = 0;
        while !normal.is_empty() || !mirrored.is_empty() {
            let mut a = normal.pop();
            let mut b = if a.is_none() || !foldable || (pair_symmetric && full_fits) {
                mirrored.pop()
            } else {
                None
            };
            if (faceless || symmetric) && full_fits && (pair_symmetric || !foldable) {
                if a.is_none() {
                    a = mirrored.pop();
                }
                if b.is_none() {
                    b = normal.pop();
                }
            }
            // A single symmetric demand can be cut as a half. A mirrored
            // requested copy has the same geometry and an explicit hand label.
            let on_fold = foldable && (a.is_none() || b.is_none());
            let allocations = if on_fold {
                vec![a.or(b)]
            } else {
                vec![a.take(), b.take()]
            };
            let bundle = allocations.iter().flatten().next().map(|a| a.bundle);
            out.push(Slot {
                piece: index,
                placement: Placement {
                    piece_id: piece.id.clone(),
                    copy,
                    bundle,
                    on_fold,
                    layer_allocations: allocations,
                    ..Default::default()
                },
            });
            copy += 1;
        }
    }
    ensure!(!out.is_empty(), "no pieces to place");
    Ok(out)
}

/// Explain a folded marker whose length is set by its fold pieces, not by the search.
/// Fold halves all need the one fold edge, and a full cut too deep to sit beside
/// them cannot share their place along the marker, so their lengths add up.
/// Shapes can interlock a little, so this is an estimate, not a strict bound.
pub(super) fn fold_edge_note(job: &Job, result: &MarkerResult) -> Option<String> {
    if job.fabric.layout != Layout::Folded || result.length <= 0.0 {
        return None;
    }
    let usable = job.fabric.width - job.spacing.edge_gap;
    let (mut halves, mut wide) = (Vec::new(), Vec::new());
    for pl in &result.placements {
        let piece = job.pieces.iter().find(|p| p.id == pl.piece_id)?;
        let b = bbox(&beta_marker_core::rules::cut_outline(piece, pl))?;
        let item = (b[2] - b[0], b[3] - b[1]);
        if pl.on_fold {
            halves.push(item);
        } else if item.1 > usable / 2.0 {
            wide.push(item);
        }
    }
    if halves.is_empty() {
        return None;
    }
    let total = |items: &[(f64, f64)]| {
        items.iter().map(|i| i.0).sum::<f64>()
            + job.spacing.piece_gap * items.len().saturating_sub(1) as f64
    };
    // Halves all share the fold edge. A full cut joins them when it is too deep
    // to sit beside the shallowest half, and other such cuts when it is wider
    // than half the table, so no two items of the chain fit side by side.
    let shallowest = halves.iter().map(|i| i.1).fold(f64::INFINITY, f64::min);
    let threshold = (usable / 2.0).max(usable - shallowest);
    let chain: Vec<(f64, f64)> = halves
        .iter()
        .chain(wide.iter().filter(|i| i.1 > threshold))
        .copied()
        .collect();
    let (count, needed) = (chain.len(), total(&chain));
    (count > 1 && needed >= 0.85 * result.length).then(|| {
        format!(
            "Fold edge: {count} pieces cut on the fold, or too deep to sit beside them, measure about {:.2} m end to end; this marker is {:.2} m. Even counts per size (cut as full two-layer pieces), a tube or open fabric can make it shorter.",
            needed / 1000.0,
            result.length / 1000.0
        )
    })
}

/// Shared cuts couple the directions of all garments they serve.
pub(super) fn directions(
    job: &Job,
    slots: &[Slot],
    fixed: &[Placement],
) -> Result<BTreeMap<u32, f64>> {
    let mut groups: Vec<usize> = (0..bundle_sizes(job).len()).collect();
    if job.fabric.nap != beta_marker_core::Nap::OneWayPerBundle {
        return Ok(groups.iter().map(|&i| (i as u32, 0.0)).collect());
    }
    for slot in slots {
        let bundles = beta_marker_core::rules::placement_bundles(&slot.placement);
        if let Some(&first) = bundles.first() {
            let target = *groups
                .get(first as usize)
                .ok_or_else(|| anyhow::anyhow!("invalid layer bundle"))?;
            for bundle in bundles {
                let previous = *groups
                    .get(bundle as usize)
                    .ok_or_else(|| anyhow::anyhow!("invalid layer bundle"))?;
                for group in &mut groups {
                    if *group == previous {
                        *group = target;
                    }
                }
            }
        }
    }
    let mut forced = BTreeMap::new();
    for slot in slots {
        let piece = &job.pieces[slot.piece];
        let base = grain_alignment_deg(piece.grain, slot.placement.mirrored);
        if !beta_marker_core::allowed_rotations(job, piece, slot.placement.mirrored)
            .allows(base + 180.0)
        {
            for bundle in beta_marker_core::rules::placement_bundles(&slot.placement) {
                forced.insert(groups[bundle as usize], 0.0);
            }
        }
    }
    for pl in fixed {
        let piece = job
            .pieces
            .iter()
            .find(|p| p.id == pl.piece_id)
            .ok_or_else(|| anyhow::anyhow!("unknown fixed piece"))?;
        let direction = checks::direction(piece, pl)?;
        for bundle in beta_marker_core::rules::placement_bundles(pl) {
            let group = *groups
                .get(bundle as usize)
                .ok_or_else(|| anyhow::anyhow!("invalid fixed layer bundle"))?;
            if let Some(old) = forced.insert(group, direction) {
                ensure!(
                    old == direction,
                    "shared layer garments have incompatible fixed directions"
                );
            }
        }
    }
    Ok(groups
        .iter()
        .enumerate()
        .map(|(i, group)| (i as u32, forced.get(group).copied().unwrap_or(0.0)))
        .collect())
}

fn explicit_legacy(job: &Job, mut result: MarkerResult) -> Result<MarkerResult> {
    let mut remaining = BTreeMap::new();
    for piece in &job.pieces {
        for bundle in super::bundles(job, piece) {
            remaining.insert(
                (piece.id.as_str(), bundle),
                [piece.quantity, piece.mirror_quantity],
            );
        }
    }
    for pl in &mut result.placements {
        let piece = job
            .pieces
            .iter()
            .find(|p| p.id == pl.piece_id)
            .ok_or_else(|| anyhow::anyhow!("unknown legacy piece"))?;
        let bundle = pl
            .bundle
            .ok_or_else(|| anyhow::anyhow!("unassigned legacy piece"))?;
        let demand = remaining
            .get_mut(&(piece.id.as_str(), bundle))
            .ok_or_else(|| anyhow::anyhow!("unknown legacy bundle"))?;
        pl.layer_allocations.clear();
        for physical in 0..if pl.on_fold { 1 } else { 2 } {
            let hand = if demand[physical] > 0 {
                Some(physical)
            } else if flip_allowed(job, piece) && demand[1 - physical] > 0 {
                Some(1 - physical)
            } else {
                None
            };
            pl.layer_allocations.push(hand.map(|hand| {
                demand[hand] -= 1;
                LayerAllocation {
                    bundle,
                    mirrored: hand == 1,
                }
            }));
        }
    }
    ensure!(
        remaining.values().all(|n| *n == [0, 0]),
        "legacy seed misses useful demand"
    );
    checks::independently_validate(job, &result)?;
    Ok(result)
}

fn compatible(slots: &[Slot], result: &MarkerResult) -> bool {
    let mut unused = vec![true; slots.len()];
    for pl in &result.placements {
        let Some(index) = slots.iter().enumerate().position(|(i, slot)| {
            unused[i]
                && slot.placement.piece_id == pl.piece_id
                && slot.placement.bundle == pl.bundle
                && slot
                    .placement
                    .layer_allocations
                    .iter()
                    .flatten()
                    .eq(pl.layer_allocations.iter().flatten())
        }) else {
            return false;
        };
        unused[index] = false;
    }
    unused.iter().all(|used| !used)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn pack(
    job: &Job,
    original: &[Slot],
    fixed: &[Placement],
    opts: &NestOptions,
    variant: usize,
    tuning: &crate::SearchTuning,
    progress: &mut dyn FnMut(NestProgress),
    cancel: Option<&AtomicBool>,
) -> Result<MarkerResult> {
    let started = Instant::now();
    if original.iter().all(|s| !s.placement.on_fold) {
        return super::run_allocated(
            job, original, fixed, opts, variant, tuning, progress, cancel,
        );
    }
    if original
        .iter()
        .any(|s| s.placement.layer_allocations.is_empty())
    {
        let directions = super::directions(job, original, fixed, variant)?;
        return bounded::pack(
            job,
            original,
            fixed,
            opts,
            variant,
            &directions,
            progress,
            cancel,
        );
    }
    // Keep both garment directions equal for shared layers. This remains legal
    // for one-way-per-bundle and avoids coupling incompatible garment directions.
    let directions = directions(job, original, fixed)?;
    let paired = slots(job, true)?;
    let sparrow = job.fabric.defects.is_empty()
        && job.pieces.iter().all(|p| p.buffer <= 0.0)
        && cancel.is_none()
        && (cfg!(target_arch = "wasm32")
            || std::thread::available_parallelism().is_ok_and(|n| n.get() >= 3));
    // (slots, bounded packer only)
    let mut policies: Vec<(&[Slot], bool)> =
        if fixed.is_empty() && opts.warm_start.is_none() && paired.len() < original.len() {
            if job.fabric.layout == Layout::Tubular {
                vec![(original, false), (&paired, false)]
            } else {
                vec![(&paired, false), (original, false)]
            }
        } else {
            vec![(original, false)]
        };
    // Sparrow keeps every fold half on the fold. On folded fabric a shorter
    // marker can cut some of them as full two-layer pieces beside the halves,
    // with a surplus layer; the bounded packer can mix both, so it also runs.
    // The shortest valid marker wins: surplus copies cost their fabric length.
    if sparrow
        && fixed.is_empty()
        && job.fabric.layout == Layout::Folded
        && original.iter().any(|s| s.placement.on_fold)
    {
        policies.push((original, true));
    }
    // Keep the old fold search as a checked incumbent. Without Sparrow, tubes
    // reserve half their budget for that established ordering before trying
    // the additional hands and full-cut allocations.
    let seed = if fixed.is_empty() && opts.warm_start.is_none() {
        let mut quick = opts.clone();
        quick.time_limit_s = if job.fabric.layout == Layout::Tubular && !sparrow {
            opts.time_limit_s * 0.5
        } else {
            (opts.time_limit_s * 0.02).min(0.25)
        };
        let legacy = super::legacy_slots(job)?;
        bounded::pack(
            job,
            &legacy,
            &[],
            &quick,
            variant,
            &directions,
            &mut |_| {},
            cancel,
        )
        .and_then(|result| explicit_legacy(job, result))
        .ok()
    } else {
        None
    };
    // When fold halves and wide pieces already fill the fold edge, the order
    // sets the length; the bounded packer, which mixes halves and full cuts,
    // gets all the time.
    if seed
        .as_ref()
        .or(opts.warm_start.as_ref())
        .is_some_and(|marker| fold_edge_note(job, marker).is_some())
    {
        policies.retain(|(_, bounded_only)| !bounded_only);
        for policy in &mut policies {
            policy.1 = true;
        }
    }
    let mut best = seed.clone();
    let mut errors = Vec::new();
    let mut reported = f64::INFINITY;
    for (index, &(policy, bounded_only)) in policies.iter().enumerate() {
        if index > 0
            && (started.elapsed().as_secs_f64() >= opts.time_limit_s
                || cancel.is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed)))
        {
            break;
        }
        let mut options = opts.clone();
        let left = (opts.time_limit_s - started.elapsed().as_secs_f64()).max(0.001);
        options.time_limit_s = if !sparrow && policies.len() == 2 && index == 0 {
            left * if job.fabric.layout == Layout::Tubular {
                0.8
            } else {
                0.6
            }
        } else {
            // Sparrow searches share the remaining time equally.
            left / (policies.len() - index) as f64
        };
        if options.warm_start.is_none()
            && let Some(seed) = seed.as_ref().filter(|seed| compatible(policy, seed))
        {
            options.warm_start = Some(seed.clone());
        }
        let mut on_progress = |mut event: NestProgress| {
            if event.length < reported {
                reported = event.length;
                event.seconds = started.elapsed().as_secs_f64();
                progress(event);
            }
        };
        // Fold halves use pinned Sparrow when possible, else the bounded packer.
        let result = if bounded_only {
            bounded::pack(job, policy, fixed, &options, variant, &directions, &mut on_progress, cancel)
        } else {
            super::run_allocated(job, policy, fixed, &options, variant, tuning, &mut on_progress, cancel)
        };
        match result {
            Ok(result)
                if best.as_ref().is_none_or(|old| {
                    result.length < old.length - 0.01
                        || ((result.length - old.length).abs() <= 0.01
                            && result.piece_area < old.piece_area)
                }) =>
            {
                best = Some(result)
            }
            Ok(_) => {}
            Err(error) => errors.push(error.to_string()),
        }
    }
    let mut best =
        best.ok_or_else(|| anyhow::anyhow!("layer packing failed: {}", errors.join("; ")))?;
    checks::independently_validate(job, &best)?;
    best.runtime_s = started.elapsed().as_secs_f64();
    Ok(best)
}
