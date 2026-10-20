# Quick Marker API v1

Import `loadQuickMarkerCore` and the `Quick*`, `TiledMarkerOptions`,
`ChalkGuideOptions`, `ProjectorLayout` types from `@beta/marker-wasm`.
`await loadQuickMarkerCore()` loads the local `wasm/beta_marker_wasm.js` bundle.
Calls are synchronous after loading. Run photo scans in a Web Worker on phones.
The engine needs no threads, network, server or cloud model.

```ts
const quick = await loadQuickMarkerCore();
const scan = quick.quickScan(bytes, {reference: 'a4-sheet', photoId: 'photo-1'});
const session = quick.quickMerge([scan]);
// Review outlines, scale, grain, pairs and warnings. Set size and accepted quantities.
const job = quick.quickPiecesToJob(acceptedPieces, {width_mm: 1500}, {M: 2});
const pdf = quick.tiledMarkerPdf(job, result, {paper: 'A4', overlap_mm: 10});
const chalk = quick.chalkGuidePdf(job, result, {language: 'hi'});
const projector = quick.projectorLayout(job, result);
```

## Scan and coordinates

`quickScan(photo: Uint8Array, options: QuickScanOptions): QuickScan` accepts JPEG
or PNG, at most 20 MB encoded and 12 million decoded pixels. Other formats throw.
Rust downsamples to a 2000 pixel long edge (option 640..2400) before vision.
The returned `overlayPng` is a **source-photo** overlay at `processedSize`, not a
rectified image. Input orientation is the stored pixel orientation (EXIF is not
applied). Display the returned overlay to avoid EXIF coordinate mismatches.
`sourceSize` and `processedSize` are `[width,height]`. Coordinates in `outline`,
`grain`, `marks` and suggestions are mm, Cartesian y up. A4 origin is one sheet
corner; coordinates may be negative. Nesting handles this. `imageToMm` maps
original pixels to mm; BETA target mode also requires lens correction, so use
its overlay for review. `mmPerPx` is the local scale at the reference, not a
constant across a tilted photo.

Options: `reference` is `a4-sheet` or `beta-target`; `targetManifest` is a JSON
string (default landscape A3 BETA target); `photoId` defaults to `photo-1`;
`maxPieces` defaults to 40 (1..100); `maxLongEdge` defaults to 2000.
`a4Corners` optionally supplies four clockwise corners in original pixels,
starting at either end of the 210 mm side. It resolves sheet/pattern ambiguity.
`useSecondSheet` defaults true: a second separate A4 sheet checks consistency
of scale away from the first. It does not promise camera calibration or lens
correction. Both sheets and every piece must lie on the same flat surface.

Scale contains `confidence` (high/medium/low), `expectedErrorMm`, `tiltScore`,
`referenceCorners` in original pixels, and `method`. Error is a
**heuristic estimate**, not a confidence interval or verified real-photo bound. It excludes unknown lens distortion
and out-of-plane paper height; those can greatly exceed the estimate.
It increases away from the reference and with perspective. Plain rectangles
cannot uniquely identify A4 under unrestricted perspective. Review the highlighted
sheet; ambiguous detection warns or fails. BETA target remains the accurate mode.

Warnings have stable `code`, English `message`, and optional `pieceId`/`photoId`.
Codes include `blur`, `tilt`, `glare`, `shadow`, `piece_outside_photo`,
`low_contrast`, `scale_low_confidence`, `reference_ambiguous`,
`second_sheet_mismatch`, `touching_pieces`, `grain_unconfirmed`,
`duplicate_piece`, `review_required` and `piece_limit`. Detection cannot prove
that a photo is free from these problems. Hard calibration/decode/input failures
throw a readable error. No result must be treated as permission to cut fabric.

## Assist, review and multi-photo sessions

`QuickPiece` extends the existing `Piece` contract and adds `photoId`,
`expectedErrorMm`, `notches`, `drills`, and `suggestions`. `marks` is authoritative;
notches and drills are convenience views. Suggestions carry value, confidence
and reason. Name is a small offline shape classifier, not semantic certainty.
A symmetry axis suggests cut-on-fold; the full outline is retained. Grain uses
a drawn line where found, otherwise symmetry or long axis at low confidence.
A mirror suggestion identifies `pairPieceId` and proposes normal 1, mirrored 1.
If accepted, the UI keeps one representative and removes its pair; applying 1+1
to both would double the demand. No suggestion changes `quantity`,
`mirror_quantity`, `grain` or `fold_line` without operator confirmation.
`quickAssist(pieces)` refreshes suggestions after outline edits.

