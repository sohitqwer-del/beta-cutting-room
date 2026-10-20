# Production operations handoff

Branch: `marker/ops`, based on `marker/mvp` `17d7e75`. No other agent branch was
merged. Changes are confined to the queue/plan/cutroom crates and the new CLI
queue command module. Core contracts, workspace manifest, CLI main and cmd/mod
remain lead-owned and unchanged. Generated geometry, measurements, PDF/XLSX and
lockfiles are excluded from commits.

## Working APIs

- `beta_marker_queue::Queue`: `open`, `add`, `list`, `status`, `pause`, `cancel`,
  `resume`, `checkpoint`, `marker_checkpoints`, `run`. `Submission` embeds a typed
  `Task::{Nest,Batch,Plan,Pipeline,WidthTrials}`. `RunOptions` bounds total engine workers
  and a reserved checkpoint interval. Execution is continuous with this engine.
  `Progress` distinguishes an engine length update
  from a durable geometry checkpoint. `RunSummary` isolates failures and corrupt
  job files. JSON per job, atomic flushed replacements and OS runner/state locks
  support process-restart recovery, including an orphan checkpoint created before
  its job-pointer rename.
- Queue pipeline: compensated order -> checked plan -> audited stock -> allocated
  defect checks -> physical parts/bundles -> QR tickets/stickers, planned XLSX and PLT/ISO/mini-marker
  output. Optional `ReportSettings` writes the entire merchandiser report pack.
  Shortage and pending defect repair stop production output with reviewable JSON.
- Planner: `plan_with_progress`, `plan_ratios_with_progress`,
  `plan_ratios_with_engine`, `refine_plan_with_progress`, `refine_plan_with_engine`
  and the `NestExecutor` callback type. Explicit ratio menus do not silently add
  repair ratios. Existing direct `plan` avoids the extra partial-plan solves.
- `beta_marker_plan::cost`: `CostRequest`, `WidthOffer`, `RatioSet`,
  `FabricPrice::{Metre,Kilogram}`, `compare`, `compare_with_progress`,
  `compare_with_engine`, `price_plan`. Shortlisted alternatives receive longer
  warm refinement; actual lengths drive new integer lays and a fresh audited roll
  allocation. Price includes consumed cloth, explicitly charged recoverable ends,
  purchase wastage and lay cost. Kg-priced stock uses measured roll width, including
  width trim. Only feasible final alternatives can win; failed alternatives remain.
- `beta_marker_cutroom::production`: `ReportInput`, `ReportMeta`, `OrderActuals`,
  `RollActual`, `ReportPack`, `build`, `csv_files`, `xlsx`, `pdf`. One full-precision
  result feeds costing, per-lay tickets, width comparison, roll/end-bit registers
  and size reconciliation. Net/planned, gross/ordered and actual/good-cut
  denominators are separate. Units include m/kg per piece and dozen, yards, GSM
  and inches. Folded/tubular mass expands physical width once. Wider measured stock
  affects planned and actual kg. Missing actuals stay blank. XLSX uses numeric
  cells and cached editable formulas; CSV escapes formula-leading identifiers.

See `OPS.md`, the queue README and cutroom `PRODUCTION_REPORTS.md` for schemas,
cost formulas, time-budget semantics and limits.

## Required lead registration

Remove the temporary `[workspace]` table from
`crates/beta-marker-queue/Cargo.toml`. In `marker/Cargo.toml` add:

```toml
# [workspace] members
"crates/beta-marker-queue",

# [workspace.dependencies]
beta-marker-queue = { path = "crates/beta-marker-queue" }
```

In `crates/beta-marker-cli/Cargo.toml` `[dependencies]` add:

```toml
beta-marker-queue.workspace = true
```

In `crates/beta-marker-cli/src/cmd/mod.rs` add exactly:

```rust
pub mod queue;
```

In the `Command` enum in `crates/beta-marker-cli/src/main.rs` add:

```rust
/// Persistent unattended production jobs, checkpoints and recovery
Queue(cmd::queue::Args),
```

In its dispatch match add:

```rust
Command::Queue(a) => cmd::queue::run(a),
```

