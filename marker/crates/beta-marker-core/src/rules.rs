//! Garment rules: how many copies to cut, and which rotations each copy may take.

use crate::geometry::{angle_diff_deg, area, clip_left_of_line, grain_alignment_deg, norm_deg, transform_points};
use crate::model::{Issue, Job, Layout, Nap, Piece, Placement, Pt, RotationRule, JOB_FORMAT};
use std::collections::BTreeSet;

/// Copies of one piece shape to place (normal or mirrored).
#[derive(Debug, Clone, PartialEq)]
pub struct Demand {
    pub piece_index: usize,
    pub piece_id: String,
    pub mirrored: bool,
    pub count: u32,
}

/// Garment multiplier for a piece: garments of its size in the order, or 1 without an order.
/// A piece without a size, or with a size not in the order, gets 0.
pub fn multiplier(job: &Job, piece: &Piece) -> u32 {
    match &job.order {
        None => 1,
        Some(order) => piece
            .size
            .as_ref()
            .and_then(|s| order.sizes.get(s).copied())
            .unwrap_or(0),
    }
}

/// All demands of a job, in piece order (normal before mirrored). Zero counts are left out.
pub fn demands(job: &Job) -> Vec<Demand> {
    let mut out = Vec::new();
    for (i, p) in job.pieces.iter().enumerate() {
        let m = multiplier(job, p);
        for (mirrored, q) in [(false, p.quantity), (true, p.mirror_quantity)] {
            let count = q * m;
            if count > 0 {
                out.push(Demand { piece_index: i, piece_id: p.id.clone(), mirrored, count });
            }
        }
    }
    out
}

/// The rotation rule after resolving `inherit` against the fabric nap.
pub fn effective_rule(job: &Job, piece: &Piece) -> RotationRule {
    match piece.rotation {
        RotationRule::Inherit => match job.fabric.nap {
            Nap::None | Nap::OneWayPerBundle => RotationRule::Flip180,
            Nap::OneWay => RotationRule::Fixed,
        },
        r => r,
    }
}

/// Rotations allowed for one copy of a piece, in marker terms (grain alignment included).
#[derive(Debug, Clone, PartialEq)]
pub struct AllowedRotations {
    /// Absolute rotations in degrees, `[0, 360)`. Empty when `free`.
    pub angles: Vec<f64>,
    /// Allowed deviation from each angle, degrees.
    pub tilt_deg: f64,
    /// Any angle is allowed.
    pub free: bool,
}

impl AllowedRotations {
    /// `true` if `rotation_deg` is within tilt (plus a small tolerance) of an allowed angle.
    pub fn allows(&self, rotation_deg: f64) -> bool {
        self.free || self.angles.iter().any(|&a| angle_diff_deg(a, rotation_deg) <= self.tilt_deg + 1e-3)
    }
}

/// Allowed rotations for a piece copy.
///
/// Relative steps: `fixed` = {0}, `flip180` = {0, 180}, `quarter` = {0, 90, 180, 270},
/// `free` = any. One-way fabric removes the reversed directions (180 and 270).
/// Each step is added to the grain alignment angle of the (possibly mirrored) piece.
pub fn allowed_rotations(job: &Job, piece: &Piece, mirrored: bool) -> AllowedRotations {
    let one_way = job.fabric.nap == Nap::OneWay;
    let steps: &[f64] = match effective_rule(job, piece) {
        RotationRule::Free => {
            return AllowedRotations { angles: vec![], tilt_deg: 0.0, free: true };
        }
        RotationRule::Fixed | RotationRule::Inherit => &[0.0],
        RotationRule::Flip180 if one_way => &[0.0],
        RotationRule::Flip180 => &[0.0, 180.0],
        RotationRule::Quarter if one_way => &[0.0, 90.0],
        RotationRule::Quarter => &[0.0, 90.0, 180.0, 270.0],
    };
    let base = grain_alignment_deg(piece.grain, mirrored);
    AllowedRotations {
        angles: steps.iter().map(|s| norm_deg(base + s)).collect(),
        tilt_deg: piece.tilt_deg.unwrap_or(job.fabric.tilt_deg).max(0.0),
        free: false,
    }
}

