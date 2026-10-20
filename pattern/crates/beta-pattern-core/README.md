# BETA Pattern core

Offline Rust. Millimetres, y up, counter-clockwise degrees.
JSON format: `beta-pattern.doc/v1`. Public types in `src/model.rs` are the contract.

`Document` holds an ordered size list, a base size, measurements per size, variables,
a drawing map, piece paths and a factory rule library. Drawing keys are stable IDs.
A point contains a tagged construction operation. Lines, cubics, interpolating
splines and arcs refer to points by ID. Forward references are allowed; cycles fail.

`Value` is a JSON number or formula string. References use `m.chest` for body
measurements and `v.ease` for variables. Geometry functions use object IDs:
`length(side)`, `distance(A,B)`, `angle(A,B)`, `x(A)`, `y(A)`.
Trig functions take and return degrees; `atan2(y,x)` follows that argument order.
Arithmetic supports `+ - * / ^`, parentheses and `sqrt sin cos tan asin acos atan2
min max abs round`. Powers associate right and bind tighter than unary minus.
Expressions cannot access files or the network.

A piece is a closed list of directed edges. Each edge owns an allowance formula
and its start-corner style. A seam allowance is fabric outside the stitch line.
Marks include notches (small seam matching marks), drills, buttons, lines and text.
Quantities, mirrored pairs, grain, fold, material and rotation rules map to Marker.

With no `grade_points`, the draft evaluates each size's body measurements.
Factory grading evaluates the base size and moves attached outline points using
named rules. Relative rules store movement from base. Incremental rules store
size i minus size i-1, integrated in document size order (subtracted below base).
Unattached vertices interpolate movement by outline distance between attachments.
Attach rules to stitch-outline points, not Bezier control points. Different pieces
may use different grading modes.

Documents derive Clone, PartialEq, Serialize and Deserialize. Editors can keep
immutable snapshots for undo/redo. Evaluation returns new geometry. `GradedPiece`
retains stitch and cut outlines and point IDs. `Nest` aligns sizes at one point.
Optional JSON fields use serde defaults. Breaking changes need a new format tag.

## Calling the library

```rust
use beta_pattern_core::{starters, evaluate, exchange};
use beta_marker_core::Fabric;
use std::collections::BTreeMap;

let doc = starters::bodice();
let sizes = evaluate::grade(&doc, "M", 0.1)?;
let overlay = evaluate::nest(&doc, &doc.sizes, "front.A", 0.1)?;
let job = exchange::to_job(&doc, Fabric { width: 1500.0,
    ..Default::default() }, BTreeMap::from([("M".into(), 2)]), 0.1)?;
# Ok::<(), anyhow::Error>(())
```

`evaluate::Evaluator` resolves individual points, paths and formulas for an editor.
Errors carry the piece, object, variable and size through an anyhow error chain.
Display errors with `{error:#}` to include the dependency context. Geometry IDs in
formulas must be ASCII identifiers (letters, digits, underscores and dots; start
with a letter or underscore). The document itself may use other IDs when they
are referenced directly through model fields.

Spline tangents are Cartesian derivative vectors, not angle/length pairs.
Empty tangents select centred finite differences, with one-sided endpoint tangents.
The evaluator turns Hermite segments into cubics before flattening them.
Construction-line intersections use infinite lines. Curve intersections use finite
flattened segments and return distinct points in first-object traversal order.
Coincident curve portions have no unique intersection; their shared sampled
endpoints are returned. Curve-intersection accuracy follows the flatten tolerance.

## Geometry and tools

Cubics subdivide until their control hull lies within tolerance of the endpoint
segment. Arc sampling limits chord sag. Each curve has at most 65,536 segments.
Lengths and areas use the sampled geometry. Tighten tolerance for final export.
The default CLI tolerance is 0.1 mm. `geometry::area` and clipping reuse Marker.

Allowance widths follow the stitch segments; curved-edge samples inherit that
edge's allowance. Joins support miter, bevel, square and mirrored hem corners.
The miter limit is ten times the larger adjacent allowance. Square joins clip
the tip with a line normal to the bisector, at the larger allowance from the
stitch vertex. Mirrored corners reflect the direction of the narrower-allowance
neighbour about the wider edge's normal, then intersect the shifted lines.
This explicit construction handles a turned hem at a sloped side. It is not a
cloth folding simulation. Both winding directions produce outward allowances.
Self-crossing offsets fail; this version does not trim loops or union them.

