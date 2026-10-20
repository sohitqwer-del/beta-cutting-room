# Cut order planning and marker queue

Version 0.4 adds [width/ratio planning by real cost](OPS.md), queue-owned nesting
executors and per-class budget calibration. See [OPS_REPORT.md](OPS_REPORT.md)
for the integration handoff and measurements.

A factory gets an order, for example 1,200 shirts: S 200, M 400, L 400,
XL 200. The cutting room cannot cut the order in one marker. A marker is the
arrangement of pattern pieces on the fabric. A lay is that marker spread over
several fabric plies. A ply is one layer of fabric.

Cut order planning chooses the lays. For example, the ratio S1 M2 L2 XL1 cuts six
shirts per ply. Spreading it over 60 plies cuts 360 shirts. Knife height limits
the number of plies. Table length and the maximum garments per marker limit
marker size. The aim is to meet every size quantity, within an allowed over-cut,
using little fabric and few lays. Each lay takes spreading and cutting time.
Gerber AccuPlan, TUKAcutplan, and Optitex CutPlan address this production problem.

## Planner

`plan(&CutOrder, &PlanOptions) -> anyhow::Result<CutPlan>` runs offline in Rust.
The embedded job holds all pattern sizes and fabric rules. Its original order
ratio is replaced for each candidate marker. Ordered quantities may be zero.
An ordered size with no pattern pieces is an input error.

1. Enumerate nonempty integer size ratios with at most the marker garment limit.
   Never include more of a size than its quantity plus allowed over-cut.
   Enumeration stops at 200,000 ratios. Rank by closeness to the order's size
   proportions, with a small preference for larger markers. Retain up to
   `max_candidates` (128 by default), including one single-size repair ratio
   for each ordered size. Pruning is reported.
2. Estimate length from effective piece area, width, and the style efficiency
   guess (`estimated_efficiency`, default 0.75). Nest single-size repair markers
   first. Optimise the estimated table, then refine the selected ratios and
   other promising ratios. Cache successes and failures by ratio within this
   planning call. Stop after `max_refinements` real nests (16 by default).
3. Nest through `beta_marker_match::nest_with_rules`, which dispatches ordinary
   jobs to the nesting engine. Use two workers, a seed, and the configured time
   budget per marker. The budget is search time; construction, repair, and
   checking add wall-clock time. Independently validate every real result.
   The table-length limit is enforced by nesting and validation. An estimated
   marker is never returned as a cutting instruction.
4. Choose integer ratio/ply actions by bounded branch-and-bound. First construct
   feasible greedy solutions, including a trial beginning with each ratio and
   every feasible plan that repeats just one measured ratio. This preserves
   simple production plans even when the search budget is small.
   Then branch on markers containing an unmet size, enumerate all useful ply
   counts up to the height limit, memoise production states, and prune with a
   fabric lower bounds (uniform cost and feasible size prices) and a capacity
   bound on the minimum number of lays. Search stops at `search_nodes` (20,000 by
   default) or depth 256. Greedy construction stops after 10,000 lays. If a
   candidate permits more than 4,096 useful plies, use quantity breakpoints
   instead of every ply count and report approximate search.

The objective is:

```text
sum((marker_length_mm + 2 * end_allowance_mm) * plies / 1000)
  + lay_cost_m * number_of_lays
```

End allowances are at both ends of **each ply**. They consume real fabric.
`lay_cost_m` (default 0.5 m) is a cost equivalent; it is not added to reported
physical fabric consumption. All lengths use millimetres except fields ending
in `_m`. Efficiency is a fraction, not a percentage.

For each size the upper quantity is
`floor(ordered * (1 + overcut_pct / 100))`. There is no rounding up to an extra
garment. Lays can repeat a ratio when the maximum plies requires several spreads.
The output lists ordered, planned, extra, and missing garments; physical fabric;
objective; measured markers; runtime; and warnings. A shortfall is explicit.
`complete` describes quantity coverage. `optimal_for_evaluated_markers` only
describes exhaustive search of the **measured ratio table**. Pruned or unevaluated
ratios and short nests prevent a claim of global fabric optimality. If the plan
is incomplete and search stopped early, infeasibility has not been proved.

## Commands

