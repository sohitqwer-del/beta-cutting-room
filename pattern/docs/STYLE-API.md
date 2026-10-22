# From scratch to cut: API v1

Contract frozen first for the `style-ui` implementation. Runtime support follows
in the engine commits. TypeScript contract and offline loader:
`marker/packages/marker-wasm/src/style.ts`, exported by `@beta/marker-wasm`.
No geometry calculations belong in the browser.

```ts
import { loadStyleCore, loadPatternCore } from '@beta/marker-wasm';
const style = await loadStyleCore();
const pattern = await loadPatternCore(); // existing inspect, snap, nest, exports
let doc = style.create({ block: 't_shirt', chart: style.size_chart() });
doc = style.edit(doc, [{ op: 'move_point', id: 'front.arm1',
  size: 'M', target: [230, 460] }]); // use actual inspected handle coordinates
const graded = style.grade(doc);
const seams = style.block_seams(doc);
```

## Transport and errors

`beta_pattern_wasm.style_command(JSON.stringify(request)) -> JSON string`.
`bindStyleCore` wraps this transport; `loadStyleCore` loads local WASM assets.
The native Rust adapter exposes the same dispatcher for server integration.
Each request has an `action` equal to its snake_case method name, followed by
named arguments in `style.ts`. `create` wraps its options in `request`;
`measure` wraps the measurement selection in `request`. No new HTTP endpoint
is required by the loader. The native server also accepts the identical JSON
envelope at `POST /api/pattern/style`, using the existing authentication and
body limits. Existing Pattern endpoints and `PatternCore` remain compatible.

Errors throw plain English strings. Do not commit failed results to history.
`edit(document, operations)` applies all operations to a clone, then validates
all sizes. The caller retains the prior snapshot for undo and redo. References
use stable IDs. Piece-edge indices are **zero based**, except legacy
`piece_action` dart/unfold fields, which retain their documented one-based input.

`save` returns versioned JSON text; `load` validates the document format,
references, formulas and every graded piece. Current format remains
`beta-pattern.doc/v1`; no format migration or change to old object tags.
Blank documents and incomplete collections of construction objects are valid;
a piece must have a closed, connected outline before `put_piece` commits it.

## Document operations

| Task | Call |
|---|---|
| Blank sheet | `create({name, chart})`, omit `block` |
| Seven basic blocks | `blocks()`, then `create({block, chart?, ease?})` |
| Point, line, cubic handles, arc | `edit` with `put_object` and the existing typed `DrawingObject` |
| Replace a curve or move a handle | `put_object` with `replace:true`, or `move_point` |
| Delete / rename construction | `delete_object` rejects used IDs; `rename_object` rewrites references |
| Piece metadata, grain, fold, label, anchors | `put_piece` with a complete `PatternPiece`; `replace:true` for edits |
| Delete piece | `delete_piece` (drawing objects retained) |
| Offset piece / join adjacent pieces | `offset_piece` / `join_pieces`; linked outputs, source retained |
| Mirror, split, dart, pleat | `piece_action` operation with the existing request below |
| Seam / hem width and corner | `set_allowance` with piece, edges, width and corner |
| Notch, drill, label, individual button | `add_annotation` with existing `Annotation` |
| Repeated buttons / buttonholes / drills | `mark_row` |
| Measurement edit and regrade | `set_chart`, then `grade` |
| Ease formula | `set_variable` (block variables named `ease_chest`, `ease_waist`, `ease_hip`, `cap_ease`) |
| Factory point rule | `set_rule` then `attach_rule`; null detaches |
| Hit testing | `hit_test(doc,size,at,radius)` returns sorted point/path/piece hits in mm |
| Length / distance / included angle | `measure`; `Measured` carries value, unit and display geometry |
| Grade nest | existing `PatternCore.nest(doc,reference)`; returns aligned size geometry |
| Graded sewing report | `seam_report(doc,check)` or generated `block_seams(doc)` |
| File export | existing `export_dxf`, `export_pdf`, `export_plt` on `PatternCore` |

`move_point` defaults `preserve_formulas:true`: adds the displacement measured
in the chosen size to the existing absolute-coordinate formulas. Explicit
`false` retains legacy fixed-coordinate drag semantics. Cubic control points
are ordinary drawing points. Constrained constructions can be replaced with a
new absolute point if the user wants to break that constraint explicitly.

