# Style engine polish report

Branch: `marker/polish-engine`, based on `marker/mvp` at `ad3173c`.
Date: 21 October 2026. Rust engine, TypeScript contracts and tests only; no web
screen edits. All checks were headless. No push or publication.

## Changes

- **Rule reports:** overlay and validator return the same per-rule, garment and
  copy-pair measurements. Rows include matched/off/not-placed status, signed X/Y
  phase residuals in mm, marker points, piece/point references and placement
  indices. Missing garments get explicit rows. Shared physical layers are
  checked for every garment. Oversized reports fail instead of truncating passes.
- **Block layout:** all seven blocks arrive in separate columns with at least
  50 mm cut-outline clearance across S?XXL, with vertical grain. Point formulas
  remain editable. Chart updates recompute column offsets. The aligned grade
  nest remains separate. Shirt yoke construction is independently editable.
- **Path tools:** linked left/right offsets for lines, cubics and circular arcs,
  including rigid transforms; parallel copies; and linked line extend/trim to
  an intersection. Curve samples use analytic normals and an adaptive 0.01 mm
  target. Unsafe cusps, foldbacks and intersections return errors.
- **Rust edge picking:** nearest points, zero-based piece edges, directed seam
  distances and curve parameters. Native curve parameters follow source
  traversal; reversed and factory-graded edges have regression coverage.
- **Derived grading:** new split/join/dart/pleat/offset boundaries retain recipes
  and re-evaluate sources in every size. Dart transfer/close remain available.
  Pleat guides regrade. Joins retain exterior allowances, marks and namespaced
  anchors, and remove notches from the deleted seam. Chart changes, save/load,
  curved source edges and source factory rules are covered.
- **Fold handoff:** missing notch/drill twins are created on the folded/tubular
  job clone. Existing twins and axis marks are deduplicated. The source document
  and open-lay behavior stay intact. Outline and allowance symmetry remain
  strict; shaped marks retain their vertices and labels.

The full contract is in [STYLE-API.md](STYLE-API.md). Runtime bindings now return
`NativeStyleCore`; new edit batches use `StyleEditOperation`, and complete
match rows use `MatchReport` / `NativeMarkerOverlay`. Existing default
`StyleCore`, `StyleOperation` switches and UI test doubles still typecheck.
Darts accept `seam_edge` directly from Rust hits, while retaining the legacy
one-based sampled `edge` input.

## Executed checks

All Cargo commands used the assigned target directory and three build jobs.
Final native suites ran sequentially with one test thread. Dependencies came
from the existing offline caches.

| Check | Result |
|---|---|
| Pattern `cargo test --workspace --offline -- --test-threads=1` | **114 passed**, 0 failed or ignored |
| Pattern `cargo clippy --workspace --all-targets --offline -- -D warnings` | Passed |
| Marker tests: core, match, export, nest, wasm, cli | **223 passed**, 0 failed, 4 existing ignores |
| Strict Clippy, same six Marker crates, all targets | Passed |
| Vitest, `marker/packages/marker-wasm/test`, three workers | **31 passed**, four files |
| TypeScript: marker-wasm and marker-web | Passed; no UI edits required |
| Pattern and Marker release WASM builds | Passed with the supplied wasm-bindgen 0.2.129 |
| Existing real-WASM nester smoke in Node | Passed: four placements, 173 stripe segments, two rounds |
| Additional real-WASM contract check in Node | Passed: S?XXL curved split/join regrade after chart edit and save/load; seven pieces per size; offsets; native hit parameter 0.3; automatic fold marks; four matched and four not-placed report rows; overlay/validator parity |
| `git diff --check` | Passed |

The four existing ignores are the long Simon production pipeline, validator
speed test, QA-006 fixed-placement budget matrix, and 60-second shirt quality
check. The first broad run's short-budget Simon matching fixture timed out
while another build was compiling. It passed in the final isolated run with
its original time limit; no production search limits were changed.

The seven new Pattern regression tests cover block spacing before/after chart
changes, S?XXL split/join/dart/pleat geometry, source factory grading, native
curve normals and parameters, reversed edges, line intersections, and all four
notch shapes plus an unduplicated axis drill on folded pieces.

## End-to-end evidence

The expanded native flow creates a T-shirt, edits curve handles, makes both
curve offsets, picks a side seam in Rust, adds one shaped side notch, grades
four sizes, creates its fold twin during handoff, nests with stripes and gaps,
checks per-rule reports, and exports PDF, DXF and PLT. Blank-sheet drafting and
exports remain covered by the companion integration test.

Result: **16 placements**, **5,680.790 mm** cut extent, **5,710.790 mm** spread
length, **56.4404%** occupied efficiency, **703** clipped stripe segments and
**four matched rules**. Phase residuals round to 0.000 mm. Nesting runtime was
30.009 seconds. These are synthetic integration results, not a performance
or fabric-saving claim.

Logs are under `$CARGO_TARGET_DIR`: `pattern-tests-final.log`,
`marker-tests-final.log`, `pattern-clippy.log`, `marker-clippy.log`,
`pattern-wasm.log`, and `marker-wasm.log`. Export and JSON evidence is under
`$CARGO_TARGET_DIR/style-evidence/`. No UI screenshots were needed or claimed.
Generated binaries, caches and evidence files are not committed.