CLI: `beta-marker queue --folder PATH add submission.json`, `list`,
`run --workers 2`, `cancel ID`, `pause ID`, `resume ID`, `status ID`.
Add accepts `--priority`, `--tier draft|costing|production|overnight`, and custom
`--seconds` for overnight. The standalone `queue/examples/cli.rs` harness compiles
and tests this exact module before registration. No core field changes are needed.

## Planner bias guard

The old endpoint-only calibration is replaced with one longer representative per
measured garment-count class. Representatives use shortest measured length per
garment, not raw percentage across sizes. Gains interpolate piecewise between
classes for nomination only. Final geometry, reported fabric and costs always use
actual validated lengths; original feasible ply actions survive bounded re-search.
Warm refinement never replaces a shorter checked incumbent.

The deterministic bias regression starts with two-garment short markers worse
per garment than a one-garment marker. Longer class probes reveal the better
two-garment arrangement, and actual final length selects 0.202 m rather than
0.404 m. These deliberately synthetic figures test the decision logic, not a
fabric-saving claim. Another regression chooses cheaper, wider, lower-density
cloth over a valid narrower, higher-density alternative and retains a third
unavailable width as an explicit failure.

## Verification and measured runs

All final checks pass:

```text
cargo test --offline --workspace --no-fail-fast -- --test-threads=1
cargo clippy --offline --workspace --all-targets -- -D warnings
cargo test --offline --manifest-path crates/beta-marker-queue/Cargo.toml --all-targets -- --test-threads=1
cargo clippy --offline --manifest-path crates/beta-marker-queue/Cargo.toml --all-targets -- -D warnings
cargo test --offline --workspace production:: -- --test-threads=1
cargo test --offline --workspace cost_tests:: -- --test-threads=1
cargo test --offline -p beta-marker-plan simon_width_cost_report -- --ignored --nocapture --test-threads=1
cargo test --offline --workspace write_report_pack_for_review -- --ignored --test-threads=1
git diff --check
```

- Full marker workspace: **242 passed, zero failed, six ignored**; all doc tests
  pass. The separate queue workspace: **eight passed**, plus **one CLI parsing
  harness test**. Both strict Clippy checks pass with zero warnings.
- Final report tests: six passed, one artifact test ignored by default. Final cost
  tests: five passed, one real-order test ignored by default. The real Simon cost
  test passed explicitly in **173.43 s**. Artifact generation passed explicitly.
- The actual CLI harness ran add/status/pause/run/resume/run/status/add/cancel/list.
  One tiny marker completed with a valid checkpoint; the second stayed cancelled.
  This uses the exact unregistered `cmd/queue.rs`, not a duplicate implementation.
- The final synthetic report is PDF 1.4, six A4 pages. Poppler rendering was checked
  visually; row rules, page headers, wrapping and the lay ticket are readable.
  XLSX ZIP/XML inspection checks numeric cells, formulas and cached results;
  CSV identifiers are escaped. Native Excel/printer acceptance remains external.

Logs and synthetic artifacts are under `$BETA_DATA_DIR/marker-bench/ops/`:
`workspace-tests-final.log`, `workspace-clippy-final.log`, `queue-tests-final.log`,
`queue-clippy-final.log`, `production-tests-final.log`, `cost-tests-final.log`,
`report-qa-final.log`, `report-sample/` and `cli-smoke/`. The report fixture reconciles
1.1 m net + 0.2 m ends = 1.3 m consumed, plus 10% purchase allowance = 1.43 m;
INR 143 fabric + INR 20 lay = INR 163. Measured synthetic use is 1.8 issued minus
0.4 returned = 1.4 m, with nine good garments versus ten planned. These are test
records, not observations of factory output.

### Real Simon width/cost trials

