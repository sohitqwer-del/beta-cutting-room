# BETA Marker academic tooling smoke check

These nine public inputs converted successfully before the working engine was merged. The nesting skeleton returned an error for every run. This is a tooling check with no valid efficiency comparison.

Revision: e331812dae955b1403a94d85456ddc2634e32d95. Dirty worktree: Some(true). Platform: windows / x86_64.

9 runs; 0 valid. Wall time 0.001 s; summed nesting time 0.000 s. Parallel tasks: 1; threads per task: 3. Exact seeds and budgets are in run.json.

Differences are percentage points (BETA minus reference). Tie: absolute difference <= 0.1 point. Invalid and failed runs are excluded from statistics and have blank numeric results. Academic reference is the published mean. Relative length saving is 1 - reference / BETA efficiency.

| Stratum | Runs | Valid comparisons | Mean difference | Median difference | Win / tie / loss |
|---|---:|---:|---:|---:|---|
| All | 9 | 0 | — | — | 0 / 0 / 0 |
| albano | 1 | 0 | — | — | 0 / 0 / 0 |
| dagli | 1 | 0 | — | — | 0 / 0 / 0 |
| gardeyn0 | 1 | 0 | — | — | 0 / 0 / 0 |
| gardeyn1 | 1 | 0 | — | — | 0 / 0 / 0 |
| mao | 1 | 0 | — | — | 0 / 0 / 0 |
| marques | 1 | 0 | — | — | 0 / 0 / 0 |
| shirts | 1 | 0 | — | — | 0 / 0 / 0 |
| swim | 1 | 0 | — | — | 0 / 0 / 0 |
| trousers | 1 | 0 | — | — | 0 / 0 / 0 |

This sample is descriptive: stratum means are unweighted. Published sparrow runs used 1200 s, 3 threads and AMD 7950X hardware (100 runs for the mean); shorter local runs are not hardware-normalized. Lectra software version, hardware and convergence-time semantics are not controlled. Geometry and constraint eligibility must be checked in the converter report. No raw geometry is included in these results.

Sources: Gardeyn, Vanden Berghe and Wauters, *An open-source heuristic to reboot 2D nesting research*, arXiv:2509.13329v4, Tables 2–3, https://arxiv.org/html/2509.13329v4 . Lectra data: Lallier, Vézard, Pinaud and Blin (2022), *Nesting Tasks Dataset for 2D-Nesting Efficiency Estimation*, Zenodo 7030786, CC BY 4.0, https://doi.org/10.5281/zenodo.7030786 . Conversion changes units to mm and excludes unmapped constraints.
