//! Bounded obstacle construction and incremental compaction. Search never owns
//! the only complete layout: a checked incumbent survives every interruption.
use super::{NestOptions, NestProgress, Slot, checks, mirror_state, renumber};
use anyhow::{Context, Result, ensure};
use beta_marker_core::geometry::{area, bbox, grain_alignment_deg, norm_deg, transform_points};
use beta_marker_core::{Job, Layout, MarkerResult, Nap, Placement, Pt, RESULT_FORMAT};
use geo::{Distance, Euclidean, Polygon, Relate};
use rand::{RngExt, SeedableRng, rngs::Xoshiro256PlusPlus, seq::SliceRandom};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[derive(Clone)]
struct Shape {
    points: Vec<Pt>,
    bounds: [f64; 4],
    polygon: Polygon<f64>,
}

impl Shape {
    fn new(points: Vec<Pt>) -> Result<Self> {
        Ok(Self {
            bounds: bbox(&points).context("empty cut outline")?,
            polygon: crate::safety::polygon(&points),
            points,
        })
    }

    fn translated(&self, x: f64, y: f64) -> Self {
        let points: Vec<_> = self.points.iter().map(|p| [p[0] + x, p[1] + y]).collect();
        Self {
            bounds: [
                self.bounds[0] + x,
                self.bounds[1] + y,
                self.bounds[2] + x,
                self.bounds[3] + y,
            ],
            polygon: crate::safety::polygon(&points),
            points,
        }
    }
}

#[derive(Clone)]
struct Obstacle {
    shape: Shape,
    buffer: f64,
    defect: bool,
}

fn boxes_conflict(a: [f64; 4], b: [f64; 4], gap: f64) -> bool {
    a[0] < b[2] + gap && b[0] < a[2] + gap && a[1] < b[3] + gap && b[1] < a[3] + gap
}

fn conflict(a: &Shape, b: &Shape, gap: f64) -> bool {
    if !boxes_conflict(a.bounds, b.bounds, gap + 1e-9) {
        return false;
    }
    a.polygon
        .relate(&b.polygon)
        .matches("T********")
        .unwrap_or(true)
        || (gap > 0.0 && Euclidean.distance(&a.polygon, &b.polygon) + 1e-8 < gap)
}

struct Limit<'a> {
    deadline: Instant,
    cancel: Option<&'a AtomicBool>,
}

impl Limit<'_> {
    fn stopped(&self) -> bool {
        Instant::now() >= self.deadline || self.cancel.is_some_and(|c| c.load(Ordering::Relaxed))
    }
}

#[derive(Clone)]
struct Orientation {
    placement: Placement,
    shape: Shape,
    ys: [f64; 2],
    fold: bool,
}

