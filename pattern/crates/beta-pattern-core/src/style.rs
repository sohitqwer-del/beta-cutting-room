//! Validated immutable transactions and editor measurements for the style workflow.
use crate::{
    editor,
    evaluate::{self, Evaluator, Geometry},
    geometry,
    model::*,
    production, tools,
};
use anyhow::{Context, Result, ensure};
use beta_marker_core::Pt;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Curve flattening tolerance used by interactive edits, in mm.
pub const TOLERANCE: f64 = 0.1;
/// Editable body measurements, in mm, keyed first by size.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeChart {
    /// Human-readable source label.
    pub name: String,
    /// Ordered size names.
    pub sizes: Vec<String>,
    /// Reference size for factory rules.
    pub base_size: String,
    /// Size -> measurement -> millimetres.
    pub measurements: BTreeMap<String, BTreeMap<String, f64>>,
}
impl SizeChart {
    /// Reject missing, duplicate, non-finite or non-positive measurements.
    pub fn validate(&self) -> Result<()> {
        ensure!(
            !self.sizes.is_empty() && self.sizes.len() <= 100,
            "Choose 1 to 100 sizes."
        );
        let names: BTreeSet<_> = self.sizes.iter().collect();
        ensure!(
            names.len() == self.sizes.len() && names.iter().all(|s| !s.trim().is_empty()),
            "Size names must be non-empty and unique."
        );
        ensure!(
            names.contains(&self.base_size),
            "The base size must be in the size chart."
        );
        ensure!(
            self.measurements.len() == names.len()
                && names.iter().all(|s| self.measurements.contains_key(*s)),
            "Supply measurements for every size, and remove unknown sizes."
        );
        let mut keys = None;
        for (size, row) in &self.measurements {
            let current: BTreeSet<_> = row.keys().collect();
            ensure!(
                keys.as_ref().is_none_or(|k| *k == current),
                "Use the same measurement names in every size ({size})."
            );
            keys = Some(current);
            for (name, value) in row {
                ensure!(
                    editor::valid_id(name) && !name.contains('.'),
                    "Measurement name '{name}' must use letters, digits and underscores."
                );
                ensure!(
                    value.is_finite() && *value > 0. && *value <= 10000.,
                    "Measurement '{name}' in {size} must be above zero and at most 10000 mm."
                );
            }
        }
        Ok(())
    }
}

