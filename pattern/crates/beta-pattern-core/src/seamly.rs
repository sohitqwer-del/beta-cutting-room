//! Read-only XML subset importer, written against original test fixtures, not GPL code.
use crate::{
    evaluate::{self, Evaluator},
    model::*,
};
use anyhow::{Context, Result, bail, ensure};
use roxmltree::{Document as Xml, Node, ParsingOptions};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Imported pattern plus explicit omissions that require review.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportReport {
    /// Native document.
    pub document: Document,
    /// Assumptions and omissions requiring review before cutting.
    pub warnings: Vec<String>,
}

/// Explicit copies per garment. Their sum must equal the native total quantity.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CopyRule {
    /// Copies with the drafted orientation.
    pub normal: u32,
    /// Additional reflected copies.
    pub mirrored: u32,
}

/// Native import choices. No pairing is inferred from quantities or flip permission.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportOptions {
    /// Rules keyed by stable imported piece ID (for example `detail_43`).
    #[serde(default)]
    pub copy_rules: BTreeMap<String, CopyRule>,
}
fn xml(source: &str) -> Result<Xml<'_>> {
    ensure!(source.len() <= 16 * 1024 * 1024, "XML exceeds 16 MiB");
    Ok(Xml::parse_with_options(
        source,
        ParsingOptions {
            nodes_limit: 200_000,
            ..Default::default()
        },
    )?)
}
fn attr<'a>(n: Node<'a, 'a>, key: &str) -> Result<&'a str> {
    n.attribute(key).with_context(|| {
        format!(
            "<{} id={}> requires '{key}'",
            n.tag_name().name(),
            n.attribute("id").unwrap_or("?")
        )
    })
}
fn unit(doc: &Xml<'_>) -> Result<f64> {
    let u = doc
        .descendants()
        .find(|n| n.has_tag_name("unit"))
        .and_then(|n| n.text())
        .unwrap_or("mm");
    match u.trim() {
        "mm" => Ok(1.),
        "cm" => Ok(10.),
        "inch" | "in" => Ok(25.4),
        _ => bail!("unsupported XML unit '{u}'"),
    }
}
fn truth(s: Option<&str>) -> bool {
    matches!(s, Some("true" | "1"))
}
fn rewrite(
    s: &str,
    measurements: &BTreeSet<String>,
    variables: &BTreeSet<String>,
    scale: f64,
) -> Result<String> {
    ensure!(
        s.is_ascii(),
        "non-ASCII formula identifiers are outside the import subset"
    );
    let bytes = s.as_bytes();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' || bytes[i] == b'@' {
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let word = &s[start..i];
            let key = word.trim_start_matches('@');
            if measurements.contains(key) {
                out.push_str(&format!("(m.{key}/{scale})"));
            } else if variables.contains(key) {
                out.push_str(&format!("(v.{key}/{scale})"));
            } else {
                out.push_str(word);
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    Ok(out)
}
/// Import one `.sm2d`/`.val` and optional `.smis`/`.vit`, using a single named size.
/// Unsupported construction operations fail rather than becoming approximate points.
pub fn import(pattern: &str, measurements: Option<&str>, size: &str) -> Result<ImportReport> {
    import_with_options(pattern, measurements, size, &ImportOptions::default())
}

/// Import a single size with explicit per-piece copy rules.
pub fn import_with_options(
    pattern: &str,
    measurements: Option<&str>,
    size: &str,
    options: &ImportOptions,
) -> Result<ImportReport> {
    ensure!(!size.is_empty(), "size label must not be empty");
    let source = xml(pattern)?;
    ensure!(
        source.root_element().has_tag_name("pattern"),
        "expected <pattern> root"
    );
    let scale = unit(&source)?;
    let current_simple_splines = source
        .descendants()
        .find(|n| n.has_tag_name("version"))
        .and_then(|n| n.text())
        .map(str::trim)
        == Some("0.7.5");
    let mut doc = Document {
        name: source
            .descendants()
            .find(|n| n.has_tag_name("description"))
            .and_then(|n| n.text())
            .unwrap_or("Imported Seamly pattern")
            .into(),
        sizes: vec![size.into()],
        base_size: size.into(),
        ..Document::default()
    };
    let mut table = BTreeMap::new();
    if let Some(source) = measurements {
        let set = crate::measurements::import_seamly(source, size)?;
        let row = set
            .values
            .get(size)
            .or_else(|| set.values.get(&set.base_size))
            .context("no measurement base row")?;
        table = row.clone();
    }
    let names = table.keys().cloned().collect();
    doc.measurements.insert(size.into(), table);
    let variables: BTreeSet<String> = source
        .descendants()
        .filter(|n| n.has_tag_name("increment"))
        .filter_map(|n| n.attribute("name"))
        .map(|n| n.trim_start_matches('@').to_owned())
        .collect();
    let value = |s: &str, length: bool| -> Result<Value> {
        let expr = rewrite(s, &names, &variables, scale)?;
        Ok(format!("({expr})*{}", if length { scale } else { 1. })
            .as_str()
            .into())
    };
    for n in source.descendants().filter(|n| n.has_tag_name("increment")) {
        let name = attr(n, "name")?.trim_start_matches('@');
        let expr = n
            .attribute("formula")
            .or(n.attribute("value"))
            .context("increment requires formula or value")?;
        ensure!(
            doc.variables
                .insert(name.into(), value(expr, true)?)
                .is_none(),
            "duplicate increment '{name}'"
        );
    }
    let mut ids = BTreeMap::new();
    let mut aliases = BTreeMap::new();
    for n in source.descendants().filter(|n| {
        n.is_element()
            && (n.has_tag_name("point")
                || n.has_tag_name("spline")
                || n.has_tag_name("line")
                || n.has_tag_name("arc")
                || n.has_tag_name("ellipticalArc")
                || n.has_tag_name("elArc")
                || n.has_tag_name("path"))
    }) {
        if let Some(id) = n.attribute("id") {
            if let Some(original) = n.attribute("idObject") {
                ensure!(
                    aliases.insert(id.to_owned(), original.to_owned()).is_none(),
                    "duplicate modeling ID {id}"
                );
            } else {
                let name = n
                    .attribute("name")
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("object_{id}"));
                ensure!(
                    ids.insert(id.to_owned(), name).is_none(),
                    "duplicate object ID {id}"
                );
            }
        }
    }
    for n in source.descendants().filter(|n| {
        n.has_tag_name("source") && n.parent().is_some_and(|p| p.has_tag_name("operation"))
    }) {
        let id = attr(n, "id")?;
        let name = n
            .attribute("name")
            .map(str::to_owned)
            .unwrap_or_else(|| format!("object_{id}"));
        ensure!(
            ids.insert(id.into(), name).is_none(),
            "duplicate operation result ID"
        );
    }
    for _ in 0..128 {
        let mut added = false;
        for (id, original) in &aliases {
            if !ids.contains_key(id)
                && let Some(name) = ids.get(original).cloned()
            {
                ids.insert(id.clone(), name);
                added = true;
            }
        }
        if !added {
            break;
        }
    }
    let reference = |id: &str| -> Result<String> {
        ids.get(id)
            .cloned()
            .with_context(|| format!("unknown XML object ID {id}"))
    };
    let r = |n: Node<'_, '_>, key: &str| -> Result<String> { reference(attr(n, key)?) };
    let index = |n: Node<'_, '_>| -> Result<usize> {
        if let Some(v) = n.attribute("index") {
            return Ok(v.parse()?);
        }
        let v = n
            .attribute("crossPoint")
            .or(n.attribute("c1CrossPoint"))
            .unwrap_or("1")
            .parse::<usize>()?;
        ensure!(v >= 1, "XML crossing index starts at 1");
        Ok(v - 1)
    };
    for n in source.descendants().filter(|n| {
        n.is_element() && n.attribute("id").is_some() && n.attribute("idObject").is_none()
    }) {
        let tag = n.tag_name().name();
        if !matches!(
            tag,
            "point" | "line" | "spline" | "arc" | "ellipticalArc" | "elArc" | "path"
        ) {
            continue;
        }
        if tag == "path" && n.attribute("type") == Some("2") {
            continue;
        }
        let id = reference(attr(n, "id")?)?;
        let object = match tag {
            "point" => {
                let operation = match attr(n, "type")? {
                    "single" => PointOp::Absolute {
                        x: value(attr(n, "x")?, true)?,
                        y: value(&format!("-({})", attr(n, "y")?), true)?,
                    },
                    "alongLine" => PointOp::AlongLine {
                        from: r(n, "firstPoint")?,
                        to: r(n, "secondPoint")?,
                        distance: value(attr(n, "length")?, true)?,
                    },
                    "endLine" => PointOp::Polar {
                        from: r(n, "basePoint")?,
                        angle: value(attr(n, "angle")?, false)?,
                        distance: value(attr(n, "length")?, true)?,
                    },
                    "normal" => {
                        let a = r(n, "firstPoint")?;
                        let b = r(n, "secondPoint")?;
                        let extra = n.attribute("angle").unwrap_or("0");
                        if extra.parse::<f64>().ok() == Some(0.) {
                            PointOp::Normal {
                                a,
                                b,
                                distance: value(attr(n, "length")?, true)?,
                            }
                        } else {
                            let angle = value(extra, false)?;
                            let angle = match angle {
                                Value::Number(n) => n.to_string(),
                                Value::Formula(s) => s,
                            };
                            PointOp::Polar {
                                from: a.clone(),
                                angle: format!("angle({a},{b})+90+({angle})").as_str().into(),
                                distance: value(attr(n, "length")?, true)?,
                            }
                        }
                    }
                    "bisector" => PointOp::Bisector {
                        vertex: r(n, "secondPoint")?,
                        a: r(n, "firstPoint")?,
                        b: r(n, "thirdPoint")?,
                        distance: value(attr(n, "length")?, true)?,
                    },
                    "lineIntersect" => PointOp::LineIntersection {
                        a: [r(n, "p1Line1")?, r(n, "p2Line1")?],
                        b: [r(n, "p1Line2")?, r(n, "p2Line2")?],
                    },
                    "pointOfIntersection" | "intersectXY" => {
                        let x = r(n, "firstPoint")?;
                        let y = r(n, "secondPoint")?;
                        PointOp::Absolute {
                            x: format!("x({x})").as_str().into(),
                            y: format!("y({y})").as_str().into(),
                        }
                    }
                    "height" => PointOp::Foot {
                        point: r(n, "basePoint")?,
                        a: r(n, "p1Line")?,
                        b: r(n, "p2Line")?,
                    },
                    "shoulder" => PointOp::Shoulder {
                        center: r(n, "pShoulder")?,
                        length: value(attr(n, "length")?, true)?,
                        a: r(n, "p1Line")?,
                        b: r(n, "p2Line")?,
                        index: index(n)?,
                    },
                    "triangle" => PointOp::Triangle {
                        a: r(n, "firstPoint")?,
                        b: r(n, "secondPoint")?,
                        axis_a: r(n, "axisP1")?,
                        axis_b: r(n, "axisP2")?,
                        index: index(n)?,
                    },
                    "lineIntersectAxis" => PointOp::LineAxis {
                        a: r(n, "p1Line")?,
                        b: r(n, "p2Line")?,
                        origin: r(n, "basePoint")?,
                        angle: value(attr(n, "angle")?, false)?,
                    },
                    "curveIntersectAxis" => PointOp::CurveAxis {
                        curve: r(n, "curve")?,
                        origin: r(n, "basePoint")?,
                        angle: value(attr(n, "angle")?, false)?,
                        index: index(n)?,
                    },
                    "pointOfIntersectionArcs" => PointOp::ArcIntersection {
                        a: r(n, "firstArc")?,
                        b: r(n, "secondArc")?,
                        index: index(n)?,
                    },
                    "pointOfIntersectionCircles" => PointOp::CircleIntersection {
                        a: r(n, "center1")?,
                        radius_a: value(attr(n, "radius1")?, true)?,
                        b: r(n, "center2")?,
                        radius_b: value(attr(n, "radius2")?, true)?,
                        index: index(n)?,
                    },
                    "pointFromCircleAndTangent" => PointOp::CircleTangent {
                        point: r(n, "tangent")?,
                        center: r(n, "center")?,
                        radius: value(attr(n, "radius")?, true)?,
                        index: index(n)?,
                    },
                    "pointFromArcAndTangent" => PointOp::ArcTangent {
                        point: r(n, "tangent")?,
                        arc: r(n, "arc")?,
                        index: index(n)?,
                    },
                    "pointOfIntersectionArcAndLine" => PointOp::ArcLine {
                        arc: r(n, "arc")?,
                        a: r(n, "p1Line")?,
                        b: r(n, "p2Line")?,
                        index: index(n)?,
                    },
                    "pointOfContact" => PointOp::CircleLine {
                        center: r(n, "center")?,
                        radius: value(attr(n, "radius")?, true)?,
                        a: r(n, "firstPoint")?,
                        b: r(n, "secondPoint")?,
                        index: index(n)?,
                    },
                    "pointOfIntersectionCurves" => PointOp::Intersection {
                        a: r(n, "curve1")?,
                        b: r(n, "curve2")?,
                        index: index(n)?,
                    },
                    "cutArc" | "cutSpline" | "cutSplinePath" => PointOp::CurvePoint {
                        curve: reference(
                            n.attribute("curve")
                                .or(n.attribute("arc"))
                                .or(n.attribute("spline"))
                                .or(n.attribute("splinePath"))
                                .context("cut tool needs curve reference")?,
                        )?,
                        distance: value(attr(n, "length")?, true)?,
                    },
                    "trueDarts" => PointOp::TrueDart {
                        apex: r(n, "apex")?,
                        leg: r(n, "leg")?,
                        other: r(n, "other")?,
                    },
                    t => bail!("unsupported point type '{t}' at '{id}'"),
                };
                Object::Point { operation }
            }
            "line" => Object::Line {
                from: r(n, "firstPoint")?,
                to: r(n, "secondPoint")?,
            },
            "path" => {
                let kind = n.attribute("type").unwrap_or("bezierPath");
                if kind == "bezierPath" {
                    let segments = n
                        .children()
                        .filter(|n| n.has_tag_name("segment"))
                        .map(|n| {
                            Ok([
                                r(n, "point1")?,
                                r(n, "point2")?,
                                r(n, "point3")?,
                                r(n, "point4")?,
                            ])
                        })
                        .collect::<Result<Vec<_>>>()?;
                    Object::CubicPath { segments }
                } else if kind == "splinePath" || kind == "interactive" {
                    let points = n
                        .children()
                        .filter(|n| n.has_tag_name("node"))
                        .map(|n| r(n, "point"))
                        .collect::<Result<Vec<_>>>()?;
                    Object::Spline {
                        points,
                        tangents: vec![],
                    }
                } else {
                    bail!("unsupported path type '{kind}' at '{id}'");
                }
            }
            "spline"
                if matches!(
                    n.attribute("type"),
                    Some("pathInteractive" | "interactive" | "splinePath")
                ) =>
            {
                let points = n
                    .children()
                    .filter(|n| n.has_tag_name("node") || n.has_tag_name("pathPoint"))
                    .map(|n| r(n, "point"))
                    .collect::<Result<Vec<_>>>()?;
                Object::Spline {
                    points,
                    tangents: vec![],
                }
            }
            "spline" => {
                ensure!(
                    n.attribute("type") == Some("simple"),
                    "unsupported spline type at '{id}'"
                );
                let a = r(n, "point1")?;
                let b = r(n, "point4")?;
                let c1 = format!("{id}.control1");
                let c2 = format!("{id}.control2");
                // Clean-room reconstruction from our original 0.7.5 exports:
                // default simple splines use equal chord-relative handles, not
                // the sample's length1/length2 attributes. The circular cubic
                // handle ratio matches all retained sizes within the DXF budget.
                let default_handle = if current_simple_splines {
                    for key in ["kAsm1", "kAsm2", "kCurve"] {
                        ensure!(
                            n.attribute(key).unwrap_or("1").parse::<f64>()? == 1.,
                            "non-default simple spline coefficient {key} is outside the export-validated subset"
                        );
                    }
                    let ratio = 4. / 3. * (2f64.sqrt() - 1.) / 2f64.sqrt();
                    Some(Value::Formula(format!("distance({a},{b})*{ratio}")))
                } else {
                    None
                };
                for (c, from, angle, length) in [
                    (&c1, &a, "angle1", "length1"),
                    (&c2, &b, "angle2", "length2"),
                ] {
                    ensure!(!doc.drawing.contains_key(c), "spline control ID collision");
                    doc.drawing.insert(
                        c.clone(),
                        Object::Point {
                            operation: PointOp::Polar {
                                from: from.clone(),
                                angle: value(attr(n, angle)?, false)?,
                                distance: match &default_handle {
                                    Some(length) => length.clone(),
                                    None => value(attr(n, length)?, true)?,
                                },
                            },
                        },
                    );
                }
                Object::Cubic {
                    points: [a, c1, c2, b],
                }
            }
            "arc" => {
                let center = r(n, "center")?;
                let radius = value(attr(n, "radius")?, true)?;
                let start = value(
                    n.attribute("angle1")
                        .or(n.attribute("start"))
                        .context("arc needs angle1")?,
                    false,
                )?;
                if matches!(n.attribute("type"), Some("arcWithLength" | "length")) {
                    Object::ArcLength {
                        center,
                        radius,
                        start,
                        length: value(attr(n, "length")?, true)?,
                    }
                } else {
                    let sweep = if let Some(s) = n.attribute("sweep") {
                        value(s, false)?
                    } else {
                        value(
                            &format!("({})-({})", attr(n, "angle2")?, attr(n, "angle1")?),
                            false,
                        )?
                    };
                    Object::Arc {
                        center,
                        radius,
                        start,
                        sweep,
                    }
                }
            }
            "ellipticalArc" | "elArc" => Object::EllipticalArc {
                center: r(n, "center")?,
                radius_x: value(attr(n, "radius1")?, true)?,
                radius_y: value(attr(n, "radius2")?, true)?,
                rotation: value(n.attribute("rotationAngle").unwrap_or("0"), false)?,
                start: value(attr(n, "angle1")?, false)?,
                sweep: value(
                    &format!("({})-({})", attr(n, "angle2")?, attr(n, "angle1")?),
                    false,
                )?,
            },
            _ => bail!("unsupported XML object tag '{tag}'"),
        };
        ensure!(
            doc.drawing.insert(id.clone(), object).is_none(),
            "duplicate drawing name '{id}'"
        );
    }
    for n in source.descendants().filter(|n| n.has_tag_name("operation")) {
        let kind = attr(n, "type")?;
        for item in n.children().filter(|c| c.has_tag_name("source")) {
            let id = r(item, "id")?;
            let original = r(item, "idObject")?;
            let object = match kind {
                "rotation" => Object::Rotate {
                    source: original,
                    pivot: r(n, "center")?,
                    angle: value(attr(n, "angle")?, false)?,
                },
                "move" => Object::Move {
                    source: original,
                    dx: value(attr(n, "dx")?, true)?,
                    dy: value(&format!("-({})", attr(n, "dy")?), true)?,
                },
                "mirrorByLine" => Object::MirrorLine {
                    source: original,
                    a: r(n, "firstPoint")?,
                    b: r(n, "secondPoint")?,
                },
                "mirrorByAxis" => Object::MirrorAxis {
                    source: original,
                    origin: r(n, "center")?,
                    axis: match attr(n, "axis")? {
                        "horizontal" => Axis::Horizontal,
                        "vertical" => Axis::Vertical,
                        a => bail!("unsupported mirror axis '{a}'"),
                    },
                },
                t => bail!("unsupported XML operation '{t}'"),
            };
            ensure!(
                doc.drawing.insert(id, object).is_none(),
                "operation result ID collision"
            );
        }
        ensure!(
            n.children().any(|n| n.has_tag_name("source")),
            "operation '{kind}' has no supported result sources"
        );
    }
    for n in source.descendants().filter(|n| n.has_tag_name("group")) {
        let id = attr(n, "id")?;
        let objects = n
            .children()
            .filter(|c| c.has_tag_name("item"))
            .map(|c| r(c, "idObject"))
            .collect::<Result<Vec<_>>>()?;
        ensure!(
            doc.groups
                .insert(
                    id.into(),
                    DrawingGroup {
                        name: n.attribute("name").unwrap_or(id).into(),
                        objects,
                        visible: n
                            .attribute("visible")
                            .is_none_or(|v| v == "true" || v == "1")
                    }
                )
                .is_none(),
            "duplicate XML group"
        );
    }
    let mut warnings=vec!["Read-only import: layout and display styles are not transferred. Review seam allowance joins and cutting metadata.".into()];
    for n in source
        .descendants()
        .filter(|n| n.has_tag_name("detail") || n.has_tag_name("piece"))
    {
        let id = format!("detail_{}", attr(n, "id")?);
        let name = n.attribute("name").unwrap_or(&id).to_owned();
        let mut edges = vec![];
        let mut first: Option<String> = None;
        let mut last: Option<String> = None;
        let allowance =
            if truth(n.attribute("seamAllowance")) && !truth(n.attribute("seamAllowanceBuiltIn")) {
                value(n.attribute("width").unwrap_or("0"), true)?
            } else {
                0.0.into()
            };
        for node in n
            .children()
            .filter(|c| c.has_tag_name("nodes"))
            .flat_map(|c| c.children())
            .filter(|c| c.has_tag_name("node"))
        {
            ensure!(
                !truth(node.attribute("excluded")),
                "excluded detail nodes outside subset"
            );
            for key in ["before", "after", "saBefore", "saAfter"] {
                if let Some(v) = node.attribute(key) {
                    ensure!(
                        v == "0" || v == "-1",
                        "per-node seam allowance outside subset ({key}={v})"
                    );
                }
            }
            let obj = reference(attr(node, "idObject")?)?;
            let reversed = truth(node.attribute("reverse"));
            let (start, end, curve) = match doc
                .drawing
                .get(&obj)
                .with_context(|| format!("detail references missing object '{obj}'"))?
            {
                Object::Point { .. } => (obj.clone(), obj.clone(), false),
                Object::Line { from, to } => (from.clone(), to.clone(), true),
                Object::Cubic { points } => (points[0].clone(), points[3].clone(), true),
                _ => {
                    let start = format!("{id}.{obj}.start");
                    let end = format!("{id}.{obj}.end");
                    for (point, index) in [(&start, 0), (&end, 1)] {
                        ensure!(
                            !doc.drawing.contains_key(point),
                            "detail endpoint collision"
                        );
                        doc.drawing.insert(
                            point.clone(),
                            Object::Point {
                                operation: PointOp::PathEndpoint {
                                    curve: obj.clone(),
                                    index,
                                },
                            },
                        );
                    }
                    (start, end, true)
                }
            };
            let (start, end) = if reversed { (end, start) } else { (start, end) };
            if let Some(previous) = &last
                && previous != &start
                && crate::geometry::distance(
                    Evaluator::new(&doc, size, 0.1)?.point(previous)?,
                    Evaluator::new(&doc, size, 0.1)?.point(&start)?,
                ) > 1e-6
            {
                let line = format!("{id}.join{}", edges.len());
                ensure!(!doc.drawing.contains_key(&line), "detail line ID collision");
                doc.drawing.insert(
                    line.clone(),
                    Object::Line {
                        from: previous.clone(),
                        to: start.clone(),
                    },
                );
                edges.push(Edge {
                    object: line,
                    reversed: false,
                    allowance: allowance.clone(),
                    corner: Corner::Miter,
                });
            }
            if first.is_none() {
                first = Some(start);
            }
            if curve {
                edges.push(Edge {
                    object: obj,
                    reversed,
                    allowance: allowance.clone(),
                    corner: Corner::Miter,
                });
            }
            last = Some(end);
        }
        let first = first.context("detail has no path nodes")?;
        let last = last.context("detail has no last node")?;
        if first != last
            && crate::geometry::distance(
                Evaluator::new(&doc, size, 0.1)?.point(&first)?,
                Evaluator::new(&doc, size, 0.1)?.point(&last)?,
            ) > 1e-6
        {
            let line = format!("{id}.close");
            ensure!(
                !doc.drawing.contains_key(&line),
                "detail closing ID collision"
            );
            doc.drawing.insert(
                line.clone(),
                Object::Line {
                    from: last,
                    to: first,
                },
            );
            edges.push(Edge {
                object: line,
                reversed: false,
                allowance: allowance.clone(),
                corner: Corner::Miter,
            });
        }
        doc.pieces.push(PatternPiece {
            internal_paths: vec![],
            anchors: BTreeMap::new(),
            grain_angle: None,
            label_point: None,
            id: id.clone(),
            name,
            edges,
            quantity: 1,
            mirror_quantity: 0,
            material: None,
            grain: None,
            fold_line: None,
            rotation: Default::default(),
            buffer: 0.0.into(),
            annotations: vec![],
            grade_points: BTreeMap::new(),
        });
        let mut internal_paths = vec![];
        for path in n.descendants().filter(|n| n.has_tag_name("internalPath")) {
            let edges = path
                .children()
                .filter(|n| n.has_tag_name("node"))
                .map(|node| {
                    Ok(Edge {
                        object: r(node, "idObject")?,
                        reversed: truth(node.attribute("reverse")),
                        allowance: 0.0.into(),
                        corner: Corner::Miter,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            internal_paths.push(InternalPath {
                name: path.attribute("name").unwrap_or("Internal path").into(),
                edges,
                closed: truth(path.attribute("closed")),
            });
        }
        for record in n.descendants().filter(|n| {
            n.has_tag_name("record") && n.parent().is_some_and(|p| p.has_tag_name("iPaths"))
        }) {
            let path_id = attr(record, "path")?;
            let path = source
                .descendants()
                .find(|n| n.has_tag_name("path") && n.attribute("id") == Some(path_id))
                .context("missing internal path")?;
            ensure!(
                !truth(path.attribute("cut")),
                "internal cutout paths are not supported"
            );
            let nodes = path
                .descendants()
                .filter(|n| n.has_tag_name("node"))
                .map(|n| r(n, "idObject"))
                .collect::<Result<Vec<_>>>()?;
            ensure!(nodes.len() >= 2, "internal path needs at least two points");
            let mut edges = vec![];
            for (i, w) in nodes.windows(2).enumerate() {
                let edge_id = format!("internal_{path_id}.edge{i}");
                ensure!(
                    !doc.drawing.contains_key(&edge_id),
                    "internal edge collision"
                );
                doc.drawing.insert(
                    edge_id.clone(),
                    Object::Line {
                        from: w[0].clone(),
                        to: w[1].clone(),
                    },
                );
                edges.push(Edge {
                    object: edge_id,
                    reversed: false,
                    allowance: 0.0.into(),
                    corner: Corner::Miter,
                });
            }
            internal_paths.push(InternalPath {
                name: path.attribute("name").unwrap_or("Internal path").into(),
                edges,
                closed: nodes.first() == nodes.last(),
            });
        }
        let anchors = n
            .descendants()
            .filter(|n| n.has_tag_name("anchor"))
            .map(|n| Ok((attr(n, "name")?.into(), r(n, "idObject")?)))
            .collect::<Result<BTreeMap<_, _>>>()?;
        let grain_node = n
            .descendants()
            .find(|n| n.has_tag_name("grainline") || n.has_tag_name("grainLine"));
        if grain_node.is_some_and(|g| matches!(g.attribute("visible"), Some("false" | "0"))) {
            warnings.push(format!(
                "Piece '{}': grain is hidden in the source; confirm grain before cutting.",
                doc.pieces.last().context("imported piece missing")?.name
            ));
        }
        let grain_node =
            grain_node.filter(|g| !matches!(g.attribute("visible"), Some("false" | "0")));
        let grain = grain_node
            .filter(|n| n.attribute("startPoint").is_some())
            .map(|n| Ok::<_, anyhow::Error>([r(n, "startPoint")?, r(n, "endPoint")?]))
            .transpose()?;
        let grain_angle = grain_node
            .filter(|n| n.attribute("center").is_some())
            .map(|n| {
                Ok::<_, anyhow::Error>(GrainAngle {
                    origin: r(n, "center")?,
                    angle: value(attr(n, "angle")?, false)?,
                    length: value(attr(n, "length")?, true)?,
                })
            })
            .transpose()?;
        let scene_grain = if let Some(g) =
            grain_node.filter(|n| n.attribute("mx").is_some() && n.attribute("my").is_some())
        {
            warnings.push(format!("BP-004: Piece '{}': free-position grain uses the source scene anchor; Seamly export may reposition it for containment. Verify grain placement before factory export.", doc.pieces.last().context("imported piece missing")?.name));
            let origin = format!("{id}.grain_origin");
            let x = attr(g, "mx")?.parse::<f64>()? * 25.4 / 96.;
            let y = -attr(g, "my")?.parse::<f64>()? * 25.4 / 96.;
            doc.drawing.insert(
                origin.clone(),
                Object::Point {
                    operation: PointOp::Absolute {
                        x: x.into(),
                        y: y.into(),
                    },
                },
            );
            Some(GrainAngle {
                origin,
                angle: value(g.attribute("rotation").unwrap_or("0"), false)?,
                length: value(attr(g, "length")?, true)?,
            })
        } else {
            None
        };
        let scene_label = if let Some(data) = n
            .children()
            .find(|n| n.has_tag_name("data") && truth(n.attribute("visible")))
        {
            if let (Some(x), Some(y)) = (data.attribute("mx"), data.attribute("my")) {
                let point = format!("{id}.label_origin");
                let x = x.parse::<f64>()? * 25.4 / 96.;
                let y = -y.parse::<f64>()? * 25.4 / 96.;
                doc.drawing.insert(
                    point.clone(),
                    Object::Point {
                        operation: PointOp::Absolute {
                            x: x.into(),
                            y: y.into(),
                        },
                    },
                );
                Some(point)
            } else {
                None
            }
        } else {
            None
        };
        let piece = doc.pieces.last_mut().context("imported piece missing")?;
        piece.internal_paths = internal_paths;
        piece.anchors = anchors;
        piece.grain = grain;
        piece.grain_angle = grain_angle.or(scene_grain);
        if grain_node.is_some() && piece.grain.is_none() && piece.grain_angle.is_none() {
            warnings.push(format!(
                "Piece '{}': grain attributes were not recognized; set grain before cutting.",
                piece.name
            ));
        }
        let data = n.children().find(|n| n.has_tag_name("data"));
        piece.material = n.attribute("fabric").map(str::to_owned).or_else(|| {
            data.and_then(|data| {
                data.children()
                    .filter(|n| n.has_tag_name("line"))
                    .filter_map(|n| n.attribute("text"))
                    .find_map(|text| text.strip_prefix("Material:").map(|s| s.trim().to_owned()))
            })
        });
        piece.quantity = n
            .attribute("quantity")
            .or_else(|| data.and_then(|n| n.attribute("quantity")))
            .unwrap_or("1")
            .parse()?;
        if !options.copy_rules.contains_key(&piece.id)
            && let Some(data) = data
            && data
                .attribute("annotation")
                .is_some_and(|a| a.to_ascii_lowercase().contains("mirrored pair"))
        {
            warnings.push(format!("Piece '{}': quantity {} preserved as normal copies; mirrored-pair annotation needs explicit copy rules.",piece.name,piece.quantity));
        }
        let mut notch_nodes = vec![];
        for node in n
            .children()
            .filter(|n| n.has_tag_name("nodes"))
            .flat_map(|n| n.children())
            .filter(|n| n.has_tag_name("node") && truth(n.attribute("notch")))
        {
            notch_nodes.push((
                r(node, "idObject")?,
                value(node.attribute("notchLength").unwrap_or("0.3"), true)?,
                value(node.attribute("notchWidth").unwrap_or("0.2"), true)?,
                match node.attribute("notchType").unwrap_or("slit") {
                    "slit" => NotchType::Slit,
                    "v" => NotchType::V,
                    "t" => NotchType::T,
                    "castle" => NotchType::Castle,
                    other => bail!("unsupported notch type '{other}'"),
                },
            ));
        }
        // Release the mutable piece borrow before resolving its directed seam.
        let piece_edges = piece.edges.clone();
        let mut extra_notches = vec![];
        let mut evaluator = Evaluator::new(&doc, size, 0.1)?;
        for (point, depth, width, shape) in notch_nodes {
            let q = evaluator.point(&point)?;
            for (edge_index, edge) in piece_edges.iter().enumerate() {
                let mut path = evaluator.path(&edge.object)?;
                if edge.reversed {
                    path.reverse();
                }
                if crate::geometry::distance(q, path[0]) < 1e-6 {
                    extra_notches.push(Annotation::ShapedNotch {
                        edge: edge_index,
                        distance: 0.0.into(),
                        depth,
                        width,
                        shape,
                    });
                    break;
                }
            }
        }
        let piece = doc.pieces.last_mut().context("imported piece missing")?;
        piece.annotations.extend(extra_notches);
        piece.label_point = n
            .attribute("labelPoint")
            .map(reference)
            .transpose()?
            .or(scene_label);
    }
    for n in source.descendants().filter(|n| n.has_tag_name("union")) {
        let a = format!("detail_{}", attr(n, "firstDetail")?);
        let b = format!("detail_{}", attr(n, "secondDetail")?);
        let id = format!("detail_{}", attr(n, "id")?);
        doc = crate::tools::union_pieces(&doc, &a, &b, &id, size, 0.1)?;
    }
    for (id, rule) in &options.copy_rules {
        let piece = doc
            .pieces
            .iter_mut()
            .find(|p| &p.id == id)
            .with_context(|| format!("copy rule references unknown piece '{id}'"))?;
        let total = rule
            .normal
            .checked_add(rule.mirrored)
            .context("copy-rule total overflow")?;
        ensure!(
            total > 0 && total == piece.quantity,
            "copy rule for '{id}' must preserve native total quantity {}",
            piece.quantity
        );
        piece.quantity = rule.normal;
        piece.mirror_quantity = rule.mirrored;
    }
    ensure!(
        !doc.drawing.is_empty(),
        "no supported drawing objects found"
    );
    // Validate every construction as well as piece topology before returning success.
    let mut evaluator = Evaluator::new(&doc, size, 0.1)?;
    for id in doc.drawing.keys() {
        evaluator.object(id)?;
    }
    evaluate::grade(&doc, size, 0.1)?;
    for n in source.descendants().filter(|n| n.is_element()) {
        ensure!(
            !matches!(n.tag_name().name(), "hole" | "holes" | "cutout"),
            "unsupported piece cutout element <{}>",
            n.tag_name().name()
        );
        if matches!(n.tag_name().name(), "image" | "backgroundImage") {
            warnings
                .push("Background tracing image omitted; attach the image in the editor.".into());
        }
        if n.parent().is_some_and(|p| p.has_tag_name("calculation"))
            && !matches!(
                n.tag_name().name(),
                "point"
                    | "line"
                    | "arc"
                    | "spline"
                    | "path"
                    | "ellipticalArc"
                    | "elArc"
                    | "operation"
            )
        {
            bail!("unsupported calculation element <{}>", n.tag_name().name());
        }
    }
    // Keep actionable warnings with the document for later plotting/export.
    doc.meta.insert(
        "seamly_import_warnings".into(),
        serde_json::to_value(&warnings)?,
    );
    Ok(ImportReport {
        document: doc,
        warnings,
    })
}
/// Import the explicit numeric size/stature selections from a pattern's gradation.
/// Uses multisize measurement steps and validates every generated size before returning.
pub fn import_multisize(pattern: &str, measurements: &str) -> Result<ImportReport> {
    import_multisize_with_options(pattern, measurements, &ImportOptions::default())
}

/// Import and grade explicit size/stature selections with per-piece copy rules.
pub fn import_multisize_with_options(
    pattern: &str,
    measurements: &str,
    options: &ImportOptions,
) -> Result<ImportReport> {
    let source = xml(pattern)?;
    let selected = |tag: &str, prefix: char| -> Result<Vec<f64>> {
        let mut values = vec![];
        if let Some(n) = source.descendants().find(|n| n.has_tag_name(tag)) {
            for a in n.attributes() {
                if a.name().starts_with(prefix) && truth(Some(a.value())) {
                    values.push(a.name()[1..].parse::<f64>()?);
                }
            }
        }
        values.sort_by(f64::total_cmp);
        Ok(values)
    };
    let sizes = selected("sizes", 's')?;
    let heights = selected("heights", 'h')?;
    ensure!(
        !sizes.is_empty() && !heights.is_empty(),
        "pattern must declare explicit size and height selections"
    );
    let set = crate::measurements::import_seamly_sizes(measurements, &sizes, &heights)?;
    let mut report = import_with_options(pattern, Some(measurements), &set.base_size, options)?;
    report.document = set.apply(&report.document)?;
    for size in &report.document.sizes {
        evaluate::grade(&report.document, size, 0.1)?;
    }
    Ok(report)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn valentina_centimetres_and_spline_detail() {
        let r = import(
            include_str!("../tests/fixtures/basic.val"),
            Some(include_str!("../tests/fixtures/basic.vit")),
            "custom",
        )
        .unwrap();
        assert_eq!(r.document.measurements["custom"]["width"], 100.);
        let g = evaluate::grade(&r.document, "custom", 0.01).unwrap();
        assert!((crate::geometry::area(&g[0].seam) - 5000.).abs() < 1e-8);
        assert!((crate::geometry::area(&g[0].piece.outline) - 6600.).abs() < 1e-8);
    }
    #[test]
    fn original_fixture() {
        let p = include_str!("../tests/fixtures/basic.sm2d");
        let m = include_str!("../tests/fixtures/basic.smis");
        let r = import(p, Some(m), "M").unwrap();
        let g = evaluate::grade(&r.document, "M", 0.01).unwrap();
        assert_eq!(crate::geometry::area(&g[0].seam), 5000.);
        assert_eq!(crate::geometry::area(&g[0].piece.outline), 6600.);
        let mut e = Evaluator::new(&r.document, "M", 0.01).unwrap();
        for (id, expected) in [
            ("along", [50., 0.]),
            ("normal", [0., 10.]),
            ("bisector", [2f64.sqrt(), 2f64.sqrt()]),
            ("intersection", [0., 0.]),
        ] {
            assert!(
                crate::geometry::distance(e.point(id).unwrap(), expected) < 1e-8,
                "{id}"
            );
        }
        assert_eq!(e.path("curve").unwrap().first(), Some(&[0., 0.]));
    }
    #[test]
    fn rejects_unknown_and_external_entities() {
        let p = include_str!("../tests/fixtures/basic.sm2d");
        assert!(import(&p.replace("type=\"single\"", "type=\"unknown\""), None, "M").is_err());
        assert!(
            import(
                "<!DOCTYPE pattern [<!ENTITY x SYSTEM 'file:///secret'>]><pattern>&x;</pattern>",
                None,
                "M"
            )
            .is_err()
        );
    }
}
