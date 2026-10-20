# BETA Marker specification (version 0.1)

BETA Marker is the marker planner of Project BETA. It nests garment pattern pieces on the fabric width to use the least fabric length, with garment rules (grain, nap, left/right pairs, size ratio, gaps). It aims to match commercial marker software (Tukatech TUKAmark/SMARTmark, Gerber AccuMark/AccuNest, Lectra Diamino, Optitex) on cuttable, rule-correct markers.

Code licence: Apache-2.0. Open data: CC BY 4.0. The plan and comparison are in [docs/PLAN.md](docs/PLAN.md).

## 1. Hard rules (every contributor, every agent)

1. **Rust for everything that computes.** Import, nesting, validation, export, the HTTP server and the benchmark runner are Rust. The browser UI is TypeScript, and it calls Rust (WebAssembly or the local server) for geometry. The only non-Rust code allowed outside the UI: the one-time Lectra dataset converter (`bench/tools/`, Python), because the dataset is stored as pandas pickle files.
2. **Permissive licences only:** MIT, Apache-2.0, BSD, ISC, Zlib, Unicode. No GPL, AGPL, LGPL or "non-commercial". The single exception is **jagua-rs (MPL-2.0)**, used unmodified as a crates.io dependency of sparrow. Never vendor, copy or patch its source in this repository.
3. **No proprietary formats or code.** Never copy file-format specifications, code, or sample files from Tukatech, Gerber, Lectra, Optitex or other vendors. Use public standards (AAMA / ASTM D6673 DXF conventions, HPGL) and our own test files.
4. **User data is private by default.** Factory patterns and markers go in `marker/private-patterns/` (git-ignored) or outside the repository. Never commit or publish them without written permission.
5. **No machine-specific paths in git.** Large files go outside the repository. Pass paths as arguments or environment variables (see section 7).
6. **No network at runtime.** The app and the CLI work offline. The server binds to `127.0.0.1` only.
7. Never touch files outside `marker/` in the Project BETA repository. Never commit `package-lock.json`.

## 2. Architecture

```text
 DXF-AAMA / ASTM file                         browser (TypeScript UI)
        |                                       |   view, drag, rotate, lock
        v                                       |   live checks via WASM
 [beta-marker-dxf] -- import --> Job JSON <-----+
                                   |            |  HTTP 127.0.0.1 (beta-marker serve)
                                   v            |
                       [beta-marker-nest]  garment rules -> sparrow / jagua-rs
                                   |
                                   v
                           MarkerResult JSON
                                   |
              +--------------------+--------------------+
              v                                         v
 [beta-marker-core::validate]                  [beta-marker-export]
   independent checks                            SVG, PLT (HPGL), PDF 1:1
              |
              v
       [beta-marker bench]  academic instances + Lectra dataset sample
```

| Path | Contents | Owner |
|---|---|---|
| `Cargo.toml`, `.cargo/`, `SPEC.md`, `README.md`, `docs/PLAN.md` | Workspace and contract | Lead |
| `crates/beta-marker-core/src/{model,geometry,rules,metrics,lib}.rs` | Data contract, rules, metrics | Lead (request changes in your report) |
| `crates/beta-marker-core/src/validate.rs`, `crates/beta-marker-export/`, `cli/src/cmd/{validate,export}.rs` | Validator, SVG/PLT/PDF output | Agent `qa-export` |
| `crates/beta-marker-dxf/`, `testdata/dxf/`, `cli/src/cmd/import.rs` | DXF-AAMA/ASTM import and export, test patterns | Agent `dxf` |
| `crates/beta-marker-nest/`, `examples/`, `cli/src/cmd/nest.rs` | Garment nesting on sparrow | Agent `nest` |
| `crates/beta-marker-wasm/`, `packages/marker-wasm/`, `apps/web/`, `cli/src/cmd/serve.rs` | Web app, WASM bindings, local server | Agent `web` |
| `bench/`, `cli/src/cmd/bench.rs`, `docs/benchmarks.md` | Benchmarks | Agent `bench` |
| `crates/beta-marker-match/` | Stripe/check matching, shade bands | Agent `match` |
| `crates/beta-marker-plan/`, `cli/src/cmd/{plan,batch}.rs` | Cut order planning, batch markers | Agent `plan` |

`crates/beta-marker-cli/src/main.rs` and `cmd/mod.rs` belong to the lead. Each agent may add dependencies to its own crate's `Cargo.toml`. An agent may add lines to `crates/beta-marker-cli/Cargo.toml` `[dependencies]` only for its own subcommand. Do not edit the workspace `Cargo.toml`; ask in your report.

## 3. Data contract

