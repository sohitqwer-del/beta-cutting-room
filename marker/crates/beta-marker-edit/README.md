# Editor4 implementation and handoff

Work is on `marker/editor4`, based on `marker/mvp` at `de68343`. Only the Editor source, marker WASM source, this new crate and ignored Editor QA files were changed. Other screens and `transport.ts` were left to their owners.

## Operator tools

| Tool | Keyboard or entry |
| --- | --- |
| Nudge selected group | Arrows; choose 0.5 / 1 / 5 / 10 mm; Shift+Arrows uses 10 mm |
| Bump left / right / up / down | Alt+Arrows; also four named buttons |
| Compact all in x order | C |
| Close the occupied marker end | E |
| Next allowed rotation for a group | R |
| Small tilt within each piece's rule | `[` / `]`, in 0.5 degree steps |
| Rotate group about its common center | Enter an angle and activate Rotate group |
| Flip, when face-down placement is allowed | F |
| Swap exactly two unlocked pieces | S |
| Lock / unlock selected group | L |
| Multi-select | Shift+click, Select all (A), or filter by size, 1-based bundle, piece-name substring |
| Rubber-band selection | V, then drag empty space; fully enclosed copies are selected |
| Group drag / empty-space pan | G, then drag |
| Return selected copies to the unplaced list | Delete |
| Build a marker by hand | Remove all unlocked, then choose unplaced copies |
| Place an unplaced copy | Choose its list button, then click a clear canvas point; or use numeric X/Y and Place armed copy |
| Fill gaps | Fill gaps button; Rust tries free-space candidates and bumps accepted copies left |
| Live overlap and gap display | Exact polygon intersection area in mm² and actual / required clearance per pair |
| Resolve initial overlap or gap violation | Select affected copy/group, choose direction, activate Resolve by bump |
| Per-piece clearance | Apply a buffer to selected piece types; the shared model applies it to every copy of that type |
| Full-width section cut lines | Enter X, then Add / Remove cut line |
| Ruler / measure | M and two canvas clicks; or numeric start/end coordinates; Enter on the canvas runs the numeric measurement |
| Marker end | Drag the orange handle, focus it and use Left/Right, or enter Table end X |
| Finished-marker library | Save with job, named entry, JSON backup / restore, delete and reuse for the current job |
| Undo / redo | Ctrl+Z / Ctrl+Y (also Ctrl+Shift+Z) |
| Fit / zoom / cancel | 0 or Home, + / −, Escape |
| Keyboard help | ? on the canvas, or Keyboard help button |

Every control also supports normal Tab, Shift+Tab, Enter and Space operation. Numeric placement, measurement, selection filters, section entry and marker-end entry provide keyboard alternatives to pointer tools.

The status bar shows occupied length, physical table length, table efficiency, placed / unplaced copies, overlap/gap pairs and shared-validator error count. Occupied length and efficiency retain the shared result contract. Extending the table changes the separate table-efficiency display.

## Rust geometry and saved data

`apply`, `bump`, `inspect`, `unplaced`, `settings` and `reuse` are pure Rust APIs. WASM exposes `edit`, `edit_inspect` and `edit_reuse` as JSON-in / JSON-out functions. The UI's small local adapter keeps the existing public TypeScript package unchanged.

Bump enumerates moving vertex / segment contact events against Euclidean clearance capsules. It checks interval interiors to stop at the first collision, including thin blockers and concave pieces. Bounding boxes only reject distant candidates. Area uses ear triangulation and convex clipping. Calculations use f64, with small numerical guards; these are polygon calculations, not raster or bounding-box collision approximations. Inputs must have simple finite outlines. There is no added geometry package or vendor code.

`apply` and `inspect` use `effective_job` and `cut_outline`, so shrinkage, fusing-block margins and clipped fold outlines use the shared physical boundaries. Direct `bump` callers pass an already-effective job and explicit settings. Locks are protected. Group commands preserve relative positions. Failed commands return errors and retain the accepted document. One group drag or command is one history entry. Display overlap tolerance never changes validation or enables export.

`job.meta.editor_v1` stores:

```json
{
  "sections": [1600],
  "table_length": 1700,
  "removed": []
}
```

`removed` holds full shared `Placement` values to retain handedness, copy, fold and garment identity. Missing-copy accounting uses physical layer counts for open, folded and tubular layouts. The library stores the complete job/result in this browser, under `beta-marker-library-v1`, with named entries and JSON backup/restore. Reuse compares piece ids and the full unchanged piece/style/rule/ratio/width data, then revalidates the saved layout. Display job names, piece ordering and free-form metadata such as import filenames may differ.

No shared model changes are required for the Editor. Sections are hard cut lines: this version does not split pieces or permit a piece to cross a line. A server nest that crosses a saved section or table end is rejected, preserving the current marker. The Editor also checks its metadata before export and library save/reuse.

## Performance evidence

Original synthetic fixture: 300 placed 10 × 10 mm rectangles in ten rows, with 2 mm gaps. One last-column copy is bumped left. These timings are not a factory-pattern benchmark or a commercial-CAD parity claim.

| Measurement | Median | p95 | Maximum |
| --- | ---: | ---: | ---: |
| Native release `bump`, 100 runs | 0.557 ms | 2.370 ms | 14.286 ms |
| Browser JSON/WASM `apply`, 100 runs after 10 warmups | 3.000 ms | 4.200 ms | 4.700 ms |

The browser measurement includes serialization, job checks, geometry, result metrics and output decoding. It excludes drawing and full live validation. The benchmark asserts the expected contact coordinate as well as the time. The native ignored test asserts p95 below 20 ms. `editor-smoke.mjs` contains the browser benchmark and repeatable UI smoke.