Run these commands from the `marker` workspace. Set `CARGO_TARGET_DIR` to a build
folder outside the repository and `CARGO_BUILD_JOBS=3` on the shared machine.
The example generator embeds the existing MIT fixture; it does not copy a new
dataset into git.

```sh
cargo run -p beta-marker-plan --example simon -- "$BETA_DATA_DIR/marker-bench/plan"
cargo run -p beta-marker-cli -- plan "$BETA_DATA_DIR/marker-bench/plan/simon-1200.cutorder.json" --time-per-marker 20 --lay-cost 0.5 -o "$BETA_DATA_DIR/marker-bench/plan/simon-1200.cutplan.json"
cargo run -p beta-marker-cli -- batch testdata/open/freesewing --widths 1450,1500,1550 --time 60 --runs 2 --jobs 1 -o "$BETA_DATA_DIR/marker-bench/plan/batch"
```

`plan` also accepts `--end-allowance` (mm), `--max-markers`, and `--search-nodes`.
It writes the JSON and prints a lay table and size totals. It exits with an error
after writing an incomplete plan, so queues can detect a shortfall.

`batch` accepts a folder, one job JSON, or a text file of job paths. Folder scans
are nonrecursive and skip JSON files whose format is not a marker job. Malformed
JSON is kept as a failed trial. In lists, one path appears per line; blank lines
and lines beginning with `#` are skipped. Relative paths refer to the list's
folder. Duplicate paths and duplicate widths are removed.

Each width/run gets a different seed. `--runs` is executed here, so it also works
with a nesting implementation that has not yet implemented portfolio nesting.
`--jobs` limits concurrent job/width trials; each nest uses two engine workers.
Results are independently checked, and the shortest valid run is retained.
Result filenames have a trial index to avoid collisions between equal basenames.
The exact width-adjusted job is saved beside every successful result.

The queue writes `summary.csv`, `summary.md`, `summary.json`, and
`best-widths.json`. Best width minimises fabric **area** (width × length), with
length breaking ties. This compares fabric use across different widths without
assuming the same price per linear metre. Failed trials remain in the summary;
the command reports an error after processing all trials. Individual failed
runs do not fail a width trial if another run returns a valid marker.

## Tests and measurement

Fast tests use synthetic length tables, including an independent exhaustive
dynamic-programming cross-check, height limits, over-cut, end allowances,
infeasible tables, and bounded search. The real-nest test is ignored by default:

```sh
cargo test -p beta-marker-plan
cargo test -p beta-marker-plan simon_real_plans -- --ignored --nocapture
cargo test --workspace
cargo clippy --workspace --all-targets
```

The ignored test makes both Simon orders, uses at most six garments per marker
and 80 plies, and writes orders and validated plans to
`$BETA_DATA_DIR/marker-bench/plan/`. The awkward order permits 5% extra per size.
Measurements and limitations are recorded in [REPORT.md](REPORT.md).

No new dependencies are required. The solver and queue use the Rust standard
library. Existing direct dependencies are `anyhow` (MIT OR Apache-2.0), `serde`
(MIT OR Apache-2.0), `serde_json` (MIT OR Apache-2.0), and the internal marker
crates (Apache-2.0). The existing CLI uses `clap` (MIT OR Apache-2.0).

Simon test data: FreeSewing 4.10.2, MIT, Joost De Cock and FreeSewing contributors.
Source and geometric assumptions remain in
`testdata/open/freesewing/simon-ratio.job.json` metadata.

## Integrated planning and final nesting

The planner updates unmeasured length estimates to the median measured/estimated
length ratio. This keeps estimates and short nests on the same scale. It then
nests one representative of each measured garment-count class for three times
the short budget (capped at 60 seconds each), using valid warm starts. Their
relative length gains calibrate ranking piecewise by garment count. The shortest
measured length per garment selects each class representative. Calibration belongs to the current
order/style; it is not reused across unrelated patterns.

`PlanOptions.final_time_s` defaults to **0**, which disables final re-nesting.
A positive value re-nests each distinct ratio in both the incumbent plan and the
calibrated nominee. Repeated lays reuse that marker. Only a shorter, independently
validated result replaces an incumbent. The final comparison uses actual lengths,
allowances and lay costs. It retains the explicit feasible short-plan incumbent
if bounded re-search misses it. Calibration estimates never become cutter geometry
or reported fabric totals. Final shortlist search does not claim optimality across
the whole evaluated table. The warning records probe gains and before/after costs.

