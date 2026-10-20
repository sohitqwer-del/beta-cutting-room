# Lectra conversion audit

ASSUMPTION: sheet_type 0 is an ordinary rectangular strip. Tier A is exact conditional on that assumption and flip_x meaning x -> -x. Tier B includes A and uses a subset of orientations; codes 2/3 remain diagnostic only.

Source: Lallier, Vezard, Pinaud and Blin (2022), *Nesting Tasks Dataset for 2D-Nesting Efficiency Estimation*, [Zenodo 7030786](https://doi.org/10.5281/zenodo.7030786), version 1.1, CC BY 4.0.

Population 100,000; Tier A 95,661; Tier B including A 96,747; excluded 3,253.

Seed 7030786. The 300-task sample contains 296 Tier A tasks and 4 Tier B-only tasks. The diagnostic samples contain 20 code-2 tasks and 19 code-3 tasks.

There are 403 otherwise eligible code-2 tasks and 19 code-3 tasks. Population handicap counts overlap: 0 tasks drop optional mirror states; 1,086 restrict angles or tilt.

Tier B pre-transforms arbitrary base angles and uses the largest regular 1/2/4-angle subset or continuous free rotation. Each piece has its own tilt; optional reflection is retained when the opposite-handed set contains the reflected retained angles. Each sampled job records the original and retained set per part. flip_x is assumed to reflect x before rotation. All other constraint classes and extra parameters remain excluded. Multiple rings remain excluded unless verified; none is silently dropped from a headline job.

| Sheet-type code | Tasks |
|---|---:|
| 0 | 99,403 |
| 2 | 570 |
| 3 | 27 |

| Exclusion reason (overlapping) | Tasks |
|---|---:|
| sheet_type_not_headline | 597 |
| unsupported_constraint:c1 | 71 |
| unsupported_constraint:c2 | 10 |
| unsupported_constraint:c3 | 15 |
| unsupported_constraint:c4 | 9 |
| unsupported_constraint:c5 | 11 |
| unsupported_constraint:c6 | 14 |
| unsupported_constraint:c7 | 2 |
| unsupported_constraint:c8 | 124 |
| unsupported_constraint:g1 | 1,100 |
| unsupported_constraint:g2 | 438 |
| unsupported_constraint:g3 | 553 |
| unsupported_constraint:g4 | 307 |
| unsupported_constraint:t1 | 121 |
| unsupported_constraint:t2 | 201 |

| Primary exclusion reason | Tasks |
|---|---:|
| sheet_type_not_headline | 597 |
| unsupported_constraint:c1 | 71 |
| unsupported_constraint:c2 | 10 |
| unsupported_constraint:c3 | 15 |
| unsupported_constraint:c4 | 1 |
| unsupported_constraint:c5 | 7 |
| unsupported_constraint:c8 | 63 |
| unsupported_constraint:g1 | 1,097 |
| unsupported_constraint:g2 | 438 |
| unsupported_constraint:g3 | 548 |
| unsupported_constraint:g4 | 256 |
| unsupported_constraint:t1 | 114 |
| unsupported_constraint:t2 | 36 |

| Sample stratum | Tasks |
|---|---:|
| pieces:1-25;lectra:75-85 | 25 |
| pieces:1-25;lectra:85+ | 25 |
| pieces:1-25;lectra:<75 | 25 |
| pieces:151+;lectra:75-85 | 25 |
| pieces:151+;lectra:85+ | 25 |
| pieces:151+;lectra:<75 | 25 |
| pieces:26-75;lectra:75-85 | 25 |
| pieces:26-75;lectra:85+ | 25 |
| pieces:26-75;lectra:<75 | 25 |
| pieces:76-150;lectra:75-85 | 25 |
| pieces:76-150;lectra:85+ | 25 |
| pieces:76-150;lectra:<75 | 25 |

Multi-ring tasks: 0; verified-hole tasks: 0.

All four pinned MD5 checks passed before restricted unpickling. Geometry and the complete per-task audit stay outside git. This stratified sample is unweighted, not a population estimate.
