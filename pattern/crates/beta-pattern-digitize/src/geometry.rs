//! Small, deterministic projective-geometry routines, implemented here.
use anyhow::{Result, bail, ensure};
use beta_marker_core::Pt;
use serde::{Deserialize, Serialize};

/// A projective map, stored row-major, with arbitrary coefficient scale.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Homography(pub [f64; 9]);
impl Homography {
    /// Map a point, rejecting the projective horizon.
    pub fn map(&self, p: Pt) -> Option<Pt> {
        let h = self.0;
        let w = h[6] * p[0] + h[7] * p[1] + h[8];
        if !w.is_finite() || w == 0. {
            return None;
        }
        let q = [
            (h[0] * p[0] + h[1] * p[1] + h[2]) / w,
            (h[3] * p[0] + h[4] * p[1] + h[5]) / w,
        ];
        q.iter().all(|v| v.is_finite()).then_some(q)
    }
    /// Invert the map, rejecting singular matrices.
    pub fn inverse(&self) -> Result<Self> {
        let scale = self.0.iter().copied().map(f64::abs).fold(0., f64::max);
        ensure!(scale.is_finite() && scale > 0., "singular homography");
        let a = self.0.map(|v| v / scale);
        let b = [
            a[4] * a[8] - a[5] * a[7],
            a[2] * a[7] - a[1] * a[8],
            a[1] * a[5] - a[2] * a[4],
            a[5] * a[6] - a[3] * a[8],
            a[0] * a[8] - a[2] * a[6],
            a[2] * a[3] - a[0] * a[5],
            a[3] * a[7] - a[4] * a[6],
            a[1] * a[6] - a[0] * a[7],
            a[0] * a[4] - a[1] * a[3],
        ];
        ensure!(
            (a[0] * b[0] + a[1] * b[3] + a[2] * b[6]).abs() > 1e-14,
            "singular homography"
        );
        let scale = if b[8].abs() > 1e-15 {
            b[8]
        } else {
            b.iter().copied().map(f64::abs).fold(0., f64::max)
        };
        Ok(Self(b.map(|v| v / scale)))
    }
}
fn multiply(a: Homography, b: Homography) -> Homography {
    let mut c = [0.; 9];
    for i in 0..3 {
        for j in 0..3 {
            for k in 0..3 {
                c[3 * i + j] += a.0[3 * i + k] * b.0[3 * k + j];
            }
        }
    }
    let scale = if c[8].abs() > 1e-100 { c[8] } else { 1. };
    Homography(c.map(|v| v / scale))
}
fn normalize(points: &[Pt]) -> Result<(Homography, Vec<Pt>)> {
    ensure!(
        !points.is_empty() && points.iter().flatten().all(|x| x.is_finite()),
        "invalid correspondences"
    );
    let n = points.len() as f64;
    let mean = [
        points.iter().map(|p| p[0]).sum::<f64>() / n,
        points.iter().map(|p| p[1]).sum::<f64>() / n,
    ];
    let d = points.iter().map(|p| distance(*p, mean)).sum::<f64>() / n;
    ensure!(d > 1e-10, "coincident calibration points");
    let s = 2_f64.sqrt() / d;
    let h = Homography([s, 0., -s * mean[0], 0., s, -s * mean[1], 0., 0., 1.]);
    Ok((
        h,
        points
            .iter()
            .map(|p| [(p[0] - mean[0]) * s, (p[1] - mean[1]) * s])
            .collect(),
    ))
}
/// Normalized DLT with a fixed final coefficient and pivoted least squares.
/// Pairs must span a plane. No external linear-algebra code is used.
pub fn fit_homography(pairs: &[(Pt, Pt)]) -> Result<Homography> {
    ensure!(
        pairs.len() >= 4,
        "need at least four calibration correspondences"
    );
    let (a, src) = normalize(&pairs.iter().map(|p| p.0).collect::<Vec<_>>())?;
    let (b, dst) = normalize(&pairs.iter().map(|p| p.1).collect::<Vec<_>>())?;
    let mut normal = [[0.; 9]; 8];
    for (s, d) in src.iter().zip(dst) {
        let [x, y] = *s;
        let [u, v] = d;
        for (row, rhs) in [
            ([x, y, 1., 0., 0., 0., -u * x, -u * y], u),
            ([0., 0., 0., x, y, 1., -v * x, -v * y], v),
        ] {
            for i in 0..8 {
                for j in 0..8 {
                    normal[i][j] += row[i] * row[j];
                }
                normal[i][8] += row[i] * rhs;
            }
        }
    }
    for col in 0..8 {
        let pivot = (col..8)
            .max_by(|&i, &j| normal[i][col].abs().total_cmp(&normal[j][col].abs()))
            .unwrap_or(col);
        ensure!(
            normal[pivot][col].abs() > 1e-10,
            "degenerate calibration (collinear points)"
        );
        normal.swap(col, pivot);
        let divisor = normal[col][col];
        for value in &mut normal[col][col..] {
            *value /= divisor;
        }
        let pivot_row = normal[col];
        for (i, row) in normal.iter_mut().enumerate() {
            if i != col {
                let factor = row[col];
                for (value, pivot) in row[col..].iter_mut().zip(&pivot_row[col..]) {
                    *value -= factor * pivot;
                }
            }
        }
    }
    let mut h = [0.; 9];
    for i in 0..8 {
        h[i] = normal[i][8];
    }
    h[8] = 1.;
    Ok(multiply(multiply(b.inverse()?, Homography(h)), a))
}
/// RANSAC with deterministic samples and an inlier least-squares refit.
/// The threshold is in destination units.
pub fn ransac(pairs: &[(Pt, Pt)], threshold: f64) -> Result<(Homography, Vec<usize>)> {
    ensure!(
        pairs.len() >= 4 && threshold.is_finite() && threshold > 0.,
        "invalid RANSAC inputs"
    );
    let mut rng = 0x519da12_u64;
    let mut best = Vec::new();
    let mut best_error = f64::INFINITY;
    for _ in 0..512 {
        let mut ids = Vec::new();
        while ids.len() < 4 {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let i = (rng as usize) % pairs.len();
            if !ids.contains(&i) {
                ids.push(i);
            }
        }
        let sample: Vec<_> = ids.iter().map(|&i| pairs[i]).collect();
        let Ok(h) = fit_homography(&sample) else {
            continue;
        };
        let mut inside = Vec::new();
        let mut error = 0.;
        for (i, (s, d)) in pairs.iter().enumerate() {
            if let Some(q) = h.map(*s) {
                let e = distance(q, *d);
                if e <= threshold {
                    inside.push(i);
                    error += e * e;
                }
            }
        }
        if inside.len() > best.len() || (inside.len() == best.len() && error < best_error) {
            best = inside;
            best_error = error;
        }
    }
    if best.len() < 4 {
        bail!("no consistent calibration plane");
    }
    let mut h = fit_homography(&best.iter().map(|&i| pairs[i]).collect::<Vec<_>>())?;
    for _ in 0..3 {
        best = pairs
            .iter()
            .enumerate()
            .filter_map(|(i, (s, d))| {
                h.map(*s)
                    .filter(|q| distance(*q, *d) <= threshold)
                    .map(|_| i)
            })
            .collect();
        ensure!(best.len() >= 4, "unstable calibration");
        h = fit_homography(&best.iter().map(|&i| pairs[i]).collect::<Vec<_>>())?;
    }
    Ok((h, best))
}
/// Euclidean distance.
pub fn distance(a: Pt, b: Pt) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}
/// Signed polygon area.
pub fn area(p: &[Pt]) -> f64 {
    if p.len() < 3 {
        return 0.;
    }
    (0..p.len())
        .map(|i| {
            let q = p[(i + 1) % p.len()];
            p[i][0] * q[1] - p[i][1] * q[0]
        })
        .sum::<f64>()
        * 0.5
}
/// Distance to a finite segment.
pub fn segment_distance(p: Pt, a: Pt, b: Pt) -> f64 {
    let d = [b[0] - a[0], b[1] - a[1]];
    let n = d[0] * d[0] + d[1] * d[1];
    let t = if n > 0. {
        ((p[0] - a[0]) * d[0] + (p[1] - a[1]) * d[1]) / n
    } else {
        0.
    }
    .clamp(0., 1.);
    distance(p, [a[0] + t * d[0], a[1] + t * d[1]])
}
/// Distance to a closed polygon.
pub fn outline_distance(p: Pt, poly: &[Pt]) -> f64 {
    (0..poly.len())
        .map(|i| segment_distance(p, poly[i], poly[(i + 1) % poly.len()]))
        .fold(f64::INFINITY, f64::min)
}
/// Even-odd point-in-polygon test.
pub fn contains(poly: &[Pt], p: Pt) -> bool {
    let mut yes = false;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            yes = !yes;
        }
    }
    yes
}
/// Closed Douglas-Peucker simplification, preserving sharp corners.
pub fn simplify(poly: &[Pt], tolerance: f64) -> Vec<Pt> {
    if poly.len() < 4 {
        return poly.to_vec();
    }
    let split = (1..poly.len())
        .max_by(|&a, &b| distance(poly[a], poly[0]).total_cmp(&distance(poly[b], poly[0])))
        .unwrap_or(1);
    fn open(p: &[Pt], t: f64, out: &mut Vec<Pt>) {
        let mut stack = vec![(0, p.len() - 1)];
        let mut keep = vec![false; p.len()];
        keep[0] = true;
        keep[p.len() - 1] = true;
        while let Some((a, b)) = stack.pop() {
            let mut best = t;
            let mut k = None;
            for (i, &point) in p.iter().enumerate().take(b).skip(a + 1) {
                let d = segment_distance(point, p[a], p[b]);
                if d > best {
                    best = d;
                    k = Some(i);
                }
            }
            if let Some(i) = k {
                keep[i] = true;
                stack.push((a, i));
                stack.push((i, b));
            }
        }
        out.extend(p.iter().zip(keep).filter_map(|(p, k)| k.then_some(*p)));
    }
    let mut first = Vec::new();
    open(&poly[..=split], tolerance, &mut first);
    first.pop();
    let mut rest = poly[split..].to_vec();
    rest.push(poly[0]);
    open(&rest, tolerance, &mut first);
    first.pop();
    first
}
/// Convex hull with collinear points removed.
pub(crate) fn hull(mut p: Vec<Pt>) -> Vec<Pt> {
    p.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    p.dedup();
    if p.len() < 3 {
        return p;
    }
    fn cross(a: Pt, b: Pt, c: Pt) -> f64 {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    }
    let mut h = Vec::new();
    for &q in &p {
        while h.len() >= 2 && cross(h[h.len() - 2], h[h.len() - 1], q) <= 0. {
            h.pop();
        }
        h.push(q);
    }
    let n = h.len();
    for &q in p.iter().rev().skip(1) {
        while h.len() > n && cross(h[h.len() - 2], h[h.len() - 1], q) <= 0. {
            h.pop();
        }
        h.push(q);
    }
    h.pop();
    h
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projective_fit_rejects_outliers() {
        let h = Homography([2., 0.3, 50., 0.1, 3., 80., 0.0002, 0.0008, 1.]);
        let mut p: Vec<_> = (0..5)
            .flat_map(|y| {
                (0..5).map(move |x| {
                    let q = [x as f64 * 100., y as f64 * 70.];
                    (q, h.map(q).unwrap())
                })
            })
            .collect();
        p[3].1 = [-100., 90.];
        p[17].1 = [2000., 4000.];
        let (fit, ids) = ransac(&p, 0.1).unwrap();
        assert_eq!(ids.len(), 23);
        assert!(distance(fit.map([123., 87.]).unwrap(), h.map([123., 87.]).unwrap()) < 1e-7);
        assert!(fit_homography(&[([0., 0.], [0., 0.]); 4]).is_err());
    }
}
