use crate::{assign_bundles, bundle_for, bundles, identity, nest_matching, refresh};
use anyhow::{Context, Result, bail, ensure};
use beta_marker_core::geometry::{area, bbox};
use beta_marker_core::{Job, MarkerResult, Order, Placement, ShadeAxis};
use beta_marker_nest::NestOptions;
use std::collections::{BTreeMap, BTreeSet};

fn garment_area(job: &Job, size: &Option<String>) -> f64 {
    job.pieces
        .iter()
        .filter(|p| job.order.is_none() || p.size == *size)
        .map(|p| area(&p.outline) * f64::from(p.quantity + p.mirror_quantity))
        .sum()
}

fn minimum_width(job: &Job, size: &Option<String>) -> f64 {
    job.pieces
        .iter()
        .filter(|p| job.order.is_none() || p.size == *size)
        .map(|p| {
            [(false, p.quantity), (true, p.mirror_quantity)]
                .into_iter()
                .filter(|(_, n)| *n > 0)
                .map(|(mirrored, _)| {
                    let allowed = beta_marker_core::allowed_rotations(job, p, mirrored);
                    let angles = if allowed.free {
                        vec![0.0, 90.0, 180.0, 270.0]
                    } else {
                        allowed.angles
                    };
                    angles
                        .into_iter()
                        .filter_map(|a| {
                            bbox(&crate::search::outline(
                                p,
                                &Placement {
                                    mirrored,
                                    rotation_deg: a,
                                    ..Default::default()
                                },
                            ))
                        })
                        .map(|b| b[3] - b[1] + 2.0 * (job.spacing.edge_gap + p.buffer) + 0.1)
                        .fold(f64::INFINITY, f64::min)
                })
                .fold(0.0, f64::max)
        })
        .fold(0.0, f64::max)
}

/// Global IDs already use natural size order, independent of bin insertion order.
fn local_order(selected: &[usize]) -> Vec<usize> {
    let mut ids = selected.to_vec();
    ids.sort_unstable();
    ids
}

