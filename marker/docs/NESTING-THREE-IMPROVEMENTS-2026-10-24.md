# All three nesting changes: implementation and pilot, 24 October 2026

Branch: marker/efficiency9. Baseline engine: db5f4ff (the prior efficiency8 work). Implemented core: a8b1a3f; refined layer ranking and browser warm-start metadata: 63b4571. The published 0.7.0 application has not been updated.

## What is implemented

1. **Mixed folded/full cuts per piece and size.** The layer allocator can compare masks that pair useful copies on both layers for some piece IDs while retaining fold halves for others. Piece IDs are size-specific. Candidates are ranked by complementary length/depth, rather than forcing all narrow panels onto the fold. Exact layer demand, odd quantities, fixed allocations and garment-direction checks remain mandatory. The number of trials is limited by the search budget.
2. **Stronger search around locks and interior defects.** A compact obstacle-free Sparrow proposal is repaired against the original fixed pieces and defects. Incomplete or invalid proposals are discarded. The bounded packer can also remove and reinsert groups of movable pieces and processes the open/constrained marker's tail first. Only a complete, independently validated layout replaces the incumbent. A changed lock cannot be bypassed with an older warm start.
3. **Browser state between rounds.** Compatible plain open jobs keep a Sparrow separator, imported geometry, worker RNGs, feasible solution pool, collision penalties and unfinished separation attempts. Constrained jobs retain prepared outlines, order, RNG, incumbent and the move cursor. Folded/allocated jobs retain a short repair workspace alongside native policy probes; their complete Sparrow policy search is not itself fully resumable. Changed physical allocations rebuild the incompatible workspace. Cancellation returns the last valid complete marker. Duplicate warnings are removed, and legacy warm-start bundle labels are normalized.

The public NestOptions/BrowserOptions and Worker method signatures are unchanged. No dependency was forked or modified; no dependencies or licenses were added.

## Verification

- 68 release-mode beta-marker-nest tests passed, zero failed/ignored.
- Strict Clippy passed for all targets of the nesting crate.
- The complete beta-marker-wasm package built for wasm32-unknown-unknown and its matching wasm-bindgen bindings were generated.
- The independent Rust audit replayed **50 final native layouts**: 12 cases x 2 seeds x before/after, plus the original mixed-panel pair. Every layout passed geometric/quantity/grain/gap validation. Every fixed record compared exactly equal.
- Actual Chromium Worker smoke checks covered plain, early-locked and mixed-fold jobs. All 11 work-step responses and all three Stop responses validated. Fixed transforms remained exact; Stop retained the prior length. The plain model's diagnostic advanced from 1,276 to 2,018 to 3,126 cumulative evaluations while retaining one imported model.
- Native/warm correctness tests include a valid old marker with a newly moved lock, an impossible new maximum-length/lock combination, invalid partial hints crossing a defect, mixed useful layers without surplus, monotone browser incumbents, model reuse and cancellation.

## Native pilot: all results

Public FreeSewing ratio jobs were used. Folded/tubular widths are half the original open width. Declared synthetic interior defects and identical exact fixed records were applied to both variants. Two preselected initial seeds (1, 7); 6 seconds requested per run; one requested separator worker. Runs were sequential, without concurrent builds. Variant order was reversed for seed 7.

Positive fabric saving means less length for the same width, layout and required pieces. Negative values mean more fabric. Useful yield excludes surplus and includes both layers on folded/tubular fabric. It is not the occupied-footprint percentage.

