# GPU raster placement: measured partial results

**Do not enable GPU warm starts by default.** The GPU executes the raster scans,
but the available paired results have lower mean efficiency than plain nesting.
Gap filling improved none of the 11 tested baselines. Keep this crate as an
optional research path. These measurements do not establish a production win.

Date: 19 October 2026. Branch: `marker/gpu`, based on `63c21af`.
The proposal algorithm is frozen at `c008339`; later commits improve benchmark
recovery and provenance. The final runner is `7773e60`. Only this crate is
changed. Sparrow, Jagua, the core contract, parent workspace and CLI are
unchanged. No geometry or lockfile is committed. No window was opened.

## Paired efficiency

Differences are GPU-warm efficiency minus plain efficiency, in **percentage
points**. Means are unweighted across pairs with two valid results. A difference
within 0.1 point is a tie. Failed jobs are listed separately; they receive no
efficiency and are not counted as ties.

| Nominal allowance | Completed Lectra pairs / planned 20 | Valid pairs | Plain mean | GPU-warm mean | Mean difference | Win / tie / loss |
|---|---:|---:|---:|---:|---:|---:|
| 10 s, partial | 13 / 20 | 12 | 74.614% | 74.018% | **-0.596 pp** | 6 / 1 / 5 |
| 30 s, partial | 12 / 20 | 11 | 75.719% | 75.489% | **-0.230 pp** | 5 / 2 / 4 |
| 60 s, partial | 4 / 20 | 4 | 78.810% | 78.673% | **-0.137 pp** | 1 / 1 / 2 |

The 10 s differences range from -6.537 to +2.296 points. The 30 s range is
-2.978 to +1.334 points. A win count alone hides the larger losses.

All returned layouts passed `beta_marker_core::validate`: 54 layouts across
27 valid pairs. Four attempted runs returned errors rather than layouts, making
two both-failed pairs. Standby-affected measurements are excluded.

### Missing cells

The planned Lectra subset, in order, was:

```text
8411 1561 88853 14851 18110 4122 18896 58688 12127 89590
20628 77578 41118 77136 71193 89319 56750 48543 45886 58904
```

- 10 s: completed through task 41118, the first 13 tasks. The last seven are unmeasured.
- 30 s: completed through task 77578, the first 12 tasks. The last eight are unmeasured. The interrupted task-41118 cell has no completed pair.
- 60 s: only the first four tasks are retained. The other 16 cells are unmeasured.
- FreeSewing Simon, Teagan and Titan ratio jobs: **no paired cells completed**, at any budget. They are not represented by zeroes or included in the means.

The lead requested stopping measurements after restart/standby interruptions.
This is a partial suite, not the originally requested full 20-task and
FreeSewing comparison. No unfinished or missing cell is presented as a tie.

### Failed job

| Job | Allowances | Plain result | GPU-warm result |
|---|---|---|---|
| Lectra 12127 | 10 s and 30 s | Sparrow could not construct or optimise a marker | Raster proposal: piece `10` does not fit the raster width; normal nesting also failed |

This is not proof that the original job is infeasible. The raster is
conservative, and the CPU engine has numerical guards. The engine owner should
inspect the saved job and errors. No constraint was relaxed to obtain a result.

## GPU and CPU cost

Device: **NVIDIA GeForce RTX 3050 Ti Laptop GPU, 4 GB, driver 610.60**.
The measured backend was Vulkan. DX12 is compiled but was not hardware-tested.
No CUDA toolkit is used.

Times below are successful constructor trials associated with complete pairs.
GPU time comes from timestamp queries around compute passes. Host scan time
includes allocation, upload, submission, synchronization and readback. Total
proposal time also includes CPU geometry work and independent validation.

| Allowance / subset | Trials | Proposal mean s | Proposal range s | Host scan mean s | GPU compute-pass mean s |
|---|---:|---:|---:|---:|---:|
| 10 s subset | 12 | 2.802 | 0.236-12.454 | 2.146 | 0.341 |
| 30 s subset | 11 | 4.203 | 0.283-32.483 | 3.513 | 0.294 |
| 60 s subset, partial | 4 | 7.007 | 0.132-26.728 | 6.717 | 0.267 |