fn submarker(
    job: &Job,
    opts: &NestOptions,
    all: &[Option<String>],
    selected: &[usize],
    band: usize,
) -> Result<MarkerResult> {
    let shade = job.fabric.shade.context("missing shade rule")?;
    let ids = local_order(selected);
    let axis = usize::from(shade.axis == ShadeAxis::Width);
    let mut shift = [0.0; 2];
    shift[axis] = band as f64 * shade.band_mm;
    let mut sub = job.clone();
    sub.fabric.shade = None;
    // Neighbouring sub-markers together must reserve the global piece gap.
    sub.spacing.edge_gap = job.spacing.edge_gap.max(job.spacing.piece_gap / 2.0);
    let end_clearance = job.spacing.piece_gap / 2.0
        + job.pieces.iter().map(|p| p.buffer).fold(0.0, f64::max)
        + 0.025;
    sub.fabric.max_length = match shade.axis {
        ShadeAxis::Length => Some(
            job.fabric
                .max_length
                .map_or(shade.band_mm, |m| (m - shift[0]).min(shade.band_mm))
                - end_clearance,
        ),
        ShadeAxis::Width => job.fabric.max_length,
    };
    if shade.axis == ShadeAxis::Width {
        sub.fabric.width = shade.band_mm.min(job.fabric.width - shift[1]);
    }
    if job.order.is_some() {
        let mut sizes = BTreeMap::new();
        for &id in &ids {
            if let Some(size) = &all[id] {
                *sizes.entry(size.clone()).or_insert(0_u32) += 1;
            }
        }
        sub.pieces
            .retain(|p| p.size.as_ref().is_some_and(|s| sizes.contains_key(s)));
        sub.order = Some(Order { sizes });
        sub.match_rules.retain(|r| {
            sub.pieces.iter().any(|p| p.id == r.a.piece_id)
                && r.b
                    .as_ref()
                    .is_none_or(|b| sub.pieces.iter().any(|p| p.id == b.piece_id))
        });
    }
    if let Some(repeat) = &mut sub.fabric.repeat {
        repeat.offset_x -= shift[0];
        repeat.offset_y -= shift[1];
    }
    // Keep only defect zones whose bounds intersect this section. Their original
    // polygons are translated, so a flaw crossing a band boundary stays a flaw.
    sub.fabric.defects.retain(|z| {
        bbox(&z.polygon).is_some_and(|b| {
            b[2] >= shift[0]
                && b[0] <= sub.fabric.max_length.unwrap_or(f64::INFINITY) + shift[0]
                && b[3] >= shift[1]
                && b[1] <= sub.fabric.width + shift[1]
        })
    });
    for z in &mut sub.fabric.defects {
        for p in &mut z.polygon {
            p[0] -= shift[0];
            p[1] -= shift[1];
        }
    }
    let mut options = opts.clone();
    options.warm_start = None;
    options.fixed.clear();
    let mut local_counts = BTreeMap::<(String, bool), u32>::new();
    let mut fixed_copies = BTreeMap::<(String, bool, u32), u32>::new();
    for pl in &opts.fixed {
        let global = pl
            .bundle
            .or_else(|| bundle_for(job, pl))
            .context("fixed piece needs a valid bundle")? as usize;
        if let Some(local) = ids.iter().position(|&id| id == global) {
            let mut p = pl.clone();
            let next = local_counts
                .entry((p.piece_id.clone(), p.mirrored))
                .or_default();
            p.copy = *next;
            *next = next.checked_add(1).context("too many fixed copies")?;
            fixed_copies.insert((p.piece_id.clone(), p.mirrored, p.copy), pl.copy);
            p.bundle = Some(local as u32);
            p.translate[0] -= shift[0];
            p.translate[1] -= shift[1];
            options.fixed.push(p);
        }
    }
    let mut result = nest_matching(&sub, &options)?;
    assign_bundles(&sub, &mut result.placements);
    for pl in &mut result.placements {
        let local = pl.bundle.context("sub-marker has no bundle")? as usize;
        let &global = ids
            .get(local)
            .context("sub-marker returned an unknown bundle")?;
        // A freely reflected copy can have mirror_quantity == 0. Physical
        // identities must not be inferred by dividing by that fixed quantity.
        // Restore user identities, then number all remaining joined copies.
        pl.copy = fixed_copies
            .get(&(pl.piece_id.clone(), pl.mirrored, pl.copy))
            .copied()
            .unwrap_or(0);
        pl.bundle = Some(global as u32);
        pl.translate[0] += shift[0];
        pl.translate[1] += shift[1];
    }
    Ok(result)
}

pub(crate) fn nest_bands(job: &Job, opts: &NestOptions) -> Result<MarkerResult> {
    nest_bands_search(job, opts, 1)
}

pub(crate) fn nest_bands_search(
    job: &Job,
    opts: &NestOptions,
    attempts: usize,
) -> Result<MarkerResult> {
    ensure!(
        (1..=16).contains(&attempts),
        "shade assignment attempts must be between 1 and 16"
    );
    if attempts == 1 {
        return nest_bands_once(job, opts, 0, None);
    }
    let started = crate::Instant::now();
    let mut best: Option<MarkerResult> = None;
    let mut failures = Vec::new();
    for variant in 0..attempts {
        let mut probe = opts.clone();
        probe.time_limit_s = opts.time_limit_s * 0.35 / attempts as f64;
        probe.seed = Some(opts.seed.unwrap_or(1).wrapping_add(variant as u64));
        match nest_bands_once(job, &probe, variant, None) {
            Ok(result)
                if beta_marker_core::validate(job, &result).ok
                    && crate::checks::shade_errors(job, &result).is_empty() =>
            {
                if best.as_ref().is_none_or(|old| result.length < old.length) {
                    best = Some(result);
                }
            }
            Ok(_) => failures.push("assignment did not validate".into()),
            Err(error) => failures.push(error.to_string()),
        }
    }
    let Some(mut result) = best else {
        bail!("shade assignment searches failed: {}", failures.join("; "));
    };
    let shade = job.fabric.shade.context("missing shade rule")?;
    let axis = usize::from(shade.axis == ShadeAxis::Width);
    let assignment: BTreeMap<usize, usize> = result
        .placements
        .iter()
        .filter_map(|pl| {
            let piece = job.pieces.iter().find(|p| p.id == pl.piece_id)?;
            let b = bbox(&crate::search::outline(piece, pl))?;
            Some((
                pl.bundle? as usize,
                ((b[axis] + 1e-7) / shade.band_mm).floor() as usize,
            ))
        })
        .collect();
    let mut finish = opts.clone();
    finish.time_limit_s = (opts.time_limit_s - started.elapsed().as_secs_f64()).max(0.01);
    if let Ok(candidate) = nest_bands_once(job, &finish, 0, Some(&assignment))
        && candidate.length < result.length
        && beta_marker_core::validate(job, &candidate).ok
        && crate::checks::shade_errors(job, &candidate).is_empty()
    {
        result = candidate;
    }
    result.warnings.push(format!(
        "Shade assignment search: {attempts} probes, {} failed; best valid retained.",
        failures.len()
    ));
    Ok(result)
}

