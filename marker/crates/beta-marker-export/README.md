# Marker exports

`to_svg`, `to_hpgl`, `to_pdf` and `to_report` use the job geometry and recompute
length and efficiency. They do not trust the result's stored metrics. Unknown
piece ids or non-finite drawing coordinates fail the vector exports. Run the
independent validator before sending output to a cutter.

- SVG uses millimetres and a flipped drawing group for the model's upward y axis.
  Text stays upright. Sizes have stable colours. The fabric rectangle excludes
  the title area.
- HPGL uses absolute integer coordinates at 40 units/mm. Outlines use pen 1;
  marks and labels use pen 2. Labels use `DT` with ETX as the terminator and an
  explicit `SI` character size. `rotate` turns the whole marker 90 degrees so
  its length follows plotter y. Drill rings use 32 segments, with less than
  0.004 mm radial error before plotter rounding (up to 0.0125 mm per axis).
- PDF is vector PDF 1.4 with a byte-accurate classic cross-reference table.
  One millimetre is exactly `72 / 25.4` points. Print at **actual size / 100%**.
  Pages have 5 mm margins, alignment crosses and page numbers. `pdf_tile_mm`
  sets the maximum page side including margins, and must exceed 10 mm.
  Both axes tile automatically above 5080 mm (200 inches), even without an
  explicit tile setting. Content is clipped at matching tile boundaries;
  neighbouring sheets align at the crosses. Margins are not part of the fabric.
- Reports include placed piece counts by size and average fabric metres per
  garment in the order. Without an order, one garment per distinct size is
  assumed; unsized pieces form one garment group.

Pattern notches use a 3 mm tick in their direction. A notch with only an anchor
uses the nearest outline edge's inward normal. Drills have a 0.75 mm radius.
Marks receive the same mirror, rotation and translation as their piece.
Piece labels include the name, size, zero-based copy number and mirror flag.

SVG preserves Unicode. HPGL and PDF base Helvetica labels replace characters
outside printable ASCII with `?`; control characters become spaces. Embedding
Unicode fonts and testing a physical pen plotter are outside this implementation.
The v0.1 formats add no dependencies. Version 0.2 workbook dependencies are listed below.

From the marker workspace, with `CARGO_TARGET_DIR` set to the build directory:

```text
cargo test --workspace
cargo clippy --workspace --all-targets
cargo test -p beta-marker-core --release validation_speed -- --ignored --nocapture
```

The ignored speed test validates 1,000 placed 100-vertex pieces. It requires
less than one second when built in release mode. Unit tests also parse HPGL
back to coordinates and check a 100 mm square, parse PDF xref offsets, stream
lengths and page counts, and check the calibration square's point coordinates.

The release speed check on 18 October 2026 measured **405.155 ms** (final run; first run: 438.940 ms) for
1,000 pieces with 100 vertices each (Windows x86-64, concurrent Rust builds
running on the same machine). The test includes recomputing metrics, demands,
rotation and edge checks, broad-phase filtering and exact polygon checks.
This is one synthetic valid layout, not a promise for arbitrary invalid layouts
with hundreds of thousands of overlapping pairs to report.

## Version 0.2 output

All formats apply `rules::effective_job` once. Fusing blocks use
`beta_marker_core::validate::cut_outline`: the grain-aligned rectangle plus the
margin, transformed with the placement. Length includes the outer block edge;
efficiency uses the actual piece area, excluding block margins. Fold placements
show the retained half and a FOLD marker. Marks on the discarded half are omitted;
internal and grain lines are clipped to the retained half.

SVG and PDF show hatched defects, light repeat grids and shade boundaries. HPGL
puts these guides on pen 3; configure a light pen for that group. Cut boundaries
remain on pen 1 and pattern marks on pen 2. These plotting guides are not emitted
as machine operations by the ISO writer. Reports list layout, rule settings,
bundles, layer-copy counts and block margins. Bundle and copy numbers are zero-based.

### Cutter program (`to_iso`)

`to_iso(job, result, &IsoProfile)` emits a generic ISO 6983-style program: G21 mm,
G90 absolute coordinates, G00 knife-up travel and G01 strokes. Drills, notches
and internal marks are processed before all outlines. Each stage uses a greedy
nearest-neighbour order. Closed outlines have counter-clockwise winding after
mirroring, start at a corner and omit redundant collinear vertices. The edge on
a fold is left uncut. There is no lead-in, kerf compensation, tangential-knife
angle control or controller-specific Z motion.

