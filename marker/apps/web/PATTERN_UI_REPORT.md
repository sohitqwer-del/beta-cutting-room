# Pattern drafting editor report

Branch: `marker/pattern-ui`. Base: `609da9b`.

## Result

The Pattern tab is now a drafting editor. A maker can create a document, enter
measurements, place constructions, make and mark a piece, grade it, and export it
without editing JSON. The other cutting-room tabs retain their existing workflows.

The engine agent's tested commits were merged through `70e2e5b`:
`3f1d4e4`, `4fb1045`, `a981642`, and `70e2e5b`. The last commit supplies the
shared evaluator, print writers, Clippy cleanup and expanded import coverage.
No engine source was edited directly by the UI agent.

## Working features

- SVG draft canvas with 1 mm, 5 mm or 1 cm grid; point and path snapping;
  zoom, fit and pan; labels; size selection; reference selection on the canvas
  or through keyboard-accessible lists. Numeric absolute and polar points can
  be moved. Arrow keys move a selected movable point by 1 mm, or 10 mm with
  Shift. With no movable point selected, arrows pan. Plus/minus zoom; 0 fits.
- All 35 construction forms come from `tools::catalog()`. The palette groups
  points, lines, curves, arcs, operations and pieces. Labels, defaults, reference
  filters, choices and one-line tooltips come from the typed catalog. Group
  move, rotate and mirror operations retain their parametric sources. Groups
  can be shown or hidden.
- Safe formula fields, live Rust checks, a measurement/variable picker and
  autocomplete for the active `m.` or `v.` token. Tangents and cubic paths use
  compact labelled coordinate/ID fields, rather than a document JSON editor.
- Ordered construction history; selection and highlighting; editing; explicit
  transitive dependency warnings before deletion; undo/redo with 100 snapshots.
  Failed edits retain the accepted document. Construction order persists in
  document metadata. Undo snapshots are session-local.
- Multisize and individual measurements, base size selection, adding sizes and
  measurements, variables/ease, base-plus-size steps, CSV import/export, and
  individual/multisize Seamly measurement XML import through the engine. Steps
  are retained in document metadata. CSV accepts both the engine's long table
  and a convenient wide table headed `measurement,M,L`.
- Piece construction from ordered path clicks or lists, reverse/remove edge
  controls, per-edge allowance formulas and corner types, grain/fold lines,
  quantities/pairs/material, label positions, cutting instructions, slit/V/T/
  castle notches, drills, buttons, text, seam-intersection notches, internal
  marking paths and named anchors. Inline checks include closure/crossings and
  evaluated edge lengths.
- Measurement grading, a relative/incremental dx/dy rule library, point-rule
  attachments, return to measurement grading, coloured size overlays, and a
  chosen reference point. The engine aligns the nest.
- Piece menu actions for dart create/transfer/close, slash-and-spread, knife/box
  pleats, split, mirror, seam walking and seam truing. Results keep their source
  pieces. Quantity/material settings and a uniform allowance are retained.
  Topology-changing results have fixed coordinates at the chosen size; the UI
  explains that grain, marks, allowance and grading need review.
- JSON save/open and browser autosave, DXF-AAMA/ASTM-style output with all sizes
  and BETA grade-rule comments, A4/A3 tiled 1:1 PDF with 10 mm overlap and a
  calibration page, PLT, and sizes/ratio handoff to Editor. Pattern import accepts
  `.sm2d`/`.val` plus measurements. For a multisize imported pattern, importing
  its measurement table in Measurements expands the document size rows.
- Photo tracing from PNG/JPEG, two-point known-distance calibration in Rust,
  opacity control, JSON/local-storage retention, and a link to Digitize. Photos
  are limited to 3 MB to leave room for browser storage. A quota failure is
  reported and JSON backup remains available.
- A nine-step in-app first-shirt-block guide, including the original BETA armhole
  and neck control-point formulas. It explains the need for a fit sample.

All garment geometry, formula evaluation, grading, snapping, calibration and
piece operations run in Rust. TypeScript handles forms, history snapshots,
selection, SVG display and file entry. The final PDF/PLT adapters wrap the core's
writers; there is no second print implementation or new heavy export dependency.

## Verification

All commands used the assigned external Cargo target directories, assigned
`WASM_BINDGEN`, and `CARGO_BUILD_JOBS=3`. Build output and generated QA files are
not committed. No private data or GPL/vendor code was used.

From the repository root:

```text
npm install --cache <assigned npm cache>
npm run build:wasm -w @beta/marker-wasm
npm run build:pattern-wasm -w @beta/marker-wasm
npm run typecheck -w @beta/marker-wasm -w @beta/marker-web
npx vitest run marker
npm run build -w @beta/marker-web
```

Both WASM builds, both TypeScript workspace checks and the production Vite build
pass. Vitest: **28 passed in 5 files**. New state tests cover snapshot isolation,
undo/redo branching and bounds, construction order, formula completion, directed
piece paths and transitive dependency deletion. No runtime npm package was added.

From `pattern/` with the assigned pattern target:

