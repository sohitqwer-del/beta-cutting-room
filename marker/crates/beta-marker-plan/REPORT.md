# Cut planning and batch implementation report

Measured on 19 October 2026, on branch `marker/plan`, based on contract commit
`a6be23a`. No other agent branches were merged. Only the planner crate and the
two assigned CLI command files were changed. No core contract changes are needed.

## What works

- Candidate ratios, cheap area estimates, bounded refinement, and a per-call
  ratio cache. Only real, independently validated nests become cutting lays.
- Integer ratio/ply optimisation with quantity limits, maximum plies, per-ply
  end allowances, and a configurable lay cost. Feasible single-ratio plans are
  included as incumbents, alongside greedy solutions. Branch-and-bound uses
  memoisation, feasible size prices, and fabric and lay-count lower bounds.
- Explicit ordered/planned balances, over-cut, shortfalls, search limits,
  runtime, costing options, fabric totals, and marker results in the JSON.
- `plan` prints both requested tables and writes incomplete plans before
  returning a failure for a shortfall.
- `batch` supports folders, text lists, width trials, repeated seeds, bounded
  concurrency, independent validation, saved width-adjusted jobs, result JSONs,
  CSV/Markdown/JSON summaries, and best widths by fabric area. Failed trials
  remain visible and do not stop the rest of the queue.

## Simon measurements

Source: `testdata/open/freesewing/simon-ratio.job.json`, FreeSewing 4.10.2,
MIT, Joost De Cock and FreeSewing contributors. No new dataset was committed.
Both orders use 1,500 mm fabric width, at most 80 plies and six garments per
marker. End allowances are zero; lay cost is 0.5 m. Each planning call retains
up to 128 candidate ratios, evaluates ten real markers with two workers and a
0.3 s search budget per marker, and allows 5,000 integer search states per pass.
The base seed is 42.

These are short-nest smoke measurements in the optimised debug profile on a
shared machine. Construction, repair, validation, and integer search add to
the nesting search budget. Build time is excluded. Both plans reached the
integer search limit, and candidate ratios were pruned. Neither result proves
global optimality. Longer nests and larger search limits can change the result.

### Order: S 200, M 400, L 400, XL 200

| Lay | Ratio | Plies | Marker (m) | Efficiency | Fabric (m) |
|---|---|---:|---:|---:|---:|
| 1 | S1 M2 L2 XL1 | 80 | 8.092 | 80.68% | 647.397 |
| 2 | S1 M2 L2 XL1 | 80 | 8.092 | 80.68% | 647.397 |
| 3 | S1 M2 L2 XL1 | 40 | 8.092 | 80.68% | 323.698 |

Total physical fabric: **1,618.492 m**. Cost objective: **1,619.992 m**.
Runtime: **154.869 s**. Ten distinct markers were evaluated; repeated lays reuse
the same cached marker. All size quantities are met exactly.

| Size | Ordered | Planned | Extra | Missing |
|---|---:|---:|---:|---:|
| S | 200 | 200 | 0 | 0 |
| M | 400 | 400 | 0 | 0 |
| L | 400 | 400 | 0 | 0 |
| XL | 200 | 200 | 0 | 0 |

### Awkward order: S 37, M 113, L 96, XL 41

Allowed over-cut is 5% per size, rounded down. Upper quantities are S 38,
M 118, L 100, and XL 43.

| Lay | Ratio | Plies | Marker (m) | Efficiency | Fabric (m) |
|---|---|---:|---:|---:|---:|
| 1 | S1 M2 L2 XL1 | 37 | 8.092 | 80.68% | 299.421 |
| 2 | M2 L1 | 20 | 4.038 | 79.90% | 80.758 |
| 3 | L3 XL3 | 1 | 8.439 | 82.44% | 8.439 |
| 4 | XL1 | 1 | 1.821 | 65.71% | 1.821 |

Total physical fabric: **390.439 m**. Cost objective: **392.439 m**.
Runtime: **78.728 s**. Ten distinct markers were evaluated. There is no shortfall.

| Size | Ordered | Planned | Extra | Missing |
|---|---:|---:|---:|---:|
| S | 37 | 37 | 0 | 0 |
| M | 113 | 114 | 1 | 0 |
| L | 96 | 97 | 1 | 0 |
| XL | 41 | 41 | 0 | 0 |

### Comparison with one repeated ratio

The naive plan repeats S1 M2 L2 XL1 until every size is covered, splitting
plies at 80. Its independently validated marker is 8.092461 m long and 80.68%
efficient. It was nested once in **17.986 s** and reused for both comparisons.

| Order | Planner fabric (m) | Naive fabric (m) | Naive lays | Naive within over-cut? |
|---|---:|---:|---:|---|
| 1,200 shirts | 1,618.492 | 1,618.492 | 3 | Yes |
| Awkward | 390.439 | 461.270 | 1 | No |