`apply_roll_group(job, group)` assigns a group's mean shrinkage and returns its
roll IDs. `spreading_requests(plan, width_mm, eligible_ids)` converts
lays to `beta_marker_roll::LayRequest`, including both end allowances in metres,
plies and stock restrictions. IDs stay `L001`, `L002`, etc. when the allocator
changes execution order. Each lay keeps one inventory shade. Splicing is off.

```text
beta-marker plan order.json --time-per-marker 20 --final-time 120 -o plan.json
beta-marker plan pipeline order.json --rolls rolls.json --final-time 120 -o cutting-room
```

The pipeline is under `plan` so the lead can add a top-level `pipeline` alias
without changes to owned CLI entry files. Its final budget defaults to 120 s.
Use `--time-per-marker`, `--final-time`, `--max-markers`, `--search-nodes`,
`--end-allowance` (mm), `--pieces-per-bundle`, and `--colour` to adjust it.
`--help` includes examples. Use an empty output directory for each run.

The pipeline chooses the largest available shrinkage-band pair by usable stock,
keeps all its inventory shades separate per lay, and compensates by the largest
measured shrinkage on each axis within that band. This deliberately covers the
selected stock rather than averaging away a larger test result. Other shrinkage
bands are excluded. For a precise single-group mean, use `apply_roll_group` in
an application and restrict allocation to its returned IDs. The pipeline does
not silently use incompatible stock to fill a shortage. A partial spread is
saved and returns an error; split large lays or supply compatible stock.

Stages: stock/shrinkage -> cut plan -> longer final nests -> independent marker
validation -> audited roll allocation -> allocated-defect checks -> physical
parts and bundles -> QR ticket/sticker PDFs -> Excel production plan -> PLT/ISO
and mini-marker PDF per lay. Low-ply defects that hit a piece, or pending full-width
defect replacements, stop print/cutter output and preserve reviewable JSON.
Head allowance is removed from mapped defect x coordinates. The pipeline does
not model replacement cloth as already cut or fabricate actual cutting-room use.
The workbook explicitly describes **planned production**. Feed measured actuals
to `cutroom report` later.

Output includes `cutorder.json` (stock compensation applied), `cutplan.json`,
`lay-requests.json`, `spreading-plan.json/csv`, `bundles.json`, `tickets.pdf`,
`stickers.pdf`, `report.xlsx`, and per-lay job/result, defect/validation/cutroom
JSON plus `*-plt.plt`, `*-iso.iso`, and `*-mini.pdf`. Printed garment numbers
restart per lay; order/lay/full piece identity and globally reserved floor-bundle
numbers prevent ambiguity.

Fast tests check allowance units, stock restrictions, conservative shrinkage,
legacy command parsing, and a complete small pipeline. The ignored
`simon_long_pipeline` test runs the generated 1200-shirt Simon order with the public
40-roll synthetic demo stock and 120 s final budgets. Artifacts go under
`$BETA_DATA_DIR/marker-bench/integrate`. Short-budget measurements and integration
handoff are in [INTEGRATE_REPORT.md](INTEGRATE_REPORT.md).

Roll usable widths must be measured in the same cuttable, laid-flat frame as
the job fabric width. Folded/tubular physical part expansion belongs to the
cutroom adapter. Measured cutting reports still require physical area/width
and layer counts in the report input, as described by the cutroom crate.

Every planner engine invocation has an unwind boundary. A panic that unwinds
to the caller invalidates that trial and produces a warning; calibration/final
trials retain their earlier checked marker. No partial geometry is reused.
This does not repair the underlying assertion, recover process aborts, or force
a worker that never returns to finish. Those failures need nesting-adapter or
upstream supervision.

`plan` and `plan pipeline` accept `--seed` (default 42) so comparisons can use
an explicit base seed. Refinement uses a valid warm start and also keeps the
incumbent outside the engine. Additional fresh 120-second Simon trials measured
79.60% at seed 42 and 82.25% at seed 1 in this integration environment; these did
not reproduce the earlier approximately 89% engine benchmark. See the report
for the retained complete pipeline result and upstream assertion details.
