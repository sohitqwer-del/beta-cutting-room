# Public runner checks, 19 October 2026

The standalone Rust tool passed five tests. They cover CSV quoting, tie
boundaries, failed-layout denominators, length savings, correlation with tied
ranks, UTC dates, and refusal to score missing or failed validator evidence.
Clippy passed with warnings denied.

```sh
cargo test --locked --manifest-path bench/public/Cargo.toml
cargo clippy --locked --manifest-path bench/public/Cargo.toml --all-targets -- -D warnings
```

The documented one-command path built the release CLI. A 20-task smoke at
30-second budgets returned 20 valid layouts. A second pipeline smoke at
`--quick --time-scale 0.01` also returned 20 valid layouts and completed the
export. Both exports passed the website's actual loader. Their IDs have a
`smoke-` prefix; neither run is a launch quality result.

The first smoke exposed an old-CSV compatibility issue in the laptop comparison:
round-2 CSVs have no tier column. The reader now accepts that older format for
paired comparisons. New Lectra exports still require explicit A/B evidence.
The original 30-second evidence was re-exported successfully after that fix.

The 192-task server checkpoint exported as provisional. All 192 CSV scores had
successful saved independent validator reports. It passed all 17 website
contract tests, using a byte-for-byte snapshot of the site owner's loader and
tests from `marker/opus-site`, in an external test directory. This confirms the
JSON contract, not the final quality result.

The Linux extra-suite script passed `bash -n`. Its gate was tested while the
server status was `running 60s`: it refused to start and created no output
directory. No second Lectra run was started, and no existing run was stopped.
Invalid worker counts and a zero time scale were also refused before a search.

Raw smoke jobs, layouts, reports and the contract-test snapshot stay under
`$BETA_DATA_DIR/marker-bench/public-2026-10/`. No geometry is committed.
