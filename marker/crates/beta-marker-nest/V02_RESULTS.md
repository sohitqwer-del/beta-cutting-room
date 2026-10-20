# Nesting 0.2: implementation and measured results

This is a historical measurement report. The launch update's bounded obstacle
search, cancellation API and new measurements are in [LAUNCH_FIX.md](LAUNCH_FIX.md).
In particular, the old construction-before-deadline limitation described below
does not describe the updated constrained path.

All requested engine paths and CLI controls are implemented on `marker/nest2`.
The branch includes qa2 `d4c6a82`, merged after its recorded passing tests and
Clippy checks. Every current BETA layout below passed `beta_marker_core::validate`
with no errors or validation warnings. Expected result notes identify blocks,
shrinkage, portfolio settings and the obstacle packer.

## Feature checks

Input: `examples/jobs/gaps-buffers.json`, 1200 mm wide, 2 s search budgets.
Fixed and warm tests first create a separate 2 s previous result. That preparation
is excluded from the reported re-nesting runtime. Library tests also check S1 M2
bundle IDs 0, 1 and 2, faceless layered counts, surplus layers, exact fold edges,
and a locked full cut replacing a suggested fold half.

| Feature/test | Efficiency | Length mm | Runtime s | Valid |
|---|---:|---:|---:|---|
| Baseline | 74.03% | 1152.19 | 2.14 | Yes |
| Fixed: 2 of 6 placements locked | 74.17% | 1150.02 | 2.01 | Yes |
| Native warm start | 74.14% | 1150.56 | 2.28 | Yes |
| Defect: 200 × 240 mm zone | 65.62% | 1300.00 | 2.01 | Yes |
| Bundles / one-way-per-bundle, 3 runs | 69.41% | 1228.86 | 2.14 | Yes |
| Faceless, 5 allocations | 74.16% | 1150.23 | 2.26 | Yes |
| Fusing blocks, 5 mm margin | 65.69% | 1298.50 | 2.33 | Yes |
| Shrinkage, 2% length / 1% width | 74.83% | 1174.28 | 2.21 | Yes |

The wider construction-order sampling improved the defect fixture from
57.35% / 1487.46 mm to 65.62% / 1300.00 mm. Fusing efficiency uses useful
pattern area over the fabric occupied by the block cuts; block waste is excluded
from the numerator.

## Simon portfolio, 60 s

Same job, base seed 1, independent seeds, concurrent runs. Two retained Sparrow
phase pools and each run coordinator count towards the 16-thread limit.
The single run keeps three separator workers; three runs use two each; five use
one each. Every run receives the same 60 s search budget on this host.

| Runs | Workers/run | Efficiency | Length mm | Runtime s | CPU mean / peak | Valid |
|---:|---:|---:|---:|---:|---:|---|
| 1 | 3 | 89.02% | 7334.26 | 61.73 | 27.21% / 49.91% | Yes |
| 3 | 2 | 88.77% | 7354.86 | 62.21 | 43.17% / 58.43% | Yes |
| 5 | 1 | 88.24% | 7399.02 | 63.86 | 41.71% / 57.29% | Yes |

The single run won this experiment. More seeds did not compensate for fewer
workers per seed. This is one base-seed comparison, not a claim that a portfolio
always improves quality.

## Garments, layers and shrinkage

All Teagan variants keep the same requested garment demand. The fold variants
add symmetry lines to the full front/back and halve the spread width; sleeves
remain pairs. Folded fabric has one fold edge, while tubular fabric has two.
Their edge constraints reduce packing freedom. Efficiencies use the occupied
spread's useful outline area and are not doubled for the second layer.

| Job/layout | Width mm | Budget s | Efficiency | Length mm | Runtime s | Valid |
|---|---:|---:|---:|---:|---:|---|
| Teagan open | 1500 | 60 | 78.56% | 4568.77 | 60.78 | Yes |
| Teagan tubular | 750 | 60 | 69.83% | 5139.96 | 65.09 | Yes |
| Teagan folded | 750 | 60 | 40.90% | 8774.59 | 60.94 | Yes |
| Simon baseline | 1500 | 120 | 89.24% | 7316.13 | 121.42 | Yes |
| Simon 2% / 1% shrinkage | 1500 | 60 | 89.06% | 7551.98 | 61.50 | Yes |

The broader order sampling improved tubular Teagan from 66.15% / 5425.67 mm
to 69.83% / 5139.96 mm. Folded Teagan did not improve beyond its first construction.
These are feasible heuristic results, not proven optima. The shrinkage result
explicitly refers to `rules::effective_job(job)`.

## Version 0.1 regression checks

The prior Simon value is from [ROUND2.md](ROUND2.md). Academic shirts/trousers
values are from [BENCHMARKS.md](BENCHMARKS.md); ROUND2 does not contain new
academic rows. Academic lengths retain the source dataset's units. Simon lengths
are millimetres.

| Test | Prior BETA efficiency | Current | Change pp | Length | Runtime s | Valid |
|---|---:|---:|---:|---:|---:|---|
| Shirts, trial 1 | 87.16% | 87.31% | +0.15 | 61.85 | 60.63 | Yes |
| Shirts, repeat | 87.16% | 87.97% | +0.81 | 61.39 | 60.61 | Yes |
| Trousers | 90.42% | 90.21% | -0.21 | 241.44 | 60.29 | Yes |
| Simon, 120 s | 88.99% | 89.24% | +0.25 | 7316.13 | 121.42 | Yes |