/// One atomic document operation. Geometry-specific menu actions use the WASM adapter.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum Operation {
    PutObject {
        id: String,
        object: Object,
        #[serde(default)]
        replace: bool,
    },
    DeleteObject {
        id: String,
    },
    RenameObject {
        id: String,
        new_id: String,
    },
    MovePoint {
        id: String,
        size: String,
        target: Pt,
        #[serde(default = "yes")]
        preserve_formulas: bool,
    },
    PutPiece {
        piece: Box<PatternPiece>,
        #[serde(default)]
        replace: bool,
    },
    DeletePiece {
        id: String,
    },
    SetAllowance {
        piece: String,
        edges: Vec<usize>,
        width: Value,
        corner: Corner,
    },
    AddAnnotation {
        piece: String,
        annotation: Annotation,
    },
    MarkRow {
        row: production::MarkRow,
    },
    SetChart {
        chart: SizeChart,
    },
    SetVariable {
        name: String,
        value: Value,
    },
    SetRule {
        id: String,
        rule: GradeRule,
    },
    AttachRule {
        piece: String,
        point: String,
        rule: Option<String>,
    },
    OffsetPath {
        id: String,
        source: String,
        distance: Value,
        side: Side,
    },
    /// Move a line endpoint to its intersection with another infinite line.
    ExtendTrim {
        id: String,
        source: String,
        target: String,
        end: usize,
    },
    OffsetPiece {
        piece: String,
        id: String,
        size: String,
        distance: f64,
    },
    JoinPieces {
        pieces: [String; 2],
        id: String,
        size: String,
    },
}
/// Side of a directed path, in the document's y-up coordinates.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    /// Counter-clockwise normal.
    Left,
    /// Clockwise normal.
    Right,
}
fn yes() -> bool {
    true
}
fn piece_mut<'a>(doc: &'a mut Document, id: &str) -> Result<&'a mut PatternPiece> {
    doc.pieces
        .iter_mut()
        .find(|p| p.id == id)
        .with_context(|| format!("Piece '{id}' was not found."))
}
fn shifted(value: &Value, offset: f64) -> Value {
    match value {
        Value::Number(n) => Value::Number(n + offset),
        Value::Formula(s) => Value::Formula(format!("({s})+({offset})")),
    }
}
fn symmetric_outline(outline: &[Pt], axis: [Pt; 2]) -> Result<bool> {
    let direction = geometry::unit(geometry::sub(axis[1], axis[0]))?;
    for q in outline {
        let local = geometry::sub(*q, axis[0]);
        let foot = geometry::add(
            axis[0],
            geometry::mul(direction, geometry::dot(local, direction)),
        );
        let reflected = geometry::sub(geometry::mul(foot, 2.), *q);
        let mut distance = f64::INFINITY;
        for i in 0..outline.len() {
            let a = outline[i];
            let b = outline[(i + 1) % outline.len()];
            let v = geometry::sub(b, a);
            let length2 = geometry::dot(v, v);
            let t = if length2 > 1e-18 {
                (geometry::dot(geometry::sub(reflected, a), v) / length2).clamp(0., 1.)
            } else {
                0.
            };
            distance = distance.min(geometry::distance(
                reflected,
                geometry::add(a, geometry::mul(v, t)),
            ));
        }
        if distance > 0.2 {
            return Ok(false);
        }
    }
    Ok(true)
}
/// Prepare one operation without final validation, for an atomic multi-operation batch.
pub fn prepare(doc: &Document, operation: &Operation) -> Result<Document> {
    let mut next = doc.clone();
    match operation {
        Operation::PutObject {
            id,
            object,
            replace,
        } => {
            ensure!(editor::valid_id(id), "Use a valid drawing ID.");
            ensure!(
                !next.drawing.contains_key(id) || *replace,
                "Drawing ID '{id}' already exists. Use replace to edit it."
            );
            next.drawing.insert(id.clone(), object.clone());
        }
        Operation::DeleteObject { id } => return editor::delete(doc, id),
        Operation::RenameObject { id, new_id } => return editor::rename(doc, id, new_id),
        Operation::MovePoint {
            id,
            size,
            target,
            preserve_formulas,
        } => {
            ensure!(
                target.iter().all(|x| x.is_finite()),
                "Use finite point coordinates."
            );
            let old = Evaluator::new(doc, size, TOLERANCE)?.point(id)?;
            if *preserve_formulas {
                let Some(Object::Point {
                    operation: PointOp::Absolute { x, y },
                }) = doc.drawing.get(id)
                else {
                    anyhow::bail!(
                        "This point has a construction constraint. Edit its inputs, or turn off formula preservation."
                    );
                };
                next.drawing.insert(
                    id.clone(),
                    Object::Point {
                        operation: PointOp::Absolute {
                            x: shifted(x, target[0] - old[0]),
                            y: shifted(y, target[1] - old[1]),
                        },
                    },
                );
            } else {
                return editor::move_point(doc, id, size, *target);
            }
        }
        Operation::PutPiece { piece, replace } => {
            ensure!(editor::valid_id(&piece.id), "Use a valid piece ID.");
            if let Some(p) = next.pieces.iter_mut().find(|p| p.id == piece.id) {
                ensure!(
                    *replace,
                    "Piece '{}' already exists. Use replace to edit it.",
                    piece.id
                );
                *p = piece.as_ref().clone();
            } else {
                next.pieces.push(piece.as_ref().clone());
            }
        }
        Operation::DeletePiece { id } => {
            ensure!(
                next.pieces.iter().any(|p| p.id == *id),
                "Piece '{id}' was not found."
            );
            next.pieces.retain(|p| p.id != *id);
            // Checks naming a deleted piece must not silently become stale.
            if let Some(checks) = next
                .meta
                .get_mut("block_seams")
                .and_then(|v| v.as_array_mut())
            {
                checks.retain(|c| c["a"]["piece"] != *id && c["b"]["piece"] != *id);
            }
        }
        Operation::SetAllowance {
            piece,
            edges,
            width,
            corner,
        } => {
            ensure!(!edges.is_empty(), "Choose at least one seam edge.");
            let p = piece_mut(&mut next, piece)?;
            for i in edges {
                let edge = p
                    .edges
                    .get_mut(*i)
                    .context("The seam edge number is out of range.")?;
                edge.allowance = width.clone();
                edge.corner = *corner;
            }
            let paths: Vec<_> = edges.iter().map(|i| p.edges[*i].object.clone()).collect();
            for path in paths {
                if let Some(Object::DerivedPath { recipe, .. }) = next.drawing.get_mut(&path) {
                    recipe.inherit_allowances = false;
                }
            }
        }
        Operation::AddAnnotation { piece, annotation } => piece_mut(&mut next, piece)?
            .annotations
            .push(annotation.clone()),
        Operation::MarkRow { row } => return production::add_mark_row(doc, row),
        Operation::SetChart { chart } => {
            chart.validate()?;
            next.sizes.clone_from(&chart.sizes);
            next.base_size.clone_from(&chart.base_size);
            next.measurements.clone_from(&chart.measurements);
            crate::blocks::refresh_layout(&mut next)?;
        }
        Operation::SetVariable { name, value } => {
            ensure!(
                editor::valid_id(name) && !name.contains('.'),
                "Use letters, digits and underscores for a variable name."
            );
            next.variables.insert(name.clone(), value.clone());
        }
        Operation::SetRule { id, rule } => {
            ensure!(!id.trim().is_empty(), "Give the grade rule an ID.");
            next.rules.insert(id.clone(), rule.clone());
        }
        Operation::AttachRule { piece, point, rule } => {
            if let Some(rule) = rule {
                piece_mut(&mut next, piece)?
                    .grade_points
                    .insert(point.clone(), rule.clone());
            } else {
                piece_mut(&mut next, piece)?.grade_points.remove(point);
            }
        }
        Operation::OffsetPath {
            id,
            source,
            distance,
            side,
        } => {
            ensure!(
                editor::valid_id(id) && !next.drawing.contains_key(id),
                "Choose an unused path ID."
            );
            for size in &doc.sizes {
                ensure!(
                    Evaluator::new(doc, size, TOLERANCE)?.value(distance)? >= 0.,
                    "Offset distance must be non-negative; choose left or right."
                );
            }
            let distance = match side {
                Side::Left => distance.clone(),
                Side::Right => match distance {
                    Value::Number(n) => Value::Number(-n),
                    Value::Formula(s) => Value::Formula(format!("-({s})")),
                },
            };
            next.drawing.insert(
                id.clone(),
                Object::OffsetPath {
                    source: source.clone(),
                    distance,
                },
            );
        }
        Operation::ExtendTrim {
            id,
            source,
            target,
            end,
        } => {
            ensure!(*end <= 1, "Choose endpoint 0 (start) or 1 (end).");
            ensure!(
                editor::valid_id(id) && !next.drawing.contains_key(id),
                "Choose an unused path ID."
            );
            let Some(Object::Line { from, to }) = doc.drawing.get(source) else {
                anyhow::bail!("Extend/trim currently needs two straight lines.");
            };
            let Some(Object::Line { from: a, to: b }) = doc.drawing.get(target) else {
                anyhow::bail!("Extend/trim currently needs two straight lines.");
            };
            let point = format!("{id}.intersection");
            ensure!(
                !next.drawing.contains_key(&point),
                "Intersection ID already exists."
            );
            next.drawing.insert(
                point.clone(),
                Object::Point {
                    operation: PointOp::LineIntersection {
                        a: [from.clone(), to.clone()],
                        b: [a.clone(), b.clone()],
                    },
                },
            );
            next.drawing.insert(
                id.clone(),
                Object::Line {
                    from: if *end == 0 {
                        point.clone()
                    } else {
                        from.clone()
                    },
                    to: if *end == 1 { point } else { to.clone() },
                },
            );
        }
        Operation::OffsetPiece {
            piece,
            id,
            size,
            distance,
        } => {
            ensure!(
                distance.is_finite() && *distance >= 0.,
                "Offset distance must be finite and non-negative."
            );
            ensure!(doc.sizes.contains(size), "Choose a chart size");
            return crate::derived::insert(
                doc,
                id,
                crate::derived::Recipe {
                    source: piece.clone(),
                    operation: crate::derived::Shape::Offset {
                        distance: (*distance).into(),
                    },
                    inherit_allowances: false,
                },
            );
        }
        Operation::JoinPieces { pieces, id, size } => {
            ensure!(doc.sizes.contains(size), "Choose a chart size");
            return crate::derived::insert(
                doc,
                id,
                crate::derived::Recipe {
                    source: pieces[0].clone(),
                    operation: crate::derived::Shape::Join {
                        other: pieces[1].clone(),
                    },
                    inherit_allowances: true,
                },
            );
        }
    }
    Ok(next)
}

