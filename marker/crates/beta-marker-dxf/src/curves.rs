//! Curve flattening in source coordinates, with a caller-supplied tolerance.

use anyhow::{Result, ensure};
use beta_marker_core::Pt;
use std::f64::consts::{PI, TAU};

pub(crate) const TOLERANCE: f64 = 0.01;
const MAX_SAMPLES: usize = 65_536;

pub(crate) fn distance(a: Pt, b: Pt) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

pub(crate) fn arc(center: Pt, radius: f64, start: f64, sweep: f64, tol: f64) -> Result<Vec<Pt>> {
    ensure!(
        radius > 0.0 && radius.is_finite(),
        "arc/circle radius must be positive"
    );
    let step = if tol >= radius {
        PI / 4.0
    } else {
        (2.0 * (1.0 - tol / radius).acos()).clamp(1e-8, PI / 4.0)
    };
    let n = (sweep.abs() / step).ceil().max(1.0) as usize;
    ensure!(
        n < MAX_SAMPLES,
        "curve exceeds sampling limit; check drawing units or dimensions"
    );
    Ok((0..=n)
        .map(|i| {
            let a = start + sweep * i as f64 / n as f64;
            [center[0] + radius * a.cos(), center[1] + radius * a.sin()]
        })
        .collect())
}

pub(crate) fn polyline(vertices: &[(Pt, f64)], closed: bool, tol: f64) -> Result<Vec<Pt>> {
    ensure!(vertices.len() >= 2, "polyline needs at least two vertices");
    let mut points = vec![vertices[0].0];
    let count = if closed {
        vertices.len()
    } else {
        vertices.len() - 1
    };
    for i in 0..count {
        let (a, bulge) = vertices[i];
        let b = vertices[(i + 1) % vertices.len()].0;
        let chord = distance(a, b);
        if bulge.abs() < 1e-12 || chord < 1e-12 {
            points.push(b);
        } else {
            let sweep = 4.0 * bulge.atan();
            let offset = chord * (1.0 - bulge * bulge) / (4.0 * bulge);
            let center = [
                (a[0] + b[0]) / 2.0 - (b[1] - a[1]) / chord * offset,
                (a[1] + b[1]) / 2.0 + (b[0] - a[0]) / chord * offset,
            ];
            let sampled = arc(
                center,
                distance(a, center),
                (a[1] - center[1]).atan2(a[0] - center[0]),
                sweep,
                tol,
            )?;
            points.extend(sampled.into_iter().skip(1));
            if let Some(last) = points.last_mut() {
                *last = b;
            }
        }
        ensure!(points.len() < 500_000, "polyline exceeds point limit");
    }
    Ok(points)
}

/// Rational or non-rational B-spline, evaluated with homogeneous de Boor.
pub(crate) fn spline(
    controls: &[Pt],
    weights: &[f64],
    knots: &[f64],
    degree: usize,
    tol: f64,
) -> Result<Vec<Pt>> {
    ensure!(
        (1..=16).contains(&degree) && controls.len() > degree,
        "invalid SPLINE degree/control count"
    );
    ensure!(
        knots.len() == controls.len() + degree + 1,
        "SPLINE knot count does not match controls and degree"
    );
    ensure!(
        knots.windows(2).all(|w| w[0] <= w[1]),
        "SPLINE knots must be nondecreasing"
    );
    ensure!(
        weights.is_empty() || weights.len() == controls.len(),
        "SPLINE weight count does not match controls"
    );
    ensure!(
        weights.iter().all(|&w| w > 0.0),
        "SPLINE weights must be positive"
    );
    let eval = |u: f64, span: usize| -> Result<Pt> {
        let mut d = Vec::with_capacity(degree + 1);
        for j in 0..=degree {
            let i = span - degree + j;
            let w = weights.get(i).copied().unwrap_or(1.0);
            d.push([controls[i][0] * w, controls[i][1] * w, w]);
        }
        for r in 1..=degree {
            for j in (r..=degree).rev() {
                let i = span - degree + j;
                let denom = knots[i + degree + 1 - r] - knots[i];
                let alpha = if denom.abs() < 1e-15 {
                    0.0
                } else {
                    (u - knots[i]) / denom
                };
                let previous = d[j - 1];
                for (value, old) in d[j].iter_mut().zip(previous) {
                    *value = (1.0 - alpha) * old + alpha * *value;
                }
            }
        }
        ensure!(
            d[degree][2].abs() > 1e-15,
            "SPLINE has zero homogeneous weight"
        );
        Ok([d[degree][0] / d[degree][2], d[degree][1] / d[degree][2]])
    };
    let mut result = vec![];
    for span in degree..controls.len() {
        let (a, b) = (knots[span], knots[span + 1]);
        if b <= a {
            continue;
        }
        let f = |u| eval(u, span);
        if result.is_empty() {
            result.push(f(a)?);
        }
        // Seed more than once per control degree, so a high-degree oscillation
        // cannot be hidden by one set of quarter-point probes.
        let seeds = degree * 2;
        for j in 0..seeds {
            let left = a + (b - a) * j as f64 / seeds as f64;
            let right = a + (b - a) * (j + 1) as f64 / seeds as f64;
            subdivide(
                &f,
                (left, right),
                (f(left)?, f(right)?),
                tol,
                0,
                &mut result,
            )?;
        }
    }
    ensure!(result.len() >= 2, "SPLINE has an empty parameter domain");
    Ok(result)
}

fn subdivide(
    f: &impl Fn(f64) -> Result<Pt>,
    domain: (f64, f64),
    ends: (Pt, Pt),
    tol: f64,
    depth: u32,
    out: &mut Vec<Pt>,
) -> Result<()> {
    ensure!(out.len() < MAX_SAMPLES, "SPLINE exceeds sampling limit");
    let (a, b) = domain;
    let (p, q) = ends;
    let mut error: f64 = 0.0;
    for fraction in [0.25, 0.5, 0.75] {
        let s = f(a + (b - a) * fraction)?;
        let chord = [
            p[0] + (q[0] - p[0]) * fraction,
            p[1] + (q[1] - p[1]) * fraction,
        ];
        error = error.max(distance(s, chord));
    }
    if error <= tol {
        out.push(q);
        return Ok(());
    }
    ensure!(depth < 20, "SPLINE did not converge at 0.01 mm tolerance");
    let mid = (a + b) / 2.0;
    let m = f(mid)?;
    subdivide(f, (a, mid), (p, m), tol, depth + 1, out)?;
    subdivide(f, (mid, b), (m, q), tol, depth + 1, out)
}

pub(crate) fn circle(center: Pt, radius: f64, tol: f64) -> Result<Vec<Pt>> {
    arc(center, radius, 0.0, TAU, tol)
}