Factory rules retain existing semantics: attaching any rule makes that piece
use base geometry plus factory displacements (not measurement deformation plus
another displacement). Report this choice in the UI. Point rules must target
outline vertices; free curve handles are edited through their formulas.

Existing shape request examples (inside `{op:'piece_action',request:...}`):

```json
{"action":"mirror","piece":"front","id":"front_copy","a":"axis0","b":"axis1"}
{"action":"split","piece":"front","id":"split_front","size":"M","a":"cut0","b":"cut1"}
{"action":"dart","piece":"front","id":"dart_front","size":"M","edge":1,"center":80,"width":20,"apex":"dart_tip"}
{"action":"pleat","piece":"front","id":"pleated","size":"M","a":"cut0","b":"cut1","depth":20,"pleat_kind":"box"}
```

Mirror/unfold and new split/join/offset/dart/pleat pieces are linked constructions.
New derivatives reevaluate their source geometry and construction inputs in every
size. Existing saved fixed outlines retain their previous meaning.
Do not silently remove the source or claim an associative style edit.

## Blocks and charts

IDs: `t_shirt`, `shirt`, `trouser`, `a_line_skirt`, `kurta`, `polo`, `track_pant`.
All return the ordinary editable document. `SizeChart` uses sizes as ordered
columns and `measurements[size][name]` as the source values. Charts are original
BETA **example body measurements**, S/M/L/XL/XXL, with millimetre naming suitable
for Indian factory entry and ISO-style body measurements. They are not a
certified population survey or an ISO size-label standard. Preset IDs are
`indian_adult` and `iso_example`; operators should replace them with their buyer
chart. Details and original formulas are in `BLOCKS.md`.

`BlockEase` controls chest/waist/hip circumference ease, sleeve cap ease, seam
and hem allowances. Templates retain formulas referencing `m.measurement`
and `v.variable`; a chart change therefore regenerates geometry. Generated
sewing checks live in `document.meta.block_seams`. `block_seams` recomputes
actual stitch paths, including point-rule grading, after edits.

Block sleeve-cap checks retain an `ease_formula` in metadata. Each report row
includes `required_ease_mm`, reevaluated for that size after variable edits;
the report's `check.ease_mm` is the base-size value.

## Lay and job handoff

`to_job(doc, settings)` grades the selected ratio, preserves marks and fold
axes, applies gaps and per-piece buffers/blocks, and expands match rules from
document piece IDs to each size's Marker ID (`front-M`, etc.). Match points
come from `PatternPiece.anchors` (name -> drawing-point ID).

`settings.material` optionally selects one material group. It is required
when a style includes multiple fabrics, such as shell legs and lining pocket
bags. Each marker is for one fabric roll.

`settings.fabric` is the existing Marker `Fabric`: width is the **flat spread
width**, `layout` is `open|folded|tubular`; `nap` is
`none|one_way|one_way_per_bundle`. Existing `allow_flip` remains the explicit
permission to turn a pattern over. Physical face does not grant that permission.

`settings.spacing` is the existing `{piece_gap,edge_gap}`. Required pair
clearance = global gap + both piece buffers. Uniform edge gap applies at the
start and cut selvedges; actual fold edges have no edge gap. A fusing block is
the grain-aligned bounding box expanded by `block_margin`, then gaps apply to
that rectangle. `piece_settings` keys use unsuffixed document IDs.

`settings.lay` is retained as typed, validated `job.meta.beta_style_lay`:
`{face:'up'|'down'|'face_to_face',plies,end_allowance_mm}`. Plies count physical
fabric layers, so folded/tubular and face-to-face lays require even counts.
End allowance is waste at **each** end; spread length = marker cut extent +
2 * end allowance. It does not translate the marker, increase nesting gaps,
or multiply the marker size ratio. `fabric.max_length` limits the marker cut
extent; table allowance must be reserved separately when setting that limit.
These fields are spreading instructions, not a new layer allocation scheme.

## Repeats and overlays

`fabric.repeat` keeps the existing `{period_x?,period_y?,offset_x,offset_y,
tolerance_mm}`. X is marker length, Y is width; an X period gives transverse
stripe lines. Two periods give checks. Zero tolerance means the established
1 mm default. A match rule refers to A, optional B, axes, and required phase
difference. B omitted anchors A to the fabric repeat. Rules are per garment
and must include every garment served by a shared layered cut.

Colour bands are `RepeatBand[]` in `settings.repeat_bands`, persisted in
`job.meta.beta_repeat_bands`. Each specifies axis, offset within one repeat,
width in mm, and `#RRGGBB` colour. Bands affect drawing only, not matching.