`tools` provides dart create/transfer/close/remove, slash-and-spread, knife/box
pleats, split, reflection, seam walking, seam truing, allowance edits and notches.
A dart is a triangular intake that shapes cloth. Transfer closes the old intake
and opens a cut on another edge by rotating one panel about the apex. It keeps
the opening angle and flat fabric area. Closure and transfer require equal-length
legs and a cut point away from the mouth: one boundary chain
rotates about the apex, closing the old legs and opening a slash at that vertex.
Removing a dart instead restores the uncut original outline.

Slash-and-spread rotates the right panel clockwise about a boundary pivot and
bridges the open mouth. Pleats translate that panel and return fold guides.
A knife pleat inserts twice its folded depth; a box pleat inserts four times.
Split and fullness tools accept a single cut with exactly two boundary crossings;
multiple disconnected panels return an error. Truing scales an open seam about
its start to match a target length; it does not automatically reshape a garment.

Tool geometry is returned as new polygons. `insert_polygon` converts it to fixed
drawing objects plus a piece. `add_fold_guides` adds a pleat's internal marks.
These edits do not retain a parametric recipe. Callers can keep the original
document in `history::History` before committing the result. Geometry edits reject
invalid topology. Factory grading interpolates movement along the closed stitch
outline; internal marks and grain points take movement from their nearest stitch
segment. This is a stated approximation for internal features, not a mesh warp.

## DXF and the grade table

`exchange::export_dxf` evaluates all selected sizes and calls
`beta_marker_dxf::export_aama` with the resulting Marker pieces. Each piece-size
gets its own R12 block. Standard layers and garment text follow the existing
public AAMA-style conventions documented in that crate's README. This is not a
certification of compliance with a paywalled ASTM specification.

Grade rules use our own documented, public extension. DXF group code 999 is a
comment. The following comment payloads precede the normal AAMA-style content:

```text
BETA_PATTERN_RULES_BEGIN beta-pattern.rules/v1
BETA_PATTERN_RULES_DATA <up to 200 UTF-8 bytes of compact JSON>
BETA_PATTERN_RULES_DATA <next bytes, with no added separator>
BETA_PATTERN_RULES_END
```

Each payload has its own `999` group pair. Join DATA payloads exactly and decode
the resulting JSON as `exchange::RuleTable`. It records the format, base size,
ordered sizes, rules with modes/deltas, piece-point rule attachments and evaluated
named points per piece/size. The stable point IDs therefore identify which rule
belongs to each point even when curve flattening adds unnamed vertices.
`import_rule_table` reads the extension back. Other AAMA readers can ignore it and
still read every exported size. We do not claim that commercial CAD imports these
rules as native grading. No Gerber/Tukatech rule format is copied or implemented.

The inherited Marker writer preserves outlines, size, quantities/pairs, material,
rotation, buffer, grain and marks. It does not currently preserve `fold_line` or
`block_margin`; the JSON Marker job keeps fold lines. Use the job path when those
fields are needed until the shared DXF contract is extended.

## Starter documents and formulas

Run `cargo run -p beta-pattern-core --example make_starters` from `pattern/` to
regenerate the committed small native examples. It reads only the repository's
open FreeSewing fixtures. Their already-flattened cut outlines become fixed
points/lines; their existing seam allowances are not added again. The converted
Teagan T-shirt and Titan trouser retain FreeSewing MIT attribution and the licence
in `starters/LICENSE.FreeSewing.MIT`. See the source fixture README for assumptions.

`starters/beta-bodice.doc.json` is our original parametric teaching draft. It has
front and back quarter pieces, each cut as one mirrored pair. It is a simplified
waist-to-shoulder bodice without bust darts; fit-test before production use.
No FreeSewing or GPL drafting formulas were used for this block.

The measurement rows `(chest, waist, back_length, neck, shoulder, arm_depth)`, mm:

- S: `(880, 680, 400, 350, 115, 190)`
- M: `(920, 720, 410, 360, 120, 195)`
- L: `(960, 760, 420, 370, 125, 200)`
- XL: `(1000, 800, 430, 380, 130, 205)`

