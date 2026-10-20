# Cluster strategy measurements

Implementation and experiments started on 19 October 2026. Base: `de68343`
(`marker/mvp`). Only `crates/beta-marker-cluster/` changes in this branch.
The public engine and validator are unchanged.

**Recommendation: keep plain nesting as the production default.** Final
`Strategy::Auto` calls the unchanged engine and independently validates its
result. The former Auto race remains available as explicit `Strategy::Race`.
All tables labelled Auto below measure the former race, not the final default.
No large quality gain or matched-time win over Lectra has been established.
The lead stopped new experiments and requested this partial handoff; only the
already-running development matrix was finished.

## Scope and comparison policy

The Lectra comparison is conditional on the existing Tier B assumptions:
sheet type 0 is an ordinary open rectangular strip, and `flip_x` reflects the
x coordinate before rotation. Tier B includes Tier A and may restrict source
orientations. See [the benchmark policy](../../docs/benchmarks.md) and
[the round-2 handoff](../../bench/results/2026-10-18-round2-handoff.md).
Matching seconds does not match Lectra's unknown hardware or worker count.

Input: `$BETA_DATA_DIR/marker-bench/lectra/sample-300.jsonl`, SHA-256
`d93776b94277eb3095d93d08978407e1fcdce62a360adb6b0e750a96bb5347b0`.
The runner freezes a balanced round-robin order over the 12 strata, sorting
within each by a task-ID hash seeded with 7030786. Development: first 40.
Confirmation: next 100, disjoint from development. Matched duration: first 20
confirmation tasks. Selection does not use nesting outcomes. The exact IDs and
options are in each external run's `run.json`.

Each variant has its own adjacent fresh plain `beta_marker_nest::nest` call.
Order alternates by task and variant. Both use seed 1, two workers, one run and
the same budget. Ties are within 0.1 percentage point (pp). Means use valid
pairs; failures remain in win/tie/loss and are listed separately. Approximate
95% intervals use the sample standard error and 1.96. These sample means are
not population estimates, and a one-seed wall-clock experiment does not prove
an optimum or deterministic replay.

All individual results and independent validation reports stay outside git in
`$BETA_DATA_DIR/marker-bench/cluster/`. No geometry is committed.

## Preliminary construction checks: two-second budgets

These checks used the first three development tasks: 65817, 9443 and 32538.
They ran with three simultaneous task pairs while other agents compiled Rust.
They diagnose startup and construction behaviour. They do not measure the
requested 60-second quality target.

Version 1 used a two-dimensional grid for small-piece reinsertion. Evidence:
`smoke-v1/`; 18 pairs, 42.66 seconds runner wall time. All 36 layouts valid.

| Strategy | Mean vs plain, pp | Win / tie / loss | Mean vs Lectra, pp | Win / tie / loss |
|---|---:|---|---:|---|
| Cluster | -0.9551 | 2 / 0 / 1 | -2.3806 | 1 / 0 / 2 |
| GapFill, free | -0.5704 | 0 / 0 / 3 | -3.3053 | 1 / 0 / 2 |
| GapFill, fixed | -2.7210 | 0 / 0 / 3 | -3.5334 | 1 / 0 / 2 |
| TwoStage, free | -32.1105 | 0 / 0 / 3 | -33.0029 | 0 / 0 / 3 |
| TwoStage, fixed | -14.3362 | 1 / 0 / 2 | -16.5345 | 1 / 0 / 2 |
| Auto | -4.1391 | 0 / 0 / 3 | -5.3225 | 1 / 0 / 2 |

Version 2 replaced the translation grid with monotone blocker-guided sliding.
Evidence: `smoke-v2/`; 18 pairs, 45.91 seconds runner wall time. All 36 layouts
valid. TwoStage still used sampled small-piece insertion in this version.