fn orientations(
    job: &Job,
    slot: &Slot,
    variant: usize,
    directions: &BTreeMap<u32, f64>,
) -> Result<Vec<Orientation>> {
    let piece = &job.pieces[slot.piece];
    let preferred = mirror_state(job, slot, variant);
    // A full cut through folded layers already cuts a normal/mirrored pair.
    // When that pair is requested, either primary hand has the same demand.
    // This does not grant optional face-down cutting to a one-layer piece.
    let mirrors = if beta_marker_core::rules::flip_allowed(job, piece)
        || (job.fabric.layout == Layout::Folded && piece.mirror_quantity > 0)
    {
        vec![preferred, !preferred]
    } else {
        vec![preferred]
    };
    let folds = if slot.placement.on_fold {
        vec![true, false]
    } else {
        vec![false]
    };
    let mut out = Vec::new();
    for on_fold in folds {
        for &mirrored in &mirrors {
            let mut pl = slot.placement.clone();
            pl.on_fold = on_fold;
            pl.mirrored = mirrored;
            let allowed = beta_marker_core::allowed_rotations(job, piece, mirrored);
            let mut angles = if job.fabric.nap == Nap::OneWayPerBundle {
                vec![norm_deg(
                    grain_alignment_deg(piece.grain, mirrored)
                        + directions[&pl.bundle.context("slot without bundle")?],
                )]
            } else if allowed.free {
                (0..24).map(|i| f64::from(i) * 15.0).collect()
            } else {
                let mut a = allowed.angles.clone();
                if allowed.tilt_deg > 0.0 {
                    a.extend(allowed.angles.iter().flat_map(|&a| {
                        [
                            norm_deg(a - allowed.tilt_deg),
                            norm_deg(a + allowed.tilt_deg),
                        ]
                    }));
                }
                a
            };
            if on_fold && job.fabric.nap != Nap::OneWayPerBundle {
                let line = transform_points(
                    &piece.fold_line.context("missing fold line")?,
                    mirrored,
                    0.0,
                    [0.0, 0.0],
                );
                let a = -(line[1][1] - line[0][1])
                    .atan2(line[1][0] - line[0][0])
                    .to_degrees();
                angles.extend(
                    [norm_deg(a), norm_deg(a + 180.0)]
                        .into_iter()
                        .filter(|a| allowed.allows(*a)),
                );
            }
            angles.sort_by(f64::total_cmp);
            angles.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
            for rotation in angles {
                pl.rotation_deg = rotation;
                pl.translate = [0.0, 0.0];
                let shape = Shape::new(checks::outline(piece, &pl)?)?;
                let b = shape.bounds;
                let edge = job.spacing.edge_gap + piece.buffer;
                let low = if job.fabric.layout == Layout::Tubular {
                    0.0
                } else {
                    edge
                };
                let high = if job.fabric.layout == Layout::Open {
                    job.fabric.width - edge
                } else {
                    job.fabric.width
                };
                if b[3] - b[1] > high - low + 1e-8 {
                    continue;
                }
                let mut ys = [low - b[1], high - b[3]];
                if on_fold {
                    let line = transform_points(
                        &piece.fold_line.context("missing fold line")?,
                        mirrored,
                        rotation,
                        [0.0, 0.0],
                    );
                    if (line[0][1] - line[1][1]).abs() > 1e-6 {
                        continue;
                    }
                    ys = [
                        job.fabric.width - line[0][1],
                        if job.fabric.layout == Layout::Tubular {
                            -line[0][1]
                        } else {
                            job.fabric.width - line[0][1]
                        },
                    ];
                }
                // Some fold orientations place the half outside the spread.
                if !ys
                    .iter()
                    .any(|y| b[1] + y >= low - 1e-8 && b[3] + y <= high + 1e-8)
                {
                    continue;
                }
                out.push(Orientation {
                    placement: pl.clone(),
                    shape,
                    ys,
                    fold: on_fold,
                });
            }
        }
    }
    ensure!(
        !out.is_empty(),
        "piece '{}' cannot fit the usable width or fold direction",
        piece.id
    );
    Ok(out)
}

#[derive(Clone)]
struct Placed {
    placement: Placement,
    obstacle: Obstacle,
}

fn gap(job: &Job, buffer: f64, ob: &Obstacle) -> f64 {
    if ob.defect {
        0.0
    } else {
        job.spacing.piece_gap + buffer + ob.buffer
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Boxes,
    Exact,
    Shelf,
}

struct Search<'a> {
    job: &'a Job,
    limit: Limit<'a>,
}