| Case | Old useful yield % | New useful yield % | Fabric saved % | Mean wall s, old / new |
| --- | ---: | ---: | ---: | ---: |
| Hoodie folded | 65.41 | 64.19 | -1.90 | 7.73 / 7.16 |
| Hoodie tubular | 62.24 | 61.71 | -0.84 | 19.04 / 11.53 |
| Shirt BrowserSession proxy | 81.34 | 82.34 | 1.23 | 6.88 / 6.10 |
| Shirt early lock | 73.56 | 77.71 | 5.16 | 6.02 / 7.19 |
| Shirt interior defect | 73.98 | 75.72 | 2.29 | 6.03 / 8.65 |
| Shirt original lock | 70.63 | 75.04 | 5.66 | 6.03 / 6.75 |
| T-shirt folded | 61.67 | 61.67 | 0.00 | 6.04 / 6.05 |
| T-shirt tubular | 68.08 | 68.54 | 0.67 | 6.67 / 7.52 |
| T-shirt BrowserSession proxy | 70.71 | 72.86 | 2.93 | 6.27 / 6.37 |
| T-shirt early lock | 63.65 | 63.11 | -0.86 | 6.02 / 6.02 |
| T-shirt interior defect | 64.29 | 63.95 | -0.53 | 6.01 / 6.03 |
| T-shirt original lock | 62.83 | 62.83 | 0.00 | 6.05 / 6.03 |

[Every paired value and range](efficiency9/summary.json). BrowserSession rows are native proxies, not phone/browser timings. Search initialization and upstream separator strikes may exceed the requested budget; actual wall time is shown.

The shirt lock/defect cases and BrowserSession proxies improved in this pilot. The T-shirt constrained cases were flat or slightly worse. Hoodie fold cases regressed slightly. This small pilot does not establish a general improvement or production non-regression. Valid supplied incumbents are retained when new candidates are worse.

The original-lock cases use the first placement of a reference layout. Early locks use its smallest physical x-end. Neither is assumed to define the marker's total length. Both sets of exact lock records were included in the independent audit.

## Mixed-allocation witness

An original two-panel fixture uses a 100 mm folded width, 2 mm piece gaps, 1 mm edge clearance, and two required copies of each panel. The established global policies use 123.002 mm. A useful full cut for the long panel plus two on-fold cuts for the short panel use 83.002 mm, or 32.52% less fabric. There are no surplus copies and no relaxed rules. This is a synthetic regression, not a garment efficiency claim. [Measured values](efficiency9/mixed-summary.json).

## Actual browser comparison

One public T-shirt job, initial seed 1, 8-second host search window after Worker loading, 0.25-second requested rounds:

| Variant | Marker m | Wall s | Completed steps | Valid |
| --- | ---: | ---: | ---: | --- |
| Before | 5.355 | 8.36 | 5 | true |
| After | 5.274 | 8.21 | 12 | true |

[Raw timed Worker responses](efficiency9/wasm-timed.json) and [functional state/lock/Stop checks](efficiency9/wasm-smoke.json). This is one desktop Chromium run, not a broad browser-speed result. Stop is cooperative at round boundaries; it cannot interrupt an upstream strike mid-call. Existing host checkpoint/watchdog recovery still applies.

## Reproduce and artifacts

The Rust examples three_study and three_audit own generation, search, metrics, geometry validation, exact fixed-record checks and aggregation. Generate one common case file and give that same file to both binaries. Keep it frozen: reference-layout construction can vary with wall-clock scheduling. Public source jobs and their notices remain in testdata/open/freesewing.

Build the driver against baseline db5f4ff in an isolated worktree and against the candidate branch. Run from marker/; use the full path to the built examples under the configured Cargo target directory (and .exe on Windows):

~~~sh
cargo build --release -p beta-marker-nest --example three_study --example three_audit --offline
three_study --prepare . OUT/cases.json
three_study OUT/cases.json OUT/results-1.jsonl 6 1
three_study OUT/cases.json OUT/results-7.jsonl 6 7
three_audit OUT/cases.json BEFORE_FILES_CSV AFTER_FILES_CSV OUT/summary.json
~~~

The same portable driver can be copied into the baseline worktree; it uses existing public APIs. Exact common inputs, original panel fixture, raw layouts, binaries, build/test logs, browser harness and interactive comparison are retained in the task's external efficiency9 data folder. Aggregates and browser checks are committed here. [Artifact hashes and environment](efficiency9/environment.json).

Only public FreeSewing geometry and original synthetic fixtures were used. No private factory patterns, commercial formats/specifications, external compute servers or native GUI windows were used. The improvements remain local and experimental; broader fixed/defect/folded matrices and factory-pattern comparison are required before release.
