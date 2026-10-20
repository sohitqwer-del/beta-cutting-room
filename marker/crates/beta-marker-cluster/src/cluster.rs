//! Compact pair envelopes, with explicit transforms for every physical copy.
use crate::{checked, geometry, refine, remaining};
use anyhow::{Context, Result, ensure};
use beta_marker_core::geometry::{area, bbox, norm_deg, transform_point, transform_points};
use beta_marker_core::{Job, MarkerResult, Piece, Placement, Pt, RotationRule};
use beta_marker_nest::{NestOptions, nest};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub(crate) struct Member {
    pub piece: usize,
    pub placement: Placement,
}

#[derive(Clone)]
struct Pair {
    outline: Vec<Pt>,
    angles: [f64; 2],
    offsets: [Pt; 2],
    rotation: RotationRule,
    score: f64,
    contour: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Hull,
    Contour,
    Aggregate,
}

pub(crate) struct Prepared {
    pub job: Job,
    pub mapping: Vec<Vec<Vec<Member>>>,
    pub pairs: usize,
    pub contours: usize,
}

pub(crate) fn slots(job: &Job) -> Vec<Member> {
    let bundles = beta_marker_core::rules::bundle_sizes(job);
    let mut out = Vec::new();
    for (index, piece) in job.pieces.iter().enumerate() {
        let mut copies = [0u32, 0];
        for (bundle, _) in bundles
            .iter()
            .enumerate()
            .filter(|(_, size)| **size == piece.size)
        {
            for (mirrored, quantity) in [(false, piece.quantity), (true, piece.mirror_quantity)] {
                for _ in 0..quantity {
                    let copy = &mut copies[usize::from(mirrored)];
                    out.push(Member {
                        piece: index,
                        placement: Placement {
                            piece_id: piece.id.clone(),
                            copy: *copy,
                            mirrored,
                            bundle: Some(bundle as u32),
                            ..Default::default()
                        },
                    });
                    *copy += 1;
                }
            }
        }
    }
    out.sort_by(|a, b| {
        area(&job.pieces[b.piece].outline).total_cmp(&area(&job.pieces[a.piece].outline))
    });
    out
}

fn angles(job: &Job, member: &Member) -> Vec<f64> {
    let allowed = beta_marker_core::allowed_rotations(
        job,
        &job.pieces[member.piece],
        member.placement.mirrored,
    );
    if allowed.free {
        vec![0.0, 90.0, 180.0, 270.0]
    } else {
        allowed.angles
    }
}

fn common_rotation(job: &Job, members: &[(&Member, f64)]) -> RotationRule {
    let rules: Vec<_> = members
        .iter()
        .map(|(m, _)| {
            beta_marker_core::allowed_rotations(job, &job.pieces[m.piece], m.placement.mirrored)
        })
        .collect();
    if rules.iter().all(|r| r.free) {
        return RotationRule::Free;
    }
    let allows = |delta| {
        rules
            .iter()
            .zip(members)
            .all(|(r, (_, base))| r.allows(base + delta))
    };
    if [90.0, 180.0, 270.0].iter().all(|&r| allows(r)) {
        RotationRule::Quarter
    } else if allows(180.0) {
        RotationRule::Flip180
    } else {
        RotationRule::Fixed
    }
}

fn search_pair(job: &Job, a: &Member, b: &Member, deadline: Instant) -> Option<Pair> {
    let (pa, pb) = (&job.pieces[a.piece], &job.pieces[b.piece]);
    let gap = job.spacing.piece_gap + pa.buffer + pb.buffer + geometry::GUARD;
    let mut best: Option<Pair> = None;
    // Fix the first member's base; relative rotations of the second produce
    // head-to-tail and face-to-face pairs without changing the grain contract.
    let ar = *angles(job, a).first()?;
    let aa = transform_points(&pa.outline, a.placement.mirrored, ar, [0.0; 2]);
    for br in angles(job, b) {
        if Instant::now() >= deadline {
            break;
        }
        let bb = transform_points(&pb.outline, b.placement.mirrored, br, [0.0; 2]);
        let ab = bbox(&aa)?;
        let bbound = bbox(&bb)?;
        let individual_box =
            (ab[2] - ab[0]) * (ab[3] - ab[1]) + (bbound[2] - bbound[0]) * (bbound[3] - bbound[1]);
        let useful = area(&aa) + area(&bb);
        geometry::contacts(&aa, &bb, gap, |offset| {
            if Instant::now() >= deadline {
                return false;
            }
            let moved = transform_points(&bb, false, 0.0, offset);
            let mut points = aa.clone();
            points.extend(moved);
            let bounds = match bbox(&points) {
                Some(b) => b,
                None => return true,
            };
            let width = bounds[3] - bounds[1];
            let height_room =
                job.fabric.width - 2.0 * (job.spacing.edge_gap + pa.buffer.max(pb.buffer));
            if width > height_room {
                return true;
            }
            let box_area = (bounds[2] - bounds[0]) * width;
            // Avoid constraining flexible convex pairs that offer no compactness
            // gain. The hull encloses every member, including its concavities.
            if box_area >= individual_box * 0.94 {
                return true;
            }
            let outline = geometry::hull(&points);
            let hull_area = area(&outline);
            let score = useful / hull_area * 0.65 + useful / box_area * 0.35;
            if useful / hull_area < 0.82 || best.as_ref().is_some_and(|p| p.score >= score) {
                return true;
            }
            let origin = [-bounds[0], -bounds[1]];
            best = Some(Pair {
                outline: transform_points(&outline, false, 0.0, origin),
                angles: [ar, br],
                offsets: [origin, [offset[0] + origin[0], offset[1] + origin[1]]],
                rotation: common_rotation(job, &[(a, ar), (b, br)]),
                score,
                contour: false,
            });
            true
        });
    }
    best
}