Both existing FreeSewing orders were read from `$BETA_DATA_DIR/marker-bench/plan/`.
Widths: 1450 and 1500 mm. The public synthetic 40-roll register supplies physical
1500 mm stock, 2% length/1% width compensation and separate shades. Assumptions:
150 GSM, INR 240/kg (INR 54 per physical stock metre), INR 150/lay, 2% purchase
allowance, 25% charge on recoverable opened-roll ends, zero head/tail allowance.
Short/probe/final search budgets are 0.05/0.15/0.3 s, two workers, seed 42, five
short ratios, 32 candidate limit and 3000 integer states. These are short-budget
checks, not production-quality or optimality claims.

- **1200 shirts:** choose 1500 mm, **1699.698791 m**, three lays, exact S200/M400/
  L400/XL200. Total **INR 95,573.60**, versus **INR 97,225.81** at 1450 mm
  (1731.444 m). The narrower marker is denser (80.3696% versus 79.1417%) yet costs
  more because it consumes more linear stock and incurs its own roll balance.
  Chosen opened-roll ends: 111.421209 m recoverable, 27.855302 m charged.
- **Awkward order:** choose 1500 mm, **412.173444 m**, four lays, exact S37/M113/
  L96/XL41. Total **INR 23,426.13**, versus **INR 24,095.68** at 1450 mm
  (425.016 m). Chosen opened-roll ends: 9.156556 m recoverable, 2.289139 m charged.

Final marker geometry and fresh roll allocations validate for all chosen lays.
The full alternatives and price components are in
`simon-1200-ops-width-cost.json` and `simon-awkward-ops-width-cost.json` in the plan
data folder. Both offers price physical 1500 mm stock, including the narrower
trial's trim. No reusable end bit is silently called scrap.

### Paired continuous/sliced quality experiment

Requested inputs: Simon ratio and development IDs 8411, 1561, 88853, 14851, 18110.
Seed 42 and two workers for all cells. Strategies: continuous 60 s, 12 x 5 s warm
starts, and 4 x 15 s warm starts. Each slice starts from its previous checked best;
strategy order rotates across jobs. Every retained result validates independently.
Hardware: i9-11900H, 8 cores/16 logical processors, Windows, Rust 1.98.1; Cargo dev
profile (workspace opt-level 1, external dependencies opt-level 3), shared load.

Completed results below are continuous / 12x5 / 4x15, with wall seconds in brackets:

- Simon: **80.756866 / 80.676111 / 80.756876%**
  [167.24 / 861.18 / 252.41 s]. Five-second loss: **0.080755 pp**.
- Dev 8411: **67.571488 / 67.072811 / 67.971370%**
  [61.52 / 76.77 / 67.00 s]. Five-second loss: **0.498677 pp**.
- Dev 1561: **91.626081 / 91.648920 / 91.648920%**
  [157.06 / 1412.00 / 396.97 s]. Five-second gain: **0.022839 pp**.
- Dev 88853: **65.051532 / 66.972612 / 66.614720%**
  [64.11 / 91.50 / 69.81 s]. Five-second gain: **1.921080 pp**.

Across these **four completed pairs**, mean continuous-minus-sliced differences
are **-0.341122 pp** for 5 s and **-0.496480 pp** for 15 s (negative means the sliced
result was better). Thus the mean does not exceed the 0.2 pp loss guard; the
individual 8411 regression and repeated setup overhead still prevent a safety or
throughput equivalence claim. Equal search budgets are not equal wall/CPU budgets.

The 367-piece dev 14851 case returned no first layout/result for its first 5 s
trial for about **11 min 43 s**. The lead explicitly stopped it at 18:30:18 local
time and skipped the second large case, dev 18110 (230 pieces). **Four of six
pairs are complete; the other two are unscored.** No efficiency was invented for
them. Without phase-level instrumentation, this is a bound on first-layout/setup
latency, not an exact pure-constructor measurement. Finding: large (300+ piece)
construction can outlast a short slice by orders of magnitude.

The resulting policy is stronger than simply raising the default: **checkpoint
slicing is disabled** until construction can be retained. Each phase is one
continuous engine call. The 60 s soft construction guard exposes a persistent
continuous-fallback status; it does not restart construction. Hard preemption and
full-geometry checkpoints inside a long call remain explicit engine API limits.
Future slices must start from valid geometry, retain construction state and last
at least max(60 s, measured construction time).

