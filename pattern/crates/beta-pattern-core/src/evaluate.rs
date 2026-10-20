//! Dependency evaluation and Marker-ready piece construction.
use crate::{
    formula::{self, Context as FormulaContext},
    geometry::*,
    grading,
    model::*,
};
use anyhow::{Context, Result, bail, ensure};
use beta_marker_core::{Mark, MarkKind, Piece, Pt};
use std::collections::{BTreeMap, BTreeSet};

/// Evaluated drawing object: one point or an open sampled path.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Geometry {
    /// Constructed point.
    Point(Pt),
    /// Sampled line or curve, including both ends.
    Path(Vec<Pt>),
}
/// Size-specific dependency evaluator. Cached values are immutable within one evaluation.
pub struct Evaluator<'a> {
    doc: &'a Document,
    size: &'a str,
    tolerance: f64,
    objects: BTreeMap<String, Geometry>,
    variables: BTreeMap<String, f64>,
    point_snapshot: Option<BTreeMap<String, Pt>>,
    stack: Vec<String>,
}
impl<'a> Evaluator<'a> {
    /// Create an evaluator. Curves are flattened to a positive tolerance in millimetres.
    pub fn new(doc: &'a Document, size: &'a str, tolerance: f64) -> Result<Self> {
        ensure!(
            doc.format == DOC_FORMAT,
            "unsupported document format '{}'",
            doc.format
        );
        ensure!(doc.sizes.iter().any(|s| s == size), "unknown size '{size}'");
        ensure!(
            tolerance.is_finite() && tolerance > 0.,
            "tolerance must be positive"
        );
        Ok(Self {
            doc,
            size,
            tolerance,
            objects: BTreeMap::new(),
            variables: BTreeMap::new(),
            point_snapshot: None,
            stack: vec![],
        })
    }
    fn enter(&mut self, name: String) -> Result<()> {
        ensure!(
            self.stack.len() < 128,
            "dependency depth exceeds 128 at {name}"
        );
        ensure!(
            !self.stack.contains(&name),
            "dependency cycle: {} -> {name}",
            self.stack.join(" -> ")
        );
        self.stack.push(name);
        Ok(())
    }
    /// Evaluate a numeric literal or formula in this size's context.
    pub fn value(&mut self, v: &Value) -> Result<f64> {
        let n = match v {
            Value::Number(n) => *n,
            Value::Formula(s) => formula::evaluate(s, self)?,
        };
        ensure!(n.is_finite(), "non-finite numeric value");
        Ok(n)
    }
    /// Resolve one point, with object IDs in any error chain.
    pub fn point(&mut self, id: &str) -> Result<Pt> {
        match self.object(id)? {
            Geometry::Point(p) => Ok(p),
            _ => bail!("object '{id}' is not a point"),
        }
    }
    /// Resolve one line or curve.
    pub fn path(&mut self, id: &str) -> Result<Vec<Pt>> {
        match self.object(id)? {
            Geometry::Path(p) => Ok(p),
            _ => bail!("object '{id}' is not a line or curve"),
        }
    }
    /// Evaluate an object, detecting cycles across points, curves and variables.
    pub fn object(&mut self, id: &str) -> Result<Geometry> {
        if let Some(g) = self.objects.get(id) {
            return Ok(g.clone());
        }
        self.enter(format!("object:{id}"))?;
        let result = self
            .compute(id)
            .with_context(|| format!("object '{id}' (size {})", self.size));
        self.stack.pop();
        if let Ok(g) = &result {
            self.objects.insert(id.into(), g.clone());
        }
        result
    }
    fn all_points(&mut self) -> Result<BTreeMap<String, Pt>> {
        if let Some(points) = &self.point_snapshot {
            return Ok(points.clone());
        }
        let mut points = BTreeMap::new();
        for (id, o) in &self.doc.drawing {
            if is_point_object(self.doc, o, 0) {
                points.insert(id.clone(), self.point(id)?);
            }
        }
        self.point_snapshot = Some(points.clone());
        Ok(points)
    }
    fn circular_arc(&mut self, id: &str) -> Result<(Pt, f64, f64, f64)> {
        let o = self
            .doc
            .drawing
            .get(id)
            .context("unknown circular arc")?
            .clone();
        let (center, radius, start, sweep) = match o {
            Object::Arc {
                center,
                radius,
                start,
                sweep,
            } => (
                self.point(&center)?,
                self.value(&radius)?,
                self.value(&start)?,
                self.value(&sweep)?,
            ),
            Object::ArcLength {
                center,
                radius,
                start,
                length,
            } => {
                let r = self.value(&radius)?;
                ensure!(r > 0., "radius must be positive");
                (
                    self.point(&center)?,
                    r,
                    self.value(&start)?,
                    (self.value(&length)? / r).to_degrees(),
                )
            }
            _ => bail!("'{id}' is not a circular arc"),
        };
        ensure!(
            radius > 0. && sweep.abs() <= 360. && sweep.abs() > 1e-9,
            "invalid circular arc"
        );
        Ok((center, radius, start, sweep))
    }
    fn point_on_path(&mut self, id: &str, d: f64) -> Result<Pt> {
        if matches!(
            self.doc.drawing.get(id),
            Some(Object::Arc { .. } | Object::ArcLength { .. })
        ) {
            let (c, r, start, sweep) = self.circular_arc(id)?;
            ensure!(
                d >= 0. && d <= r * sweep.to_radians().abs() + 1e-9,
                "distance outside arc"
            );
            let angle = start.to_radians() + d / r * sweep.signum();
            Ok(add(c, [r * angle.cos(), r * angle.sin()]))
        } else {
            Ok(at_distance(&self.path(id)?, d)?.0)
        }
    }
    fn compute(&mut self, id: &str) -> Result<Geometry> {
        let o = self
            .doc
            .drawing
            .get(id)
            .with_context(|| format!("unknown drawing object '{id}'"))?
            .clone();
        let geometry = match o {
            Object::Point { operation } => Geometry::Point(match operation {
                PointOp::PathEndpoint { curve, index } => {
                    ensure!(index <= 1, "endpoint index must be 0 or 1");
                    let p = self.path(&curve)?;
                    *if index == 0 { p.first() } else { p.last() }.context("empty path")?
                }
                PointOp::Absolute { x, y } => [self.value(&x)?, self.value(&y)?],
                PointOp::Midpoint { a, b } => mul(add(self.point(&a)?, self.point(&b)?), 0.5),
                PointOp::Polar {
                    from,
                    angle,
                    distance,
                } => {
                    let p = self.point(&from)?;
                    let a = self.value(&angle)?.to_radians();
                    add(p, mul([a.cos(), a.sin()], self.value(&distance)?))
                }
                PointOp::AlongLine { from, to, distance } => {
                    let a = self.point(&from)?;
                    let b = self.point(&to)?;
                    add(a, mul(unit(sub(b, a))?, self.value(&distance)?))
                }
                PointOp::Normal { a, b, distance } => {
                    let a = self.point(&a)?;
                    let b = self.point(&b)?;
                    let u = unit(sub(b, a))?;
                    add(a, mul([-u[1], u[0]], self.value(&distance)?))
                }
                PointOp::Foot { point, a, b } => {
                    let p = self.point(&point)?;
                    let a = self.point(&a)?;
                    let u = unit(sub(self.point(&b)?, a))?;
                    add(a, mul(u, dot(sub(p, a), u)))
                }
                PointOp::Bisector {
                    vertex,
                    a,
                    b,
                    distance,
                } => {
                    let v = self.point(&vertex)?;
                    let u = unit(add(
                        unit(sub(self.point(&a)?, v))?,
                        unit(sub(self.point(&b)?, v))?,
                    ))?;
                    add(v, mul(u, self.value(&distance)?))
                }
                PointOp::LineIntersection { a, b } => line_intersection(
                    self.point(&a[0])?,
                    self.point(&a[1])?,
                    self.point(&b[0])?,
                    self.point(&b[1])?,
                )
                .context("lines are parallel or coincident")?,
                PointOp::IntersectXY { x_point, y_point } => {
                    [self.point(&x_point)?[0], self.point(&y_point)?[1]]
                }
                PointOp::Shoulder {
                    center,
                    length,
                    a,
                    b,
                    index,
                } => pick(
                    circle_line(
                        self.point(&center)?,
                        self.value(&length)?,
                        self.point(&a)?,
                        self.point(&b)?,
                    )?,
                    index,
                )?,
                PointOp::Triangle {
                    a,
                    b,
                    axis_a,
                    axis_b,
                    index,
                } => {
                    let a = self.point(&a)?;
                    let b = self.point(&b)?;
                    pick(
                        circle_line(
                            mul(add(a, b), 0.5),
                            distance(a, b) * 0.5,
                            self.point(&axis_a)?,
                            self.point(&axis_b)?,
                        )?,
                        index,
                    )?
                }
                PointOp::LineAxis {
                    a,
                    b,
                    origin,
                    angle,
                } => {
                    let origin = self.point(&origin)?;
                    let angle = self.value(&angle)?.to_radians();
                    line_intersection(
                        self.point(&a)?,
                        self.point(&b)?,
                        origin,
                        add(origin, [angle.cos(), angle.sin()]),
                    )
                    .context("line and axis are parallel or coincident")?
                }
                PointOp::CurveAxis {
                    curve,
                    origin,
                    angle,
                    index,
                } => pick(
                    path_axis(
                        &self.path(&curve)?,
                        self.point(&origin)?,
                        self.value(&angle)?,
                    )?,
                    index,
                )?,
                PointOp::CircleIntersection {
                    a,
                    radius_a,
                    b,
                    radius_b,
                    index,
                } => pick(
                    circle_intersections(
                        self.point(&a)?,
                        self.value(&radius_a)?,
                        self.point(&b)?,
                        self.value(&radius_b)?,
                    )?,
                    index,
                )?,
                PointOp::CircleLine {
                    center,
                    radius,
                    a,
                    b,
                    index,
                } => pick(
                    circle_line(
                        self.point(&center)?,
                        self.value(&radius)?,
                        self.point(&a)?,
                        self.point(&b)?,
                    )?,
                    index,
                )?,
                PointOp::CircleTangent {
                    point,
                    center,
                    radius,
                    index,
                } => pick(
                    circle_tangents(
                        self.point(&point)?,
                        self.point(&center)?,
                        self.value(&radius)?,
                    )?,
                    index,
                )?,
                PointOp::ArcIntersection { a, b, index } => {
                    let a = self.circular_arc(&a)?;
                    let b = self.circular_arc(&b)?;
                    pick(
                        circle_intersections(a.0, a.1, b.0, b.1)?
                            .into_iter()
                            .filter(|p| arc_contains(a, *p) && arc_contains(b, *p))
                            .collect(),
                        index,
                    )?
                }
                PointOp::ArcLine { arc, a, b, index } => {
                    let arc = self.circular_arc(&arc)?;
                    pick(
                        circle_line(arc.0, arc.1, self.point(&a)?, self.point(&b)?)?
                            .into_iter()
                            .filter(|p| arc_contains(arc, *p))
                            .collect(),
                        index,
                    )?
                }
                PointOp::ArcTangent { point, arc, index } => {
                    let arc = self.circular_arc(&arc)?;
                    pick(
                        circle_tangents(self.point(&point)?, arc.0, arc.1)?
                            .into_iter()
                            .filter(|p| arc_contains(arc, *p))
                            .collect(),
                        index,
                    )?
                }
                PointOp::CurvePoint { curve, distance } => {
                    let d = self.value(&distance)?;
                    self.point_on_path(&curve, d)?
                }
                PointOp::TrueDart { apex, leg, other } => {
                    let a = self.point(&apex)?;
                    add(
                        a,
                        mul(
                            unit(sub(self.point(&leg)?, a))?,
                            distance(a, self.point(&other)?),
                        ),
                    )
                }
                PointOp::Intersection { a, b, index } => {
                    *intersections(&self.path(&a)?, &self.path(&b)?)
                        .get(index)
                        .context("intersection index out of range")?
                }
            }),
            Object::Line { from, to } => {
                let a = self.point(&from)?;
                let b = self.point(&to)?;
                ensure!(distance(a, b) > 1e-9, "zero-length line");
                Geometry::Path(vec![a, b])
            }
            Object::Cubic { points } => Geometry::Path(flatten_cubic(
                [
                    self.point(&points[0])?,
                    self.point(&points[1])?,
                    self.point(&points[2])?,
                    self.point(&points[3])?,
                ],
                self.tolerance,
            )?),
            Object::Spline { points, tangents } => {
                let ps = points
                    .iter()
                    .map(|p| self.point(p))
                    .collect::<Result<Vec<_>>>()?;
                let ts = tangents
                    .iter()
                    .map(|t| Ok([self.value(&t[0])?, self.value(&t[1])?]))
                    .collect::<Result<Vec<_>>>()?;
                Geometry::Path(flatten_spline(&ps, &ts, self.tolerance)?)
            }
            Object::ArcLength {
                center,
                radius,
                start,
                length,
            } => {
                let r = self.value(&radius)?;
                ensure!(r > 0., "radius must be positive");
                let sweep = (self.value(&length)? / r).to_degrees();
                Geometry::Path(flatten_arc(
                    self.point(&center)?,
                    r,
                    self.value(&start)?,
                    sweep,
                    self.tolerance,
                )?)
            }
            Object::EllipticalArc {
                center,
                radius_x,
                radius_y,
                rotation,
                start,
                sweep,
            } => Geometry::Path(flatten_ellipse(
                self.point(&center)?,
                [self.value(&radius_x)?, self.value(&radius_y)?],
                self.value(&rotation)?,
                self.value(&start)?,
                self.value(&sweep)?,
                self.tolerance,
            )?),
            Object::CubicPath { segments } => {
                ensure!(
                    !segments.is_empty() && segments.len() <= 16384,
                    "invalid cubic path segment count"
                );
                let mut out = Vec::new();
                for ids in segments {
                    let ps = [
                        self.point(&ids[0])?,
                        self.point(&ids[1])?,
                        self.point(&ids[2])?,
                        self.point(&ids[3])?,
                    ];
                    if let Some(p) = out.last() {
                        ensure!(distance(*p, ps[0]) < 1e-6, "disconnected cubic path");
                    }
                    let segment = flatten_cubic(ps, self.tolerance)?;
                    out.extend(segment.into_iter().skip(usize::from(!out.is_empty())));
                    ensure!(out.len() <= 65537, "cubic path sample limit exceeded");
                }
                Geometry::Path(out)
            }
            Object::CutPath {
                curve,
                start,
                length: interval,
            } => {
                let start = self.value(&start)?;
                let interval = self.value(&interval)?;
                Geometry::Path(cut_path(&self.path(&curve)?, start, interval)?)
            }
            Object::Move { source, dx, dy } => {
                let delta = [self.value(&dx)?, self.value(&dy)?];
                map_geometry(self.object(&source)?, |p| add(p, delta))
            }
            Object::Rotate {
                source,
                pivot,
                angle,
            } => {
                let pivot = self.point(&pivot)?;
                let (s, c) = self.value(&angle)?.to_radians().sin_cos();
                map_geometry(self.object(&source)?, |p| {
                    let v = sub(p, pivot);
                    add(pivot, [c * v[0] - s * v[1], s * v[0] + c * v[1]])
                })
            }
            Object::MirrorLine { source, a, b } => {
                let a = self.point(&a)?;
                let u = unit(sub(self.point(&b)?, a))?;
                map_geometry(self.object(&source)?, |p| {
                    let v = sub(p, a);
                    add(a, sub(mul(u, 2. * dot(v, u)), v))
                })
            }
            Object::MirrorAxis {
                source,
                origin,
                axis,
            } => {
                let o = self.point(&origin)?;
                map_geometry(self.object(&source)?, |p| match axis {
                    Axis::Horizontal => [p[0], 2. * o[1] - p[1]],
                    Axis::Vertical => [2. * o[0] - p[0], p[1]],
                })
            }
            Object::Arc {
                center,
                radius,
                start,
                sweep,
            } => Geometry::Path(flatten_arc(
                self.point(&center)?,
                self.value(&radius)?,
                self.value(&start)?,
                self.value(&sweep)?,
                self.tolerance,
            )?),
        };
        let finite = match &geometry {
            Geometry::Point(p) => p.iter().all(|v| v.is_finite()),
            Geometry::Path(p) => p.iter().flatten().all(|v| v.is_finite()),
        };
        ensure!(finite, "construction produced non-finite coordinates");
        Ok(geometry)
    }
}
fn pick(points: Vec<Pt>, index: usize) -> Result<Pt> {
    points
        .get(index)
        .copied()
        .context("intersection/contact index out of range")
}
fn arc_contains(arc: (Pt, f64, f64, f64), p: Pt) -> bool {
    let v = sub(p, arc.0);
    angle_in_arc(v[1].atan2(v[0]).to_degrees(), arc.2, arc.3)
}
fn map_geometry(g: Geometry, f: impl Fn(Pt) -> Pt) -> Geometry {
    match g {
        Geometry::Point(p) => Geometry::Point(f(p)),
        Geometry::Path(p) => Geometry::Path(p.into_iter().map(f).collect()),
    }
}
impl FormulaContext for Evaluator<'_> {
    fn resolve(&mut self, n: &str) -> Result<f64> {
        if let Some(m) = n.strip_prefix("m.") {
            return self
                .doc
                .measurements
                .get(self.size)
                .and_then(|t| t.get(m))
                .copied()
                .with_context(|| format!("measurement '{m}' missing for size {}", self.size));
        }
        if let Some(v) = n.strip_prefix("v.") {
            if let Some(n) = self.variables.get(v) {
                return Ok(*n);
            }
            let formula = self
                .doc
                .variables
                .get(v)
                .with_context(|| format!("unknown variable '{v}'"))?
                .clone();
            self.enter(format!("variable:{v}"))?;
            let result = self
                .value(&formula)
                .with_context(|| format!("variable '{v}'"));
            self.stack.pop();
            if let Ok(n) = result {
                self.variables.insert(v.into(), n);
            }
            return result;
        }
        bail!("unknown reference '{n}' (use m.name or v.name)")
    }
    fn geometry(&mut self, n: &str, ids: &[String]) -> Result<f64> {
        match (n, ids) {
            ("length", [id]) => Ok(length(&self.path(id)?)),
            ("x", [id]) => Ok(self.point(id)?[0]),
            ("y", [id]) => Ok(self.point(id)?[1]),
            ("distance", [a, b]) => Ok(distance(self.point(a)?, self.point(b)?)),
            ("angle", [a, b]) => {
                let v = unit(sub(self.point(b)?, self.point(a)?))?;
                Ok(v[1].atan2(v[0]).to_degrees())
            }
            _ => bail!("wrong geometry arguments for '{n}'"),
        }
    }
}