## Deliberate limits

- Curve offsets currently support lines, cubics and circular arcs. Other path
  kinds return a clear error. Offset polylines approximate the analytic normal
  curve; they are not exact cubic representations. Extend/trim is line-to-line.
- New derivatives are linked; existing saved fixed polygon derivatives cannot
  be reconstructed without their missing history. Keep source pieces, setting
  both copy quantities to zero when they are construction-only. Apply factory
  rules to sources rather than adding a second rule table to linked outputs.
- Split/dart/pleat changes still require allowance and mark review. Uniform
  allowances are retained; nonuniform source allowances start at zero on those
  new boundaries. Joins preserve exterior source allowances and marks. Curved
  dart mouths are rejected; straight `seam_edge` selections are stable even
  after preceding curves.
- Blocks remain starting drafts, not fit-certified production patterns. Physical
  fit, cutter hardware and desktop windows were not tested in this engine round.

---

# Original style engine report (historical)

The report below records the earlier style-engine round. Its fixed-derivative,
stacked-block and manual-fold-twin limitations are superseded above.

# Style engine implementation report

Branch: `marker/style-engine`, based on `marker/mvp` at `dac704f`.
Date: 21 October 2026. No push, publication, deployment or visible app launch.

## API and implementation

The API-first commit is `4306aa8`. The typed offline loader is
`@beta/marker-wasm`'s `loadStyleCore`; the versioned document remains
`beta-pattern.doc/v1`. See [STYLE-API.md](STYLE-API.md) for exact requests.

- Immutable, all-size-validated edit transactions cover point/curve/arc
  construction, handle movement, replacement/deletion, pieces, annotations,
  seam/hem allowances and corners, charts and factory rules. Existing shape
  tools supply mirror, split/join, offset, darts and pleats. Undo keeps snapshots.
- Formula-preserving dragging adds a displacement without replacing the
  underlying measurement formula. Invalid edits return errors and retain the
  caller's prior document. Save/load validates the version and all sizes.
- Rust provides point/path/piece hit testing, length/distance/angle measurements,
  graded size-nest geometry and sewing reports. Cap-ease requirements update
  after variable changes. Reports retain each size's required ease.
- Full-piece fold declarations are checked against reflected cut geometry.
  Asymmetric allowance edits fail instead of silently producing a wrong half.
- Default body notches are paired across the fold. Folded/tubular handoff
  rejects one-sided notches or drills on declared fold pieces, so a chosen
  half cannot silently omit a cut mark. Open lays keep one-sided marks.
- Seam-distance notches project to the allowance cut edge. Slit depth and
  V/T/castle geometry survive marker plotting. Joining pieces keeps exterior
  notches and removes notches on the deleted joining seam.
- `to_job` selects one fabric group, grades the chosen ratio, transfers grain,
  fold axes and marks, applies gaps/buffers/fusing blocks, and expands template
  match-point rules for each selected size.
- Typed lay instructions and repeat colour bands use validated job metadata.
  The established full/folded/tubular layer-allocation contract is preserved.
  Matching and shade checks now inspect every garment served by a shared cut.
- The overlay API returns clipped stripe/check lines, graded outlines and
  previous-version geometry differences. Optional PDF/PLT guides use pen 3
  and internal layer 8. They do not become cutter contours.
- The existing browser worker routes matching/shade jobs through the matching
  engine; plain jobs keep the existing browser session. Its message protocol
  is unchanged. `POST /api/pattern/style` uses the same dispatcher and existing
  server authentication/body limits.
- Unrepresentable matching time limits return a plain error instead of
  panicking while constructing a deadline.

Core runtime commit: `ec16238`; matching/server/export integration: `ac66104`;
fold/notch/chart robustness: `aff6860`. Later verification fixes and the report
commit are listed in the handoff.

## Blocks and sewing results

The formulas and limitations are documented in [BLOCKS.md](BLOCKS.md).
Both original example charts use S/M/L/XL/XXL. They are editable body-measurement
examples, not certified ISO sizing or Indian population-survey data.

Tests cover **70 style/size combinations, 330 closed cut outlines and 280
graded sewing-check rows**. The permitted sewing error is 1 mm. Maximum actual
errors below include both example charts; the small nonzero cap error comes
from curve flattening.

| Block | Pieces per size | Sewing rows | Largest error, mm |
|---|---:|---:|---:|
| T-shirt: front, back, sleeve, neck rib | 4 | 50 | 0.001715 |
| Shirt: front, back, yoke, sleeve, cuff, collar, stand, placket | 8 | 80 | 0.000868 |
| Trouser: front, back, waistband, pocket bag | 4 | 20 | 0 |
| A-line skirt: front, back, waistband | 3 | 10 | 0 |
| Kurta/tunic: front, back, sleeve, neck binding, placket | 5 | 50 | 0.001715 |
| Polo: front, back, sleeve, collar, placket | 5 | 50 | 0.001715 |
| Track pant: front, back, waistband, pocket bag | 4 | 20 | 0 |

