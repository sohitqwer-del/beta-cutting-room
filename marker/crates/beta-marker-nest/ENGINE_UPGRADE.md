# Engine upgrade and paired measurements

## Recommendation

**Do not promote the Sparrow main/Jagua 1.1 upgrade as a production speed or
quality improvement.** Keep the existing unmodified Sparrow 0.3.0/Jagua 1.0
production engine for now. At equal AVX2 flags the upgrade lost throughput;
the combined upgrade/native build was essentially tied on both 30-second
confirmation sets. The 10-task Lectra study averaged -0.0053 efficiency points
(6 wins / 4 losses), and the nine garment pairs averaged +0.0123 points (5/4).

The upstream migration is a small, reviewable integration candidate, not a
recommended performance-default change. **Only this report is committed.**
The workspace test build did not finish within the closeout window, so the
lead's condition for an engine commit was not met. The engine candidate and
benchmark tools remain uncommitted in the assigned worktree. No Cargo profile
or tracked `.cargo/config.toml` change is part of this work.

Nightly SIMD/native and the separate Jagua 2 port are the promising next
experiments. Both won their short screens, but neither has a 30-second Lectra
confirmation. Portable AVX2 + nightly SIMD was **not measured**. The lead's final
30-minute closeout instruction stops further builds/benchmarks for those variants.
Do not ship this host's native executable or claim that the SIMD result transfers
unchanged to a portable build.

## Dependency boundary

The upstream pair is Sparrow `1fe3e3146110c34d77510636defb737c7ce893c6`
(MIT) and Jagua `9a0b28d86ec05ef287c0b641269be83b6f29c230` (1.1.0,
MPL-2.0). Both are unmodified. Sparrow still reports package version 0.3.0,
so result metadata includes revision identifiers. A crates.io source override
connects Sparrow's exact `=1.1.0` dependency to the same pinned git source as
the adapter. Without it, Cargo creates incompatible Jagua types from two sources.

Jagua main is **2.0.0**, not an unreleased 1.1 patch. Revision
`4661c08efa9cd9bbf4404abcb73ad337ca8fc778` includes the redesigned collision
engine and the subsequent index and ray-casting fixes. Upstream Sparrow main
does not compile against it. The separate MIT Sparrow compatibility commit
`a9446764f3095df945b471e96aa1b919be43c445` pins that Jagua revision, removes
obsolete quadtree/pier settings, and renames the debug collision-index assertion.
It changes no search parameters. Jagua remains completely unmodified.
That experimental Sparrow checkout is outside BETA. No Sparrow source is vendored.
An untracked Cargo config points to it only for experiments.

The port passed the predeclared short-screen gate: at least 1.10x geometric
mean wall and process-CPU throughput, at least 9/12 wall-throughput wins, and
all results independently valid. Its format-patch file is under
`$BETA_DATA_DIR/sparrow-port/`. Publish that MIT fork or offer the patch upstream,
then pin its published revision in a separate integration change. Do not ship
an absolute-path dependency or silently change Jagua's version in place.

The adapter's f32 precision guard, full polygon-coverage test, conservative
inflation, simplification retry, independent progress validation and final
validation/repair are unchanged. The used importer, strip and export APIs stayed
compatible, so no geometry-mapping rewrite was needed. The new regression test imports and exports a
valid warm start with mirrors, diagonal grain and nonzero clearances, then
checks coordinates, physical-copy identities and independent validity.

## Measurement protocol

Evidence lives under `$BETA_DATA_DIR/marker-bench/astra-engine/`. Binaries are
copied before changing dependency or compiler settings. Every study records
binary SHA-256 hashes, input SHA-256 hashes, seed, workers, CPU samples,
process CPU seconds, results, progress and independent validation. Raw evidence
and generated inputs stay outside git.

`binary-manifest.json` maps each preserved executable's SHA-256 to its exact
engine revisions, compiler, SIMD feature and CPU flag. The old and upstream
AVX2 executables were preserved before the expanded engine-label change, so
both print the upstream package version `sparrow 0.3.0`; use the manifest and
study hashes to distinguish them. Later binaries include the richer label.