fn nest_bands_once(
    job: &Job,
    opts: &NestOptions,
    variant: usize,
    assignment: Option<&BTreeMap<usize, usize>>,
) -> Result<MarkerResult> {
    let shade = job.fabric.shade.context("missing shade rule")?;
    let all = bundles(job);
    let active: Vec<_> = (0..all.len())
        .filter(|&i| garment_area(job, &all[i]) > 0.0)
        .collect();
    ensure!(!active.is_empty(), "no garments to place in shade bands");
    let mut forced = BTreeMap::<usize, usize>::new();
    for p in &opts.fixed {
        let global = p
            .bundle
            .or_else(|| bundle_for(job, p))
            .context("fixed piece needs a bundle")? as usize;
        ensure!(global < all.len(), "fixed piece has an unknown bundle");
        let piece = job
            .pieces
            .iter()
            .find(|piece| piece.id == p.piece_id)
            .context("unknown fixed piece")?;
        let b = bbox(&crate::search::outline(piece, p)).context("empty fixed piece")?;
        let axis = usize::from(shade.axis == ShadeAxis::Width);
        let band = (b[axis] / shade.band_mm).floor();
        ensure!(
            (0.0..=100_000.0).contains(&band) && b[axis + 2] <= (band + 1.0) * shade.band_mm + 1e-7,
            "fixed piece crosses a shade band"
        );
        let old = forced.insert(global, band as usize);
        ensure!(
            old.is_none_or(|old| old == band as usize),
            "fixed bundle crosses shade bands"
        );
    }
    if let Some(assignment) = assignment {
        for (&bundle, &band) in assignment {
            forced.entry(bundle).or_insert(band);
        }
    }
    let initial = match shade.axis {
        ShadeAxis::Length => {
            let total: f64 = active.iter().map(|&id| garment_area(job, &all[id])).sum();
            (total / (job.fabric.width * shade.band_mm * 0.78))
                .ceil()
                .max(1.0) as usize
        }
        ShadeAxis::Width => (job.fabric.width / shade.band_mm).ceil() as usize,
    };
    ensure!(initial <= 100_000, "too many shade sections");
    let count = initial.max(forced.values().copied().max().map_or(0, |k| k + 1));
    ensure!(
        shade.axis != ShadeAxis::Width || count == initial,
        "fixed piece lies outside width lanes"
    );
    let mut bins = vec![vec![]; count];
    let mut loads = vec![0.0; count];
    for (&id, &band) in &forced {
        bins[band].push(id);
        loads[band] += garment_area(job, &all[id]);
    }
    let mut pending: Vec<_> = active
        .into_iter()
        .filter(|id| !forced.contains_key(id))
        .collect();
    pending.sort_by(|&a, &b| garment_area(job, &all[b]).total_cmp(&garment_area(job, &all[a])));
    if variant % 4 == 2 {
        pending.reverse();
    }
    if variant % 4 == 3 {
        pending.sort_by_key(|&id| {
            (id as u64 ^ opts.seed.unwrap_or(1)).wrapping_mul(0x9e3779b97f4a7c15)
        });
    }
    for id in pending {
        let area = garment_area(job, &all[id]);
        let band = match shade.axis {
            ShadeAxis::Length => {
                // 78% is a starting estimate, not a feasibility test. Failed bins
                // below are split and re-nested until each holds whole garments.
                let capacity = job.fabric.width * shade.band_mm * 0.78;
                (0..bins.len())
                    .filter(|&i| loads[i] + area <= capacity || bins[i].is_empty())
                    .min_by(|&a, &b| {
                        if variant % 4 == 1 {
                            loads[b].total_cmp(&loads[a])
                        } else {
                            loads[a].total_cmp(&loads[b])
                        }
                    })
                    .unwrap_or_else(|| {
                        bins.push(vec![]);
                        loads.push(0.0);
                        bins.len() - 1
                    })
            }
            ShadeAxis::Width => {
                let min = minimum_width(job, &all[id]);
                (0..bins.len())
                    .filter(|&i| {
                        shade
                            .band_mm
                            .min(job.fabric.width - i as f64 * shade.band_mm)
                            >= min
                    })
                    .min_by(|&a, &b| {
                        let width = |i| {
                            shade
                                .band_mm
                                .min(job.fabric.width - i as f64 * shade.band_mm)
                        };
                        (loads[a] / width(a)).total_cmp(&(loads[b] / width(b)))
                    })
                    .context("no width lane can hold all pieces of a garment")?
            }
        };
        bins[band].push(id);
        loads[band] += area;
    }
    let mut placements = Vec::new();
    let mut warnings = Vec::new();
    let mut band = 0;
    let mut options = opts.clone();
    let mut remaining_budget = opts.time_limit_s;
    while band < bins.len() {
        if bins[band].is_empty() {
            band += 1;
            continue;
        }
        let pending = bins[band..].iter().filter(|bin| !bin.is_empty()).count();
        // Share optimizer time between the sections still to run. Failed
        // attempts consume their share too. Keep a small construction budget
        // after a split exhausts the original allowance.
        options.time_limit_s = (remaining_budget / pending as f64).max(0.01);
        remaining_budget = (remaining_budget - options.time_limit_s).max(0.0);
        match submarker(job, &options, &all, &bins[band], band) {
            Ok(result) => {
                placements.extend(result.placements);
                warnings.extend(result.warnings);
                band += 1;
            }
            Err(error) if shade.axis == ShadeAxis::Length && bins[band].len() > 1 => {
                let movable = bins[band].iter().rposition(|id| !forced.contains_key(id));
                let Some(index) = movable else {
                    return Err(error.context("fixed shade band cannot hold its bundles"));
                };
                let moved = bins[band].remove(index);
                let capacity = job.fabric.width * shade.band_mm * 0.78;
                let moved_area = garment_area(job, &all[moved]);
                let target = (band + 1..bins.len()).find(|&i| {
                    let load: f64 = bins[i].iter().map(|&id| garment_area(job, &all[id])).sum();
                    load + moved_area <= capacity
                });
                if let Some(target) = target {
                    bins[target].push(moved);
                } else {
                    bins.push(vec![moved]);
                }
            }
            Err(error) => bail!("shade band {band} could not be nested: {error:#}"),
        }
    }
    let mut result = MarkerResult {
        format: beta_marker_core::RESULT_FORMAT.into(),
        job_name: job.name.clone(),
        width: job.fabric.width,
        length: 0.0,
        piece_area: 0.0,
        efficiency: 0.0,
        engine: "whole-bundle shade sub-markers + beta-marker-nest".into(),
        seed: opts.seed,
        runtime_s: 0.0,
        placements,
        warnings,
    };
    // Fixed-length bands have an unavoidable boundary cost. Put the section
    // with the shortest measured tail last, where unused fabric is not charged.
    // Skip when absolute coordinates carry meaning (locks, defects or matches).
    if shade.axis == ShadeAxis::Length
        && opts.fixed.is_empty()
        && job.fabric.defects.is_empty()
        && job.match_rules.is_empty()
    {
        shortest_tail_last(job, &mut result, shade.band_mm);
    }
    number_joined_copies(&mut result.placements, &opts.fixed)?;
    refresh(job, &mut result);
    for fixed in &opts.fixed {
        let p = result
            .placements
            .iter()
            .find(|p| identity(p) == identity(fixed))
            .context("fixed shade placement disappeared")?;
        ensure!(
            p.translate
                .iter()
                .zip(fixed.translate)
                .all(|(a, b)| (*a - b).abs() < 1e-8)
                && p.rotation_deg == fixed.rotation_deg,
            "fixed shade piece moved"
        );
    }
    Ok(result)
}

