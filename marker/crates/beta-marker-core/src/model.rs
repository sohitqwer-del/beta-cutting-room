//! BETA Marker data contract: job (input) and result (output).
//!
//! Every crate, the web app and the benchmark use these types. JSON field names are
//! snake_case. See `marker/SPEC.md` section 3 for the meaning of each field.
//!
//! Coordinate system
//! - Units are millimetres everywhere.
//! - Marker: `x` runs along the fabric length (warp, the grain direction), `y` runs across
//!   the width. The usable fabric is `0 <= y <= fabric.width`, and the marker starts at `x = 0`.
//! - Pieces keep the coordinates they had in the pattern file.
//! - A placement maps a piece point `p` to the marker as
//!   `p' = R(rotation_deg) * M(p) + translate`, where `M(x, y) = (-x, y)` if `mirrored`,
//!   else the identity, and `R` is a counter-clockwise rotation (y axis up).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const JOB_FORMAT: &str = "beta-marker.job/v1";
pub const RESULT_FORMAT: &str = "beta-marker.result/v1";

/// A 2D point `[x, y]` in millimetres.
pub type Pt = [f64; 2];

/// A marker job: fabric, rules, pieces and the size ratio to nest.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Job {
    /// Always [`JOB_FORMAT`].
    pub format: String,
    pub name: String,
    pub fabric: Fabric,
    #[serde(default)]
    pub spacing: Spacing,
    pub pieces: Vec<Piece>,
    /// Garments per size in this marker (the size ratio). `None` = one garment of every piece.
    #[serde(default)]
    pub order: Option<Order>,
    /// Stripe/check matching rules between match points (needs `fabric.repeat`).
    #[serde(default)]
    pub match_rules: Vec<MatchRule>,
    /// Free-form metadata (source file, customer style number, ...). Never used for nesting.
    #[serde(default)]
    pub meta: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Fabric {
    /// Cuttable width in mm (selvedges already removed).
    pub width: f64,
    /// Longest marker the table allows, in mm. `None` = no limit.
    #[serde(default)]
    pub max_length: Option<f64>,
    #[serde(default)]
    pub nap: Nap,
    /// Fabric has no face side, so any copy may be turned face down (placed mirrored).
    /// Counts then apply to `quantity + mirror_quantity` per piece, whatever the mirror state.
    #[serde(default)]
    pub allow_flip: bool,
    /// Allowed tilt off the grain line, in degrees, in both directions. 0 = exact grain.
    #[serde(default)]
    pub tilt_deg: f64,
    /// How the fabric is spread: open, folded or tubular. See [`Layout`].
    #[serde(default)]
    pub layout: Layout,
    /// Areas that must not be cut (holes, flaws), in marker coordinates.
    #[serde(default)]
    pub defects: Vec<Zone>,
    /// Shade bands: every garment bundle must lie inside one band.
    #[serde(default)]
    pub shade: Option<ShadeRule>,
    /// Stripe/check repeat of the fabric, in marker coordinates.
    #[serde(default)]
    pub repeat: Option<Repeat>,
    /// Expected fabric shrinkage. Pieces are enlarged before nesting (see `rules::effective_job`).
    #[serde(default)]
    pub shrinkage: Shrinkage,
}

/// How the fabric is spread on the table.
///
/// - `open`: one layer, face up. Each placement cuts one copy.
/// - `folded`: folded along the length, fold at `y = width` (`width` is the folded width).
///   Each placement cuts two layers: the placed copy and its mirror image.
/// - `tubular`: knitted tube laid flat, folds at `y = 0` and `y = width`. Two layers like `folded`.
///
/// A piece with `fold_line` may be placed `on_fold` (only its half on the marker, fold line on
/// a fold edge); that placement cuts one full piece.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Layout {
    #[default]
    Open,
    Folded,
    Tubular,
}

/// A closed area in marker coordinates (mm).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Zone {
    #[serde(default)]
    pub id: String,
    pub polygon: Vec<Pt>,
    #[serde(default)]
    pub note: Option<String>,
}

/// Shade bands: the marker is divided into bands; all placements of one bundle (garment)
/// must lie inside the same band.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ShadeRule {
    pub axis: ShadeAxis,
    /// Band size in mm: band `k` covers `[k * band_mm, (k + 1) * band_mm)` on the axis.
    pub band_mm: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShadeAxis {
    /// Bands are sections along the marker length (x).
    Length,
    /// Bands are lanes across the width (y).
    Width,
}

