# Browser factory files and live layouts

Round 0.7 contract. All geometry and file generation run in Rust. These calls
load local WASM assets and work offline after those assets are cached.

```ts
import { loadMarkerExportCore, loadQuickMarkerCore, loadPatternCore } from '@beta/marker-wasm';
const files = await loadMarkerExportCore();
const plt = files.exportMarker(job, result, 'plt'); // Uint8Array
const dxf = files.exportMarker(job, result, 'dxf');
const roll = files.exportMarker(job, result, 'pdf'); // 1:1 vector PDF
const tiles = (await loadQuickMarkerCore()).tiledMarkerPdf(job, result, {
  paper: 'A4', overlap_mm: 10, // A3 also supported
});
const pattern = await loadPatternCore();
const pieces = pattern.export_dxf(document); // DXF-AAMA text, all sizes
const patternPlt = pattern.export_plt(document); // HPGL text
const patternPdf = pattern.export_pdf(document, 'A4'); // 1:1 A4/A3 tiles
```

`MarkerExportFormat` is `svg | plt | hpgl | pdf | dxf | iso`. PLT and HPGL
are aliases. The bytes come from `beta-marker-export`'s native writers.
`MarkerExportOptions` accepts `marks` and `labels` (default true), `rotate`
(HPGL only, default false), and `pdf_tile_mm` (optional square tile size).
Roll PDF automatically tiles both axes beyond the PDF 5080 mm page limit.
Physical A4/A3 tiles use `tiledMarkerPdf`, with overlap and calibration marks.
Print at actual size; no fit-to-page scaling. ISO uses the existing default
machine profile. Review that profile for the cutter before production.

Invalid JSON, unsupported formats and writer errors throw. These calls format
the supplied layout, like the native API; call `MarkerCore.validate` and review
the result before cutting. Set `job.meta.beta_stripe_guides=true` on an
export-only clone to include clipped stripe guides in PDF/PLT/SVG. ISO excludes
guides. Options and dimensions have exactly the native writer semantics.

The static transport handles `POST /api/export?format=...` with
`{job,result,options?}` and returns binary `Response` data, `Content-Type` and
`X-Beta-Extension`. Existing export calls need no UI-specific file writer.
Other API routes retain their existing behaviour.

## DXF import

```ts
const importer = await loadMarkerImportCore(); // from @beta/marker-wasm
const report = importer.importDxf(bytes, { units: 'auto', pairs: false }, optionalRulText);
```

This is the native `beta-marker-dxf` importer, with the same `ImportReport`
(pieces, sizes, units, dialect, warnings and `pairs_assumed`). Since round 0.7
an omitted `pairs` means `true`, natively and in WASM: an even Quantity of an
asymmetric piece becomes left/right copies and `pairs_assumed` is `true`;
symmetric pieces keep same-hand copies. Pass `pairs: false` for same-hand copies. Source units accept `auto`, `mm`,
`cm` or `inch`; geometry is returned in mm. The optional `.rul` text uses the
existing experimental grading parser. Unknown or unsupported variants still
produce the native errors/warnings. Private factory files are not bundled.

Static `POST /api/import?format=dxf&units=auto&pairs=true` (the default) accepts original
file bytes and returns `{report}`. The existing Open DXF button uses this route.
Quantities, names, sizes, grain, internal lines, notches and drills stay in the
Rust report. PLT input remains on its existing native route.

## Piece labels

`files.labels(job, result.placements)` returns `MarkerLabel[]` in placement
order. Each entry has `placement`, `piece_id`, `name`, nullable `size`,
`anchor:[x,y]`, `rotation_deg`, and `clearance_mm`. Units are mm; Y points up;
angles are counter-clockwise. Anchors lie inside placed pattern outlines,
including shrinkage, mirroring and folded halves. Fusing block margins never
move text outside the pattern itself. The angle is
normalized for readable text. Clearance is the distance to the nearest edge,
not a guarantee that an arbitrary string fits. Fit or hide text at small zoom.
An invalid/unknown piece or unusable polygon throws rather than placing a
misleading label. No label geometry should be computed in TypeScript.

## Live nesting messages

The existing `createMarkerNester().nest(job, options, onProgress)` callback
continues to receive `NestProgress`:

```ts
{ round, seconds, round_seconds, improved, done, cancelled,
  result: { placements, length, efficiency, /* existing MarkerResult fields */ },
  memory_bytes, checkpoint?: true }
```

Every `result` is a complete independently validated best-so-far layout.
Efficiency is a fraction, not percent. Within-round improvements set
`checkpoint:true` and are throttled to at most one per 500 ms. Round-boundary
and final events remain available even without an improvement. A checkpoint
does not request a `continue` acknowledgement; the Worker is still searching.
The private Worker envelope remains `{type:'progress',id,progress}`; existing
callers can ignore `checkpoint`. Stop/watchdog recovery retains the latest
checked layout. Callback changes cannot mutate the retained recovery copy.
Matching and fold-constrained jobs must pass their original job's rule checks
before any layout can be published.