pub(crate) fn prepare(job: &Job, seconds: f64) -> Result<Prepared> {
    prepare_mode(job, seconds, Mode::Hull)
}

pub(crate) fn prepare_mode(job: &Job, seconds: f64, mode: Mode) -> Result<Prepared> {
    let deadline = Instant::now() + Duration::from_secs_f64(seconds.max(0.001));
    let all = slots(job);
    ensure!(!all.is_empty(), "no pieces to place");
    let mut used = vec![false; all.len()];
    let mut transformed = job.clone();
    transformed.order = None;
    transformed.pieces.clear();
    transformed.fabric.tilt_deg = 0.0;
    let mut mapping: Vec<Vec<Vec<Member>>> = Vec::new();
    // Physical copies can have different piece IDs. Share search results only
    // when geometry, handedness, clearance and allowed angles match exactly.
    // Original identities stay in the expansion map.
    let mut identities = BTreeMap::new();
    let mut classes = BTreeMap::new();
    for member in &all {
        let key = (member.piece, member.placement.mirrored);
        if classes.contains_key(&key) {
            continue;
        }
        let piece = &job.pieces[member.piece];
        let allowed = beta_marker_core::allowed_rotations(job, piece, member.placement.mirrored);
        let mut identity = vec![
            u64::from(member.placement.mirrored),
            piece.buffer.to_bits(),
            u64::from(allowed.free),
            allowed.tilt_deg.to_bits(),
            allowed.angles.len() as u64,
        ];
        identity.extend(allowed.angles.iter().map(|v| v.to_bits()));
        identity.extend(piece.outline.iter().flatten().map(|v| v.to_bits()));
        let next = identities.len();
        let class = *identities.entry(identity).or_insert(next);
        classes.insert(key, class);
    }
    let class = |m: &Member| classes[&(m.piece, m.placement.mirrored)];
    let mut cache: BTreeMap<(usize, usize), Option<Pair>> = BTreeMap::new();
    let mut types = BTreeMap::new();
    let mut pairs = 0;
    let mut contours = 0;
    for (i, a) in all.iter().enumerate() {
        if used[i] {
            continue;
        }
        used[i] = true;
        let mut partner = None;
        let mut seen = std::collections::BTreeSet::new();
        for (j, b) in all.iter().enumerate().skip(i + 1) {
            if mode == Mode::Aggregate {
                break;
            }
            if used[j] || !seen.insert(class(b)) {
                continue;
            }
            if seen.len() > 6 {
                break;
            }
            let key = (class(a), class(b));
            if Instant::now() < deadline {
                cache.entry(key).or_insert_with(|| {
                    let mut pair = search_pair(job, a, b, deadline)?;
                    if mode == Mode::Contour {
                        let aa = transform_points(
                            &job.pieces[a.piece].outline,
                            a.placement.mirrored,
                            pair.angles[0],
                            pair.offsets[0],
                        );
                        let bb = transform_points(
                            &job.pieces[b.piece].outline,
                            b.placement.mirrored,
                            pair.angles[1],
                            pair.offsets[1],
                        );
                        if let Some(outline) = geometry::connected_envelope(&aa, &bb) {
                            let room = job.fabric.width
                                - 2.0
                                    * (job.spacing.edge_gap
                                        + job.pieces[a.piece]
                                            .buffer
                                            .max(job.pieces[b.piece].buffer));
                            if bbox(&outline).is_some_and(|b| b[3] - b[1] <= room) {
                                pair.outline = outline;
                                pair.contour = true;
                            }
                        }
                    }
                    Some(pair)
                });
            }
            if let Some(Some(pair)) = cache.get(&key)
                && partner
                    .as_ref()
                    .is_none_or(|(_, p): &(usize, Pair)| pair.score > p.score)
            {
                partner = Some((j, pair.clone()));
            }
        }
        let mut members = vec![a.clone()];
        let (key, outline, rotation, buffer, tilt) = if let Some((j, pair)) = partner {
            used[j] = true;
            pairs += 1;
            contours += usize::from(pair.contour);
            members.push(all[j].clone());
            for (k, member) in members.iter_mut().enumerate() {
                member.placement.rotation_deg = pair.angles[k];
                member.placement.translate = pair.offsets[k];
            }
            (
                (class(a), Some(class(&all[j]))),
                pair.outline,
                pair.rotation,
                job.pieces[a.piece]
                    .buffer
                    .max(job.pieces[all[j].piece].buffer),
                0.0,
            )
        } else {
            let base = *angles(job, a).first().context("no allowed rotation")?;
            let member = &mut members[0];
            member.placement.rotation_deg = base;
            let points = transform_points(
                &job.pieces[a.piece].outline,
                a.placement.mirrored,
                base,
                [0.0; 2],
            );
            let bounds = bbox(&points).context("empty outline")?;
            member.placement.translate = [-bounds[0], -bounds[1]];
            let rule = beta_marker_core::allowed_rotations(
                job,
                &job.pieces[a.piece],
                a.placement.mirrored,
            );
            (
                (class(a), None),
                transform_points(&points, false, 0.0, member.placement.translate),
                job.pieces[a.piece].rotation,
                job.pieces[a.piece].buffer,
                rule.tilt_deg,
            )
        };
        if let Some(&index) = types.get(&key) {
            let index: usize = index;
            transformed.pieces[index].quantity += 1;
            mapping[index].push(members);
        } else {
            let index = transformed.pieces.len();
            transformed.pieces.push(Piece {
                id: format!("__cluster_{index}"),
                name: format!("Cluster item {index}"),
                outline,
                quantity: 1,
                rotation,
                buffer,
                tilt_deg: Some(tilt),
                ..Default::default()
            });
            types.insert(key, index);
            mapping.push(vec![members]);
        }
    }
    Ok(Prepared {
        job: transformed,
        mapping,
        pairs,
        contours,
    })
}