Each cutter needs a factory-verified post-processor profile. The defaults are
illustrative commands, not a certified machine dialect: M05 knife up, M03 knife
down, M70 drill, M71 notch, M30 end. M70/M71 mean a controller macro at the current
XY point; direction-dependent notch cutting needs a factory post-processor.
Verify the commands and dry-run the program before driving a physical machine.
We do not copy or claim compatibility with any vendor dialect.

`IsoProfile` is JSON-loadable, accepts omitted fields as defaults, and rejects
unknown fields. Command strings must be single-line printable ASCII. Feed rates
are optional positive mm/min. Preamble runs before G21/G90. Postamble runs after
final knife-up. Example (adapt the M-codes to the controller):

```json
{
  "knife_up": "M05",
  "knife_down": "M03",
  "drill": "M70",
  "notch": "M71",
  "cut_feed_mm_min": 800,
  "mark_feed_mm_min": 200,
  "preamble": [],
  "postamble": ["M30"]
}
```

### DXF (`to_dxf`)

The whole marker is exploded R12 geometry (AC1009), in mm: layer 1 cut,
4 notches, 13 drills (POINT), 8 internal lines, 7 grain, 15 labels and 0 guides.
It carries millimetre unit metadata and AAMA-style layers. Each placed piece is
one closed POLYLINE. Labels carry name, size, bundle and copy. The fabric boundary
is four LINEs, so it does not become an extra garment on generic re-import.
The round-trip test calls `beta_marker_dxf::import_dxf` and verifies piece count,
area and marker coordinates. Generic DXF import preserves outlines; it does not
restore the original marker job, bundles or style rules from labels.

### Workbook (`to_xlsx`)

Uses `rust_xlsxwriter`, MIT OR Apache-2.0. Sheet 1 has recomputed marker dimensions,
efficiency, garment count, consumption and rules. Sheet 2 groups physical
placements and normal/mirrored layer copies by size, bundle and piece, and lists
block margins. Sheet 3 is optional: pass `Some(&CutPlanTable { headers, rows })`
to attach a generic cut-plan table. The exporter has no dependency on the planner's
current stub implementation. User text is written as literal strings, not formulas.
The CLI exports the first two sheets. Planner integration can pass its lay table
through the library API after merge.

### A4 PDFs

`to_mini_marker_pdf` creates one A4 portrait overview, with a summary table,
scaled vector marker, scale ratio and guide legend. It is not an actual-size pattern.
`to_labels_pdf` creates one label per physical placement (not one per layer), with
style, size, bundle, name and copy. `LabelOptions` defaults to a 3 by 8 A4 sticker
grid, 10 mm margins and 2 mm gutters. Adapt the grid to the sticker stock. Labels
wrap text at readable size and return an error when it cannot fit. PDF uses base
Helvetica: unsupported Unicode becomes `?`, as in the existing full-size PDF.

### CLI

```text
beta-marker export job.json result.json --format iso --profile profile.json -o marker.nc
beta-marker export job.json result.json --format dxf -o marker.dxf
beta-marker export job.json result.json --format xlsx -o cutting.xlsx
beta-marker export job.json result.json --format labels -o labels.pdf
beta-marker export job.json result.json --format mini -o mini.pdf
```

Existing `svg`, `plt`, `pdf` and `report` formats remain available. `--profile` is
accepted only with `--format iso`.

### Added dependencies and licences

Direct runtime dependency: rust_xlsxwriter 0.95.0 (MIT OR Apache-2.0).
Direct test dependencies: beta-marker-dxf (workspace Apache-2.0), zip 7.4.0 (MIT).
New dependency graph packages, checked from Cargo metadata:

- crc32fast 1.5.2, equivalent 1.0.3, flate2 1.1.10, hashbrown 0.17.1,
  indexmap 2.14.2, typed-path 0.12.3: MIT OR Apache-2.0.
- simd-adler32 0.3.10: MIT.
- zlib-rs 0.6.8: Zlib.
- zopfli 0.8.4: Apache-2.0.

Core validation adds no dependencies. Rust XLSX writer licence and buffer API:
https://github.com/jmcnamara/rust_xlsxwriter and
https://docs.rs/rust_xlsxwriter/0.95.0/rust_xlsxwriter/struct.Workbook.html.

## Validator integration notes

The validator independently checks v0.2 features. Layer counts use
`rules::layer_copies`; faceless counts pool normal and mirrored copies. Open-layout
surplus is still `extra_piece`; folded/tubular surplus is `surplus_copy` warning.
Within a bundle, folded surplus is limited to the unavoidable second layer; it
cannot hide a second full garment in the same bundle. Each bundle must otherwise
supply every piece of one size and the number of bundles must match the size ratio.
Missing bundle ids are errors when shade, bundle nap or matching needs bundles.