The scalar CPU raster constructor completed 12 of 13 attempted jobs, averaging
**7.393 s**, range **0.539-19.313 s**. Task 12127 failed on width. These are the
same successful job IDs as the 10 s GPU constructor subset. This compares the
raster initializer on CPU/GPU; it is not a speedup figure for the complete nest.

On the 367-piece task 14851, GPU construction took **11.076 s**, versus
**11.647 s** on CPU. Its GPU compute passes totaled only **0.583 s**; host scan
cost was **10.170 s**. Per-orientation allocation/upload and synchronous
readback dominate this prototype on that case. A later construction of the
same job took 26.728 s under different shared-host load. The few-hundred-
millisecond hypothesis does not hold for large jobs here.

An earlier construction-only Simon smoke test produced identical CPU/GPU
placements and passed validation: GPU 0.594 s, CPU 15.253 s, efficiency 78.73%.
It precedes the paired freeze and is **not** evidence of better Simon nesting
at 10/30/60 s. Teagan and Titan were not measured.

### Utilization samples

| Device-wide sample set | Samples | Mean GPU utilization | Peak |
|---|---:|---:|---:|
| Recorded proposal windows | 68 | 12.22% | 74% |
| All retained current/prior telemetry | 2,557 | 0.68% | 85% |

Peak recorded GPU memory was **203 MiB**. These are device-wide samples, not
process attribution. The full set includes CPU-refinement idle periods and
retained prior/aborted-session telemetry. Sampling pauses 500 ms plus subprocess
latency, so bursts can be missed. Compute-pass timestamps separately measure
this crate's GPU work.

## Gap filling

There were **11 eligible baseline trials**: four after 60 s and seven after
30 s. Failed, interrupted and unreached cells had no eligible baseline.
Each attempt removed up to 12 small tail pieces and reinserted them around the
remaining placements. Locked placements stayed exact.

- Improved / unchanged / regressed: **0 / 11 / 0**.
- Mean efficiency change: **0.000 pp**.
- Mean extra time: **0.853 s**, range **0.095-4.276 s**.

The valid incumbent was retained when reconstruction failed or did not shorten
it. A synthetic loose-tail test demonstrates valid reinsertion, but these
optimized-baseline results do not justify a production GPU "Fill gaps" feature.

## Protocol and interruptions

Both variants use seed 1, one run and two separator workers. Pairs run
sequentially; first-variant order alternates by job and budget. The GPU variant
constructs a 2 mm proposal and subtracts preparation wall time from its search
allowance. The unchanged adapter adds import, construction, repair and checking
overhead. These are nominal search allowances, **not hard elapsed deadlines**.
Some constructors exceeded the allowance. Actual wall times are saved per run;
summary wall means include quick failed attempts.

The host was an Intel i9-11900H, 8 cores / 16 logical CPUs, shared with other
agents. CPU load sampled at 100% during preparation and the initial pair start.
This agent finished compilation before measurement batches. Other load was
not controlled. One reusable device was initialized per batch; setup cost is
recorded separately in `device.json` and excluded from pair allowances.

Windows Kernel-Power events confirmed Modern Standby from about 19:23 to
20:46. Telemetry included a 4,899.630 s gap. Task 1561's 30 s and 60 s pairs
overlapped standby; both variants were discarded and rerun. Original data is
retained. Resume audits detect gaps inside each actual sampler session;
shutdown/restart gaps are not treated as standby. Reuse is bound to byte-
identical canonical job JSON, including geometry and rules.

Fixed seeds do not remove worker-scheduling and wall-clock cutoff noise. A
clean repeat of task 88853 changed its 10 s GPU-warm efficiency by 3.43 points
under the same seed. Tables use one designated run per cell, not the best
repeat. This busy development-host sample does not establish held-out gains
or a universal conclusion about other GPU algorithms.

## Verification

Run Cargo from `marker/` with the assigned external `CARGO_TARGET_DIR`,
`CARGO_BUILD_JOBS=3` and `BETA_DATA_DIR`.

| Command | Result |
|---|---|
| `cargo test --workspace` | 253 passed, 5 existing tests ignored |
| `cargo clippy --workspace --all-targets -- -D warnings` | Passed |
| `cargo test --release --manifest-path crates/beta-marker-gpu/Cargo.toml` | 12 passed, 1 hardware test ignored |
| `cargo test --release --manifest-path crates/beta-marker-gpu/Cargo.toml gpu_matches_cpu -- --ignored` | 1 passed on RTX / Vulkan |
| `cargo clippy --manifest-path crates/beta-marker-gpu/Cargo.toml --all-targets -- -D warnings` | Passed |
| `cargo build --release --manifest-path crates/beta-marker-gpu/Cargo.toml --bins` | Passed |