Ease is 80 mm around the body. Quarter chest width is `(chest+80)/4`, quarter
waist width is `(waist+80)/4`, and neck width is `neck/6`. Let these be W, U, N;
let back length be H, shoulder be S and arm depth be R. In waist-origin coordinates:

- Waist centre A = `(0,0)`; side waist B = `(U,0)`.
- Underarm C = `(W,H-R)`; shoulder tip D = `(N+S,H-25)`.
- Neck shoulder E = `(N,H)`; neck centre F = `(0,H-depth)`.
- Front neck depth = `N+10`; back neck depth = `20`.
- Armhole cubic C -> D has controls `(W-30,H-R+20)` and `(N+S-15,H-65)`.
- Neck cubic E -> F has controls `(0.45*N,H)` and `(0,H-0.55*depth)`.
- Remaining boundaries A-B-C and D-E and F-A are straight.
- All edges have 10 mm allowance. Grain is vertical at x = W/2.
- One notch lies halfway along each side seam.

Changing a size's measurements re-evaluates these formulas and both curves.
`starters/verification.json` reports stitch dimensions, stitch/cut areas and the
eight-piece S-XL DXF round trip.

## Seamly2D / Valentina XML import

`seamly::import(pattern_xml, optional_measurement_xml, size_label)` reads our
original XML fixtures. It supports coordinates, along-line, end-line, normal,
bisector, height, shoulder, triangle, line/curve-axis, line/curve intersections,
circle/arc intersections and tangent contacts, cut-curve points, trued dart legs,
lines, simple cubics, cubic paths, interpolating paths, circular/length arcs and
elliptical arcs. Operations include rotation, translation and reflection. Details
can contain curves, internal paths, anchors and grain. Construction-only files
are accepted. Groups preserve visibility. Unsupported calculation elements and
operation types return explicit errors. Display styles, tracing images and print
layouts are warned as omissions.

This is independent format exploration against hand-written BETA fixtures,
not a claim of full Seamly compatibility. Attribute names for newly added families
have not all been verified against application-generated files. `families.sm2d`
documents the currently recognized structures. XML aliases, coordinate reflection,
source-unit length conversion and formula measurement references remain supported.
Generated geometry names such as `Line_*`, per-node allowances and excluded detail
nodes still need explicit conversion. Measurements must be numeric; formulas in
measurement XML fail clearly. XML is limited to 16 MiB / 200,000 nodes. External
entities and DTD resolution are disabled.

## CLI

From the `pattern/` workspace:

```sh
cargo run -p beta-pattern-core --bin beta-pattern -- grade crates/beta-pattern-core/starters/beta-bodice.doc.json -o output
cargo run -p beta-pattern-core --bin beta-pattern -- export-dxf crates/beta-pattern-core/starters/beta-bodice.doc.json --sizes S,M,L -o bodice.dxf
cargo run -p beta-pattern-core --bin beta-pattern -- to-job crates/beta-pattern-core/starters/beta-bodice.doc.json --width 1500 --ratio S=1,M=2 -o job.json
cargo run -p beta-pattern-core --bin beta-pattern -- import-seamly crates/beta-pattern-core/tests/fixtures/basic.sm2d --measurements crates/beta-pattern-core/tests/fixtures/basic.smis -o imported.doc.json
```

Grade writes one `<size>.pieces.json` per size plus `summary.json` with areas and
bounds. Export defaults to every document size if `--sizes` is omitted. Import
uses the size label M. Commands report errors with nonzero exit status. The
library itself has no file I/O; only the CLI and regeneration example read/write.

## Dependencies and licence