The Rust types in `crates/beta-marker-core/src/model.rs` are the contract. JSON uses the same snake_case names. TypeScript mirrors them in `packages/marker-wasm/src/types.ts`.

- **Units:** millimetres. Angles: degrees, counter-clockwise, y axis up.
- **Marker axes:** `x` along the fabric length (warp = grain), `y` across the width. Usable area: `y` from 0 to `fabric.width`, `x` from 0.
- **Placement transform:** `p' = R(rotation_deg) * M(p) + translate`. `M(x, y) = (-x, y)` when `mirrored`. The rotation includes the grain alignment.
- **Length:** the largest `x` of any placed outline point. **Efficiency** = sum of outline areas / (width x length), a fraction. Gaps and buffers do not count as piece area.
- **Copies to cut:** for each piece, `quantity x m` normal copies and `mirror_quantity x m` mirrored copies. `m` is `order.sizes[piece.size]`, or 1 when there is no order. A piece without a size, or with a size not in the order, gets 0. See `rules::demands`.
- **Rotations:** `rules::allowed_rotations`. The grain line is first turned to `+x`. Then the rule adds steps: `fixed` {0}, `flip180` {0, 180}, `quarter` {0, 90, 180, 270}, `free` any. `inherit` means `flip180` on `nap: none` and `fixed` on `nap: one_way`. One-way fabric removes 180 and 270. `fabric.tilt_deg` allows a small deviation.
- **Mirroring:** a mirrored placement is allowed only for the mirrored copies (`mirror_quantity`). `fabric.allow_flip` is recorded but not used in version 0.1.
- **Gaps:** two pieces must be at least `spacing.piece_gap + buffer_a + buffer_b` apart. Every piece must be at least `spacing.edge_gap + buffer` from `y = 0`, `y = width` and `x = 0`.

Example job: `examples/jobs/*.json` (agent `nest` creates them).

## 4. Nesting (agent `nest`)

- `beta_marker_nest::nest(&Job, &NestOptions) -> MarkerResult`, on sparrow 0.3.0 (MIT) and jagua-rs 1.0.0 (MPL-2.0).
- Convert each demand (piece, mirrored) into one jagua-rs item with its count. Pre-transform the shape (mirror, then grain alignment) so the engine only sees the allowed rotation steps. Tilt: add discrete angles inside the tilt range (for example steps of 0.5 degrees), or report that tilt is not used.
- Gaps: inflate each shape by half of the needed gap (or use `min_item_separation` when all buffers are equal), and shrink the strip by `edge_gap`. Map placements back to the original, un-inflated piece coordinates.
- Every result must pass `beta_marker_core::validate` before it is returned. Report engine, seed, runtime, and warnings. A marker longer than `fabric.max_length` is a validation error (`too_long`), so `nest` returns an error for it.
- Collision shapes must fully cover the true outline: the engine works in f32, so every shape is inflated by a small guard and checked for coverage. Outlines are cleaned (duplicate and collinear points) and self-intersecting outlines are rejected with the piece id.
- Version 0.2 targets: locked placements (partial re-nest), a second engine (PackingSolver, MIT) for comparison, multi-start portfolio.

## 5. Validation (agent `qa-export`)

`beta_marker_core::validate(&Job, &MarkerResult) -> ValidationReport` is independent of the nesting code. Error codes: `unknown_piece`, `missing_piece`, `extra_piece` (counts per piece and mirror state), `mirror` (mirrored copy without `mirror_quantity`), `grain` (rotation not allowed), `outside_width`, `outside_start` (x below the edge gap), `overlap`, `gap` (closer than the required gap), `too_long` (`max_length`). Warnings: efficiency or length in the result differ from the recomputed values. Use exact polygon geometry with a tolerance of 0.01 mm. It must handle 1,000 pieces in under one second in release builds (use bounding boxes or a grid to skip far pairs).

## 6. Input and output formats

- **Import (agent `dxf`):** DXF-AAMA and ASTM D6673 style files: one BLOCK per piece; layer 1 = cut line, 7 = grain line, 4 = notches, 13 = drill holes, 8 / 11 = internal lines; TEXT `Piece Name:`, `Size:`, `Quantity:`, `Category:`, `Material:`; grading (rule tables) is out of scope for 0.1 except graded nests already split into sizes. Units from the header (`$INSUNITS`) or AAMA `UNITS` text; inches are common. Report every assumption as a warning. Also accept a generic DXF of closed polylines (one piece each).
- **Output (agent `qa-export`):** SVG (preview), HPGL/PLT (40 units per mm, `IN; SP1; PU/PD`, outlines, marks, labels), PDF (vector, 1:1, optional tiling). Calibration test: a 100 mm square must plot as 100 mm.

