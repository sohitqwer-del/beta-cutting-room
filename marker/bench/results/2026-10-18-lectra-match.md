# Lectra matched-duration comparison — 18 October 2026

**Assumption:** `sheet_type=0` means an ordinary rectangular strip of `sheet_width`, with open single-layer fabric. No public codebook verifies this. `flip_x=1` is interpreted as x → −x before rotation. Tier A is exact conditional on these assumptions. Tier B includes Tier A and restricts each part to a subset of Lectra’s rotations and mirror states.

Lectra’s recorded efficiencies came from its own nesting engine on real client tasks. The paper gives a nominal five-minute budget. Hardware, engine version and worker count are unknown. These runs compare quality at selected search budgets; they do not establish hardware-equivalent speed or full garment-rule coverage.

The local machine was shared. Workspace verification overlapped early 60-second trials, and background CPU load was not controlled. Paired time differences also include stochastic scheduling and load effects.

Tier B fixes one handedness per physical part. It keeps the largest regular 1/2/4-angle subset or continuous rotation expressible with one shared fabric tilt. The converter maximizes summed allowed angular measure, then base-angle count. It records original and retained sets for every sampled part. Other constraint classes, extra parameters and unverified multi-ring shapes remain excluded. A positive sheet length is enforced as max_length.

Differences are BETA minus Lectra, in percentage points. A tie is within 0.1 point. Length saving is 100 × (1 − Lectra efficiency / BETA efficiency); positive means shorter. Numerical statistics use valid layouts only. A too_long failure is a loss with blank numerical scores. Other failures remain separate. The fixed-seed sample balances piece count and Lectra efficiency; its unweighted means are descriptive, not estimates for all customers.

## Population and sample

| Population category | Tasks |
|---|---:|
| All | 100,000 |
| Tier A | 94,239 |
| Tier B including A | 96,747 |
| Tier B only | 2,508 |
| Excluded | 3,253 |

Multi-ring tasks: 0; verified-hole stratum: 0. No holes were silently filled in a headline task.

Population handicap task counts overlap: 1,514 drop optional mirror states; 1,047 restrict angles or tilt.

Sampling seed **7030786**. This run has 60 tasks; 59 are Tier A. The 60-task matched sample is a stratified subset of the original 300, chosen before examining run results. All Lectra trials use nesting seed **1**. Academic trials use seeds 1, 2 and 3.

This run drops optional mirror choices for 0 tasks and angles or tilt for 1 task. These counts can overlap. A finite max_length applies to 5 tasks.

## Results

| Group | Attempts / valid | Mean difference, pp | Median difference, pp | Win / tie / loss | Mean length saving % | Median length saving % | Other failures |
|---|---:|---:|---:|---|---:|---:|---:|
| Tier B headline, includes A | 60 / 60 | -1.0865 | -1.1278 | 20 / 2 / 38 | -1.3668 | -1.6003 | 0 |
| Tier A, exact | 59 / 59 | -1.0788 | -1.0672 | 20 / 2 / 37 | -1.3576 | -1.5492 | 0 |
| Tier B only | 1 / 1 | -1.5415 | -1.5415 | 0 / 0 / 1 | -1.9072 | -1.9072 | 0 |

| Group | Attempts / valid | Mean difference, pp | Median difference, pp | Win / tie / loss | Mean length saving % | Median length saving % | Other failures |
|---|---:|---:|---:|---|---:|---:|---:|
| pieces:1-25;lectra:<75 | 5 / 5 | 1.3350 | 1.1210 | 4 / 1 / 0 | 2.5453 | 1.9308 | 0 |
| pieces:1-25;lectra:75-85 | 5 / 5 | 0.7049 | 0.6426 | 3 / 0 / 2 | 0.8071 | 0.7994 | 0 |
| pieces:1-25;lectra:85+ | 5 / 5 | 0.3317 | 0.4585 | 3 / 0 / 2 | 0.3814 | 0.5306 | 0 |
| pieces:26-75;lectra:<75 | 5 / 5 | 3.1537 | 0.9933 | 3 / 0 / 2 | 4.8198 | 1.6849 | 0 |
| pieces:26-75;lectra:75-85 | 5 / 5 | -0.6032 | -1.3196 | 2 / 0 / 3 | -0.7680 | -1.7258 | 0 |
| pieces:26-75;lectra:85+ | 5 / 5 | -2.0005 | -2.5339 | 0 / 1 / 4 | -2.3846 | -2.9758 | 0 |
| pieces:76-150;lectra:<75 | 5 / 5 | -2.1202 | -2.0439 | 1 / 0 / 4 | -3.2929 | -3.7114 | 0 |
| pieces:76-150;lectra:75-85 | 5 / 5 | -2.2826 | -1.5415 | 2 / 0 / 3 | -3.5033 | -1.9072 | 0 |
| pieces:76-150;lectra:85+ | 5 / 5 | -2.2933 | -2.6369 | 0 / 0 / 5 | -2.7029 | -3.1333 | 0 |
| pieces:151+;lectra:<75 | 5 / 5 | -1.3219 | -1.1884 | 1 / 0 / 4 | -2.0320 | -1.6513 | 0 |
| pieces:151+;lectra:75-85 | 5 / 5 | -4.9176 | -3.6367 | 0 / 0 / 5 | -6.6060 | -4.7544 | 0 |
| pieces:151+;lectra:85+ | 5 / 5 | -3.0240 | -3.9271 | 1 / 0 / 4 | -3.6655 | -4.8184 | 0 |