| Strategy | Mean vs plain, pp | Win / tie / loss | Mean vs Lectra, pp | Win / tie / loss |
|---|---:|---|---:|---|
| Cluster | -3.1025 | 0 / 0 / 3 | -2.6613 | 1 / 0 / 2 |
| GapFill, free | -1.5156 | 0 / 1 / 2 | -1.8319 | 1 / 0 / 2 |
| GapFill, fixed | -3.0800 | 0 / 0 / 3 | -2.7858 | 1 / 0 / 2 |
| TwoStage, free | -37.9303 | 0 / 0 / 3 | -40.5118 | 0 / 0 / 3 |
| TwoStage, fixed | +0.9961 | 2 / 0 / 1 | +0.8310 | 1 / 0 / 2 |
| Auto | -4.6589 | 0 / 0 / 3 | -5.1137 | 1 / 0 / 2 |

The sampled insertion allowance expired with small pieces still unplaced.
Its valid shelf fallback was too long for a subsecond free compression phase
to repair. Version 3 therefore uses the public obstacle packer for TwoStage's
middle phase. Large pieces stay fixed while it completes all small demand;
the final phase then releases or retains the large pieces as requested.
GapFill retains sampled reinsertion and its full-job incumbent. The first two
versions are rejected as two-stage constructors. The difference between their
two-second means is not a controlled quality comparison.

Version 3 uses that public fixed-piece middle phase. Evidence: `smoke-v3/`;
18 pairs, 68.92 seconds runner wall time. All 36 layouts valid. The longer
wall time includes obstacle construction outside the nominal phase budget.

| Strategy | Mean vs plain, pp | Win / tie / loss | Mean vs Lectra, pp | Win / tie / loss |
|---|---:|---|---:|---|
| Cluster | -0.5056 | 1 / 1 / 1 | -1.0658 | 1 / 0 / 2 |
| GapFill, free | -2.6168 | 0 / 0 / 3 | -3.6636 | 1 / 0 / 2 |
| GapFill, fixed | -3.1309 | 0 / 0 / 3 | -3.4753 | 1 / 0 / 2 |
| TwoStage, free | -2.8762 | 1 / 0 / 2 | -3.5608 | 1 / 0 / 2 |
| TwoStage, fixed | +2.0171 | 2 / 0 / 1 | +0.2264 | 1 / 0 / 2 |
| Auto | -2.8468 | 0 / 0 / 3 | -5.2225 | 1 / 0 / 2 |

These are smoke checks. Their settings are frozen for the full development
matrix; the confirmation subset has not been used to choose settings.

## Stopped in-process development run

Revision `36e34cf`; evidence `dev-36e34cf-60s/`. Five pairs ran concurrently,
seed 1, two workers, nominal 60-second search budgets. The host was fully loaded;
workspace builds and other agents' compilation overlapped the start. The host
is an i9-11900H, eight physical cores and 16 logical processors.

The run was stopped at 17:37:47 Asia/Kolkata after 44 complete pairs, all 88
completed layouts valid. Five in-flight pairs were discarded. This prefix is
diagnostic evidence only; it is not the required full 40-task result and is not
used to select a strategy. Its summary reflects unequal task counts.

| Strategy | Complete pairs | Mean vs plain, pp | Win / tie / loss | Mean vs Lectra, pp |
|---|---:|---:|---|---:|
| Cluster | 8 | -0.6409 | 3 / 1 / 4 | -2.0484 |
| GapFill, free | 8 | +0.4168 | 4 / 3 / 1 | -1.3387 |
| GapFill, fixed | 7 | -0.6473 | 0 / 3 / 4 | -1.9648 |
| TwoStage, free | 7 | -1.6948 | 3 / 2 / 2 | -2.4539 |
| TwoStage, fixed | 7 | -4.2856 | 0 / 0 / 7 | -4.6299 |
| Auto | 7 | -1.9862 | 0 / 1 / 6 | -2.9823 |

Task 85876 exposed the native obstacle packer's unbounded first construction:

