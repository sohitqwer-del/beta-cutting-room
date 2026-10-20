//! Small-piece reinsertion into sampled empty regions, with exact predicates.
use crate::{checked, geometry, refine, remaining};
use anyhow::{Context, Result};
use beta_marker_core::geometry::{area, bbox, transform_points};
use beta_marker_core::{Job, MarkerResult, Placement, Pt, RESULT_FORMAT};
use beta_marker_nest::{NestOptions, nest};
use std::collections::{BTreeMap, BTreeSet};
use std::time::{Duration, Instant};

pub(crate) fn small_ids(job: &Job) -> BTreeSet<String> {
    let max = job
        .pieces
        .iter()
        .filter(|p| {
            (p.quantity > 0 || p.mirror_quantity > 0)
                && beta_marker_core::rules::multiplier(job, p) > 0
        })
        .map(|p| area(&p.outline))
        .fold(0.0, f64::max);
    job.pieces
        .iter()
        .filter(|p| area(&p.outline) <= max * 0.20)
        .map(|p| p.id.clone())
        .collect()
}

struct Obstacle {
    points: Vec<Pt>,
    bounds: [f64; 4],
    buffer: f64,
}

fn coordinates(mut values: Vec<f64>, low: f64, high: f64, limit: usize) -> Vec<f64> {
    values.retain(|v| v.is_finite() && *v >= low && *v <= high);
    values.sort_by(f64::total_cmp);
    values.dedup_by(|a, b| (*a - *b).abs() < 0.005);
    if values.len() <= limit {
        return values;
    }
    (0..limit)
        .map(|i| values[i * (values.len() - 1) / (limit - 1)])
        .collect()
}

fn assign_bundles(job: &Job, placements: &mut [Placement]) {
    let sizes = beta_marker_core::rules::bundle_sizes(job);
    for placement in placements {
        if let Some(piece) = job.pieces.iter().find(|p| p.id == placement.piece_id) {
            let quantity = if placement.mirrored {
                piece.mirror_quantity
            } else {
                piece.quantity
            };
            placement.bundle = sizes
                .iter()
                .enumerate()
                .filter(|(_, s)| **s == piece.size)
                .nth((placement.copy / quantity.max(1)) as usize)
                .map(|(i, _)| i as u32);
        }
    }
}

