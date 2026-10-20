# Cutting-room integration report

Branch: `marker/integrate`, based on `31297f1`. No other branch was merged.
Only the integrate-owned commands/crates and the explicitly permitted shared
cut-outline/metrics changes are included. No TypeScript, workspace manifest,
CLI entry table or root README was edited.

## Implementation

- Validator counts and garment checks use `rules::flip_allowed` per piece.
  Canonical bundle IDs/sizes use `rules::bundle_sizes`. Garment nap checks honor
  the per-piece tilt override. Floor bundles also use canonical garment size order.
- Physical `cut_outline` moved into core rules; the old validator path remains
  a compatibility re-export. Core metrics now includes block margins in length
  and keeps actual outline area for efficiency. Export reports reuse it.
- `roll inspect|group|allocate|spread|defects-to-marker` and
  `cutroom bundle|tickets|stickers|recut|report|cost` are wired. Help includes
  plain-English descriptions and examples. Incomplete roll allocations are
  saved and return an error. Re-cut output includes jobs and updated ledger.
- Typed adapters carry compensation/stock IDs, mm/metre conversion, both end
  allowances, stable lay identity, ply shade identity and physical folded/on-fold
  counts. Full shade identity is a compact JSON array in QR data; printed text
  uses a readable lot/dye/shade line. Repeated parts get unique physical slots.
- `plan pipeline` writes final checked markers, an audited spreading plan,
  physical bundles, QR tickets/stickers, an Excel planned-production report,
  PLT/ISO and mini-marker files per lay. Recorded stock defects are checked
  before print/cutter output. Pending replacements stop the pipeline with maps.
- Planner estimates are updated from measured short nests. Small/large ratio
  probes get three times the short budget, capped at 60 s. Their relative
  gains calibrate shortlist ranking. `PlanOptions.final_time_s` defaults to 0;
  the pipeline default is 120 s. Distinct ratios are re-nested once and reused
  across repeated lays. Actual validated lengths decide the final comparison.
  The explicit feasible short-plan incumbent is retained even if bounded
  re-search misses it; a longer final nest cannot worsen its marker length.
- `beta_marker_export::export(format, job, result, options)` returns bytes,
  MIME and extension for all nine formats. Typed functions retain custom ISO,
  label-grid and workbook-table options. `to_tables_xlsx` writes whole-order
  tables without presenting one marker's summary as an order total.

## Validation and measurements

- `cargo test --workspace`: fails in the existing server wire-schema fixture
  assertion (missing piece `flip`/`tilt_deg`). It runs 27 passing CLI tests,
  one failure and one intentionally ignored long pipeline test.
- `cargo test --workspace --no-fail-fast`: **213 passed, 4 failed, 4 ignored**;
  all doc tests passed. All integrate-owned tests pass. Failures are detailed below.
- `cargo clippy --workspace --all-targets -- -D warnings`: **passed, zero warnings**.
- `cargo build -p beta-marker-cli`: **passed**.
- `git diff --check`: passed.
- Poppler confirms valid A4/PDF 1.4 ticket and sticker sheets. Rendered first
  pages have readable text, intact QR quiet zones and no clipping. OpenCV 5
  independently decoded the small pipeline ticket: order O1, lay L001, full
  inventory identity `["lot","dye","S1"]`, part front, bundle 1, quantity 3.
  No QA-tool package was added to the product.

Workspace failures requiring parallel-owner work:

1. `cmd::serve::tests::v02_routes_preserve_options_and_plan_contract`: the shared
   fixture omits the new optional piece fields. Owner: ui3.
2. `beta_marker_wasm::tests::json_api_obeys_the_shared_model_contract`: same
   stale fixture fields. Owner: ui3.
3. `beta_marker_nest::tests::examples_are_reproducible`: generated examples omit
   `flip: null`/`tilt_deg: null` now emitted by the contract. Owner: quality.