/// Validate all geometry, including unused construction objects, in every size.
pub fn validate(doc: &Document) -> Result<()> {
    ensure!(
        doc.format == DOC_FORMAT,
        "Unsupported pattern document version. Expected {DOC_FORMAT}."
    );
    ensure!(
        !doc.sizes.is_empty() && doc.sizes.len() <= 100,
        "Choose 1 to 100 sizes."
    );
    ensure!(
        doc.sizes.contains(&doc.base_size),
        "The base size must be in the size list."
    );
    let unique: BTreeSet<_> = doc.sizes.iter().collect();
    ensure!(
        unique.len() == doc.sizes.len() && unique.iter().all(|s| !s.trim().is_empty()),
        "Size names must be non-empty and unique."
    );
    ensure!(
        doc.drawing.len() <= 20000 && doc.pieces.len() <= 1000,
        "This document exceeds the editing limit (20000 objects or 1000 pieces)."
    );
    ensure!(
        doc.drawing.keys().all(|id| editor::valid_id(id)),
        "A drawing ID is invalid."
    );
    let mut ids = BTreeSet::new();
    for p in &doc.pieces {
        ensure!(
            editor::valid_id(&p.id) && ids.insert(&p.id),
            "Piece IDs must be valid and unique."
        );
        ensure!(
            p.quantity.checked_add(p.mirror_quantity).is_some(),
            "Piece copy counts are too large."
        );
    }
    for (id, group) in &doc.groups {
        ensure!(
            group.objects.iter().all(|o| doc.drawing.contains_key(o)),
            "Drawing group '{id}' refers to a missing object."
        );
    }
    for (size, measurements) in &doc.measurements {
        ensure!(
            doc.sizes.contains(size),
            "Measurements refer to unknown size '{size}'."
        );
        ensure!(
            measurements.values().all(|n| n.is_finite()),
            "Measurements must be finite ({size})."
        );
    }
    editor::reference_order(doc, None)?;
    for (id, rule) in &doc.rules {
        ensure!(
            rule.deltas.keys().all(|s| doc.sizes.contains(s))
                && rule.deltas.values().flatten().all(|v| v.is_finite()),
            "Grade rule '{id}' has an unknown size or invalid movement."
        );
        if rule.mode == GradeMode::Relative {
            ensure!(
                rule.deltas
                    .get(&doc.base_size)
                    .is_none_or(|v| *v == [0., 0.]),
                "Base-size grade movement must be zero ({id})."
            );
        }
        for size in &doc.sizes {
            crate::grading::rule_delta(doc, rule, size)?;
        }
    }
    for size in &doc.sizes {
        let mut ev = Evaluator::new(doc, size, TOLERANCE)?;
        for name in doc.variables.keys() {
            ev.value(&Value::Formula(format!("v.{name}")))?;
        }
        for id in doc.drawing.keys() {
            if let Geometry::Path(path) = ev.object(id)? {
                let length = geometry::length(&path);
                ensure!(
                    length.is_finite() && length > 1e-9,
                    "Drawing path '{id}' has zero or invalid length in {size}."
                );
            }
        }
        for g in evaluate::grade(doc, size, TOLERANCE)? {
            ensure!(
                g.piece.buffer.is_finite() && g.piece.buffer >= 0.,
                "Piece buffer must be non-negative ({}).",
                g.piece.id
            );
            if let Some(line) = g.piece.grain {
                ensure!(
                    geometry::distance(line[0], line[1]) > 1e-6,
                    "Grain line must have a direction ({}).",
                    g.piece.id
                );
            }
            if let Some(line) = g.piece.fold_line {
                ensure!(
                    geometry::distance(line[0], line[1]) > 1e-6,
                    "Fold line must have a direction ({}).",
                    g.piece.id
                );
                ensure!(
                    symmetric_outline(&g.piece.outline, line)?,
                    "Piece '{}' is not symmetric about its fold line. Edit both sides or remove the fold line.",
                    g.piece.id
                );
            }
        }
    }
    Ok(())
}
/// Apply a batch, then validate it. The input is never changed.
pub fn edit(doc: &Document, operations: &[Operation]) -> Result<Document> {
    ensure!(
        operations.len() <= 1000,
        "Use at most 1000 edits in one transaction."
    );
    let mut next = doc.clone();
    for op in operations {
        next = prepare(&next, op)?;
    }
    validate(&next)?;
    Ok(next)
}
/// Save only valid versioned documents.
pub fn save(doc: &Document) -> Result<String> {
    validate(doc)?;
    Ok(serde_json::to_string_pretty(doc)?)
}
/// Load an offline snapshot and validate its version and geometry.
pub fn load(source: &str) -> Result<Document> {
    ensure!(
        source.len() <= 32 * 1024 * 1024,
        "Pattern JSON exceeds 32 MB."
    );
    let doc =
        serde_json::from_str(source).context("The pattern file is not valid document JSON.")?;
    validate(&doc)?;
    Ok(doc)
}

