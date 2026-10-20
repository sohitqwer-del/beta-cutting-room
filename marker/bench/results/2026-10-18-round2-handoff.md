# Bench round-2 handoff

Runs started on 18 October 2026. The matched run finished on 19 October, Asia/Kolkata. All runs use the round-2 nesting engine from `3022b12`. Core, nest, SPEC and workspace Cargo.toml were not edited.

**Assumptions:** sheet_type 0 means an ordinary rectangular strip of sheet_width, with open single-layer fabric. flip_x means reflection of the x coordinate before rotation. Tier A is exact conditional on those assumptions. Tier B includes A and retains a subset of source orientations. Other constraint classes remain excluded.

## Counts and results

The full audit found **94,239 Tier A tasks**, **96,747 Tier B tasks including A**, and **3,253 excluded tasks**. Tier B-only: 2,508. There are no referenced multi-ring tasks; the verified-hole stratum is empty. [Population exclusions and policy](2026-10-18-lectra-round2-policy.md).

Differences are BETA minus Lectra, in percentage points. A tie is within 0.1 point. Length savings are means of per-task relative savings; negative means BETA uses more length.

| Comparison | Valid / attempted | Mean difference, pp | Median difference, pp | Win / tie / loss | Mean length saving |
|---|---:|---:|---:|---|---:|
| Tier B, 60 seconds | 300 / 300 | -2.3556 | -2.9653 | 68 / 10 / 222 | -3.6211% |
| Tier A, 60 seconds | 293 / 293 | -2.3328 | -2.9024 | 68 / 10 / 215 | -3.6110% |
| Same 60-task subset, 60 seconds | 60 / 60 | -2.5276 | -2.9054 | 18 / 1 / 41 | -3.5077% |
| Tier B, matched duration | 60 / 60 | -1.0865 | -1.1278 | 20 / 2 / 38 | -1.3668% |
| Tier A, matched duration | 59 / 59 | -1.0788 | -1.0672 | 20 / 2 / 37 | -1.3576% |

Extra time improves paired efficiency by **1.4411 points on average**, with a median gain of **0.7624**. Improved / within 0.1 point / regressed: **42 / 14 / 4**. The observed change closes 57.0% of the paired subset's mean deficit. The host was shared, early 60-second trials overlapped workspace checks, and background CPU load was uncontrolled. Time and scheduling effects are not isolated.

The 300-task sample has 25 tasks per stratum. The matched subset has five per stratum. Sampling seed: 7030786; nesting seed: 1. The sample is unweighted. Three sampled tasks drop mirror choices and four restrict angles or tilt. The matched subset has one task with a tilt/angle handicap. All 25 finite-length tasks at 60 seconds, and all five in the matched subset, respect max_length. No too_long failure occurred; the reporting test verifies that such a failure counts as a loss.

The paper describes a nominal five-minute Lectra budget. Recorded matched durations range from **244 to 608 seconds**, with median **304**. Tasks 96058 and 60855 record 608 seconds. Their cause is unknown. The run uses `--match-cap 10000` so those values are preserved. Lectra hardware, version and worker count remain unknown.

Wall time: **3599.976 seconds** for the 300-task run; **3743.858 seconds** for matched duration. Five tasks run concurrently with three workers each, at the requested ceiling of 15 search workers. Runner wall time excludes build/setup/input parsing and final summary writing.

Full results: [60 seconds](2026-10-18-lectra-60s.md), [matched duration](2026-10-18-lectra-match.md), and [benchmark documentation](../../docs/benchmarks.md).

## Sheet-type evidence

Code-0 correlation at 60 seconds: **Pearson 0.8517; Spearman 0.8559**, 300 valid pairs. Matched duration: **0.9592 / 0.9285**, 60 valid pairs. The code-0 differences have wide tails. Their 5th/50th/95th percentiles at 60 seconds are -9.7790 / -2.9653 / 5.6572 points. The codebook remains unverified.

Code 2: **20/20 valid**, mean/median differences **-1.7249 / -3.0304 points**; win/tie/loss **6/2/12**. No analytic width contradiction was found. These are diagnostics, outside the headline.

Code 3: **15/19 valid**, mean/median differences among valid layouts **8.7419 / -3.8454 points**. The four construction failures are exactly the four tasks whose pieces cannot fit the stated width under the full source orientation sets. Required width is 1.243–1.692 times the stated width. Different sheet semantics, such as folded or tubular fabric, are plausible; the actual meaning is unknown. Eight other code-3 tasks have unsupported constraints and were excluded.

Code 0 has no width-fit contradiction. Task 63488 has a reference-implied length 0.02916% below an analytic bound, equivalent to **0.01249 percentage point** of efficiency. This small anomaly remains unexplained. It is below the tie threshold. The strip assumption is useful for a conditional comparison, with denominator and precision uncertainty retained.

## Academic and verification

[Academic rerun](2026-10-18-academic-round2-60s.md): **all 21 trials are valid**, including shirts seed 1. Mean/median differences from published means: **-2.3730 / -2.3398 points**; win/tie/loss **0/0/21**. This uses seven instances, three seeds and 60-second budgets. The published reference used 1200 seconds and different hardware. BETA uses sparrow itself.

`cargo test --workspace`: **70 passed, two intentionally ignored, no failures**. `cargo clippy --workspace --all-targets`: **passed with no warnings**. Converter self-tests passed. Independent interval/quantity audits passed for 35,630 physical parts in the main sample, 6,416 in the matched subset, 2,020 in code 2 and 1,651 in code 3. Analytic projection bounds were also checked against dense angular scans.

The restricted unpickler, explicit global allow-list and pinned checksums remain in use. No new dependencies were added. No successful academic, main or diagnostic trial had engine/validator warnings in the completed 60-second runs.

## Handoff and owner requests

Only summaries and benchmark documentation are committed, as requested. The converter, runner, CLI diagnostic gate and two reporting/checking scripts remain as uncommitted tooling changes in this worktree. [The evidence manifest](2026-10-18-round2-manifest.md) gives their SHA-256 hashes and the external input hashes. No geometry, dataset, build output or Cargo.lock change is committed.

- **Core/contract owner:** support optional handedness for each physical copy, while preserving fixed normal/mirrored quantities. This would remove the mirror handicap on 1,514 eligible tasks. The existing fabric allow_flip flag does not provide a search choice for a fixed physical copy.
- **Core/contract and nest owners:** consider per-piece tilt and arbitrary rotation sets. Angle/tilt restrictions affect 1,047 eligible tasks. Fifty-three tasks have both mirror and angle/tilt restrictions. No core change was made in this role.
- **Dataset/domain owner:** confirm sheet_type and flip-axis meanings, explain the two 608-second durations, and resolve finite-sheet denominator/coordinate-precision details, including task 63488. Keep codes 2/3 and unsupported classes out of the headline until their rules can be honoured.

Data attribution: Lallier, Vézard, Pinaud and Blin (2022), *Nesting Tasks Dataset for 2D-Nesting Efficiency Estimation*, [Zenodo 7030786](https://doi.org/10.5281/zenodo.7030786), version 1.1, [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). Conversion changes units to mm and may restrict orientations. No raw geometry is included.
