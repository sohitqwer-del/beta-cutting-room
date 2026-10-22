//! Entity expansion, unit resolution and garment conventions.

use crate::{
    ImportOptions, ImportReport, Units, curves,
    parser::{Drawing, Pair, Record},
};
use anyhow::{Context, Result, ensure};
use beta_marker_core::{
    Mark, MarkKind, MatchPoint, Piece, Pt, RotationRule,
    geometry::{area, bbox},
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, f64::consts::TAU};

const SNAP: f64 = 0.05;

#[derive(Clone, Copy)]
struct Transform([f64; 6]);

impl Transform {
    fn scale(s: f64) -> Self {
        Self([s, 0.0, 0.0, s, 0.0, 0.0])
    }
    fn point(self, [x, y]: Pt) -> Pt {
        let [a, b, c, d, e, f] = self.0;
        [a * x + c * y + e, b * x + d * y + f]
    }
    fn compose(self, rhs: Self) -> Self {
        let [a, b, c, d, e, f] = self.0;
        let [g, h, i, j, k, l] = rhs.0;
        Self([
            a * g + c * h,
            b * g + d * h,
            a * i + c * j,
            b * i + d * j,
            a * k + c * l + e,
            b * k + d * l + f,
        ])
    }
    fn tolerance(self) -> f64 {
        let [a, b, c, d, _, _] = self.0;
        curves::TOLERANCE / (a.hypot(b) + c.hypot(d)).max(1e-12)
    }
}

#[derive(Clone)]
struct Shape {
    layer: String,
    points: Vec<Pt>,
    closed: bool,
    text: Option<String>,
    center: Option<Pt>,
    mark_text: Option<Option<String>>,
}

#[derive(Default)]
struct Expansion {
    points: usize,
    instances: usize,
    stack: Vec<String>,
}