Each pair runs the same input, seed, budget and worker count. Baseline/candidate
order alternates. Two task streams each use three separator workers. A pair's
two arms remain adjacent in its stream. There is no own compilation during
the selected timing studies. Other agents remain active on the shared host.
CPU saturation does not establish equal available CPU capacity, so process CPU
time is recorded as an additional diagnostic.

The throughput screen uses shirts, trousers, swim and FreeSewing Simon, seeds
1/2/3, and 10-second search budgets. CPU variants use the same workspace release
profile: opt-level 3, thin LTO, four codegen units. The CPU comparison therefore
does not also change LTO or codegen units. Native SIMD uses a newer nightly
compiler as required by Sparrow; this is a combined compiler/SIMD comparison.

The lead shortened the task while the screen was running. The quality study
therefore uses shirts, trousers and Simon, seeds 1/2/3, at 30 seconds. The
separate Lectra study uses the first 10 of the already frozen 20 tasks from
`marker-bench/lectra/sample-300.jsonl`, at 30 seconds and seed 1. All ten are
Tier A and therefore conservative Tier B eligible. They contain 16 to 508
pieces. The 60-second and 20-task quality studies were deferred by the lead.
No task was replaced based on its score. This small
development set is not a representative sample or a new comparison with Lectra.
Both arms use exactly the same saved converted jobs.

Counters come from Sparrow's `SolutionListener::report_separation_result`.
They count placement-loss evaluations, moves and completed separator iterations.
**Evaluations are not individual polygon collision queries.** Separator time
excludes initial construction and final validation. Nesting-call wall throughput
includes adapter import, construction and validation. Process CPU seconds also
include the benchmark's extra revalidation and JSON serialization. As the search
trajectories diverge, these are fixed-input anytime-search throughput measures,
not identical-operation microbenchmarks. Quality remains the decisive output.

The telemetry feature is disabled by default. Its process-wide counters support
one benchmark task per process, including the adapter's worker threads. Reading
them while unrelated nesting calls run in the same process mixes those calls.
The PowerShell runner only orchestrates processes and reads Windows telemetry;
all geometry, counters, pairing and score aggregation run in Rust.

## Completed short screens

Each row has its own adjacent baseline/candidate pairs: four jobs, three seeds,
10 seconds, 12 valid pairs. Speedups are geometric means of paired ratios.
Quality differences are mean efficiency percentage points; wins/losses compare
quality. Do not multiply rows measured during different periods.

| Candidate versus paired baseline | Evaluations/wall s | Evaluations/separator s | Evaluations/CPU s | Quality gain pp | Win/loss |
|---|---:|---:|---:|---:|---:|
| Upstream 1.1 AVX2 vs old 1.0 AVX2 | 0.867x | 0.859x | 0.905x | -0.014 | 6/6 |
| Upstream stable native vs upstream stable AVX2 | 1.245x | 1.160x | 1.246x | +0.350 | 8/4 |
| Upstream nightly SIMD native vs upstream stable native | 1.222x | 1.306x | 1.329x | +0.706 | 9/3 |
| Experimental Jagua 2 AVX2 vs upstream 1.1 AVX2 | 1.258x | 1.238x | 1.380x | +0.552 | 10/2 |

All 96 screen layouts validated. Mean sampled system CPU was 99.94–100% across
arms; maximum telemetry gap was 2.44 seconds. There were no score exclusions.
The engine upgrade alone was slower. Native compilation and nightly SIMD were
the useful changes in the tested upstream pair. The separate Jagua 2 port won
10/12 wall-throughput pairs and all 12 process-CPU-throughput pairs, but this
short development screen is not a production qualification of that fork.