## Verification and evidence

Commands use the assigned external `CARGO_TARGET_DIR`, `CARGO_BUILD_JOBS=3`, npm cache and matching `WASM_BINDGEN`. No machine-specific environment values are committed.

```text
npm install --cache <assigned npm cache>
cargo test --manifest-path marker/crates/beta-marker-edit/Cargo.toml
cargo clippy --manifest-path marker/crates/beta-marker-edit/Cargo.toml --all-targets -- -D warnings
cargo clippy --release --manifest-path marker/crates/beta-marker-edit/Cargo.toml --all-targets -- -D warnings
cargo test --release --manifest-path marker/crates/beta-marker-edit/Cargo.toml benchmark_bump_300 -- --ignored --nocapture
cargo test --release --manifest-path marker/crates/beta-marker-edit/Cargo.toml -- --include-ignored --nocapture
npm run build:wasm -w @beta/marker-wasm
npm run typecheck -w @beta/marker-wasm -w @beta/marker-web
npx vitest run marker
npm run build -w @beta/marker-web
cargo test --manifest-path marker/Cargo.toml --workspace --no-fail-fast
cargo clippy --manifest-path marker/Cargo.toml --workspace --all-targets -- -D warnings
git diff --check
```

Vitest: **34 passed** in six files, including six Editor state tests. Both TypeScript workspaces typecheck; marker WASM and the production web build pass. The final standalone release Rust suite passes **20 tests**, including the performance assertion. Standalone debug and release Clippy and full marker workspace Clippy (`--all-targets -- -D warnings`) pass with zero warnings. These tests cover each command plus malformed inputs, self-intersections, effective cut geometry, diagonal clearance, physical fold counts, size bundles and repeated removal/placement identity. A separate WASM JSON binding test checks the adapter boundary.

Final marker workspace: **251 passed, 0 failed, 5 ignored**, including **23 passed / 1 ignored** in marker WASM. The standalone debug edit crate also passes **19 tests / 1 ignored**. The ignored editing benchmark is included in the successful 20-test release run. `git diff --check` passes. The historical failures in `UI3_REPORT.md` do not reproduce on this branch's MVP base.

The hidden collaborative-browser smoke passed group selection/nudge/bump, remove/undo/redo, numeric placement with auto-bump, fill gaps, a 3–4–5 measurement, table end, sections, library save/reuse, a 24,000 mm² overlap display and recovery. Native browser drag also selected all eight demo copies with a rubber band, moved the group, and restored it with one undo. The end handle was dragged to about 1.700 m, then moved by a keyboard step. A 390 × 844 viewport had document scroll width 390 px. No repeated visible browser windows were opened.

Ignored evidence paths, relative to the worktree:

```text
marker/apps/web/.qa/screenshots/editor4-tools.png
marker/apps/web/.qa/screenshots/editor4-shortcuts.png
marker/apps/web/.qa/screenshots/editor4-mobile.png
marker/apps/web/.qa/editor-smoke-results.json
marker/apps/web/.qa/editor-workspace-tests.txt
marker/apps/web/.qa/editor-workspace-clippy.txt
```

## Lead workspace changes

Cargo automatically pulls path dependencies into a workspace, and rejects the edit crate's temporary `[workspace]` when it is used as a normal dependency of the main workspace. To keep the lead-owned manifest untouched and keep builds working, the WASM crate temporarily compiles the exact edit source through a `#[path]` module. There is no second implementation.

When registering the crate, make these changes together:

1. Add `"crates/beta-marker-edit",` to `marker/Cargo.toml` workspace members.
2. Remove the temporary `[workspace]` table in `crates/beta-marker-edit/Cargo.toml`.
3. Add `beta-marker-edit = { path = "../beta-marker-edit" }` to the WASM crate's dependencies.
4. Remove the commented `#[path = "../../beta-marker-edit/src/lib.rs"] mod beta_marker_edit;` block from WASM `src/lib.rs`.
5. Regenerate lockfiles. No lockfile or generated output belongs in this branch's commit.

New local crate: `beta-marker-edit`, Apache-2.0. External direct dependencies reuse `serde` and `serde_json` (MIT OR Apache-2.0) and `beta-marker-core` (Apache-2.0). No new external crate family or npm package was added. No proprietary or private samples were used.

## Limits and integration checks

- Fill gaps is a deterministic, bounded placement aid. It tries edge-derived candidate points in the remembered/default orientation, up to 2,000 attempts per click. It does not prove optimality or find every possible concave free space. Remaining pieces stay in the list.
- Compact repeats left bumps in x order until stable, bounded by the placed-copy count. Close the end targets unlocked copies touching the occupied rightmost edge.
- The manual editor limits required physical copies to 10,000 and the browser library to 50 markers. Large or detailed geometries still need factory-scale performance testing. Measurements above use the stated synthetic fixture.
- The full live validator and SVG redraw are outside the bump benchmark. They still run after each accepted edit.
- The library is local to this browser. JSON backup/restore is provided; it is not a shared server catalog.
- Section/table metadata is currently enforced by the Editor Rust API and Editor export gate. The shared server, CLI, nesting and other screens do not yet understand it. Teach those entry points to call `beta_marker_edit::inspect` after workspace integration. No piece-splitting contract exists, so crossing is always rejected here.
- Recheck automatic nesting, folded/on-fold placement, garment bundles, matching, shrinkage, section/export gates and marker reuse after merging geometry/desktop work. The browser smoke used WASM offline; physical cutter/table calibration and full commercial-CAD parity were not tested.
