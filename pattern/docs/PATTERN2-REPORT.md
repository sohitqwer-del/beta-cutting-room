# Pattern production tools: implementation report

Date: 20 October 2026. Branch: `marker/pattern2`, based on `0d7b298`.
The commit hash is supplied in the handoff. No push or publication was performed.
Changes are confined to `pattern/` and Pattern files in `marker/apps/web/src/`.
The shared workspace manifest, website comparison, logo and other screens were
not changed.

## Before and after

The [public-source parity table](PARITY.md) was written before implementation.
It covers Gerber AccuMark, Lectra Modaris, TUKAcad, Optitex PDS, CLO and Seamly
feature documentation. Supplier material supplied feature names only.

| Daily task | Before | After |
| --- | --- | --- |
| Check seams after factory grading | Draft-path lengths and a single-size difference | Actual graded stitch-edge runs, all sizes, required ease, tolerance, aligned preview, CSV and JSON report |
| Reuse approved grade increments | Library functions without file controls | JSON/RUL export, preview, compatible-size validation, explicit replacement and all-size geometry validation |
| Mirror a production piece | Fixed outline; lost marks/grain; unequal allowances not retained | Linked construction geometry with copied/reflected rules, marks, paths, grain, labels, anchors, allowances and quantities |
| Prepare a symmetric piece from a half | No direct half-to-full operation | Measurement-graded unfold with a full-piece fold axis, mirrored details and bounded geometry checks |
| Mark a placket | Individual marks | Parametric button, buttonhole and drill rows; buttonholes have internal end bars |
| Export a full-piece fold guide | Axis reached Marker, but was absent from pattern print/DXF output | DXF internal guide, dashed PDF guide and PLT guide |

An original native regression starts with a 100 mm draft edge. Its factory
rules produce **90 / 100 / 120 mm** across three sizes. The old inspector used
the draft path. The new inspector and seam report use those actual graded
lengths. This is a correctness check, not a production-time or fabric-saving
claim.

The browser fixture deliberately separates body measurements from factory
rules. Its draft widths are 140 / 150 / 160 mm; the graded widths are
**135 / 150 / 175 mm**. Compared with a 145 mm seam, required ease of 5 mm and
1 mm tolerance, the report gives **CHECK / PASS / CHECK**, with errors
**−15 / 0 / +25 mm**.

Five buttonholes become ten on an unfolded piece. Three sizes with garment
ratio **S=1, M=2, L=1** reach Marker with marks, fold axes, material, grain and
mirrored quantities intact.

## Implementation

- `beta-pattern-core::production`: graded seam reports, CSV, aligned preview
  geometry, and parametric production-mark rows.
- `beta-pattern-core::rule_exchange`: library JSON and the existing marker
  reader's bounded ASCII RUL subset. Mixed rule modes export as cumulative
  displacement. Imported numeric IDs remain editable.
- `beta-pattern-core::symmetry`: complete-piece reflection and straight-edge
  unfolding. A crossing guide is restricted to its own half so it does not
  create duplicate notch paths. On-fold marks/guides are kept once.
- `GradedPiece.seam_edges`: additive evaluated output for actual stitch lengths.
  The piece inspector now reports these lengths after factory grading.
- Crossing-notch annotations have an optional edge filter, defaulting to all
  edges. Existing JSON remains readable.
- `beta-pattern-wasm` uses the existing `piece_action` transport. The native
  server includes these same adapters, so no separate server route or Marker
  WASM contract change was needed.
- `pattern-production.ts` adds Seam checks, rule-file controls and mark-row
  controls. The existing Piece menu exposes linked mirror/unfold.

The existing browser trace smoke test used exact floating-point equality for
the image scale. Screen-coordinate conversion returned 1.99999977 or 2.00000008
for the expected scale 2. The assertion now allows `1e-5`; calibration code was
not changed.

No new third-party crate was added. The Pattern WASM adapter now directly uses
the workspace's existing `anyhow` crate, licensed **MIT OR Apache-2.0**.
Lockfiles, generated WASM, build output and npm dependencies are not committed.

## Executed checks

All build commands used the assigned target/cache directories and
`CARGO_BUILD_JOBS=3`. Only original synthetic and existing public test data were
used.