impl Search<'_> {
    // Return the best completed placement when interrupted. The caller keeps
    // the old placement if this search found no safe improvement.
    fn place(
        &self,
        choices: &[Orientation],
        buffer: f64,
        obstacles: &[&Obstacle],
        length: f64,
        mode: Mode,
        slice: Instant,
    ) -> Option<Placed> {
        if mode == Mode::Boxes && self.limit.stopped() {
            return self.place(choices, buffer, obstacles, length, Mode::Shelf, slice);
        }
        let job = self.job;
        let edge = job.spacing.edge_gap + buffer;
        let mut best: Option<(f64, f64, f64, Placed)> = None;
        for choice in choices {
            if mode == Mode::Exact && (self.limit.stopped() || Instant::now() >= slice) {
                break;
            }
            let b = choice.shape.bounds;
            let low = if job.fabric.layout == Layout::Tubular {
                0.0
            } else {
                edge
            };
            let high = if job.fabric.layout == Layout::Open {
                job.fabric.width - edge
            } else {
                job.fabric.width
            };
            let mut ys = choice.ys.to_vec();
            if !choice.fold && mode != Mode::Shelf {
                for ob in obstacles {
                    let g = gap(job, buffer, ob) + 0.002;
                    ys.extend([
                        ob.shape.bounds[1] - g - b[3],
                        ob.shape.bounds[3] + g - b[1],
                        ob.shape.bounds[1] - b[1],
                        ob.shape.bounds[3] - b[3],
                    ]);
                }
                // Fixed uniform samples make the search independent of point
                // density and reach gaps not aligned with a bounding box.
                if mode == Mode::Exact {
                    ys.extend(
                        (1..16).map(|i| {
                            low - b[1] + (high - low - (b[3] - b[1])) * f64::from(i) / 16.0
                        }),
                    );
                }
            }
            ys.retain(|y| b[1] + y >= low - 1e-8 && b[3] + y <= high + 1e-8);
            ys.sort_by(f64::total_cmp);
            ys.dedup_by(|a, b| (*a - *b).abs() < 1e-6);
            if mode == Mode::Exact && ys.len() > 48 {
                ys = (0..48).map(|i| ys[i * (ys.len() - 1) / 47]).collect();
            }
            for y in ys {
                if mode == Mode::Boxes && self.limit.stopped() {
                    return best.map(|b| b.3).or_else(|| {
                        self.place(choices, buffer, obstacles, length, Mode::Shelf, slice)
                    });
                }
                if mode == Mode::Exact && (self.limit.stopped() || Instant::now() >= slice) {
                    break;
                }
                let mut x = edge - b[0];
                if mode == Mode::Shelf {
                    x = obstacles
                        .iter()
                        .map(|o| o.shape.bounds[2] + gap(job, buffer, o) + 0.002 - b[0])
                        .fold(x, f64::max);
                }
                let mut found = None;
                for _ in 0..=obstacles.len() + 1 {
                    if mode == Mode::Exact && (self.limit.stopped() || Instant::now() >= slice) {
                        break;
                    }
                    let bounds = [b[0] + x, b[1] + y, b[2] + x, b[3] + y];
                    let mut shape = None;
                    let mut blocker = None;
                    for &ob in obstacles {
                        if mode == Mode::Exact && (self.limit.stopped() || Instant::now() >= slice)
                        {
                            return best.map(|b| b.3);
                        }
                        let g = gap(job, buffer, ob);
                        if !boxes_conflict(bounds, ob.shape.bounds, g + 0.001) {
                            continue;
                        }
                        if mode != Mode::Exact
                            || conflict(
                                shape.get_or_insert_with(|| choice.shape.translated(x, y)),
                                &ob.shape,
                                g,
                            )
                        {
                            blocker = Some(ob);
                            break;
                        }
                    }
                    if let Some(ob) = blocker {
                        let g = gap(job, buffer, ob) + 0.002;
                        let next = ob.shape.bounds[2] + g - b[0];
                        if next <= x + 1e-9 {
                            break;
                        }
                        if mode == Mode::Exact {
                            let (mut left, mut right) = (x, next);
                            for _ in 0..10 {
                                if self.limit.stopped() || Instant::now() >= slice {
                                    return best.map(|b| b.3);
                                }
                                let mid = (left + right) / 2.0;
                                if conflict(&choice.shape.translated(mid, y), &ob.shape, g) {
                                    left = mid;
                                } else {
                                    right = mid;
                                }
                            }
                            x = right;
                        } else {
                            x = next;
                        }
                    } else {
                        found = Some(shape.unwrap_or_else(|| choice.shape.translated(x, y)));
                        break;
                    }
                }
                if let Some(shape) = found {
                    let mut pl = choice.placement.clone();
                    pl.translate = [x, y];
                    let end = shape.bounds[2];
                    let score = length.max(end);
                    let used_area = area(&shape.points);
                    if job.fabric.max_length.is_some_and(|max| score > max + 0.01) {
                        continue;
                    }
                    if best.as_ref().is_none_or(|(old, old_end, old_area, _)| {
                        score < *old - 1e-7
                            || ((score - old).abs() < 1e-7
                                && (end < *old_end - 1e-7
                                    || ((end - old_end).abs() < 1e-7 && used_area < *old_area)))
                    }) {
                        best = Some((
                            score,
                            end,
                            used_area,
                            Placed {
                                placement: pl,
                                obstacle: Obstacle {
                                    shape,
                                    buffer,
                                    defect: false,
                                },
                            },
                        ));
                    }
                }
            }
        }
        best.map(|b| b.3)
    }
}