fn is_point_object(doc: &Document, o: &Object, depth: usize) -> bool {
    if depth >= 128 {
        return false;
    }
    match o {
        Object::Point { .. } => true,
        Object::Move { source, .. }
        | Object::Rotate { source, .. }
        | Object::MirrorLine { source, .. }
        | Object::MirrorAxis { source, .. } => doc
            .drawing
            .get(source)
            .is_some_and(|o| is_point_object(doc, o, depth + 1)),
        _ => false,
    }
}
fn shaped_notch(q: Pt, t: Pt, depth: f64, width: f64, shape: NotchType, ccw: bool) -> Result<Mark> {
    ensure!(
        depth.is_finite()
            && depth > 0.
            && width.is_finite()
            && (shape == NotchType::Slit || width > 0.),
        "invalid notch dimensions"
    );
    let n = mul([-t[1], t[0]], if ccw { depth } else { -depth });
    let side = mul(t, width * 0.5);
    let tip = add(q, n);
    let points = match shape {
        NotchType::Slit => vec![q, tip],
        NotchType::V => vec![sub(q, side), tip, add(q, side)],
        NotchType::T => vec![q, tip, sub(tip, side), add(tip, side)],
        NotchType::Castle => vec![sub(q, side), sub(tip, side), add(tip, side), add(q, side)],
    };
    Ok(Mark {
        kind: MarkKind::Notch,
        points,
        text: Some(format!("{:?}", shape).to_lowercase()),
    })
}
/// Evaluate and grade every piece for one size. Factory rules use the base-size draft.
pub fn grade(doc: &Document, size: &str, tolerance: f64) -> Result<Vec<GradedPiece>> {
    validate_document(doc)?;
    let mut measured = Evaluator::new(doc, size, tolerance)?;
    let mut factory = Evaluator::new(doc, &doc.base_size, tolerance)?;
    doc.pieces
        .iter()
        .map(|p| {
            let e = if p.grade_points.is_empty() {
                &mut measured
            } else {
                &mut factory
            };
            grade_piece(doc, p, size, e, true)
                .with_context(|| format!("piece '{}' size '{size}'", p.id))
        })
        .collect()
}
fn validate_document(doc: &Document) -> Result<()> {
    ensure!(
        doc.sizes.iter().collect::<BTreeSet<_>>().len() == doc.sizes.len(),
        "duplicate sizes"
    );
    ensure!(
        doc.sizes.contains(&doc.base_size),
        "base size not in size order"
    );
    ensure!(
        doc.pieces
            .iter()
            .map(|p| &p.id)
            .collect::<BTreeSet<_>>()
            .len()
            == doc.pieces.len(),
        "duplicate piece IDs"
    );
    Ok(())
}
/// Complete size evaluation with drawing geometry returned once and per-piece errors.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PatternEvaluation {
    /// Every drawing object, including individual construction errors.
    pub objects: Vec<crate::editor::ObjectEvaluation>,
    /// Successful pieces in document order. Their `points` maps contain factory-rule,
    /// grain, fold, anchor and label references; full drawing geometry is in `objects`.
    pub pieces: Vec<GradedPiece>,
    /// Piece ID -> full evaluation error chain.
    pub piece_errors: BTreeMap<String, String>,
}
/// Evaluate the whole size with one shared construction cache. This avoids evaluating
/// the drawing a second time when an editor needs both objects and graded pieces.
/// Drawing points are returned once in `objects`, rather than copied into every
/// piece. Use `grade` when all global points are needed in each piece's point map.
pub fn evaluate_pattern(doc: &Document, size: &str, tolerance: f64) -> Result<PatternEvaluation> {
    validate_document(doc)?;
    let mut e = Evaluator::new(doc, size, tolerance)?;
    let mut factory = Evaluator::new(doc, &doc.base_size, tolerance)?;
    let objects = doc
        .drawing
        .keys()
        .map(|id| match e.object(id) {
            Ok(g) => crate::editor::ObjectEvaluation {
                id: id.clone(),
                geometry: Some(g),
                error: None,
            },
            Err(err) => crate::editor::ObjectEvaluation {
                id: id.clone(),
                geometry: None,
                error: Some(format!("{err:#}")),
            },
        })
        .collect();
    let mut pieces = vec![];
    let mut piece_errors = BTreeMap::new();
    for p in &doc.pieces {
        let evaluator = if p.grade_points.is_empty() {
            &mut e
        } else {
            &mut factory
        };
        match grade_piece(doc, p, size, evaluator, false) {
            Ok(piece) => pieces.push(piece),
            Err(err) => {
                piece_errors.insert(p.id.clone(), format!("{err:#}"));
            }
        }
    }
    Ok(PatternEvaluation {
        objects,
        pieces,
        piece_errors,
    })
}
fn grade_piece(
    doc: &Document,
    p: &PatternPiece,
    size: &str,
    e: &mut Evaluator<'_>,
    include_drawing_points: bool,
) -> Result<GradedPiece> {
    ensure!(doc.sizes.iter().any(|s| s == size), "unknown size '{size}'");
    ensure!(!p.edges.is_empty(), "piece has no edges");
    let mut paths = vec![];
    let mut seam = vec![];
    let mut widths = vec![];
    let mut corners = vec![];
    for edge in &p.edges {
        let mut path = e.path(&edge.object)?;
        if edge.reversed {
            path.reverse();
        }
        ensure!(path.len() >= 2, "edge has no segments");
        if let Some(last) = paths.last().and_then(|v: &Vec<Pt>| v.last()) {
            ensure!(
                distance(*last, path[0]) < 1e-6,
                "disconnected edge '{}'",
                edge.object
            );
        }
        let w = e.value(&edge.allowance)?;
        ensure!(w >= 0., "negative allowance on '{}'", edge.object);
        for (i, q) in path.iter().take(path.len() - 1).enumerate() {
            seam.push(*q);
            widths.push(w);
            corners.push(if i == 0 { edge.corner } else { Corner::Miter });
        }
        paths.push(path);
    }
    ensure!(
        distance(
            paths[paths.len() - 1][paths[paths.len() - 1].len() - 1],
            paths[0][0]
        ) < 1e-6,
        "piece path is not closed"
    );
    validate_polygon(&seam)?;
    let mut points = if include_drawing_points {
        e.all_points()?
    } else {
        // The combined API already returns the whole drawing once. Keep only
        // piece-specific reference points here, avoiding N copies of the drawing.
        let ids: BTreeSet<_> = p
            .grade_points
            .keys()
            .chain(p.grain.iter().flatten())
            .chain(p.fold_line.iter().flatten())
            .chain(p.anchors.values())
            .chain(p.label_point.iter())
            .collect();
        ids.into_iter()
            .map(|id| Ok((id.clone(), e.point(id)?)))
            .collect::<Result<BTreeMap<_, _>>>()?
    };
    let shifts = if p.grade_points.is_empty() {
        vec![[0., 0.]; seam.len()]
    } else {
        let mut anchors = BTreeMap::new();
        for (id, r) in &p.grade_points {
            let q = points
                .get(id)
                .with_context(|| format!("unknown grade point '{id}'"))?;
            let i = seam
                .iter()
                .position(|s| distance(*s, *q) < 1e-6)
                .with_context(|| format!("grade point '{id}' is not an outline vertex"))?;
            let rule = doc
                .rules
                .get(r)
                .with_context(|| format!("unknown grade rule '{r}'"))?;
            let delta = grading::rule_delta(doc, rule, size)
                .with_context(|| format!("rule '{r}' point '{id}'"))?;
            if let Some(old) = anchors.insert(i, delta) {
                ensure!(
                    distance(old, delta) < 1e-9,
                    "conflicting grade rules at point '{id}'"
                );
            }
        }
        grading::interpolate(&seam, &anchors)?
    };
    let stationary = shifts.iter().all(|d| *d == [0., 0.]);
    let move_p = |q| {
        if stationary {
            q
        } else {
            grading::move_point(q, &seam, &shifts)
        }
    };
    let graded_seam: Vec<_> = seam.iter().zip(&shifts).map(|(q, d)| add(*q, *d)).collect();
    if !stationary {
        validate_polygon(&graded_seam)?;
    }
    let cut = offset(&graded_seam, &widths, &corners)?;
    let line = |ids: &[String; 2], points: &BTreeMap<String, Pt>| -> Result<[Pt; 2]> {
        Ok([
            move_p(*points.get(&ids[0]).context("line start point missing")?),
            move_p(*points.get(&ids[1]).context("line end point missing")?),
        ])
    };
    let mut marks = vec![];
    for a in &p.annotations {
        if let Annotation::SeamIntersections {
            path,
            depth,
            width,
            shape,
        } = a
        {
            let guide = e.path(path)?;
            let guide: Vec<_> = guide.into_iter().map(move_p).collect();
            let mut seen = vec![];
            for path in &paths {
                let path: Vec<_> = path.iter().copied().map(move_p).collect();
                for w in path.windows(2) {
                    for q in intersections(w, &guide) {
                        if seen.iter().any(|p| distance(*p, q) < 1e-6) {
                            continue;
                        }
                        seen.push(q);
                        let t = unit(sub(w[1], w[0]))?;
                        marks.push(shaped_notch(
                            q,
                            t,
                            e.value(depth)?,
                            e.value(width)?,
                            *shape,
                            signed_area(&graded_seam) > 0.,
                        )?);
                    }
                }
            }
            continue;
        }
        marks.push(match a {
            Annotation::SeamIntersections { .. } => unreachable!(),
            Annotation::ShapedNotch {
                edge,
                distance,
                depth,
                width,
                shape,
            } => {
                let path = paths.get(*edge).context("notch edge index out of range")?;
                let path: Vec<_> = path.iter().copied().map(move_p).collect();
                let (q, t) = at_distance(&path, e.value(distance)?)?;
                shaped_notch(
                    q,
                    t,
                    e.value(depth)?,
                    e.value(width)?,
                    *shape,
                    signed_area(&graded_seam) > 0.,
                )?
            }
            Annotation::Points {
                mark,
                points: ids,
                text,
            } => {
                ensure!(!ids.is_empty(), "empty annotation");
                let ps = ids
                    .iter()
                    .map(|id| Ok(move_p(e.point(id)?)))
                    .collect::<Result<Vec<_>>>()?;
                ensure!(
                    !matches!(mark, MarkKind::Drill | MarkKind::Text) || ps.len() == 1,
                    "drill/text must have one point"
                );
                Mark {
                    kind: *mark,
                    points: ps,
                    text: text.clone(),
                }
            }
            Annotation::Button { point, diameter } => {
                let d = e.value(diameter)?;
                ensure!(d > 0., "button diameter must be positive");
                Mark {
                    kind: MarkKind::Drill,
                    points: vec![move_p(e.point(point)?)],
                    text: Some(format!("button diameter {d} mm")),
                }
            }
            Annotation::Notch {
                edge,
                distance,
                depth,
            } => {
                let path = paths.get(*edge).context("notch edge index out of range")?;
                let path: Vec<_> = path.iter().map(|q| move_p(*q)).collect();
                let (q, t) = at_distance(&path, e.value(distance)?)?;
                let d = e.value(depth)?;
                ensure!(d > 0., "notch depth must be positive");
                let inward = mul([-t[1], t[0]], signed_area(&graded_seam).signum() * d);
                Mark {
                    kind: MarkKind::Notch,
                    points: vec![q, add(q, inward)],
                    text: None,
                }
            }
        });
    }
    for internal in &p.internal_paths {
        ensure!(!internal.edges.is_empty(), "empty internal path");
        let mut points = Vec::new();
        for edge in &internal.edges {
            let mut path = e.path(&edge.object)?;
            if edge.reversed {
                path.reverse();
            }
            if let Some(last) = points.last() {
                ensure!(
                    distance(*last, path[0]) < 1e-6,
                    "disconnected internal path"
                );
            }
            let skip = usize::from(!points.is_empty());
            points.extend(path.into_iter().skip(skip));
        }
        if internal.closed {
            ensure!(
                distance(points[0], points[points.len() - 1]) < 1e-6,
                "internal path is not closed"
            );
        }
        marks.push(Mark {
            kind: MarkKind::Line,
            points: points.into_iter().map(move_p).collect(),
            text: Some(internal.name.clone()),
        });
    }
    if let Some(id) = &p.label_point {
        let mut text = format!(
            "{} | size {} | cut {} + {} mirrored | fabric {}",
            p.name,
            size,
            p.quantity,
            p.mirror_quantity,
            p.material.as_deref().unwrap_or("unspecified")
        );
        if let Some(label) = &doc.label {
            text.push_str(" | ");
            text.push_str(label);
        }
        marks.push(Mark {
            kind: MarkKind::Text,
            points: vec![move_p(e.point(id)?)],
            text: Some(text),
        });
    }
    let grain = if let Some(ids) = &p.grain {
        Some(line(ids, &points)?)
    } else if let Some(g) = &p.grain_angle {
        let origin = e.point(&g.origin)?;
        let angle = e.value(&g.angle)?.to_radians();
        let len = e.value(&g.length)?;
        ensure!(len > 0., "grain length must be positive");
        Some([
            move_p(origin),
            move_p(add(origin, mul([angle.cos(), angle.sin()], len))),
        ])
    } else {
        None
    };
    let match_points = p
        .anchors
        .iter()
        .map(|(id, point)| {
            Ok(beta_marker_core::MatchPoint {
                id: id.clone(),
                point: move_p(e.point(point)?),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let buffer = e.value(&p.buffer)?;
    ensure!(buffer >= 0., "negative piece buffer");
    let piece = Piece {
        id: format!("{}-{size}", p.id),
        name: p.name.clone(),
        style: Some(doc.name.clone()),
        size: Some(size.into()),
        material: p.material.clone(),
        outline: cut,
        grain,
        match_points,
        quantity: p.quantity,
        mirror_quantity: p.mirror_quantity,
        rotation: p.rotation,
        buffer,
        marks,
        fold_line: p.fold_line.as_ref().map(|g| line(g, &points)).transpose()?,
        ..Piece::default()
    };
    if !stationary {
        for q in points.values_mut() {
            *q = move_p(*q);
        }
    }
    Ok(GradedPiece {
        piece,
        seam: graded_seam,
        points,
    })
}
/// Grade all selected sizes, then align every piece at the requested point.
pub fn nest(doc: &Document, sizes: &[String], reference: &str, tolerance: f64) -> Result<Nest> {
    let mut out = BTreeMap::new();
    for size in sizes {
        let mut pieces = grade(doc, size, tolerance)?;
        for p in &mut pieces {
            let origin = *p
                .points
                .get(reference)
                .context("nest reference point missing")?;
            translate(p, mul(origin, -1.));
        }
        out.insert(size.clone(), pieces);
    }
    Ok(Nest {
        reference: reference.into(),
        sizes: out,
    })
}
fn translate(p: &mut GradedPiece, d: Pt) {
    for q in p
        .seam
        .iter_mut()
        .chain(p.piece.outline.iter_mut())
        .chain(p.points.values_mut())
        .chain(p.piece.grain.iter_mut().flatten())
        .chain(p.piece.fold_line.iter_mut().flatten())
        .chain(p.piece.marks.iter_mut().flat_map(|m| &mut m.points))
    {
        *q = add(*q, d);
    }
}

/// Generic editor evaluation API.
pub use crate::editor::{ObjectEvaluation, evaluate_document};

#[cfg(test)]
mod tests {
    use super::*;
    fn point(d: &mut Document, id: &str, x: Value, y: Value) {
        d.drawing.insert(
            id.into(),
            Object::Point {
                operation: PointOp::Absolute { x, y },
            },
        );
    }
    #[test]
    fn constructions_and_formulas() {
        let mut d = Document::default();
        point(&mut d, "A", 0.0.into(), 0.0.into());
        point(&mut d, "B", 10.0.into(), 0.0.into());
        point(&mut d, "C", 0.0.into(), 10.0.into());
        let ops = [
            (
                "mid",
                PointOp::Midpoint {
                    a: "A".into(),
                    b: "B".into(),
                },
                [5., 0.],
            ),
            (
                "along",
                PointOp::AlongLine {
                    from: "A".into(),
                    to: "B".into(),
                    distance: 3.0.into(),
                },
                [3., 0.],
            ),
            (
                "polar",
                PointOp::Polar {
                    from: "A".into(),
                    angle: 90.0.into(),
                    distance: 4.0.into(),
                },
                [0., 4.],
            ),
            (
                "normal",
                PointOp::Normal {
                    a: "A".into(),
                    b: "B".into(),
                    distance: 4.0.into(),
                },
                [0., 4.],
            ),
            (
                "foot",
                PointOp::Foot {
                    point: "C".into(),
                    a: "A".into(),
                    b: "B".into(),
                },
                [0., 0.],
            ),
            (
                "bisector",
                PointOp::Bisector {
                    vertex: "A".into(),
                    a: "B".into(),
                    b: "C".into(),
                    distance: (2f64.sqrt()).into(),
                },
                [1., 1.],
            ),
            (
                "intersect",
                PointOp::LineIntersection {
                    a: ["A".into(), "B".into()],
                    b: ["C".into(), "A".into()],
                },
                [0., 0.],
            ),
        ];
        for (id, op, _) in &ops {
            d.drawing.insert(
                (*id).into(),
                Object::Point {
                    operation: op.clone(),
                },
            );
        }
        d.drawing.insert(
            "line".into(),
            Object::Line {
                from: "A".into(),
                to: "B".into(),
            },
        );
        let mut e = Evaluator::new(&d, "M", 0.01).unwrap();
        for (id, _, expected) in ops {
            assert!(distance(e.point(id).unwrap(), expected) < 1e-9, "{id}");
        }
        assert_eq!(
            e.value(&"length(line)+distance(A,C)+angle(A,C)+x(B)+y(C)".into())
                .unwrap(),
            130.
        );
    }
    #[test]
    fn cycles_name_the_object() {
        let mut d = Document::default();
        point(&mut d, "A", "v.loop".into(), 0.0.into());
        d.variables.insert("loop".into(), "x(A)".into());
        let s = format!(
            "{:#}",
            Evaluator::new(&d, "M", 0.1)
                .unwrap()
                .point("A")
                .unwrap_err()
        );
        assert!(
            s.contains("cycle") && s.contains("A") && s.contains("loop"),
            "{s}"
        );
    }
}
