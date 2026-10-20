# Lectra learning experiment — astra-learn

Date: 19 October 2026. Base: `marker/mvp` at `25e1233`.

**Keep the efficiency predictor as an optional research/costing estimator.** On
all 15,000 supplied test tasks it reaches **1.7946 percentage points MAE and
R² 0.90944**. Rust performs both feature extraction and inference. The model is
**2,610,621 bytes**, below 5 MB. Do not change search or planner defaults yet:
the strategy experiment did not clear its held-out gate, and the small comparable
width-ranking check tied the constant-efficiency baseline.

## Data and leakage controls

All **100,000 tasks**, **7,360,718 physical parts** and **399,746 shapes** were
processed. Supplied `is_train`, `is_val`, `is_test` flags define 70,000 / 15,000 /
15,000 rows. No row was dropped from the efficiency experiment. Every task has
exactly one split. Part IDs repeat across tasks: joins use `(tasks_index, part_id)`
and verify one-to-one rotation records. The existing restricted unpickler and
shape decoder are reused with pinned source checksums and the audited allow-list.
No unrestricted pickle load, proprietary sample or geometry is committed.

Training uses only `is_train`. Even-ID validation rows (**7,602**) select the
model; odd-ID validation rows (**7,398**) calibrate its residual interval. The test
split is evaluated after selection. Neither validation subset is used to fit tree
weights. All nine tree configurations and two ridge baselines are preserved in
[metrics.json](models/metrics.json). No hyperparameter is selected on test scores.

There are 50 copy-weighted geometry/task features. Shape hashes, task IDs, split
flags, recorded runtime, achieved length and efficiency are excluded from the
feature vector. Shape hashes are used only for joins and the overlap audit.
There are no usable garment size labels; approximate shape diversity and area
spread are proxies, not recovered size/garment identities. Features discard
absolute rotation phase and fine geometric arrangement. They cannot prove fit.

**The supplied split is not a new-style or new-customer holdout.** In the test set:

- 8,519 tasks contain only shapes also present in training.
- 2,404 tasks contain only unseen shapes; their MAE is **3.0673 pp**, R² **0.86189**.
- 4,077 tasks mix seen and unseen shapes.
- Mean per-task copy-weighted seen-shape fraction is **72.72%**.
- 1,574 test feature vectors exactly duplicate a training vector.

This overlap is disclosed, not removed by silently replacing the required split.
Even the unseen-shape subset does not establish a new-factory holdout.

## Accuracy

Efficiencies are fractions in the API; errors below are percentage points.
Train and pooled-all figures include fitted observations and are **not held-out**.

| Supplied split | Tasks | MAE, pp | RMSE, pp | R² |
|---|---:|---:|---:|---:|
| Train | 70,000 | 1.3164 | 1.9959 | 0.96681 |
| Validation, selection + calibration | 15,000 | 1.7975 | 3.1290 | 0.91736 |
| Test, untouched during selection | 15,000 | **1.7946** | **3.2950** | **0.90944** |
| All, descriptive only | 100,000 | 1.4603 | 2.4261 | 0.95086 |

Test mean bias is **+0.0507 pp**. Test 90th-percentile absolute error is **3.8946 pp**.
The calibrated half-width is **3.9145 pp**; nominal 90% interval coverage on test
is **90.10%**. This is empirical coverage on this supplied split, not a guarantee
under factory/style shift. The interval is clipped to the model's 1–100% range.

| Baseline | Test MAE, pp | Test R² |
|---|---:|---:|
| Training mean (79.2115%) | 7.1390 | approximately 0 |
| Training median (82.0120%) | 6.6950 | -0.06634 |
| Current planner constant (75%) | 9.1134 | -0.14656 |
| Learned trees | **1.7946** | **0.90944** |

Ridge with the same 50 standardized features and alpha 1 or 100 achieved
**4.2005 / 4.2006 pp** on the selection subset. The selected trees achieved
**1.8111 pp** there. Ridge was not promoted. No comparison with a locally trained
factory costing model was made.

