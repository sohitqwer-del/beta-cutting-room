//! Stable JSON document contract. Maps use IDs, never array positions.
use beta_marker_core::{MarkKind, Pt, RotationRule};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Supported document format.
pub const DOC_FORMAT: &str = "beta-pattern.doc/v1";
/// A numeric literal or safe formula. Angles use degrees.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    /// Numeric literal.
    Number(f64),
    /// Safe expression.
    Formula(String),
}
impl Default for Value {
    fn default() -> Self {
        Self::Number(0.0)
    }
}
impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Self::Number(v)
    }
}
impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Self::Formula(v.into())
    }
}
/// Complete pattern; cloning creates an immutable undo snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    /// Display groups. Visibility affects editor display only.
    #[serde(default)]
    pub groups: BTreeMap<String, DrawingGroup>,
    /// Optional whole-pattern print label.
    #[serde(default)]
    pub label: Option<String>,
    /// Must equal [`DOC_FORMAT`].
    pub format: String,
    /// Display name.
    pub name: String,
    /// Sizes ordered smallest to largest.
    pub sizes: Vec<String>,
    /// Origin for factory grading.
    pub base_size: String,
    /// Size -> measurement name -> millimetres.
    #[serde(default)]
    pub measurements: BTreeMap<String, BTreeMap<String, f64>>,
    /// Named increments and intermediate formulas (`v.name`).
    #[serde(default)]
    pub variables: BTreeMap<String, Value>,
    /// Construction objects, keyed by stable ID.
    #[serde(default)]
    pub drawing: BTreeMap<String, Object>,
    /// Piece paths and cutting metadata.
    #[serde(default)]
    pub pieces: Vec<PatternPiece>,
    /// Factory rule library, keyed by rule ID.
    #[serde(default)]
    pub rules: BTreeMap<String, GradeRule>,
    /// Attribution and application metadata, never evaluated.
    #[serde(default)]
    pub meta: BTreeMap<String, serde_json::Value>,
}
impl Default for Document {
    fn default() -> Self {
        Self {
            format: DOC_FORMAT.into(),
            groups: BTreeMap::new(),
            label: None,
            name: String::new(),
            sizes: vec!["M".into()],
            base_size: "M".into(),
            measurements: BTreeMap::new(),
            variables: BTreeMap::new(),
            drawing: BTreeMap::new(),
            pieces: vec![],
            rules: BTreeMap::new(),
            meta: BTreeMap::new(),
        }
    }
}
/// Named drawing geometry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Object {
    /// Constructed point.
    Point {
        /// Construction operation.
        operation: PointOp,
    },
    /// Directed line segment.
    Line {
        /// Start point ID.
        from: String,
        /// End point ID.
        to: String,
    },
    /// Cubic Bezier.
    Cubic {
        /// Start, control 1, control 2, end IDs.
        points: [String; 4],
    },
    /// Interpolating Hermite spline.
    Spline {
        /// Path point IDs.
        points: Vec<String>,
        /// Derivative at each point; empty means automatic tangents.
        #[serde(default)]
        tangents: Vec<[Value; 2]>,
    },
    /// Circular arc.
    Arc {
        /// Centre point ID.
        center: String,
        /// Radius, mm.
        radius: Value,
        /// Start angle, degrees.
        start: Value,
        /// Signed sweep, at most one turn.
        sweep: Value,
    },
    /// Circular arc specified by signed arc length.
    ArcLength {
        /// Centre point ID.
        center: String,
        /// Radius in millimetres.
        radius: Value,
        /// Start angle in degrees.
        start: Value,
        /// Signed length, at most one circumference.
        length: Value,
    },
    /// Rotated elliptical arc; angles are ellipse parameters, not polar bearings.
    EllipticalArc {
        /// Centre point ID.
        center: String,
        /// X semi-axis length.
        radius_x: Value,
        /// Y semi-axis length.
        radius_y: Value,
        /// Ellipse rotation, degrees.
        rotation: Value,
        /// Start parameter, degrees.
        start: Value,
        /// Signed parameter sweep, at most one turn.
        sweep: Value,
    },
    /// Connected cubic Bezier segments.
    CubicPath {
        /// Four point IDs per segment, sharing consecutive endpoints.
        segments: Vec<[String; 4]>,
    },
    /// An interval of a path, measured by arc length.
    CutPath {
        /// Source path ID.
        curve: String,
        /// Distance from its start.
        start: Value,
        /// Positive interval length.
        length: Value,
    },
    /// Translate any point or path without changing its source.
    Move {
        /// Source drawing object.
        source: String,
        /// X movement.
        dx: Value,
        /// Y movement.
        dy: Value,
    },
    /// Rotate any point or path about a construction point.
    Rotate {
        /// Source drawing object.
        source: String,
        /// Rotation origin.
        pivot: String,
        /// Counter-clockwise angle, degrees.
        angle: Value,
    },
    /// Reflect any point or path about an infinite line.
    MirrorLine {
        /// Source drawing object.
        source: String,
        /// First axis point.
        a: String,
        /// Second axis point.
        b: String,
    },
    /// Reflect about a horizontal or vertical axis through a point.
    MirrorAxis {
        /// Source drawing object.
        source: String,
        /// Axis origin.
        origin: String,
        /// Horizontal or vertical axis.
        axis: Axis,
    },
}
/// Point construction; references may point forward in the drawing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum PointOp {
    /// First or last sampled endpoint of a path.
    PathEndpoint {
        /// Source curve.
        curve: String,
        /// Zero selects start; one selects end.
        index: usize,
    },
    /// Cartesian coordinates.
    Absolute {
        /// X coordinate.
        x: Value,
        /// Y coordinate.
        y: Value,
    },
    /// Signed distance on an infinite directed line.
    AlongLine {
        /// Start ID.
        from: String,
        /// Direction ID.
        to: String,
        /// Distance, mm.
        distance: Value,
    },
    /// Polar displacement.
    Polar {
        /// Origin ID.
        from: String,
        /// Absolute angle, degrees.
        angle: Value,
        /// Distance, mm.
        distance: Value,
    },
    /// Midpoint.
    Midpoint {
        /// First ID.
        a: String,
        /// Second ID.
        b: String,
    },
    /// Perpendicular projection on an infinite line.
    Foot {
        /// Point ID.
        point: String,
        /// Line start.
        a: String,
        /// Line end.
        b: String,
    },
    /// Left-normal displacement from a.
    Normal {
        /// Origin ID.
        a: String,
        /// Direction ID.
        b: String,
        /// Distance, mm.
        distance: Value,
    },
    /// Internal angle bisector.
    Bisector {
        /// Vertex ID.
        vertex: String,
        /// First arm ID.
        a: String,
        /// Second arm ID.
        b: String,
        /// Distance, mm.
        distance: Value,
    },
    /// Two infinite lines.
    LineIntersection {
        /// First line point IDs.
        a: [String; 2],
        /// Second line point IDs.
        b: [String; 2],
    },
    /// Intersection of flattened line/curve segments, in first-object order.
    Intersection {
        /// First object ID.
        a: String,
        /// Second object ID.
        b: String,
        /// Zero-based intersection index.
        #[serde(default)]
        index: usize,
    },
    /// X coordinate of one point and Y coordinate of another.
    #[serde(rename = "intersect_xy")]
    IntersectXY {
        /// Point supplying X.
        x_point: String,
        /// Point supplying Y.
        y_point: String,
    },
    /// A line meets a circle about the shoulder centre; index follows line direction.
    Shoulder {
        /// Circle centre.
        center: String,
        /// Shoulder length (circle radius).
        length: Value,
        /// Line origin.
        a: String,
        /// Line direction point.
        b: String,
        /// Zero-based solution index.
        index: usize,
    },
    /// Right-angle vertex on a line, opposite the supplied hypotenuse.
    Triangle {
        /// First hypotenuse end.
        a: String,
        /// Second hypotenuse end.
        b: String,
        /// Axis first point.
        axis_a: String,
        /// Axis second point.
        axis_b: String,
        /// Zero-based solution index along the axis.
        index: usize,
    },
    /// Infinite line intersected with an angled axis.
    LineAxis {
        /// First line point.
        a: String,
        /// Second line point.
        b: String,
        /// Axis origin.
        origin: String,
        /// Axis angle, degrees.
        angle: Value,
    },
    /// Finite curve intersected with an infinite angled axis.
    CurveAxis {
        /// Curve ID.
        curve: String,
        /// Axis origin.
        origin: String,
        /// Axis angle, degrees.
        angle: Value,
        /// Intersection index in curve order.
        index: usize,
    },
    /// Exact intersection of two circles; index zero is left of centre A to B.
    CircleIntersection {
        /// First centre.
        a: String,
        /// First radius.
        radius_a: Value,
        /// Second centre.
        b: String,
        /// Second radius.
        radius_b: Value,
        /// Solution index (0 or 1; tangent has only 0).
        index: usize,
    },
    /// Exact intersection of two circular arcs, filtered by their signed sweeps.
    ArcIntersection {
        /// First circular arc ID.
        a: String,
        /// Second circular arc ID.
        b: String,
        /// Solution index, left of centres first.
        index: usize,
    },
    /// Circle and infinite line intersection.
    CircleLine {
        /// Circle centre.
        center: String,
        /// Circle radius.
        radius: Value,
        /// Line start.
        a: String,
        /// Line direction.
        b: String,
        /// Solution index in line order.
        index: usize,
    },
    /// Circular arc and infinite line intersection.
    ArcLine {
        /// Circular arc ID.
        arc: String,
        /// Line start.
        a: String,
        /// Line direction.
        b: String,
        /// Solution index in line order.
        index: usize,
    },
    /// Circle tangent contact from an external point.
    CircleTangent {
        /// External point.
        point: String,
        /// Circle centre.
        center: String,
        /// Circle radius.
        radius: Value,
        /// Contact index, counter-clockwise from centre-to-point first.
        index: usize,
    },
    /// Tangent contact constrained to a circular arc.
    ArcTangent {
        /// External point.
        point: String,
        /// Circular arc ID.
        arc: String,
        /// Contact index after sweep filtering.
        index: usize,
    },
    /// Point at distance on an arc, spline or any other path.
    CurvePoint {
        /// Source path ID.
        curve: String,
        /// Distance from path start, millimetres.
        distance: Value,
    },
    /// Extend a dart leg to match the other leg length along its ray.
    TrueDart {
        /// Dart apex.
        apex: String,
        /// Leg endpoint to adjust.
        leg: String,
        /// Other leg endpoint supplying the target length.
        other: String,
    },
}
/// Directed edge in a closed piece path.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    /// Line, cubic, spline or arc ID.
    pub object: String,
    /// Traverse backwards.
    #[serde(default)]
    pub reversed: bool,
    /// Outward allowance, mm.
    #[serde(default)]
    pub allowance: Value,
    /// Join at this edge's start.
    #[serde(default)]
    pub corner: Corner,
}
/// Allowance corner style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Corner {
    /// Intersection, limited to ten times the larger allowance.
    #[default]
    Miter,
    /// Tip clipped perpendicular to the bisector.
    Square,
    /// Direct join of shifted edge endpoints.
    Bevel,
    /// Neighbour edge reflected about hem normal before intersection.
    Mirrored,
}
/// Parametric piece marks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Annotation {
    /// Marker mark referring to drawing points.
    Points {
        /// Mark category.
        mark: MarkKind,
        /// Point IDs.
        points: Vec<String>,
        /// Optional text.
        #[serde(default)]
        text: Option<String>,
    },
    /// Button centre, exported as a drill with descriptive text.
    Button {
        /// Point ID.
        point: String,
        /// Diameter, mm.
        diameter: Value,
    },
    /// Notch measured from a directed edge's start.
    Notch {
        /// Piece edge index.
        edge: usize,
        /// Distance, mm.
        distance: Value,
        /// Depth, mm.
        depth: Value,
    },
    /// A shaped seam matching mark.
    ShapedNotch {
        /// Piece edge index.
        edge: usize,
        /// Distance from directed edge start.
        distance: Value,
        /// Inward depth.
        depth: Value,
        /// Width across the edge (ignored for slit).
        width: Value,
        /// Mark shape.
        shape: NotchType,
    },
    /// Create notches at every intersection of a marking path with piece seams.
    SeamIntersections {
        /// Drawing path ID.
        path: String,
        /// Optional piece-edge filter. Used to keep a mirrored guide on its
        /// own half after unfolding; absent means every edge.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        edges: Option<Vec<usize>>,
        /// Notch depth.
        depth: Value,
        /// Notch width.
        width: Value,
        /// Mark shape.
        shape: NotchType,
    },
}
/// Closed piece path and fabric rules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PatternPiece {
    /// Internal open or closed marking paths.
    #[serde(default)]
    pub internal_paths: Vec<InternalPath>,
    /// Named anchor points for matching and assembly.
    #[serde(default)]
    pub anchors: BTreeMap<String, String>,
    /// Grain by origin, angle and length; used when `grain` is absent.
    #[serde(default)]
    pub grain_angle: Option<GrainAngle>,
    /// Label location; exports name, size, quantity and fabric at this point.
    #[serde(default)]
    pub label_point: Option<String>,
    /// Stable unique ID.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Closed, connected path in traversal order.
    pub edges: Vec<Edge>,
    /// Normal copies per garment.
    #[serde(default = "one")]
    pub quantity: u32,
    /// Extra mirrored copies per garment.
    #[serde(default)]
    pub mirror_quantity: u32,
    /// Material grouping.
    #[serde(default)]
    pub material: Option<String>,
    /// Grain line point IDs.
    #[serde(default)]
    pub grain: Option<[String; 2]>,
    /// Symmetry line point IDs; the outline describes a full piece.
    #[serde(default)]
    pub fold_line: Option<[String; 2]>,
    /// Cutting direction restriction.
    #[serde(default)]
    pub rotation: RotationRule,
    /// Extra clearance, mm.
    #[serde(default)]
    pub buffer: Value,
    /// Notches, drills, buttons, lines and labels.
    #[serde(default)]
    pub annotations: Vec<Annotation>,
    /// Outline point ID -> factory rule ID. Empty means measurement grading.
    #[serde(default)]
    pub grade_points: BTreeMap<String, String>,
}
fn one() -> u32 {
    1
}
/// Factory rule in millimetres.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GradeRule {
    /// Rule interpretation.
    #[serde(default)]
    pub mode: GradeMode,
    /// Size -> [dx,dy]. Increment at i is size i minus size i-1.
    pub deltas: BTreeMap<String, Pt>,
}
/// Interpretation of rule table entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GradeMode {
    /// Movement relative to base size (cumulative).
    #[default]
    Relative,
    /// Integrate adjacent-size increments outward from base.
    Incremental,
}
/// Flattened and graded output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GradedPiece {
    /// Directed stitch paths after grading, in piece-edge order.
    /// These exclude seam allowance and support assembly length checks.
    #[serde(default)]
    pub seam_edges: Vec<Vec<Pt>>,
    /// Marker-ready cut piece.
    pub piece: beta_marker_core::Piece,
    /// Stitch outline before allowance.
    pub seam: Vec<Pt>,
    /// Evaluated points, graded where attached to this outline.
    pub points: BTreeMap<String, Pt>,
}
/// Size overlay aligned at one construction point.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Nest {
    /// Common reference point ID.
    pub reference: String,
    /// All geometry translated to reference [0,0].
    pub sizes: BTreeMap<String, Vec<GradedPiece>>,
}

/// Horizontal or vertical reflection axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Axis {
    /// Horizontal axis, Y is reflected.
    Horizontal,
    /// Vertical axis, X is reflected.
    Vertical,
}
/// A named set of drawing objects. Hidden groups remain available to formulas.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DrawingGroup {
    /// Display name.
    pub name: String,
    /// Stable drawing IDs.
    pub objects: Vec<String>,
    /// Whether to display these objects.
    #[serde(default = "visible")]
    pub visible: bool,
}
fn visible() -> bool {
    true
}
/// A piece's open or closed internal marking path, not a cutout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InternalPath {
    /// Display label.
    pub name: String,
    /// Connected directed edges. Allowance fields are ignored.
    pub edges: Vec<Edge>,
    /// Require and preserve closure.
    #[serde(default)]
    pub closed: bool,
}
/// An angle-defined grain line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrainAngle {
    /// Start point ID.
    pub origin: String,
    /// Direction in degrees.
    pub angle: Value,
    /// Line length, millimetres.
    pub length: Value,
}
/// Supported physical notch shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotchType {
    /// Single slit.
    #[default]
    Slit,
    /// V with its tip inward.
    V,
    /// Slit with a cross bar.
    T,
    /// Rectangular U-shaped notch.
    Castle,
}
