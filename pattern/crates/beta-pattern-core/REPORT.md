# Seamly2D interop closure report (pattern3)

Work starts at `marker/mvp` commit `de68343` on branch `marker/pattern3`.
Changes are confined to the assigned core, native/export test data and Marker
DXF reader paths. No Seamly, Qt test or GUI program was launched. Only retained
exports were used. Nothing was pushed or merged.

## Fixes

- **BP-004:** visible free-position grain keeps its direction and length and
  now emits a piece-specific placement warning. Import warnings stay in document
  metadata and reappear during CLI grading, PDF/PLT/DXF export and Marker job
  creation. The retained regression measures the original roughly 24.977 mm
  placement difference. Hidden native grain is omitted with a specific warning;
  it is no longer presented as visible/known grain.
- **BP-005:** distance checks found a real curve discrepancy hidden by close
  bounds: up to 4.222 mm at 0.01 mm native sampling. The 0.7.5 simple-spline
  default in these exports uses equal chord-relative handles. Treating the
  sample's `length1/length2` as physical control lengths was wrong. An independent
  reconstruction using the circular cubic handle ratio reduces fine-output
  contour differences to below 0.1 mm on the retained cases. Non-default legacy
  shape coefficients fail explicitly. No GPL implementation was read or ported.
  The README documents the reconstructed model and its limited validation scope.
- **BP-005 controls:** geometry commands accept `--tolerance` in mm, default 0.1.
  The regression budget is 0.5 mm at that default and 0.15 mm at 0.01 mm sampling.
  Tests measure both directions along edges at intervals <=0.1 mm and include
  a conservative 0.05 mm allowance for unsampled positions. Both contours use
  one shared translation from stitch bounds, preserving relative allowance
  placement. Width/height and independent area gates also apply. This is a BETA
  engineering budget, not factory acceptance or garment-fit certification.
- **BP-006:** additive import APIs take per-piece normal/mirrored copy rules.
  The CLI exposes `--copy-rules`. Rules use stable piece IDs and must preserve
  source total quantity; unknown IDs, missing/unknown JSON fields, overflow,
  zero totals and changed totals fail. Partial mappings preserve warnings for
  unmapped pieces. Neither even counts nor flip permission imply pairing.
- **DXF metadata:** a mirrored-pair text note remains a drawing mark and raises
  an actionable warning without a pairing rule. Explicit BETA copy metadata
  and opt-in even-count pairs do not need that warning. Empty label fields no
  longer hide later populated labels. Explicit mm units, unitless baseline
  inference, size and quantity fields are covered by actual-export tests.
  Missing fabric remains unknown in the core; a caller's shell default remains
  an assumption that needs its existing warning and bill-of-materials check.

## Data and coverage

`pattern/testdata/seamly/` contains unchanged original `.sm2d`, `.smms`, `.smis`,
expected metadata, licence, source notice, generator, 14 retained DXFs and three
explicit copy maps. The exports cover nine garment/size selections plus shirt
visibility, baseline R12 and modern-writer variants. Provenance records source
commit, runtime version/revision, executable hashes, relative export locations,
export command and per-file SHA-256. Marker has six small regression exports
with the same Apache-2.0 provenance. Combined data is under 0.75 MB, below 2 MB.
No upstream examples, GPL code or proprietary patterns are included.

New regressions cover:

- All 15 native piece/size combinations at both 0.1 and 0.01 mm sampling,
  compared with independent Seamly geometry and `expected.json`.
- Multisize grading versus all three independent measurement tables, exact
  native `.smms`/`.smis` contour equality and individual CLI size M behavior.
- Names, total quantities, intended explicit mirrored quantities, materials,
  sizes, grain direction/length, hidden grain, internal paths and notch segments.
- Full and partial native copy maps; invalid IDs/counts/JSON; preservation of
  normal totals independently of the native flip flag; option serialization.
- CLI import, finer tessellation, all geometry output commands, repeated grain
  warnings, and invalid tolerances rejected before output creation.