| Test subgroup | Tasks | MAE, pp | R² |
|---|---:|---:|---:|
| 1–25 pieces | 5,000 | 2.5476 | 0.90143 |
| 26–75 pieces | 5,974 | 1.4275 | 0.86820 |
| 76–150 pieces | 2,094 | 1.4650 | 0.90906 |
| 151+ pieces | 1,932 | 1.3378 | 0.90525 |
| Sheet code 0 | 14,915 | 1.7798 | 0.90854 |
| Sheet code 2 | 79 | 3.5996 | 0.90069 |
| Sheet code 3 | 6 | 14.8034 | -0.44847 |

Sheet codes remain opaque. The live job adapter accepts only plain open jobs,
conditionally corresponding to code 0. It does not map codes 2/3 to folded or
tubular fabric. Code 3 has poor accuracy and only six test observations.

Under the rectangular-strip denominator assumption, code-0 implied-length MAPE is
**2.6055%**, computed as `mean(abs(reference_efficiency / prediction - 1))`.
It excludes ends, roll loss, defects and other production allowances. It is not
an independently measured factory consumption error.

## Model and Rust verification

Selected model: scikit-learn 1.7.2 histogram gradient boosting, 600 trees, at most
63 leaves each, learning rate 0.06, minimum leaf size 25, L2 regularization 2,
seed 7030786, no automatic early stopping. Validation selects among 15/31/63
leaves and 150/350/600 iterations. Numeric leaf contributions already include
the learning rate. Rust traverses numeric thresholds directly; it does not need
Python, NumPy, scikit-learn, a GPU or network access.

- All **100,000** Python and Rust predictions agree within **2.22×10⁻¹⁶** absolute
  efficiency (required tolerance 1e-12).
- A separate converted-job audit compared **271 supported Tier A jobs** against
  source-table features: maximum relative difference **1.54×10⁻¹⁵**. It explicitly
  skipped 25 finite-sheet jobs and four orientation-handicapped jobs. No mismatch.
- Twelve geometry-free held-out feature vectors and their independent predictions
  form the bundled-model regression test.
- Shared-host inference mean was **95.662 microseconds/task**, over 100,000 rows,
  excluding JSON I/O, model loading and feature extraction. This is not an end-to-end
  planner speed measurement or a quiet-machine benchmark.
- Offline fitting/selection took **907.33 seconds**, using two CPU threads on the
  shared laptop. Dataset transport and Rust builds are excluded. No GPU was used.

The model loader checks format/schema, finite values, calibration, size limits,
feature indices and acyclic forward tree edges before inference. Runtime inputs
are checked for demand overflow and unsupported garment constraints. Predictions
and intervals are estimates, never valid placements or certified upper bounds.

Model SHA-256:
`66fdbb04ed919849692563d79271c7fc61033fa4da63b53dce0014fdd1cec07f`.
Feature matrix SHA-256:
`af7f0bed8b3b79741bcb95b3fe333bbb1cc23f84e15ae0717d6eef855170f2bd`.

## Planning, strategy and stopping evidence

The complete diagnostic, including paired-source checksums, is in
[use-diagnostics.json](models/use-diagnostics.json).

**Ranking:** test Spearman efficiency correlation is **0.93909**. On 18,206 random
test pairs separated by at least one reference efficiency point, direction
accuracy is **93.32%**. These are different markers, not competing cut plans.
A narrower test found **41** test pairs with identical shape/count/orientation
requirements, no extra constraints and different widths, with over 1% implied
reference-length difference. Both learned and constant-efficiency rankings scored
**41/41**. This provides **no measured planning advantage**. Pairs may share groups;
no independence or factory-order claim is made.

**Strategy choice:** reuse the existing v0.3 effective paired studies: compression
30%, compression 40%, sequential three, race three and lighter samples. Each arm
has its own paired baseline. The prior frozen geometric exclusion, task 12127,
remains excluded; it cannot fit at its sole angle. Each arm has 39 valid tasks,
split **27 train / 7 validation / 5 test** by the supplied dataset flags. Fit one
decision stump per arm using only pre-run piece count or predicted efficiency;
select the arm on validation mean paired gain. No baseline result is used as an
input to this pre-run selector.