fn expand(
    d: &Drawing,
    records: &[Record],
    transform: Transform,
    inherited: &str,
    shapes: &mut Vec<Shape>,
    warnings: &mut Vec<String>,
    budget: &mut Expansion,
) -> Result<()> {
    ensure!(budget.stack.len() < 32, "INSERT nesting exceeds 32 levels");
    for r in records {
        let layer = if r.layer() == "0" {
            inherited
        } else {
            r.layer()
        };
        ensure!(
            r.num(210, 0.0)? == 0.0 && r.num(220, 0.0)? == 0.0 && r.num(230, 1.0)? == 1.0,
            "{} uses a non-XY extrusion; flatten the drawing before import",
            r.kind
        );
        if r.kind == "INSERT" {
            budget.instances += 1;
            ensure!(
                budget.instances < 100_000,
                "DXF exceeds INSERT expansion limit"
            );
            let name = r.value(2).context("INSERT missing block name")?;
            let lookup = name.to_ascii_uppercase();
            ensure!(
                !budget.stack.contains(&lookup),
                "cyclic INSERT of BLOCK {name}"
            );
            let block = d
                .blocks
                .get(&lookup)
                .with_context(|| format!("INSERT refers to missing BLOCK {name}"))?;
            ensure!(
                r.int(70, 1)? == 1 && r.int(71, 1)? == 1,
                "array INSERT is unsupported; explode it before import"
            );
            let sx = r.num(41, 1.0)?;
            let sy = r.num(42, 1.0)?;
            ensure!(sx != 0.0 && sy != 0.0, "INSERT scale must not be zero");
            let (sin, cos) = r.num(50, 0.0)?.to_radians().sin_cos();
            let a = cos * sx;
            let b = sin * sx;
            let c = -sin * sy;
            let e = cos * sy;
            let t = Transform([
                a,
                b,
                c,
                e,
                r.num(10, 0.0)? - a * block.base[0] - c * block.base[1],
                r.num(20, 0.0)? - b * block.base[0] - e * block.base[1],
            ]);
            budget.stack.push(lookup);
            let entities: Vec<_> = block
                .entities
                .iter()
                .filter(|entity| {
                    entity.kind != "ATTDEF"
                        || !r.vertices.iter().any(|attr| {
                            attr.value(2)
                                .zip(entity.value(2))
                                .is_some_and(|(a, b)| a.eq_ignore_ascii_case(b))
                        })
                })
                .cloned()
                .collect();
            expand(
                d,
                &entities,
                transform.compose(t),
                layer,
                shapes,
                warnings,
                budget,
            )?;
            // INSERT ATTRIB positions are in the containing coordinate system,
            // unlike ATTDEF positions inside the block definition.
            for attr in &r.vertices {
                expand(
                    d,
                    std::slice::from_ref(attr),
                    transform,
                    layer,
                    shapes,
                    warnings,
                    budget,
                )?;
            }
            budget.stack.pop();
            continue;
        }
        let tol = transform.tolerance();
        let mut closed = false;
        let mut center = None;
        let mut text = None;
        let points = match r.kind.as_str() {
            "LINE" => vec![r.point(10)?, r.point(11)?],
            "POINT" => vec![r.point(10)?],
            "TEXT" | "MTEXT" | "ATTRIB" | "ATTDEF" => {
                let value = if matches!(r.kind.as_str(), "ATTRIB" | "ATTDEF") {
                    r.value(1).unwrap_or("").to_string()
                } else {
                    r.text()
                };
                text = Some(if matches!(r.kind.as_str(), "ATTRIB" | "ATTDEF") {
                    format!("{}: {value}", r.value(2).unwrap_or("Attribute"))
                } else {
                    value
                });
                vec![[r.num(10, 0.0)?, r.num(20, 0.0)?]]
            }
            "POLYLINE" => {
                let flags = r.int(70, 0)?;
                ensure!(
                    flags & (8 | 16 | 64) == 0,
                    "3D/mesh POLYLINE is unsupported; flatten it before import"
                );
                closed = flags & 1 != 0;
                let vertices: Result<Vec<_>> = r
                    .vertices
                    .iter()
                    .map(|v| Ok((v.point(10)?, v.num(42, 0.0)?)))
                    .collect();
                curves::polyline(&vertices?, closed, tol)?
            }
            "LWPOLYLINE" => {
                closed = r.int(70, 0)? & 1 != 0;
                let mut vertices = vec![];
                let mut current: Option<(f64, Option<f64>, f64)> = None;
                for p in &r.fields {
                    if p.code == 10 {
                        if let Some((x, y, b)) = current.take() {
                            vertices.push(([x, y.context("LWPOLYLINE vertex missing y")?], b));
                        }
                        current = Some((number(&p.value)?, None, 0.0));
                    } else if let Some((_, y, b)) = current.as_mut() {
                        match p.code {
                            20 => *y = Some(number(&p.value)?),
                            42 => *b = number(&p.value)?,
                            _ => {}
                        }
                    }
                }
                if let Some((x, y, b)) = current {
                    vertices.push(([x, y.context("LWPOLYLINE vertex missing y")?], b));
                }
                if r.value(90).is_some() {
                    ensure!(
                        r.int(90, 0)? as usize == vertices.len(),
                        "LWPOLYLINE vertex count mismatch"
                    );
                }
                curves::polyline(&vertices, closed, tol)?
            }
            "ARC" => {
                let a = r.required(50)?.to_radians();
                let b = r.required(51)?.to_radians();
                let sweep = (b - a).rem_euclid(TAU);
                curves::arc(
                    r.point(10)?,
                    r.required(40)?,
                    a,
                    if sweep < 1e-12 { TAU } else { sweep },
                    tol,
                )?
            }
            "CIRCLE" => {
                closed = true;
                let c = r.point(10)?;
                center = Some(transform.point(c));
                // A drill stores its centre, so it need not be tessellated.
                let radius = r.required(40)?;
                ensure!(radius > 0.0, "CIRCLE radius must be positive");
                if layer == "13" {
                    vec![c]
                } else {
                    curves::circle(c, radius, tol)?
                }
            }
            "SPLINE" => {
                closed = r.int(70, 0)? & 3 != 0;
                let controls = repeated_points(r, 10)?;
                if controls.is_empty() {
                    warn(warnings, "SPLINE with fit points only approximated by a polyline; no tangent reconstruction".into());
                    let fit = repeated_points(r, 11)?;
                    ensure!(fit.len() >= 2, "SPLINE needs control points or fit points");
                    fit
                } else {
                    let values = |code| {
                        r.fields
                            .iter()
                            .filter(|p| p.code == code)
                            .map(|p| number(&p.value))
                            .collect::<Result<Vec<_>>>()
                    };
                    curves::spline(
                        &controls,
                        &values(41)?,
                        &values(40)?,
                        r.int(71, 3)?.try_into().context("negative SPLINE degree")?,
                        tol,
                    )?
                }
            }
            _ => {
                warn(warnings, format!("Unsupported entity {} ignored", r.kind));
                continue;
            }
        };
        let points: Vec<Pt> = points.into_iter().map(|p| transform.point(p)).collect();
        closed |= points.len() > 2 && curves::distance(points[0], points[points.len() - 1]) <= SNAP;
        ensure!(
            points
                .iter()
                .flatten()
                .all(|v| v.is_finite() && v.abs() <= 1e12),
            "transformed coordinates out of range"
        );
        budget.points += points.len();
        ensure!(
            budget.points <= 2_000_000,
            "expanded DXF exceeds 2 million points"
        );
        let mark_text = if r.value(1001) == Some("BETA_MARK") {
            let json: String = r
                .fields
                .iter()
                .filter(|p| p.code == 1000)
                .map(|p| p.value.as_str())
                .collect();
            Some(serde_json::from_str(&json).context("invalid BETA_MARK text metadata")?)
        } else {
            None
        };
        shapes.push(Shape {
            layer: layer.to_string(),
            points,
            closed,
            text,
            center,
            mark_text,
        });
    }
    Ok(())
}

