//! Translation-phase graph and bounded bottom-left search, separate from validation.
use crate::checks::{axes, point, residual};
use crate::{assign_bundles, identity};
use anyhow::{Context, Result, bail, ensure};
use beta_marker_core::geometry::bbox;
use beta_marker_core::{Job, MarkerResult, Piece, Placement, Pt};
use geo::{Distance, Euclidean, Intersects, Polygon};
use std::collections::BTreeSet;

const GUARD: f64 = 0.025;
type Phases = Vec<[Option<f64>; 2]>;
type Edge = (usize, usize, f64);

struct Shape {
    polygon: Polygon<f64>,
    bounds: [f64; 4],
    buffer: f64,
}

fn polygon(points: &[Pt]) -> Polygon<f64> {
    Polygon::new(points.iter().map(|p| (p[0], p[1])).collect(), vec![])
}

/// Fusing rectangles are collision envelopes; match points stay in piece space.
pub(crate) fn outline(piece: &Piece, pl: &Placement) -> Vec<Pt> {
    beta_marker_core::rules::cut_outline(piece, pl)
}

fn shape(job: &Job, pl: &Placement) -> Result<Shape> {
    let p = job
        .pieces
        .iter()
        .find(|p| p.id == pl.piece_id)
        .context("unknown fixed piece")?;
    let points = outline(p, pl);
    Ok(Shape {
        polygon: polygon(&points),
        bounds: bbox(&points).context("empty outline")?,
        buffer: p.buffer,
    })
}

fn clear(candidate: &Shape, others: &[Shape], gap: f64) -> bool {
    others.iter().all(|other| {
        let d = gap + candidate.buffer + other.buffer + GUARD;
        let a = candidate.bounds;
        let b = other.bounds;
        if a[0] > b[2] + d || b[0] > a[2] + d || a[1] > b[3] + d || b[1] > a[3] + d {
            return true;
        }
        !candidate.polygon.intersects(&other.polygon)
            && Euclidean.distance(&candidate.polygon, &other.polygon) >= d - 1e-8
    })
}

fn edges(job: &Job, nodes: &[Placement], axis: usize) -> Result<Vec<Edge>> {
    let mut out = Vec::new();
    let repeat = job.fabric.repeat.context("matching needs repeat")?;
    let ground = nodes.len();
    for rule in &job.match_rules {
        if !axes(rule.axes, axis) {
            continue;
        }
        for (ai, a) in nodes
            .iter()
            .enumerate()
            .filter(|(_, p)| p.piece_id == rule.a.piece_id)
        {
            let mut local_a = a.clone();
            local_a.translate = [0.0; 2];
            let ap = point(job, &rule.a, &local_a).context("unknown match point")?[axis];
            if let Some(bref) = &rule.b {
                let partners: Vec<_> = nodes
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| {
                        p.piece_id == bref.piece_id
                            && !beta_marker_core::rules::placement_bundles(p)
                                .is_disjoint(&beta_marker_core::rules::placement_bundles(a))
                    })
                    .collect();
                ensure!(
                    !partners.is_empty(),
                    "no match partner for {} in bundle {:?}",
                    a.piece_id,
                    a.bundle
                );
                for (bi, b) in partners {
                    let mut local_b = b.clone();
                    local_b.translate = [0.0; 2];
                    let bp = point(job, bref, &local_b).context("unknown match point")?[axis];
                    // t_a = t_b + offset - local_a + local_b (mod period).
                    out.push((bi, ai, rule.offset[axis] - ap + bp));
                }
            } else {
                out.push((
                    ground,
                    ai,
                    [repeat.offset_x, repeat.offset_y][axis] + rule.offset[axis] - ap,
                ));
            }
        }
    }
    Ok(out)
}