```text
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

**56 tests passed**, including six WASM-adapter tests and the core doc test.
Strict Clippy passes without warnings. The adapter tests draft/grade/export a
rectangle, reopen saved geometry, check CSV and formula failures, exercise every
piece-menu operation, retain sources and check the exact trued-seam length.

From `marker/` with the assigned marker target and `BETA_DATA_DIR`:

```text
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets
```

The final full workspace run passed: **227 passed, 4 ignored, 0 failed**. This
includes nesting, planning, receiving,
cutting-room, print/export, WASM and CLI tests. The new pattern-route test covers
catalog forms, drawing inspection, formula errors, physical PDF/PLT exports and
measurement CSV round trips. Workspace Clippy passes without warnings.

Final test logs are local in `.qa/pattern-tests.txt`, `.qa/pattern-clippy.txt`,
and `.qa/marker-tests.txt`. The earlier full Marker compile took about 24 minutes
on the shared machine; no build jobs outside this assignment were changed.

## Browser evidence

The browser used the native T3 collaborative preview and the actual Rust WASM.
The committed `src/pattern-smoke.mjs` drives the same form/click handlers as a
maker. It does not inject a ready-made document. In a local Vite page, its entry
points are:

```javascript
const smoke = await import('/src/pattern-smoke.mjs');
await smoke.runPatternSmoke();
await smoke.runPatternChecks();
await smoke.runPatternTraceChecks();
await smoke.benchmarkPattern();
```

The first script starts a blank document, places four formula-driven points and
four lines, builds a rectangle skirt piece, sets 12 mm allowance, grades M/L,
exports DXF, and measures actual WASM calls. The extended checks passed for
invalid-edit retention, dependency warning/deletion, undo/redo, shaped notches,
grain/labels, seam walking, rule-based and measurement grading, stored measurement
steps, PDF/PLT/JSON downloads and size-ratio handoff to Marker. Group forms,
keyboard movement and tracing were also checked. A synthetic original photo
calibrated to **2 mm/pixel**; its photo and scale survived reload.

Measured on this machine, in the browser:

- Rectangle: 8 constructions, 1 piece, 2 sizes; 30 inspect/grade calls averaged
  **1.407 ms**, range **1.1-1.7 ms**.
- Teaching bodice: 36 constructions, 2 pieces, 4 sizes. Thirty warm runs which
  inspect every size and grade the document measured **16.6 ms median**, **18.7 ms
  p95**, **19.4 ms max**. A real ease edit through the UI measured **24.8 ms**.
- All six Pattern modes fit **390 x 844** without page-level horizontal overflow.

These are local smoke measurements, not hardware-independent timing guarantees.

Downloaded PDF files were checked with Poppler, rendered and inspected. The A4
file has 13 pages at **210 x 297 mm**; A3 has 9 pages at **297 x 420 mm**. Both
calibration squares measure **100 x 100 mm** in the PDF drawing coordinates.
Rendered tiles show the overlap marks, piece/size labels and tile identifiers.
The printer and physical cutter were not available for hardware calibration.

Screenshots were captured from native preview recordings, then extracted as PNG
frames with FFmpeg. Paths relative to this worktree:

```text
marker/apps/web/.qa/screenshots/draft.png
marker/apps/web/.qa/screenshots/measurements.png
marker/apps/web/.qa/screenshots/piece.png
marker/apps/web/.qa/screenshots/grading.png
marker/apps/web/.qa/screenshots/output.png
marker/apps/web/.qa/screenshots/guide.png
marker/apps/web/.qa/screenshots/digitize.png
marker/apps/web/.qa/screenshots/mobile-draft.png
marker/apps/web/.qa/screenshots/pdf-final-01.png
marker/apps/web/.qa/screenshots/pdf-final-02.png
```

Additional local evidence: `.qa/browser-results.json`, `.qa/pdf-checks.json`,
`.qa/skirt-a4.pdf`, `.qa/skirt-a3.pdf`, `.qa/skirt.dxf` and `.qa/skirt.plt`.
The Vite watcher ignores `.qa/` so rendering a PDF or writing screenshots does
not trigger page reloads or Windows file-watch errors.

## HTTP and WASM integration

Existing starter/import/grade/DXF/from-job/to-job contracts remain available.
New POST routes are:

```text
/api/pattern/catalog
/api/pattern/inspect
/api/pattern/formula
/api/pattern/snap
/api/pattern/image-scale
/api/pattern/tool
/api/pattern/piece-action
/api/pattern/move-point
/api/pattern/nest
/api/pattern/measurements-import
/api/pattern/measurements-export
/api/pattern/export-pdf
/api/pattern/export-plt
```

Routes and WASM share the pure adapters in `beta-pattern-wasm/src/`. The server
includes those small adapter modules by path; it does not depend on the WASM
runtime or duplicate calculations. Route parsing remains bounded by the existing
loopback server body limits. The typed browser contract is in
`packages/marker-wasm/src/pattern.ts`.

## Dependencies, limits and handoff

No new crate or npm dependency was added by this work. Existing `serde_json`,
`serde`, and `wasm-bindgen` are MIT OR Apache-2.0; the local BETA crates are
Apache-2.0. The merged engine reports no new dependencies. Lockfiles, WASM/build
output, screenshots, downloads and generated data are excluded from the commit.

The engine's stated limits still apply: supported Seamly schemas are a subset;
self-crossing seam allowances fail rather than being trimmed; fixed garment
operations need a new grading review; internal marks use the documented nearest
stitch-edge grading approximation; DXF rules use BETA's public comment extension
and do not claim proprietary rule compatibility or formal ASTM certification.
PDF/PLT text uses the core's ASCII font handling. PLT is a plot, not cutter control.
Accepted digitizer-to-document conversion is intentionally single-size. Save JSON
backups before clearing browser storage. Fit-test teaching blocks before cutting.

No shared model or workspace-manifest change is requested. After the lead merges
other agents, rebuild both WASM packages and the web app, rerun both workspace
checks, then repeat the browser draft-to-marker and digitizer acceptance flows.
The engine interoperability report remains the source for verified external
schema coverage; this UI report does not broaden those claims.