Stable was Rust 1.98.1 / LLVM 22.1.8. Nightly was Rust 1.101.0-nightly
`b6eeb93c9` (2026-10-18 compiler, 2026-10-19 toolchain) / LLVM 23.1.4.
The host was an i9-11900H, 8 cores / 16 logical CPUs. Every build used the
existing thin-LTO/four-codegen-unit release profile. The first cold baseline
build took 88m23s wall time; a cold full-workspace test build was interrupted
before timing and its partial log retained for transparency. The one-second
same-binary smoke run is excluded because it tested the runner during compilation.

## 30-second stable-native confirmation

This directly compares the old AVX2 build with upstream main/Jagua 1.1 compiled
with stable `target-cpu=native`. It is the priority comparison requested by the
lead, separate from the short-screen SIMD selection. Each garment row has three
seeds and all pairs independently validate.

| Job | Evaluations/wall s | Evaluations/CPU s | Efficiency gain pp | Win/loss |
|---|---:|---:|---:|---:|
| Shirts | 1.026x | 1.055x | +0.3850 | 2/1 |
| Trousers | 0.959x | 0.916x | +0.1131 | 2/1 |
| Simon | 0.951x | 0.872x | -0.4614 | 1/2 |
| All nine pairs | 0.978x | 0.945x | +0.0123 | 5/4 |

This is effectively neutral overall, not a demonstrated marker-quality gain.
All 18 layouts validated. Mean system CPU was 99.999% for both arms; the largest
telemetry gap was 2.223 seconds. The native compilation benefit in the short
within-upstream screen did not establish a gain for the combined upgrade over
the old AVX2 engine at 30 seconds.

The ten Lectra pairs likewise show no clear quality gain: **-0.0053 pp**, six
wins and four losses, all 20 layouts valid without warnings. Wall throughput
was 1.106x and separator throughput 1.120x, but process-CPU throughput was only
1.010x. These throughput means cover **nine** pairs: task 2689 had zero search
evaluations in both arms, making its ratio undefined. Its valid result remains
in the quality denominator. Mean of per-run mean CPU loads was 91.410% versus
91.388%; maximum telemetry gap was 2.448 seconds.

| Lectra task | Efficiency gain pp | Wall throughput ratio |
|---|---:|---:|
| 53060 | +0.4335 | 2.363x |
| 37729 | +0.4128 | 1.524x |
| 2689 | +0.0110 | Undefined: zero evaluations in both arms |
| 33718 | -0.4080 | 0.874x |
| 39737 | +0.4567 | 1.114x |
| 1816 | +1.2607 | 1.058x |
| 33513 | -0.0217 | 0.832x |
| 3914 | +0.2311 | 2.220x |
| 97458 | -2.3359 | 0.322x |
| 49743 | -0.0933 | 1.127x |

The extra wall throughput on Lectra did not translate to shorter markers on
average. The large spread and single Lectra seed make these development results,
not a production-quality claim. Keep task 97458's regression visible. Do not
infer that the small overall mean guarantees non-regression on an individual job.

## Reproduction

Run from `marker/`. Set the assigned `CARGO_TARGET_DIR`, `BETA_DATA_DIR` and
`CARGO_BUILD_JOBS=3`. Preserve each executable before changing flags or sources.

```powershell
$env:RUSTFLAGS = '-C target-cpu=x86-64-v3'
cargo build --release -p beta-marker-nest --example engine_bench --features engine-metrics
$env:RUSTFLAGS = '-C target-cpu=native'
cargo build --release -p beta-marker-nest --example engine_bench --features engine-metrics
cargo +nightly build --release -p beta-marker-nest --example engine_bench --features engine-metrics,simd
cargo build --release -p beta-marker-nest --example engine_summary
```