## 7. Version 0.2: full cutting-room features

Contract fields are in `model.rs` (all optional, default = version 0.1 behaviour). Helpers in `rules.rs`: `layer_copies`, `placed_outline`, `effective_job`. Every feature must be checked by `validate`.

| Feature | Contract | Meaning | Owner |
|---|---|---|---|
| Partial re-nest | `NestOptions.fixed`, `Placement.locked` | Keep given placements exactly; nest the rest of the demand around them. | nest |
| Portfolio / warm start | `NestOptions.runs`, `warm_start` | Several seeds in parallel, best valid wins; start from an earlier result. | nest |
| Faceless fabric | `fabric.allow_flip`, `Piece.flip` | Any copy may be placed mirrored; counts are per piece (`quantity + mirror_quantity`). Per-piece override: `rules::flip_allowed`. | nest, validator |
| Per-piece tilt | `Piece.tilt_deg` | Overrides `fabric.tilt_deg` for one piece (`rules::allowed_rotations`). | nest, validator |
| Folded / tubular | `fabric.layout`, `Piece.fold_line`, `Placement.on_fold` | Two layers per placement (`rules::layer_copies`); symmetric pieces may be placed as a half on a fold edge (folded: `y = width`; tubular: `y = 0` or `y = width`). Surplus layer copies are warnings, missing copies are errors. | nest, validator |
| Defects | `fabric.defects` | No placed outline may overlap a defect zone (gaps do not apply to defects). | nest, validator |
| Fusing blocks | `Piece.block_margin` | Nest the grain-aligned bounding rectangle + margin instead of the outline. Validation and output use the block rectangle as the cut line; the report lists blocks. | nest, validator, export |
| Shrinkage | `fabric.shrinkage` | `rules::effective_job` enlarges pieces before nesting; results refer to the effective job; `validate` applies it too. | nest, validator |
| Bundles, one way per garment | `Placement.bundle`, `Nap::OneWayPerBundle` | Every nest result fills `bundle`: garment `k` of `rules::bundle_sizes` (sizes in garment order XS < S < M < L < XL, numeric sizes by value). With `one_way_per_bundle` all pieces of a bundle share 0 or 180. | nest, validator |
| Shade bands | `fabric.shade` | All placements of a bundle lie in one band (`k * band_mm` to `(k + 1) * band_mm` on the axis). | match, validator |
| Stripe/check matching | `fabric.repeat`, `Piece.match_points`, `Job.match_rules` | Placed match points are in phase with the repeat within `tolerance_mm` (1 mm default), per bundle. | match, validator |
| Cut order planning | `beta_marker_plan::{CutOrder, CutPlan}` | Split an order (garments per size) into lays (ratio + plies) under ply and marker-size limits, minimising fabric. | plan |
| Batch / width trials | `beta-marker batch` | Nest many jobs, optionally at several widths, with a summary CSV. | plan |
| Compare with current CAD | `beta-marker import` of HPGL/PLT markers | Read a marker plotted by another CAD, rebuild pieces and their placement, recompute its length/efficiency, re-nest the same pieces. | dxf |
| Grading | AAMA/ASTM rule tables | Generate sizes from a base size and grade rules. | dxf |
| Cutter output | `beta-marker export --format iso|dxf|xlsx` | ISO 6983 style G-code (configurable), AAMA-style marker DXF, Excel cutting report, bundle labels. | export |

## 8. Paths and commands

Environment variables (never commit their values):

| Variable | Meaning | Example on the lead machine |
|---|---|---|
| `CARGO_TARGET_DIR` | Rust build output, one folder per worktree | `<data-disk>/cargo-target/marker-nest` |
| `BETA_DATA_DIR` | Datasets and benchmark files | `<data-disk>/project-beta-data` |

```sh
cd marker
cargo test --workspace                       # all Rust tests
cargo run --release -p beta-marker-cli -- import testdata/dxf/shirt-aama-mm.dxf --width 1500 -o out/job.json
cargo run --release -p beta-marker-cli -- nest out/job.json --time 30 -o out/result.json
cargo run --release -p beta-marker-cli -- validate out/job.json out/result.json
cargo run --release -p beta-marker-cli -- export out/job.json out/result.json --format plt -o out/marker.plt
cargo run --release -p beta-marker-cli -- serve                       # http://127.0.0.1:7878
npm test                                     # at the repository root: vitest, includes marker packages
```

Definition of done for every agent: `cargo test --workspace` and `cargo clippy --workspace` pass in `marker/`, `npm test` and `npm run typecheck` pass for your TypeScript workspace (if any), no `not implemented` left in your files, and your work is committed on your branch.
