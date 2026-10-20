# Academic round-2 rerun — 18 October 2026

**21 of 21 trials passed independent validation.** Seven instances, seeds 1/2/3, 60-second search budget, round-2 collision guard and repair. The round-1 shirts seed-1 overlap is checked again here.

| Group | Attempts / valid | Mean difference, pp | Median difference, pp | Win / tie / loss | Mean length saving % | Median length saving % | Other failures |
|---|---:|---:|---:|---|---:|---:|---:|
| All | 21 / 21 | -2.3730 | -2.3398 | 0 / 0 / 21 | -2.8331 | -2.7058 | 0 |
| albano | 3 / 3 | -1.5552 | -1.8648 | 0 / 0 / 3 | -1.7730 | -2.1287 | 0 |
| dagli | 3 / 3 | -3.8187 | -4.3730 | 0 / 0 / 3 | -4.4805 | -5.1516 | 0 |
| mao | 3 / 3 | -2.3822 | -2.3875 | 0 / 0 / 3 | -2.8443 | -2.8507 | 0 |
| marques | 3 / 3 | -1.3011 | -1.8033 | 0 / 0 / 3 | -1.4595 | -2.0233 | 0 |
| shirts | 3 / 3 | -2.4101 | -2.3621 | 0 / 0 / 3 | -2.7624 | -2.7058 | 0 |
| swim | 3 / 3 | -3.5178 | -3.6224 | 0 / 0 / 3 | -4.7075 | -4.8534 | 0 |
| trousers | 3 / 3 | -1.6258 | -1.5825 | 0 / 0 / 3 | -1.8045 | -1.7555 | 0 |

Run window: 2026-10-18 21:51:48 to 2026-10-18 21:56:52, Asia/Kolkata. Wall time **304.512 s**. Summed nesting time **1287.257 s**; median 61.070 s; maximum 63.375 s. Independent validation adds 0.026 s. Search budgets range from 60 to 60 s. 5 simultaneous tasks × 3 workers. Intel64 Family 6 Model 141 Stepping 1, GenuineIntel; 16 logical CPUs. Engine revision `3022b1285ff7`, dirty worktree `True`. Import, initial construction, final repair and validation add time outside the search budget. Runner wall time excludes compilation, input-file parsing, setup and final summary writing.

All attempted layouts passed independent validation.

References: Gardeyn, Vanden Berghe and Wauters, [*An open-source heuristic to reboot 2D nesting research*, arXiv:2509.13329v4](https://arxiv.org/html/2509.13329v4), Tables 2–3. Published means use 100 runs, 1200 seconds, three threads and an AMD 7950X. BETA uses sparrow itself. This measures the adapter and budget, not a separate algorithm or a hardware-normalized comparison. Geometry is not included.