`engine_bench prepare <academic-dir> <simon-job> <lectra-jsonl> <new-input-dir>`
freezes the inputs. `engine_pairs.ps1` accepts `-Baseline`, `-Candidate`,
`-Inputs`, `-Output`, `-Budgets`, `-Seeds`, `-Suite academic|dev30|lectra`,
`-Workers 3`, `-ShardCount 2` and `-ShardIndex 0|1`. Use a fresh study directory
containing one output directory for each shard. Start the two shards together,
then wait for both before starting a new study. `engine_summary <study-dir>`
writes per-pair and grouped results to `paired-summary.json`.
`engine_matrix.ps1` automates the two streams and the revised screen/quality/
Lectra suites using a directory of the preserved named executables.

For the experimental Jagua 2 port, the local config replaces the Sparrow git
source with the separate compatibility checkout and the Jagua git source with
an unmodified checkout at `4661c08`. After enabling it, run `cargo update -p
jagua-rs@1.1.0 --config <local-config>` so the lockfile actually selects 2.0.0.
Verify `cargo tree -p beta-marker-nest --depth 1 --config <local-config>` shows
one Jagua version before building. Do not commit that config or Cargo.lock.
Set `BETA_MARKER_ENGINE_ID` to an explicit experimental identity for that build.

## Distribution and upstream risk

Never distribute this laptop's unconditional `target-cpu=native` executable.
The native feature list includes AVX-512; many factory PCs cannot execute it.
Even an x86-64-v3 build needs a fallback: CPU age alone is not an ISA guarantee.

A practical release uses a small generic-x86-64 launcher and two engine binaries:
generic x86-64 and x86-64-v3. The launcher checks the entire v3 feature set and
OS-enabled AVX register state before selecting the v3 binary; otherwise it starts
the generic binary. Test both choices, including an older CPU or VM with features
masked. Use the same offline job/result protocol and independent validation.
Do not compile the launcher itself with v3 or native flags. Function-level
multiversion dispatch would need the whole search/collision call graph specialized;
adding a dispatch annotation only around `nest` does not specialize dependencies.

Build flags belong to the lead-owned config. The exact v3 profile proposal is:

```toml
[target.x86_64-pc-windows-gnu]
rustflags = ["-C", "target-cpu=x86-64-v3"]
```

This is a CPU-tier packaging proposal, not a measured speedup over the current
generic production binary: the old comparator in this study was already AVX2.
For the generic fallback, explicitly use `RUSTFLAGS='-C target-cpu=x86-64'`.
Keep the existing release profile exactly as tested:

```toml
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 4
```

For the recommended unchanged production engine, the exact dependency lines are:

```toml
[workspace.dependencies]
sparrow = { git = "https://github.com/JeroenGar/sparrow", rev = "c5e64b3b7a0e102de851f44c1f8dd2520a30633b" }
jagua-rs = { version = "=1.0.0", features = ["spp"] }
```

Do not add the Jagua 1.1 source override to that retained-1.0 configuration.
Keep the old engine label too; if selectively backporting telemetry, update
`ENGINE_ID` to match the retained dependency rather than copying the 1.1 label.
If the lead chooses the tested upstream migration for another reason, replace
those two lines and add the following entry to the existing patch table:

```toml
[workspace.dependencies]
sparrow = { git = "https://github.com/JeroenGar/sparrow", rev = "1fe3e3146110c34d77510636defb737c7ce893c6" }
jagua-rs = { git = "https://github.com/JeroenGar/jagua-rs", rev = "9a0b28d86ec05ef287c0b641269be83b6f29c230", features = ["spp"] }

[patch.crates-io]
jagua-rs = { git = "https://github.com/JeroenGar/jagua-rs", rev = "9a0b28d86ec05ef287c0b641269be83b6f29c230" }
```

For future SIMD experiments, use the opt-in `beta-marker-nest/simd` feature and
pin the release toolchain to `nightly-2026-10-19` to reproduce this compiler.
Keep the ordinary stable default feature set unchanged. The measured library
build command is in Reproduction above. A portable SIMD benchmark must set
`target-cpu=x86-64-v3` explicitly; it cannot inherit a preceding native setting.