Checks include side seams, shoulders, both sleeve-cap halves with stated ease,
sleeve underarms, shirt back/yoke, cuff/sleeve, collar/stand and trouser inseams.
The T-shirt neck rib explicitly assumes 15% stretch; it is not reported as a
zero-ease join. Positive sleeve-cap ease is tested separately. Chart editing,
formula-preserving edits, fold rejection and JSON round trips also have tests.

## Native end-to-end result

The original test creates a T-shirt, edits matching armhole handles, adds
paired shaped notches and hem allowance, grades S/M/L/XL, sets a 750 mm folded lay with
3 mm piece gaps, 5 mm edge gaps, 10 mm transverse stripes, matched front/back
side hems, 20 physical plies and 15 mm allowance at each spread end. It nests
for 30 seconds, validates independently, builds the overlay and exports
PDF/DXF/PLT. DXF and PLT are read back by BETA's importers.

The recorded result has **16 placements**, **5,680.790 mm** cut extent,
**56.4404% occupied efficiency**, and **703 clipped stripe segments**. The
recorded nesting runtime is **30.009 seconds**. Spread length includes another
30 mm of end allowance. This is a synthetic integration check, not a fabric
saving or commercial-performance claim. The blank-sheet test separately
builds a closed piece in one atomic transaction and exports it.

Evidence is under `$CARGO_TARGET_DIR/style-evidence/`: `tshirt.doc.json`,
`job.json`, `result.json`, `validation.json`, `seams.json`, `block-matrix.json`,
`overlay.json`, `tshirt.dxf`, `marker.pdf`, `marker.plt` and `marker.svg`.
Generated binaries and evidence files are not committed. UI screenshots are
the `style-ui` agent's responsibility; no screenshots are claimed here.

## Executed verification

All commands used the assigned build/cache directories and three Cargo build
jobs. All inputs were original synthetic or existing public fixtures.

| Check | Result |
|---|---|
| Pattern `cargo test --workspace --offline -- --test-threads=2` | 107 passed, none failed or ignored |
| Pattern strict Clippy, workspace and all targets | Passed |
| Marker `cargo test --workspace --exclude beta-cutting-room --offline -- --test-threads=2` | 411 passed, none failed, 9 existing ignores |
| Marker strict Clippy, workspace/all targets excluding desktop | Passed |
| Focused core tests after the count-bound guard | 42 passed, 1 existing performance ignore |
| Vitest `run marker --maxWorkers=3` | 201 passed across 24 files, including 4 new loader tests |
| TypeScript checks for marker-wasm and marker-web | Passed |
| Native 30-second folded/matching/export flow | Passed; included in Marker tests |
| Public Simon QA-006 locked-placement matrix | Passed all four budgets; all results valid and locked identity unchanged |
| Pattern and Marker WASM release builds | Passed with the assigned wasm-bindgen tool |
| Real generated WASM modules in headless Node | Passed: one-size folded/matched T-shirt, 4 placements, 173 clipped stripe segments |
| `git diff --check` | Passed |

QA-006 measured wall times were **1.017 / 5.042 / 15.046 / 60.040 seconds** for
the 1 / 5 / 15 / 60 second budgets. The 114-placement marker lengths were
8,818.486 / 8,818.486 / 8,803.198 / 8,780.815 mm. This checks the existing
budget-plus-ten-seconds gate on the public fixture; it is not a general hard
real-time or packing-quality guarantee. The plain/open search algorithm was
not changed.

## Limits

These are starting blocks, not fit-certified production patterns. Shirt
collar/stand and placket components, pocket bags and waistbands are deliberately
simple. Closure extensions, pocket entries, shaped collar fitting, sleeve
placket slashes and polo/kurta openings need operator drafting. Physical sample
fit, fabric stretch and production-machine output were not tested.

Split/join/offset/dart/pleat derivatives use fixed geometry and need grading
review. Factory point rules retain their existing base-geometry semantics;
attaching them does not add a second deformation on top of measurement grading.
Allowances that cross themselves are rejected, not automatically repaired.
Ambiguous corner notches may require moving the notch.
For folded/tubular handoff, an asymmetric cut mark must have a reflected
partner, or the fold constraint must be removed. This check does not prevent
one-sided marks in an open lay.

Lay face, physical plies and end allowances are spreading instructions, not
new flip permissions or a replacement for physical-layer allocation. One
marker contains one fabric group. `fabric.max_length` limits cut extent;
reserve spread-end waste separately from the table limit.

Overlay lines are bounded to 50,000 candidates/segments; very fine repeats
can return a clear limit error. Band centre lines are clipped in Rust; a UI
must also clip thick painted band strokes to the supplied outline. PDF/PLT
guides are thin centre lines. Previous-marker comparison is a geometry diff.

PDF is structurally/scale tested by the writer's native tests; there is no PDF
pattern importer to round-trip. DXF/RUL support remains BETA's documented
subset, not universal proprietary CAD interoperability. No external service,
vendor implementation, new third-party licence or private factory data was
introduced. The direct Jagua dependency reuses the already approved, unmodified
MPL-2.0 exception solely for its native/WASM-compatible clock.