/// Copies cut by one placement, as `(normal, mirrored)`.
///
/// Open fabric: one copy in the placed mirror state. Folded or tubular fabric: two layers,
/// the placed copy and its mirror image, so `(1, 1)`; an `on_fold` placement cuts one full
/// (symmetric) piece and counts as `(1, 0)`.
pub fn layer_copies(layout: Layout, placement: &Placement) -> (u32, u32) {
    match layout {
        Layout::Open if placement.mirrored => (0, 1),
        Layout::Open => (1, 0),
        _ if placement.on_fold => (1, 0),
        _ => (1, 1),
    }
}

/// The outline a placement puts on the marker, in marker coordinates: the full outline, or
/// for an `on_fold` placement the half on the left of the piece's `fold_line`.
pub fn placed_outline(piece: &Piece, placement: &Placement) -> Vec<Pt> {
    let local = match (placement.on_fold, piece.fold_line) {
        (true, Some([a, b])) => clip_left_of_line(&piece.outline, a, b),
        _ => piece.outline.clone(),
    };
    transform_points(&local, placement.mirrored, placement.rotation_deg, placement.translate)
}

/// Physical cut boundary, including a grain-aligned fusing block margin.
/// Apply `effective_job` first. On-fold placements use the clipped half outline.
pub fn cut_outline(piece: &Piece, placement: &Placement) -> Vec<Pt> {
    let Some(margin) = piece.block_margin else {
        return placed_outline(piece, placement);
    };
    let local = placed_outline(
        piece,
        &Placement {
            on_fold: placement.on_fold,
            ..Default::default()
        },
    );
    let angle = grain_alignment_deg(piece.grain, false);
    let aligned = transform_points(&local, false, angle, [0.0; 2]);
    let Some([x, y, max_x, max_y]) = crate::geometry::bbox(&aligned) else {
        return vec![];
    };
    let rectangle = [
        [x - margin, y - margin],
        [max_x + margin, y - margin],
        [max_x + margin, max_y + margin],
        [x - margin, max_y + margin],
    ];
    crate::geometry::place(
        &transform_points(&rectangle, false, -angle, [0.0; 2]),
        placement,
    )
}

/// The job as the nesting engine and the validator must see it: fabric shrinkage applied.
///
/// Every piece (outline, grain, marks, match points, fold line) is scaled in its
/// grain-aligned frame by `1 + length_pct / 100` along the grain and `1 + width_pct / 100`
/// across it, around the piece origin. Placements always refer to the effective job.
/// Shrinkage is then set to zero so that applying this twice changes nothing.
pub fn effective_job(job: &Job) -> Job {
    let sh = job.fabric.shrinkage;
    let mut out = job.clone();
    out.fabric.shrinkage = Default::default();
    if sh.length_pct == 0.0 && sh.width_pct == 0.0 {
        return out;
    }
    let (fx, fy) = (1.0 + sh.length_pct / 100.0, 1.0 + sh.width_pct / 100.0);
    for p in &mut out.pieces {
        let a = grain_alignment_deg(p.grain, false);
        let scale = |q: Pt| -> Pt {
            let r = transform_points(&[q], false, a, [0.0, 0.0])[0];
            transform_points(&[[r[0] * fx, r[1] * fy]], false, -a, [0.0, 0.0])[0]
        };
        p.outline = p.outline.iter().map(|&q| scale(q)).collect();
        p.grain = p.grain.map(|[g0, g1]| [scale(g0), scale(g1)]);
        p.fold_line = p.fold_line.map(|[f0, f1]| [scale(f0), scale(f1)]);
        for m in &mut p.marks {
            m.points = m.points.iter().map(|&q| scale(q)).collect();
        }
        for mp in &mut p.match_points {
            mp.point = scale(mp.point);
        }
    }
    out
}

/// `true` if copies of this piece may be placed mirrored freely (face down):
/// the piece's `flip` override, else `fabric.allow_flip`. Counts are then pooled per piece.
pub fn flip_allowed(job: &Job, piece: &Piece) -> bool {
    piece.flip.unwrap_or(job.fabric.allow_flip)
}