`quickMerge(scans)` namespaces IDs as `photoId:pieceId`, combines all pieces and
warnings, and returns candidate `duplicates`. Unique photo IDs are required. A session is limited to 30 photos and 300 pieces.
Duplicate detection compares physical size and aligned shape across different
photos. It cannot distinguish identical-looking parts of different sizes/styles
or prove that two photographs show the same paper. It **does not remove pieces**.
The UI asks the operator to remove confirmed duplicates, then passes accepted
pieces to `quickPiecesToJob`.

`quickPiecesToJob(pieces, {width_mm,nap?,layout?}, garments)` returns the existing
`Job`. Every accepted piece needs an explicit `size` present in garments.
All ordered sizes must have pieces. There is **no automatic grading**. The UI must
collect each size's real patterns or send them through Pattern's grading workflow.
The function requires a confirmed grain line on every piece and uses accepted
quantities/grain/fold fields only. It validates the
job, rejects invalid geometry/counts, and preserves marks. No scan images enter
the job. `fabric.width_mm` becomes `Job.fabric.width`.

## Outputs

All three outputs validate the job and result before export and use effective
shrinkage/fold geometry. Invalid or incomplete markers throw.

- `tiledMarkerPdf(job,result,{paper:'A4'|'A3',overlap_mm?:10})`: fixed physical
  portrait pages, 10 mm margins, matching crosses within overlaps, page numbers,
  first-page map and a 100 mm check line. Print at **100% / actual size**; disable
  fit-to-page. Overlap must be 5..30 mm. A page count cap prevents runaway exports.
- `chalkGuidePdf(job,result,{language?:'en'|'hi'|'ta'})`: A4 guide, table and small
  map. Each placement has name, size, rotation, flip and a marked reference point:
  the first vertex of its placed cut outline. X is measured from marker start;
  Y is from the y=0 selvedge. This is placement guidance, not an outline substitute.
  Hindi/Tamil labels are marked **needs native review**. Fixed native-script labels are pre-shaped vector glyphs with Unicode ActualText;
  dynamic piece names retain the existing ASCII export limit.
- `projectorLayout(job,result)`: JSON object with mm outlines, placement metadata,
  reference points, fabric width, recomputed marker length and a 100 mm calibration
  square size. The UI must physically calibrate both projector axes; CSS pixels
  are not millimetres. Format is `beta-marker.projector/v1`.

## Wire functions

The marker WASM exports `quick_scan(bytes, optionsJson)`, `quick_assist(piecesJson)`,
`quick_merge(scansJson)`, `quick_pieces_to_job(piecesJson,fabricJson,garmentsJson)`,
`tiled_marker_pdf(jobJson,resultJson,optionsJson)`,
`chalk_guide_pdf(jobJson,resultJson,optionsJson)`, `projector_layout(jobJson,resultJson)`.
JSON returns are strings; PDF returns are byte arrays. The TS adapter turns the
scan's JSON PNG byte array into `Uint8Array` and measures elapsed time. Loaders
accept an optional local module URL. No UI code needs to call raw bindings.

The early contract commit precedes implementation. See digitizer `ACCURACY.md`
and `REAL-PHOTO-TEST.md` for measured results and physical test protocol when ready.


## Measured runtime and memory (20 October 2026)

Measured on the shared i9-11900H laptop, single-threaded. This is an engineering
check, not an actual mid-range phone measurement. The compiled marker WASM is
about 2.58 MB (uncompressed). The headless Node/V8 check loads the real generated
wasm-bindgen module, scans, merges, accepts a sized/grain-confirmed job, and calls
all three export paths. No mocked engine is used in that check.

| Input / path | Cold scan | Repeat scans | WASM linear memory | Peak total Node process |
|---|---:|---:|---:|---:|
| 12 MP grayscale JPEG, 2000-pixel processing edge | 2.20 s | 1.59-1.77 s | 108.1 MiB | 242.7 MiB current RSS (peak not recorded in this run) |
| 12 MP RGB JPEG, same scene and processing edge | 4.00 s | 3.41-3.61 s | 140.1 MiB | 275.0 MiB (288.4 MB) |

The RGB fixture has three encoded JPEG channels, derived from the public
synthetic perspective/shadow scene. Its file is about 0.62 MB. Four scans run in
the same instance; linear memory does not grow beyond the stated high-water mark.
Earlier runs on this shared laptop took 3.12-3.62 s cold and 2.49-3.15 s warm.
`WASM-PERFORMANCE.json` in the digitizer crate records the final measured run.
This is below 300 MB for the tested RGB photo, including the Node harness. It is
not a bound on an entire browser process with its camera, decoded preview,
other tabs, photos retained by the UI or a very large encoded file. Release
camera previews and previous photo byte buffers; keep only accepted geometry and
small previews in a multi-photo session. The engine's 20 MB encoded/12 MP decoded
limits, 96 MiB decoder allocation limit, reduced vision image, metric-image cap,
component cap and contour-complexity guard bound its work. Busy texture may be
rejected with a readable error instead of consuming unbounded contour memory.