For the balanced order the planner keeps the simple three-lay plan. For the
awkward order the naive plan needs 57 plies and cuts S 57, M 114, L 114, XL 57:
55 extra garments. S, L, and XL exceed their limits. The planner uses
**70.831 m less fabric (15.36%)** and meets the allowed quantity limits. The
naive awkward plan is therefore a useful comparison, but is not a feasible
alternative under the requested constraints.

The final ignored test passed in **252.18 s**, including both plans, the naive
marker, repeated validation, and artifact writing. An earlier exploratory run
was stopped while improving search bounds and single-ratio incumbents; its
partial figures are excluded from the tables above.

## Reproduction and checks

Run Cargo from the `marker` workspace. Set `CARGO_TARGET_DIR` to the assigned
external build folder, `CARGO_BUILD_JOBS=3`, and `BETA_DATA_DIR` to the data root.
The example generator and ignored test write generated orders and results to
`$BETA_DATA_DIR/marker-bench/plan/`. This folder also holds the full logs.

```sh
cargo test -p beta-marker-plan -p beta-marker-cli
cargo test -p beta-marker-plan
cargo test -p beta-marker-plan simon_real_plans -- --ignored --nocapture
cargo test --workspace
cargo clippy --workspace --all-targets
```

The focused tests pass: 17 CLI tests and eight fast planner tests. One real-nest
test is ignored by default and passed when run explicitly. Synthetic tests
cover ply splitting, over-cut, allowances, lay cost, incomplete length tables,
bounded search, single-ratio incumbents, and an independent exhaustive dynamic
program over 50 small quantity/cost cases. Batch tests cover summary escaping,
best width by area, relative and duplicate list entries, UTF-8 BOM handling,
folder filtering, and failed-trial preservation.

Final workspace results: **80 passed, zero failed, three ignored**; all doc tests
passed. `cargo clippy --workspace --all-targets` passed with **zero warnings**.
`cargo build -p beta-marker-cli` passed. `git diff --check` passed.

The actual CLI was also exercised:

```sh
beta-marker plan "$BETA_DATA_DIR/marker-bench/plan/cli-smoke.cutorder.json" --time-per-marker 0.1 --lay-cost 0.5 --max-markers 6 --search-nodes 1000 -o "$BETA_DATA_DIR/marker-bench/plan/cli-smoke.cutplan.json"
beta-marker batch "$BETA_DATA_DIR/marker-bench/plan/batch-jobs.txt" --widths 1450,1500 --time 0.1 --runs 2 --jobs 1 -o "$BETA_DATA_DIR/marker-bench/plan/batch"
beta-marker batch "$BETA_DATA_DIR/marker-bench/plan/batch-jobs.txt" --widths 1450 --time 0.1 --runs 1 --jobs 2 -o "$BETA_DATA_DIR/marker-bench/plan/batch-parallel"
```

The CLI plan smoke order uses `examples/jobs/shirts-sml.json`, quantities S 7,
M 11, L 5, maximum five plies and three garments per marker. It returned all
quantities exactly, four lays, 15.194 m of fabric, and a 1.878 s runtime. Both
requested tables printed, and the saved plan was complete.

The batch list contains `examples/jobs/gaps-buffers.json` and
`examples/jobs/shirts-sml.json`. The two-width run returned **four valid trials,
eight successful runs, zero failures**, and wrote the result/job pairs and all
four summaries. Trial runtime includes both independent runs and checking.

| Job | Width (mm) | Length (m) | Efficiency | Runtime (s) | Best width? |
|---|---:|---:|---:|---:|---|
| gaps-buffers | 1450 | 1.116 | 63.27% | 0.353 | Yes |
| gaps-buffers | 1500 | 1.121 | 60.85% | 0.356 | No |
| shirts-sml | 1450 | 2.786 | 73.92% | 1.233 | No |
| shirts-sml | 1500 | 2.666 | 74.67% | 1.136 | Yes |

The separate two-worker queue smoke run uses `--jobs 2` (two concurrent nests,
each with two engine workers). It returned two valid trials with no failures.

## Dependencies and merge checks

No dependencies were added and no licence exceptions were introduced. Existing
direct dependencies: `anyhow`, `serde`, and `serde_json` (MIT OR Apache-2.0),
internal marker crates (Apache-2.0), and the CLI's existing `clap` (MIT OR
Apache-2.0). The existing unmodified Sparrow/Jagua dependency chain is unchanged.

The current matching crate is a stub for stripe/check matching and shade bands;
ordinary jobs dispatch to the nesting engine. Requested matching/shade jobs
produce explicit failed trials or unavailable-ratio warnings until that agent's
implementation is merged. Planner and batch commands do not silently discard
these rules.

After merging `nest2`, `match`, and `qa2`, rerun the workspace tests, clippy, both
Simon plans, and batch width/run trials. Also test cut orders using matching,
shade, folded/tubular fabric, shrinkage, defects, blocks, and bundle rules against
the merged independent validator. Marker lengths and runtime may change with
the new engines and rules. Persistent caches across calls and a proof across
all candidate ratios are not provided. No contract changes are requested.