Nine library tests cover concave/mirrored pieces, gaps, ratios and bundles,
blocks/shrinkage, bad input, missing-adapter fallback, bitmap shifts, native warm
import, fixed placements, gap-fill locks, non-regression and stale editor
metrics. Three runner tests cover standby overlap, sampler restarts and the
partial-budget plan. Hardware parity crosses the 65,535-workgroup boundary.
A physical GPU-less machine and DX12-only device have not been tested.

All resolved dependency licence expressions were checked. See
[LICENSES.md](LICENSES.md) for versions and permitted choices, including
target-specific packages. Direct packages include wgpu 30.0.1, bytemuck 1.25.2,
pollster 0.4.0, geo 0.33.1, anyhow, serde and serde_json; their choices are MIT,
Apache-2.0 or Zlib. Jagua remains the existing unmodified MPL-2.0 exception.
Sparrow's MIT LICENSE was checked. No engine source is vendored or patched.

## Evidence

Root: `$BETA_DATA_DIR/marker-bench/gpu-v04/`.

- `short-10-30-final/`: 50 recorded short-budget attempts, job copies, manifest,
  conditions, proposals, results, validation, CPU timings, gap-fill results,
  telemetry and `summary.json`.
- `revised-final-20-10x60/`: the four completed 60 s pairs used here. Its other
  cells are not substituted into the short-budget table.
- `paired-20-2mm/` and recovery directories: original/discarded evidence,
  timestamp audits and recovery lineage. `standby-events.json` records power events.
- Test logs, hardware snapshots and the Simon construction-only smoke test
  remain at the root. All geometry stays outside git.

Input: `$BETA_DATA_DIR/marker-bench/quality-v03/conversion/development-40.jsonl`.
The planned subset was its first 20 records, plus the three FreeSewing ratio
fixtures. It has 12-367 pieces per Lectra job. This compares BETA against itself
on converted jobs, not against commercial Lectra results. Dataset attribution:
Lallier et al. (2022), *Nesting Tasks Dataset for 2D-Nesting Efficiency
Estimation*, Zenodo 7030786, version 1.1, CC BY 4.0. See the
[conversion audit](../../bench/lectra/README.md) for source and assumptions.
FreeSewing fixtures are MIT; attribution is in
[their README](../../testdata/open/freesewing/README.md).

```text
cargo run --release --manifest-path crates/beta-marker-gpu/Cargo.toml --bin gpu-summary -- <evidence-directory>
```

The report can be reproduced from saved records without more measurements.

## Integration decision and limits

Keep production nesting on the current CPU path. Do not add automatic GPU
selection based on this evidence. If the lead wants the prototype available,
register it behind an optional Cargo feature and explicit experimental
`beta-marker nest --gpu` flag. Reuse a `Proposer` and call
`beta_marker_gpu::nest(job, options, proposer)`. Existing fixed placements and
user warm starts keep their native path. With no GPU, automatic mode uses
native CPU nesting rather than a slow scalar raster warm start.

The lead must register the crate, remove its local `[workspace]` and duplicate
profiles, regenerate the lockfile and repeat the licence audit. No core model
field is requested. Keep the dependency direction GPU crate -> nest; GPU
ownership inside `NestOptions` would risk a cycle. See [README.md](README.md).

The raster uses conservative vertical-span hulls and square dilation. It loses
narrow pockets and some concavities. Free rotation is sampled at quarter turns;
tilt is not sampled. Open fabric, prescribed mirror counts, ratios, blocks and
shrinkage are supported. Folded/tubular fabric, coupled bundle nap, defects,
shade and repeat/matching use the normal adapter. Thousands of alternate
layouts or piece orders were not evaluated.

The measured bottleneck is host coordination around many small dispatches.
Resident buffers, batched orientations and fewer readbacks are follow-up ideas,
with no measured quality benefit claimed. Editor placement previews, digitizing
and rendering were not benchmarked. After merge, re-test bundle ordering,
block/shrinkage metrics, fixed layouts and coupled-rule fallbacks. Repeat on an
idle host and held-out jobs before any production promotion.
