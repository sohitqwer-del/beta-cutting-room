# Version 0.3 quality validation

The quality branch merged validator/integration commit `55c0123` in merge
`2588c05`. No other worktree was edited. The lead explicitly authorized testing
the combined branch with the two stale UI contract snapshots skipped.

## Commands and results

All Cargo commands ran from `marker/`, with the assigned external
`CARGO_TARGET_DIR`, `BETA_DATA_DIR`, and `CARGO_BUILD_JOBS=4`.
Tests used `RUST_TEST_THREADS=2`; nesting tests use small budgets and two workers.

```text
cargo test --workspace -- --skip cmd::serve::tests::v02_routes_preserve_options_and_plan_contract --skip tests::json_api_obeys_the_shared_model_contract
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release -p beta-marker-cli -p beta-marker-nest -p beta-marker-match --bin beta-marker --example feature_bench --example quality_summary --example measure
cargo test -p beta-marker-nest --example quality_summary
```

Workspace: **230 passed, 4 ignored, 2 filtered out**. Clippy: **passed with
warnings denied**. Release build: **passed**. The separate CSV evidence parser
test passed. All 40 nesting tests and 22 matching tests passed.

Only these two tests were skipped:

- `cmd::serve::tests::v02_routes_preserve_options_and_plan_contract`
- `tests::json_api_obeys_the_shared_model_contract` in `beta-marker-wasm`

The lead will resolve these snapshots when merging `ui3`. Neither file was edited
on the quality branch.

The two other integration-report failures are fixed here. Generated example JSON
now includes the optional piece fields and passes `examples_are_reproducible`.
The old no-order test expected a separate garment for each size. The new test
follows `rules::bundle_sizes`: one garment without an order. It checks a valid
one-size job through native nesting and independent validation, and rejects
mixed size labels before searching, with a request to supply `order.sizes`.
No private size ordering remains.

Native tests verify `Piece.flip=true` on faced fabric and `Piece.flip=false` on
faceless fabric. Separate checks cover per-piece tilt, numeric bundle order,
full-cut/fold-half choices, warm starts, fixed transforms, copy identities,
conservative collision outlines, and adaptive-count overflow. A combined fixed/shade/per-piece-flip regression
checks that freely mirrored copies do not require a positive mirror_quantity,
and preserves fixed copy identities when sections are joined. Nesting and
matching now use the shared physical `rules::cut_outline` helper.

## Tooling and interrupted trials

The 0.1-second tooling smoke saved a valid marker and validation report. Its
first wrapper run incorrectly saw a null background-process exit code. Retaining
the native process handle fixed it; the second wrapper returned zero. A separate
CLI validation of its saved marker returned `ok: true`, without warnings.
These smoke results are not quality comparisons.

Two pre-integration baseline attempts were stopped when another compiler started.
Their partial CSV/layout/load evidence is retained under
`$BETA_DATA_DIR/marker-bench/quality-v03/interrupted-baseline-attempt-*`.
They are excluded from default selection and confirmation. A third attempt was
stopped while waiting so the authorized validator merge could be tested.
No trial from that quiet-machine plan was accepted. The user then explicitly
authorized shared-machine measurements, with unrelated background programs left
running, two task pairs in parallel and three separator workers per task.
The current protocol is in `bench/experiments/v03/README.md`: adjacent pairs,
alternating order, CPU samples in conditions.json and score-independent load
reruns. Only our own worktree's builds/benchmarks gate startup; unrelated programs
are not stopped. Raw and effective evidence are kept separately.

## Dependencies

The quality implementation adds no crate or npm dependency. The authorized
integration merge adds a direct `flate2` 1.1.10 edge (MIT OR Apache-2.0), using
the existing `zlib-rs` 0.6.8 backend (Zlib); both packages were already in the
workspace graph. Its new internal dependency edges are Apache-2.0 workspace
crates. The generated Cargo.lock changes are left to the lead to regenerate.
Sparrow remains MIT; Jagua is its existing MPL-2.0 dependency. No engine source
was copied, vendored, or patched.

Matched-duration CLI runs now keep the full recorded duration by default. The
optional `--match-cap` is applied only when supplied.

## Paired shared-machine runner

The new runner tests actual adjacent, alternating native baseline/variant calls
with the same tasks and seeds. CPU-only rerun tests prove that a later, better
score cannot replace the first acceptable-load retry. CSV evidence preserves
quoted fields and embedded newlines. The full shared-machine tooling smoke
completed two short pairs, wrote CPU samples inside conditions.json, passed the
load audit and produced effective paired CSVs. Smoke scores are excluded.

Each timed trial now stores job.json, result.json, validation.json and start/end
UTC epoch times. `bench audit` prepares only load-selected tasks; `bench replay`
reuses their original jobs, seeds, budgets and settings; `bench finalize` keeps
raw trials and records score-independent replacement provenance. Garment jobs
use the same audit path. Layered rows report demanded-product efficiency as well
as occupied-outline efficiency, so surplus layer cuts cannot inflate a claim.

## Final reduced-scope checks

The updated totals above include the common tuning/progress path and sampling-gap
audit. The two new tests check monotonic restart progress on one clock and detect
missing telemetry even when every CPU sample is 100%. The full workspace, Clippy
with warnings denied, and release CLI/summary build passed after the original
five studies finished. The release build took 2m 01s on the shared host.
PowerShell parsing passed for all quality scripts. No build ran alongside our
timed trials. Sampling-gap replays use the original executable; the new audit
selects and finalizes evidence separately.

The lead then limited remaining measurements to the completed 40-task paired
development studies and three FreeSewing jobs at 60 seconds. Shuffle, adaptive
compression and faster shrink remain options without measurements. Full
confirmation, matched duration, academic and longer feature runs are deferred.

All five development studies were re-audited. Two task pairs crossing the
sampling interruption were replayed with the frozen executable. Their four new
layouts passed validation and sampling/load checks; no persistent outlier
remains. The selected profile stays at 80/20. Simon, Teagan and Titan each passed
their 60-second run without warnings or timing outliers. The native summary
verified the geometry-only exclusion against the saved source job. After a
reporting-only filter excluded the unaudited earlier baseline, Clippy and the
release summary build passed again, and the native report was regenerated.