Reproduction tools are `examples/slice_bench.rs` and `examples/slice_report.rs`.
Raw rows, checked per-cell results and curves, input manifest, comparison JSON and
the lead's stop/skip selection record live under
`$BETA_DATA_DIR/marker-bench/ops/slice-bench/`. The existing approximately 87-89%
Simon figure was **not reproduced** here, including the continuous 60 s control.
The planner ranking guard is tested, but closing that engine/runtime quality gap
needs a separate matched-profile investigation after the quality branch merges.

All Cargo commands run from `marker/`, with the assigned external target,
`CARGO_BUILD_JOBS=3` and `BETA_DATA_DIR`. Fast nesting tests pass two workers and
explicit budgets no greater than 5 s. Real Simon checks are ignored by default.
No TypeScript changes were made.

## Operational limits and re-tests after merge

- This is a serial offline queue, not a service that stays open when empty.
  Priority orders waiting jobs; it does not preempt a running engine call.
  Separate queue folders have separate worker limits. Planner jobs need two
  workers; a single Nest task can use one. Occupied worker seconds are engine
  wall seconds x declared workers, not measured hardware CPU utilisation.
- Draft/costing/production tiers are 20/60/300 search seconds. Nest uses a total
  budget; plan/pipeline tiers apply per marker, matching the existing planner
  convention. Overnight is custom up to 24 h per marker. Construction, checking
  and export add wall time. Search counters describe dispatched search budgets.
- Checkpoint slicing is disabled. Each engine phase is one continuous call;
  cold construction is never repeated just to obtain a checkpoint. Warm
  refinement/resume still uses checked incumbents. The 60 s soft construction
  guard writes a continuous-fallback status and persistent flag, and records
  first-layout latency. It does not restart or preempt construction. A hard
  timeout, retained engine construction state and full geometry saves inside a
  single long call need the quality agent's callback/session or managed-process
  supervision. These are known limits. Cancel/pause waits for a continuous call.
  Any future slice must be >=60 s and >=measured construction time, after a valid
  layout exists; the current implementation avoids that risk by not slicing.
- Unwinding engine and engine-callback panics isolate a trial/job and retain earlier checked
  geometry. Process aborts or a worker that never returns need external process
  supervision; they cannot be stopped by the current in-process engine API.
- Width-cost planning currently requires open fabric and pre-compensated approved
  stock. It evaluates allocation/end-bit effects across supplied width/ratio menus,
  rather than solving an exact joint nonlinear roll/ratio problem. No global
  optimality or vendor efficiency parity is claimed. Allocated defects still need
  pipeline checks before a cutting release.
- Report actuals need measured issue/returns and good garments. Test actuals and
  stock are labelled synthetic. PDF uses Helvetica with ASCII replacement; XLSX
  and CSV preserve Unicode. No factory PO approval, plotter calibration or ERP
  accounting posting is implied by these files.
- After merge: remove the nested workspace table, regenerate the lead lockfile,
  register CLI, run full workspace tests/clippy including the queue, call APIs from
  desktop background workers, and re-test using the quality agent's final engine
  and full-geometry callback. Re-run paired budgets when engine orchestration
  changes. Keep physical shade/shrinkage and stock-defect checks in the pipeline.

## Dependencies and licences

No new external package is introduced. New direct cutroom edges use existing
`rust_xlsxwriter` 0.95.0 (MIT OR Apache-2.0) and test-only `zip` 7.4.0 (MIT), checked
in the downloaded published manifests. New internal edges (cutroom -> plan and
queue -> existing marker crates) are Apache-2.0. Queue's anyhow/serde/serde_json
and its CLI harness's clap are existing MIT OR Apache-2.0 packages. Existing
Sparrow is MIT; Jagua remains the documented unmodified MPL-2.0 dependency
exception. No vendor format/code/sample, runtime network access, dataset, build
output or lockfile is committed.

Simon attribution: FreeSewing 4.10.2, MIT, Joost De Cock and contributors; source
geometry/assumptions remain in the existing fixture metadata. Lectra development
measurements use the existing external converted benchmark data without copying
geometry into the repository or touching private directories.