4. `v02::tests::no_order_keeps_distinct_sizes_in_distinct_garments`: newly exposed
   contract mismatch. The shared `bundle_sizes` explicitly returns `[None]`
   without an order; the old nester returns one garment ID per distinct size.
   The migrated validator follows the requested shared contract and rejects
   the second ID. Quality/lead must agree on no-order semantics. Do not silently
   add a second private ordering implementation. Explicit size-ratio orders,
   including both Simon cut orders and the full integration pipeline, are unaffected.

Short-budget real-order runs used the public `demo_register()` fixture:
40 deterministic synthetic rolls, three inventory shades, 1500 mm usable
width, 2% length/1% width compensation. End allowance was 10 mm at each end
of every ply. Candidate/final budgets were 0.1/0.3 s, ten measured ratios,
5000 integer states, seed 42, two nesting workers. These are deliberately
short checks, not production-quality nesting claims.

- **Simon 1200:** 1703.698791 m, 3 lays, ratio S1 M2 L2 XL1 at 80/80/40 plies,
  marker 8498.493953 mm, 79.141660% efficiency, all 1200 garments exactly,
  1140 floor bundles and 22,800 physical pieces. Planner runtime 69.590 s.
  Calibration retained the original lay choice/objective (1705.199 m including
  1.5 m lay-cost equivalents). All three repeated lays reuse one final marker.
- **Simon awkward:** 401.146309 m, 5 lays, all S37 M113 L96 XL41 exactly,
  407 floor bundles and 5453 physical pieces. Marker efficiencies range
  78.36-80.53%. Calibration retained the short-plan objective, 403.646 m.
  Search is bounded; its warning explicitly reports the state limit.

Both spreads are complete; allocated metres equal planned fabric within
1e-7 m. Every per-lay validator passes. The workbooks contain Summary, Cut plan
and Bundles sheets, with 3/5 lay rows and 1140/407 bundle rows respectively.
There are **33 files** for the balanced order and **49 files** for the awkward
order: cutorder/cutplan, lay requests, audited spread JSON/CSV, bundle JSON,
tickets/stickers, whole-order XLSX, and eight files per lay (job, result,
validation, defects, physical cutroom lay, PLT, ISO and mini-marker PDF).
Artifacts are under `$BETA_DATA_DIR/marker-bench/integrate/simon-1200-short`
and `simon-awkward-short`.

Exact CLI invocations (use the built beta-marker executable):

```text
beta-marker roll --help
beta-marker cutroom --help
beta-marker plan pipeline --help
beta-marker plan pipeline "$BETA_DATA_DIR/marker-bench/plan/simon-1200.cutorder.json" --rolls "$BETA_DATA_DIR/marker-bench/integrate/demo-rolls.json" --time-per-marker 0.1 --final-time 0.3 --max-markers 10 --search-nodes 5000 --end-allowance 10 --colour Blue -o "$BETA_DATA_DIR/marker-bench/integrate/simon-1200-short"
beta-marker plan pipeline "$BETA_DATA_DIR/marker-bench/plan/simon-awkward.cutorder.json" --rolls "$BETA_DATA_DIR/marker-bench/integrate/demo-rolls.json" --time-per-marker 0.1 --final-time 0.3 --max-markers 10 --search-nodes 5000 --end-allowance 10 --colour Blue -o "$BETA_DATA_DIR/marker-bench/integrate/simon-awkward-short"
```

The print files are regenerated through `cutroom tickets|stickers` using the
lossless compressed writer. The original uncompressed 22,800-piece sticker
batch was 1,854,072,239 bytes; the compressed file is **260,403,184 bytes**
(7.12 times smaller). Balanced-order tickets fall from 66,500,387 to
**9,468,117 bytes**. Awkward-order stickers/tickets are 62,393,939/3,340,693 bytes.
The small one-ticket PDF falls from 50,014 to 7,784 bytes and renders
pixel-identically. OpenCV independently decodes both its original payload and
a real Simon bundle ticket (SIMON-BACK-S, 20 pieces, L001, full lot/dye/S0).
Poppler confirms the label-printer ticket is exactly 100 x 50 mm.