fn marker(
    job: &Job,
    fixed: &[Placement],
    placed: &[Placed],
    seed: Option<u64>,
) -> Result<MarkerResult> {
    let mut placements = fixed.to_vec();
    placements.extend(placed.iter().map(|p| p.placement.clone()));
    renumber(&mut placements, fixed);
    let mut result = MarkerResult {
        format: RESULT_FORMAT.into(),
        job_name: job.name.clone(),
        width: job.fabric.width,
        length: 0.0,
        piece_area: 0.0,
        efficiency: 0.0,
        engine: "beta-marker bounded obstacle packer 0.4".into(),
        seed,
        runtime_s: 0.0,
        placements,
        warnings: Vec::new(),
    };
    checks::refresh(job, &mut result)?;
    Ok(result)
}

fn checked(job: &Job, result: &MarkerResult) -> Result<()> {
    let _phase = crate::profile::Phase::new("obstacle_validate");
    checks::complete(job, result)?;
    checks::independently_validate(job, result)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn pack(
    job: &Job,
    slots: &[Slot],
    fixed: &[Placement],
    opts: &NestOptions,
    variant: usize,
    directions: &BTreeMap<u32, f64>,
    progress: &mut dyn FnMut(NestProgress),
    cancel: Option<&AtomicBool>,
) -> Result<MarkerResult> {
    let _phase = crate::profile::Phase::new("obstacle_pack");
    let started = Instant::now();
    let duration = Duration::try_from_secs_f64(opts.time_limit_s).context("invalid time budget")?;
    let deadline = started
        .checked_add(duration)
        .context("time budget too large")?;
    let search = Search {
        job,
        limit: Limit { deadline, cancel },
    };
    let mut remaining = slots.to_vec();
    let mut obstacles = Vec::new();
    for zone in &job.fabric.defects {
        obstacles.push(Obstacle {
            shape: Shape::new(zone.polygon.clone())?,
            buffer: 0.0,
            defect: true,
        });
    }
    for pl in fixed {
        let i = remaining
            .iter()
            .position(|s| {
                s.placement.piece_id == pl.piece_id
                    && s.placement.bundle == pl.bundle
                    && (beta_marker_core::rules::flip_allowed(job, &job.pieces[s.piece])
                        || job.fabric.layout != Layout::Open
                        || s.placement.mirrored == pl.mirrored)
                    && (!pl.on_fold || s.placement.on_fold)
            })
            .context("fixed demand has no matching slot")?;
        let slot = remaining.remove(i);
        let piece = &job.pieces[slot.piece];
        obstacles.push(Obstacle {
            shape: Shape::new(checks::outline(piece, pl)?)?,
            buffer: piece.buffer,
            defect: false,
        });
    }
    let fixed_length = obstacles
        .iter()
        .filter(|o| !o.defect)
        .map(|o| o.shape.bounds[2])
        .fold(0.0, f64::max);
    let mut best = opts
        .warm_start
        .clone()
        .filter(|r| fixed.iter().all(|p| r.placements.contains(p)));
    // Warm starts have already passed the public gate. Preparation may stop
    // immediately if one is available; otherwise build a cheap complete seed.
    if best.is_some() && search.limit.stopped() {
        let mut result = best.context("missing incumbent")?;
        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            result.warnings.push(
                "Search cancelled; best independently valid complete marker retained.".into(),
            );
        }
        return Ok(result);
    }
    let prepare_started = Instant::now();
    let mut prepared = Vec::new();
    let mut cache: BTreeMap<(usize, bool, bool, u64), Vec<Orientation>> = BTreeMap::new();
    for slot in &remaining {
        let direction = directions
            .get(&slot.placement.bundle.unwrap_or(0))
            .copied()
            .unwrap_or(0.0);
        let key = (
            slot.piece,
            slot.placement.on_fold,
            mirror_state(job, slot, variant),
            direction.to_bits(),
        );
        let choices = if let Some(choices) = cache.get(&key) {
            choices
        } else {
            cache.insert(key, orientations(job, slot, variant, directions)?);
            &cache[&key]
        };
        let mut choices: Vec<Orientation> = choices.clone();
        for choice in &mut choices {
            choice.placement.copy = slot.placement.copy;
            choice.placement.bundle = slot.placement.bundle;
        }
        prepared.push(choices);
    }
    let prepare_s = prepare_started.elapsed().as_secs_f64();
    let mut order: Vec<usize> = (0..remaining.len()).collect();
    order.sort_by(|&a, &b| {
        remaining[b]
            .placement
            .on_fold
            .cmp(&remaining[a].placement.on_fold)
            .then_with(|| {
                area(&prepared[b][0].shape.points).total_cmp(&area(&prepared[a][0].shape.points))
            })
    });
    let construct_started = Instant::now();
    let mut placed = Vec::new();
    let mut placed_slots = Vec::new();
    let mut length = fixed_length;
    let mut construction_error = None;
    for &i in &order {
        let all: Vec<_> = obstacles
            .iter()
            .chain(placed.iter().map(|p: &Placed| &p.obstacle))
            .collect();
        let mode = if search.limit.stopped() {
            Mode::Shelf
        } else {
            Mode::Boxes
        };
        if let Some(p) = search.place(
            &prepared[i],
            job.pieces[remaining[i].piece].buffer,
            &all,
            length,
            mode,
            deadline,
        ) {
            length = length.max(p.obstacle.shape.bounds[2]);
            placed.push(p);
            placed_slots.push(i);
        } else {
            construction_error = Some(format!(
                "could not construct piece '{}' within width/length constraints",
                remaining[i].placement.piece_id
            ));
            break;
        }
    }
    if construction_error.is_none() {
        let result = marker(job, fixed, &placed, opts.seed)?;
        match checked(job, &result) {
            Ok(()) if best.as_ref().is_none_or(|b| result.length < b.length) => best = Some(result),
            Err(error) => construction_error = Some(error.to_string()),
            _ => {}
        }
    }
    let construct_s = construct_started.elapsed().as_secs_f64();
    ensure!(
        best.is_some(),
        "constrained packing failed: {}",
        construction_error.unwrap_or_default()
    );
    let mut reported = f64::INFINITY;
    let mut passes = 0;
    let mut rejected = 0;
    let search_started = Instant::now();
    let mut rng = Xoshiro256PlusPlus::seed_from_u64(opts.seed.unwrap_or(0));
    while !search.limit.stopped() {
        let incumbent = best.as_ref().context("missing incumbent")?;
        if incumbent.length < reported {
            reported = incumbent.length;
            progress(NestProgress {
                seconds: started.elapsed().as_secs_f64(),
                length: reported,
            });
        }
        // Rebuild correspondence by piece/bundle (not renumbered copy), consuming
        // each slot once. Fixed identities remain outside the movable list.
        placed.clear();
        placed_slots.clear();
        let mut unused = vec![true; remaining.len()];
        for pl in &incumbent.placements {
            if fixed.contains(pl) {
                continue;
            }
            let i = remaining
                .iter()
                .enumerate()
                .position(|(i, s)| {
                    unused[i]
                        && s.placement.piece_id == pl.piece_id
                        && s.placement.bundle == pl.bundle
                        && (job.fabric.layout != Layout::Open
                            || beta_marker_core::rules::flip_allowed(job, &job.pieces[s.piece])
                            || s.placement.mirrored == pl.mirrored)
                })
                .context("incumbent demand has no matching slot")?;
            unused[i] = false;
            placed_slots.push(i);
            placed.push(Placed {
                placement: pl.clone(),
                obstacle: Obstacle {
                    shape: Shape::new(checks::outline(&job.pieces[remaining[i].piece], pl)?)?,
                    buffer: job.pieces[remaining[i].piece].buffer,
                    defect: false,
                },
            });
        }
        let mut move_order: Vec<_> = (0..placed.len()).collect();
        if passes % 3 == 2 {
            move_order.shuffle(&mut rng);
        } else {
            move_order.sort_by(|&a, &b| {
                placed[a].obstacle.shape.bounds[0].total_cmp(&placed[b].obstacle.shape.bounds[0])
            });
        }
        for index in move_order {
            if search.limit.stopped() {
                break;
            }
            let all: Vec<_> = obstacles
                .iter()
                .chain(
                    placed
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i != index)
                        .map(|(_, p)| &p.obstacle),
                )
                .collect();
            let other_length = all
                .iter()
                .filter(|o| !o.defect)
                .map(|o| o.shape.bounds[2])
                .fold(0.0, f64::max);
            let i = placed_slots[index];
            // A bounded slice stops a complex early piece from starving all later
            // pieces. Every accepted replacement leaves a complete valid marker.
            let slice = deadline.min(Instant::now() + Duration::from_millis(80));
            // One fold edge is scarce. Rotate which half/full orientation gets
            // the first bounded slice, so an expensive half cannot starve the
            // full two-layer alternatives on every compaction pass.
            let mut rotated;
            let choices = if job.fabric.layout == Layout::Folded {
                rotated = prepared[i].clone();
                let offset = passes % rotated.len();
                rotated.rotate_left(offset);
                &rotated
            } else {
                &prepared[i]
            };
            if let Some(candidate) = search.place(
                choices,
                job.pieces[remaining[i].piece].buffer,
                &all,
                other_length,
                Mode::Exact,
                slice,
            ) {
                let old_end = placed[index].obstacle.shape.bounds[2];
                let new_end = candidate.obstacle.shape.bounds[2];
                let new_area = area(&candidate.obstacle.shape.points);
                let old_area = area(&placed[index].obstacle.shape.points);
                if new_end < old_end - 1e-7
                    || ((new_end - old_end).abs() <= 1e-7
                        && (new_area < old_area - 1e-7
                            || (job.fabric.layout == Layout::Folded
                                && new_area <= old_area + 1e-7
                                && candidate.obstacle.shape.bounds[3]
                                    < placed[index].obstacle.shape.bounds[3] - 1e-7)))
                {
                    placed[index] = candidate;
                }
            }
        }
        let result = marker(job, fixed, &placed, opts.seed)?;
        if result.length <= best.as_ref().context("missing incumbent")?.length + 1e-7 {
            if checked(job, &result).is_ok() {
                best = Some(result);
            } else {
                rejected += 1;
            }
        }
        passes += 1;
        if remaining.is_empty() {
            break;
        }
        // Alternate the full/half choice in construction without discarding the
        // checked incumbent. Exact compaction above is always resumed from best.
        if !search.limit.stopped() && passes % 2 == 0 {
            order.shuffle(&mut rng);
            if rng.random_bool(0.5) {
                order.sort_by_key(|&i| !remaining[i].placement.on_fold);
            }
            let mut fresh: Vec<Placed> = Vec::new();
            let mut fresh_length = fixed_length;
            for &i in &order {
                if search.limit.stopped() {
                    break;
                }
                let all: Vec<_> = obstacles
                    .iter()
                    .chain(fresh.iter().map(|p| &p.obstacle))
                    .collect();
                let mode = if passes % 4 == 0 {
                    Mode::Exact
                } else {
                    Mode::Boxes
                };
                let slice_ms = if job.fabric.layout == Layout::Folded {
                    500
                } else {
                    40
                };
                let slice = deadline.min(Instant::now() + Duration::from_millis(slice_ms));
                let p = search
                    .place(
                        &prepared[i],
                        job.pieces[remaining[i].piece].buffer,
                        &all,
                        fresh_length,
                        mode,
                        slice,
                    )
                    .or_else(|| {
                        search.place(
                            &prepared[i],
                            job.pieces[remaining[i].piece].buffer,
                            &all,
                            fresh_length,
                            Mode::Boxes,
                            deadline,
                        )
                    });
                let Some(p) = p else { break };
                fresh_length = fresh_length.max(p.obstacle.shape.bounds[2]);
                fresh.push(p);
            }
            if fresh.len() == remaining.len()
                && fresh_length < best.as_ref().context("missing incumbent")?.length - 1e-7
            {
                let result = marker(job, fixed, &fresh, opts.seed)?;
                if checked(job, &result).is_ok() {
                    best = Some(result);
                } else {
                    rejected += 1;
                }
            }
        }
    }
    let mut result = best.context("missing incumbent")?;
    checked(job, &result)?;
    result.runtime_s = started.elapsed().as_secs_f64();
    if result.length < reported {
        progress(NestProgress {
            seconds: result.runtime_s,
            length: result.length,
        });
    }
    result.warnings.push(format!("Bounded obstacle packing: {passes} compaction passes; fixed transforms preserved. Prepare {prepare_s:.3}s; initial construction {construct_s:.3}s; search/check {:.3}s.", search_started.elapsed().as_secs_f64()));
    if rejected > 0 {
        result.warnings.push(format!(
            "Rejected {rejected} invalid search candidates; retained the checked incumbent."
        ));
    }
    if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
        result
            .warnings
            .push("Search cancelled; best independently valid complete marker retained.".into());
    }
    Ok(result)
}