| Command / check | Result |
| --- | --- |
| `cd pattern; cargo test --workspace` | **80 passed**, including one doctest; zero failures or ignored tests |
| `cd pattern; cargo clippy --workspace --all-targets -- -D warnings` | Passed, zero warnings |
| `cd marker; cargo test -p beta-marker-dxf -p beta-marker-wasm` | **39 passed**: 35 DXF and 4 WASM |
| `cd marker; cargo test -p beta-marker-cli pattern` | **8 passed**, 51 unrelated tests filtered out |
| `npm test -- --maxWorkers=3` | **254 passed** across 24 files |
| `npm run typecheck -w @beta/marker-web` | Passed |
| `npm run build:pattern-wasm -w @beta/marker-wasm` | Passed with the assigned wasm-bindgen tool |
| `npm run build:wasm -w @beta/marker-wasm` | Passed |
| `npm run build -w @beta/marker-web` | Passed, including TypeScript checking |
| Hidden browser: 5 new production routines | Passed: seam reports, rule preview/export, rule import/edit/rejection, marks/unfold, Marker handoff |
| Hidden browser: 4 existing Pattern routines | Passed: drafting, edit/undo/redo/exports, tracing/groups, pattern switching/nest refresh |
| Narrow viewport | 390 px viewport and 390 px page width; tables scroll inside their container |
| New production mark form labels | Every input/select has a label |
| `git diff --check` | Passed |

There are **18 new Rust tests** and **2 new Vitest tests**. Coverage includes
curved and chained seams, lower/upper grades, diagonal reflection, invalid axes,
fold allowances, ID collisions, duplicate on-fold marks, crossing guides,
library collisions, geometry collapse, repeated marks, and DXF/Marker handoff.
Tests verify reflected quantities and unequal allowances as well as outlines.

The DXF regression imports BETA's exported bytes with `beta-marker-dxf` and
checks geometry, piece information, notch/drill/internal-line counts and mirror
quantities. RUL/JSON tests round-trip movements on both sides of the base size.
They do not certify other suppliers' native dialects.

To repeat the browser checks on a local dev build, import
`/src/pattern-production-smoke.mjs` and run, in order:
`runProductionSeams`, `runProductionLibraryPreview`, `applyProductionLibrary`,
`runProductionMarksAndFold`, then `runProductionHandoff`.
The last routine expects Piece mode. The existing routines remain in
`/src/pattern-smoke.mjs`. These smoke modules use real form handlers and original
synthetic files; they are not part of the production bundle.

## Screenshots

Six PNGs are saved under `$BETA_DATA_DIR/pattern2-shots/`:

1. `01-seam-report.png` — graded lengths, ease errors and aligned paths.
2. `02-rule-library-preview.png` — imported incremental RUL normalized for review.
3. `03-unfold-and-buttonholes.png` — symmetric piece, fold axis and mirrored marks.
4. `04-grade-nest.png` — three pieces across three sizes, nine outlines.
5. `05-buttonhole-row.png` — repeated production-mark controls.
6. `06-mobile-seam-report.png` — narrow viewport with a contained scrolling table.

The browser remained hidden. No native application windows were opened.

## Remaining work and lead notes

Keep the website's **partial parity** assessment. The broader daily CAD
workflow is still incomplete. See [the operator guide](PRODUCTION-TOOLS.md)
for exact limits.

- Unfold currently requires measurement grading. Factory-rule half patterns
  are rejected. Geometry stays linked; annotation membership and reflected
  grade-rule values are copied when the operation runs.
- RUL is a bounded existing ASCII subset. No general vendor dialect support
  or external DXF/RUL point-attachment reconstruction is claimed.
- Full-piece fold axes export as labelled internal DXF guides. BETA's importer
  retains the guide but does not restore the `fold_line` cutting constraint.
  Direct Pattern-to-Marker handoff and pattern JSON preserve it.
- General allowance cut-edge notch projection, reverse/fold-back corner tools,
  fold-in darts, dedicated tucks, associative dart/pleat/slash edits, automatic
  truing, rolling seam walk and finished-garment measurement solving remain.
- Physical sample fit, plotter/cutter output and commercial CAD interoperability
  were not tested on production equipment.

No required workspace or Marker `Piece` contract change is pending. A useful
follow-up for the DXF owner is a full-piece fold-axis metadata round trip.
Source templates must have both normal and mirrored quantities set to zero
when they are retained only for construction.