Pin full revisions, ship MIT/MPL notices, retain the unmodified Jagua source
corresponding to each distributed build, and regenerate Cargo.lock centrally.
An update of upstream main needs the coordinate/coverage/warm-start regressions,
full independent validation and paired quality checks again. Main can change
public APIs and numerical behavior between releases. The experimental port also
needs broader collision-oracle, real factory and constrained-layout checks before
being adopted as a published fork. No new crate package is introduced: Sparrow
remains MIT; Jagua is the existing explicitly permitted MPL-2.0 exception.
No core model or nesting-option contract change is requested. After any merge,
regenerate the central lockfile, rerun the workspace checks, and repeat combined
fixed-piece, fold, shade and warm-start checks with the other agents' changes.

## Evidence and attribution

Under `$BETA_DATA_DIR/marker-bench/astra-engine/`, the completed study roots are
`screen/upgrade`, `screen/native`, `screen/simd`, `screen/jagua2`,
`confirmation-native/quality`, and `lectra-native/lectra`. Each has a
`paired-summary.json`, shard conditions and run telemetry, results and validation.
The report explicitly gives the nine-pair Lectra throughput denominator; the
updated Rust summarizer also writes throughput-denominator fields when rerun.
The original summary executable already excluded undefined ratios correctly.

The external port patch is
`$BETA_DATA_DIR/sparrow-port/0001-Port-Sparrow-configuration-and-debug-checks-to-unmod.patch`,
from commit `a9446764f3095df945b471e96aa1b919be43c445`, based on upstream
`1fe3e3146110c34d77510636defb737c7ce893c6`. Jagua's tracked sources were not edited.

Academic inputs are the supplied public Sparrow/ESICUP fixtures. Simon is from
the MIT-licensed FreeSewing fixtures already in the repository. Lectra converted
jobs derive from Lallier, Vezard, Pinaud and Blin, *Nesting Tasks Dataset for
2D-Nesting Efficiency Estimation*, version 1.1, CC BY 4.0,
[DOI 10.5281/zenodo.7030786](https://doi.org/10.5281/zenodo.7030786).
No geometry, dataset, build binary or machine-specific configuration is committed.

## Verification and closeout

- The five preserved release benchmark builds succeeded: old AVX2, upstream
  AVX2, upstream native, upstream nightly SIMD/native, and the external Jagua 2
  compatibility port on AVX2. All **134 scored layouts** passed the independent
  validator. This is benchmark validation, not a passing workspace test suite.
- `cargo test --workspace` was attempted with `CARGO_BUILD_JOBS=3` and the
  assigned external target directory. The last cold rebuild ran from about
  21:40 to 22:06 local time on 19 October. It was still compiling test
  executables, with no test results reported, when stopped to honor the lead's
  bounded closeout instruction. **Incomplete; not passed.** No compiler error
  or failing test had been observed before stopping.
- `cargo clippy --workspace --all-targets --features beta-marker-nest/engine-metrics -- -D warnings`
  and `cargo test -p beta-marker-nest --features engine-metrics` were queued
  after that build and **did not run**. The added warm-start and summary
  regressions therefore remain unverified. Do not treat them as passed.
- The report-only commit excludes every engine/source change, the generated
  `Cargo.lock`, and `.cargo/astra-jagua2.local.toml`. The lockfile was restored;
  the experimental override remains untracked. Own build processes were stopped.
- For review and later verification, the complete uncommitted BETA candidate
  (including the benchmark tools) is saved as
  `$BETA_DATA_DIR/marker-bench/astra-engine/uncommitted-upstream-candidate.patch`.
  It contains no local override, lockfile, dataset or machine-specific path.
  Saving that patch does not satisfy the workspace-test commit gate.
- Logs are `workspace-test-final.log` and `closeout-checks.json` in the evidence
  root. Earlier interrupted-build logs are retained. Before any engine commit,
  finish the full workspace test and Clippy commands above, then run
  `cargo test -p beta-marker-nest --example engine_summary` for the zero-evaluation
  denominator regression. No further performance studies were started after
  the final closeout instruction.