All ten new operations were also invoked through the actual CLI, using the
saved Simon plan/physical lay and clearly named synthetic actuals. Inspection
produced CSV; grouping returned three groups; CutPlan-to-spread allocation was
complete; defect-to-job application preserved the defect-free marker; bundling,
label-size tickets/stickers, re-cut scheduling, reporting and quick costing all
wrote valid outputs. Exactly one damaged piece became one scheduled re-cut job.
The synthetic report reconciles 61.200 m planned / 61.500 m actual, 0.300 m
variance, 240 pieces and 58.5366% cutting efficiency. It is a CLI fixture,
not an observation of actual Simon production. Smoke artifacts are in
`$BETA_DATA_DIR/marker-bench/integrate/cli-smoke`.

The first optional long test attempted both orders with 6 s candidate nests,
120 s final nests and ten ratios. It failed after **864.88 s** in jagua-rs
`collision_detection/quadtree/qt_hazard.rs:116`, assertion
`constricted_hazards.iter().filter(|h| h.is_some()).count() > 0`. No production
plan was saved. Raw log: `$BETA_DATA_DIR/marker-bench/integrate/simon-long.log`.
No vendor/library source was patched.

Planner engine calls now have an unwind boundary. A failed short trial becomes
an unavailable ratio; calibration/final failures keep their previous checked
marker and name the ratio in a warning. A regression test verifies that worker
panics become errors and ordinary successes remain unchanged. This contains
an engine failure; it does not solve its geometric cause.

The final ignored `simon_long_pipeline` retry **passed in 357.50 s**. It covers
the balanced 1200-shirt order with five measured ratios (four single-size
repairs plus the balanced mixed ratio), 6 s candidates, 18 s calibration probes,
120 s final search, seed 42 and 5000 integer states. Both full orders are
exercised end to end with short budgets above.

Long-run output: **1681.724180 m**, three lays at 80/80/40 plies, unchanged
S1 M2 L2 XL1 ratio and exact size totals, marker **8388.620898 mm**, efficiency
**80.178247%**, 1140 bundles / 22,800 pieces / 33 files. Planner wall time is
253.791 s; the full 357.50 s test includes print/export generation. All three
validators pass. Allocated cloth is 1681.724180 m and the spread is complete.
Artifacts: `$BETA_DATA_DIR/marker-bench/integrate/simon-1200-long-26500`.
The calibrated short-plan objective was 1705.199 m; final actual objective is
1683.224 m. This saves **21.974611 m (1.29%)** against the short run, with no
worse lay choice and no loss of quantity coverage. The same final marker is
reused for all three lays.

The earlier approximately 89% benchmark was **not reproduced** in this
integration environment. Separate fresh 120 s trials on exactly the same
stock-compensated job reached 79.598628% at seed 42 (8449.704891 mm, 133.34 s)
and 82.249223% at seed 1 (8177.401465 mm, 145.17 s). Geometry, quantities and spacing
match the source Simon ratio job; differences are explicit stock compensation
and default serialized fields. A cold planner comparison also hit jagua-rs
`qt_hazard.rs:116` and Sparrow `optimizer/worker.rs:87` assertions and was
stopped; it is not counted as a passing result. Logs/results are in
`marker-bench/integrate/simon-cold120*`, `simon-seed1*`, `simon-long-cold.log`.
The retained implementation uses the checked warm-refinement method whose
complete pipeline passed. Quality must investigate the engine quality gap and
worker assertions after merge; no vendor source was patched or validator rule
weakened. The planner's guard contains unwinding panics, not process aborts or
workers that never return.

`plan` and `plan pipeline` now expose `--seed` (default 42), matching the library
option, for controlled comparisons. The long test passes the same seed 42
explicitly. Reproduce it with:

```text
cargo test -p beta-marker-cli simon_long_pipeline -- --ignored --nocapture
```

Final focused planner check: 12 passed, one historical real-plan test ignored;
this includes the new unwind-boundary regression. Final workspace/clippy/build
logs are `marker-bench/integrate-workspace-delivery.log`,
`integrate-clippy-delivery.log`, and `integrate-build-delivery.log` under the data
root. (The standard workspace run intentionally excludes the long test.)

