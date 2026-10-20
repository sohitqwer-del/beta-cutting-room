//! Conservative full-outline predicates and sampled contact searches.
use beta_marker_core::Pt;
use beta_marker_core::geometry::{bbox, transform_points};
use geo::{
    BooleanOps, Buffer, ConvexHull, Covers, Distance, Euclidean, Intersects, LineString,
    MultiPoint, Polygon, Validation,
};

pub(crate) const GUARD: f64 = 0.02;

pub(crate) fn polygon(points: &[Pt]) -> Polygon<f64> {
    Polygon::new(LineString::from(points.to_vec()), vec![])
}

pub(crate) fn hull(points: &[Pt]) -> Vec<Pt> {
    let p = MultiPoint::from(points.to_vec()).convex_hull();
    let mut out: Vec<_> = p.exterior().0.iter().map(|c| [c.x, c.y]).collect();
    if out.first() == out.last() {
        out.pop();
    }
    out
}

/// Join disjoint outlines by a narrow bridge. The returned simple exterior
/// covers both members. Any uncertain Boolean result falls back to the caller's
/// convex envelope. Holes are conservatively filled by keeping the exterior.
pub(crate) fn connected_envelope(a: &[Pt], b: &[Pt]) -> Option<Vec<Pt>> {
    let mut closest: Option<(f64, Pt, Pt)> = None;
    let project = |p: Pt, a: Pt, b: Pt| {
        let d = [b[0] - a[0], b[1] - a[1]];
        let l2 = d[0] * d[0] + d[1] * d[1];
        let t = if l2 > 0.0 {
            ((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / l2
        } else {
            0.0
        };
        [
            a[0] + t.clamp(0.0, 1.0) * d[0],
            a[1] + t.clamp(0.0, 1.0) * d[1],
        ]
    };
    let mut consider = |p: Pt, q: Pt| {
        let d2 = (p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2);
        if closest.as_ref().is_none_or(|c| d2 < c.0) {
            closest = Some((d2, p, q));
        }
    };
    for &p in a {
        for i in 0..b.len() {
            consider(p, project(p, b[i], b[(i + 1) % b.len()]));
        }
    }
    for &q in b {
        for i in 0..a.len() {
            consider(project(q, a[i], a[(i + 1) % a.len()]), q);
        }
    }
    let (distance2, from, to) = closest?;
    if distance2 <= 0.0 || !distance2.is_finite() {
        return None;
    }
    let distance = distance2.sqrt();
    let u = [(to[0] - from[0]) / distance, (to[1] - from[1]) / distance];
    let n = [-u[1] * GUARD, u[0] * GUARD];
    let start = [from[0] - u[0] * GUARD, from[1] - u[1] * GUARD];
    let end = [to[0] + u[0] * GUARD, to[1] + u[1] * GUARD];
    let bridge = polygon(&[
        [start[0] + n[0], start[1] + n[1]],
        [end[0] + n[0], end[1] + n[1]],
        [end[0] - n[0], end[1] - n[1]],
        [start[0] - n[0], start[1] - n[1]],
    ]);
    let (aa, bb) = (polygon(a), polygon(b));
    let joined = aa.union(&bridge).union(&bb);
    if joined.0.len() != 1 {
        return None;
    }
    let mut outline: Vec<_> = joined.0[0]
        .exterior()
        .0
        .iter()
        .map(|p| [p.x, p.y])
        .collect();
    if outline.first() == outline.last() {
        outline.pop();
    }
    let envelope = polygon(&outline);
    if !envelope.is_valid() {
        return None;
    }
    if envelope.covers(&aa) && envelope.covers(&bb) {
        return Some(outline);
    }
    // Boolean clipping can move a boundary by a few ulps. Reserve explicit
    // outward clearance, then verify full coverage again rather than accepting
    // a small uncovered sliver.
    let grown = envelope.buffer(GUARD);
    if grown.0.len() != 1 {
        return None;
    }
    let mut outline: Vec<_> = grown.0[0].exterior().0.iter().map(|p| [p.x, p.y]).collect();
    if outline.first() == outline.last() {
        outline.pop();
    }
    let guarded = polygon(&outline);
    (guarded.is_valid() && guarded.covers(&aa) && guarded.covers(&bb)).then_some(outline)
}

pub(crate) fn separated(a: &[Pt], b: &[Pt], gap: f64) -> bool {
    let (Some(aa), Some(bb)) = (bbox(a), bbox(b)) else {
        return false;
    };
    if aa[0] >= bb[2] + gap || bb[0] >= aa[2] + gap || aa[1] >= bb[3] + gap || bb[1] >= aa[3] + gap
    {
        return true;
    }
    let (a, b) = (polygon(a), polygon(b));
    !a.intersects(&b) && Euclidean.distance(&a, &b) >= gap
}

/// Slide from outside the two bounding boxes to the first sampled contact.
/// Bisection refines that boundary. Feasibility can be non-monotone; we do not
/// claim to enumerate every no-fit-polygon boundary or narrow pocket.
pub(crate) fn contacts(a: &[Pt], b: &[Pt], gap: f64, mut visit: impl FnMut([f64; 2]) -> bool) {
    let (Some(aa), Some(bb)) = (bbox(a), bbox(b)) else {
        return;
    };
    for axis in 0..2 {
        let other = 1 - axis;
        let low = aa[other] - bb[other + 2] - gap;
        let high = aa[other + 2] - bb[other] + gap;
        for sample in 0..=16 {
            let transverse = low + (high - low) * f64::from(sample) / 16.0;
            for sign in [-1.0, 1.0] {
                let outside = if sign > 0.0 {
                    aa[axis + 2] - bb[axis] + gap
                } else {
                    aa[axis] - bb[axis + 2] - gap
                };
                let inside = (aa[axis] + aa[axis + 2] - bb[axis] - bb[axis + 2]) / 2.0;
                let offset = |v| {
                    let mut t = [0.0; 2];
                    t[axis] = v;
                    t[other] = transverse;
                    t
                };
                let clear = |v| separated(a, &transform_points(b, false, 0.0, offset(v)), gap);
                let mut feasible = outside;
                for step in 1..=20 {
                    let trial = outside + (inside - outside) * f64::from(step) / 20.0;
                    if clear(trial) {
                        feasible = trial;
                    } else {
                        let mut blocked = trial;
                        for _ in 0..18 {
                            let mid = (feasible + blocked) / 2.0;
                            if clear(mid) {
                                feasible = mid;
                            } else {
                                blocked = mid;
                            }
                        }
                        break;
                    }
                }
                if !visit(offset(feasible)) {
                    return;
                }
            }
        }
    }
}