- Real AAMA R12/modern exports with normal/pair modes, default/hidden labels,
  hidden grain, explicit units and baseline inference warnings.
- A boundary-distance self-check where identical vertices but different edges
  would fool a vertex-only comparison.

## Validation

All Cargo commands ran from the requested workspace directories, using the
assigned external target and `CARGO_BUILD_JOBS=3`.

- `pattern/`: `cargo test --workspace` ? **62 passed**, including the doc test.
- `pattern/`: `cargo clippy --workspace --all-targets -- -D warnings` ? passed,
  zero warnings.
- `marker/`: `cargo test --workspace` ? **233 passed, 4 ignored**, zero failures.
  The new actual-Seamly DXF and blank-label regressions both passed.
- `marker/`: `cargo clippy --workspace --all-targets -- -D warnings` ? passed,
  zero warnings.

The four ignored Marker tests are the existing long Simon production pipeline,
release validation-speed check, 60-second shirt quality check and real Simon
planning benchmark. They were not run or counted as passes.

All 30 native/reference contour comparisons passed: 15 piece/sizes at each of
two tolerances. At fine tolerance the conservative boundary bound is <=0.15 mm
and both area errors are <2 square mm. Default tolerance meets the documented
0.5 mm boundary budget and <10 square mm area budget.

Staged-blob checks verify all 14 export SHA-256 digests, all 11 original input/
notice/licence/generator files byte-for-byte against their sources, and all six
Marker export copies against the corresponding Pattern fixtures. Git attributes
preserve original line endings. Combined fixture size is **748,213 bytes**.
`git diff --cached --check` passes. All 49 changed files are within ownership.
No build output is staged.

No new dependency or workspace manifest change is needed. Existing dependencies
and licences are unchanged. Original fixtures are Apache-2.0. Cargo-generated
lockfile changes are not part of this delivery.

## Remaining gaps and lead handoff

Exact Seamly free-grain containment placement remains unimplemented; the warning
path requested by BP-004 is complete and stays visible before CLI factory
output. Library/UI clients must display retained warnings before cutting.
Non-default simple-spline coefficients now require original export fixtures
before support is expanded. Other XML versions, elliptical/spline-path tools,
variable allowances, complex notches and garment fit are not certified by this
sample set. DXF stitch/internal paths still share Marker line marks without a
new structured line-role field; ASTM conformance is not claimed. The original
manual combined-Seamly run gaps remain untested; no
headless application run or visual review is claimed.

Lead action: expose the new copy-rule input and existing tolerance controls in
UI/WASM flows, and show `seamly_import_warnings` before plotting or cutting.
No Marker Piece contract or workspace dependency change is requested.

---

# Earlier BETA Pattern tools implementation report

Work is confined to `pattern/crates/beta-pattern-core/` on `marker/pattern-tools`.
The native JSON tag remains `beta-pattern.doc/v1`: additions are new tagged
construction variants and optional fields with defaults. Existing starter files
and the existing `evaluate::grade` contract remain readable.

## Delivery and integration

- `3f1d4e4`: catalog, generic apply, safe edits and per-object evaluation.
- `4fb1045`: UTF-8 documentation correction.
- `a981642`: 35 constructions, group transforms, piece metadata, measurement API
  and extended hand-written XML family fixtures.
- Final implementation commit: printing, multisize/native-sample interoperability,
  shared evaluation, reusable rule libraries, performance and strict Clippy cleanup.
  The final handoff message identifies its hash.

The UI can generate every construction form with `tools::catalog` and call
`tools::apply(doc, tool_id, id, params)`. Parameter names are native JSON fields.
`object_ref` accepts a point or path; transform output follows its source type.
Reference lists account for direct dependencies and geometry references reached
through variables. Rename updates references; delete lists blocking users.
Absolute and Polar points support numeric dragging. Snapshot history remains
available. Hidden groups still evaluate and export.