pub(crate) fn expand(
    job: &Job,
    prepared: &Prepared,
    mut result: MarkerResult,
) -> Result<MarkerResult> {
    let mut placements = Vec::new();
    for composite in &result.placements {
        ensure!(!composite.mirrored, "composite reflections are not allowed");
        let index = prepared
            .job
            .pieces
            .iter()
            .position(|p| p.id == composite.piece_id)
            .context("unknown composite")?;
        let members = prepared
            .mapping
            .get(index)
            .and_then(|copies| copies.get(composite.copy as usize))
            .context("unknown composite copy")?;
        for member in members {
            let mut placement = member.placement.clone();
            placement.rotation_deg = norm_deg(placement.rotation_deg + composite.rotation_deg);
            placement.translate = transform_point(
                placement.translate,
                false,
                composite.rotation_deg,
                composite.translate,
            );
            ensure!(
                beta_marker_core::allowed_rotations(
                    job,
                    &job.pieces[member.piece],
                    placement.mirrored
                )
                .allows(placement.rotation_deg),
                "composite rotation violates member grain rule"
            );
            placements.push(placement);
        }
    }
    result.job_name = job.name.clone();
    result.placements = placements;
    result.engine = format!("pair clusters / {}", result.engine);
    result.warnings.push(format!(
        "Expanded {} compact pairs from conservative envelopes.",
        prepared.pairs
    ));
    checked(job, result)
}

pub(crate) fn run(job: &Job, opts: &NestOptions) -> Result<MarkerResult> {
    run_mode(job, opts, Mode::Hull)
}

pub(crate) fn run_mode(job: &Job, opts: &NestOptions, mode: Mode) -> Result<MarkerResult> {
    let started = Instant::now();
    let prepared = if mode == Mode::Hull {
        prepare(job, (opts.time_limit_s * 0.08).min(3.0))?
    } else {
        prepare_mode(job, (opts.time_limit_s * 0.08).min(3.0), mode)?
    };
    if prepared.pairs == 0 && mode == Mode::Hull {
        let mut options = opts.clone();
        options.time_limit_s = remaining(opts, started).max(0.001);
        let mut result = nest(job, &options)?;
        result
            .warnings
            .push("No compact legal pair found; used plain nesting.".into());
        return Ok(result);
    }
    let mut options = opts.clone();
    options.time_limit_s =
        (remaining(opts, started) * if mode == Mode::Aggregate { 1.0 } else { 0.70 }).max(0.001);
    options.warm_start = None;
    let candidate = expand(job, &prepared, nest(&prepared.job, &options)?)?;
    let best = if let Some(warm) = &opts.warm_start
        && warm.length < candidate.length
    {
        warm.clone()
    } else {
        candidate
    };
    let mut result = refine(job, opts, best, remaining(opts, started), false)?;
    result.engine = format!(
        "{} / {}",
        match mode {
            Mode::Hull => "pair clusters",
            Mode::Contour => "contact contours",
            Mode::Aggregate => "exact aggregation",
        },
        result.engine
    );
    result.warnings.push(format!(
        "Cluster construction used {} pairs; {} synthetic types for {} physical members.",
        prepared.pairs,
        prepared.job.pieces.len(),
        result.placements.len()
    ));
    if mode == Mode::Contour {
        result.warnings.push(format!(
            "Conservative contact contours used for {} of {} pairs; other pairs used convex hulls.",
            prepared.contours, prepared.pairs
        ));
    } else if mode == Mode::Aggregate {
        result.warnings.push(format!("Exact copy aggregation: {} source piece types to {} synthetic types; no paired members.", job.pieces.len(), prepared.job.pieces.len()));
    }
    Ok(result)
}
