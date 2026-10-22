//! Parametric path sampling, normal offsets and nearest-point queries.
use crate::{evaluate::Evaluator, geometry::*, model::*};
use anyhow::{Context, Result, ensure};
use beta_marker_core::Pt;

pub(crate) enum Curve {
    Line([Pt; 2]),
    Cubic([Pt; 4]),
    Arc(Pt, f64, f64, f64),
    Polyline(Vec<Pt>),
    Transform(Box<Curve>, [f64; 6]),
}
impl Curve {
    pub(crate) fn from_evaluator(doc: &Document, e: &mut Evaluator<'_>, id: &str) -> Result<Self> {
        // Resolve through the guarded evaluator before recursively inspecting transforms.
        let path = e.path(id)?;
        ensure!(
            path.len() >= 2 && length(&path) > 1e-9,
            "Path has zero length"
        );
        Ok(match doc.drawing.get(id).context("Path missing")? {
            Object::Line { .. } => Self::Line([path[0], path[1]]),
            Object::OffsetPath { source, .. }
                if Self::from_evaluator(doc, e, source)?.is_line() =>
            {
                Self::Line([path[0], *path.last().context("Empty offset line")?])
            }
            Object::Cubic { points: p } => Self::Cubic([
                e.point(&p[0])?,
                e.point(&p[1])?,
                e.point(&p[2])?,
                e.point(&p[3])?,
            ]),
            Object::Arc {
                center,
                radius,
                start,
                sweep,
            } => Self::Arc(
                e.point(center)?,
                e.value(radius)?,
                e.value(start)?.to_radians(),
                e.value(sweep)?.to_radians(),
            ),
            Object::ArcLength {
                center,
                radius,
                start,
                length,
            } => {
                let r = e.value(radius)?;
                Self::Arc(
                    e.point(center)?,
                    r,
                    e.value(start)?.to_radians(),
                    e.value(length)? / r,
                )
            }
            Object::Move { source, dx, dy } => Self::Transform(
                Box::new(Self::from_evaluator(doc, e, source)?),
                [1., 0., 0., 1., e.value(dx)?, e.value(dy)?],
            ),
            Object::Rotate {
                source,
                pivot,
                angle,
            } => {
                let p = e.point(pivot)?;
                let (s, c) = e.value(angle)?.to_radians().sin_cos();
                Self::Transform(
                    Box::new(Self::from_evaluator(doc, e, source)?),
                    [
                        c,
                        -s,
                        s,
                        c,
                        p[0] - c * p[0] + s * p[1],
                        p[1] - s * p[0] - c * p[1],
                    ],
                )
            }
            Object::MirrorAxis {
                source,
                origin,
                axis,
            } => {
                let p = e.point(origin)?;
                let m = match axis {
                    Axis::Horizontal => [1., 0., 0., -1., 0., 2. * p[1]],
                    Axis::Vertical => [-1., 0., 0., 1., 2. * p[0], 0.],
                };
                Self::Transform(Box::new(Self::from_evaluator(doc, e, source)?), m)
            }
            Object::MirrorLine { source, a, b } => {
                let p = e.point(a)?;
                let u = unit(sub(e.point(b)?, p))?;
                let a = 2. * u[0] * u[0] - 1.;
                let b = 2. * u[0] * u[1];
                let d = 2. * u[1] * u[1] - 1.;
                Self::Transform(
                    Box::new(Self::from_evaluator(doc, e, source)?),
                    [
                        a,
                        b,
                        b,
                        d,
                        p[0] - a * p[0] - b * p[1],
                        p[1] - b * p[0] - d * p[1],
                    ],
                )
            }
            _ => Self::Polyline(path),
        })
    }
    fn is_line(&self) -> bool {
        match self {
            Self::Line(_) => true,
            Self::Transform(c, _) => c.is_line(),
            _ => false,
        }
    }
    pub(crate) fn native(&self) -> bool {
        match self {
            Self::Transform(c, _) => c.native(),
            Self::Polyline(_) => false,
            _ => true,
        }
    }
    pub(crate) fn sample(&self, t: f64) -> Result<(Pt, Pt)> {
        Ok(match self {
            Self::Line(p) => (add(p[0], mul(sub(p[1], p[0]), t)), sub(p[1], p[0])),
            Self::Cubic(p) => {
                let u = 1. - t;
                let q = add(
                    add(mul(p[0], u * u * u), mul(p[1], 3. * u * u * t)),
                    add(mul(p[2], 3. * u * t * t), mul(p[3], t * t * t)),
                );
                let d = add(
                    add(
                        mul(sub(p[1], p[0]), 3. * u * u),
                        mul(sub(p[2], p[1]), 6. * u * t),
                    ),
                    mul(sub(p[3], p[2]), 3. * t * t),
                );
                (q, d)
            }
            Self::Arc(c, r, start, sweep) => {
                let (s, co) = (start + t * sweep).sin_cos();
                (add(*c, [r * co, r * s]), [-r * s * sweep, r * co * sweep])
            }
            Self::Polyline(p) => at_distance(p, t * length(p))?,
            Self::Transform(c, m) => {
                let (p, d) = c.sample(t)?;
                (
                    [
                        m[0] * p[0] + m[1] * p[1] + m[4],
                        m[2] * p[0] + m[3] * p[1] + m[5],
                    ],
                    [m[0] * d[0] + m[1] * d[1], m[2] * d[0] + m[3] * d[1]],
                )
            }
        })
    }
    fn normal_point(&self, t: f64, d: f64) -> Result<Pt> {
        let (p, v) = self.sample(t)?;
        if d == 0. {
            return Ok(p);
        }
        let u =
            unit(v).context("Offset meets a cusp or zero tangent; split the curve there first")?;
        Ok(add(p, mul([-u[1], u[0]], d)))
    }
    fn sampled(&self, d: f64, tolerance: f64) -> Result<Vec<(f64, Pt)>> {
        fn recurse(
            c: &Curve,
            d: f64,
            tol: f64,
            a: (f64, Pt),
            b: (f64, Pt),
            depth: u8,
            out: &mut Vec<(f64, Pt)>,
        ) -> Result<()> {
            ensure!(out.len() < 65536, "Path exceeds the sampling limit");
            let mut error: f64 = 0.;
            for f in [0.25, 0.5, 0.75] {
                let q = c.normal_point(a.0 + (b.0 - a.0) * f, d)?;
                error = error.max(distance(q, add(a.1, mul(sub(b.1, a.1), f))));
            }
            if error <= tol {
                out.push(b);
            } else {
                ensure!(
                    depth < 20,
                    "Offset is too tight or has a cusp; reduce distance or split the curve"
                );
                let t = (a.0 + b.0) * 0.5;
                let m = (t, c.normal_point(t, d)?);
                recurse(c, d, tol, a, m, depth + 1, out)?;
                recurse(c, d, tol, m, b, depth + 1, out)?;
            }
            Ok(())
        }
        let a = (0., self.normal_point(0., d)?);
        let b = (1., self.normal_point(1., d)?);
        let mut out = vec![a];
        recurse(self, d, tolerance, a, b, 0, &mut out)?;
        Ok(out)
    }
    pub(crate) fn offset(&self, distance_mm: f64, tolerance: f64) -> Result<Vec<Pt>> {
        ensure!(distance_mm.is_finite(), "Use a finite offset distance");
        // A polyline has corner joins rather than a unique normal at each vertex.
        // Reject unsupported sampled curves instead of presenting a false exact offset.
        ensure!(
            self.native(),
            "Exact normal offset supports lines, cubics and circular arcs (including moved, rotated or mirrored copies). Split other paths into these segments first."
        );
        let samples = self.sampled(distance_mm, tolerance.min(0.01))?;
        let out: Vec<_> = samples.iter().map(|s| s.1).collect();
        ensure!(
            out.len() <= 8192,
            "Offset exceeds 8192 samples; split the curve into smaller sections"
        );
        for w in samples.windows(2) {
            let t = (w[0].0 + w[1].0) * 0.5;
            let (_, v) = self.sample(t)?;
            ensure!(
                dot(sub(w[1].1, w[0].1), v) > 1e-12,
                "Offset folds back through a cusp; reduce the distance"
            );
        }
        for i in 0..out.len().saturating_sub(1) {
            for j in i + 2..out.len().saturating_sub(1) {
                if i == 0
                    && j == out.len() - 2
                    && crate::geometry::distance(out[0], out[out.len() - 1]) < 1e-8
                {
                    continue;
                }
                ensure!(
                    intersections(&out[i..=i + 1], &out[j..=j + 1]).is_empty(),
                    "Offset crosses itself; reduce the distance or split the curve"
                );
            }
        }
        Ok(out)
    }
    pub(crate) fn nearest(&self, at: Pt) -> Result<(Pt, f64, f64)> {
        let polyline = match self {
            Self::Polyline(p) => Some(p.as_slice()),
            Self::Line(p) => Some(p.as_slice()),
            _ => None,
        };
        if let Some(p) = polyline {
            let (q, along) = project(p, at)?;
            return Ok((q, along / length(p), along));
        }
        if !self.native() {
            let path = self.sampled(0., 0.01)?;
            let p: Vec<_> = path.iter().map(|s| s.1).collect();
            let (q, along) = project(&p, at)?;
            return Ok((q, along / length(&p), along));
        }
        let samples = self.sampled(0., 0.01)?;
        let mut best = (f64::INFINITY, 0., samples[0].1);
        // Each sampled span is a bracket. Only refine spans close enough to beat the best.
        for w in samples.windows(2) {
            let (q, _) = project(&[w[0].1, w[1].1], at)?;
            if distance(q, at) > best.0 + 0.02 {
                continue;
            }
            let (mut lo, mut hi) = (w[0].0, w[1].0);
            for _ in 0..36 {
                let a = lo + (hi - lo) / 3.;
                let b = hi - (hi - lo) / 3.;
                if distance(self.sample(a)?.0, at) < distance(self.sample(b)?.0, at) {
                    hi = b;
                } else {
                    lo = a;
                }
            }
            for t in [w[0].0, (lo + hi) * 0.5, w[1].0] {
                let p = self.sample(t)?.0;
                let d = distance(p, at);
                if d < best.0 {
                    best = (d, t, p);
                }
            }
        }
        let mut along = 0.;
        for w in samples.windows(2) {
            if best.1 >= w[1].0 {
                along += distance(w[0].1, w[1].1);
            } else if best.1 > w[0].0 {
                along += distance(w[0].1, best.2);
                break;
            } else {
                break;
            }
        }
        Ok((best.2, best.1, along))
    }
}
pub(crate) fn project(path: &[Pt], at: Pt) -> Result<(Pt, f64)> {
    let mut best = (f64::INFINITY, *path.first().context("Empty path")?, 0.);
    let mut along = 0.;
    for w in path.windows(2) {
        let v = sub(w[1], w[0]);
        let n = dot(v, v);
        let t = if n > 1e-18 {
            (dot(sub(at, w[0]), v) / n).clamp(0., 1.)
        } else {
            0.
        };
        let q = add(w[0], mul(v, t));
        let d = distance(at, q);
        if d < best.0 {
            best = (d, q, along + t * n.sqrt());
        }
        along += n.sqrt();
    }
    Ok((best.1, best.2))
}