`overlay(job,result,{bands?,graded?,previous?})` returns:

- `stripes`: repeat lines clipped to actual placed outlines, including fold
  halves; each line has placement index, endpoints, colour and width.
- `outlines`: displayed cut outlines in marker coordinates.
- `graded`: supplied graded piece outlines in their source coordinates, for a
  size nest display. Use `PatternCore.nest` first for reference alignment.
- `previous`: added/removed/moved/unchanged identities, with before/after
  geometry. Pass the previous job too so changed source patterns display correctly.
- `spread_length_mm`: cut extent plus both spreading end allowances.

Overlay requests are bounded and reject invalid repeats or excessive geometry.
Optional stripe-guide export uses the same clipped geometry in PDF/PLT (pen 3,
internal guide, never a cut contour). Old exports omit it by default.

Use `job.meta.beta_stripe_guides=true` on an export-only clone with the existing
marker export API. The native `export_with_stripe_guides` helper provides the
same option. The browser worker now routes jobs with match rules or shade bands
through `beta-marker-match::browser::MatchingSession`; plain jobs retain
`BrowserSession`. Worker messages and `createMarkerNester` do not change.

## Precision and limits

Units are mm; y points up; angles are degrees counter-clockwise. Draft path
joins use 0.000001 mm tolerance, normal curve flattening 0.1 mm, seam reporting
0.05 mm, independent marker validation 0.01 mm, and default sewing mismatch
tolerance 1 mm. Flattened length is an approximation, not analytic arc length.
Declared full-piece fold outlines must agree with their reflected cut boundary
within 0.2 mm (two curve-sampling tolerances). An asymmetric allowance edit is
rejected; edit the corresponding side as part of the same transaction or remove
the fold constraint. Corner types belong to each edge's start vertex.
Folded/tubular handoff also checks that notches and drills have reflected
partners on a declared full-piece fold outline. Default block notches are
paired. The folded/tubular handoff now creates any missing reflected notch or drill
automatically on its job clone, including shaped notch vertices and text. It
does not duplicate an existing partner or a mark on the axis. The source
document and open-lay marks remain unchanged. The same strict symmetry check
runs after completion; asymmetric outlines or allowances are still rejected.
Bad input returns an error; no failed transaction changes its source document.
Basic blocks are original starting drafts, not fit-certified production patterns.

Seam-distance notches are projected along the outward seam normal onto the
allowance cut edge. Slit depth and V/T/castle vertices are retained in marker
plot/print output. A notch at an ambiguous corner can fail with a request to
move it; the engine does not silently place it inside the seam allowance.


## Polish additions (backward-compatible v1 transport)

`bindStyleCore` and `loadStyleCore` now return `NativeStyleCore`.
Use `StyleEditOperation` for batches containing the new path operations and
`NativeMarkerOverlay` / `MatchReport` for the full measurement fields.
The original `StyleOperation` union and default `StyleCore` remain compatible
with existing exhaustive switches and test doubles. `StyleCore` accepts generic
operation and match-row types; its runtime transport has not changed.
`MarkerOverlay` defaults to the common `MatchSummary` accepted by older UI
adapters; `MarkerOverlay<MatchReport>` describes the native overlay.

### Rule measurements

`overlay(...).matches` and `ValidationReport.matches` use the **same** Rust
phase check. Each row identifies `rule` (zero-based job rule index), `garment`
(bundle ID), `a_ref` / `b_ref` (piece and named match point), and
`a_placement` / `b_placement` (placement array indices, or null).
`a` and `b` are marker coordinates; fabric-anchored rules use the fabric
repeat origin for B and a null B reference/placement. Missing points are null.
There is one row per applicable rule, garment and endpoint-copy pair, including
all garments served by a shared layered cut. An entirely unplaced garment
still has rows; rules for other sizes do not.

`status` is `matched | off | not_placed`; `pass` is true only for matched rows.
`offset_mm:[x,y]` is the signed shortest periodic residual of
`A - B - rule.offset`. Positive residual means A is ahead along that axis.
Unchecked or missing axes are null. `error_mm` is the maximum absolute checked
residual, or null when it cannot be measured. `tolerance_mm` includes the
established 1 mm default for a zero fabric tolerance. X/Y use marker axes,
not pattern axes. Old engines may omit `matches`; the TS field is optional.
Reports are limited to 100000 endpoint pairs. Exceeding that bound returns an
overlay error and a failing validator issue (`match_report_limit`), never a
partial passing report.

