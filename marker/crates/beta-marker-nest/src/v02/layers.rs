//! Allocate useful copies before choosing their physical cut geometry.
use super::{Slot, bounded, checks};
use crate::Instant;
use crate::{NestOptions, NestProgress};
use anyhow::{Result, ensure};
use beta_marker_core::geometry::{bbox, grain_alignment_deg, transform_points};
use beta_marker_core::rules::{bundle_sizes, flip_allowed};
use beta_marker_core::{Job, LayerAllocation, Layout, MarkerResult, Placement};
use std::collections::BTreeMap;
use std::sync::atomic::AtomicBool;

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
    slots_policy(job, &vec![pair_symmetric; job.pieces.len()], fixed)
}

fn slots_policy(job: &Job, pair_policy: &[bool], fixed: &[Placement]) -> Result<Vec<Slot>> {
    ensure!(
        pair_policy.len() == job.pieces.len(),
        "allocation policy length mismatch"
    );
    let sizes = bundle_sizes(job);
    let mut out = from_placements(job, fixed)?;
    for (index, piece) in job.pieces.iter().enumerate() {
        let pair_symmetric = pair_policy[index];
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

fn allocation_signature(slots: &[Slot]) -> Vec<String> {
    let mut keys: Vec<_> = slots
        .iter()
        .map(|s| {
            format!(
                "{}:{}:{:?}",
                s.piece, s.placement.on_fold, s.placement.layer_allocations
            )
        })
        .collect();
    keys.sort();
    keys
}

/// A piece ID is size-specific. Each bit chooses useful two-layer pairing for
/// that piece/size; the other pieces remain on the fold when possible.
fn mixed_policies(job: &Job, fixed: &[Placement]) -> Result<Vec<Vec<Slot>>> {
    let eligible: Vec<_> = job
        .pieces
        .iter()
        .enumerate()
        .filter(|(_, p)| p.fold_line.is_some() && p.block_margin.is_none_or(|m| m == 0.))
        .map(|(i, _)| i)
        .collect();
    if eligible.len() < 2 {
        return Ok(vec![]);
    }
    let dims: Vec<_> = job
        .pieces
        .iter()
        .map(|p| {
            let rotation = grain_alignment_deg(p.grain, false);
            let full = beta_marker_core::rules::cut_outline(
                p,
                &Placement {
                    rotation_deg: rotation,
                    ..Default::default()
                },
            );
            let half = beta_marker_core::rules::cut_outline(
                p,
                &Placement {
                    rotation_deg: rotation,
                    on_fold: true,
                    ..Default::default()
                },
            );
            bbox(&full)
                .zip(bbox(&half))
                .map(|(f, h)| (f[2] - f[0], f[3] - f[1], h[3] - h[1]))
        })
        .collect();
    let useful_pair = |i: usize| {
        let p = &job.pieces[i];
        (u64::from(p.quantity) + u64::from(p.mirror_quantity))
            * u64::from(beta_marker_core::rules::multiplier(job, p))
            >= 2
    };
    // Rank the particular short panel worth keeping on the fold, rather than
    // converting every narrow panel (including easy-to-pack trims) into halves.
    let mut ranked = vec![];
    for &b in &eligible {
        if !useful_pair(b) {
            continue;
        }
        let Some((lb, db, hb)) = dims[b] else {
            continue;
        };
        let usable = job.fabric.width - job.spacing.edge_gap;
        if db + job.pieces[b].buffer > usable {
            continue;
        }
        let mut score = 0.0_f64;
        for &a in &eligible {
            if a == b || !useful_pair(a) {
                continue;
            }
            let Some((la, da, _)) = dims[a] else {
                continue;
            };
            let gap = job.spacing.piece_gap + job.pieces[a].buffer + job.pieces[b].buffer;
            let before = la + lb + gap;
            let saving = before - la.max(2. * lb + gap);
            if saving > 0.03 * before && da + hb + gap <= usable + 1e-7 && da + db + gap > usable {
                score = score.max(saving);
            }
        }
        if score > 0. {
            ranked.push((score, b));
        }
    }
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
    let mut masks = vec![];
    for &(_, b) in ranked.iter().take(2) {
        let mut mask = vec![true; job.pieces.len()];
        mask[b] = false;
        masks.push(mask);
    }
    if ranked.len() >= 2 {
        let mut mask = vec![true; job.pieces.len()];
        mask[ranked[0].1] = false;
        mask[ranked[1].1] = false;
        masks.push(mask);
    }
    for &(_, b) in ranked.iter().skip(2).take(2) {
        let mut mask = vec![true; job.pieces.len()];
        mask[b] = false;
        masks.push(mask);
    }
    let mut seen = std::collections::BTreeSet::new();
    seen.insert(allocation_signature(&slots_after(job, false, fixed)?));
    seen.insert(allocation_signature(&slots_after(job, true, fixed)?));
    let mut out = vec![];
    for mask in masks {
        let candidate = slots_policy(job, &mask, fixed)?;
        if seen.insert(allocation_signature(&candidate)) {
            out.push(candidate);
        }
    }
    Ok(out)
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
    if original.iter().all(|s| !s.placement.on_fold)
        && !job.pieces.iter().any(|p| p.fold_line.is_some())
    {
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
    let paired = slots_after(job, true, fixed)?;
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
    let mut mixed = mixed_policies(job, fixed)?;
    let mixed_budget = (opts.time_limit_s / 2.).ceil().clamp(1., 5.) as usize;
    mixed.truncate(mixed_budget);
    policies.extend(mixed.iter().map(|policy| (policy.as_slice(), true)));
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
        for policy in &mut policies {
            policy.1 = true;
        }
    }
    let mut seen = std::collections::BTreeSet::new();
    policies.retain(|(policy, bounded)| seen.insert((allocation_signature(policy), *bounded)));
    let mut best = opts
        .warm_start
        .as_ref()
        .filter(|warm| fixed.iter().all(|p| warm.placements.contains(p)))
        .cloned()
        .or_else(|| seed.clone());
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
        options.warm_start = opts
            .warm_start
            .as_ref()
            .filter(|warm| compatible(policy, warm))
            .cloned();
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
            self::directions(job, policy, fixed).and_then(|dirs| {
                bounded::pack(
                    job,
                    policy,
                    fixed,
                    &options,
                    variant,
                    &dirs,
                    &mut on_progress,
                    cancel,
                )
            })
        } else {
            super::run_allocated(
                job,
                policy,
                fixed,
                &options,
                variant,
                tuning,
                &mut on_progress,
                cancel,
            )
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
    if !mixed.is_empty() {
        best.warnings.push(format!(
            "Compared {} mixed per-piece/size half/full layer allocations.",
            mixed.len()
        ));
    }
    checks::independently_validate(job, &best)?;
    best.runtime_s = started.elapsed().as_secs_f64();
    Ok(best)
}

#[cfg(test)]
mod mixed_tests {
    use super::*;
    use beta_marker_core::{Fabric, JOB_FORMAT, Piece, RotationRule};
    #[test]
    fn useful_full_body_and_folded_small_panels_can_mix_without_surplus() -> Result<()> {
        let make = |id: &str, length: f64, depth: f64| Piece {
            id: id.into(),
            quantity: 2,
            outline: vec![[0., 0.], [length, 0.], [length, depth], [0., depth]],
            grain: Some([[0., depth / 2.], [length, depth / 2.]]),
            fold_line: Some([[0., depth / 2.], [length, depth / 2.]]),
            rotation: RotationRule::Flip180,
            ..Default::default()
        };
        let job = Job {
            format: JOB_FORMAT.into(),
            name: "original mixed fold fixture".into(),
            fabric: Fabric {
                width: 100.,
                layout: Layout::Folded,
                ..Default::default()
            },
            pieces: vec![make("body", 80., 80.), make("small", 40., 40.)],
            ..Default::default()
        };
        let candidates = mixed_policies(&job, &[])?;
        let policy = candidates
            .iter()
            .find(|slots| {
                slots.iter().any(|s| s.piece == 0 && !s.placement.on_fold)
                    && slots.iter().any(|s| s.piece == 1 && s.placement.on_fold)
            })
            .expect("per-piece policy");
        let dirs = directions(&job, policy, &[])?;
        let result = bounded::pack(
            &job,
            policy,
            &[],
            &NestOptions {
                time_limit_s: 0.2,
                seed: Some(1),
                ..Default::default()
            },
            0,
            &dirs,
            &mut |_| {},
            None,
        )?;
        assert!(beta_marker_core::validate(&job, &result).ok);
        assert_eq!(result.surplus_copies, 0);
        assert!(result.length < 81., "{}", result.length);
        Ok(())
    }
}