All Cargo commands run from `marker/` with the assigned worktree-specific
`CARGO_TARGET_DIR`, `CARGO_BUILD_JOBS=4` and `BETA_DATA_DIR`. No generated dataset,
PDF, lockfile or build output is committed. Fast nesting tests use two workers
and small search budgets. The long Simon pipeline test is explicitly ignored.

## Dependencies and licences

New direct dependency: `flate2` 1.1 (resolved 1.1.10), **MIT OR Apache-2.0**,
with default features disabled and `zlib-rs` enabled. Licence/features were
checked in the downloaded crates.io manifest. Both flate2 and zlib-rs 0.6.8
(**Zlib**) already belonged to the XLSX dependency graph: no new transitive
package is needed. PDF pages now use lossless FlateDecode streams, preserving
vector geometry and exact QR payloads while reducing large print batches.

New internal dependency edges are plan -> roll, cutroom -> roll and cutroom ->
match; all are workspace Apache-2.0 crates. The existing unmodified
Sparrow/Jagua exception is unchanged. Cargo.lock is left for lead regeneration.

## Limits

The pipeline chooses the shrinkage-band pair with the most usable stock, then
uses the largest measured compensation on each axis among those rolls. It
keeps inventory shades separate per lay. It does not split an order across
incompatible shrinkage bands or silently mix them to cover a shortage. The
library `apply_roll_group` supports exact group-mean compensation with an
explicit stock restriction. A partial spreading result can require smaller
lays or more compatible stock. Splicing is off in pipeline-generated requests.

The workbook describes planned production. Actual cutting efficiency needs
measured withdrawals/returns and actual piece counts through `cutroom report`.
Physical plotter/cutter/printer calibration remains a shop-floor check. ISO is
an exchange profile, not a vendor machine-control protocol.

## Handoff requests

- **Lead:** add `Pipeline(cmd::plan::pipeline::Args)` to the top-level command
  enum and dispatch it to `cmd::plan::pipeline::run(a)` if the shorter
  `beta-marker pipeline` spelling is wanted. `plan pipeline` already works.
  Regenerate `Cargo.lock` after integration. No model changes are requested.
- **quality:** replace private `nest/src/v02.rs::size_key`/`bundles` and
  `match/src/lib.rs::bundles`/`bundle_for` ordering with shared `bundle_sizes`.
  Regenerate example JSON for the new optional fields. Resolve the no-order
  mismatch above with the lead (shared helper says one garment, current nester
  test expects one per distinct size). Migrate remaining fabric-wide flip tests in nesting to per-piece
  `flip_allowed`. Recheck numeric sizes, flip overrides, blocks, folds and
  calibrated/final warm starts after merge. Investigate the jagua-rs quadtree
  assertion from the first long attempt (log and settings above); the planner
  now contains panics and retains checked incumbents. These files were not
  changed here.
- **ui3:** use the unified byte export API and honor its returned MIME/extension.
  Preserve/default `PlanOptions.final_time_s` in server/UI options. Update the
  stale server/WASM contract fixture that omits piece `flip` and `tilt_deg`; the existing
  server wire-key assertion fails on this base. Re-run the entire workspace
  after that fixture correction and the quality branch merge.

## Proposed root README text (lead-owned)

BETA Marker now connects cut orders to the cutting room. `beta-marker roll`
inspects and groups measured cloth, allocates stock and maps defects.
`beta-marker cutroom` makes physical bundles, QR tickets and numbering stickers,
schedules shade-matched replacements, reports measured use and estimates cost.
`beta-marker plan pipeline order.json --rolls rolls.json -o cutting-room`
applies stock shrinkage, plans and finally re-nests chosen lays, validates them,
allocates rolls and writes print, Excel and cutter files. Final nests default
to 120 seconds per distinct chosen ratio; use `--final-time` to change this.
See the plan, roll, cutroom and export crate READMEs for schemas and limits.