`evaluate::evaluate_document` returns errors per object. The faster
`evaluate::evaluate_pattern` returns objects once, graded pieces and errors per
piece with one shared cache. Its piece point maps hold factory-rule, grain,
fold, anchor and label references; the full drawing is in `objects`.
`grade` retains all drawing points per piece. A regression test checks that both
APIs produce identical seam/cut geometry, Marker fields and retained reference
coordinates. An invalid piece does not hide an independent valid piece.

## Tool coverage

All 35 catalog IDs have known-coordinate cases in `tests/catalog.rs`. The test
compares its tested-ID set with the actual catalog, so adding an untested catalog
entry fails the test. The following maps expected tool families to our designs.

| Seamly-style family | BETA construction or API | Evidence |
|---|---|---|
| Point at distance and angle / end line | `polar` | Exact (0,4) from a 90-degree offset |
| Along line | `along_line` | Exact signed length |
| Shoulder point | `shoulder` | Exact circle/infinite-line solution |
| Normal | `normal` | Exact perpendicular offset; rotated XML normals use Polar |
| Bisector | `bisector` | Exact (1,1) on a right-angle bisector |
| Height / perpendicular foot | `foot` | Exact projection |
| Triangle | `triangle` | Right-angle vertex on axis, opposite a hypotenuse |
| Line-line intersection | `line_intersection` | Exact (3,0) |
| Line intersect axis | `line_axis` | Infinite line and angled axis |
| Curve intersect axis | `curve_axis` | Curve-order crossing, within flatten tolerance |
| Arc-arc intersection | `arc_intersection` | Exact (3,4), sweep filtering |
| Circle-circle intersection | `circle_intersection` | Exact (3,4), tangent/disjoint/concentric cases |
| Circle tangent contact | `circle_tangent` | Exact (2.5,sqrt(18.75)), inside-circle rejection |
| Arc tangent contact | `arc_tangent` | Exact contact with sweep filtering |
| Arc-line intersection | `arc_line` | Exact circle crossing with signed sweep filtering |
| Circle-line / contact | `circle_line` | Exact (-5,0), tangency tested |
| Intersection X/Y | `intersect_xy` | Exact (6,8) |
| Intersection of curves | `intersection` | Finite sampled paths and stable crossing order |
| Cut arc at length | `curve_point` | Exact circular arc-length point |
| Cut spline at length | `curve_point` | Known straight-spline distance; other curves use tolerance |
| Cut spline path at length | `curve_point`, `cut_path` | Known path point and interval endpoints |
| True darts | `true_dart`, existing dart functions | Equalize one leg along its ray; transfer/close preserve area |
| Lines | `line` | Exact endpoints |
| Circular arcs | `arc` | Known endpoints and arc length bounds |
| Arc with length | `arc_length` | Known quarter-circle endpoints |
| Elliptical arcs | `elliptical_arc` | Rotated ellipse endpoints and invalid-axis rejection |
| Cubic Bezier | `cubic` | Known endpoints; existing curved sampling tests |
| Bezier path | `cubic_path` | Connected segments and endpoint checks |
| Interactive spline / spline path | `spline` | Interpolating Hermite path, automatic or explicit derivatives |
| Rotation | `rotate`, `transform_group` | Point and path copies, exact coordinates |
| Mirror by line | `mirror_line`, `transform_group` | Exact reflected point/path |
| Mirror by axis | `mirror_axis`, `transform_group` | Horizontal and vertical reflection |
| Move selected objects | `move`, `transform_group` | Exact offsets; collisions fail atomically |
| Groups and visibility | `DrawingGroup`, `set_group_visibility` | Membership, deletion checks and visibility test |
| Additional point helpers | `absolute`, `midpoint`, `path_endpoint` | Known coordinates/endpoints |

The native implementations are original Rust designs. No GPL code, text, icons
or third-party pattern examples were copied.

## Pieces, measurements and factory extras