Repeat-matching search publishes only after all matched placements are solved;
its free baseline is not a valid checkpoint. Shade-band subjobs report the
whole marker at round boundaries, because an individual band is incomplete.
Native `GET /api/nest/status` adds nullable `result:MarkerResult` alongside
`best_length_mm` and `best_efficiency`. Layout updates are throttled to 500 ms;
the completed result is always retained. Native status remains the current
server search, so clients must not treat another client's job as their own.

## Repeatable byte-parity test

From `marker/`, after remapped release WASM builds:

```sh
cargo run -p beta-marker-cli --example export_parity -- out/export-parity
node packages/marker-wasm/scripts/export-parity.mjs out/export-parity
```

The native example creates only synthetic geometry and the original BETA
starter document. The test compares 46 files byte for byte: marker formats
with default and custom options, shrinkage, rotations, mirrors and stripe
guides, A4/A3 marker tiles, plus pattern DXF, PLT and A4/A3 PDF. It does not
compare timed nesting output, which is intentionally not deterministic.

The harness also compares six native/WASM import reports: synthetic AAMA mm,
ASTM mm, AAMA inch, explicit mm/pairs options, and cumulative/incremental `.rul`
grading. It prints counts for each output format and each import case.

## DXF robustness coverage

The synthetic Rust regressions cover AAMA/ASTM, inch/mm, mirrored INSERTs,
internal lines, grain, notches and drill holes; equivalent LWPOLYLINE, classic
POLYLINE and joined LINE boundaries; TEXT grade anchors and `.rul` displacement.
Existing tests cover nested INSERT transforms/base points, attributes, MTEXT,
curves, quantities, pair expansion, malformed input and export/import round trips.

Two importer defects are fixed:

- A block-only pattern library with no model-space INSERTs now imports each
  root block once, with a quantity-review warning. Nested helper blocks and
  reserved model/paper-space definitions are excluded. Explicit INSERTs remain
  authoritative, so unused definitions are not accidentally cut.
- INSERT attributes override ATTDEF defaults case-insensitively. Metadata tags
  such as `PIECE_NAME` now match their space-separated text equivalents.

These fixtures are synthetic or pre-existing public fixtures. The optional
rule-table parser remains experimental; arbitrary vendor dialect support is
not implied by these checks.

## Round 0.7 verification (22 October 2026)

Native reference files and the remapped release WASM passed exact byte comparison:

| Output | Identical / compared |
| --- | ---: |
| Marker SVG | 6 / 6 |
| Marker PLT | 6 / 6 |
| Marker HPGL alias | 6 / 6 |
| Marker roll/square-tile PDF | 6 / 6 |
| Marker DXF-AAMA | 6 / 6 |
| Marker ISO | 6 / 6 |
| Marker A4 tiles | 3 / 3 |
| Marker A3 tiles | 3 / 3 |
| Pattern DXF, PLT, A4 PDF, A3 PDF | 1 / 1 each |
| **Total files** | **46 / 46** |

All six import reports also matched exactly, including both grading-rule modes.
No exporter mismatch required a production-code change in the verification pass.

Final packaged WASM sizes (bytes; gzip level 9):

| Module | Raw | Gzip |
| --- | ---: | ---: |
| Marker, including factory writers, labels, checkpoints and DXF importer | 4,368,636 | 1,283,856 |
| Pattern | 5,022,540 | 954,106 |
| Floor | 3,799,359 | 932,071 |

Marker baseline was 3,894,325 raw / 1,170,555 gzip. The complete addition is
474,311 raw bytes (+12.18%) / 113,301 gzip bytes (+9.68%). The earlier 53,412-byte
raw increase excluded the importer and is not the final download-size increase.

Build all four modules through `deploy/release-build.mjs` (`marker-wasm`,
`pattern-wasm`, `floor-wasm`, and `cutroom-wasm` since plan7) with the external Cargo target and three build jobs.
Set `BETA_SKIP_WASM_BUILD=1` when running `npm run build:static -w @beta/marker-web`
afterwards: its default rebuild would otherwise replace the remapped modules.
Each actual packaged `out/static-app/wasm/*_bg.wasm` passed
`node deploy/release-build.mjs --verify <file>` (UTF-8 and UTF-16 path scans).

Completed gates:

- Marker `cargo test --workspace`: **451 passed, 0 failed, 9 ignored** by the
  existing suite; includes 39 DXF, 27 export and 51 nesting unit tests.
- Pattern `cargo test --workspace`: **114 passed, 0 failed**, including doctests.
- Both workspaces `cargo clippy --workspace --all-targets`: passed. Marker has
  one non-blocking `chunks_exact_to_as_chunks` suggestion in `labels.rs`.
- Root Vitest: **436 passed across 40 files**; workspace TypeScript checks passed.
- Three remapped release WASM builds, static packaging and three packaged-module
  path scans passed.
- The saved first-session static Style smoke remains **14 / 14**, with seven
  real downloads and no console errors or external requests. Its separate
  synthetic DXF Open -> nest -> PLT/DXF/PDF/ISO/SVG journey records five real
  downloads, no errors and no HTTP API/external requests. Those browser journeys
  were reviewed, not rerun in the verification session; production code did not
  change after them.

Remaining limits: the nine opt-in Rust tests were not enabled; machine/plotter
calibration and arbitrary vendor dialect acceptance are outside these synthetic
parity checks. ISO still uses the existing generic profile. No private factory
data or derived artifacts are included.