fn phases(
    job: &Job,
    nodes: &[Placement],
    fixed: &[Placement],
    variant: usize,
) -> Result<Option<Phases>> {
    let repeat = job.fabric.repeat.context("matching needs repeat")?;
    let mut out = vec![[None; 2]; nodes.len()];
    for (axis, period) in [repeat.period_x, repeat.period_y].into_iter().enumerate() {
        let Some(period) = period else { continue };
        let mut graph = vec![vec![]; nodes.len() + 1];
        for (a, b, delta) in edges(job, nodes, axis)? {
            graph[a].push((b, delta));
            graph[b].push((a, -delta));
        }
        if axis == 1 {
            for (i, node) in nodes.iter().enumerate().filter(|(_, n)| n.on_fold) {
                graph[nodes.len()].push((i, node.translate[axis]));
                graph[i].push((nodes.len(), -node.translate[axis]));
            }
        }
        let mut values = vec![None; graph.len()];
        values[nodes.len()] = Some(0.0);
        let mut roots = vec![nodes.len()];
        for (i, node) in nodes.iter().enumerate() {
            if let Some(f) = fixed.iter().find(|p| identity(p) == identity(node)) {
                values[i] = Some(f.translate[axis].rem_euclid(period));
                roots.push(i);
            }
        }
        roots.extend(0..nodes.len());
        let mut visited = vec![false; graph.len()];
        for root in roots {
            if visited[root] || graph[root].is_empty() {
                continue;
            }
            if values[root].is_none() {
                let seed = if variant == 0 {
                    nodes[root].translate[axis]
                } else {
                    let mut local = nodes[root].clone();
                    local.translate = [0.0; 2];
                    let b = shape(job, &local)?.bounds;
                    let p = job
                        .pieces
                        .iter()
                        .find(|p| p.id == local.piece_id)
                        .context("unknown piece")?;
                    job.spacing.edge_gap + p.buffer + GUARD - b[axis]
                        + (variant - 1) as f64 * period / 4.0
                };
                values[root] = Some(seed.rem_euclid(period));
            }
            let mut queue = vec![root];
            while let Some(a) = queue.pop() {
                if visited[a] {
                    continue;
                }
                visited[a] = true;
                let value = values[a].context("phase propagation lost its root")?;
                for &(b, delta) in &graph[a] {
                    let next = (value + delta).rem_euclid(period);
                    if let Some(old) = values[b] {
                        // Search solves exact phases. A narrow numerical tolerance
                        // avoids accumulating the user's tolerance around cycles.
                        if residual(old - next, period) > 1e-6 {
                            return Ok(None);
                        }
                    } else {
                        values[b] = Some(next);
                    }
                    if !visited[b] {
                        queue.push(b);
                    }
                }
            }
        }
        for i in 0..nodes.len() {
            out[i][axis] = values[i];
        }
    }
    Ok(Some(out))
}

fn coordinates(
    low: f64,
    high: f64,
    phase: Option<f64>,
    period: Option<f64>,
    seeds: &[f64],
) -> Vec<f64> {
    let mut out = Vec::new();
    if high < low {
        return out;
    }
    if let (Some(phase), Some(period)) = (phase, period) {
        let first = phase + ((low - phase) / period - 1e-10).ceil() * period;
        // Small repeats use contact events rather than walking millions of lines.
        let count = ((high - first) / period).floor().max(-1.0) + 1.0;
        if count <= 256.0 {
            out.extend((0..count as usize).map(|k| first + k as f64 * period));
        } else {
            out.extend((0..32).map(|k| first + k as f64 * period));
        }
        for &s in seeds {
            let snap = phase + ((s - phase) / period).ceil() * period;
            out.extend([snap - period, snap, snap + period]);
        }
    } else {
        out.extend([low, high]);
        out.extend_from_slice(seeds);
    }
    out.retain(|x| x.is_finite() && *x >= low - 1e-7 && *x <= high + 1e-7);
    out.sort_by(f64::total_cmp);
    out.dedup_by(|a, b| (*a - *b).abs() < 1e-7);
    out
}