| Capability | Delivered behavior and test |
|---|---|
| Internal paths | Connected open/closed marking paths; exported as Marker line marks |
| Anchor points | Named point references; exported as Marker match points |
| Piece union | Removes complete shared seam segments; checks simple exterior and summed area; retains exterior allowances, marks, grain and anchors; shared-seam notches are removed |
| Per-edge allowance | Existing formula widths and miter/square/bevel/mirrored corners retained |
| Notch types | Slit, V, T and castle; exact point arrays tested |
| Automatic notches | Intersections of an internal drawing path with piece seams; shared-vertex duplicates removed |
| Grain | Two-point grain or origin/angle/length; known 80 mm vertical line tested |
| Labels | Name, size, quantities, fabric and optional whole-pattern label |
| Individual measurements | One named measurement row applied to a document |
| Multisize and heights | Base values, adjacent-size increments and height adjustment; lower/base/upper sizes tested |
| CSV | Long format with quoting, duplicate/non-finite checks and round trip |
| Measurement XML | `.smis`, `.vit`, `.smms`, `.vst`; source units become millimetres |
| Factory rule library | Versioned reusable library without body measurements; size compatibility, coverage and finite-delta validation |
| Factory grading | Relative/incremental rules and outline-distance interpolation retained |
| AAMA-style DXF | All selected sizes, evaluated points and explicit BETA rule-table extension; round trips tested |
| Dart transfer / spread / pleats | Existing area/angle preservation and exact area tests retained |
| Walk and true seams | Length comparison and target-length scaling retained and tested |
| Send to BETA Marker | Existing validated job and size-ratio conversion retained |
| Tiled print | Dependency-free A4/A3 PDF, actual-size vectors, real overlap and shared-grid registration crosses |
| Calibration | Separate first page with exactly 100 x 100 mm square |
| Plotter | Dependency-free HPGL/PLT at 40 units/mm; cut and annotation pens, bounded integer coordinates |

XML size increases are per dimension step, not per numeric label unit. The
reader uses explicit steps where supplied; otherwise size step 2 and stature
step 60 mm. `import_seamly_sizes` accepts source-unit size/height selections.
`seamly::import_multisize` reads explicit pattern gradation selections and
validates all generated sizes. The original single-size import API remains.

## Seamly import coverage and sample provenance

`tests/fixtures/families.sm2d` is our hand-written exploration fixture. It covers
all new point/curve families, both spline path representations, the four group
transforms and visibility. Tests check known coordinates and native JSON round
trips. Unsupported point, path, operation and calculation tags fail explicitly.

The independent interoperability agent supplied original Apache-2.0 files in
the permitted sample directory during this task. Their original provenance
notice is retained in `tests/fixtures/interop/README.md`. The copied data includes
three garment patterns (skirt front/back, shirt front/back, trouser front), one
multisize table, three individual tables and intended-metadata JSON. No GPL
application source, application example, lock file or build output was copied.

The modern sample structures required support for `draftBlock/pieces/piece`,
modeling internal paths selected through `iPaths`, label quantity/material,
scene-positioned grain and node notches. Tests now import all three patterns,
grade 48/50/52 and DXF-round-trip 15 piece-sizes. Multisize measurement values must
exactly equal all three independently supplied individual tables. Seam/cut areas,
marks, quantities, material and grain are checked. The supplied skirt XML has no
notch attributes; shirt/trouser notch attributes are tested when present.

The sample data was authored independently and schema-checked by the other
agent. Application loading/export validation belongs to that agent's report.
These BETA tests do not claim a complete Seamly-native round trip. Material is
read from an explicit `Material:` template line when present. Native quantity is
preserved as normal copies; a mirrored-pair text note raises a warning because
it does not define a machine-readable mirrored-copy permission. Scene grain and
label positions use 96 dpi pixel conversion; lengths use the declared file unit.

## Validation

All Cargo commands ran from `pattern/` with the assigned external target and
`CARGO_BUILD_JOBS=3`, as requested. No new workspace dependency, lockfile commit,
Marker edit or edit in another checkout is needed.

Commands used:

```text
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo check -p beta-pattern-core --lib --target wasm32-unknown-unknown
cargo bench -p beta-pattern-core --bench evaluate
cargo fmt -p beta-pattern-core
```