| Group | Attempts / valid | Mean difference, pp | Median difference, pp | Win / tie / loss | Mean length saving % | Median length saving % | Other failures |
|---|---:|---:|---:|---|---:|---:|---:|
| No finite max_length | 55 / 55 | -0.8512 | -1.0498 | 19 / 2 / 34 | -1.0241 | -1.1815 | 0 |
| Finite max_length enforced | 5 / 5 | -3.6745 | -3.1579 | 1 / 0 / 4 | -5.1363 | -3.8578 | 0 |

## Correlation and differences

Valid pairs: 60. Pearson **0.9592**; Spearman **0.9285** (average ranks for ties).

| Difference quantile | pp |
|---|---:|
| 0% | -8.9374 |
| 5% | -6.5161 |
| 25% | -3.0667 |
| 50% | -1.1278 |
| 75% | 0.7121 |
| 95% | 2.5998 |
| 100% | 15.2550 |

## Runtime

Run window: 2026-10-18 23:08:14 to 2026-10-19 00:10:38, Asia/Kolkata. Wall time **3743.858 s**. Summed nesting time **18254.768 s**; median 309.497 s; maximum 615.032 s. Independent validation adds 24.464 s. Search budgets range from 244 to 608 s. 5 simultaneous tasks × 3 workers. Intel64 Family 6 Model 141 Stepping 1, GenuineIntel; 16 logical CPUs. Engine revision `3022b1285ff7`, dirty worktree `True`. Import, initial construction, final repair and validation add time outside the search budget. Runner wall time excludes compilation, input-file parsing, setup and final summary writing.

## Failures

All attempted layouts passed independent validation.

## Paired time effect

Matched budgets use the recorded Lectra duration without the old 300-second cap (`--match-cap 10000`). The same seed and worker count are used. Separate time-limited stochastic runs can regress as well as improve.

Recorded budgets: minimum 244 s, median 304 s, maximum 608 s. Two tasks (96058 and 60855) record 608 seconds. The nominal five-minute description does not explain these values. They are retained without inventing an overhead explanation.

| Group | Attempts / valid | Mean difference, pp | Median difference, pp | Win / tie / loss | Mean length saving % | Median length saving % | Other failures |
|---|---:|---:|---:|---|---:|---:|---:|
| Same subset at 60 s | 60 / 60 | -2.5276 | -2.9054 | 18 / 1 / 41 | -3.5077 | -3.6243 | 0 |
| Matched duration | 60 / 60 | -1.0865 | -1.1278 | 20 / 2 / 38 | -1.3668 | -1.6003 | 0 |

Valid paired runs: 60. Mean gain from extra time: **1.4411 pp**; median **0.7624 pp**. Improved / within 0.1 pp / regressed: 42 / 14 / 4.

Same subset at 60 seconds: Valid pairs: 60. Pearson **0.9251**; Spearman **0.8971** (average ranks for ties).

The observed change closes 57.0% of the paired subset’s mean efficiency deficit. Background load and stochastic scheduling remain confounders.

## Attribution and evidence

Lallier, Vézard, Pinaud and Blin (2022), *Nesting Tasks Dataset for 2D-Nesting Efficiency Estimation*, [Zenodo 7030786](https://doi.org/10.5281/zenodo.7030786), version 1.1, **[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/)**. Conversion changes coordinates to mm, fixes one handedness per part, and may restrict rotations. No geometry is published.

The four source checksums match the pinned Zenodo files. The restricted unpickler and explicit global allow-list remain in use. Full per-task metadata, Job inputs, CSV results and run configurations stay in the external data directory. No raw geometry or build output is committed.

An independent metadata audit checked complete retained-interval inclusion and quantity preservation for all **35,630 physical parts** in the 300-task sample. This checks the declared orientation sets; the solver’s actual placements also pass the independent Job validator.
