# External nesting strategies

This crate wraps `beta_marker_nest::nest`. It does not change Sparrow, Jagua or
the existing adapter. Every successful `nest_clustered` call passes
`beta_marker_core::validate` against the original job.

```rust
use beta_marker_cluster::{nest_clustered, Strategy};
use beta_marker_nest::NestOptions;
# fn example(job: &beta_marker_core::Job) -> anyhow::Result<()> {
let result = nest_clustered(job, &NestOptions::default(), Strategy::default())?;
# Ok(())
# }
```

`Strategy::default()` is `Auto`, which currently uses plain nesting. The partial
development study did not establish a gain that warrants a new default. See
[RESULTS.md](RESULTS.md). The former Auto race remains explicit as `Race`.

## Strategies

| Strategy | Construction | Final search |
|---|---|---|
| Plain | Existing engine | Existing engine |
| Cluster | Sample pair contacts; nest convex envelopes; expand members | Free warm start |
| ClusterContour | Score the same pairs; join their outlines with a narrow conservative bridge | Free warm start |
| Aggregate | Combine exact duplicate demands into counted types; no relative pair placement | Full grouped search |
| Restart | GapFill's full-job free search with 55% of the budget | Free warm start with the time left |
| GapFill | Nest the full job; remove and reinsert small pieces | Free warm start |
| GapFillFixed | Same reinsertion | Large pieces fixed |
| TwoStage | Nest large pieces; insert small pieces | Free warm start |
| TwoStageFixed | Same two-stage construction | Large pieces fixed |
| Race | Race plain, cluster and two-stage | Free warm start from the best |
| Auto | Plain nesting pending independent confirmation | Existing engine |

Pairs use allowed member orientations, full-outline intersection and Euclidean
distance tests. Contacts are sampled by sliding along both axes and refining
the first sampled collision boundary. Pair scoring combines useful area divided
by convex hull area (65%) and by bounding box area (35%). A pair must reduce the
sum of separate bounding box areas by at least 6%, use at least 82% of its hull,
and fit the fabric width. These are heuristic thresholds, not geometric bounds.
Repeated pair types share a synthetic piece type. Each physical copy retains an
explicit member map, including its original copy number, mirror state and bundle.
The envelope allows only rotations legal for all members. Pair tilt is zero;
single pieces retain their allowed tilt. A free member can turn freely only
when every member of that composite allows it.

Pieces with different IDs share a synthetic type only when their full geometry,
mirror state, buffers and allowed orientations match exactly. Every ID still
has its own expansion record. No rounded geometry hash decides equivalence.

Aggregate uses that grouping without forming pairs, and preserves each singleton's
original rotation rule and tilt. ClusterContour uses the same pair search and
score as Cluster. It joins the two outlines with a 0.04 mm-wide bridge between
their closest boundaries. Boolean union must produce one valid simple exterior
that covers both members; otherwise it keeps the convex hull. Any enclosed holes
are conservatively filled. The bridge is only a collision envelope. Expansion
returns the original cut lines and their true area, without a bridge to cut.

Restart is an ablation for staged warm search. It forms no clusters and removes
no small pieces. It uses GapFill's small-piece eligibility rule and first-phase
allowance of 55% of the budget; the second
keeps that valid result as a warm start and uses the wall time left. Both phases
use the same seed. It separates restart effects from reinsertion effects in
GapFill; their phase allowances differ because Restart has no insertion cost.

The convex hull covers both members and all space between them. Its buffer is
the largest member buffer. This is conservative for external gaps and edges.
It can reserve useful empty space inside the hull. Free compression releases
that space after expansion. Pair generation receives up to 8% of the budget,
capped at three seconds; envelope nesting receives 70% of the remaining time.

Small pieces have at most 20% of the largest active piece's area. Insertion tries
sampled width positions, bounding edges and vertex differences. It tests the
full polygons and requested gaps with a 0.02 mm numerical guard. It minimizes
the inserted piece's right edge and then its bottom edge. This samples gaps and
concave pockets; it does not represent polygon holes or enumerate all maximal
empty regions. A disjoint shelf at the end is a valid fallback. Large-first or
full-job construction gets 55% of the budget, insertion gets at most 12%, and
final compression gets the time left. An inferior reinsertion cannot replace
the full-job incumbent in GapFill.

At each sampled width position, insertion starts at the left edge. On collision
it bisects towards a position beyond that obstacle, then checks every obstacle
again. Translations increase monotonically. This avoids evaluating an entire
two-dimensional translation grid, though it can skip narrow feasible intervals.

TwoStage instead uses the public engine's obstacle packer for its middle phase:
large pieces are fixed while all small demand is constructed around them. Its
complete result becomes the warm start for the free or fixed final phase. This
avoids giving free compression a long shelf of uninserted small pieces when a
very short sampled-insertion budget expires.

Race gives each candidate 20% of the budget, or 24% when demand exceeds 150
pieces. The remaining time refines the shortest valid result. Jobs with fewer
than four pieces or budgets under 0.5 seconds use plain nesting. All phases use
the requested seed and worker limit. Import, first construction and final checks
can exceed a phase allowance, as in the existing engine. Time is deducted between
phases. The options do not promise a strict end-to-end deadline.

If no seed is supplied, the wrapper chooses one once, uses it for all phases,
and records it in the result.

Advanced constraints use the original engine: fixed placements, layered fabric,
one-way-per-bundle nap, faceless flipping, defects, shade, repeat/match rules and
fusing blocks. Their rules are preserved. Shrinkage is applied once before
transformation and validated against the original input. Failed variants fall
back to plain nesting with the time left; the result records the failure.
This wrapper does not implement search for those advanced constraints itself.