A **10-25 second estimate** for one comparable 12 MP photo on a mid-range phone
assumes roughly 3-6 times the laptop's cold WASM time, rounded upward. This is
an assumption, not measured phone performance. Several pieces or complex marks
can take longer. Use a Web Worker and show progress/working status. No shared
memory, cross-origin isolation or WASM threads are required. Native benchmark
scans take 0.638-1.160 s on grayscale JPEGs; see the separate accuracy tables.

Raw WASM scan JSON has runtimeMs=0 because std::time::Instant is unavailable on
wasm32-unknown-unknown. The typed adapter fills runtimeMs with performance.now()
around the complete synchronous call, including serialization transfer.

Reproduce after building local WASM assets:

```sh
cargo run --manifest-path pattern/Cargo.toml -p beta-pattern-digitize --example rgb_fixture -- \
  pattern/crates/beta-pattern-digitize/tmp/full/a4-0-perspective-shadow-texture.jpg \
  pattern/crates/beta-pattern-digitize/tmp/full/a4-rgb.jpg
node marker/packages/marker-wasm/scripts/quick-smoke.mjs \
  marker/packages/marker-wasm/pkg \
  pattern/crates/beta-pattern-digitize/tmp/full/a4-rgb.jpg \
  marker/crates/beta-marker-export/tmp/quick
```

The smoke script writes local PDF samples, the overlay and `wasm-timing.json`.
It checks repeated-scan memory and real exported bytes. The A4 200 x 600 mm
layout fixture has seven pages (one map + six tiles); A3 has three. Hindi/Tamil
headings are inspected as rendered paths, rather than trusting text extraction.


## Verification and handoff

The branch includes `marker/mvp` audit merge `1e55f59`; those changes were merged
without conflicts. No CLI, nesting, matching or fold implementation was edited
by this task. The existing export formats remain available. New print paths
validate the job/result, use actual cut geometry, and label copies from 1 for
operators (the underlying JSON copy index stays zero-based).

Final checks on the merged branch:

| Check | Result |
|---|---|
| `cargo test` in `pattern/` workspace | 99 passed, including one doc test: core 63, digitizer 28, pattern WASM 8 |
| `cargo clippy --workspace --all-targets -- -D warnings` in `pattern/` | Passed |
| Marker export tests | 22 passed |
| Marker WASM tests | 5 passed |
| Strict clippy for marker export/WASM, all targets | Passed |
| `@beta/marker-wasm` and `@beta/marker-web` typecheck | Passed |
| Vitest marker-wasm package tests | 12 passed in two files |
| Marker and Pattern release wasm32 builds | Passed, without threads |
| Real compiled WASM, 12 MP RGB JPEG | Scan, merge, duplicate warning, accepted job and all exports passed |
| Synthetic contour benchmark | 18 scans; results and failures against accuracy goals reported separately |
| PDF page metadata | A4: 7 pages at 595.276 x 841.890 pt; A3: 3 pages at 841.890 x 1190.550 pt |
| PDF visual review | Map, A4/A3 tiles, English/Hindi/Tamil guides rendered headlessly with Poppler |

Local inspection images (ignored build artifacts, paths from repository root):

- `marker/crates/beta-marker-export/tmp/quick/a4-1.png` (page map)
- `marker/crates/beta-marker-export/tmp/quick/a4-2.png` (1:1 tile)
- `marker/crates/beta-marker-export/tmp/quick/a3-tile.png`
- `marker/crates/beta-marker-export/tmp/quick/chalk-en.png`
- `marker/crates/beta-marker-export/tmp/quick/chalk-hi.png`
- `marker/crates/beta-marker-export/tmp/quick/chalk-ta.png`
- `marker/crates/beta-marker-export/tmp/quick/scan-overlay.png`

Poppler's installation emits missing Symbol/ArialUnicode fallback-font messages;
these PDFs use Helvetica and explicit Indic glyph paths. The rendered pages were
inspected and do not depend on those missing fallback fonts. No GUI windows were
opened. PDFs are test fixtures, not physical printer or projector validation.

The lead should regenerate the two workspace Cargo.lock files after merging;
this task changes only owned crate manifests and leaves lockfile ownership with
the lead. Build outputs and large synthetic photos are ignored and are not part
of the commits. No push or publication was performed.