| Strategy | Plain wall seconds | Variant wall seconds |
|---|---:|---:|
| Cluster | 65.18 | 67.80 |
| GapFill, free | 70.16 | 73.08 |
| GapFill, fixed | 67.24 | 407.02 |
| TwoStage, free | 66.25 | 450.50 |
| TwoStage, fixed | 66.86 | 390.13 |
| Auto | 68.44 | 399.67 |

The public engine checks its deadline between complete constructions. It does
not expose cancellation inside that construction. The wrapper does not patch
it. The benchmark now isolates every call in a child process and gives both
sides the same cap: search budget plus 60 seconds for construction and checking.
Timeouts have no efficiency score and remain failures. Parent validation checks
the original parsed input again after child validation. The new full matrix
uses a fresh output directory and fresh controls; these prefix rows are not
mixed into it. Actual wall times remain part of every final comparison.

Two additional constructions are measured separately from that revision:
exact duplicate aggregation without pairing, and connected pair contours that
leave external concavities available. A Boolean-union coverage regression test
initially rejected a contour; a verified 0.02 mm outward guard fixes the rounding
loss, with a convex-hull fallback if coverage still cannot be certified.

The isolated release runner passed a two-second protocol smoke check on the
same three development tasks. Evidence: `smoke-isolated-v1/`; six pairs,
12.03 seconds runner wall time. All 12 layouts passed child and parent
validation; no timeout or plain fallback occurred. These numbers only verify
the protocol and are not 60-second quality evidence.

| Strategy | Pairs | Mean vs plain, pp | Win / tie / loss | Mean vs Lectra, pp |
|---|---:|---:|---|---:|
| Exact aggregation | 3 | -0.4422 | 0 / 0 / 3 | +2.0872 |
| Contact contours | 3 | -2.0176 | 0 / 0 / 3 | +0.3333 |

## Handoff scope under the lead's time cap

The frozen eight-method executable is built from `a47c95f`, SHA-256
`dffd6a67bc89556988b18675884cccfe80dc38d3ede802d67925a0d7328f774d`.
Raw evidence is in `$BETA_DATA_DIR/marker-bench/cluster/dev-a47c95f-60s/`.
Later commits add the Restart control, input robustness checks, analysis and
the plain Auto default. They do not replace the frozen measured executable.
Restart's smoke executable is `bd2c7e1`, SHA-256
`620ce412ffdb972ca1bf4018086d8c1d5afe789c039e9199875b31a7f79598e5`.

The lead requested no new experiments, completion of the already-running paired
batch, and a report within 60 minutes. The remaining protocol cells are partial
or unrun, as listed below. They cannot support a confirmation claim.

| Study | Status | Interpretation |
|---|---|---|
| Eight-method Lectra development, selected 40, 60 s | 320 raw pairs finished; partial 312 evaluable | Task 9644 excluded uniformly due to timing interruption |
| Restart-only Lectra development, 40, 60 s | Not run | Only three-task two-second smoke exists |
| FreeSewing simon/teagan/titan ratios, 60 s | Not run | Short M-fixture integration tests are not this study |
| FreeSewing simon/teagan/titan ratios, 120 s | Not run | No duration comparison available |
| Disjoint Lectra confirmation, 100, 60 s | Not run | No independently confirmed strategy |
| Lectra matched recorded duration, 20, about 300 s | Not run | No matched-time claim against Lectra |
| Interrupted task 9644 recheck, all eight methods | Not run | New experiments stopped; original evidence retained |

Task 9644 has five recorded trial walls of about 4,903–4,945 seconds despite
the supervisor's nominal 120-second cap. A concurrent Cargo check also showed
an approximately 84-minute wall interval. This is an unexplained host or
orchestration timing interruption, not reliable algorithm-runtime evidence.
No cause such as machine sleep has been established. All eight task pairs are
excluded together, based on that timing anomaly, regardless of efficiency.
The raw records remain in `dev-a47c95f-60s/`; no rows are replaced or rescored.
The final analysis therefore remains **partial: 39 of 40 selected tasks**.