Code: Apache-2.0. Existing workspace dependencies remain unchanged. The sole new
crate is [roxmltree 0.21.1](https://docs.rs/crate/roxmltree/0.21.1), licensed
MIT OR Apache-2.0. Its `memchr` dependency already exists in the workspace and is
MIT OR Unlicense (MIT is selected). The formula parser and geometry operations
here are original Rust implementations. No runtime network access is used.

See `REPORT.md` for validation results, limitations and proposed upstream work.

## Generic editor API

`tools::catalog() -> Vec<ToolSpec>` lists supported construction forms. Each spec
has `id`, `group`, `name`, `help`, ordered `parameters`, and `point` (point output).
Each parameter has `name`, `label`, `kind`, `default` (JSON or null), and `choices`.
Kinds serialize as `point_ref`, `line_ref`, `curve_ref`, `point_refs`, `formula`,
`angle_formula`, `index`, `enum`, `tangents`, or `cubic_segments`.
Names are the serde fields; arrays retain their native shape. Angles are degrees.

`tools::apply(&doc, tool_id, new_id, params: serde_json::Map<String, Value>)`
returns a new `Document`. It rejects ID collisions and dependency cycles. The
geometry evaluator reports invalid numbers or impossible constructions.
`tools::construct(tool_id, params)` builds an `Object` without editing a document.

`evaluate::evaluate_document(&doc, size, tolerance)` returns one
`ObjectEvaluation { id, geometry: Option<Geometry>, error: Option<String> }` per
object. Geometry uses serde's externally tagged `Point: [x,y]` or `Path: [[x,y],...]`.
Independent geometry remains visible when another object fails.
`tools::reference_order(&doc, editing_id)` lists references in dependency order;
an edited object and its dependent objects are excluded.
`tools::rename`, `tools::delete`, and `tools::dependents` check direct and formula
references. `tools::move_point` supports Absolute and Polar points; dragging
replaces their formulas with numeric values. All edits return cloned documents.


## Expanded construction and piece API

The catalog now has 35 entries. `object_ref` is an additional parameter kind for
transforming either a point or a path. Transform entries set `point=false`; their
actual geometry type follows `source`. `curve_point` cuts an arc, spline or path
at length. Circular arc points and circle/tangent intersections are exact;
other curve cuts and crossings use the requested flattening tolerance.
`cubic_path` uses `segments: [[start, control1, control2, end], ...]`.
Spline `tangents` use `[[dx,dy], ...]`, or an empty array for automatic tangents.

`tools::transform_group(doc, selection, suffix, tool_id, params)` creates copies
using `move`, `rotate`, `mirror_line` or `mirror_axis`. Pass the transform's
catalog fields except `source`, which the function supplies per selected object.
It creates a visible `Document.groups[suffix]`. `set_group_visibility` changes
editor visibility; hidden geometry still evaluates and exports.

Piece fields are additive: `internal_paths: [{name, edges, closed}]`,
`anchors: {name: point_id}`, `grain_angle: {origin, angle, length}`, and
`label_point: point_id`. Existing `grain` by two points takes precedence.
`Document.label` adds a whole-pattern label to requested piece labels.
Internal paths become Marker line marks; anchors become Marker match points.
Labels include piece name, size, normal/mirrored quantity and fabric.

Annotations add `shaped_notch {edge,distance,depth,width,shape}` and
`seam_intersections {path,depth,width,shape}`. Shapes are `slit`, `v`, `t`,
`castle`. Automatic notches are deduplicated at shared seam vertices.
`union_pieces(doc, a, b, new_id, size, tolerance)` creates a fixed-size union on a
complete shared seam, preserves exterior allowance widths, grain, internal paths
and anchors, and retains source pieces. Overlap away from the seam, holes,
partial-edge joins and disconnected results fail. Union is a fixed edit, not a
parametric seam operation; review source quantities and labels after joining.

## Measurement API

`measurements::import_seamly(xml, size_label)` reads measurement-only
`.smis`/`.vit` and `.smms`/`.vst`. It returns
`MeasurementSet { sizes, base_size, values, heights }`, all millimetres.
`values` maps size to measurement-name/value rows. `heights` maps size to stature.
Use `set.apply(&doc)` to replace document measurement rows and size order.
For individual measurements use one size label. `MultisizeTable` stores base
values, adjacent-size increments and a per-mm height adjustment. `expand()`
generates every size/stature combination with `size@height` labels when needed.
`row(size, height_mm)` evaluates one combination. Heights must include base height.

`import_csv` / `export_csv` use the header
`size,height_mm,name,value_mm,base`. Each row is one named measurement. Size order
follows first appearance. `base` is `true` for base-size rows, otherwise `false`.
Quoted fields and doubled quotes are supported. Duplicate or non-finite values
and inconsistent heights fail. No new crate is required for these additions.


## Tiled printing and plotter output

`print::tiled_pdf(&doc, &sizes, &PrintOptions)` returns PDF bytes.
`PrintOptions { paper: Paper::A4 | Paper::A3, margin_mm, overlap_mm, tolerance_mm }`
defaults to A4, 10 mm margin, 10 mm overlap and 0.1 mm flattening tolerance.
`print::tile_layout` returns origins, dimensions, row/column numbers and piece IDs.
Rows count from the bottom and columns from the left. Each piece-size has its own
tile grid. A separate first page contains a 100 x 100 mm calibration square.
Print at **Actual size / 100%** with page-fit scaling disabled. Adjacent tiles
share true overlap content and alignment crosses on a shared 50 mm grid.
Paper margins must be 5-40 mm; overlap must be at least 2 mm and below half the
printable width/height. Output is limited to 10,000 tiles.

`print::plotter_plt(&doc, &sizes, tolerance)` returns HPGL text at 40 units/mm.
It places piece-sizes in a horizontal strip with 20 mm separation. Pen 1 draws
cut outlines; pen 2 draws grain, internal paths, notches, drills and text.
Coordinates are rounded to 0.025 mm units. Each piece-size is exported once;
quantity and mirrored-copy labels describe cutting demand. These are drawing
plots, not cutter control files. Text uses built-in Helvetica / ASCII in PDF and
plotter lettering in PLT; non-ASCII characters become `?`.

The CLI adds `export-pdf`, `export-plt`, `import-measurements`, and
`export-measurements`. PDF accepts `--paper A4|A3`, `--sizes S,M`, and
`--overlap 10`. Plotter output accepts `--sizes`. Measurement import outputs a
`MeasurementSet` JSON; measurement export outputs the documented CSV.

### Original interoperability samples

The Apache-2.0 original samples in `tests/fixtures/interop/` were supplied by the
independent interoperability agent. Their own provenance notice is retained.
They exercise modern `draftBlock/pieces/piece`, modeling paths referenced by
`iPaths`, body measurements in centimetres, visible labels, grain positions,
node notches and per-piece allowances. Tests compare `.smms` selections against
three individual `.smis` tables, grade every size, and round-trip pieces through
AAMA-style DXF. This is a BETA round trip, not evidence of a Seamly export/import
round trip; the external application-loading report belongs to the other agent.

`measurements::import_seamly_sizes(xml, &[48.,50.,52.], &[176.])` evaluates explicit
numeric selections in source units. XML increases are per dimension step,
defaulting to size step 2 and height step 60 mm. Explicit `step` overrides that.
`seamly::import_multisize(pattern_xml, measurement_xml)` reads the pattern's
explicit gradation selections, imports all rows, and validates all generated
sizes. The CLI selects this path when a multisize measurement file is supplied.
`seamly::import` keeps its original single-size contract.

Native quantity is preserved as normal copies. A text note saying "mirrored pair"
produces a warning; it does not silently invent mirrored-copy permissions.
Material may be read from the explicit `Material:` label template. Grain and
label scene positions use the 96 dpi scene-coordinate convention; their lengths
and body measurements use the file's declared units.

### Whole-pattern evaluation and rule libraries

`evaluate::evaluate_pattern(doc, size, tolerance)` returns
`PatternEvaluation {objects, pieces, piece_errors}` using one shared cache.
Every drawing object has its geometry or full error chain. Invalid pieces are
listed by ID in `piece_errors`; independent valid pieces are retained.
Drawing geometry is returned once. Each piece's `points` map contains only its
factory-rule, grain, fold, anchor and label references. `evaluate::grade` retains
its original behavior of returning all drawing points for every piece.
Both paths produce identical cut/seam geometry and Marker metadata.

`grading::rule_library(doc, name)` extracts reusable rules without body
measurements. `grading::apply_library(doc, library)` checks the size order,
base size, finite values and rule coverage before replacing those rule IDs.
Libraries serialize as `beta-pattern.rule-library/v1`. DXF continues to carry
both the evaluated sizes and the documented BETA rule-table extension.

Run `cargo bench -p beta-pattern-core --bench evaluate` for the release benchmark.
It evaluates all 1,000 objects, 30 pieces (including 30 cubics) and six sizes at
0.1 mm tolerance. It checks errors and the 180-piece result. The harness reports
ten timed runs after two warmups and requires median elapsed time below 100 ms.


### Seamly export conformance (pattern3)

The original Apache-2.0 input files and actual Seamly2D exports now live in
`../../testdata/seamly/`. That directory records the source version, patch
provenance, export command, hashes and licence. The new tests compare native
`.sm2d` plus `.smms` grading with `.smis` imports and independent AAMA exports for
all 15 piece/size combinations. `expected.json` supplies the metadata oracle.
No GPL application code or upstream examples are used. Seamly is not launched.

**BP-004, grain placement:** a visible free-position grain keeps its scene anchor,
direction and length. Seamly's containment repositioning is not reconstructed.
Import emits a piece-specific BP-004 warning before factory use; the retained
shirt regression still measures about 24.977 mm displacement after cut-bound
alignment. Hidden grain is omitted and explicitly warned. Warnings are retained
in `Document.meta.seamly_import_warnings`; the CLI repeats them when grading,
plotting, exporting or making a Marker job. Library clients must display the
import warnings and keep them visible before cutting. An explicit copy rule
does not silence the separate grain warning.

**BP-005, curves and geometry:** the observed 0.7.5 `simple` spline defaults use
equal handles proportional to the endpoint chord. The old reader incorrectly
used `length1/length2` as physical control lengths for these files. The default
model is reconstructed independently from our own exports using the circular
cubic handle ratio `(4/3)*(sqrt(2)-1)/sqrt(2)`. It is an export-validated model,
not a port of Seamly's evaluator or a claim of identical tessellation. It uses
geometry formulas, so handles change with graded endpoint distance.
Non-default `kAsm1`, `kAsm2` or `kCurve` now fail clearly until original exports
validate them. Older/versionless hand-written subset files keep their explicit
control-length behavior. Other native versions and spline modes are not
certified by these fixtures.

The proposed BETA engineering budget for these fixtures is **0.5 mm** maximum
bidirectional seam/cut boundary distance at the default **0.1 mm** native
flattening tolerance. At **0.01 mm** native tolerance the regression limit is
**0.15 mm**. Tests apply one shared translation to both seam and cut from the seam
lower-left bounds, without rotation or scaling. This also checks the relative
placement of the two contours. Width and height deltas must fit the same budget. Area
errors must also remain below 10 square mm (default) or 2 square mm (fine).
Area and bounding boxes alone do not prove curve agreement.

Boundary distances include segment interiors at intervals no greater than
0.1 mm in both directions. The asserted distance includes an extra 0.05 mm:
distance to a boundary is 1-Lipschitz, so this bounds unsampled locations.
This is a conservative polygon-distance bound, not just a vertex comparison.
The tolerance is an engineering regression gate, not an agreed cutting-room
standard or garment-fit certification. Production acceptance still belongs to
the factory. Allowance corners can amplify tangent differences, so fine output
is recommended for comparing pattern engines.

`evaluate::grade`, `evaluate::evaluate_pattern`, DXF exchange and print APIs
already accept a positive tolerance in millimetres. The CLI now exposes
`--tolerance` on `grade`, `export-dxf`, `export-pdf`, `export-plt` and `to-job`;
the default remains 0.1 mm. Zero, negative, non-finite and nonnumeric values are
rejected before output is created. Curve flattening tolerance bounds the
original curve sampling; it is not a promise that offset corners move by that
same distance.

**BP-006, copy rules:** `seamly::import_with_options` and
`import_multisize_with_options` accept `seamly::ImportOptions`. Its `copy_rules`
map is keyed by imported piece ID, such as `detail_43`, rather than a potentially
duplicate display name. Every rule supplies `normal` and `mirrored` counts.
Their checked sum must equal the source total quantity and be positive. Unknown
piece IDs, missing counts, unknown JSON fields, changed totals and overflow
fail. Unmapped pieces keep their total as normal copies and retain any
mirrored-pair annotation warning. Neither even quantity nor permission to flip
creates mirrored demand. Partial mappings are allowed.

The CLI accepts a JSON map with `import-seamly --copy-rules rules.json`:

```json
{
  "detail_43": { "normal": 1, "mirrored": 1 },
  "detail_86": { "normal": 1, "mirrored": 1 }
}
```

Use the supplied `shirt-front-back.copy-rules.json` (and corresponding skirt
and trouser maps) for exact IDs. These maps express the independent sample's
intent; they are not inferred from the native flip flag. The older `import`
and `import_multisize` APIs retain their signatures and default copy behavior.
A `.smis` CLI import remains a single size named M; `.smms` uses the explicit
48/50/52 selections in the pattern.