Matching checks every placed copy of each named endpoint within a bundle, per
requested axis, with circular distance to the nearest repeat. The sign is
`a - b - rule.offset`, or `a - repeat.offset - rule.offset`. Tolerance defaults
to 1 mm when zero. Rules with both endpoints belonging to a different size are
skipped in that bundle. A selected axis without a repeat period fails `match`.
Direction classes use rotation minus the mirrored grain-alignment angle.

Fold clearance is waived on physical fold edges for every piece, but pieces may
never extend beyond the fabric boundary. An on-fold placement additionally needs
a non-degenerate fold line on one allowed edge, within 0.01 mm. Defect, overlap,
gap, width and maximum length checks use the effective cut boundary, including
the physical block rectangle when present. Margin is in millimetres after
shrinkage, as specified by the contract.

Shared-contract follow-up requested from the lead: move the public helper
`validate::cut_outline` to a shared geometry/rules location when convenient, and
use it for block-aware marker length in `compute_metrics` (retain actual outline
area for efficiency). This branch keeps the helper in the owned validator files
and uses it from exports. No model or rules files were changed. After merging,
re-test nest2's block origin/rotation and fold clipping, match's bundle allocation
and phase conventions, plan's garment/ply table adapter, and dxf2's generic import.

## Version 0.2 verification (19 October 2026)

Run from `marker/`, with `CARGO_TARGET_DIR` set to an external build directory
and `CARGO_BUILD_JOBS=3` for every Cargo command:

```text
cargo test --workspace
cargo clippy --workspace --all-targets
cargo test -p beta-marker-core --release validation_speed -- --ignored --nocapture
cargo build -p beta-marker-cli
cargo run -p beta-marker-cli -- validate crates/beta-marker-export/tests/fixtures/v02-job.json crates/beta-marker-export/tests/fixtures/v02-result.json --json
```

Results: **89 tests passed**, two intentionally ignored (the release speed check
and the long nesting quality run). Clippy completed with **zero warnings** across
the workspace and all targets. The separate release speed check passed at
**340.3363 ms for 1,000 pieces with 100 vertices each**. This is one synthetic valid
layout; dense invalid overlaps can require many more pairs and issue messages.

The hand-made `v02-job.json` / `v02-result.json` fixture combines tubular fabric,
a fusing block, an on-fold piece, shrinkage, a defect, shade bands, bundle nap and
repeat matching. The CLI validator returns no errors or warnings: length
**186 mm**, piece area **5,720 mm2**, efficiency **19.22043%**. Consumption is
**0.186 m per garment**. Each of the nine CLI formats exported successfully; ISO
also loaded a JSON profile with custom feed rates. Use the export commands above
with these fixture paths to repeat the format checks.

The DXF unit test re-imports exploded marker geometry as generic pieces and
checks count, area and coordinates. Workbook tests read the saved ZIP/XML and
check summary numbers, bundle counts, literal text and the optional cut-plan
sheet. PDF tests check xref offsets, stream sizes, A4 dimensions and pagination.
Poppler `pdfinfo` confirmed single-page A4 for the combined fixture's mini-marker
and labels; `pdftoppm -f 1 -singlefile -scale-to 1300 -png` rendered both, and the
images were visually checked. Physical cutters, plotters and sticker stock have
not been calibrated. Base-font PDF/PLT Unicode replacement and the generic ISO
profile limits described above remain intentional.

No TypeScript files changed. No other agent branches were merged. `Cargo.lock`
is excluded from the commit so the lead can regenerate it after integration.

## Unified byte export for servers

`export(format, &job, &result, &ExportOptions)` returns
`Result<(Vec<u8>, &'static str, &'static str)>`: bytes, HTTP MIME type, filename
extension. Formats are `svg`, `plt` (`hpgl` alias), `pdf`, `report`, `iso`, `dxf`,
`xlsx`, `labels`, and `mini`, case-insensitive. Unsupported formats return an
error. Report extension is `txt`; labels and mini-marker extension is `pdf`.
ISO uses the default `IsoProfile`; labels use the default grid. The existing
`to_*` functions accept custom ISO profiles, label grids and workbook tables.
Validate before releasing cutter files. Export is a formatter, not production
approval. `to_tables_xlsx` writes standalone literal report tables for an order,
without using one marker's summary as the order total.

All formats now share `rules::cut_outline` through the geometry scene and core
metrics. Marker length includes grain-aligned fusing block margins. Efficiency
keeps actual pattern outline area. The Excel summary lists effective per-piece
flip permissions, including overrides.