fn number(s: &str) -> Result<f64> {
    let n: f64 = s.parse().context("invalid DXF number")?;
    ensure!(n.is_finite() && n.abs() <= 1e12, "DXF number out of range");
    Ok(n)
}

fn repeated_points(r: &Record, xcode: i32) -> Result<Vec<Pt>> {
    let xs: Result<Vec<_>> = r
        .fields
        .iter()
        .filter(|p| p.code == xcode)
        .map(|p| number(&p.value))
        .collect();
    let ys: Result<Vec<_>> = r
        .fields
        .iter()
        .filter(|p| p.code == xcode + 10)
        .map(|p| number(&p.value))
        .collect();
    let (xs, ys) = (xs?, ys?);
    ensure!(xs.len() == ys.len(), "{} coordinate count mismatch", r.kind);
    Ok(xs.into_iter().zip(ys).map(|(x, y)| [x, y]).collect())
}

fn warn(warnings: &mut Vec<String>, message: String) {
    if !warnings.contains(&message) {
        warnings.push(message);
    }
}

fn field<'a>(texts: impl Iterator<Item = &'a str>, key: &str) -> Option<String> {
    for text in texts {
        for line in text.lines() {
            if let Some((label, value)) = line.split_once(':')
                && label.trim().replace('_', " ").eq_ignore_ascii_case(key)
                && !value.trim().is_empty()
            {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

fn metadata(shapes: &[Shape], key: &str) -> Option<String> {
    field(
        shapes
            .iter()
            .filter(|s| s.mark_text.is_none())
            .filter_map(|s| s.text.as_deref()),
        key,
    )
}

/// Private extension used only for lossless BETA-to-BETA round trips. Other CAD
/// readers may ignore this TEXT entity and still read standard AAMA geometry.
#[derive(Serialize, Deserialize)]
pub(crate) struct BetaFields {
    pub id: String,
    pub style: Option<String>,
    pub quantity: u32,
    pub mirror_quantity: u32,
    pub rotation: RotationRule,
    pub buffer: f64,
}

pub(crate) fn import(
    mut d: Drawing,
    opts: &ImportOptions,
    grades: Option<&crate::GradeTable>,
) -> Result<ImportReport> {
    let mut warnings = vec![];
    // Some garment exporters store the pattern library only in BLOCKS and put
    // no INSERTs in model space. Use root piece blocks once, never nested helper
    // blocks or reserved model/paper-space blocks. Explicit INSERTs remain authoritative.
    if !d.entities.iter().any(|r| r.kind == "INSERT")
        && !d.entities.iter().any(|r| {
            matches!(
                r.kind.as_str(),
                "LINE" | "POLYLINE" | "LWPOLYLINE" | "ARC" | "CIRCLE" | "SPLINE" | "ELLIPSE"
            )
        })
    {
        let nested: BTreeSet<_> = d
            .blocks
            .values()
            .flat_map(|b| &b.entities)
            .filter(|r| r.kind == "INSERT")
            .filter_map(|r| r.value(2).map(str::to_ascii_uppercase))
            .collect();
        let roots: Vec<_> = d
            .blocks
            .keys()
            .filter(|name| !name.starts_with('*') && !nested.contains(*name))
            .cloned()
            .collect();
        for name in roots {
            d.entities.push(Record {
                kind: "INSERT".into(),
                fields: vec![Pair {
                    code: 2,
                    value: name,
                }],
                vertices: vec![],
            });
        }
        if d.entities.iter().any(|r| r.kind == "INSERT") {
            warnings.push("No model-space INSERTs: imported root BLOCK definitions once each. Check piece quantities.".into());
        }
    }
    if grades.is_some() {
        check_grade_axes(&d, &d.entities, 0, &mut BTreeSet::new())?;
    }
    let file_texts: Vec<String> = d
        .entities
        .iter()
        .filter(|r| matches!(r.kind.as_str(), "TEXT" | "MTEXT"))
        .map(Record::text)
        .collect();
    let file_field = |key| field(file_texts.iter().map(String::as_str), key);
    let mut units = opts.units;
    if units == Units::Auto {
        if let Some(values) = d.header.get("$INSUNITS")
            && let Some(p) = values.iter().find(|p| p.code == 70)
        {
            units = match p.value.parse::<i32>().context("invalid $INSUNITS")? {
                0 => Units::Auto,
                1 => Units::Inch,
                4 => Units::Mm,
                5 => Units::Cm,
                other => {
                    warn(
                        &mut warnings,
                        format!("Unsupported $INSUNITS {other}; trying AAMA Units text"),
                    );
                    Units::Auto
                }
            };
        }
        let text_units = file_field("Units").and_then(|v| match v.to_ascii_uppercase().as_str() {
            "ENGLISH" | "INCH" | "INCHES" | "IN" => Some(Units::Inch),
            "METRIC" | "MM" | "MILLIMETERS" | "MILLIMETRES" => Some(Units::Mm),
            "CM" | "CENTIMETERS" | "CENTIMETRES" => Some(Units::Cm),
            _ => None,
        });
        if units == Units::Auto {
            units = text_units.unwrap_or(Units::Auto);
        } else if text_units.is_some_and(|u| u != units) {
            warn(
                &mut warnings,
                "Header units override conflicting AAMA Units text".into(),
            );
        }
        if units == Units::Auto
            && let Some(value) = d
                .header
                .get("$MEASUREMENT")
                .and_then(|p| p.iter().find(|p| p.code == 70))
        {
            units = match value.value.as_str() {
                "0" => Units::Inch,
                "1" => Units::Mm,
                _ => Units::Auto,
            };
            if units != Units::Auto {
                warn(&mut warnings, "Units inferred from $MEASUREMENT (drawing convention, not an explicit length unit)".into());
            }
        }
    } else {
        warn(
            &mut warnings,
            format!("Source units explicitly overridden to {units:?}"),
        );
    }
    // Each top-level INSERT represents one piece. Nested inserts are geometry.
    let mut batches = vec![];
    let mut loose = vec![];
    let mut budget = Expansion::default();
    for r in &d.entities {
        let mut shapes = vec![];
        expand(
            &d,
            std::slice::from_ref(r),
            Transform::scale(units.scale()),
            "0",
            &mut shapes,
            &mut warnings,
            &mut budget,
        )?;
        if r.kind == "INSERT" {
            batches.push((r.value(2).unwrap_or("Piece").to_string(), shapes));
        } else {
            loose.extend(shapes);
        }
    }
    if units == Units::Auto {
        let mut longest: f64 = 0.0;
        for shapes in batches
            .iter()
            .map(|b| b.1.as_slice())
            .chain(std::iter::once(loose.as_slice()))
        {
            let boundary_points: Vec<_> = shapes
                .iter()
                .filter(|s| s.layer == "1" && s.text.is_none())
                .flat_map(|s| s.points.iter().copied())
                .collect();
            if let Some(b) = bbox(&boundary_points) {
                longest = longest.max((b[2] - b[0]).max(b[3] - b[1]));
                continue;
            }
            for s in shapes.iter().filter(|s| s.text.is_none()) {
                if let Some(b) = bbox(&s.points) {
                    longest = longest.max((b[2] - b[0]).max(b[3] - b[1]));
                }
            }
        }
        units = if longest > 0.0 && longest < 80.0 {
            Units::Inch
        } else {
            Units::Mm
        };
        warn(
            &mut warnings,
            format!(
                "No source units: size heuristic assumes {units:?} (largest entity {longest:.2} units); verify the scale"
            ),
        );
        // Resample in the resolved units, so curve tolerance remains in mm.
        batches.clear();
        loose.clear();
        budget = Expansion::default();
        for r in &d.entities {
            let mut shapes = vec![];
            expand(
                &d,
                std::slice::from_ref(r),
                Transform::scale(units.scale()),
                "0",
                &mut shapes,
                &mut warnings,
                &mut budget,
            )?;
            if r.kind == "INSERT" {
                batches.push((r.value(2).unwrap_or("Piece").into(), shapes));
            } else {
                loose.extend(shapes);
            }
        }
    }
    let style = file_field("Style Name");
    let sample = file_field("Sample Size");
    let aama = batches
        .iter()
        .any(|(_, ss)| ss.iter().any(|s| s.layer == "1"))
        || batches
            .iter()
            .any(|(_, ss)| metadata(ss, "Piece Name").is_some());
    let astm = d
        .comments
        .iter()
        .chain(file_texts.iter())
        .any(|s| s.to_ascii_uppercase().contains("ASTM"));
    let dialect = if aama {
        if astm { "astm" } else { "aama" }
    } else {
        "generic"
    };
    let mut pieces = vec![];
    // Blocks per `Pairing` outcome.
    let mut pairs = [0usize; 3];
    if aama {
        for (block, shapes) in batches {
            let piece = garment(
                &block,
                &shapes,
                style.clone(),
                sample.clone(),
                opts,
                &mut warnings,
                grades,
            )?;
            if let Some((p, pairing)) = piece {
                pairs[pairing as usize] += 1;
                if let Some(table) = grades {
                    pieces.extend(crate::grading::generate(&p, table, &mut warnings)?);
                } else {
                    pieces.push(p);
                }
            }
        }
        if loose.iter().any(|s| s.layer == "1" && s.text.is_none()) {
            warn(
                &mut warnings,
                "Loose layer-1 geometry outside piece BLOCKs ignored".into(),
            );
        }
    } else {
        let shapes: Vec<_> = batches
            .into_iter()
            .flat_map(|(_, s)| s)
            .chain(loose)
            .collect();
        pieces = generic(&shapes, style, &mut warnings);
        ensure!(
            grades.is_none(),
            "experimental grading requires one garment BLOCK per base piece"
        );
    }
    ensure!(
        !pieces.is_empty(),
        "DXF contains no usable pattern outlines{}",
        if warnings.is_empty() {
            String::new()
        } else {
            format!(": {}", warnings.join("; "))
        }
    );
    let mut sizes = vec![];
    let mut ids = BTreeSet::new();
    for p in &mut pieces {
        let original = p.id.clone();
        let mut suffix = 2;
        while !ids.insert(p.id.clone()) {
            p.id = format!("{original}-{suffix}");
            suffix += 1;
        }
        if let Some(size) = &p.size
            && !sizes.contains(size)
        {
            sizes.push(size.clone());
        }
    }
    let [_, split, kept] = pairs;
    if split > 0 {
        let kept = if kept > 0 {
            format!(" {kept} symmetric pieces with an even Quantity keep same-hand copies.")
        } else {
            String::new()
        };
        warnings.push(format!(
            "Pairs: {split} asymmetric pieces with an even Quantity N were split into N/2 normal and N/2 mirrored (left/right) copies.{kept} Check left/right pieces before cutting, or import with pairs off."
        ));
    }
    Ok(ImportReport {
        pairs_assumed: split > 0,
        pieces,
        sizes,
        dialect: dialect.into(),
        units,
        warnings,
    })
}

fn check_grade_axes(
    d: &Drawing,
    records: &[Record],
    depth: usize,
    checked: &mut BTreeSet<String>,
) -> Result<()> {
    ensure!(depth < 32, "grade INSERT nesting exceeds 32 levels");
    for record in records.iter().filter(|r| r.kind == "INSERT") {
        ensure!(
            (record.num(41, 1.0)? - 1.0).abs() < 1e-9
                && (record.num(42, 1.0)? - 1.0).abs() < 1e-9
                && record.num(50, 0.0)?.rem_euclid(360.0).abs() < 1e-9,
            "experimental grading requires unrotated, unscaled INSERTs; expand transforms and align rule vectors before import"
        );
        if let Some(name) = record.value(2).map(str::to_ascii_uppercase)
            && checked.insert(name.clone())
            && let Some(block) = d.blocks.get(&name)
        {
            check_grade_axes(d, &block.entities, depth + 1, checked)?;
        }
    }
    Ok(())
}

/// What the importer did with a piece's Quantity.
#[derive(Clone, Copy)]
enum Pairing {
    Unchanged,
    /// Split into left/right copies.
    Split,
    /// Even Quantity of a symmetric piece, kept as same-hand copies.
    KeptSymmetric,
}

fn garment(
    block: &str,
    shapes: &[Shape],
    style: Option<String>,
    sample: Option<String>,
    opts: &ImportOptions,
    warnings: &mut Vec<String>,
    grades: Option<&crate::GradeTable>,
) -> Result<Option<(Piece, Pairing)>> {
    let name = metadata(shapes, "Piece Name").unwrap_or_else(|| {
        warn(
            warnings,
            format!("{block}: no Piece Name; using the block name"),
        );
        block.to_string()
    });
    let boundaries: Vec<_> = shapes
        .iter()
        .filter(|s| s.layer == "1" && s.text.is_none() && s.points.len() >= 2)
        .cloned()
        .collect();
    if boundaries.is_empty() {
        warn(warnings, format!("{block}: no layer-1 boundary; skipped"));
        return Ok(None);
    }
    let mut loops = join(&boundaries);
    loops.sort_by(|a, b| area(&b.0).total_cmp(&area(&a.0)));
    let Some((mut outline, closed)) = loops.first().cloned() else {
        return Ok(None);
    };
    if outline.len() < 3 || area(&outline) < 1e-10 {
        warn(warnings, format!("{block}: degenerate boundary; skipped"));
        return Ok(None);
    }
    if !closed {
        warn(
            warnings,
            format!(
                "{block}: layer-1 boundary is not closed within {SNAP} mm; closed by a straight segment"
            ),
        );
    }
    let mut marks = vec![];
    for (mut points, closed) in loops.into_iter().skip(1) {
        warn(
            warnings,
            format!("{block}: additional disconnected layer-1 loop retained as an internal line"),
        );
        if closed && !points.is_empty() {
            points.push(points[0]);
        }
        marks.push(Mark {
            kind: MarkKind::Line,
            points,
            text: None,
        });
    }
    let mut grain: Option<[Pt; 2]> = None;
    let mut mirror = None;
    let mut match_points = vec![];
    for s in shapes {
        let match_name = s.text.as_deref().unwrap_or(&s.layer);
        let upper = match_name.to_ascii_uppercase();
        if ["MATCH", "PLAID", "STRIPE"]
            .iter()
            .any(|k| upper.starts_with(k))
            && let Some(&point) = s.points.first()
        {
            let base = s.text.as_deref().unwrap_or(&s.layer).trim();
            let mut id = base.to_string();
            let mut n = 2;
            while match_points.iter().any(|p: &MatchPoint| p.id == id) {
                id = format!("{base}-{n}");
                n += 1;
            }
            match_points.push(MatchPoint { id, point });
        }
        if let Some(text) = &s.text {
            if let Some(original) = &s.mark_text {
                marks.push(Mark {
                    kind: MarkKind::Text,
                    points: s.points.clone(),
                    text: original.clone(),
                });
                continue;
            }
            let keys = [
                "piece name",
                "size",
                "quantity",
                "material",
                "style name",
                "beta",
            ];
            for line in text.lines() {
                let consumed = line.split_once(':').is_some_and(|(k, _)| {
                    keys.iter().any(|key| k.trim().eq_ignore_ascii_case(key))
                });
                if !consumed {
                    marks.push(Mark {
                        kind: MarkKind::Text,
                        points: s.points.clone(),
                        text: Some(line.into()),
                    });
                }
            }
            continue;
        }
        let ends = || s.points.first().zip(s.points.last()).map(|(a, b)| [*a, *b]);
        match s.layer.as_str() {
            "7" => {
                if let Some(g) = ends()
                    && curves::distance(g[0], g[1]) > 1e-9
                    && grain.is_none_or(|old| {
                        curves::distance(g[0], g[1]) > curves::distance(old[0], old[1])
                    })
                {
                    grain = Some(g);
                }
            }
            "6" => {
                if let Some(g) = ends() {
                    mirror = Some(g);
                }
            }
            "4" => {
                let mut points = s.points.clone();
                if s.mark_text.is_none()
                    && points.len() == 2
                    && boundary_distance(&outline, points[1])
                        < boundary_distance(&outline, points[0])
                {
                    points.reverse();
                }
                marks.push(Mark {
                    kind: MarkKind::Notch,
                    points,
                    text: None,
                });
            }
            "2" | "3" => marks.push(Mark {
                kind: MarkKind::Line,
                points: s.points.clone(),
                text: None,
            }),
            "13" => {
                if let Some(p) = s.center.or_else(|| s.points.first().copied()) {
                    marks.push(Mark {
                        kind: MarkKind::Drill,
                        points: vec![p],
                        text: None,
                    });
                }
            }
            "8" | "11" | "14" => marks.push(Mark {
                kind: MarkKind::Line,
                points: s.points.clone(),
                text: None,
            }),
            _ => {}
        }
        if let Some(original) = &s.mark_text
            && let Some(mark) = marks.last_mut()
        {
            mark.text = original.clone();
        }
    }
    if let Some(axis) = mirror {
        unfold(&mut outline, &mut marks, axis)
            .with_context(|| format!("{block}: invalid half-pattern mirror line"))?;
        warn(
            warnings,
            format!("{block}: half pattern unfolded around layer-6 mirror line"),
        );
    }
    if grain.is_none() {
        warn(
            warnings,
            format!("{block}: no grain line; piece x axis is the grain"),
        );
    }
    if metadata(shapes, "Rule Table").is_some() && grades.is_none() {
        warn(
            warnings,
            format!(
                "{block}: grade rule table found; grading is not applied; only supplied base/size geometry imported"
            ),
        );
    }
    let size = metadata(shapes, "Size").or_else(|| {
        warn(
            warnings,
            format!(
                "{block}: no Size; {}",
                if sample.is_some() {
                    "using Sample Size"
                } else {
                    "imported as an unsized base piece"
                }
            ),
        );
        sample
    });
    let style = metadata(shapes, "Style Name").or(style);
    let quantity = metadata(shapes, "Quantity").map_or_else(
        || {
            warn(
                warnings,
                format!("{block}: no Quantity; assuming one normal copy"),
            );
            Ok(1)
        },
        |s| s.parse::<u32>().context("invalid Quantity text"),
    )?;
    // A symmetric piece has no left and right versions: keep its copies same-hand.
    let mut pairing = Pairing::Unchanged;
    if opts.pairs && quantity > 0 && quantity.is_multiple_of(2) {
        pairing = if mirror.is_none() && !beta_marker_core::rules::mirror_symmetric(&outline, grain, true) {
            Pairing::Split
        } else {
            Pairing::KeptSymmetric
        };
    }
    // One summary warning is added by `import`.
    let (mut quantity, mut mirror_quantity) = if matches!(pairing, Pairing::Split) {
        (quantity / 2, quantity / 2)
    } else {
        (quantity, 0)
    };
    let mut id = format!(
        "{}-{}-{}",
        sanitize(style.as_deref().unwrap_or("STYLE")),
        sanitize(&name),
        sanitize(size.as_deref().unwrap_or("BASE"))
    );
    let mut style = style;
    let mut rotation = RotationRule::Inherit;
    let mut buffer = 0.0;
    if let Some(beta) = metadata(shapes, "BETA") {
        let fields: BetaFields =
            serde_json::from_str(&beta).context("invalid BETA round-trip metadata")?;
        ensure!(
            fields.buffer.is_finite() && fields.buffer >= 0.0,
            "invalid BETA buffer"
        );
        id = fields.id;
        style = fields.style;
        // BETA's own export records the exact hands; nothing is assumed.
        pairing = Pairing::Unchanged;
        quantity = fields.quantity;
        mirror_quantity = fields.mirror_quantity;
        rotation = fields.rotation;
        buffer = fields.buffer;
    }
    if mirror_quantity == 0
        && metadata(shapes, "BETA").is_none()
        && shapes
            .iter()
            .filter(|s| s.mark_text.is_none())
            .filter_map(|s| s.text.as_deref())
            .any(|text| text.to_ascii_lowercase().contains("mirrored pair"))
    {
        warn(
            warnings,
            format!(
                "{block}: mirrored-pair note is text only; Quantity {quantity} remains normal copies. Supply an explicit pairing rule before cutting."
            ),
        );
    }
    let piece = Piece {
        id,
        name,
        style,
        size,
        material: metadata(shapes, "Material")
            .filter(|s| !s.is_empty())
            .or_else(|| metadata(shapes, "Category").filter(|s| !s.is_empty())),
        outline,
        grain,
        quantity,
        mirror_quantity,
        rotation,
        buffer,
        marks,
        fold_line: mirror,
        match_points,
        ..Default::default()
    };
    Ok(Some((piece, pairing)))
}

fn sanitize(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_uppercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let result = out.trim_matches('-');
    if result.is_empty() {
        "UNKNOWN".into()
    } else {
        result.into()
    }
}

fn clean(points: &mut Vec<Pt>, closed: bool) -> bool {
    points.dedup_by(|a, b| curves::distance(*a, *b) < 1e-9);
    let joined = points.len() > 2 && curves::distance(points[0], points[points.len() - 1]) <= SNAP;
    if joined {
        points.pop();
    }
    closed || joined
}

fn join(shapes: &[Shape]) -> Vec<(Vec<Pt>, bool)> {
    let mut remaining: Vec<_> = shapes
        .iter()
        .map(|s| (s.points.clone(), s.closed))
        .collect();
    let mut result = vec![];
    while let Some((mut points, mut closed)) = remaining.pop() {
        closed = clean(&mut points, closed);
        while !closed {
            let a = points[0];
            let b = points[points.len() - 1];
            let next = remaining.iter().enumerate().find_map(|(i, (p, c))| {
                if *c || p.is_empty() {
                    return None;
                }
                let x = p[0];
                let y = p[p.len() - 1];
                if curves::distance(b, x) <= SNAP {
                    Some((i, false, false))
                } else if curves::distance(b, y) <= SNAP {
                    Some((i, false, true))
                } else if curves::distance(a, y) <= SNAP {
                    Some((i, true, false))
                } else if curves::distance(a, x) <= SNAP {
                    Some((i, true, true))
                } else {
                    None
                }
            });
            let Some((i, prepend, reverse)) = next else {
                break;
            };
            let (mut p, _) = remaining.swap_remove(i);
            if reverse {
                p.reverse();
            }
            if prepend {
                p.pop();
                p.extend(points);
                points = p;
            } else {
                points.extend(p.into_iter().skip(1));
            }
            closed = clean(&mut points, false);
        }
        result.push((points, closed));
    }
    result
}

fn reflect(p: Pt, [a, b]: [Pt; 2]) -> Pt {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy);
    [2.0 * (a[0] + t * dx) - p[0], 2.0 * (a[1] + t * dy) - p[1]]
}

fn unfold(outline: &mut Vec<Pt>, marks: &mut Vec<Mark>, axis: [Pt; 2]) -> Result<()> {
    ensure!(
        curves::distance(axis[0], axis[1]) > 1e-9,
        "zero-length mirror line"
    );
    let on_axis = |p| curves::distance(p, reflect(p, axis)) <= 2.0 * SNAP;
    // Remove the straight fold edge, then append the reversed reflected outer path.
    let n = outline.len();
    let edge = (0..n).find(|&i| on_axis(outline[i]) && !on_axis(outline[(i + 1) % n]));
    let i = edge.context("mirror axis must coincide with an outline fold edge")?;
    outline.rotate_left(i);
    let end = (1..n)
        .find(|&i| on_axis(outline[i]))
        .context("mirror axis must touch both ends of the fold edge")?;
    ensure!(
        outline[end..].iter().all(|&p| on_axis(p)),
        "outline crosses mirror axis more than twice"
    );
    outline.truncate(end + 1);
    let reflected: Vec<_> = outline
        .iter()
        .rev()
        .skip(1)
        .take(outline.len().saturating_sub(2))
        .map(|&p| reflect(p, axis))
        .collect();
    outline.extend(reflected);
    let extra: Vec<_> = marks
        .iter()
        .filter(|m| !m.points.iter().all(|&p| on_axis(p)))
        .map(|m| Mark {
            kind: m.kind,
            points: m.points.iter().map(|&p| reflect(p, axis)).collect(),
            text: m.text.clone(),
        })
        .collect();
    marks.extend(extra);
    Ok(())
}

pub(crate) fn contains(poly: &[Pt], p: Pt) -> bool {
    if boundary_distance(poly, p) <= 1e-9 {
        return true;
    }
    let mut inside = false;
    for i in 0..poly.len() {
        let a = poly[i];
        let b = poly[(i + 1) % poly.len()];
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    inside
}

pub(crate) fn boundary_distance(poly: &[Pt], p: Pt) -> f64 {
    (0..poly.len())
        .map(|i| {
            let a = poly[i];
            let b = poly[(i + 1) % poly.len()];
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let len2 = dx * dx + dy * dy;
            let t = if len2 > 0.0 {
                (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            curves::distance(p, [a[0] + t * dx, a[1] + t * dy])
        })
        .fold(f64::INFINITY, f64::min)
}

pub(crate) fn nested(outer: &[Pt], inner: &[Pt]) -> bool {
    if !inner.iter().all(|&p| contains(outer, p)) {
        return false;
    }
    let cross = |a: Pt, b: Pt, c: Pt| (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    // All vertices can be inside a concave polygon even when an edge crosses a
    // concavity. Such an outline is a separate piece, not an internal line.
    for i in 0..inner.len() {
        let a = inner[i];
        let b = inner[(i + 1) % inner.len()];
        for j in 0..outer.len() {
            let c = outer[j];
            let d = outer[(j + 1) % outer.len()];
            if cross(a, b, c) * cross(a, b, d) < -1e-12 && cross(c, d, a) * cross(c, d, b) < -1e-12
            {
                return false;
            }
        }
    }
    true
}

fn generic(shapes: &[Shape], style: Option<String>, warnings: &mut Vec<String>) -> Vec<Piece> {
    warn(warnings, "Generic DXF: each outer closed outline becomes a piece; nested outlines are internal lines; quantity defaults to 1".into());
    let mut loops: Vec<Vec<Pt>> = shapes
        .iter()
        .filter(|s| s.text.is_none() && s.closed && s.points.len() >= 3)
        .map(|s| {
            let mut p = s.points.clone();
            clean(&mut p, true);
            p
        })
        .filter(|p| area(p) > 1e-10)
        .collect();
    loops.sort_by(|a, b| area(b).total_cmp(&area(a)));
    let mut pieces: Vec<Piece> = vec![];
    for outline in loops {
        let parent = pieces.iter_mut().find(|p| nested(&p.outline, &outline));
        if let Some(p) = parent {
            let mut points = outline;
            points.push(points[0]);
            p.marks.push(Mark {
                kind: MarkKind::Line,
                points,
                text: None,
            });
        } else {
            let name = format!("Outline {}", pieces.len() + 1);
            pieces.push(Piece {
                id: format!(
                    "{}-OUTLINE-{}-BASE",
                    sanitize(style.as_deref().unwrap_or("STYLE")),
                    pieces.len() + 1
                ),
                name,
                style: style.clone(),
                size: None,
                material: None,
                outline,
                grain: None,
                quantity: 1,
                mirror_quantity: 0,
                rotation: RotationRule::Inherit,
                buffer: 0.0,
                marks: vec![],
                ..Default::default()
            });
        }
    }
    for p in &mut pieces {
        let mut grain = None;
        for s in shapes
            .iter()
            .filter(|s| s.layer == "7" && s.text.is_none() && s.points.len() >= 2)
        {
            let g = [s.points[0], s.points[s.points.len() - 1]];
            if contains(&p.outline, g[0])
                && contains(&p.outline, g[1])
                && grain.is_none_or(|old: [Pt; 2]| {
                    curves::distance(g[0], g[1]) > curves::distance(old[0], old[1])
                })
            {
                grain = Some(g);
            }
        }
        p.grain = grain;
        if grain.is_none() {
            warn(
                warnings,
                format!("{}: no grain line; piece x axis is the grain", p.name),
            );
        }
    }
    pieces
}