fn shortest_tail_last(job: &Job, result: &mut MarkerResult, band_mm: f64) {
    let mut used = BTreeMap::<usize, f64>::new();
    for pl in &result.placements {
        if let Some(piece) = job.pieces.iter().find(|p| p.id == pl.piece_id)
            && let Some(b) = bbox(&crate::search::outline(piece, pl))
        {
            let band = ((b[0] + 1e-7) / band_mm).floor() as usize;
            let end = used.entry(band).or_default();
            *end = end.max(b[2] - band as f64 * band_mm);
        }
    }
    let Some((&shortest, _)) = used.iter().min_by(|a, b| a.1.total_cmp(b.1)) else {
        return;
    };
    let Some(&last) = used.keys().last() else {
        return;
    };
    if shortest == last {
        return;
    }
    for pl in &mut result.placements {
        let Some(piece) = job.pieces.iter().find(|p| p.id == pl.piece_id) else {
            continue;
        };
        let Some(b) = bbox(&crate::search::outline(piece, pl)) else {
            continue;
        };
        let band = ((b[0] + 1e-7) / band_mm).floor() as usize;
        if band == shortest {
            pl.translate[0] += (last - shortest) as f64 * band_mm;
        } else if band == last {
            pl.translate[0] -= (last - shortest) as f64 * band_mm;
        }
    }
}

