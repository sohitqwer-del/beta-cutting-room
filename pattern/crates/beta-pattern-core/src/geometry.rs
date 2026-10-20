//! Bounded curve sampling and planar operations. Invalid topology returns an error.
use crate::Corner;
use anyhow::{Result, ensure};
use beta_marker_core::Pt;
pub use beta_marker_core::geometry::{area, bbox, signed_area};
const EPS: f64 = 1e-9;
pub(crate) fn add(a: Pt, b: Pt) -> Pt {
    [a[0] + b[0], a[1] + b[1]]
}
pub(crate) fn sub(a: Pt, b: Pt) -> Pt {
    [a[0] - b[0], a[1] - b[1]]
}
pub(crate) fn mul(a: Pt, s: f64) -> Pt {
    [a[0] * s, a[1] * s]
}
pub(crate) fn dot(a: Pt, b: Pt) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
pub(crate) fn cross(a: Pt, b: Pt) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
/// Euclidean distance in millimetres.
pub fn distance(a: Pt, b: Pt) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}
pub(crate) fn unit(v: Pt) -> Result<Pt> {
    let n = distance(v, [0., 0.]);
    ensure!(n.is_finite() && n > EPS, "zero-length or invalid direction");
    Ok(mul(v, 1. / n))
}
fn finite(p: &[Pt]) -> Result<()> {
    ensure!(
        p.iter().flatten().all(|v| v.is_finite()),
        "non-finite coordinate"
    );
    Ok(())
}
/// Intersection of infinite lines; parallel or coincident lines return None.
pub fn line_intersection(a: Pt, b: Pt, c: Pt, d: Pt) -> Option<Pt> {
    let (r, s) = (sub(b, a), sub(d, c));
    let den = cross(r, s);
    if den.abs() <= EPS {
        None
    } else {
        Some(add(a, mul(r, cross(sub(c, a), s) / den)))
    }
}
/// Intersections of two polylines, including touching and collinear endpoints, in a traversal order.
pub fn intersections(a: &[Pt], b: &[Pt]) -> Vec<Pt> {
    let mut out = vec![];
    for ab in a.windows(2) {
        let mut local = vec![];
        for cd in b.windows(2) {
            if let Some(p) = line_intersection(ab[0], ab[1], cd[0], cd[1]) {
                if on(p, ab[0], ab[1]) && on(p, cd[0], cd[1]) {
                    local.push(p);
                }
            } else {
                for p in [ab[0], ab[1], cd[0], cd[1]] {
                    if on(p, ab[0], ab[1]) && on(p, cd[0], cd[1]) {
                        local.push(p);
                    }
                }
            }
        }
        local.sort_by(|p, q| distance(*p, ab[0]).total_cmp(&distance(*q, ab[0])));
        for p in local {
            if !out.iter().any(|q| distance(p, *q) < EPS) {
                out.push(p);
            }
        }
    }
    out
}
fn on(p: Pt, a: Pt, b: Pt) -> bool {
    cross(sub(p, a), sub(b, a)).abs() <= EPS * (1. + distance(a, b))
        && dot(sub(p, a), sub(p, b)) <= EPS
}
fn segments_touch(a: Pt, b: Pt, c: Pt, d: Pt) -> bool {
    // Most boundary segments have disjoint bounds. Avoid allocating intersection
    // vectors or calculating an infinite-line crossing for those pairs.
    if a[0].max(b[0]) + EPS < c[0].min(d[0])
        || c[0].max(d[0]) + EPS < a[0].min(b[0])
        || a[1].max(b[1]) + EPS < c[1].min(d[1])
        || c[1].max(d[1]) + EPS < a[1].min(b[1])
    {
        return false;
    }
    if let Some(p) = line_intersection(a, b, c, d) {
        on(p, a, b) && on(p, c, d)
    } else {
        [a, b, c, d].into_iter().any(|p| on(p, a, b) && on(p, c, d))
    }
}
/// Whether a polygon has crossing/touching non-adjacent edges or degenerate edges.
pub fn self_intersects(poly: &[Pt]) -> bool {
    let n = poly.len();
    if n < 3 {
        return true;
    }
    for i in 0..n {
        if distance(poly[i], poly[(i + 1) % n]) < EPS {
            return true;
        }
        for j in i + 1..n {
            if j == i + 1 || (i == 0 && j == n - 1) {
                continue;
            }
            if segments_touch(poly[i], poly[(i + 1) % n], poly[j], poly[(j + 1) % n]) {
                return true;
            }
        }
    }
    false
}
/// Validate a simple, nonzero-area polygon.
pub fn validate_polygon(poly: &[Pt]) -> Result<()> {
    finite(poly)?;
    ensure!(poly.len() >= 3 && area(poly) > EPS, "polygon has zero area");
    ensure!(
        !self_intersects(poly),
        "polygon self-intersects or has repeated vertices"
    );
    Ok(())
}
/// Length of an open polyline.
pub fn length(p: &[Pt]) -> f64 {
    p.windows(2).map(|w| distance(w[0], w[1])).sum()
}
/// Point and unit tangent at a distance along an open polyline.
pub fn at_distance(p: &[Pt], mut d: f64) -> Result<(Pt, Pt)> {
    finite(p)?;
    ensure!(
        d.is_finite() && d >= 0. && d <= length(p) + EPS,
        "distance is outside seam"
    );
    for w in p.windows(2) {
        let n = distance(w[0], w[1]);
        if n <= EPS {
            continue;
        }
        if d <= n + EPS {
            let t = unit(sub(w[1], w[0]))?;
            return Ok((add(w[0], mul(t, d.min(n))), t));
        }
        d -= n;
    }
    anyhow::bail!("seam has no usable segment")
}
/// Flatten cubic Bezier using a control-hull distance bound; at most 65,536 segments.
pub fn flatten_cubic(p: [Pt; 4], tolerance: f64) -> Result<Vec<Pt>> {
    finite(&p)?;
    ensure!(
        tolerance.is_finite() && tolerance > 0.,
        "tolerance must be positive"
    );
    fn recur(p: [Pt; 4], t: f64, depth: u8, out: &mut Vec<Pt>) -> Result<()> {
        let chord = sub(p[3], p[0]);
        let n = dot(chord, chord);
        let deviation = |q: Pt| {
            let u = if n > EPS {
                (dot(sub(q, p[0]), chord) / n).clamp(0., 1.)
            } else {
                0.
            };
            distance(q, add(p[0], mul(chord, u)))
        };
        if deviation(p[1]).max(deviation(p[2])) <= t {
            ensure!(out.len() < 65537, "curve sample limit exceeded");
            out.push(p[3]);
            return Ok(());
        }
        ensure!(depth < 24, "curve did not converge");
        let a = mul(add(p[0], p[1]), 0.5);
        let b = mul(add(p[1], p[2]), 0.5);
        let c = mul(add(p[2], p[3]), 0.5);
        let d = mul(add(a, b), 0.5);
        let e = mul(add(b, c), 0.5);
        let f = mul(add(d, e), 0.5);
        recur([p[0], a, d, f], t, depth + 1, out)?;
        recur([f, e, c, p[3]], t, depth + 1, out)
    }
    let mut out = vec![p[0]];
    recur(p, tolerance, 0, &mut out)?;
    Ok(out)
}
/// Flatten a circular arc with sagitta at most tolerance.
pub fn flatten_arc(center: Pt, r: f64, start: f64, sweep: f64, t: f64) -> Result<Vec<Pt>> {
    finite(&[center])?;
    ensure!(
        [r, start, sweep, t].iter().all(|v| v.is_finite())
            && r > 0.
            && t > 0.
            && sweep.abs() > EPS
            && sweep.abs() <= 360.,
        "invalid arc or tolerance"
    );
    let step = 2. * (1. - (t / r).min(1.)).acos();
    let n = (sweep.to_radians().abs() / step).ceil().max(1.);
    ensure!(n <= 65536., "arc sample limit exceeded");
    let n = n as usize;
    Ok((0..=n)
        .map(|i| {
            let a = (start + sweep * i as f64 / n as f64).to_radians();
            add(center, [r * a.cos(), r * a.sin()])
        })
        .collect())
}
/// Flatten an interpolating Hermite spline. Empty tangents use centred finite differences.
pub fn flatten_spline(p: &[Pt], tangents: &[Pt], t: f64) -> Result<Vec<Pt>> {
    finite(p)?;
    finite(tangents)?;
    ensure!(
        p.len() >= 2 && (tangents.is_empty() || tangents.len() == p.len()),
        "spline needs >=2 points and one tangent per point"
    );
    let auto: Vec<_> = (0..p.len())
        .map(|i| {
            if i == 0 {
                sub(p[1], p[0])
            } else if i + 1 == p.len() {
                sub(p[i], p[i - 1])
            } else {
                mul(sub(p[i + 1], p[i - 1]), 0.5)
            }
        })
        .collect();
    let ts = if tangents.is_empty() { &auto } else { tangents };
    let mut out = vec![p[0]];
    for i in 0..p.len() - 1 {
        out.extend(
            flatten_cubic(
                [
                    p[i],
                    add(p[i], mul(ts[i], 1. / 3.)),
                    sub(p[i + 1], mul(ts[i + 1], 1. / 3.)),
                    p[i + 1],
                ],
                t,
            )?
            .into_iter()
            .skip(1),
        );
        ensure!(out.len() <= 65537, "spline sample limit exceeded");
    }
    Ok(out)
}
/// Variable outward polygon offset. Rejects self-crossing results; does not union or trim loops.
pub fn offset(poly: &[Pt], widths: &[f64], corners: &[Corner]) -> Result<Vec<Pt>> {
    validate_polygon(poly)?;
    let n = poly.len();
    ensure!(
        widths.len() == n && corners.len() == n,
        "one allowance and corner required per segment"
    );
    ensure!(
        widths.iter().all(|w| w.is_finite() && *w >= 0.),
        "allowances must be finite and nonnegative"
    );
    let sign = signed_area(poly).signum();
    let mut out = vec![];
    for i in 0..n {
        let prev = (i + n - 1) % n;
        let v = poly[i];
        let u = unit(sub(v, poly[prev]))?;
        let w = unit(sub(poly[(i + 1) % n], v))?;
        let a = add(v, mul([u[1], -u[0]], sign * widths[prev]));
        let b = add(v, mul([w[1], -w[0]], sign * widths[i]));
        if distance(a, b) < EPS {
            out.push(a);
            continue;
        }
        let (incoming, outgoing) = if corners[i] == Corner::Mirrored {
            if widths[i] >= widths[prev] {
                (sub(u, mul(w, 2. * dot(u, w))), w)
            } else {
                (u, sub(w, mul(u, 2. * dot(u, w))))
            }
        } else {
            (u, w)
        };
        let tip = line_intersection(a, add(a, incoming), b, add(b, outgoing));
        match (corners[i], tip) {
            (Corner::Miter | Corner::Mirrored, Some(p))
                if distance(p, v) <= 10. * widths[prev].max(widths[i]) + EPS =>
            {
                out.push(p)
            }
            (Corner::Square, Some(p)) => {
                let limit = widths[prev].max(widths[i]);
                let bis = unit(sub(p, v))?;
                let cut = add(v, mul(bis, limit));
                let direction = [-bis[1], bis[0]];
                if let (Some(c), Some(d)) = (
                    line_intersection(a, add(a, u), cut, add(cut, direction)),
                    line_intersection(b, add(b, w), cut, add(cut, direction)),
                ) {
                    out.extend([c, d]);
                } else {
                    out.extend([a, b]);
                }
            }
            _ => out.extend([a, b]),
        }
    }
    out.dedup_by(|a, b| distance(*a, *b) < EPS);
    if out.len() > 1 && distance(out[0], out[out.len() - 1]) < EPS {
        out.pop();
    }
    validate_polygon(&out)?;
    Ok(out)
}

