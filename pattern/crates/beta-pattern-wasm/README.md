# BETA Pattern WASM

Small JSON bindings for the offline drafting editor. The core owns construction,
formula, allowance, grading, import and print geometry. This crate adapts those
results for browser forms and canvas drawing. There is no runtime network use.

Build from the repository root with the assigned pattern `CARGO_TARGET_DIR`,
`CARGO_BUILD_JOBS` and matching `WASM_BINDGEN`:

```text
npm run build:pattern-wasm -w @beta/marker-wasm
```

Generated bindings remain in the ignored Marker WASM package directory. The Vite
app serves them as local assets. `marker/packages/marker-wasm/src/pattern.ts`
provides the typed wrapper.

## Drafting API

Document arguments are native `beta-pattern.doc/v1` JSON strings. Invalid input
returns an error; edits return new documents and never mutate the input.

| Export | Purpose |
|---|---|
| `catalog()` | Serialize every typed construction form from `tools::catalog()`. |
| `inspect(document,size)` | Evaluate drawing objects and return inline piece checks and edge lengths. Uses the shared core evaluator. |
| `check_formula(document,size,expression)` | Check a formula without accepting an edit. |
| `apply_tool(document,request)` | Apply `{tool,id,parameters,selection?}`. Optional selection applies a parametric group transform with `id` as suffix. |
| `move_point(document,id,size,target)` | Move an absolute or polar point through the engine. |
| `snap(document,size,request)` | Point/path/grid snap from `{at,radius,grid,points,paths,ignore?}`. Ignore excludes a dragged point and dependent geometry. |
| `image_scale(a,b,distance_mm)` | Millimetres per image pixel from two distinct positions. |
| `piece_action(document,request)` | Dart, spread, pleat, split, mirror, walk or true-seam menu command; retains source pieces. |
| `grade(document)` | Grade all sizes. Also reject invalid unused constructions/variables. |
| `nest(document,reference)` | Align the graded sizes at a named reference point. |
| `starter()` | Original teaching bodice. Fit-test before production. |

`inspect` returns `{objects,pieces}`. Drawing entries are
`{kind:"point"|"path",points:[[x,y],...]}`. Piece entries have stable `id`, `ok`,
an optional error, successful graded geometry and directed edge lengths. Invalid
construction/formula edits fail. Unfinished piece errors are returned inline.

Piece commands use `action`, `piece`, `size`, and an output `id`, plus labelled
point/path IDs and numeric/formula fields. Dart create/transfer/close records its
operation data in document metadata. Topology changes create fixed outlines for
the chosen size; source geometry remains available. Seam truing adds a separate
open path. The UI must show the review message before a maker uses that result.

## Files and marker handoff

- `import_seamly(pattern,measurements?,size)` uses the supported native XML reader.
- `import_measurements(document,source,format,size)` accepts native long CSV or
  Seamly measurement XML. Individual rows merge into the matching size; multisize
  imports use the engine's size order and base size.
- `export_measurements(document)` writes the engine's long CSV format.
- `export_dxf(document)` exports all sizes and the public BETA grade-rule extension.
- `export_pdf(document,paper)` wraps `print::tiled_pdf` for A4/A3 with 10 mm overlap
  and a 100 mm calibration page. It returns bytes.
- `export_plt(document)` wraps `print::plotter_plt`, at 40 units/mm.
- `to_job(document,fabric,ratio)` creates a Marker job.
- `from_job(job)` accepts a single-size digitizer job as fixed editable geometry.

The pure `editor`, `operations` and `output` adapters are also included by the
loopback server's pattern routes. They depend only on existing local core crates
and JSON support. The server does not load wasm-bindgen or a browser runtime.

## Checks

From the pattern workspace, with the assigned external target directory:

```text
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Six adapter tests cover JSON drafting/grade/export/reopen, measurement changes,
CSV round trips, formula failures, snapping, image calibration, every piece-menu
operation, immutable sources, seam-truing length and physical output wrappers.
The web state tests and real-WASM browser smoke are documented in
`marker/apps/web/PATTERN_UI_REPORT.md`.

No new crate or npm dependency was added. Local BETA crates are Apache-2.0;
existing serde/serde_json and wasm-bindgen are MIT OR Apache-2.0.