/// Sort key for size labels: standard letter sizes in garment order, then numeric sizes
/// by value, then any other label alphabetically.
pub fn size_order_key(size: &str) -> (u8, f64, String) {
    const LETTERS: [&str; 11] = ["XXXS", "XXS", "XS", "S", "M", "L", "XL", "XXL", "3XL", "4XL", "5XL"];
    let upper = size.trim().to_ascii_uppercase();
    if let Some(i) = LETTERS.iter().position(|s| *s == upper) {
        return (0, i as f64, upper);
    }
    match upper.parse::<f64>() {
        Ok(v) if v.is_finite() => (1, v, upper),
        _ => (2, 0.0, upper),
    }
}

/// Canonical garment (bundle) order: one entry per garment, sizes in `size_order_key`
/// order, each size repeated by its ratio. Bundle `k` is garment `k` of this list.
///
/// Without an order every piece is cut once per garment of its own size, so there is one
/// garment per distinct size among pieces with a non-zero quantity (pieces without a size
/// form one garment, listed first as `None`).
pub fn bundle_sizes(job: &Job) -> Vec<Option<String>> {
    let cmp = |a: &str, b: &str| {
        let (ka, kb) = (size_order_key(a), size_order_key(b));
        ka.0.cmp(&kb.0).then(ka.1.total_cmp(&kb.1)).then(ka.2.cmp(&kb.2))
    };
    let Some(order) = &job.order else {
        let mut sizes: Vec<Option<String>> = job
            .pieces
            .iter()
            .filter(|p| p.quantity > 0 || p.mirror_quantity > 0)
            .map(|p| p.size.clone())
            .collect();
        sizes.sort_by(|a, b| match (a, b) {
            (Some(a), Some(b)) => cmp(a, b),
            _ => a.is_some().cmp(&b.is_some()),
        });
        sizes.dedup();
        return sizes;
    };
    let mut sizes: Vec<(&String, &u32)> = order.sizes.iter().collect();
    sizes.sort_by(|a, b| cmp(a.0, b.0));
    sizes
        .into_iter()
        .flat_map(|(s, &n)| std::iter::repeat_n(Some(s.clone()), n as usize))
        .collect()
}