/// Requested construction measurement.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[allow(missing_docs)]
pub enum Measurement {
    Length {
        object: String,
    },
    Distance {
        a: String,
        b: String,
    },
    Angle {
        a: String,
        vertex: String,
        b: String,
    },
}
/// Measured value and coordinates for display.
#[derive(Debug, Clone, Serialize)]
#[allow(missing_docs)]
pub struct Measured {
    pub value: f64,
    pub unit: &'static str,
    pub points: Vec<Pt>,
}
/// Measure in Rust; the angle is the smaller included angle, 0 to 180 degrees.
pub fn measure(doc: &Document, size: &str, request: &Measurement) -> Result<Measured> {
    let mut e = Evaluator::new(doc, size, TOLERANCE)?;
    let (value, unit, points) = match request {
        Measurement::Length { object } => {
            let p = e.path(object)?;
            (geometry::length(&p), "mm", p)
        }
        Measurement::Distance { a, b } => {
            let p = vec![e.point(a)?, e.point(b)?];
            (geometry::distance(p[0], p[1]), "mm", p)
        }
        Measurement::Angle { a, vertex, b } => {
            let p = vec![e.point(a)?, e.point(vertex)?, e.point(b)?];
            let u = geometry::unit(geometry::sub(p[0], p[1]))?;
            let v = geometry::unit(geometry::sub(p[2], p[1]))?;
            (
                geometry::dot(u, v).clamp(-1., 1.).acos().to_degrees(),
                "deg",
                p,
            )
        }
    };
    Ok(Measured {
        value,
        unit,
        points,
    })
}
/// One nearby construction or containing piece.
#[derive(Debug, Clone, Serialize)]
#[allow(missing_docs)]
pub struct Hit {
    pub id: String,
    pub kind: &'static str,
    pub point: Pt,
    pub distance_mm: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub piece_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub along_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_kind: Option<&'static str>,
}
/// Nearby points, native path projections, graded seam edges and containing pieces.
pub fn hit_test(doc: &Document, size: &str, at: Pt, radius: f64) -> Result<Vec<Hit>> {
    ensure!(
        at.iter().all(|v| v.is_finite()) && radius.is_finite() && radius > 0.,
        "Use finite coordinates and a positive hit radius."
    );
    let mut e = Evaluator::new(doc, size, TOLERANCE)?;
    let mut hits = vec![];
    for id in doc.drawing.keys() {
        let (kind, point, along_mm, parameter, parameter_kind) = match e.object(id)? {
            Geometry::Point(p) => ("point", p, None, None, None),
            Geometry::Path(_) => {
                let curve = crate::paths::Curve::from_evaluator(doc, &mut e, id)?;
                let (p, t, along) = curve.nearest(at)?;
                (
                    "path",
                    p,
                    Some(along),
                    Some(t),
                    Some(if curve.native() {
                        "native"
                    } else {
                        "arc_length_fraction"
                    }),
                )
            }
        };
        let distance_mm = geometry::distance(at, point);
        if distance_mm <= radius {
            hits.push(Hit {
                id: id.clone(),
                kind,
                point,
                distance_mm,
                piece_id: None,
                edge: None,
                along_mm,
                parameter,
                parameter_kind,
            });
        }
    }
    let mut interiors = vec![];
    for (p, g) in doc
        .pieces
        .iter()
        .zip(evaluate::grade(doc, size, TOLERANCE)?)
    {
        for (index, path) in g.seam_edges.iter().enumerate() {
            let (mut point, mut along) = crate::paths::project(path, at)?;
            let mut parameter = along / geometry::length(path);
            let mut parameter_kind = "edge_length_fraction";
            if p.grade_points.is_empty() {
                let curve =
                    crate::paths::Curve::from_evaluator(doc, &mut e, &p.edges[index].object)?;
                if curve.native() {
                    let (q, t, _) = curve.nearest(at)?;
                    point = q;
                    along = crate::paths::project(path, q)?.1;
                    parameter = t;
                    parameter_kind = "native";
                }
            }
            let distance_mm = geometry::distance(at, point);
            if distance_mm <= radius {
                hits.push(Hit {
                    id: p.edges[index].object.clone(),
                    kind: "path",
                    point,
                    distance_mm,
                    piece_id: Some(p.id.clone()),
                    edge: Some(index),
                    along_mm: Some(along),
                    parameter: Some(parameter),
                    parameter_kind: Some(parameter_kind),
                });
            }
        }
        if beta_marker_core::style::contains(at, &g.piece.outline) {
            interiors.push(Hit {
                id: p.id.clone(),
                kind: "piece",
                point: at,
                distance_mm: 0.,
                piece_id: Some(p.id.clone()),
                edge: None,
                along_mm: None,
                parameter: None,
                parameter_kind: None,
            });
        }
    }
    hits.sort_by(|a, b| {
        (a.kind != "point")
            .cmp(&(b.kind != "point"))
            .then(a.distance_mm.total_cmp(&b.distance_mm))
            .then(a.id.cmp(&b.id))
            .then(a.edge.cmp(&b.edge))
    });
    hits.extend(interiors);
    Ok(hits)
}

