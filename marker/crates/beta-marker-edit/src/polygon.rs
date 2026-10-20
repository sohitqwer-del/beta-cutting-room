//! Polygon contact events, Euclidean clearance and intersection area.
//! Operates on simple polygons. No bounding box is used as a collision shape.
use beta_marker_core::{
    Pt,
    geometry::{area, signed_area},
};
const EPS: f64 = 1e-8;
pub(crate) fn simple(poly: &[Pt]) -> bool {
    let mut p = poly.to_vec();
    p.dedup_by(|a, b| sub(*a, *b)[0].abs() < EPS && sub(*a, *b)[1].abs() < EPS);
    if p.len() > 1 && sub(p[0], p[p.len() - 1]).iter().all(|x| x.abs() < EPS) {
        p.pop();
    }
    if p.len() < 3
        || p.iter().flatten().any(|x| !x.is_finite())
        || !area(&p).is_finite()
        || area(&p) < EPS
    {
        return false;
    }
    for (i, (a, b)) in edges(&p).enumerate() {
        for (j, (c, d)) in edges(&p).enumerate().skip(i + 1) {
            if j == i + 1 || (i == 0 && j == p.len() - 1) {
                continue;
            }
            let ab = sub(b, a);
            let cd = sub(d, c);
            let det = cross(ab, cd);
            if det.abs() > EPS {
                let t = cross(sub(c, a), cd) / det;
                let u = cross(sub(c, a), ab) / det;
                if (-EPS..=1. + EPS).contains(&t) && (-EPS..=1. + EPS).contains(&u) {
                    return false;
                }
            } else if point_distance(a, c, d) < EPS
                || point_distance(b, c, d) < EPS
                || point_distance(c, a, b) < EPS
                || point_distance(d, a, b) < EPS
            {
                return false;
            }
        }
    }
    true
}
pub(crate) fn sub(a: Pt, b: Pt) -> Pt {
    [a[0] - b[0], a[1] - b[1]]
}
fn dot(a: Pt, b: Pt) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn cross(a: Pt, b: Pt) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn edges(p: &[Pt]) -> impl Iterator<Item = (Pt, Pt)> + '_ {
    p.iter()
        .enumerate()
        .map(|(i, &a)| (a, p[(i + 1) % p.len()]))
}
fn point_distance(p: Pt, a: Pt, b: Pt) -> f64 {
    let v = sub(b, a);
    let t = (dot(sub(p, a), v) / dot(v, v).max(EPS * EPS)).clamp(0.0, 1.0);
    (p[0] - a[0] - v[0] * t).hypot(p[1] - a[1] - v[1] * t)
}
fn inside(p: Pt, poly: &[Pt]) -> bool {
    let mut yes = false;
    for (a, b) in edges(poly) {
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            yes = !yes;
        }
    }
    yes
}
pub(crate) fn distance(a: &[Pt], b: &[Pt]) -> f64 {
    let mut d = f64::INFINITY;
    for (p, q) in edges(a) {
        for (r, s) in edges(b) {
            let v = sub(q, p);
            let w = sub(s, r);
            let det = cross(v, w);
            if det.abs() > EPS {
                let t = cross(sub(r, p), w) / det;
                let u = cross(sub(r, p), v) / det;
                if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
                    return 0.0;
                }
            }
            d = d
                .min(point_distance(p, r, s))
                .min(point_distance(q, r, s))
                .min(point_distance(r, p, q))
                .min(point_distance(s, p, q));
        }
    }
    if a.first().is_some_and(|p| inside(*p, b)) || b.first().is_some_and(|p| inside(*p, a)) {
        0.0
    } else {
        d
    }
}
fn triangles(poly: &[Pt]) -> Vec<Vec<Pt>> {
    let mut p = poly.to_vec();
    if signed_area(&p) < 0.0 {
        p.reverse();
    }
    // Remove duplicate and collinear vertices before ear clipping.
    loop {
        let n = p.len();
        if n <= 3 {
            break;
        }
        let remove = (0..n)
            .find(|&i| cross(sub(p[i], p[(i + n - 1) % n]), sub(p[(i + 1) % n], p[i])).abs() < EPS);
        if let Some(i) = remove {
            p.remove(i);
        } else {
            break;
        }
    }
    let mut out = vec![];
    while p.len() > 3 {
        let n = p.len();
        let ear = (0..n).find(|&i| {
            let a = p[(i + n - 1) % n];
            let b = p[i];
            let c = p[(i + 1) % n];
            cross(sub(b, a), sub(c, b)) > EPS
                && !(0..n).any(|j| {
                    j != i
                        && j != (i + n - 1) % n
                        && j != (i + 1) % n
                        && cross(sub(b, a), sub(p[j], a)) >= -EPS
                        && cross(sub(c, b), sub(p[j], b)) >= -EPS
                        && cross(sub(a, c), sub(p[j], c)) >= -EPS
                })
        });
        let Some(i) = ear else {
            break;
        };
        out.push(vec![p[(i + n - 1) % n], p[i], p[(i + 1) % n]]);
        p.remove(i);
    }
    if p.len() == 3 {
        out.push(p);
    }
    out
}
fn clip(poly: &[Pt], a: Pt, b: Pt) -> Vec<Pt> {
    let mut out = vec![];
    for (p, q) in edges(poly) {
        let sp = cross(sub(b, a), sub(p, a));
        let sq = cross(sub(b, a), sub(q, a));
        if sp >= 0.0 {
            out.push(p);
        }
        if (sp >= 0.0) != (sq >= 0.0) {
            let t = sp / (sp - sq);
            out.push([p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])]);
        }
    }
    out
}
pub(crate) fn overlap_area(a: &[Pt], b: &[Pt]) -> f64 {
    let ta = triangles(a);
    let tb = triangles(b);
    let mut total = 0.0;
    for a in &ta {
        for b in &tb {
            let mut p = a.clone();
            for (r, s) in edges(b) {
                p = clip(&p, r, s);
                if p.is_empty() {
                    break;
                }
            }
            total += area(&p);
        }
    }
    total
}
pub(crate) fn conflict(a: &[Pt], b: &[Pt], gap: f64) -> bool {
    let d = distance(a, b);
    if gap > EPS && d < gap - EPS {
        return true;
    }
    d < EPS && overlap_area(a, b) > EPS
}
// Add every time a moving vertex enters/leaves a segment's radius capsule.
fn capsule_events(p: Pt, v: Pt, a: Pt, b: Pt, r: f64, out: &mut Vec<f64>) {
    for q in [a, b] {
        let w = sub(p, q);
        let bv = dot(w, v);
        let disc = bv * bv - dot(v, v) * (dot(w, w) - r * r);
        if disc >= 0.0 {
            for t in [
                (-bv - disc.sqrt()) / dot(v, v),
                (-bv + disc.sqrt()) / dot(v, v),
            ] {
                if t.is_finite() {
                    out.push(t);
                }
            }
        }
    }
    let e = sub(b, a);
    let len = dot(e, e).sqrt();
    let cv = cross(e, v);
    if cv.abs() > EPS && len > EPS {
        for offset in [-r * len, r * len] {
            let t = (offset - cross(e, sub(p, a))) / cv;
            let q = [p[0] + t * v[0], p[1] + t * v[1]];
            let u = dot(sub(q, a), e) / (len * len);
            if (-EPS..=1.0 + EPS).contains(&u) {
                out.push(t);
            }
        }
    }
}
pub(crate) fn events(a: &[Pt], b: &[Pt], v: Pt, gap: f64, out: &mut Vec<f64>) {
    for &p in a {
        for (r, s) in edges(b) {
            capsule_events(p, v, r, s, gap, out);
        }
    }
    for &p in b {
        for (r, s) in edges(a) {
            capsule_events(p, [-v[0], -v[1]], r, s, gap, out);
        }
    }
}
pub(crate) fn shifted(a: &[Pt], v: Pt, t: f64) -> Vec<Pt> {
    a.iter()
        .map(|p| [p[0] + v[0] * t, p[1] + v[1] * t])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn concave_overlap_and_touch() {
        let a = vec![[0., 0.], [4., 0.], [4., 1.], [1., 1.], [1., 4.], [0., 4.]];
        let b = vec![[2., 2.], [3., 2.], [3., 3.], [2., 3.]];
        assert_eq!(overlap_area(&a, &b), 0.);
        assert!((distance(&a, &b) - 1.).abs() < EPS);
        assert!((overlap_area(&a, &a) - 7.).abs() < EPS);
        let c = shifted(&b, [-1., -1.], 1.);
        assert!(!conflict(&a, &c, 0.));
        assert!(conflict(&a, &c, 0.1));
        assert!(simple(&a));
        assert!(!simple(&[[0., 0.], [4., 0.], [0., 4.], [4., 4.], [1., 0.]]));
    }
}