/// Check a job before nesting. Returns errors; an empty list means the job is usable.
pub fn check_job(job: &Job) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut push = |code: &str, message: String, ids: Vec<String>| {
        issues.push(Issue { code: code.into(), message, piece_ids: ids });
    };
    if job.format != JOB_FORMAT {
        push("format", format!("format must be \"{JOB_FORMAT}\", found \"{}\"", job.format), vec![]);
    }
    if job.fabric.width.is_nan() || job.fabric.width <= 0.0 {
        push("fabric_width", "fabric width must be greater than 0".into(), vec![]);
    }
    if job.spacing.piece_gap < 0.0 || job.spacing.edge_gap < 0.0 {
        push("spacing", "gaps must not be negative".into(), vec![]);
    }
    let mut seen = BTreeSet::new();
    for p in &job.pieces {
        if !seen.insert(p.id.as_str()) {
            push("duplicate_id", format!("piece id \"{}\" is used more than once", p.id), vec![p.id.clone()]);
        }
        if p.outline.len() < 3 || area(&p.outline) <= 0.0 {
            push("bad_outline", format!("piece \"{}\" has no usable outline", p.id), vec![p.id.clone()]);
        }
        if p.outline.iter().any(|q| !q[0].is_finite() || !q[1].is_finite()) {
            push("bad_outline", format!("piece \"{}\" has a non-finite point", p.id), vec![p.id.clone()]);
        }
        if p.buffer < 0.0 {
            push("spacing", format!("piece \"{}\" has a negative buffer", p.id), vec![p.id.clone()]);
        }
    }
    if let Some(sh) = job.fabric.shade
        && (sh.band_mm.is_nan() || sh.band_mm <= 0.0)
    {
        push("shade", "shade band size must be greater than 0".into(), vec![]);
    }
    if let Some(r) = job.fabric.repeat
        && (r.period_x.is_some_and(|v| v.is_nan() || v <= 0.0) || r.period_y.is_some_and(|v| v.is_nan() || v <= 0.0))
    {
        push("repeat", "repeat periods must be greater than 0".into(), vec![]);
    }
    for rule in &job.match_rules {
        for pr in std::iter::once(&rule.a).chain(rule.b.as_ref()) {
            let found = job
                .pieces
                .iter()
                .find(|p| p.id == pr.piece_id)
                .is_some_and(|p| p.match_points.iter().any(|m| m.id == pr.point_id));
            if !found {
                push(
                    "match_rule",
                    format!("match rule refers to unknown point {}/{}", pr.piece_id, pr.point_id),
                    vec![pr.piece_id.clone()],
                );
            }
        }
    }
    if !job.match_rules.is_empty() && job.fabric.repeat.is_none() {
        push("match_rule", "match rules need fabric.repeat".into(), vec![]);
    }
    if demands(job).is_empty() {
        push("empty", "no pieces to place (check quantities and the size ratio)".into(), vec![]);
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use std::collections::BTreeMap;

    fn job(nap: Nap) -> Job {
        let piece = |id: &str, size: &str| Piece {
            id: id.into(),
            name: id.into(),
            style: None,
            size: Some(size.into()),
            material: None,
            outline: vec![[0.0, 0.0], [100.0, 0.0], [100.0, 50.0], [0.0, 50.0]],
            grain: Some([[0.0, 0.0], [0.0, 10.0]]),
            quantity: 1,
            mirror_quantity: 1,
            rotation: RotationRule::Inherit,
            buffer: 0.0,
            marks: vec![],
            ..Default::default()
        };
        Job {
        match_rules: Vec::new(),
            format: JOB_FORMAT.into(),
            name: "t".into(),
            fabric: Fabric { width: 1500.0, max_length: None, nap, allow_flip: false, tilt_deg: 0.0, ..Default::default() },
            spacing: Spacing::default(),
            pieces: vec![piece("FRONT-M", "M"), piece("FRONT-L", "L"), piece("FRONT-XL", "XL")],
            order: Some(Order { sizes: BTreeMap::from([("M".into(), 2), ("L".into(), 1)]) }),
            meta: BTreeMap::new(),
        }
    }

    #[test]
    fn demand_follows_size_ratio_and_pairs() {
        let d = demands(&job(Nap::None));
        let counts: Vec<_> = d.iter().map(|d| (d.piece_id.as_str(), d.mirrored, d.count)).collect();
        assert_eq!(
            counts,
            vec![("FRONT-M", false, 2), ("FRONT-M", true, 2), ("FRONT-L", false, 1), ("FRONT-L", true, 1)]
        );
    }

    #[test]
    fn nap_controls_rotations() {
        let j = job(Nap::None);
        let r = allowed_rotations(&j, &j.pieces[0], false);
        assert_eq!(r.angles, vec![270.0, 90.0]);
        assert!(r.allows(90.0) && !r.allows(0.0));
        let j = job(Nap::OneWay);
        let r = allowed_rotations(&j, &j.pieces[0], false);
        assert_eq!(r.angles, vec![270.0]);
    }

    #[test]
    fn bundles_use_garment_size_order() {
        let mut j = job(Nap::None);
        j.order = Some(Order { sizes: BTreeMap::from([("XL".into(), 1), ("M".into(), 2), ("S".into(), 1)]) });
        let b: Vec<_> = bundle_sizes(&j).into_iter().map(|s| s.unwrap_or_default()).collect();
        assert_eq!(b, vec!["S", "M", "M", "XL"]);
        j.order = Some(Order { sizes: BTreeMap::from([("34".into(), 1), ("30".into(), 1), ("100".into(), 1)]) });
        let b: Vec<_> = bundle_sizes(&j).into_iter().map(|s| s.unwrap_or_default()).collect();
        assert_eq!(b, vec!["30", "34", "100"]);
        j.order = None;
        assert_eq!(bundle_sizes(&j), vec![Some("M".into()), Some("L".into()), Some("XL".into())]);
    }

    #[test]
    fn check_job_finds_problems() {
        let mut j = job(Nap::None);
        assert!(check_job(&j).is_empty());
        j.pieces[1].id = "FRONT-M".into();
        j.fabric.width = 0.0;
        let codes: Vec<_> = check_job(&j).into_iter().map(|i| i.code).collect();
        assert!(codes.contains(&"duplicate_id".to_string()) && codes.contains(&"fabric_width".to_string()));
    }
}