/// Stripe/check repeat in marker coordinates. Two points are "in phase" when their
/// coordinates differ by a whole number of periods (plus the rule offset).
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Repeat {
    /// Repeat period along the length (x), mm. `None` = no repeat on this axis.
    #[serde(default)]
    pub period_x: Option<f64>,
    /// Repeat period across the width (y), mm.
    #[serde(default)]
    pub period_y: Option<f64>,
    /// Position of a reference stripe on each axis, mm.
    #[serde(default)]
    pub offset_x: f64,
    #[serde(default)]
    pub offset_y: f64,
    /// Allowed matching error, mm (1 mm when 0).
    #[serde(default)]
    pub tolerance_mm: f64,
}

/// Shrinkage in percent. Positive values enlarge pieces: along the grain by `length_pct`,
/// across the grain by `width_pct`.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Shrinkage {
    #[serde(default)]
    pub length_pct: f64,
    #[serde(default)]
    pub width_pct: f64,
}

/// A matching rule between two match points (stripes/checks).
///
/// Applies within each bundle: the copy of piece `a` and the copy of piece `b` in the same
/// bundle. With `b = None`, point `a` itself must be in phase with the repeat (plus offset).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchRule {
    pub a: PointRef,
    #[serde(default)]
    pub b: Option<PointRef>,
    pub axes: MatchAxes,
    /// Required phase difference `a - b` (or `a - repeat offset`), mm.
    #[serde(default)]
    pub offset: Pt,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PointRef {
    pub piece_id: String,
    pub point_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchAxes {
    X,
    Y,
    Both,
}

/// Fabric direction rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Nap {
    /// No nap: every piece may lie at 0 or 180 degrees to the grain.
    #[default]
    None,
    /// One-way fabric (nap, pile, one-way print): every piece must point the same way.
    OneWay,
    /// One way per garment: all pieces of one bundle point the same way (0 or 180);
    /// different bundles may differ.
    OneWayPerBundle,
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Spacing {
    /// Minimum gap between two pieces, mm.
    #[serde(default)]
    pub piece_gap: f64,
    /// Minimum gap between a piece and the fabric edges (y = 0, y = width, x = 0), mm.
    #[serde(default)]
    pub edge_gap: f64,
}

/// One pattern piece of one size, as drafted.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Piece {
    /// Unique in the job, for example `"SHIRT01-FRONT-M"`.
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub style: Option<String>,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub material: Option<String>,
    /// Cut line as a closed polygon (the last point does not repeat the first). Any winding.
    pub outline: Vec<Pt>,
    /// Grain line `[from, to]`. The direction `from -> to` must lie along the marker `+x`
    /// (or `-x` when 180 degrees is allowed). `None` = the piece x axis is the grain.
    #[serde(default)]
    pub grain: Option<[Pt; 2]>,
    /// Copies per garment as drafted.
    #[serde(default = "one")]
    pub quantity: u32,
    /// Extra mirrored copies per garment (left/right pair: quantity 1, mirror_quantity 1).
    #[serde(default)]
    pub mirror_quantity: u32,
    #[serde(default)]
    pub rotation: RotationRule,
    /// Extra clearance around this piece in mm, added to `spacing.piece_gap`.
    #[serde(default)]
    pub buffer: f64,
    /// Internal marks: notches, drills, internal lines, text. Carried to the output.
    #[serde(default)]
    pub marks: Vec<Mark>,
    /// Named reference points for stripe/check matching, in piece coordinates.
    #[serde(default)]
    pub match_points: Vec<MatchPoint>,
    /// Symmetry line `[from, to]` of a symmetric piece (for example a back cut on fold).
    /// `outline` is always the full piece. On folded/tubular fabric the piece may be placed
    /// `on_fold`: only the half on the left of `from -> to` is placed, with the line on a fold edge.
    /// Either reflected representation of this half is allowed without enabling full-piece flips.
    #[serde(default)]
    pub fold_line: Option<[Pt; 2]>,
    /// Fusing block: nest a rectangle around the piece (in the grain-aligned frame) enlarged
    /// by this margin in mm, for fusing and re-cutting. `None` = nest the outline.
    #[serde(default)]
    pub block_margin: Option<f64>,
    /// Per-piece override of `fabric.allow_flip` (see `rules::flip_allowed`).
    #[serde(default)]
    pub flip: Option<bool>,
    /// Per-piece override of `fabric.tilt_deg`, degrees.
    #[serde(default)]
    pub tilt_deg: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct MatchPoint {
    pub id: String,
    pub point: Pt,
}

fn one() -> u32 {
    1
}

/// Rotations a piece may take, relative to its grain-aligned position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RotationRule {
    /// Use the fabric rule: `nap: none` gives `flip180`, `nap: one_way` gives `fixed`.
    #[default]
    Inherit,
    /// 0 degrees only.
    Fixed,
    /// 0 or 180 degrees.
    Flip180,
    /// 0, 90, 180 or 270 degrees (for example bias trims or fusing).
    Quarter,
    /// Any angle (non-woven, interlining).
    Free,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mark {
    pub kind: MarkKind,
    /// Notch: `[position, direction point]` or `[position]`. Drill: `[centre]`.
    /// Line: polyline points. Text: `[anchor]`.
    pub points: Vec<Pt>,
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkKind {
    Notch,
    Drill,
    Line,
    Text,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Order {
    /// Garments per size, for example `{"S": 1, "M": 2, "L": 1}`.
    pub sizes: BTreeMap<String, u32>,
}

/// A finished marker.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarkerResult {
    /// Always [`RESULT_FORMAT`].
    pub format: String,
    pub job_name: String,
    /// Fabric width used, mm.
    pub width: f64,
    /// Marker length, mm: the largest x of any placed outline point.
    pub length: f64,
    /// Sum of placed outline areas, mm^2 (buffers and gaps not included).
    pub piece_area: f64,
    /// `piece_area / (width * length)`, a fraction from 0 to 1.
    pub efficiency: f64,
    /// For example `"sparrow 0.3.0"` or `"manual"`.
    pub engine: String,
    #[serde(default)]
    pub seed: Option<u64>,
    /// Wall-clock nesting time, seconds.
    #[serde(default)]
    pub runtime_s: f64,
    pub placements: Vec<Placement>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

/// One placed copy of a piece.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Placement {
    pub piece_id: String,
    /// 0-based copy number among copies with the same `piece_id` and `mirrored` value.
    pub copy: u32,
    pub mirrored: bool,
    /// Counter-clockwise, degrees, applied after mirroring. Includes the grain alignment.
    pub rotation_deg: f64,
    pub translate: Pt,
    /// Locked by the user in the editor; re-nesting keeps it in place.
    #[serde(default)]
    pub locked: bool,
    /// Garment number (0-based) this copy belongs to. Needed for shade bands,
    /// one-way-per-bundle nap and matching rules; `None` = not assigned.
    /// With explicit layers this is the first useful layer's garment.
    #[serde(default)]
    pub bundle: Option<u32>,
    /// Placed as a half on a fold edge (see [`Layout`] and `Piece::fold_line`).
    #[serde(default)]
    pub on_fold: bool,
    /// Explicit useful-copy assignments for a layered cut. Empty keeps the legacy
    /// assignment to `bundle`. Otherwise entries are physical normal then mirrored
    /// layer (one normal entry for an on-fold cut). `None` marks surplus, never demand.
    /// A declared symmetric piece or a flip permission can exchange handedness.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layer_allocations: Vec<Option<LayerAllocation>>,
}

/// One useful copy from a physical fabric layer, assigned to a garment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayerAllocation {
    /// Canonical garment number from `rules::bundle_sizes`.
    pub bundle: u32,
    /// Handedness of the demanded copy, independent of the displayed source half.
    pub mirrored: bool,
}

/// Output of the independent validator (`beta_marker_core::validate`).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ValidationReport {
    /// `true` when `errors` is empty.
    pub ok: bool,
    pub errors: Vec<Issue>,
    pub warnings: Vec<Issue>,
    /// Recomputed from the geometry, not copied from the result.
    pub length: f64,
    pub piece_area: f64,
    pub efficiency: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Issue {
    /// Stable machine code, for example `"overlap"`, `"outside_width"`, `"missing_piece"`,
    /// `"extra_piece"`, `"grain"`, `"mirror"`, `"gap"`, `"too_long"`, `"unknown_piece"`.
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub piece_ids: Vec<String>,
}

#[cfg(test)]
mod json_precision_tests {
    use super::Placement;

    #[test]
    fn locked_coordinates_survive_json_round_trips_exactly() {
        // Public Simon regression: the fast JSON parser changed the last bit of Y.
        let json = r#"{"piece_id":"public-panel","copy":0,"mirrored":false,"rotation_deg":90.0,"translate":[-114.46139526367188,964.6422119140625],"locked":true,"bundle":3,"on_fold":false}"#;
        let placement: Placement = serde_json::from_str(json).unwrap();
        assert_eq!(
            placement.translate,
            [-114.46139526367188, 964.6422119140625]
        );
        let serialized = serde_json::to_string(&placement).unwrap();
        let restored: Placement = serde_json::from_str(&serialized).unwrap();
        assert_eq!(restored.translate, placement.translate);
        assert!(restored.locked);
    }
}