fn place(
    job: &Job,
    nodes: &[Placement],
    phases: &Phases,
    fixed: &[Placement],
    baseline_length: f64,
    deadline: Option<crate::Instant>,
) -> Result<Vec<Placement>> {
    let repeat = job.fabric.repeat.context("matching needs repeat")?;
    let mut placed = fixed.to_vec();
    assign_bundles(job, &mut placed);
    let mut occupied = placed
        .iter()
        .map(|p| shape(job, p))
        .collect::<Result<Vec<_>>>()?;
    let defects: Vec<_> = job
        .fabric
        .defects
        .iter()
        .map(|z| polygon(&z.polygon))
        .collect();
    let mut order: Vec<_> = (0..nodes.len()).collect();
    // Large pieces first. The fixed subset is passed to the general nester later.
    order.sort_by(|&a, &b| {
        let pa = job.pieces.iter().find(|p| p.id == nodes[a].piece_id);
        let pb = job.pieces.iter().find(|p| p.id == nodes[b].piece_id);
        let area =
            |p: Option<&Piece>| p.map_or(0.0, |p| beta_marker_core::geometry::area(&p.outline));
        area(pb).total_cmp(&area(pa)).then(a.cmp(&b))
    });
    let extra: f64 = nodes
        .iter()
        .map(|n| shape(job, n).map(|s| s.bounds[2] - s.bounds[0]))
        .collect::<Result<Vec<_>>>()?
        .iter()
        .sum();
    let limit = job.fabric.max_length.unwrap_or(
        baseline_length + extra + repeat.period_x.unwrap_or(0.0) * nodes.len() as f64 + 10.0,
    );
    for i in order {
        ensure!(
            deadline.is_none_or(|d| crate::Instant::now() < d),
            "Matching search reached its time limit before finding a complete marker."
        );
        let node = &nodes[i];
        if placed.iter().any(|p| identity(p) == identity(node)) {
            continue;
        }
        let mut local = node.clone();
        local.translate = [0.0; 2];
        let ls = shape(job, &local)?;
        let b = ls.bounds;
        let edge = job.spacing.edge_gap + ls.buffer + GUARD;
        let low = [edge - b[0], edge - b[1]];
        let high = [limit - b[2], job.fabric.width - edge - b[3]];
        let mut seeds = [
            vec![low[0], node.translate[0]],
            vec![low[1], node.translate[1]],
        ];
        for other in &occupied {
            let gap = job.spacing.piece_gap + ls.buffer + other.buffer + GUARD * 1.01;
            for axis in 0..2 {
                seeds[axis].extend([
                    other.bounds[axis + 2] + gap - b[axis],
                    other.bounds[axis] - gap - b[axis + 2],
                ]);
            }
        }
        for z in &job.fabric.defects {
            if let Some(zb) = bbox(&z.polygon) {
                for axis in 0..2 {
                    seeds[axis].extend([
                        zb[axis + 2] + GUARD - b[axis],
                        zb[axis] - GUARD - b[axis + 2],
                    ]);
                }
            }
        }
        let xs = coordinates(low[0], high[0], phases[i][0], repeat.period_x, &seeds[0]);
        let ys = if node.on_fold {
            vec![node.translate[1]]
        } else {
            coordinates(low[1], high[1], phases[i][1], repeat.period_y, &seeds[1])
        };
        let mut found = None;
        'search: for &x in &xs {
            ensure!(
                deadline.is_none_or(|d| crate::Instant::now() < d),
                "Matching search reached its time limit before finding a complete marker."
            );
            for &y in &ys {
                let mut candidate = local.clone();
                candidate.translate = [x, y];
                let s = shape(job, &candidate)?;
                if clear(&s, &occupied, job.spacing.piece_gap)
                    && defects.iter().all(|z| !s.polygon.intersects(z))
                {
                    found = Some((candidate, s));
                    break 'search;
                }
            }
        }
        let (candidate, s) = found.with_context(|| {
            format!(
                "no lattice placement found for {} in bundle {:?}",
                node.piece_id, node.bundle
            )
        })?;
        placed.push(candidate);
        occupied.push(s);
    }
    Ok(placed)
}

