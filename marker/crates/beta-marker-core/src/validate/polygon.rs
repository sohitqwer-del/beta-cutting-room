//! Polygon predicates independent of the nesting engine. No polygon offsets or
//! sampling grid are used: tests operate on the original line segments.
use crate::geometry::{bbox, signed_area};
use crate::model::Pt;

const EPS: f64 = 1e-9;
fn cross(a: Pt, b: Pt, c: Pt) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
fn edges(p: &[Pt]) -> impl Iterator<Item = (Pt, Pt)> + '_ {
    p.iter()
        .copied()
        .zip(p.iter().copied().cycle().skip(1))
        .take(p.len())
}
fn parameter(p: Pt, a: Pt, b: Pt) -> Option<f64> {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len = dx.hypot(dy);
    if len <= EPS || cross(a, b, p).abs() > EPS * len {
        return None;
    }
    let t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (len * len);
    if (-EPS..=1.0 + EPS).contains(&t) {
        Some(t.clamp(0.0, 1.0))
    } else {
        None
    }
}
fn inside(p: Pt, poly: &[Pt]) -> bool {
    let mut odd = false;
    for (a, b) in edges(poly) {
        if parameter(p, a, b).is_some() {
            return false;
        }
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            odd = !odd;
        }
    }
    odd
}
fn point_distance(p: Pt, a: Pt, b: Pt) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let l2 = dx * dx + dy * dy;
    let t = if l2 == 0.0 {
        0.0
    } else {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0)
    };
    (p[0] - a[0] - t * dx).hypot(p[1] - a[1] - t * dy)
}
// A boundary segment can enter at a vertex of the other polygon. Split at
// every such vertex before testing interval interiors (handles T junctions).
fn boundary_inside(a: &[Pt], b: &[Pt]) -> bool {
    for (p, q) in edges(a) {
        if inside(p, b) {
            return true;
        }
        if (p[0] - q[0]).hypot(p[1] - q[1]) <= EPS {
            continue;
        }
        let mut cuts = vec![0.0, 1.0];
        cuts.extend(b.iter().filter_map(|&v| parameter(v, p, q)));
        cuts.sort_by(f64::total_cmp);
        for pair in cuts.windows(2) {
            if pair[1] - pair[0] <= EPS {
                continue;
            }
            let t = (pair[0] + pair[1]) / 2.0;
            if inside([p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])], b) {
                return true;
            }
        }
    }
    false
}

pub(super) fn relation(a: &[Pt], b: &[Pt], required: f64) -> (bool, f64) {
    let wa = signed_area(a).signum();
    let wb = signed_area(b).signum();
    let mut distance = f64::INFINITY;
    for (p, q) in edges(a) {
        for (r, s) in edges(b) {
            let dx = (p[0].min(q[0]) - r[0].max(s[0]))
                .max(r[0].min(s[0]) - p[0].max(q[0]))
                .max(0.0);
            let dy = (p[1].min(q[1]) - r[1].max(s[1]))
                .max(r[1].min(s[1]) - p[1].max(q[1]))
                .max(0.0);
            // A separated segment box cannot cross or reduce a relevant gap.
            if dx > required.max(0.0) || dy > required.max(0.0) {
                continue;
            }
            let lp = (q[0] - p[0]).hypot(q[1] - p[1]);
            let lr = (s[0] - r[0]).hypot(s[1] - r[1]);
            let c1 = cross(p, q, r);
            let c2 = cross(p, q, s);
            let c3 = cross(r, s, p);
            let c4 = cross(r, s, q);
            let opposite = |u: f64, v: f64, tolerance: f64| {
                (u > tolerance && v < -tolerance) || (u < -tolerance && v > tolerance)
            };
            if opposite(c1, c2, EPS * lp) && opposite(c3, c4, EPS * lr) {
                return (true, 0.0);
            }
            if lp > EPS && lr > EPS && c1.abs() <= EPS * lp && c2.abs() <= EPS * lp {
                let dot = (q[0] - p[0]) * (s[0] - r[0]) + (q[1] - p[1]) * (s[1] - r[1]);
                let t1 =
                    ((r[0] - p[0]) * (q[0] - p[0]) + (r[1] - p[1]) * (q[1] - p[1])) / lp.powi(2);
                let t2 =
                    ((s[0] - p[0]) * (q[0] - p[0]) + (s[1] - p[1]) * (q[1] - p[1])) / lp.powi(2);
                if t1.max(t2).min(1.0) - t1.min(t2).max(0.0) > EPS && dot * wa * wb > 0.0 {
                    return (true, 0.0);
                }
            }
            // Distance work can stop at zero, but crossing checks must continue.
            if distance > 0.0 && dx.hypot(dy) <= distance.min(required.max(0.0)) {
                distance = distance
                    .min(point_distance(p, r, s))
                    .min(point_distance(q, r, s))
                    .min(point_distance(r, p, q))
                    .min(point_distance(s, p, q));
            }
        }
    }
    let boxes_intersect = bbox(a)
        .zip(bbox(b))
        .is_some_and(|(a, b)| a[0] <= b[2] && b[0] <= a[2] && a[1] <= b[3] && b[1] <= a[3]);
    (
        boxes_intersect && (boundary_inside(a, b) || boundary_inside(b, a)),
        distance,
    )
}