Background CPU load was uncontrolled, with other agents building and searching.
Five pairs ran concurrently, each trial using two workers. Own workspace checks
also overlapped parts of development. Paired alternating order reduces order
bias but cannot remove shared-load noise. Report actual wall times, and do not
interpret nominal seconds as equal CPU work or equal Lectra hardware.

## Final paired development results: partial 39/40

All **320 raw pairs finished** in the existing batch (runner wall time
13,292.93 s, including the timing interruption). Excluding every method of
task 9644 leaves **312/320 evaluable pairs: 39/40 tasks per strategy**.
No recheck or new experiment was started after the lead's stop instruction.
These partial development results are not confirmation evidence.

| Strategy | Valid pairs / attempts | vs plain mean pp | 95% approx. half-width pp | W / T / L | vs Lectra mean pp | W / T / L | Failed plain / variant | Timeout plain / variant | Mean wall plain / variant, s |
|---|---:|---:|---:|---|---:|---|---|---|---|
| aggregate | 39 / 39 | +0.0461 | +0.5195 | 14 / 11 / 14 | -2.8666 | 7 / 2 / 30 | 0 / 0 | 0 / 0 | 60.45 / 62.16 |
| auto | 30 / 39 | -2.5045 | +0.7666 | 0 / 6 / 33 | -5.8668 | 5 / 2 / 32 | 0 / 9 | 0 / 9 | 60.26 / 74.35 |
| cluster | 39 / 39 | -0.1311 | +0.4485 | 12 / 11 / 16 | -2.9832 | 9 / 2 / 28 | 0 / 0 | 0 / 0 | 60.98 / 61.41 |
| cluster-contour | 39 / 39 | -0.7742 | +0.5075 | 9 / 8 / 22 | -3.5208 | 7 / 2 / 30 | 0 / 0 | 0 / 0 | 60.76 / 62.73 |
| gap-fill | 39 / 39 | +0.1835 | +0.3208 | 17 / 15 / 7 | -2.8875 | 9 / 3 / 27 | 0 / 0 | 0 / 0 | 61.06 / 61.37 |
| gap-fill-fixed | 30 / 39 | -0.3963 | +0.4768 | 5 / 11 / 23 | -3.7130 | 6 / 3 / 30 | 0 / 9 | 0 / 9 | 60.67 / 77.34 |
| two-stage | 30 / 39 | -0.8679 | +1.7945 | 13 / 7 / 19 | -4.5114 | 5 / 2 / 32 | 0 / 9 | 0 / 9 | 60.31 / 74.35 |
| two-stage-fixed | 30 / 39 | -1.8779 | +2.2244 | 11 / 8 / 20 | -5.5818 | 4 / 1 / 34 | 0 / 9 | 0 / 9 | 60.74 / 77.17 |

The historical `auto` row is the legacy race, now `Strategy::Race`.
Mean differences and approximate intervals use valid pairs only. Single-sided
timeouts count as losses in W/T/L; there are no both-failed pairs after the
whole-task exclusion. Lectra means use valid variant results, so methods with
timeouts have different scoring subsets. All accepted layouts passed child
and parent core validation. Every plain control in the evaluable set is valid.

GapFill has the largest observed mean gain, **+0.1835 pp**, with an approximate
95% half-width of **0.3208 pp** and W/T/L **17/15/7**. Its interval spans zero;
the requested **+1.5 pp** is not demonstrated. Exact aggregation is essentially
neutral; pair hulls are slightly worse; contours are worse. Each fixed,
two-stage and race method has **nine variant timeouts out of 39**. Their means
exclude failures and cannot justify a default. No variant beats Lectra on
average here; matched-duration comparisons were not run.

Cluster and ClusterContour each formed active pairs on **17/39** tasks; no
candidate-failure plain fallback was recorded. The remaining tasks did not
form a qualifying compact pair. Construction and refinement therefore need to
be distinguished from simply replaying a different engine search. Restart's
only smoke result is reported separately below.