Results: **51 workspace tests passed**, strict workspace Clippy passed with zero
warnings, and the core passed the `wasm32-unknown-unknown` library check.
The final commit hash is recorded in the handoff message. The full suite contains 41 core tests (including the documentation test) plus
nine digitizer tests and one existing WASM test: 51 total. The catalog coverage
test exercises all 35 entries within one parameterized test.

PDF checks include page boxes, exact calibration-square dimensions, overlap
translations, cross-reference byte offsets and PLT coordinates. CLI smoke runs
exported the M bodice to A4, A3 and PLT. A4 produced nine pages and A3 five pages,
including calibration. `pypdf` and PyMuPDF read the PDFs; rendered calibration
and tile pages were visually checked. Proof output remains external build/test
output and is not committed. Physical printer and plotter hardware are untested.

## Performance

The release bench evaluates all 1,000 drawing objects and all 30 pieces in each
of six sizes, including 30 cubic boundaries, allowance calculations, formula
points and error checks at 0.1 mm tolerance. It returns 180 pieces. Ten timed
runs follow two warmups. It asserts median wall time below 100 ms.

Measured result: **30.636 ms median, 28.341 ms minimum, 43.196 ms maximum** on the
shared machine. The original path took about 348 ms. Improvements are shared
construction caches, skipping zero factory displacement work, allocation-free
segment checks with bounding-box rejection, and returning drawing geometry once
instead of copying the full drawing point map into every piece.

This is a reproducible synthetic workload, not a bound for every possible set
of 1,000 complex curves/intersections. The original `grade` API retains its full
per-piece drawing-point maps and is intentionally more expensive.

## Dependencies and licences

**No new crate was added.** The writer, CSV reader, rule library and benchmark
are original Rust. Existing serde, serde_json, anyhow, roxmltree and Marker core /
DXF dependencies remain. roxmltree was already in the base crate and is MIT OR
Apache-2.0. Original fixture data is Apache-2.0. Existing FreeSewing starter data
retains its original MIT attribution and licence file. Runtime network access is
not used. No machine-specific data/build path is committed.

## Remaining gaps and next work

- Family coverage is broader than verified Seamly interchange coverage. Many new
  XML family attributes are tested against our own hand-written structures, not
  application-exported files for every tool. Add more original application-made
  conformance samples. Excluded nodes, custom per-node allowances and generated
  geometry-variable names still fail clearly. Layout, display style and tracing
  images are warned as omissions. Explicit size/stature selections are required
  for multisize pattern import.
- Offset loops are rejected, not trimmed. Piece holes/cutouts are rejected by the
  XML reader. Tight concave allowance cases need robust loop removal before broad
  factory use. Union needs complete shared segments and rejects partial joins,
  holes and disconnected results.
- Union and garment fullness edits produce fixed geometry for the chosen size.
  Source pieces remain available. Group transforms are parametric. Review copy
  quantities and labels after joining pieces. Independent point grading and a
  mesh-based internal warp remain useful extensions.
- True-dart construction matches a leg length along its ray. Seam truing uses
  uniform scale. Neither is a full folded-cloth simulation or a fit guarantee.
- DXF uses public AAMA-style geometry conventions and our BETA JSON grade-table
  extension. Native proprietary RUL import and formal ASTM conformance are not
  implemented or claimed. Existing Marker DXF still omits fold-line/block-margin
  fields; Marker JSON carries the fold line.
- PDF/PLT text uses ASCII and built-in fonts. Devices and printer scaling must be
  checked in the cutting room. PLT is a drawing plot, not cutter control output.
- Background tracing and image calibration belong to the concurrent UI agent.
  No browser, WASM wrapper or marker-route file was edited here.

Requested lead actions: merge this branch's tested core commits, wrap the print
and multisize APIs in the UI/WASM layer, and keep the external application's
interoperability results separate from the BETA fixture tests. No workspace
manifest or new dependency change is requested.
