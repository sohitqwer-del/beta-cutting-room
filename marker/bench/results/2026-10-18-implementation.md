# Bench implementation report — 18 October 2026

Work is on `marker/bench`. Tooling commits: `a3413d1`, `cdc7159`. The working engine (`marker/nest`, `a3feb27`) and validator (`marker/qa-export`, `1a056c1`) were merged into this branch. Nothing was pushed or merged into another branch.

## What works

- The Rust academic converter reads the nine supported sparrow v0.3.0 inputs, keeps their coordinate values, maps exact rotation sets and quantities, and rejects unsupported reflections and shapes.
- `bench academic` supports instances, budgets, seeds, task concurrency and worker counts. Missing public inputs are downloaded into the external data directory with curl.
- `bench lectra` reads compact Job JSONL, refuses non-comparable records, supports fixed or capped matched durations, and bounds task/worker concurrency.
- Every successful engine result is independently validated. Failed or invalid results have no scored efficiency. CSV and Markdown summaries include validation status, differences, length savings, mean/median differences, tie counts, strata and time.
- The one-time Python converter downloads and verifies all four Zenodo files, resolves every pickle global before restricted loading, classifies the full population, and writes fixed-seed samples plus the complete eligible id list. The full per-file globals are in `bench/lectra/pickle-globals.json`.

## Academic result

[Full summary](2026-10-18-academic-60s.md): 21 attempted trials, 20 valid; mean difference **-2.3976 percentage points**, median **-2.0556**, win/tie/loss **0/0/20** against the published means. Wall time **674.780 s**, summed nesting time **1290.761 s**. Shirts seed 1 was rejected for overlap between pieces 7 and 2 and was not replaced.

These are 60-second search budgets on an i9-11900H laptop, with two simultaneous tasks and three workers per task. The paper used 1200 seconds and an AMD 7950X. This is a shorter-budget comparison with a research baseline, not proof of hardware or algorithm equivalence. BETA itself uses sparrow. Full CSV/config files stay under `$BETA_DATA_DIR/marker-bench/results/2026-10-18-academic-60s/`.

## Lectra result and safety

All files matched the API MD5 values. Restricted loading found 100,000 tasks, 7,360,718 parts, 7,394,464 constraints and 399,746 shapes. The opcode scan found 12 distinct globals, all explicit pandas/numpy reconstruction functions/classes or `builtins.slice`. No unexpected global was allowed. The exact names, per-file lists, all 15 constraint types, counts and observed parameters are documented in [the dataset audit](../lectra/README.md).

The public sources do not define sheet-type codes 0, 2 or 3. Nonzero flip settings and the meaning of multiple sub-shapes also need evidence before exact mapping. The final eligible count is 0; 9,606 otherwise supported candidates remain blocked by sheet-type semantics. The 300-task diagnostic sample has 25 tasks per stratum and retains comparable=false. No commercial efficiency claim is made by relaxing these rules. See the final [conversion summary](2026-10-18-lectra-audit.md) for overlapping exclusions and actual sample counts. Raw geometry is neither committed nor published.

The data use flat alternating coordinates, and `sizes` counts scalar coordinates per sub-shape, not vertices. The decoder divides these counts by two and checks consistency before forming rings. The source's obsolete pandas Int64Index global is reconstructed with the current integer Index. Old numpy.core globals resolve to their trusted NumPy 2 counterparts after the explicit allow-list check. No pandas pickle-reader shortcut is used.

## Commands and verification

Run from `marker/`, with an external data directory and `CARGO_TARGET_DIR` already set. The converter venv is under the external data directory.

```powershell
$env:CARGO_TARGET_DIR = "$env:BETA_DATA_DIR/cargo-target/marker-bench"
$env:CARGO_BUILD_JOBS = '2'
$python = "$env:BETA_DATA_DIR/venvs/marker-bench/Scripts/python.exe"
& $python -I bench/tools/lectra_convert.py download
& $python -I bench/tools/lectra_convert.py inspect
& $python -I bench/tools/lectra_convert.py self-test
& $python -I bench/tools/lectra_convert.py convert --sample-size 300 --seed 7030786
cargo test --workspace
cargo clippy --workspace --all-targets
cargo run --release -p beta-marker-cli -- bench academic --instances shirts,trousers,swim,albano,dagli,mao,marques --time 60 --seeds 1,2,3 --threads-per-task 3 --jobs 2 --out "$env:BETA_DATA_DIR/marker-bench/results/2026-10-18-academic-60s"
```

Final Rust checks: **42 tests passed, 2 intentionally ignored**, no failures. Clippy exited successfully with no bench warnings; one existing warning remains in core `rules.rs:107`, outside this role. Converter self-tests cover both global opcodes, forbidden globals, units, ring counts, exact rotations and repeatable stratification. Four test-only shelf placements pass the real validator; an overlapping placement is excluded. Boundary ties and failed-run exclusion are tested. The nine-input pre-engine smoke check is documented separately and is not a quality result.

## Dependencies and integration

No new Rust benchmark crates. Converter dependencies: pandas 2.2.3 and numpy 2.2.6 (BSD-3-Clause); python-dateutil 2.9.0.post0 (Apache-2.0/BSD-3-Clause); pytz 2026.5 (MIT); tzdata 2026.5 (Apache-2.0); six 1.17.0 (MIT). The paper audit used pypdf 6.1.1 (BSD-3-Clause), which is not needed by the converter. Installed package metadata was checked. The merged nest work adds direct geo/rand references, both MIT/Apache-2.0 and already in the resolved dependency graph.

No data-contract change is requested. Cargo automatically added the merged nest crate's missing geo/rand edges to `marker/Cargo.lock`. That generated two-line diff is outside bench ownership and remains uncommitted for the lead to regenerate/include. The benchmark run records its dirty flag and code revision. No dataset, build output, geometry, private folder or package-lock file was committed.

## CLI eligibility checks

The real CLI refused the diagnostic sample before nesting: task 43028 was rejected with `sheet_type_semantics_unverified`, exit code 1. The empty strict sample was rejected with `input contains no tasks`, exit code 1. No result CSV was created. Metadata-only logs are under `$BETA_DATA_DIR/marker-bench/results/2026-10-18-lectra-eligibility-check/`.

```powershell
cargo run -p beta-marker-cli -- bench lectra --input "$env:BETA_DATA_DIR/marker-bench/lectra/diagnostic-sample-300.jsonl" --time 60 --limit 300 --jobs 2 --threads-per-task 1 --out <fresh-directory>
cargo run -p beta-marker-cli -- bench lectra --input "$env:BETA_DATA_DIR/marker-bench/lectra/sample-300.jsonl" --time match --limit 300 --jobs 2 --threads-per-task 1 --out <fresh-directory>
```

These expected refusals verify the eligibility gate. They are not commercial benchmark runs. Completing a commercial comparison needs a documented mapping for sheet types and, to widen coverage, flip and other constraint semantics. No missing semantics were guessed.