/// Per-piece marker clearance override, keyed by document piece ID.
#[derive(Debug, Clone, Default, Deserialize)]
#[allow(missing_docs)]
pub struct PieceSettings {
    pub buffer: Option<f64>,
    pub block_margin: Option<f64>,
}
/// All settings needed for a production handoff.
#[derive(Debug, Clone, Deserialize)]
#[allow(missing_docs)]
pub struct JobSettings {
    pub fabric: beta_marker_core::Fabric,
    #[serde(default)]
    pub spacing: beta_marker_core::Spacing,
    pub ratio: BTreeMap<String, u32>,
    pub lay: Option<beta_marker_core::style::LaySettings>,
    #[serde(default)]
    pub repeat_bands: Vec<beta_marker_core::style::RepeatBand>,
    #[serde(default)]
    pub piece_settings: BTreeMap<String, PieceSettings>,
    #[serde(default)]
    pub matches: Vec<beta_marker_core::MatchRule>,
    /// Filter shell/lining/fusing for one fabric roll; required when materials differ.
    pub material: Option<String>,
}
/// Grade selected sizes, expand match references, and validate the complete marker job.
pub fn to_job(doc: &Document, settings: &JobSettings) -> Result<beta_marker_core::Job> {
    validate(doc)?;
    let mut selected = doc.clone();
    if let Some(material) = &settings.material {
        selected
            .pieces
            .retain(|p| p.material.as_deref() == Some(material));
    }
    ensure!(
        !selected.pieces.is_empty(),
        "No pieces use the selected fabric."
    );
    let materials: BTreeSet<_> = selected
        .pieces
        .iter()
        .filter(|p| p.quantity > 0 || p.mirror_quantity > 0)
        .map(|p| &p.material)
        .collect();
    ensure!(
        materials.len() <= 1,
        "This style uses more than one fabric. Choose shell, lining or fusing for this marker."
    );
    let mut job = crate::exchange::to_job(
        &selected,
        settings.fabric.clone(),
        settings.ratio.clone(),
        TOLERANCE,
    )?;
    job.spacing = settings.spacing;
    for id in settings.piece_settings.keys() {
        ensure!(
            doc.pieces.iter().any(|p| p.id == *id),
            "Unknown piece clearance setting '{id}'."
        );
    }
    for p in &mut job.pieces {
        let suffix = format!(
            "-{}",
            p.size.as_deref().context("Graded piece has no size.")?
        );
        let source_id =
            p.id.strip_suffix(&suffix)
                .context("Graded piece ID has no size suffix.")?;
        let source = selected
            .pieces
            .iter()
            .find(|s| s.id == source_id)
            .context("Graded piece source was not found.")?;
        if let Some(s) = settings.piece_settings.get(&source.id) {
            if let Some(buffer) = s.buffer {
                p.buffer = buffer;
            }
            p.block_margin = s.block_margin;
        }
        if settings.fabric.layout != beta_marker_core::Layout::Open
            && (p.quantity > 0 || p.mirror_quantity > 0)
            && let Some(axis) = p.fold_line
        {
            // Complete only cut marks on this handoff clone. The source document stays editable.
            for mark in p.marks.clone().iter().filter(|m| {
                matches!(
                    m.kind,
                    beta_marker_core::MarkKind::Notch | beta_marker_core::MarkKind::Drill
                )
            }) {
                let reflected = tools::mirror(&mark.points, axis[0], axis[1])?;
                if !p.marks.iter().any(|other| {
                    other.kind == mark.kind
                        && other.text == mark.text
                        && other.points.len() == reflected.len()
                        && reflected.iter().all(|q| {
                            other
                                .points
                                .iter()
                                .any(|r| geometry::distance(*q, *r) <= 0.2)
                        })
                        && other
                            .points
                            .iter()
                            .all(|q| reflected.iter().any(|r| geometry::distance(*q, *r) <= 0.2))
                }) {
                    let mut twin = mark.clone();
                    twin.points = reflected;
                    p.marks.push(twin);
                }
            }
            for mark in p.marks.iter().filter(|m| {
                matches!(
                    m.kind,
                    beta_marker_core::MarkKind::Notch | beta_marker_core::MarkKind::Drill
                )
            }) {
                let reflected = tools::mirror(&mark.points, axis[0], axis[1])?;
                ensure!(
                    p.marks.iter().any(|other| other.kind == mark.kind
                        && other.text == mark.text
                        && other.points.len() == reflected.len()
                        && reflected.iter().all(|q| other
                            .points
                            .iter()
                            .any(|r| geometry::distance(*q, *r) <= 0.2))
                        && other
                            .points
                            .iter()
                            .all(|q| reflected.iter().any(|r| geometry::distance(*q, *r) <= 0.2))),
                    "Piece '{}' has a cut mark on only one side of its fold. Add the matching notch or drill on the other side, or remove the fold line.",
                    p.id
                );
            }
        }
    }
    for rule in &settings.matches {
        for (size, count) in &settings.ratio {
            if *count == 0 {
                continue;
            }
            let mut r = rule.clone();
            r.a.piece_id = format!("{}-{size}", r.a.piece_id);
            if let Some(b) = &mut r.b {
                b.piece_id = format!("{}-{size}", b.piece_id);
            }
            job.match_rules.push(r);
        }
    }
    if let Some(lay) = &settings.lay {
        job.meta
            .insert("beta_style_lay".into(), serde_json::to_value(lay)?);
    }
    job.meta.insert(
        "beta_repeat_bands".into(),
        serde_json::to_value(&settings.repeat_bands)?,
    );
    let errors = beta_marker_core::check_job(&job);
    ensure!(
        errors.is_empty(),
        "{}",
        errors
            .iter()
            .map(|e| e.message.as_str())
            .collect::<Vec<_>>()
            .join("; ")
    );
    Ok(job)
}