The ordinary-job path keeps the original Sparrow instance construction, f32
guard, collision-coverage checks and repair rules. The observed trousers score
is 0.21 points lower than its prior run; shirts and Simon are higher. Timing
cutoffs and worker scheduling cause variation even with seed 1.

The sanity example also compares against a separate raw Sparrow run. The raw
layouts were independently validated too. The legacy 0.5-point comparison
check failed the first shirts trial and passed its repeat. Both trials are
retained, rather than selecting only the passing trial.

| Test | BETA | Raw Sparrow | BETA minus raw pp | 0.5-point check | BETA / raw valid |
|---|---:|---:|---:|---|---|
| Shirts, trial 1 | 87.31% | 88.13% | -0.82 | Fail | Yes / Yes |
| Shirts, repeat | 87.97% | 88.09% | -0.12 | Pass | Yes / Yes |
| Trousers | 90.21% | 90.61% | -0.40 | Pass | Yes / Yes |

## Machine load and evidence

Final runs started with no compiler processes and less than 16% sampled CPU
load. No Rust compiler activity was recorded during any reported final run.
Mean total CPU load ranged from 8.01% to 43.17%; the highest sampled peak was
78.73%. Load includes the benchmark itself. Academic raw runs set
`RAYON_NUM_THREADS=3`. This remains a shared-machine experiment.

Earlier portfolio runs overlapped builds and were repeated. They remain in the
evidence directory but are excluded from the tables above. Ordinary-job
measurements use `d527b31`; the final obstacle-order measurements use `6e8bdd1`.
The final progress guard at `c56d809` adds independent checks without changing
geometry or search rules. The shirts repeat uses that commit. Each run records
its exact commit in `conditions.json`.

Evidence root: `$BETA_DATA_DIR/marker-bench/nest-v02/integrated/`.
Ordinary long runs are at its top level. Final feature, fold and academic runs
are under `final/`. Each has a summary, validation, result, source job and
progress where applicable. `conditions.json` and `load.json` contain host-load
samples. Academic runs also keep raw output, `raw-beta-result.json` and
`raw-validation.json`. Source datasets were not changed.

## Verification and reproduction

Run Cargo from `marker/`, with the assigned external `CARGO_TARGET_DIR`,
`CARGO_BUILD_JOBS=3` and `BETA_DATA_DIR`.

```text
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release -p beta-marker-cli -p beta-marker-nest --bins --examples
cargo run --release -p beta-marker-nest --example feature_bench -- <job.json> <seconds> <out> <feature> [runs]
cargo run --release -p beta-marker-nest --example sanity -- <academic.json> <raw-sparrow-executable> <out> 60
cargo run --release -p beta-marker-nest --example sanity -- --check-raw <converted-job.json> <raw.json> <out>
```

Workspace tests: **108 passed**, two existing long tests ignored. All 31 nesting
unit tests passed in 7.85 s; the synthetic garment integration test took 2.29 s.
Clippy with warnings denied passed. The release build passed. No TypeScript files
changed.

The release CLI was tested with `--time 1 --seed 1 --workers 2`. Combining
`--runs 3 --fixed <previous.json> --warm-start <previous.json>` retained both
locked transforms exactly: 1150.02 mm, 74.17%, 1.01 s. A separate native warm start
returned 1152.08 mm, 74.04%, 1.29 s, no longer than its previous result. The
separate `validate --json` command passed both without errors or warnings.
Evidence: `final/cli/checks.json`.

## Limits and merge follow-up

The obstacle/fold path uses f64 polygon construction searches, rather than
Sparrow compression. It can be less dense. It tries sampled width positions,
wide area-weighted orders and fully shuffled orders. Faceless searches use five
allocations rather than all 2^N assignments; bundle directions use all-forward
and alternating assignments. Warm starts must be valid and compatible with the
current job and width. Search budgets exclude import/final checks, and the
obstacle path completes a construction before checking time; they are not hard
end-to-end deadlines.

After merging the matching/planning agents, repeat combined fixed, shade,
repeat, bundle and layered-demand cases. Shared core follow-up: move
`validate::cut_outline` into rules/geometry and use it for block-aware length
in `compute_metrics`, retaining useful outline area. Share the natural garment
size bundle order across nest, match, plan and export. No model-field change is
requested.

The nesting implementation adds no dependencies. The qa2 merge adds these
packages, checked with `cargo metadata --offline --format-version 1`:

| Package(s) | Version(s) | Licence |
|---|---|---|
| rust_xlsxwriter | 0.95.0 | MIT OR Apache-2.0 |
| crc32fast, equivalent, flate2 | 1.5.2, 1.0.3, 1.1.10 | MIT OR Apache-2.0 |
| hashbrown, indexmap, typed-path | 0.17.1, 2.14.2, 0.12.3 | MIT OR Apache-2.0 |
| zip, simd-adler32 | 7.4.0, 0.3.10 | MIT |
| zlib-rs | 0.6.8 | Zlib |
| zopfli | 0.8.4 | Apache-2.0 |

Sparrow 0.3.0 is MIT. Jagua 1.0.0 remains its unmodified MPL-2.0 dependency.
The generated lockfile update is excluded; the lead must regenerate it after
integration.
