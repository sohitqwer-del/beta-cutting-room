# BETA Marker Learn

Offline prediction of Lectra's **recorded achieved efficiency**. Rust performs
feature extraction and inference. Python is used only for restricted dataset
transport and offline training. There is no runtime network access or Python dependency.

This is an experimental estimator, not a nesting engine or packing upper bound.
It returns estimated length and a calibrated residual interval. It never stops a
nest, changes the search defaults or returns a feasible marker.

## API

```rust
use beta_marker_learn::Model;
fn example(job: &beta_marker_core::Job) -> anyhow::Result<()> {
    let model = Model::bundled()?; // load once, then reuse
    let estimate = model.estimate_job(job)?;
    println!("Estimated length: {} mm", estimate.length_mm);
    Ok(())
}
```

`features_from_job` supports plain open, unconstrained fabric with zero piece and
edge gaps. It uses canonical size multipliers, normal plus mirrored demands,
shrinkage, per-piece rotation/tilt and optional reflection. It refuses layered
fabric, per-bundle direction, defects, shade, repeat matching, finite sheets,
buffers and fusing blocks. Refusal lets the caller retain its existing estimator
or do a real nest. Raw dataset codes 2 and 3 are included in research evaluation
without pretending their semantics are known.

`outside_training_range` is an advisory min/max check, not a reliable detector of
new styles. An empty list does not prove that a prediction is accurate. The model
is not calibrated to BETA's short search budgets or any particular factory.

## Features

Fifty features describe physical copy count, approximate shape diversity, width,
area distribution, convexity, minimum rectangle fill, compactness, aspect ratio,
normalised dimensions and legal rotation unions. Shape measurements are invariant
to rigid rotation, translation and reflection. Statistics are weighted by copy count.
Geometry diversity groups log area, log rectangle sides and convexity rounded to
1e-7; this is a proxy, not a garment/size identity. The dataset does not contain
usable garment size labels. Task IDs, shape hashes, split flags, runtime, achieved
length and target efficiency are not model inputs. Source sheet category and
non-rotation constraint count are descriptive research features, not decoded rules.

## Reproduce

Run from `marker/`. Set `BETA_DATA_DIR`, `CARGO_TARGET_DIR` and `CARGO_BUILD_JOBS=3`
to external data/build paths. Use the existing marker-bench virtual environment
with the versions in `tools/requirements-training.txt`. Data must remain outside git.
Each output command creates a new file; choose a fresh evidence folder for a rerun.

```powershell
$crate = 'crates/beta-marker-learn'
$evidence = Join-Path $env:BETA_DATA_DIR 'marker-bench/astra-learn-v1'
$python = Join-Path $env:BETA_DATA_DIR 'venvs/marker-bench/Scripts/python.exe'
cargo build --release --manifest-path "$crate/Cargo.toml"
$exe = Join-Path $env:CARGO_TARGET_DIR 'release/marker-learn.exe'
& $python -I "$crate/tools/prepare.py" --out $evidence --stage shapes
& $exe shapes "$evidence/shapes.jsonl" "$evidence/shape-stats.jsonl"
& $python -I "$crate/tools/prepare.py" --out $evidence --stage tasks
& $exe features "$evidence/shape-stats.jsonl" "$evidence/rotations.jsonl" "$evidence/tasks.jsonl" "$evidence/features.jsonl"
& $python -I "$crate/tools/train.py" --features "$evidence/features.jsonl" --executable $exe --out "$evidence/training"
```

The transport imports the existing `bench/tools/lectra_convert.py` restricted
unpickler and shape decoder, verifies all source MD5 hashes, and checks the audited
allow-list. It never calls `pandas.read_pickle` or an unrestricted pickle loader.
Part joins use `(tasks_index, part_id)` because part IDs repeat across tasks.
Training uses two CPU threads; no GPU is needed for these small histogram trees.
The scikit-learn exporter is version-pinned, and all 100,000 exported predictions
must agree with independent Rust traversal within 1e-12.

```powershell
& $exe predict "$crate/models/lectra-v1.json" 'path/to/job.json'
cargo test --manifest-path "$crate/Cargo.toml"
cargo clippy --manifest-path "$crate/Cargo.toml" --all-targets -- -D warnings
```

## Ownership and integration

The crate has its own `[workspace]` until the lead registers it. No shared model,
planner, costing, CLI or workspace contract was edited. See `RESULTS.md` for
measurements, limitations and concrete integration proposals.

Code is Apache-2.0. Model weights and derived evaluation summaries are CC BY 4.0;
see `models/ATTRIBUTION.md`. Dependencies and training-only packages are listed in
`DEPENDENCIES.md`. No source geometry is distributed here.