#[cfg(test)]
pub(crate) fn place_matched(
    job: &Job,
    baseline: &MarkerResult,
    fixed: &[Placement],
) -> Result<Vec<Placement>> {
    place_matched_until(job, baseline, fixed, None)
}
pub(crate) fn place_matched_until(
    job: &Job,
    baseline: &MarkerResult,
    fixed: &[Placement],
    deadline: Option<crate::Instant>,
) -> Result<Vec<Placement>> {
    let ids: BTreeSet<_> = job
        .match_rules
        .iter()
        .flat_map(|r| std::iter::once(&r.a.piece_id).chain(r.b.iter().map(|b| &b.piece_id)))
        .collect();
    let mut nodes: Vec<_> = baseline
        .placements
        .iter()
        .filter(|p| ids.contains(&p.piece_id))
        .cloned()
        .collect();
    for node in &mut nodes {
        if let Some(f) = fixed.iter().find(|p| identity(p) == identity(node)) {
            let bundle = node.bundle;
            *node = f.clone();
            node.bundle = node.bundle.or(bundle);
        }
    }
    let rotations: Vec<Vec<f64>> = nodes
        .iter()
        .map(|n| {
            if n.on_fold || fixed.iter().any(|p| identity(p) == identity(n)) {
                return vec![n.rotation_deg];
            }
            let p = job.pieces.iter().find(|p| p.id == n.piece_id);
            let mut angles = vec![n.rotation_deg];
            if let Some(p) = p {
                let allowed = beta_marker_core::allowed_rotations(job, p, n.mirrored);
                angles.extend(if allowed.free {
                    vec![0.0, 90.0, 180.0, 270.0]
                } else {
                    allowed.angles
                });
            }
            let mut seen = BTreeSet::new();
            angles.retain(|angle| seen.insert(angle.to_bits()));
            angles
        })
        .collect();
    let mut digits = vec![0; nodes.len()];
    let mut last = "translation phase cycles disagree for every tried rotation".to_owned();
    for _ in 0..512 {
        ensure!(
            deadline.is_none_or(|d| crate::Instant::now() < d),
            "Matching search reached its time limit before finding a complete marker."
        );
        for (i, n) in nodes.iter_mut().enumerate() {
            n.rotation_deg = rotations[i][digits[i]];
        }
        for variant in 0..5 {
            let Some(phases) = phases(job, &nodes, fixed, variant)? else {
                continue;
            };
            match place(job, &nodes, &phases, fixed, baseline.length, deadline) {
                Ok(placements) => {
                    let mut candidate = baseline.clone();
                    candidate.placements = placements.clone();
                    // Only matched placements are present; the checker deliberately
                    // checks every partner copy, rather than all job demand counts.
                    let errors = crate::match_errors(job, &candidate);
                    if errors.is_empty() {
                        return Ok(placements);
                    }
                    last = format!("phase check: {errors:?}");
                }
                Err(e) => last = e.to_string(),
            }
        }
        // Mixed-radix rotation enumeration, bounded independently of job size.
        let mut advanced = false;
        for i in 0..digits.len() {
            digits[i] += 1;
            if digits[i] < rotations[i].len() {
                advanced = true;
                break;
            }
            digits[i] = 0;
        }
        if !advanced {
            break;
        }
    }
    bail!(
        "repeat search found no feasible phase/rotation layout (512 rotation combinations, five anchor phases): {last}"
    )
}

/// Shift a buffered baseline by at most half a repeat per axis, keeping all
/// unmatched pieces in place. Validation is still required: buffer inflation
/// alone does not protect fold edges, defects or user-fixed neighbours.
pub(crate) fn snap_baseline(job: &Job, baseline: &MarkerResult) -> Result<Option<MarkerResult>> {
    let ids: BTreeSet<_> = job
        .match_rules
        .iter()
        .flat_map(|r| std::iter::once(&r.a.piece_id).chain(r.b.iter().map(|b| &b.piece_id)))
        .collect();
    let nodes: Vec<_> = baseline
        .placements
        .iter()
        .filter(|p| ids.contains(&p.piece_id))
        .cloned()
        .collect();
    let Some(phases) = phases(job, &nodes, &[], 0)? else {
        return Ok(None);
    };
    let repeat = job.fabric.repeat.context("matching needs repeat")?;
    let mut candidate = baseline.clone();
    let mut index = 0;
    for pl in &mut candidate.placements {
        if !ids.contains(&pl.piece_id) {
            continue;
        }
        for (axis, period) in [repeat.period_x, repeat.period_y].into_iter().enumerate() {
            if let (Some(phase), Some(period)) = (phases[index][axis], period) {
                pl.translate[axis] =
                    phase + ((pl.translate[axis] - phase) / period).round() * period;
            }
        }
        index += 1;
    }
    crate::refresh(job, &mut candidate);
    if !crate::match_errors(job, &candidate).is_empty()
        || !beta_marker_core::validate(job, &candidate).ok
    {
        return Ok(None);
    }
    let shapes = candidate
        .placements
        .iter()
        .map(|p| shape(job, p))
        .collect::<Result<Vec<_>>>()?;
    for (i, s) in shapes.iter().enumerate() {
        if !clear(s, &shapes[..i], job.spacing.piece_gap)
            || job
                .fabric
                .defects
                .iter()
                .any(|z| s.polygon.intersects(&polygon(&z.polygon)))
        {
            return Ok(None);
        }
    }
    Ok(Some(candidate))
}