### Block sheet layout

`create({block})` places cut outlines in separate columns with at least 50 mm
between them across **all chart sizes**, grain vertical. Formula-based points
remain directly editable. The shirt yoke has independent editable paths.
The reserved `layout_<piece>_x/y` variables store sheet offsets. `set_chart`
recomputes those offsets for the changed chart. `PatternCore.nest` still
provides the separate reference-aligned size nest; no marker packing is done
by this sheet arrangement.

### Parallel path and extend/trim

```ts
{ op: 'offset_path', id: 'facing', source: 'front.arm', distance: 12, side: 'left' }
{ op: 'offset_path', id: 'other_side', source: 'front.arm', distance: 12, side: 'right' }
{ op: 'extend_trim', id: 'extended', source: 'line_a', target: 'line_b', end: 1 }
```

Both operations create linked copies and retain their sources. Distances can
be non-negative formulas. Left/right follow source traversal in y-up pattern
coordinates. Lines, cubic Beziers and circular arcs, including translated,
rotated or reflected copies, support exact analytic normal-distance samples.
The returned curve is flattened with an adaptive 0.01 mm sampling target; it is
not an exact cubic representation of a cubic offset. Cusps, reversed offsets,
self-intersections and unsupported sampled path kinds return errors rather
than silently producing unsafe geometry. Use zero distance for a parallel
copy on top of the source. Drawing kind `offset_path` persists a signed
`distance` and `source`. Offset output is limited to 8192 samples.

`extend_trim` currently handles two straight `line` objects. Endpoint 0 or 1
of the copy moves to their infinite-line intersection; parallel/coincident
lines and a zero-length result fail atomically in all sizes.

### Edge picking

`hit_test` retains point/path/interior hits and adds path hits with `piece_id`
and zero-based `edge` for each nearby **graded stitch edge**. `point` is the
nearest position, `distance_mm` is click distance, and `along_mm` runs from
that directed piece edge's start (respecting `reversed`). Use these fields for
notches, darts and seam allowances; no browser projection is needed.

Path hits also expose `parameter` and `parameter_kind`. `native` is the
source object's 0..1 parameter for a line, cubic or circular arc, including
rigid transforms; it does not reverse with the piece edge. Other paths use
`arc_length_fraction`; factory-deformed piece edges use
`edge_length_fraction`. Path length and seam distance remain sampled mm.
Standalone path hits omit `piece_id` and `edge`. Interior hits omit parameters.

### Linked derived pieces

New drawing kind `derived_path` stores
`{recipe:{source,operation,inherit_allowances},guide}`. Operations are `split`,
`join`, `dart`, `dart_transfer`, `dart_close`, `pleat`, and whole-piece `offset`.
`guide:null` selects the closed boundary; a numbered guide selects a pleat fold.
Their boundaries are rebuilt from the retained sources in every requested
size, including source factory rules and changes to chart measurements.
Dart center/width and pleat depth retain formulas. A legacy dart request's
one-based sampled edge is mapped to a stable straight source edge when created.
New dart requests can pass `seam_edge` (zero-based) directly from an edge hit
instead of the legacy `edge` field. This remains stable when earlier edges are
curves with different sample counts. Curved dart mouths are rejected with a clear error. Pleat fold guides regrade
with the boundary. Split yields `id` and `id_2` as before. Sources must remain
available; deleting a source or creating a dependency cycle fails validation.
To cut only the derived pieces, set both source copy quantities to zero and
retain the source as construction geometry. Retarget or remove its match rules.

New derivatives retain source material, quantity and grain. As before, shape
changes require review of marks and allowance: uniform allowance is retained;
nonuniform source allowances start at zero on split/dart/pleat boundaries.
Joins retain exterior source allowances, marks and namespaced match anchors;
notches on the removed shared seam are dropped. `set_allowance` on the joined
boundary disables allowance inheritance, and inherited notches project to the
new cut edge. Fold symmetry, old seam notches and match anchors are not assumed
for split/dart/pleat shapes. The
boundary is one linked path; source control points remain the way to reshape
it. Attach factory rules to the retained source; a second factory rule table
on the linked output is rejected to avoid applying grade movements twice. Existing saved polygon derivatives are not automatically converted because
they do not contain enough construction history.

## Block quality and measurement specifications (round 0.7)