Validation selected lighter samples when predicted efficiency is at least
83.9368%. Test mean policy gain was **+0.2527 pp**, but the 95% bootstrap interval
was **[-0.0202, +0.7782] pp**, with only five test pairs. **Reject promotion; keep
the existing default.** This small, target-stratified development sample is not
100,000 strategy-labelled tasks. No cluster implementation was available on the
inspected cluster branch, and no learned construction order is claimed.

**Stopping:** ten supplied-test tasks have historical 60-second and matched-duration
valid pairs. Predicted gap versus longer-run gain has Spearman **0.4303**. Only two
are within one point of or above the prediction; neither gained over one point in
the separate longer run. Two cases cannot validate a stopping rule, and these are
separate stochastic runs rather than continuation curves. **No learned stopping
or time allocation is enabled.** A prediction is not an attainable ceiling.

No new nesting run was started. All search diagnostics use existing adjacent,
alternating paired results, preserving their shared-host and stochastic caveats.
This avoided adding solver load to the busy CPU.

## Integration and known gaps

[INTEGRATION.md](INTEGRATION.md) identifies the exact planner/costing insertion
points and proposed optional estimator contract. Use predicted length to prioritize
candidate refinement; keep actual checked nests in final plans. Costing can expose
net length and its interval with explicit garment counts and allowances. Keep the
current search parameters and stopping rules.

The live adapter refuses gaps, buffers, finite sheets, fusing blocks, folded/tubular
fabric, defects, shade, repeats/matching and per-bundle direction. It applies size
ratios, mirrored quantities, shrinkage and per-piece orientation freedom. Out-of-
training-range warnings are advisory; they do not detect every new distribution.
The target is Lectra's recorded approximately five-minute result, not BETA's short
budget outcome, maximum achievable efficiency or a proven packing bound.

No core/model, planner, costing, workspace or CLI contract was edited. Lead changes:
register this crate in `marker/Cargo.toml`, remove its temporary `[workspace]`,
regenerate Cargo.lock, and opt in via planner/costing dependencies. No `Job` or
`MarkerResult` contract change is needed. Re-test workspace integration, conversion
parity after rule changes, WASM if used, and paired factory order quality before
changing defaults.

## Validation and dependencies

Compiler: `rustc 1.98.1 (48a229cea 2026-09-01)`. Commands below run from `marker/`
with the assigned external target directory and `CARGO_BUILD_JOBS=3`.

```powershell
cargo test --release --manifest-path crates/beta-marker-learn/Cargo.toml
cargo clippy --release --manifest-path crates/beta-marker-learn/Cargo.toml --all-targets -- -D warnings
cargo build --release --manifest-path crates/beta-marker-learn/Cargo.toml
cargo fmt --manifest-path crates/beta-marker-learn/Cargo.toml --check
```

Isolated crate: **9 tests passed**, Clippy with warnings denied passed, formatting
passed. Release build passed. Full-workspace `cargo test --workspace --offline`
and `cargo clippy --workspace --offline --all-targets -- -D warnings` were started
but **cancelled before completion** when the lead requested the immediate handoff.
The test run was still compiling desktop dependencies; the Clippy run was waiting
for that build. No full-workspace pass is claimed. The lead must rerun both after
registration. Only this worktree's identified build process trees were stopped.
Python syntax, split/count checks, all-row Rust parity and the job/source feature
audit also passed. Reproduction and exact data commands are in [README.md](README.md).

Runtime dependencies are local `beta-marker-core`, `anyhow`, `serde`, `serde_json`
and their existing permissive transitive packages. No Linfa or additional runtime
ML package was added. New training-only installs are scikit-learn, SciPy, joblib,
threadpoolctl and cloudpickle (BSD-3-Clause); existing NumPy/pandas are BSD-3-Clause.
The complete resolved licence list is in [DEPENDENCIES.md](DEPENDENCIES.md).

Model weights and derived summaries are CC BY 4.0. Source attribution: Corentin
Lallier, Laurent Vézard, Bruno Pinaud and Guillaume Blin (2022), *Nesting Tasks Dataset
for 2D-Nesting Efficiency Estimation*, version 1.1,
[Zenodo 7030786](https://doi.org/10.5281/zenodo.7030786).
See [model attribution](models/ATTRIBUTION.md) for transformations and licence.