pub(crate) fn insert(
    job: &Job,
    mut placed: Vec<Placement>,
    small: Vec<Placement>,
    seconds: f64,
) -> Result<MarkerResult> {
    let deadline = Instant::now() + Duration::from_secs_f64(seconds.max(0.001));
    let by_id: BTreeMap<_, _> = job.pieces.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut obstacles = Vec::new();
    let mut length = 0.0f64;
    for pl in &placed {
        let p = by_id
            .get(pl.piece_id.as_str())
            .context("unknown large piece")?;
        let points = transform_points(&p.outline, pl.mirrored, pl.rotation_deg, pl.translate);
        let bounds = bbox(&points).context("empty large outline")?;
        length = length.max(bounds[2]);
        obstacles.push(Obstacle {
            points,
            bounds,
            buffer: p.buffer,
        });
    }
    let mut small = small;
    small.sort_by(|a, b| {
        area(&by_id[b.piece_id.as_str()].outline)
            .total_cmp(&area(&by_id[a.piece_id.as_str()].outline))
    });
    for mut pl in small {
        let piece = by_id
            .get(pl.piece_id.as_str())
            .context("unknown small piece")?;
        let edge = job.spacing.edge_gap + piece.buffer + geometry::GUARD;
        let allowed = beta_marker_core::allowed_rotations(job, piece, pl.mirrored);
        let rotations: Vec<_> = if allowed.free {
            (0..24).map(|i| f64::from(i) * 15.0).collect()
        } else {
            allowed.angles.clone()
        };
        let mut best: Option<(f64, f64, f64, Pt, Vec<Pt>)> = None;
        'rotations: for rotation in rotations {
            let local = transform_points(&piece.outline, pl.mirrored, rotation, [0.0; 2]);
            let bounds = bbox(&local).context("empty small outline")?;
            let low_x = edge - bounds[0];
            let low_y = edge - bounds[1];
            let high_y = job.fabric.width - edge - bounds[3];
            if high_y < low_y {
                continue;
            }
            let end_x = length
                + job.spacing.piece_gap
                + piece.buffer
                + obstacles.iter().map(|o| o.buffer).fold(0.0, f64::max)
                + geometry::GUARD
                - bounds[0];
            let mut ys = vec![low_y, high_y];
            for step in 0..=24 {
                ys.push(low_y + (high_y - low_y) * f64::from(step) / 24.0);
            }
            for obstacle in &obstacles {
                let gap = job.spacing.piece_gap + piece.buffer + obstacle.buffer + geometry::GUARD;
                ys.extend([
                    obstacle.bounds[1] - gap - bounds[3],
                    obstacle.bounds[3] + gap - bounds[1],
                ]);
                // Vertex differences sample contacts along concave boundaries.
                for a in obstacle
                    .points
                    .iter()
                    .step_by((obstacle.points.len() / 8).max(1))
                {
                    for b in local.iter().step_by((local.len() / 8).max(1)) {
                        ys.extend([a[1] - b[1] - gap, a[1] - b[1] + gap]);
                    }
                }
            }
            let ys = coordinates(ys, low_y, high_y, 64);
            for &y in &ys {
                let mut x = low_x;
                // Move right only when blocked. A full-polygon bisection finds
                // a safe contact with that blocker, then all obstacles are
                // checked again. This avoids a quadratic grid of translations.
                for _ in 0..=obstacles.len() * 3 {
                    if Instant::now() >= deadline {
                        break 'rotations;
                    }
                    if best.as_ref().is_some_and(|b| x + bounds[2] > b.0 + 0.005) {
                        break;
                    }
                    let points = transform_points(&local, false, 0.0, [x, y]);
                    let blocked = obstacles.iter().find(|o| {
                        !geometry::separated(
                            &points,
                            &o.points,
                            job.spacing.piece_gap + piece.buffer + o.buffer + geometry::GUARD,
                        )
                    });
                    if let Some(obstacle) = blocked {
                        let gap = job.spacing.piece_gap
                            + piece.buffer
                            + obstacle.buffer
                            + geometry::GUARD;
                        let mut safe = obstacle.bounds[2] + gap - bounds[0] + geometry::GUARD;
                        let mut blocked = x;
                        for _ in 0..16 {
                            let mid = (safe + blocked) / 2.0;
                            let trial = transform_points(&local, false, 0.0, [mid, y]);
                            if geometry::separated(&trial, &obstacle.points, gap) {
                                safe = mid;
                            } else {
                                blocked = mid;
                            }
                        }
                        x = safe;
                    } else {
                        let right = x + bounds[2];
                        if best.as_ref().is_none_or(|b| {
                            right < b.0 - 0.005 || ((right - b.0).abs() <= 0.005 && y < b.1)
                        }) {
                            best = Some((right, y, rotation, [x, y], points));
                        }
                        break;
                    }
                }
            }
            // Guaranteed disjoint shelf fallback, including after timeout.
            if best.is_none() {
                let points = transform_points(&local, false, 0.0, [end_x, low_y]);
                best = Some((end_x + bounds[2], low_y, rotation, [end_x, low_y], points));
            }
        }
        // A time limit may expire before the first rotation of this piece.
        if best.is_none() {
            let rotation = if allowed.free {
                0.0
            } else {
                *allowed
                    .angles
                    .first()
                    .context("no allowed small-piece rotation")?
            };
            let local = transform_points(&piece.outline, pl.mirrored, rotation, [0.0; 2]);
            let b = bbox(&local).context("empty small outline")?;
            let offset = [
                length
                    + job.spacing.piece_gap
                    + piece.buffer
                    + obstacles.iter().map(|o| o.buffer).fold(0.0, f64::max)
                    + geometry::GUARD
                    - b[0],
                edge - b[1],
            ];
            let points = transform_points(&local, false, 0.0, offset);
            best = Some((offset[0] + b[2], offset[1], rotation, offset, points));
        }
        let (right, _, rotation, offset, points) =
            best.context("small piece cannot fit fabric width")?;
        let bounds = bbox(&points).context("empty inserted outline")?;
        length = length.max(right);
        pl.rotation_deg = rotation;
        pl.translate = offset;
        placed.push(pl);
        obstacles.push(Obstacle {
            points,
            bounds,
            buffer: piece.buffer,
        });
    }
    assign_bundles(job, &mut placed);
    let metrics = beta_marker_core::compute_metrics(job, &placed);
    checked(
        job,
        MarkerResult {
            format: RESULT_FORMAT.into(),
            job_name: job.name.clone(),
            width: job.fabric.width,
            length: metrics.length,
            piece_area: metrics.piece_area,
            efficiency: metrics.efficiency,
            engine: "sampled empty-space fill / sparrow 0.3.0".into(),
            seed: None,
            runtime_s: 0.0,
            placements: placed,
            warnings: vec![],
        },
    )
}

pub(crate) fn run(
    job: &Job,
    opts: &NestOptions,
    two_stage: bool,
    fixed: bool,
) -> Result<MarkerResult> {
    let started = Instant::now();
    let small_ids = small_ids(job);
    if small_ids.is_empty() {
        return nest(job, opts);
    }
    let mut options = opts.clone();
    options.time_limit_s = opts.time_limit_s * 0.55;
    let mut initial_job = job.clone();
    if two_stage {
        for piece in &mut initial_job.pieces {
            if small_ids.contains(&piece.id) {
                piece.quantity = 0;
                piece.mirror_quantity = 0;
            }
        }
        options.warm_start = None;
    }
    let initial = nest(&initial_job, &options)?;
    let mut large: Vec<_> = initial
        .placements
        .iter()
        .filter(|p| !small_ids.contains(&p.piece_id))
        .cloned()
        .collect();
    assign_bundles(job, &mut large);
    let insertion_seconds = (opts.time_limit_s * 0.12)
        .min(remaining(opts, started))
        .max(0.001);
    let filled = if two_stage {
        // The public obstacle packer completes all small demand around the
        // large pieces in its first construction. This gives free compression
        // a dense complete warm start even under a short insertion allowance.
        let mut middle = opts.clone();
        middle.time_limit_s = insertion_seconds;
        middle.fixed = large;
        middle.warm_start = None;
        nest(job, &middle)?
    } else {
        let small = initial
            .placements
            .iter()
            .filter(|p| small_ids.contains(&p.piece_id))
            .cloned()
            .collect();
        insert(job, large, small, insertion_seconds)?
    };
    let mut best = if !two_stage && initial.length < filled.length {
        initial
    } else {
        filled
    };
    if let Some(warm) = &opts.warm_start
        && warm.length < best.length
    {
        best = warm.clone();
    }
    let mut result = refine(job, opts, best, remaining(opts, started), fixed)?;
    result.seed = opts.seed;
    result.warnings.push(format!(
        "{}; final large pieces {}.",
        if two_stage {
            "Large pieces first, then small-piece empty-space insertion"
        } else {
            "Small pieces removed and reinserted"
        },
        if fixed { "fixed" } else { "free" }
    ));
    Ok(result)
}