## Benchmark runner

Run from `marker/`, with `CARGO_TARGET_DIR`, `CARGO_BUILD_JOBS=3` and
`BETA_DATA_DIR` set. Output must be outside the checkout.

```text
cargo test --manifest-path crates/beta-marker-cluster/Cargo.toml
cargo clippy --manifest-path crates/beta-marker-cluster/Cargo.toml --all-targets -- -D warnings
cargo build --release --manifest-path crates/beta-marker-cluster/Cargo.toml --bin paired_bench
paired_bench <sample-300.jsonl> <external-output> dev 60 cluster,gap-fill,gap-fill-fixed,two-stage,two-stage-fixed,race,aggregate,cluster-contour 3 1
paired_bench <sample-300.jsonl> <external-output> confirm 60 <selected-strategy> 3 1
paired_bench <sample-300.jsonl> <external-output> matched 300 <selected-strategy> 3 1
paired_bench <testdata/open/freesewing> <external-output> freesewing 60 <strategies> 3 1
paired_bench <testdata/open/freesewing> <external-output> freesewing 120 <strategies> 3 1
```

Set `BETA_CLUSTER_REVISION` to the tested commit when measuring or resuming.
Each experiment records its implementation revision. Do not mix revisions in
one output directory. Copy the tested executable to a revision-specific external
path before a long run, so Windows does not hold the build output open.
Historical revision `a47c95f` named the race `auto`; current `auto` uses plain
nesting. Reproduce the recorded study with its archived binary and manifest,
or use `race` for a new study with the current API.

`matched` uses each task's recorded Lectra duration; the positional seconds are
recorded but do not cap it. The runner selects the subsets before any search.
It sorts each stratum by a fixed hash of task ID (seed 7030786), then takes tasks
round-robin over strata. Development uses the first 40; confirmation the next
100; matched uses the first 20 confirmation tasks. Development and confirmation
are disjoint. Selection is independent of nesting outcomes.

Each variant has a fresh adjacent plain run, using the same task, seed, two
workers and time budget. The order alternates across tasks and variants. Up to
three task pairs run concurrently by default. Background load remains a source
of noise. Results include paired differences, win/tie/loss (0.1 point tolerance),
failures and a normal-approximation confidence interval. They describe these
samples, not the full population. Interrupted runs resume only completed pairs,
and refuse an incompatible manifest. Individual JSON evidence stays external.

Each trial runs in its own supervised child process. The same cap applies to
plain and variant: search budget plus 60 seconds for construction and checking.
A call that still runs after the cap is terminated, reaped and recorded as a
timeout with no score. Means exclude failed pairs; single-sided failures count
as a loss or win, and both-failed pairs are listed separately. Parent-side
validation checks every accepted result against the original parsed job, in
addition to the child's independent check. Saved request, result, validation,
stderr and trial-summary files make timeout and coordinate-round-trip audits
possible. The cap is a benchmark safeguard; the library retains the public
engine's soft-budget contract.

## Lead integration

Remove this crate's empty `[workspace]` table and local profiles, then add
`crates/beta-marker-cluster` to the root workspace members. Add a path dependency
to the CLI. Map `--strategy auto|plain|cluster|two-stage` to the corresponding
`Strategy` and call `nest_clustered(&job, &opts, strategy)`. Keep other existing
CLI options in `NestOptions`. The two fixed variants and GapFill can remain
research controls until their results justify exposing them. Regenerate the
workspace lockfile after integration. No contract-field change is needed.

## Sources and licences

- Ahmed Elkeran (2013), [A new approach for sheet nesting problem using guided
  cuckoo search and pairwise clustering](https://doi.org/10.1016/j.ejor.2013.06.020).
  It motivates reducing the problem by pairing compatible shapes. This crate's
  sampled contact search and thresholds are an independent implementation, not
  a reproduction of its complete algorithm.
- Gardeyn, Vanden Berghe and Wauters, [An open-source heuristic to reboot 2D
  nesting research](https://arxiv.org/abs/2509.13329), v4. Sparrow motivates the
  free final separation/compression step. The wrapper calls the existing engine.
- Lallier, Vézard, Pinaud and Blin (2022), [Nesting Tasks Dataset for 2D-Nesting
  Efficiency Estimation](https://doi.org/10.5281/zenodo.7030786), v1.1, CC BY 4.0.
  See the existing benchmark policy for sheet-type and orientation assumptions.
  No dataset geometry is committed. FreeSewing fixtures are MIT; their source
  attribution remains in `testdata/open/freesewing/README.md`.

Direct third-party crates: anyhow 1 (MIT OR Apache-2.0), serde 1 (MIT OR
Apache-2.0), serde_json 1 (MIT OR Apache-2.0), geo 0.33 (MIT OR Apache-2.0).
The seed source is rand 0.10 (MIT OR Apache-2.0).
All are already used by the existing workspace. No new third-party package is
introduced. Sparrow remains MIT and Jagua remains its unmodified MPL-2.0
dependency, under the project's existing exception. Product code has no runtime
network access.

## Saved-result analysis

```text
cargo run --release --manifest-path crates/beta-marker-cluster/Cargo.toml --example analyze -- <report.md> <run-directory>...
```

The analyzer reads pair metadata only. It produces method, stratum and style
tables, completion and timeout counts, actual wall times and matched-versus-short
time effects on shared task IDs. It marks an incomplete matrix as ineligible for
selection. It does not load or include job geometry in the report. Repeat
`--exclude-task ID` to exclude all methods of an interrupted task. Exclusions
are printed explicitly and leave the selected matrix incomplete; raw records
are never modified. They must be justified before inspecting efficiency scores.