The additive `StyleQualityCore` methods are present on `NativeStyleCore`, from
`bindStyleCore` / `loadStyleCore`. Existing `StyleCore` test doubles are unchanged.
All requests use the existing `style_command` JSON envelope and native endpoint.

```ts
const engine = await loadStyleCore();
const checks = engine.block_checks(doc);
const spec = engine.spec_sheet(doc);
const file = engine.export_spec_sheet(doc, 'pdf'); // also csv or xlsx
const blob = new Blob([new Uint8Array(file.bytes)], { type: file.mime });
// UI downloads blob using file.filename. Rust supplies all measurements.
```

| Action | Input after `action` | Output |
|---|---|---|
| `block_checks` | `document` | `BlockReport` |
| `spec_sheet` | `document` | `SpecSheet` |
| `export_spec_sheet` | `document`, `format:'pdf'|'csv'|'xlsx'` | `{filename,mime,bytes:number[]}` |

`BlockReport` has `block`, `sampling_mm`, `pass`, `limitations`, and ordered
`sizes`. Each size has `size`, `pass`, `checks`, and `errors`. A check has `id`,
`category`, `value`, `unit`, inclusive `min`/`max`, `pass`, and `note`. Errors
stay visible as failed size rows; no failing size is silently dropped. Categories
cover seams, ease, tangent continuity across joins, grain, assembly notches,
allowances, simple boundaries, finished dimensions, chart plausibility, and
`proportion` (crotch extension, thigh, back/front rise, yoke depth). The
`geometry` category includes `concave_radius` (tight curve versus allowance).
A report is not a physical fit certificate. See BLOCKS.md for ranges and scope.

`SpecSheet` is `{schema:'beta-pattern.spec/v1',style,block,unit:'mm',sizes,
base_size,rows,notes}`. Each row (a `PointOfMeasure`) has `id`, `name`, `method`,
`tolerance_mm` and ordered `values`. A value is `{size,measured_mm,body_mm,
ease_mm,target_mm,grade_step_mm,pass}`. Nullable fields are **explicit nulls**.
`target_mm=body_mm+ease_mm` when that relationship is specified; design-only
measurements have null body/ease/target/pass. Grade step is actual measured
change from the preceding ordered size; the first is null. QC tolerances are
proposed buyer approval values, separate from 1 mm drafting tolerances.

Measurements use actual **graded stitch paths**, including point rules, and
exclude seam/hem allowances. Upper-body POMs include half chest 10 mm below
armhole, HPS body length, half hem, neck width, shoulder, armhole depth, curved
armhole, across shoulder, front and back neck drops, sleeve length including
folded shirt cuff, half bicep and half opening. Lower-body POMs include half
waist, hip at 200 below waist seam, length below waistband, band depth, hem,
and trouser inseam, outseam, half thigh 25 mm below the crotch, front/back
rises. Row order is stable; new rows may be added, so look rows up by `id`. Track-pant waist is the extended
casing, not relaxed elastic. Shrinkage, stretch recovery and wash are excluded.

PDF has one A4 table page per size, plus measurement methods and notes. It
reuses Pattern's offline PDF writer and its ASCII fallback for unsupported
characters. CSV is UTF-8 with BOM, quoted fields and CRLF, one POM/size row;
formula-like user text is escaped for spreadsheet safety. XLSX comes from a small
dependency-free Rust writer (stored ZIP) with Measurements, Targets, Grade steps,
Methods and Data sheets. Measurements are numeric cells shown with two decimals;
names, methods and notes are inline text and never formulas. Exports share the
same unrounded source report and display 0.01 mm. No runtime network,
server-only dependency, external font or UI calculation is required.

These POM mappings apply to generated blocks retaining their original active
piece set and seam topology. A blank/custom pattern, archived/replaced piece,
or linked topology change cannot produce a misleading original-block spec.
Such a spec returns a plain-English error; `block_checks` returns failed size
rows with the review reason. Renaming/replacing block pieces requires reviewed
custom seam/POM mappings, which are not yet a general POM authoring interface.

Style `piece_action` for split/dart/pleat/dart_transfer/dart_close/slash_spread
now defaults to `request.production_mode:'replace'`. The retained source's
`quantity` and `mirror_quantity` become 0. Set `'additional'` explicitly when
both pieces are intended to be cut. Linked construction is retained in either
mode. `meta.block_review_required` is user-facing review text. After topology
changes `block_seams` returns no stale green source checks; custom `seam_report`
remains available for operator-reviewed runs. Show the review text and distinguish
zero-quantity construction sources from active production pieces.