/// Exact circle intersections. First solution is left of A-to-B; tangency is unique.
pub fn circle_intersections(a: Pt, ra: f64, b: Pt, rb: f64) -> Result<Vec<Pt>> {
    finite(&[a, b])?;
    ensure!(
        ra.is_finite() && rb.is_finite() && ra > 0. && rb > 0.,
        "circle radii must be positive"
    );
    let d = distance(a, b);
    ensure!(d > EPS, "concentric circles have no unique intersection");
    if d > ra + rb + EPS || d < (ra - rb).abs() - EPS {
        return Ok(vec![]);
    }
    let x = (ra * ra - rb * rb + d * d) / (2. * d);
    let h = (ra * ra - x * x).max(0.).sqrt();
    let u = mul(sub(b, a), 1. / d);
    let p = add(a, mul(u, x));
    let v = mul([-u[1], u[0]], h);
    if h < EPS {
        Ok(vec![p])
    } else {
        Ok(vec![add(p, v), sub(p, v)])
    }
}
/// Exact intersection of a circle with an infinite line, in line order.
pub fn circle_line(center: Pt, radius: f64, a: Pt, b: Pt) -> Result<Vec<Pt>> {
    finite(&[center, a, b])?;
    ensure!(
        radius.is_finite() && radius > 0.,
        "circle radius must be positive"
    );
    let u = unit(sub(b, a))?;
    let t = dot(sub(center, a), u);
    let foot = add(a, mul(u, t));
    let h2 = radius * radius - dot(sub(foot, center), sub(foot, center));
    if h2 < -EPS {
        return Ok(vec![]);
    }
    let h = h2.max(0.).sqrt();
    if h < EPS {
        Ok(vec![foot])
    } else {
        Ok(vec![sub(foot, mul(u, h)), add(foot, mul(u, h))])
    }
}
/// Tangent contact points from an external point; counter-clockwise contact first.
pub fn circle_tangents(point: Pt, center: Pt, radius: f64) -> Result<Vec<Pt>> {
    finite(&[point, center])?;
    ensure!(
        radius.is_finite() && radius > 0.,
        "circle radius must be positive"
    );
    let v = sub(point, center);
    let d = distance(point, center);
    ensure!(d >= radius - EPS, "point lies inside circle");
    let u = unit(v)?;
    let x = radius * radius / d;
    let h = (radius * radius - x * x).max(0.).sqrt();
    let p = add(center, mul(u, x));
    let n = mul([-u[1], u[0]], h);
    if h < EPS {
        Ok(vec![p])
    } else {
        Ok(vec![add(p, n), sub(p, n)])
    }
}
/// Whether an angle belongs to a directed circular arc, including endpoints.
pub fn angle_in_arc(angle: f64, start: f64, sweep: f64) -> bool {
    sweep.abs() >= 360. - 1e-9
        || if sweep >= 0. {
            (angle - start).rem_euclid(360.) <= sweep + 1e-9
                || (angle - start).rem_euclid(360.) >= 360. - 1e-9
        } else {
            (start - angle).rem_euclid(360.) <= -sweep + 1e-9
                || (start - angle).rem_euclid(360.) >= 360. - 1e-9
        }
}
/// Intersect a sampled path with an infinite axis, in path traversal order.
pub fn path_axis(path: &[Pt], origin: Pt, angle: f64) -> Result<Vec<Pt>> {
    finite(path)?;
    finite(&[origin])?;
    ensure!(angle.is_finite(), "non-finite axis angle");
    let r = angle.to_radians();
    let toward = add(origin, [r.cos(), r.sin()]);
    let mut out = vec![];
    for w in path.windows(2) {
        if let Some(p) = line_intersection(w[0], w[1], origin, toward)
            && on(p, w[0], w[1])
            && !out.iter().any(|q| distance(*q, p) < EPS)
        {
            out.push(p);
        }
    }
    Ok(out)
}
/// Extract a positive-length interval from a sampled path, preserving interior vertices.
pub fn cut_path(path: &[Pt], start: f64, interval: f64) -> Result<Vec<Pt>> {
    ensure!(
        interval.is_finite() && interval > 0.,
        "cut interval must be positive"
    );
    let a = at_distance(path, start)?.0;
    let b = at_distance(path, start + interval)?.0;
    let mut out = vec![a];
    let mut d = 0.;
    for w in path.windows(2) {
        d += distance(w[0], w[1]);
        if d > start + EPS && d < start + interval - EPS {
            out.push(w[1]);
        }
    }
    out.push(b);
    Ok(out)
}
/// Flatten a rotated ellipse with a conservative second-derivative chord error bound.
pub fn flatten_ellipse(
    center: Pt,
    radii: Pt,
    rotation: f64,
    start: f64,
    sweep: f64,
    tolerance: f64,
) -> Result<Vec<Pt>> {
    finite(&[center, radii])?;
    ensure!(
        radii.iter().all(|r| *r > 0.)
            && [rotation, start, sweep, tolerance]
                .iter()
                .all(|n| n.is_finite())
            && sweep.abs() <= 360.
            && sweep.abs() > EPS
            && tolerance > 0.,
        "invalid ellipse parameters"
    );
    let step = (8. * tolerance / radii[0].max(radii[1]))
        .sqrt()
        .min(std::f64::consts::FRAC_PI_2);
    let n = (sweep.to_radians().abs() / step).ceil().max(1.);
    ensure!(n <= 65536., "ellipse sample limit exceeded");
    let n = n as usize;
    let (sr, cr) = rotation.to_radians().sin_cos();
    Ok((0..=n)
        .map(|i| {
            let t = (start + sweep * i as f64 / n as f64).to_radians();
            let x = radii[0] * t.cos();
            let y = radii[1] * t.sin();
            add(center, [cr * x - sr * y, sr * x + cr * y])
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn curves_and_intersections() {
        let p = flatten_cubic([[0., 0.], [1., 0.], [2., 0.], [3., 0.]], 0.01).unwrap();
        assert_eq!(p, vec![[0., 0.], [3., 0.]]);
        assert_eq!(length(&p), 3.);
        let a = flatten_arc([0., 0.], 10., 0., 90., 0.001).unwrap();
        assert!((length(&a) - 5. * std::f64::consts::PI).abs() < 0.002);
        assert_eq!(
            intersections(&[[0., 0.], [10., 10.]], &[[0., 10.], [10., 0.]]),
            vec![[5., 5.]]
        );
        assert_eq!(at_distance(&p, 1.).unwrap().0, [1., 0.]);
        assert!(at_distance(&p, -1.).is_err());
        let s = flatten_spline(&[[0., 0.], [3., 0.], [6., 0.]], &[], 0.01).unwrap();
        assert_eq!(length(&s), 6.);
        assert!(line_intersection([0., 0.], [1., 0.], [0., 1.], [1., 1.]).is_none());
    }
    #[test]
    fn corners_and_area() {
        let p = [[0., 0.], [10., 0.], [10., 10.], [0., 10.]];
        assert_eq!(area(&p), 100.);
        for (c, expected) in [
            (Corner::Miter, 144.),
            (Corner::Mirrored, 144.),
            (Corner::Bevel, 142.),
            (Corner::Square, 100. + 40. + 4. * (2. * 2f64.sqrt() - 2.)),
        ] {
            let o = offset(&p, &[1.; 4], &[c; 4]).unwrap();
            assert!((area(&o) - expected).abs() < 1e-8, "{c:?}: {}", area(&o));
        }
        let cw: Vec<_> = p.into_iter().rev().collect();
        assert_eq!(
            area(&offset(&cw, &[1.; 4], &[Corner::Miter; 4]).unwrap()),
            144.
        );
        assert!(self_intersects(&[
            [0., 0.],
            [10., 10.],
            [0., 10.],
            [10., 0.]
        ]));
        assert!(offset(&p, &[-1.; 4], &[Corner::Miter; 4]).is_err());
    }
    #[test]
    fn asymmetric_hem_corners_are_mirrored_on_both_sides() {
        let p = [[0., 0.], [100., 0.], [80., 100.], [20., 100.]];
        let m = offset(&p, &[20., 10., 10., 10.], &[Corner::Miter; 4]).unwrap();
        let h = offset(
            &p,
            &[20., 10., 10., 10.],
            &[
                Corner::Mirrored,
                Corner::Mirrored,
                Corner::Miter,
                Corner::Miter,
            ],
        )
        .unwrap();
        assert!((h[0][0] - (4. - 9.6 / 1.04f64.sqrt())).abs() < 1e-9);
        assert!((h[1][0] - (100. - h[0][0])).abs() < 1e-9);
        assert!((h[0][1] + 20.).abs() < 1e-9);
        assert!((h[1][1] + 20.).abs() < 1e-9);
        assert!(distance(m[0], h[0]) > 1.);
        let v = offset(
            &[[0., 0.], [10., 0.], [10., 10.], [0., 10.]],
            &[1., 2., 3., 4.],
            &[Corner::Miter; 4],
        )
        .unwrap();
        assert_eq!(v, vec![[-4., -1.], [12., -1.], [12., 13.], [-4., 13.]]);
        assert_eq!(area(&v), 224.);
    }
}
