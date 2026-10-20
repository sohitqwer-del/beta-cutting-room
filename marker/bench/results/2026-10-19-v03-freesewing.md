# FreeSewing v0.3: three 60-second jobs

Release build, seed 1, two tasks in parallel, three workers per task. Fabric is
open and 1500 mm wide. All three layouts passed independent validation without
warnings. Median sampled CPU was 100%; no CPU or sampling-gap outlier was found.
These are shared-machine results. Search budgets are 60 seconds; runtime includes
construction and checking overhead. Lengths are mm. Useful demand and occupied
efficiency agree for these single-layer jobs.

| Run | Job | Arm | Length | Occupied efficiency % | Demand efficiency % | Runtime s | Valid |
|---|---|---|---:|---:|---:|---:|---|
| freesewing-60s | simon-ratio-as-is | run | 7402.9986 | 88.1897 | 88.1897 | 64.2858 | true |
| freesewing-60s | teagan-ratio-as-is | run | 4691.3441 | 76.5062 | 76.5062 | 62.2702 | true |
| freesewing-60s | titan-ratio-as-is | run | 7261.9231 | 75.3237 | 75.3237 | 62.7144 | true |

Raw jobs, layouts, validation, timing and CPU samples are retained outside git
under `$BETA_DATA_DIR/marker-bench/quality-v03/freesewing-60s`.
FreeSewing fixtures are MIT-licensed. Historical 120-second figures and their
different conditions appear in the [handoff](2026-10-19-v03-handoff.md).
