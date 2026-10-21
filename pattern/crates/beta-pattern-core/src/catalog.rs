//! Machine-readable construction forms. Parameter names match serde fields.
use crate::Object;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Input accepted by an ordered tool parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterType {
    /// One construction point ID.
    PointRef,
    /// Any point or path object.
    ObjectRef,
    /// One line ID.
    LineRef,
    /// One line or curve ID.
    CurveRef,
    /// Ordered point IDs, for a path or control polygon.
    PointRefs,
    /// Number or safe length formula.
    Formula,
    /// Number or safe angle formula, degrees.
    AngleFormula,
    /// Nonnegative integer.
    Index,
    /// One of the supplied string choices.
    Enum,
    /// Array of derivative vectors, each containing two formulas.
    Tangents,
    /// Array of cubic segments, each containing four point IDs.
    CubicSegments,
}
/// One field in a generic construction form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolParameter {
    /// Serialized field name.
    pub name: String,
    /// Plain-English label.
    pub label: String,
    /// Input widget and reference filter.
    pub kind: ParameterType,
    /// Optional default; absent means required.
    pub default: Option<Value>,
    /// Permitted enum values, empty for other types.
    pub choices: Vec<String>,
}
/// A supported construction and its ordered input fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolSpec {
    /// Stable ID: point operation or object kind in snake_case.
    pub id: String,
    /// UI category.
    pub group: String,
    /// Plain-English display name.
    pub name: String,
    /// One-line explanation.
    pub help: String,
    /// Inputs in form order.
    pub parameters: Vec<ToolParameter>,
    /// True for a point operation; false for a path object.
    pub point: bool,
}
/// List all implemented construction forms. IDs are stable across releases.
pub fn catalog() -> Vec<ToolSpec> {
    use ParameterType::*;
    let mut rows = vec![
        (
            "offset_path",
            "Paths",
            "Parallel path",
            "Copy a path at a signed normal distance.",
            false,
            vec![("source", CurveRef), ("distance", Formula)],
        ),
        (
            "absolute",
            "Points",
            "Coordinates",
            "Place a point at X and Y.",
            true,
            vec![("x", Formula), ("y", Formula)],
        ),
        (
            "polar",
            "Points",
            "Distance and angle",
            "Place a point from an origin, distance and angle.",
            true,
            vec![
                ("from", PointRef),
                ("distance", Formula),
                ("angle", AngleFormula),
            ],
        ),
        (
            "along_line",
            "Points",
            "Along line",
            "Measure a signed distance along an infinite line.",
            true,
            vec![("from", PointRef), ("to", PointRef), ("distance", Formula)],
        ),
        (
            "midpoint",
            "Points",
            "Midpoint",
            "Place a point halfway between two points.",
            true,
            vec![("a", PointRef), ("b", PointRef)],
        ),
        (
            "foot",
            "Points",
            "Perpendicular foot",
            "Project a point onto an infinite line.",
            true,
            vec![("point", PointRef), ("a", PointRef), ("b", PointRef)],
        ),
        (
            "normal",
            "Points",
            "Normal",
            "Measure along the left normal of a line.",
            true,
            vec![("a", PointRef), ("b", PointRef), ("distance", Formula)],
        ),
        (
            "bisector",
            "Points",
            "Angle bisector",
            "Measure along the internal angle bisector.",
            true,
            vec![
                ("vertex", PointRef),
                ("a", PointRef),
                ("b", PointRef),
                ("distance", Formula),
            ],
        ),
        (
            "line_intersection",
            "Intersections",
            "Line intersection",
            "Intersect two infinite lines.",
            true,
            vec![("a", PointRefs), ("b", PointRefs)],
        ),
        (
            "intersection",
            "Intersections",
            "Curve intersection",
            "Choose an intersection of two finite paths.",
            true,
            vec![("a", CurveRef), ("b", CurveRef), ("index", Index)],
        ),
        (
            "line",
            "Lines and curves",
            "Line",
            "Join two construction points.",
            false,
            vec![("from", PointRef), ("to", PointRef)],
        ),
        (
            "cubic",
            "Lines and curves",
            "Cubic Bezier",
            "Draw a cubic from four control points.",
            false,
            vec![("points", PointRefs)],
        ),
        (
            "spline",
            "Lines and curves",
            "Interpolating spline",
            "Pass through points with optional derivative vectors.",
            false,
            vec![("points", PointRefs), ("tangents", Tangents)],
        ),
        (
            "arc",
            "Lines and curves",
            "Circular arc",
            "Draw an arc with a centre, radius and signed sweep.",
            false,
            vec![
                ("center", PointRef),
                ("radius", Formula),
                ("start", AngleFormula),
                ("sweep", AngleFormula),
            ],
        ),
    ];
    rows.extend(vec![
        (
            "path_endpoint",
            "Points",
            "Path endpoint",
            "Reference the start or end of any path.",
            true,
            vec![("curve", CurveRef), ("index", Index)],
        ),
        (
            "intersect_xy",
            "Intersections",
            "X and Y intersection",
            "Take X from one point and Y from another.",
            true,
            vec![("x_point", PointRef), ("y_point", PointRef)],
        ),
        (
            "shoulder",
            "Points",
            "Shoulder point",
            "Intersect a shoulder-length circle with a direction line.",
            true,
            vec![
                ("center", PointRef),
                ("length", Formula),
                ("a", PointRef),
                ("b", PointRef),
                ("index", Index),
            ],
        ),
        (
            "triangle",
            "Points",
            "Right triangle",
            "Place a right-angle vertex on an axis opposite a hypotenuse.",
            true,
            vec![
                ("a", PointRef),
                ("b", PointRef),
                ("axis_a", PointRef),
                ("axis_b", PointRef),
                ("index", Index),
            ],
        ),
        (
            "line_axis",
            "Intersections",
            "Line and axis",
            "Intersect an infinite line and an angled axis.",
            true,
            vec![
                ("a", PointRef),
                ("b", PointRef),
                ("origin", PointRef),
                ("angle", AngleFormula),
            ],
        ),
        (
            "curve_axis",
            "Intersections",
            "Curve and axis",
            "Intersect a finite curve and an infinite angled axis.",
            true,
            vec![
                ("curve", CurveRef),
                ("origin", PointRef),
                ("angle", AngleFormula),
                ("index", Index),
            ],
        ),
        (
            "circle_intersection",
            "Intersections",
            "Two circles",
            "Choose an exact intersection of two circles.",
            true,
            vec![
                ("a", PointRef),
                ("radius_a", Formula),
                ("b", PointRef),
                ("radius_b", Formula),
                ("index", Index),
            ],
        ),
        (
            "arc_intersection",
            "Intersections",
            "Two arcs",
            "Choose an exact intersection within both arc sweeps.",
            true,
            vec![("a", CurveRef), ("b", CurveRef), ("index", Index)],
        ),
        (
            "circle_line",
            "Intersections",
            "Circle and line",
            "Intersect a circle with an infinite line.",
            true,
            vec![
                ("center", PointRef),
                ("radius", Formula),
                ("a", PointRef),
                ("b", PointRef),
                ("index", Index),
            ],
        ),
        (
            "arc_line",
            "Intersections",
            "Arc and line",
            "Intersect a circular arc with an infinite line.",
            true,
            vec![
                ("arc", CurveRef),
                ("a", PointRef),
                ("b", PointRef),
                ("index", Index),
            ],
        ),
        (
            "circle_tangent",
            "Points",
            "Circle tangent contact",
            "Find a tangent contact from an external point.",
            true,
            vec![
                ("point", PointRef),
                ("center", PointRef),
                ("radius", Formula),
                ("index", Index),
            ],
        ),
        (
            "arc_tangent",
            "Points",
            "Arc tangent contact",
            "Find a tangent contact within a circular arc sweep.",
            true,
            vec![("point", PointRef), ("arc", CurveRef), ("index", Index)],
        ),
        (
            "curve_point",
            "Points",
            "Cut curve at length",
            "Place a point at length on an arc, spline or path.",
            true,
            vec![("curve", CurveRef), ("distance", Formula)],
        ),
        (
            "true_dart",
            "Points",
            "True dart leg",
            "Extend a dart leg along its ray to match the other leg.",
            true,
            vec![("apex", PointRef), ("leg", PointRef), ("other", PointRef)],
        ),
        (
            "arc_length",
            "Lines and curves",
            "Arc by length",
            "Draw a circular arc from radius and signed length.",
            false,
            vec![
                ("center", PointRef),
                ("radius", Formula),
                ("start", AngleFormula),
                ("length", Formula),
            ],
        ),
        (
            "elliptical_arc",
            "Lines and curves",
            "Elliptical arc",
            "Draw an arc of a rotated ellipse.",
            false,
            vec![
                ("center", PointRef),
                ("radius_x", Formula),
                ("radius_y", Formula),
                ("rotation", AngleFormula),
                ("start", AngleFormula),
                ("sweep", AngleFormula),
            ],
        ),
        (
            "cubic_path",
            "Lines and curves",
            "Bezier path",
            "Join cubic segments with shared endpoints.",
            false,
            vec![("segments", CubicSegments)],
        ),
        (
            "cut_path",
            "Lines and curves",
            "Cut path interval",
            "Extract a length interval from an arc, spline or path.",
            false,
            vec![("curve", CurveRef), ("start", Formula), ("length", Formula)],
        ),
        (
            "move",
            "Operations",
            "Move copy",
            "Translate a point or path while retaining its source.",
            false,
            vec![("source", ObjectRef), ("dx", Formula), ("dy", Formula)],
        ),
        (
            "rotate",
            "Operations",
            "Rotate copy",
            "Rotate a point or path about a pivot.",
            false,
            vec![
                ("source", ObjectRef),
                ("pivot", PointRef),
                ("angle", AngleFormula),
            ],
        ),
        (
            "mirror_line",
            "Operations",
            "Mirror by line",
            "Reflect a point or path across a line.",
            false,
            vec![("source", ObjectRef), ("a", PointRef), ("b", PointRef)],
        ),
        (
            "mirror_axis",
            "Operations",
            "Mirror by axis",
            "Reflect a point or path across a horizontal or vertical axis.",
            false,
            vec![("source", ObjectRef), ("origin", PointRef), ("axis", Enum)],
        ),
    ]);
    rows.into_iter()
        .map(|(id, group, name, help, point, fields)| ToolSpec {
            id: id.into(),
            group: group.into(),
            name: name.into(),
            help: help.into(),
            point,
            parameters: fields
                .into_iter()
                .map(|(name, kind)| ToolParameter {
                    label: name.replace('_', " "),
                    name: name.into(),
                    default: match kind {
                        Index => Some(json!(0)),
                        Tangents => Some(json!([])),
                        _ => None,
                    },
                    kind,
                    choices: if name == "axis" {
                        vec!["horizontal".into(), "vertical".into()]
                    } else {
                        vec![]
                    },
                })
                .collect(),
        })
        .collect()
}
/// Build a serde construction from a catalog form. Does not mutate a document.
pub fn construct(id: &str, fields: serde_json::Map<String, Value>) -> Result<Object> {
    let spec = catalog()
        .into_iter()
        .find(|s| s.id == id)
        .ok_or_else(|| anyhow::anyhow!("unknown tool '{id}'"))?;
    let mut fields = fields;
    ensure!(
        fields
            .keys()
            .all(|k| spec.parameters.iter().any(|p| &p.name == k)),
        "unknown tool parameter"
    );
    for p in spec.parameters {
        if !fields.contains_key(&p.name) {
            if let Some(v) = p.default {
                fields.insert(p.name, v);
            } else {
                anyhow::bail!("missing parameter '{}'", p.name);
            }
        }
    }
    let value = if spec.point {
        fields.insert("op".into(), json!(id));
        json!({"kind":"point","operation":fields})
    } else {
        fields.insert("kind".into(), json!(id));
        Value::Object(fields)
    };
    Ok(serde_json::from_value(value)?)
}