**Decision:** keep all wrappers experimental and keep Auto/plain as the
production default. Do not promote the development sample's small GapFill
lead without disjoint confirmation. Neither target is achieved in this report.

Geometry-free detailed tables, including every stratum and run-order counts,
are saved as `$BETA_DATA_DIR/marker-bench/cluster/final-handoff-analysis.md`.
Reproduce the analysis with the current Rust example:

```text
cargo run --manifest-path crates/beta-marker-cluster/Cargo.toml --example analyze --offline -- <report.md> <dev-a47c95f-60s> <smoke-restart-bd2c7e1> --exclude-task 9644
```

## Sources and licences

Pair pre-placement is motivated by Elkeran (2013), [A new approach for sheet
nesting problem using guided cuckoo search and pairwise clustering](https://doi.org/10.1016/j.ejor.2013.06.020).
The sampled slide search, conservative envelopes and thresholds here are an
independent heuristic, not a reproduction of the full paper. Free refinement
uses the unchanged engine described by Gardeyn, Vanden Berghe and Wauters,
[An open-source heuristic to reboot 2D nesting research](https://arxiv.org/abs/2509.13329).

Lectra data: Lallier, Vézard, Pinaud and Blin (2022), [Nesting Tasks Dataset for
2D-Nesting Efficiency Estimation](https://doi.org/10.5281/zenodo.7030786), v1.1,
CC BY 4.0. Converted jobs use mm and may restrict orientations. FreeSewing
fixtures are MIT, with attribution in their existing README.

No new third-party package is introduced. Direct dependencies resolved by
`cargo metadata --offline`: anyhow 1.0.104, serde 1.0.229, serde_json 1.0.154,
geo 0.33.1 and rand 0.10.3, all MIT OR Apache-2.0. They are already in the root lockfile.
Sparrow 0.3.0 remains MIT; Jagua 1.0.0 remains its unmodified MPL-2.0 dependency
under the project's existing exception. The generated standalone lockfile is
ignored and is not committed.

## Verification

Environment: assigned external Cargo target directory, `CARGO_BUILD_JOBS=3`.
All search tests use two workers and budgets at most 0.5 seconds; direct pair
geometry checks allow 0.5 seconds. The public API integration check reads the
three existing FreeSewing M fixtures and validates every result independently.

```text
cargo test --release --manifest-path crates/beta-marker-cluster/Cargo.toml --offline -- --test-threads 2
cargo clippy --release --manifest-path crates/beta-marker-cluster/Cargo.toml --all-targets --offline -- -D warnings
```

Release checks: 11 library tests, two runner tests and one garment integration
test passed; no failures or ignored tests. The three-fixture integration test
took 4.72 seconds on the loaded host. Release Clippy passed without warnings.
No TypeScript files changed.

The earlier in-tree checks after adding contour coverage and isolated trials:

```text
cargo test --workspace --offline -- --test-threads 2
cargo clippy --workspace --all-targets --offline -- -D warnings
cargo test --manifest-path crates/beta-marker-cluster/Cargo.toml --offline -- --test-threads 2
cargo clippy --manifest-path crates/beta-marker-cluster/Cargo.toml --all-targets --offline -- -D warnings
cargo build --release --manifest-path crates/beta-marker-cluster/Cargo.toml --bin paired_bench --offline
```

Workspace: **231 passed, four existing tests ignored**; strict Clippy passed.
The new crate is intentionally outside the member list pending lead integration,
so it is checked separately: **17 passed, one ignored helper**, strict Clippy
passed. The ignored five-second sleeper is launched by the passing supervisor
test and killed after 100 ms. The three-garment integration test took 2.49 s.
The current release runner build passed. Source computation remains Rust only.

## Restart-only ablation

After the first 20 tasks of the frozen eight-method matrix, GapFill's current
paired gain was +0.2088 pp with an approximate 95% half-width of 0.3798 pp.
This prompted one additional control, implemented for a separate study on the
same 40-task development selection: keep GapFill's small-piece eligibility and 55%
first free search, omit reinsertion, then spend the remaining wall time on a
free warm start. Both calls use the same base seed. This tests the cost and
benefit of reinsertion against staged warm search. No confirmation outcome was
used to design it. The frozen eight-method matrix remains unchanged. Only its
three-task two-second smoke ran before the lead's time cap; the 40-task study
was not started. Evidence: `smoke-restart-bd2c7e1/`, three valid pairs, all six
layouts independently valid. Mean versus plain: **-2.9569 pp**, approximate
95% half-width 4.2804 pp, W/T/L **0/0/3**. Mean versus Lectra: -3.1070 pp,
W/T/L 1/0/2. This is functional evidence, not a 60-second quality measurement.

The new control adds a warm-incumbent regression test. Robustness checks also
reject unusable effective geometry after extreme shrinkage, preserve the
shrinkage warning, and ignore zero-demand pieces when setting the small-piece
area threshold. The current crate check has **19 passed, one ignored helper**;
strict Clippy passes. Builds encountered long shared-host/cache waits, so runner
wall time must not be treated as isolated solver throughput. Per-trial wall
times and failures are reported separately.

## Final implementation checks and integration requests

Commit `c79fa77` makes Auto use plain nesting and keeps the measured legacy
race as explicit `Strategy::Race`. All other experimental strategies remain
opt-in. No experimental default is promoted from this incomplete study.

Final commands, from `marker/`, with the assigned external target directory,
`CARGO_BUILD_JOBS=3` and `BETA_DATA_DIR` set:

```text
cargo fmt --manifest-path crates/beta-marker-cluster/Cargo.toml
cargo test --manifest-path crates/beta-marker-cluster/Cargo.toml --offline -- --test-threads 2
cargo clippy --manifest-path crates/beta-marker-cluster/Cargo.toml --all-targets --offline -- -D warnings
git diff --check
```

Result: **19 passed, zero failed, one ignored timeout helper**. Library: 15;
runner: three; garments: one. The ignored helper is exercised and terminated
by the passing supervisor test. All strategy results in the tests pass the
independent validator. The three-fixture FreeSewing test took 3.57 s; all search
budgets are at most 0.5 s and use two workers. Strict Clippy and formatting
passed. One earlier test invocation picked up temporary helper examples while
they were being removed; the clean rerun above passed. No temporary helper
source or geometry is included in the commit.

Workspace checks on this branch also passed: `cargo test --workspace --offline
-- --test-threads 2` (231 passed, four existing ignored) and `cargo clippy
--workspace --all-targets --offline -- -D warnings`. The new crate has its own
workspace table, so its separate checks above are required. The source JSONL
SHA-256 was rechecked unchanged before handoff.

Requests for the lead:

- Add `crates/beta-marker-cluster` to the root members; remove its standalone
  `[workspace]` and local profiles. Add the CLI path dependency and regenerate
  the root lockfile. No core model or public engine contract change is needed.
- Wire `beta-marker nest --strategy auto|plain|cluster|two-stage` as proposed
  in README. Keep Auto/plain as the default. Experimental methods need explicit
  selection. The historical measured `auto` means Race in the final API.
- After integrating the quality agent's engine, rerun workspace and wrapper
  checks. Performance evidence here uses base `de68343`, not concurrent tuning.
  Rebenchmark before promoting any wrapper. Remaining studies are unrun.

Request for the quality agent: check deadlines inside obstacle construction
and its expensive placement loops, preserve the best independently valid full
incumbent on cancellation, and make that behaviour available through the
existing public entry point. Fixed refinement and large-first construction
otherwise exceed wrapper phase allowances before control returns. The wrapper
preserves the engine's soft-budget contract; only the benchmark has a child
process wall cap. No engine internals were modified here.
