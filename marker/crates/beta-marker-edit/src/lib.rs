//! Pure Rust editing commands. All distances use millimetres and all angles use degrees.
//! Contact uses swept polygon boundaries and circular gap capsules, including concave parts.
mod polygon;
use beta_marker_core::{
    Job, Layout, MarkerResult, Placement, Pt, ValidationReport,
    geometry::{angle_diff_deg, bbox, norm_deg, transform_point},
    rules::{
        allowed_rotations, bundle_sizes, cut_outline, demands, effective_job, flip_allowed,
        layer_copies,
    },
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const EPS: f64 = 1e-7;
fn check_counts(job: &Job) -> Result<(), String> {
    if !job.fabric.width.is_finite()
        || job.fabric.width <= 0.
        || !job.spacing.piece_gap.is_finite()
        || !job.spacing.edge_gap.is_finite()
    {
        return Err("Fabric width and gaps must be finite, with positive width.".into());
    }
    let mut total = 0_u64;
    for p in &job.pieces {
        if !polygon::simple(&p.outline) || !p.buffer.is_finite() {
            return Err(format!(
                "{} needs a simple, finite polygon and finite buffer.",
                p.name
            ));
        }
        let m = beta_marker_core::rules::multiplier(job, p);
        let a = p.quantity.checked_mul(m).ok_or("Copy count exceeds u32.")?;
        let b = p
            .mirror_quantity
            .checked_mul(m)
            .ok_or("Copy count exceeds u32.")?;
        total += u64::from(a) + u64::from(b);
    }
    if total > 10000 {
        return Err("Manual editing supports at most 10,000 required copies.".into());
    }
    Ok(())
}
/// Cardinal direction in marker coordinates (positive y is up).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    /// Toward the marker start.
    Left,
    /// Toward the marker end.
    Right,
    /// Toward the upper fabric edge.
    Up,
    /// Toward the lower fabric edge.
    Down,
}
impl Direction {
    fn vector(self) -> Pt {
        match self {
            Self::Left => [-1., 0.],
            Self::Right => [1., 0.],
            Self::Up => [0., 1.],
            Self::Down => [0., -1.],
        }
    }
}
/// Editor metadata saved with the job, separate from the shared geometry contract.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Settings {
    /// Cut lines across the full fabric width. Pieces may not cross these lines.
    #[serde(default)]
    pub sections: Vec<f64>,
    /// Physical table end. The shared result length remains the occupied cut length.
    #[serde(default)]
    pub table_length: Option<f64>,
    /// Removed copies retain their fold and garment identity for later placement.
    #[serde(default)]
    pub removed: Vec<Placement>,
}
/// A single undoable editing command.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Request {
    /// Zero-based placement indices. Locked placements cannot move.
    #[serde(default)]
    pub selected: Vec<usize>,
    /// Operation and its parameters.
    #[serde(flatten)]
    pub action: Action,
}
/// Supported operator tools.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Action {
    /// Translate the selection to first contact in the given direction.
    Bump { direction: Direction },
    /// Repeat left bumps in x order until stable (at most one pass per copy).
    Compact,
    /// Bump the copies touching the occupied marker end to the left.
    CloseEnd,
    /// Leave an initial collision and stop at the first clear position.
    Resolve { direction: Direction },
    /// Translate every selected copy by the same `[dx,dy]`.
    Move { delta: Pt },
    /// Rotate the group about its shared bounding-box center, subject to grain rules.
    Rotate { degrees: f64 },
    /// Rotate the group to the next allowed angle of its first selected copy.
    NextRotation,
    /// Apply a relative angle within each selected piece's tilt allowance.
    Tilt { degrees: f64 },
    /// Mirror each selected copy about its center when faceless-fabric rules allow it.
    Flip,
    /// Exchange the centers of exactly two selected copies.
    Swap,
    /// Remove selected copies while retaining their original garment/fold identity.
    Remove,
    /// Place an indexed missing copy with its lower-left cut bound at `at`, then bump left.
    Place { unplaced: usize, at: Pt },
    /// Try at most 2,000 edge-derived free-space candidates with exact clearance checks.
    FillGaps,
    /// Select by size, one-based bundle number, name substring, or all copies.
    Select { field: String, value: String },
    /// Select copies fully enclosed by the two corners of a rectangle.
    Rectangle { from: Pt, to: Pt },
    /// Add or remove a full-width cut line at `x`; reject crossing pieces.
    Section { x: f64, remove: bool },
    /// Save a physical table end without changing occupied marker metrics.
    Length { length: f64 },
    /// Measure Euclidean distance between two points.
    Measure { from: Pt, to: Pt },
}
/// Result of a command. The UI can commit this as one immutable history snapshot.
#[derive(Clone, Debug, Serialize)]
pub struct Output {
    /// Updated input job, including editor metadata.
    pub job: Job,
    /// Updated placements with recomputed occupied metrics.
    pub result: MarkerResult,
    /// Selection to show after the command.
    pub selected: Vec<usize>,
    /// Measured distance for the measure command only.
    pub measurement: Option<f64>,
    /// Optional plain-English feedback for the operator.
    pub notice: String,
}
/// Exact pair clearance and overlap area, without a user tolerance hiding validator errors.
#[derive(Clone, Debug, Serialize)]
pub struct PairCheck {
    /// First placement index.
    pub a: usize,
    /// Second placement index.
    pub b: usize,
    /// Positive polygon intersection area; zero for a gap-only violation.
    pub overlap_mm2: f64,
    /// Minimum Euclidean boundary clearance (zero for intersecting polygons).
    pub distance_mm: f64,
    /// Required piece gap plus the two piece buffers.
    pub required_mm: f64,
}
/// Live geometry checks plus the independent shared garment-rule validator.
#[derive(Clone, Debug, Serialize)]
pub struct Inspection {
    /// Independent counts, grain, fold, matching, shade, defect and geometry audit.
    pub validation: ValidationReport,
    /// All intersecting or under-clearance pairs.
    pub pairs: Vec<PairCheck>,
    /// Placement indices whose cut bounds straddle a saved section line.
    pub section_violations: Vec<usize>,
    /// Placed copies beyond a saved table end or the job maximum.
    pub end_violations: Vec<usize>,
    /// Strict geometry checks include editor metadata and physical clearance.
    pub geometry_ok: bool,
    /// Remaining required placement copies, including remembered removals.
    pub unplaced: Vec<Placement>,
    /// Explicit table end, job maximum, or occupied length (minimum default 100 mm).
    pub table_length: f64,
    /// Piece area divided by width times physical table length.
    pub table_efficiency: f64,
}
/// Decode the versioned editor metadata. Unknown metadata remains untouched.
pub fn settings(job: &Job) -> Result<Settings, String> {
    let s: Settings = job
        .meta
        .get("editor_v1")
        .cloned()
        .map(serde_json::from_value)
        .transpose()
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    if s.sections.iter().any(|x| !x.is_finite() || *x <= 0.)
        || s.table_length.is_some_and(|x| !x.is_finite() || x <= 0.)
    {
        return Err("Section and table lengths must be positive and finite.".into());
    }
    Ok(s)
}
fn save_settings(job: &mut Job, s: &Settings) -> Result<(), String> {
    job.meta.insert(
        "editor_v1".into(),
        serde_json::to_value(s).map_err(|e| e.to_string())?,
    );
    Ok(())
}
fn forget_removed(s: &mut Settings, p: &Placement) {
    if let Some(i) = s.removed.iter().position(|q| {
        q.piece_id == p.piece_id
            && q.mirrored == p.mirrored
            && q.bundle == p.bundle
            && q.on_fold == p.on_fold
    }) {
        s.removed.remove(i);
    }
}
#[derive(Clone)]
struct Shape {
    points: Vec<Pt>,
    bounds: [f64; 4],
    buffer: f64,
}
fn shapes(job: &Job, result: &MarkerResult) -> Result<Vec<Shape>, String> {
    result
        .placements
        .iter()
        .map(|p| {
            let piece = job
                .pieces
                .iter()
                .find(|q| q.id == p.piece_id)
                .ok_or("Unknown placed piece.")?;
            let points = cut_outline(piece, p);
            if points.len() < 3 || points.iter().flatten().any(|x| !x.is_finite()) {
                return Err("A placement has no finite cut polygon.".into());
            }
            let bounds = bbox(&points).ok_or("Empty cut polygon.")?;
            Ok(Shape {
                points,
                bounds,
                buffer: piece.buffer,
            })
        })
        .collect()
}
fn boxes_near(a: [f64; 4], b: [f64; 4], gap: f64) -> bool {
    a[0] <= b[2] + gap && b[0] <= a[2] + gap && a[1] <= b[3] + gap && b[1] <= a[3] + gap
}
fn limit(job: &Job, result: &MarkerResult, s: &Settings) -> f64 {
    s.table_length
        .or(job.fabric.max_length)
        .unwrap_or_else(|| result.length.max(100.))
}
fn section_bad(shape: &Shape, s: &Settings) -> bool {
    s.sections
        .iter()
        .any(|x| shape.bounds[0] < x - EPS && shape.bounds[2] > x + EPS)
}
fn geometry_ok(job: &Job, result: &MarkerResult, s: &Settings) -> Result<bool, String> {
    let sh = shapes(job, result)?;
    for (i, a) in sh.iter().enumerate() {
        let edge = job.spacing.edge_gap + a.buffer;
        let low = if job.fabric.layout == Layout::Tubular {
            0.
        } else {
            edge
        };
        let high = if job.fabric.layout == Layout::Open {
            edge
        } else {
            0.
        };
        if a.bounds[0] < edge - EPS
            || a.bounds[1] < low - EPS
            || a.bounds[3] > job.fabric.width - high + EPS
            || s.table_length
                .or(job.fabric.max_length)
                .is_some_and(|l| a.bounds[2] > l + EPS)
            || section_bad(a, s)
        {
            return Ok(false);
        }
        if job
            .fabric
            .defects
            .iter()
            .any(|z| polygon::conflict(&a.points, &z.polygon, 0.))
        {
            return Ok(false);
        }
        for b in &sh[..i] {
            let gap = job.spacing.piece_gap + a.buffer + b.buffer;
            if boxes_near(a.bounds, b.bounds, gap) && polygon::conflict(&a.points, &b.points, gap) {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
/// Bump a group to the first polygon contact. Returns travel in mm.
/// For recovery, `resolve` permits leaving an initial collision, then stops at first clear position.
pub fn bump(
    job: &Job,
    result: &mut MarkerResult,
    indices: &[usize],
    direction: Direction,
    s: &Settings,
    resolve: bool,
) -> Result<f64, String> {
    let selected: BTreeSet<_> = indices.iter().copied().collect();
    if selected.is_empty() {
        return Ok(0.);
    }
    if selected
        .iter()
        .any(|&i| result.placements.get(i).is_none_or(|p| p.locked))
    {
        return Err("Select unlocked pieces to move.".into());
    }
    let sh = shapes(job, result)?;
    let v = direction.vector();
    for &i in &selected {
        for &j in selected.range(..i) {
            let gap = job.spacing.piece_gap + sh[i].buffer + sh[j].buffer;
            if boxes_near(sh[i].bounds, sh[j].bounds, gap)
                && polygon::conflict(&sh[i].points, &sh[j].points, gap)
            {
                return Err(
                    "Selected pieces collide with each other. Select one copy to resolve it."
                        .into(),
                );
            }
        }
    }
    let mut max = f64::INFINITY;
    for &i in &selected {
        let a = &sh[i];
        let edge = job.spacing.edge_gap + a.buffer;
        let low = if job.fabric.layout == Layout::Tubular {
            0.
        } else {
            edge
        };
        let high = if job.fabric.layout == Layout::Open {
            edge
        } else {
            0.
        };
        max = max.min(match direction {
            Direction::Left => a.bounds[0] - edge,
            Direction::Right => limit(job, result, s) - a.bounds[2],
            Direction::Down => a.bounds[1] - low,
            Direction::Up => job.fabric.width - high - a.bounds[3],
        });
        for &x in &s.sections {
            if a.bounds[0] < x - EPS && a.bounds[2] > x + EPS {
                return Err("Move this piece clear of the section line first.".into());
            }
            if v[0] < 0. && x <= a.bounds[0] + EPS {
                max = max.min(a.bounds[0] - x);
            }
            if v[0] > 0. && x >= a.bounds[2] - EPS {
                max = max.min(x - a.bounds[2]);
            }
        }
    }
    max = max.max(0.);
    if !max.is_finite() || max <= EPS {
        return Ok(0.);
    }
    let mut obstacles: Vec<(usize, Vec<Pt>, f64)> = vec![];
    let mut times = vec![0., max];
    for &i in &selected {
        let a = &sh[i];
        let end = [
            a.bounds[0] + v[0] * max,
            a.bounds[1] + v[1] * max,
            a.bounds[2] + v[0] * max,
            a.bounds[3] + v[1] * max,
        ];
        let swept = [
            a.bounds[0].min(end[0]),
            a.bounds[1].min(end[1]),
            a.bounds[2].max(end[2]),
            a.bounds[3].max(end[3]),
        ];
        for (j, b) in sh.iter().enumerate() {
            if selected.contains(&j) {
                continue;
            }
            let gap = job.spacing.piece_gap + a.buffer + b.buffer;
            if boxes_near(swept, b.bounds, gap) {
                polygon::events(&a.points, &b.points, v, gap, &mut times);
                obstacles.push((i, b.points.clone(), gap));
            }
        }
        for z in &job.fabric.defects {
            if bbox(&z.polygon).is_some_and(|b| boxes_near(swept, b, 0.)) {
                polygon::events(&a.points, &z.polygon, v, 0., &mut times);
                obstacles.push((i, z.polygon.clone(), 0.));
            }
        }
    }
    times.retain(|t| t.is_finite() && *t >= 0. && *t <= max);
    times.sort_by(f64::total_cmp);
    times.dedup_by(|a, b| (*a - *b).abs() < EPS);
    let blocked = |t: f64| {
        obstacles
            .iter()
            .any(|(i, b, gap)| polygon::conflict(&polygon::shifted(&sh[*i].points, v, t), b, *gap))
    };
    let initial = blocked(0.);
    if initial && !resolve {
        return Err("Selection overlaps or violates a gap. Use Resolve by bump.".into());
    }
    let mut travel = max;
    for pair in times.windows(2) {
        if pair[1] - pair[0] < EPS {
            continue;
        }
        let bad = blocked((pair[0] + pair[1]) * 0.5);
        if initial {
            if !bad {
                travel = (pair[0] + EPS * 4.).min(pair[1]);
                break;
            }
        } else if bad {
            travel = pair[0].max(0.);
            break;
        }
    }
    if initial && blocked(travel) {
        return Err("No clear position in this direction before the boundary.".into());
    }
    for &i in &selected {
        let p = &mut result.placements[i];
        p.translate[0] += v[0] * travel;
        p.translate[1] += v[1] * travel;
    }
    Ok(travel)
}
fn metrics(job: &Job, r: &mut MarkerResult) {
    let m = beta_marker_core::compute_metrics(job, &r.placements);
    r.length = m.length;
    r.piece_area = m.piece_area;
    r.efficiency = m.efficiency;
    r.width = job.fabric.width;
    r.engine = "manual / Rust editing".into();
}
fn center(sh: &[Shape], indices: &[usize]) -> Pt {
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for &i in indices {
        b[0] = b[0].min(sh[i].bounds[0]);
        b[1] = b[1].min(sh[i].bounds[1]);
        b[2] = b[2].max(sh[i].bounds[2]);
        b[3] = b[3].max(sh[i].bounds[3]);
    }
    [(b[0] + b[2]) / 2., (b[1] + b[3]) / 2.]
}
fn rotate_group(
    job: &Job,
    r: &mut MarkerResult,
    indices: &[usize],
    degrees: f64,
) -> Result<(), String> {
    if !degrees.is_finite() {
        return Err("Angle must be finite.".into());
    }
    let pivot = center(&shapes(job, r)?, indices);
    for &i in indices {
        let p = &mut r.placements[i];
        let a = norm_deg(p.rotation_deg + degrees);
        let piece = job
            .pieces
            .iter()
            .find(|q| q.id == p.piece_id)
            .ok_or("Unknown piece.")?;
        if !allowed_rotations(job, piece, p.mirrored).allows(a) {
            return Err(format!("{} does not allow that angle or tilt.", piece.name));
        }
        let t = polygon::sub(p.translate, pivot);
        let q = transform_point(t, false, degrees, pivot);
        p.translate = q;
        p.rotation_deg = a;
    }
    Ok(())
}
fn set_copy(r: &MarkerResult, p: &mut Placement) {
    let used: BTreeSet<_> = r
        .placements
        .iter()
        .filter(|q| q.piece_id == p.piece_id && q.mirrored == p.mirrored)
        .map(|q| q.copy)
        .collect();
    p.copy = (0..=used.len() as u32)
        .find(|i| !used.contains(i))
        .unwrap_or(0);
}
/// Compute remaining physical demand, respecting face state and folded layer counts.
pub fn unplaced(job: &Job, r: &MarkerResult, s: &Settings) -> Result<Vec<Placement>, String> {
    check_counts(job)?;
    let mut remaining: BTreeMap<(String, bool), i64> = BTreeMap::new();
    for d in demands(job) {
        let p = &job.pieces[d.piece_index];
        *remaining
            .entry((d.piece_id, d.mirrored && !flip_allowed(job, p)))
            .or_default() += i64::from(d.count);
    }
    if remaining.values().sum::<i64>() > 10000 {
        return Err("Manual editing supports at most 10,000 required copies.".into());
    }
    let consume = |p: &Placement, counts: &mut BTreeMap<(String, bool), i64>| {
        let free = job
            .pieces
            .iter()
            .find(|q| q.id == p.piece_id)
            .is_some_and(|q| flip_allowed(job, q));
        let (a, b) = layer_copies(job.fabric.layout, p);
        for (m, n) in [(false, a), (true, b)] {
            *counts.entry((p.piece_id.clone(), m && !free)).or_default() -= i64::from(n);
        }
    };
    for p in &r.placements {
        consume(p, &mut remaining);
    }
    let mut out = vec![];
    let mut occupied = r.clone();
    for p in &s.removed {
        let free = job
            .pieces
            .iter()
            .find(|q| q.id == p.piece_id)
            .is_some_and(|q| flip_allowed(job, q));
        if remaining
            .get(&(p.piece_id.clone(), p.mirrored && !free))
            .copied()
            .unwrap_or(0)
            > 0
        {
            let mut p = p.clone();
            p.locked = false;
            set_copy(&occupied, &mut p);
            consume(&p, &mut remaining);
            occupied.placements.push(p.clone());
            out.push(p);
        }
    }
    let bundles = bundle_sizes(job);
    for ((id, mirrored), _) in remaining.clone() {
        while remaining.get(&(id.clone(), mirrored)).copied().unwrap_or(0) > 0 {
            let piece = job
                .pieces
                .iter()
                .find(|p| p.id == id)
                .ok_or("Unknown demanded piece.")?;
            let bundle = bundles
                .iter()
                .enumerate()
                .filter(|(_, size)| **size == piece.size)
                .find(|(k, _)| {
                    let n = occupied
                        .placements
                        .iter()
                        .filter(|p| p.piece_id == id && p.bundle == Some(*k as u32))
                        .map(|p| {
                            let (a, b) = layer_copies(job.fabric.layout, p);
                            if flip_allowed(job, piece) {
                                a + b
                            } else if mirrored {
                                b
                            } else {
                                a
                            }
                        })
                        .sum::<u32>();
                    n < if flip_allowed(job, piece) {
                        piece.quantity + piece.mirror_quantity
                    } else if mirrored {
                        piece.mirror_quantity
                    } else {
                        piece.quantity
                    }
                })
                .map(|(k, _)| k as u32);
            let mut p = Placement {
                piece_id: id.clone(),
                mirrored,
                bundle,
                ..Default::default()
            };
            p.rotation_deg = allowed_rotations(job, piece, mirrored)
                .angles
                .first()
                .copied()
                .unwrap_or(0.);
            set_copy(&occupied, &mut p);
            consume(&p, &mut remaining);
            occupied.placements.push(p.clone());
            out.push(p);
        }
    }
    Ok(out)
}
/// Full live inspection. Pair overlap is summed pairwise; triple-covered areas count per pair.
pub fn inspect(job: &Job, result: &MarkerResult) -> Result<Inspection, String> {
    check_counts(job)?;
    let job = effective_job(job);
    let s = settings(&job)?;
    let sh = shapes(&job, result)?;
    let mut pairs = vec![];
    for (i, a) in sh.iter().enumerate() {
        for (j, b) in sh[..i].iter().enumerate() {
            let gap = job.spacing.piece_gap + a.buffer + b.buffer;
            if !boxes_near(a.bounds, b.bounds, gap) {
                continue;
            }
            let overlap = polygon::overlap_area(&a.points, &b.points);
            let d = polygon::distance(&a.points, &b.points);
            if overlap > EPS || d < gap - EPS {
                pairs.push(PairCheck {
                    a: j,
                    b: i,
                    overlap_mm2: overlap,
                    distance_mm: d,
                    required_mm: gap,
                });
            }
        }
    }
    let validation = beta_marker_core::validate(&job, result);
    let table_length = limit(&job, result, &s);
    Ok(Inspection {
        table_efficiency: if table_length > 0. {
            validation.piece_area / (job.fabric.width * table_length)
        } else {
            0.
        },
        table_length,
        geometry_ok: geometry_ok(&job, result, &s)?,
        end_violations: sh
            .iter()
            .enumerate()
            .filter(|(_, a)| {
                s.table_length
                    .or(job.fabric.max_length)
                    .is_some_and(|x| a.bounds[2] > x + EPS)
            })
            .map(|(i, _)| i)
            .collect(),
        section_violations: sh
            .iter()
            .enumerate()
            .filter(|(_, a)| section_bad(a, &s))
            .map(|(i, _)| i)
            .collect(),
        unplaced: unplaced(&job, result, &s)?,
        validation,
        pairs,
    })
}
fn place_one(
    job: &Job,
    r: &mut MarkerResult,
    s: &Settings,
    mut p: Placement,
    at: Pt,
) -> Result<bool, String> {
    if at.iter().any(|x| !x.is_finite()) {
        return Err("Placement coordinates must be finite.".into());
    }
    p.translate = [0., 0.];
    let piece = job
        .pieces
        .iter()
        .find(|q| q.id == p.piece_id)
        .ok_or("Unknown piece.")?;
    let b = bbox(&cut_outline(piece, &p)).ok_or("Empty piece.")?;
    p.translate = [at[0] - b[0], at[1] - b[1]];
    set_copy(r, &mut p);
    r.placements.push(p);
    if !geometry_ok(job, r, s)? {
        r.placements.pop();
        return Ok(false);
    }
    let i = r.placements.len() - 1;
    let _ = bump(job, r, &[i], Direction::Left, s, false)?;
    Ok(true)
}
/// Apply one command atomically; rejected commands never mutate caller data.
pub fn apply(job: &Job, result: &MarkerResult, request: Request) -> Result<Output, String> {
    check_counts(job)?;
    let issues = beta_marker_core::check_job(job);
    if let Some(issue) = issues.first() {
        return Err(issue.message.clone());
    }
    let mut out = Output {
        job: job.clone(),
        result: result.clone(),
        selected: request.selected,
        measurement: None,
        notice: String::new(),
    };
    let effective = effective_job(job);
    let mut s = settings(job)?;
    let r = &mut out.result;
    out.selected.sort_unstable();
    out.selected.dedup();
    if out.selected.iter().any(|&i| i >= r.placements.len()) {
        return Err("Selection is out of range.".into());
    }
    let indices = out.selected.clone();
    if !matches!(
        request.action,
        Action::Select { .. }
            | Action::Rectangle { .. }
            | Action::Measure { .. }
            | Action::Section { .. }
            | Action::Length { .. }
            | Action::Compact
            | Action::CloseEnd
            | Action::Place { .. }
            | Action::FillGaps
    ) && (indices.is_empty() || indices.iter().any(|&i| r.placements[i].locked))
    {
        return Err("Select unlocked pieces first.".into());
    }
    let mut require_clear = false;
    match request.action {
        Action::Bump { direction } | Action::Resolve { direction } => {
            let resolve = matches!(request.action, Action::Resolve { .. });
            let mm = bump(&effective, r, &indices, direction, &s, resolve)?;
            out.notice = format!("Moved {mm:.3} mm to polygon contact.");
        }
        Action::Compact | Action::CloseEnd => {
            let close = matches!(request.action, Action::CloseEnd);
            let sh = shapes(&effective, r)?;
            let mut order: Vec<_> = (0..r.placements.len())
                .filter(|&i| !r.placements[i].locked)
                .collect();
            order.sort_by(|&a, &b| sh[a].bounds[0].total_cmp(&sh[b].bounds[0]));
            if close {
                let end = sh.iter().map(|a| a.bounds[2]).fold(0., f64::max);
                order.retain(|&i| sh[i].bounds[2] >= end - EPS);
            }
            for _ in 0..if close { 1 } else { r.placements.len().max(1) } {
                let sh = shapes(&effective, r)?;
                order.sort_by(|&a, &b| sh[a].bounds[0].total_cmp(&sh[b].bounds[0]));
                let mut moved = 0.;
                for &i in &order {
                    moved += bump(&effective, r, &[i], Direction::Left, &s, false)?;
                }
                if moved < EPS {
                    break;
                }
            }
        }
        Action::Move { delta } => {
            if delta.iter().any(|x| !x.is_finite()) {
                return Err("Move distances must be finite.".into());
            }
            for &i in &indices {
                r.placements[i].translate[0] += delta[0];
                r.placements[i].translate[1] += delta[1];
            }
            require_clear = true;
        }
        Action::Rotate { degrees } | Action::Tilt { degrees } => {
            rotate_group(&effective, r, &indices, degrees)?;
            require_clear = true;
        }
        Action::NextRotation => {
            let p = &r.placements[indices[0]];
            let piece = effective
                .pieces
                .iter()
                .find(|q| q.id == p.piece_id)
                .ok_or("Unknown piece.")?;
            let allowed = allowed_rotations(&effective, piece, p.mirrored);
            let degrees = if allowed.free {
                15.
            } else {
                allowed
                    .angles
                    .iter()
                    .map(|a| norm_deg(*a - p.rotation_deg))
                    .filter(|d| *d > EPS)
                    .min_by(f64::total_cmp)
                    .unwrap_or(0.)
            };
            rotate_group(&effective, r, &indices, degrees)?;
            require_clear = true;
        }
        Action::Flip => {
            let sh = shapes(&effective, r)?;
            for &i in &indices {
                let p = &mut r.placements[i];
                let piece = effective
                    .pieces
                    .iter()
                    .find(|q| q.id == p.piece_id)
                    .ok_or("Unknown piece.")?;
                if !flip_allowed(&effective, piece) {
                    return Err(format!("{} does not allow face-down flipping.", piece.name));
                }
                let center = center(&sh, &[i]);
                p.mirrored = !p.mirrored;
                let rule = allowed_rotations(&effective, piece, p.mirrored);
                if !rule.allows(p.rotation_deg) {
                    p.rotation_deg = rule
                        .angles
                        .iter()
                        .min_by(|a, b| {
                            angle_diff_deg(**a, p.rotation_deg)
                                .total_cmp(&angle_diff_deg(**b, p.rotation_deg))
                        })
                        .copied()
                        .unwrap_or(p.rotation_deg);
                }
                let b = bbox(&cut_outline(piece, p)).ok_or("Empty piece.")?;
                p.translate[0] += center[0] - (b[0] + b[2]) / 2.;
                p.translate[1] += center[1] - (b[1] + b[3]) / 2.;
            }
            // Copy ids are per mirror state. Flipping into an occupied id must
            // not create duplicate identities in labels or later manual placement.
            for &i in &indices {
                let p = &r.placements[i];
                let used: BTreeSet<_> = r
                    .placements
                    .iter()
                    .enumerate()
                    .filter(|(j, q)| {
                        *j != i && q.piece_id == p.piece_id && q.mirrored == p.mirrored
                    })
                    .map(|(_, q)| q.copy)
                    .collect();
                r.placements[i].copy = (0..=used.len() as u32)
                    .find(|n| !used.contains(n))
                    .unwrap_or(0);
            }
            require_clear = true;
        }
        Action::Swap => {
            if indices.len() != 2 {
                return Err("Select exactly two unlocked pieces to swap.".into());
            }
            let sh = shapes(&effective, r)?;
            let a = center(&sh, &[indices[0]]);
            let b = center(&sh, &[indices[1]]);
            for (i, d) in [
                (indices[0], polygon::sub(b, a)),
                (indices[1], polygon::sub(a, b)),
            ] {
                r.placements[i].translate[0] += d[0];
                r.placements[i].translate[1] += d[1];
            }
            require_clear = true;
        }
        Action::Remove => {
            for &i in indices.iter().rev() {
                let mut p = r.placements.remove(i);
                p.locked = false;
                s.removed.push(p);
            }
            out.selected.clear();
        }
        Action::Place {
            unplaced: index,
            at,
        } => {
            let p = unplaced(&effective, r, &s)?
                .get(index)
                .cloned()
                .ok_or("Choose an unplaced copy.")?;
            if !place_one(&effective, r, &s, p.clone(), at)? {
                return Err(
                    "That position has no room. Choose a clear point inside the marker.".into(),
                );
            }
            forget_removed(&mut s, &p);
            out.selected = vec![r.placements.len() - 1];
        }
        Action::FillGaps => {
            let list = unplaced(&effective, r, &s)?;
            let mut count = 0;
            let mut attempts = 0;
            'pieces: for p in list {
                let sh = shapes(&effective, r)?;
                let piece = effective
                    .pieces
                    .iter()
                    .find(|q| q.id == p.piece_id)
                    .ok_or("Unknown piece.")?;
                let edge = effective.spacing.edge_gap + piece.buffer;
                let mut xs = vec![edge];
                let mut ys = vec![edge];
                for a in &sh {
                    let g = effective.spacing.piece_gap + piece.buffer + a.buffer;
                    xs.push(a.bounds[2] + g);
                    ys.push(a.bounds[3] + g);
                }
                for &x in &s.sections {
                    xs.push(x);
                }
                xs.sort_by(f64::total_cmp);
                ys.sort_by(f64::total_cmp);
                let mut placed = false;
                'search: for y in ys {
                    for &x in &xs {
                        attempts += 1;
                        if attempts > 2000 {
                            break 'pieces;
                        }
                        if place_one(&effective, r, &s, p.clone(), [x, y])? {
                            forget_removed(&mut s, &p);
                            count += 1;
                            placed = true;
                            break 'search;
                        }
                    }
                }
                if !placed {
                    continue;
                }
            }
            out.notice =
                format!("Placed {count} copies in free spaces. Remaining copies stay in the list.");
        }
        Action::Select { field, value } => {
            out.selected = r
                .placements
                .iter()
                .enumerate()
                .filter(|(_, p)| match field.as_str() {
                    "bundle" => {
                        p.bundle.map(|v| (v + 1).to_string()).as_deref() == Some(value.as_str())
                    }
                    "size" => {
                        effective
                            .pieces
                            .iter()
                            .find(|q| q.id == p.piece_id)
                            .and_then(|q| q.size.as_deref())
                            == Some(value.as_str())
                    }
                    "name" => effective.pieces.iter().any(|q| {
                        q.id == p.piece_id && q.name.to_lowercase().contains(&value.to_lowercase())
                    }),
                    "all" => true,
                    _ => false,
                })
                .map(|(i, _)| i)
                .collect();
        }
        Action::Rectangle { from, to } => {
            if from.iter().chain(to.iter()).any(|x| !x.is_finite()) {
                return Err("Selection coordinates must be finite.".into());
            }
            let b = [
                from[0].min(to[0]),
                from[1].min(to[1]),
                from[0].max(to[0]),
                from[1].max(to[1]),
            ];
            out.selected = shapes(&effective, r)?
                .iter()
                .enumerate()
                .filter(|(_, a)| {
                    a.bounds[0] >= b[0]
                        && a.bounds[1] >= b[1]
                        && a.bounds[2] <= b[2]
                        && a.bounds[3] <= b[3]
                })
                .map(|(i, _)| i)
                .collect();
        }
        Action::Section { x, remove } => {
            if !x.is_finite() || x <= 0. || x >= limit(&effective, r, &s) {
                return Err("Section line must lie inside the marker length.".into());
            }
            s.sections.retain(|v| (*v - x).abs() > 0.01);
            if !remove {
                s.sections.push(x);
                s.sections.sort_by(f64::total_cmp);
            }
            if shapes(&effective, r)?.iter().any(|a| section_bad(a, &s)) {
                return Err(
                    "A piece crosses this section line. Move it before adding the line.".into(),
                );
            }
        }
        Action::Length { length } => {
            if !length.is_finite()
                || length <= 0.
                || job.fabric.max_length.is_some_and(|m| length > m)
            {
                return Err("Table length must be positive and within the job maximum.".into());
            }
            let sh = shapes(&effective, r)?;
            if sh.iter().any(|a| a.bounds[2] > length + EPS)
                || s.sections.iter().any(|&x| x >= length)
            {
                return Err("The end cannot pass a placed piece or section line.".into());
            }
            s.table_length = Some(length);
        }
        Action::Measure { from, to } => {
            if from.iter().chain(to.iter()).any(|x| !x.is_finite()) {
                return Err("Measurement coordinates must be finite.".into());
            }
            out.measurement = Some((to[0] - from[0]).hypot(to[1] - from[1]));
        }
    }
    if require_clear && !geometry_ok(&effective, r, &s)? {
        return Err("This move would overlap a piece, gap, defect, section or fabric edge.".into());
    }
    save_settings(&mut out.job, &s)?;
    metrics(&effective, r);
    Ok(out)
}
/// Reuse requires the same piece styles, ratio, width, rules and unchanged pieces.
/// Piece order, display job name and free-form metadata may differ. Revalidate saved geometry.
pub fn reuse(current: &Job, saved_job: &Job, saved: &MarkerResult) -> Result<MarkerResult, String> {
    let mut a = current.clone();
    let mut b = saved_job.clone();
    a.name.clear();
    b.name.clear();
    a.meta.clear();
    b.meta.clear();
    a.pieces.sort_by(|a, b| a.id.cmp(&b.id));
    b.pieces.sort_by(|a, b| a.id.cmp(&b.id));
    if a != b {
        return Err(
            "Marker does not match: piece geometry, ids, style, ratio, width or rules changed."
                .into(),
        );
    }
    let mut r = saved.clone();
    r.job_name = current.name.clone();
    let check = inspect(saved_job, &r)?;
    if !check.validation.ok || !check.geometry_ok {
        return Err("Saved marker no longer passes validation.".into());
    }
    Ok(r)
}

#[cfg(test)]
mod tests;