fn number_joined_copies(placements: &mut [Placement], fixed: &[Placement]) -> Result<()> {
    let mut used = BTreeMap::<(String, bool), BTreeSet<u32>>::new();
    for p in fixed {
        used.entry((p.piece_id.clone(), p.mirrored))
            .or_default()
            .insert(p.copy);
    }
    for p in placements {
        let is_fixed = fixed.iter().any(|f| {
            f.piece_id == p.piece_id
                && f.mirrored == p.mirrored
                && f.copy == p.copy
                && f.bundle.is_none_or(|b| p.bundle == Some(b))
                && f.rotation_deg == p.rotation_deg
                && f.translate
                    .iter()
                    .zip(p.translate)
                    .all(|(a, b)| (a - b).abs() < 1e-8)
        });
        if is_fixed {
            continue;
        }
        let copies = used.entry((p.piece_id.clone(), p.mirrored)).or_default();
        let mut copy = 0_u32;
        while copies.contains(&copy) {
            copy = copy.checked_add(1).context("too many joined copies")?;
        }
        p.copy = copy;
        copies.insert(copy);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shorter_measured_tail_is_moved_last() {
        let job = Job {
            pieces: vec![beta_marker_core::Piece {
                id: "p".into(),
                outline: vec![[0.0, 0.0], [20.0, 0.0], [20.0, 10.0], [0.0, 10.0]],
                ..Default::default()
            }],
            ..Default::default()
        };
        let mut result = MarkerResult {
            format: String::new(),
            job_name: String::new(),
            width: 100.0,
            length: 290.0,
            piece_area: 0.0,
            efficiency: 0.0,
            engine: String::new(),
            seed: None,
            runtime_s: 0.0,
            warnings: vec![],
            placements: vec![
                Placement {
                    piece_id: "p".into(),
                    translate: [1.0, 0.0],
                    bundle: Some(0),
                    ..Default::default()
                },
                Placement {
                    piece_id: "p".into(),
                    translate: [170.0, 0.0],
                    bundle: Some(1),
                    ..Default::default()
                },
                Placement {
                    piece_id: "p".into(),
                    translate: [270.0, 0.0],
                    bundle: Some(2),
                    ..Default::default()
                },
            ],
        };
        shortest_tail_last(&job, &mut result, 100.0);
        assert_eq!(result.placements[0].translate[0], 201.0);
        assert_eq!(result.placements[2].translate[0], 70.0);
        assert_eq!(result.placements[0].bundle, Some(0));
        assert_eq!(result.placements[2].bundle, Some(2));
    }
}
