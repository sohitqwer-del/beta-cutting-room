# Benchmarking BETA Marker

The headline suite below is dated 20 October 2026 and pins an older engine.
It is not a rerun of the unreleased 0.8 branch. The later
[targeted defect study](NESTING-EFFICIENCY-2026-10-24.md),
[mixed engine pilot](NESTING-THREE-IMPROVEMENTS-2026-10-24.md) and
[adaptive pilot](WORKFLOW-IMPROVEMENTS-2026-10-25.md) cover selected cases with
both gains and regressions; they do not replace the public Lectra suite.

We compare fabric use on the same pieces and width. Every scored BETA layout must pass independent validation. The comparisons cover published sparrow instances and Lectra's open client-task dataset. We have no matched public runs from Tukatech, Gerber or Optitex.

## Paired 0.8 candidate versus 0.7.0: 28 October

The [full paired report](../bench/results/2026-10-28-public-0.8.md) compares
`345f42e` with `9a852d2` on the exact frozen 300-task public Lectra sample.
Both use seed 1, 60 seconds requested, the selected 80/20 search profile,
four concurrent tasks and three search workers per task. All **636 layouts**
(600 Lectra and 36 garment layouts) passed saved independent validation.

| Comparison | Valid pairs | Mean difference, pp | Median difference, pp | Win / tie / loss |
|---|---:|---:|---:|---:|
| 0.8 minus 0.7, 300 Lectra tasks | 300 | -0.0179 | -0.0004 | 47 / 187 / 66 |
| 0.8 minus Lectra | 300 | -1.3914 | -1.5709 | 77 / 12 / 211 |
| 0.8 minus 0.7, 18 garment jobs, useful yield | 18 | +3.5692 | +0.0073 | 8 / 7 / 3 |

**0.8 is slightly worse on the ordinary-open Lectra sample in this run.** Most
pairs tie; one seed cannot establish a durable regression or broad gain.
Garment gains concentrate in the three open/start-defect jobs. Clean folded
jobs tie, and Simon clean tubular, folded-with-defect and tubular-with-defect
regress. The report gives every garment pair, occupied efficiency, useful
yield, length, surplus copies and actual wall time, plus all 12 Lectra strata.
Useful yield excludes surplus copies and includes both fabric layers.

Timed Lectra stages took 4569.396 s for 0.7 and 4558.487 s for 0.8; garment
stages took 301.278 and 302.340 s. Search budgets exclude some overhead.
Both engines had effective nice 19 because the nice-10 driver also invokes
`nice -n 10`; the historical run used nice 10. Shared load, wall-clock cutoffs,
one seed and the unverified source sheet/flip assumptions remain limitations.
This is not a controlled improvement over the older historical run or a claim
of commercial parity. The matched-time and academic suites were not rerun.

The website keeps historical data and adds `lectra-60s-08`,
`lectra-08-vs-07` and `freesewing-08-vs-07`. Engine comparisons use the additive
`engine-baseline` reference type in schema version 1.

## Historical completed public suite: 20 October

**All 387 layouts passed independent validation:** 300 Lectra tasks at 60 seconds,
60 at their recorded Lectra durations, 21 academic trials and six FreeSewing
trials. The [full public report](../bench/results/2026-10-20-public.md) gives
strata, length savings, runtime, load, correlation, assumptions and exact commands.
The engine is pinned to `a2d5de2`; the public runner is pinned to `0ecfd71`.

| Lectra sample | Valid / attempted | Mean difference, pp | Median difference, pp | Win / tie / loss | Mean length saving % |
|---|---:|---:|---:|---|---:|
| Server, 60 s | 300 / 300 | -0.7296 | -1.2210 | 84 / 12 / 204 | -1.1675 |
| Server, recorded time | 60 / 60 | +0.4208 | -0.3044 | 25 / 3 / 32 | +0.6131 |

With recorded time per task, the mean is slightly ahead, **+0.42 pp**, but the
median is behind, **-0.30 pp**, and BETA loses **32 of 60** (25 wins, three ties).
This does not establish a general win over Lectra. The actual recorded budgets
are 244-608 seconds, median 304, rather than exactly five minutes. Equal seconds
do not imply equal compute. Both comparisons retain the sheet-type and flip-axis
assumptions below; Tier B includes Tier A.

The shared Ryzen 9 9900X3D server used four tasks with three search workers each,
at nice 10. The older laptop used five tasks with three workers each. Server
versus laptop gains are **confounded by hardware, engine, converter and load**;
they are not isolated hardware speedups. The completed laptop rows remain on the
website under separate `*-laptop-r2` IDs.

Academic: mean -1.5078 pp, median -1.7451 pp, 0 wins / 1 tie / 20 losses against
the published 1200-second Sparrow means. BETA uses Sparrow itself. FreeSewing
occupied efficiencies at 60 / 300 seconds: Simon 89.3036 / 89.5675%, Teagan
78.6418 / 79.0306%, Titan 76.4009 / 77.6579%. All six layouts are valid; no
commercial reference is available for those fixtures.

## Reproduce the launch suite

The [public protocol](../bench/public/README.md) freezes the same 300 Lectra task
IDs and 60 matched-time IDs, seven academic instances with three seeds, and
FreeSewing Simon, Teagan and Titan at 60 and 300 seconds. Prepare the public
inputs once, then run from `marker/` with external data/build directories:

```sh
cargo run --release --manifest-path bench/public/Cargo.toml -- --out <fresh-output-directory>
```

Use `--quick` for a 20-task, 30-second Lectra pipeline check. `--time-scale`
changes all budgets, including recorded matched times. Reduced-budget runs are
labelled as smoke runs. `--summarize` exports existing evidence without starting
searches. Incomplete exports require `--allow-partial` and stay provisional.
The tool writes the website's `beta-site-benchmark/1` JSON, with aggregates and
public task IDs only. It checks the saved independent validator report for every
scored marker. Geometry and detailed results stay outside git.

## What the Lectra numbers mean

Lectra's recorded efficiencies came from Lectra's own nesting engine on real client tasks. They are engine results, not human estimates or model predictions. The paper describes about 100 fashion customers and a nominal five-minute nesting budget. The engine version, hardware and worker count are unknown. Recorded durations vary. The matched sample ranges from 244 to 608 seconds, including two tasks at 608 seconds. The paper's nominal budget does not explain those values. Exact stopping and overhead rules remain unknown.

Source: Lallier, Vezard, Pinaud and Blin (2022), [*Nesting Tasks Dataset for 2D-Nesting Efficiency Estimation*, Zenodo 7030786](https://doi.org/10.5281/zenodo.7030786), version 1.1, **CC BY 4.0**. The associated [paper](https://doi.org/10.1007/s10845-023-02084-6) defines efficiency as the share of consumed material occupied by parts. For an ordinary rectangular strip, we use piece area / (width x occupied length). Hole accounting, edge allowances and finite-sheet denominator details remain unverified.

## Two comparison tiers

**Assumption: sheet_type 0 means an ordinary rectangular strip of sheet_width, with open single-layer fabric.** There is no verified public codebook. Code 0 covers 99,403 of 100,000 tasks. Every headline comparison is conditional on this assumption. Codes 2 and 3 are separate diagnostics and never enter the headline.

- **Tier A, exact:** BETA has exactly the task's allowed orientations and supported constraints. Exactness is conditional on the sheet-type assumption and the flip-axis interpretation below.
- **Tier B, conservative:** each part's allowed rotation and mirror set is a subset of Lectra's set. Tier B includes Tier A. A valid BETA layout is feasible for the source problem under the stated assumptions. The headline uses Tier B and states this handicap.

The version 0.3 converter chooses a regular one-, two- or four-angle subset, or continuous free rotation. It can bake a base rotation or reflection into the outline. It interprets flip_x=1 as reflection of the x coordinate before rotation. This is an explicit assumption. Optional reflection uses per-piece `flip`, with a shared permitted angular subset for both handed states. Each sampled task records the original and retained orientations per part. Round 2 fixed handedness and sometimes dropped an optional reflection; its results retain that handicap.

Tilt is symmetric around each base angle. Version 0.3 maps it separately for each piece, so a point-angle restriction on one part does not remove tilt from another. The engine samples bounded offsets; it does not search every angle in the contract's interval. Round 2 used a shared task tilt.

Only s1 rotation records with no extra populated parameters are eligible. All other constraint classes, including c*, g* and t*, remain excluded. Missing or repeated part constraints, y ranges, motif rules, groups and frozen placements are not silently removed. A positive sheet_length becomes max_length. A too_long failure counts as a loss with its reason; it is not invalid source data.

Several sub-shapes require verified topology before headline inclusion. Unverified rings are excluded. If verified holes are filled for packing, comparisons must use true net area and report the outer-outline efficiency separately. The runner supports reference_area_ratio for this correction. The round-2 audit found no referenced multi-ring tasks, so the hole stratum is empty.

## Sampling and scores

The 300-task sample uses seed 7030786 to choose tasks. All Lectra nesting trials use seed 1. It balances piece counts (1-25, 26-75, 76-150, 151+) and Lectra efficiency (below 75%, 75% to below 85%, 85%+). Sampling sorts ids, shuffles each stratum, then selects without replacement by round-robin. These unweighted means describe the sample. They are not estimates for the whole population.

The matched-time run uses a 60-task stratified subset selected before seeing results. It uses each task's recorded Lectra duration. The cap is set above every recorded duration, rather than silently truncating values above 300 seconds. Equal seconds do not imply equal hardware or compute.

Differences are percentage points: BETA minus reference. Ties are within 0.1 point. Relative length saving is 100 x (1 - reference efficiency / BETA efficiency). Positive means a shorter marker for the same true piece area. Numerical means and medians use valid layouts only. Failed layouts have blank numeric scores. too_long counts as a loss; other failures are listed separately.

Pearson correlation measures the linear relationship between BETA and Lectra efficiencies. Spearman correlation compares their ranks. Difference quantiles show whether the gap is broad or concentrated in a tail. A gross systematic gap could falsify the sheet-width assumption. Correlation alone cannot prove the missing codebook. The diagnostic also checks whether Lectra's implied occupied length violates a safe geometric lower bound.

## Academic comparison

[Reference values](../bench/academic-references.csv) come from Gardeyn, Vanden Berghe and Wauters, [*An open-source heuristic to reboot 2D nesting research*, arXiv:2509.13329v4](https://arxiv.org/html/2509.13329v4), Tables 2-3. Published means use 100 runs, 1200 seconds and three threads on an AMD Ryzen 9 7950X. Our local runs use three seeds and 60 seconds. BETA uses sparrow itself. This measures the adapter and practical budget, not an independent algorithm or hardware-equivalent speed.

The converter reads the MIT-licensed sparrow v0.3.0 instance files. It preserves coordinates and quantities and maps supported rotations exactly. Strip height becomes fabric width. There is no added gap or grain rule. Source units are retained numerically, so academic lengths are source units despite the Job mm convention. Uniform scaling leaves efficiency unchanged. Shapes with holes and unsupported reflections are refused.

## Original round-2 commands

Use the archived round-2 source and input manifest for its numbers. The current
converter follows v0.3 policy. For new runs use the v0.3 protocol below.

Run cargo from marker/. Set BETA_DATA_DIR and CARGO_TARGET_DIR to external data and build directories first. Create the converter venv on the data drive. Run Python with -I from the checkout, never from the dataset directory. No geometry belongs in git.

```powershell
$python = Join-Path $env:BETA_DATA_DIR 'venvs/marker-bench/Scripts/python.exe'
& $python -I -m pip install --no-cache-dir -r bench/tools/requirements.txt
& $python -I bench/tools/lectra_convert.py download
& $python -I bench/tools/lectra_convert.py inspect
& $python -I bench/tools/lectra_convert.py self-test
& $python -I bench/tools/lectra_convert.py convert --sample-size 300 --seed 7030786

cargo test --workspace
cargo clippy --workspace --all-targets
cargo run --release -p beta-marker-cli -- bench academic --time 60 --seeds 1,2,3 --threads-per-task 3 --jobs 5 --out "$env:BETA_DATA_DIR/marker-bench/results/academic-round2-60s"
cargo run --release -p beta-marker-cli -- bench lectra --input "$env:BETA_DATA_DIR/marker-bench/lectra/sample-300.jsonl" --time 60 --limit 300 --threads-per-task 3 --jobs 5 --out "$env:BETA_DATA_DIR/marker-bench/results/lectra-round2-60s"
cargo run --release -p beta-marker-cli -- bench lectra --input "$env:BETA_DATA_DIR/marker-bench/lectra/sample-match-60.jsonl" --time match --match-cap 10000 --limit 60 --threads-per-task 3 --jobs 5 --out "$env:BETA_DATA_DIR/marker-bench/results/lectra-round2-match"
# Diagnostics: diagnostic-code-2.jsonl and diagnostic-code-3.jsonl,
# --diagnostic --time 60 --threads-per-task 3 --jobs 5, in separate output directories.
```

Five tasks with three search workers each use the requested ceiling of 15 workers. Reduce jobs on smaller or shared machines. The CLI rejects oversubscription against the logical CPU count. Choose a fresh output directory for every run.

Each run writes run.json, results.csv and summary.md. Completed CSV rows are flushed immediately. Search time excludes import, initial construction and final repair overhead. The recorded runtime measures the whole nesting call, including its internal validation. A separate validation time is also reported. Seeds fix random streams, but wall-clock cutoffs and scheduling can change outcomes.

The converter verifies four pinned MD5 checksums before restricted unpickling. It resolves pickle globals with an opcode scan and uses an explicit reconstruction allow-list. It never calls pandas' pickle reader. See the [historical round-1 audit](../bench/lectra/README.md) for source schemas and the original stricter policy; round-2 policy and results below supersede its eligibility counts.

## Dependencies

No new Rust crates or Python packages were added for round 2. The converter uses pinned pandas 2.2.3 and numpy 2.2.6 (BSD-3-Clause), python-dateutil 2.9.0.post0 (Apache-2.0/BSD-3-Clause), pytz 2026.5 (MIT), tzdata 2026.5 (Apache-2.0) and six 1.17.0 (MIT). The dataset is CC BY 4.0. Downloaded geometry, generated jobs, full CSV files and build output stay outside git.

## Version 0.3 development results

These notes describe the earlier development handoff. The public server suite
above later completed the Lectra, academic and open-fabric FreeSewing runs.
The deferred folded/shade experiments remain outside that public suite.

Five paired development comparisons and their load/sampling audits completed. The lead
deferred shuffle, adaptive compression and faster-shrink measurements, and the
full confirmation suite, to finish the code sooner. Corrected development
selection kept the 80/20 default: none of the measured variants improved it.
All three FreeSewing jobs at 60 seconds passed. No commercial parity claim follows
from this reduced set. [Full handoff and exact quiet-machine commands](../bench/results/2026-10-19-v03-handoff.md).
The [conversion report](../bench/results/2026-10-19-v03-conversion.md)
finds 95,661 Tier A tasks, 96,747 Tier B tasks including A, and 3,253 exclusions.
The exact tier gains 1,422 tasks. No eligible task now drops an optional mirror
state; 1,086 tasks still restrict angles or tilt. The frozen 300-task sample has
296 Tier A tasks and four Tier B-only tasks. These are eligibility counts, not
measured quality gains.

The [development protocol](../bench/experiments/v03/README.md) uses 40 different
tasks outside both confirmation sets. It runs a fresh baseline beside each
variant, with the same task, seed and 60-second budget and alternating order.
All 40 tasks are attempted. A documented geometry proof excludes task 12127 only
from search selection: fixed-angle pieces exceed the stated strip width. Its
failures remain in the report. All other 39 pairs must be valid for a candidate
to qualify. The proof is checked against the saved job before selection.

| Variant | Mean paired efficiency change, pp | Decision |
|---|---:|---|
| 30% compression | -0.0338 | Keep baseline |
| 40% compression | -0.1281 | Keep baseline |
| Sequential three | -0.5514 | Keep baseline |
| Race then focus | -0.8068 | Keep baseline |
| Lighter samples | -0.0925 | Keep baseline |

Shuffle, adaptive compression and faster shrink were deferred before their trials.
Every measured variant had 39 valid eligible pairs. Two interrupted task pairs
were replayed with the original engine, without selecting by score. No persistent
load/sampling outlier remains. These repeated 39 tasks are not 195 independent
tasks. [Experiment details](../bench/results/2026-10-19-v03-experiments.md).

FreeSewing at 60 seconds: Simon 88.1897%, Teagan 76.5062%, Titan 75.3237%.
All passed independent validation. Their whole-call runtimes were 64.29, 62.27
and 62.71 seconds. CPU was shared and saturated; the historical 120-second rows
use a different budget. Folded and shade efficiency improvements are not claimed
without new measurements. [FreeSewing report](../bench/results/2026-10-19-v03-freesewing.md).

Measurements use a shared machine with unrelated background programs left
running. Two tasks run concurrently with three search workers each. Startup
checks only that this worktree's builds and benchmarks are idle. CPU samples are
saved in `conditions.json`; a fixed load-only audit chooses reruns. The first
acceptable-load retry is retained, never the highest score. Raw evidence and
replacement provenance remain available. [Validation and exact skipped tests](../bench/results/2026-10-19-v03-validation.md).

When the lead has a quiet machine, the confirmation script can run the original 300 IDs
at 60 seconds, the original 60 IDs at their full recorded duration, seven
academic instances at three seeds, and garment/layout/shade checks. It uses the
same two-task, three-worker load-recording protocol. Run the scripts from
`marker/` with an already built release executable, external data/build paths,
and no concurrent build:

```powershell
$exe = Join-Path $env:CARGO_TARGET_DIR 'release/beta-marker.exe'
$evidence = Join-Path $env:BETA_DATA_DIR 'marker-bench/quality-v03'
# Development already selected the committed baseline. Wait for a quiet host.
./bench/tools/quality_confirmation.ps1 -Executable $exe -EvidenceRoot $evidence -Jobs 2 -Workers 3 -MaxStartCpu 20
```

## Earlier confirmation: round-2 laptop

**Assumption: code 0 is an ordinary rectangular strip.** Tier B includes Tier A and keeps a subset of allowed orientations. flip_x is interpreted as x-coordinate reflection before rotation. All figures below are conditional on those assumptions.

Population: 94,239 Tier A tasks; 96,747 Tier B tasks including A; 3,253 excluded. The [round-2 policy audit](../bench/results/2026-10-18-lectra-round2-policy.md) gives overlapping and primary exclusion counts. The hole stratum is empty.

The [60-second report](../bench/results/2026-10-18-lectra-60s.md) uses 300 tasks, with 25 in each stratum. It includes 293 Tier A tasks and seven with an orientation handicap. All 300 layouts are valid. All 25 finite-length tasks respect max_length; there are no too_long failures.

| Group | Attempts / valid | Mean difference, pp | Median difference, pp | Win / tie / loss | Mean length saving % | Median length saving % | Other failures |
|---|---:|---:|---:|---|---:|---:|---:|
| Tier B at 60 s, includes A | 300 / 300 | -2.3556 | -2.9653 | 68 / 10 / 222 | -3.6211 | -3.6829 | 0 |
| Tier A at 60 s | 293 / 293 | -2.3328 | -2.9024 | 68 / 10 / 215 | -3.6110 | -3.6557 | 0 |
| Tier B at matched time, includes A | 60 / 60 | -1.0865 | -1.1278 | 20 / 2 / 38 | -1.3668 | -1.6003 | 0 |
| Tier A at matched time | 59 / 59 | -1.0788 | -1.0672 | 20 / 2 / 37 | -1.3576 | -1.5492 | 0 |

Negative length saving means BETA uses a longer marker. The reported mean is the arithmetic mean of per-task savings.

The [matched-duration report](../bench/results/2026-10-18-lectra-match.md) uses the recorded Lectra duration without truncating it to 300 seconds. The same seed and three workers are used on a preselected stratified 60-task subset: 59 Tier A tasks and one handicapped task.

Recorded budgets range from 244 to 608 seconds; the median is 304. Two tasks record 608 seconds. Those values are kept. Their cause is unknown.

| Group | Attempts / valid | Mean difference, pp | Median difference, pp | Win / tie / loss | Mean length saving % | Median length saving % | Other failures |
|---|---:|---:|---:|---|---:|---:|---:|
| Same subset at 60 s | 60 / 60 | -2.5276 | -2.9054 | 18 / 1 / 41 | -3.5077 | -3.6243 | 0 |
| Same subset at matched duration | 60 / 60 | -1.0865 | -1.1278 | 20 / 2 / 38 | -1.3668 | -1.6003 | 0 |

Across 60 valid pairs, extra time changes efficiency by **1.4411 points on average** and 0.7624 at the median. Separate stochastic runs can regress as well as improve.

### By stratum

Each stratum has 25 tasks at 60 seconds.

| Group | Attempts / valid | Mean difference, pp | Median difference, pp | Win / tie / loss | Mean length saving % | Median length saving % | Other failures |
|---|---:|---:|---:|---|---:|---:|---:|
| pieces:1-25;lectra:<75 | 25 / 25 | 4.5500 | 1.3748 | 21 / 2 / 2 | 6.3733 | 2.0585 | 0 |
| pieces:1-25;lectra:75-85 | 25 / 25 | -0.3028 | -0.1289 | 10 / 1 / 14 | -0.4449 | -0.1530 | 0 |
| pieces:1-25;lectra:85+ | 25 / 25 | -0.0037 | -0.0594 | 11 / 2 / 12 | -0.0819 | -0.0688 | 0 |
| pieces:26-75;lectra:<75 | 25 / 25 | 0.0134 | -2.9510 | 5 / 0 / 20 | -1.2788 | -4.5780 | 0 |
| pieces:26-75;lectra:75-85 | 25 / 25 | -2.1274 | -2.0886 | 4 / 2 / 19 | -2.7822 | -2.6149 | 0 |
| pieces:26-75;lectra:85+ | 25 / 25 | -2.0264 | -1.7922 | 3 / 1 / 21 | -2.4553 | -2.1045 | 0 |
| pieces:76-150;lectra:<75 | 25 / 25 | -5.7194 | -3.7506 | 2 / 0 / 23 | -11.0919 | -5.7988 | 0 |
| pieces:76-150;lectra:75-85 | 25 / 25 | -3.6653 | -3.9784 | 5 / 0 / 20 | -5.1659 | -4.9527 | 0 |
| pieces:76-150;lectra:85+ | 25 / 25 | -3.3574 | -3.5227 | 1 / 1 / 23 | -4.0729 | -4.0488 | 0 |
| pieces:151+;lectra:<75 | 25 / 25 | -3.1013 | -4.8256 | 5 / 1 / 19 | -6.1018 | -7.4405 | 0 |
| pieces:151+;lectra:75-85 | 25 / 25 | -6.8475 | -6.8165 | 0 / 0 / 25 | -9.3122 | -9.3965 | 0 |
| pieces:151+;lectra:85+ | 25 / 25 | -5.6796 | -5.1593 | 1 / 0 / 24 | -7.0384 | -6.2656 | 0 |

The matched subset has five tasks in each stratum.

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

### Sheet-type evidence

Code 0, 60 seconds: Valid pairs: 300. Pearson **0.8517**; Spearman **0.8559** (average ranks for ties).

Code 0, matched duration: Valid pairs: 60. Pearson **0.9592**; Spearman **0.9285** (average ranks for ties).

At 60 seconds, differences range from -28.8761 to +37.1418 points. The 5th, 50th and 95th percentiles are -9.7790, -2.9653 and 5.6572. The mean difference is -2.3556 points, with a positive relationship and wide tails. The sheet-type codebook remains unverified.

| Group | Attempts / valid | Mean difference, pp | Median difference, pp | Win / tie / loss | Mean length saving % | Median length saving % | Other failures |
|---|---:|---:|---:|---|---:|---:|---:|
| Code 2 diagnostic | 20 / 20 | -1.7249 | -3.0304 | 6 / 2 / 12 | -3.7469 | -3.4046 | 0 |
| Code 3 diagnostic | 19 / 15 | 8.7419 | -3.8454 | 5 / 0 / 10 | 11.0163 | -4.4790 | 4 |

The analytic width check uses the full Lectra orientation set. Four of the 19 code-3 tasks contain pieces whose minimum required width is 1.243–1.692 times the stated width. Those same four tasks fail initial construction. Different sheet semantics, such as folded or tubular fabric, are plausible. Their actual meaning remains unknown. No code-2 width contradiction was found. Codes 2 and 3 stay outside the headline.

Code 0 has no width-fit contradiction. Task 63488 has a small reference anomaly: its efficiency is 0.01249 point above an analytic projection bound. Coordinate precision or denominator details may explain it. That explanation is unverified.

### Runtime and academic rerun

| Run | Search budget, s | Wall time, s | Summed nesting time, s |
|---|---:|---:|---:|
| Lectra 60 s | 60 | 3599.976 | 17825.488 |
| Lectra matched | 244–608 | 3743.858 | 18254.768 |
| Code 2 diagnostic | 60 | 245.086 | 1107.717 |
| Code 3 diagnostic | 60 | 186.682 | 925.151 |
| Academic | 60 | 304.512 | 1287.257 |

Runs use five simultaneous tasks and three workers per task on a 16-logical-CPU Intel laptop. Import, construction, repair and validation add time outside the search budget. Workspace checks overlapped early 60-second trials. Runner wall time excludes compilation, input parsing, setup and final summary writing. Background CPU load was not controlled. Hardware is shared and not equivalent to Lectra’s unknown machine.

The [round-2 academic rerun](../bench/results/2026-10-18-academic-round2-60s.md) has **21 of 21 valid trials**. Shirts seed 1 now passes. The mean difference from the published means is -2.3730 points; the median is -2.3398. Win/tie/loss is 0/0/21. The round-1 run had 20 valid trials; it remains available as historical evidence.

| Group | Attempts / valid | Mean difference, pp | Median difference, pp | Win / tie / loss | Mean length saving % | Median length saving % | Other failures |
|---|---:|---:|---:|---|---:|---:|---:|
| albano | 3 / 3 | -1.5552 | -1.8648 | 0 / 0 / 3 | -1.7730 | -2.1287 | 0 |
| dagli | 3 / 3 | -3.8187 | -4.3730 | 0 / 0 / 3 | -4.4805 | -5.1516 | 0 |
| mao | 3 / 3 | -2.3822 | -2.3875 | 0 / 0 / 3 | -2.8443 | -2.8507 | 0 |
| marques | 3 / 3 | -1.3011 | -1.8033 | 0 / 0 / 3 | -1.4595 | -2.0233 | 0 |
| shirts | 3 / 3 | -2.4101 | -2.3621 | 0 / 0 / 3 | -2.7624 | -2.7058 | 0 |
| swim | 3 / 3 | -3.5178 | -3.6224 | 0 / 0 / 3 | -4.7075 | -4.8534 | 0 |
| trousers | 3 / 3 | -1.6258 | -1.5825 | 0 / 0 / 3 | -1.8045 | -1.7555 | 0 |

Workspace verification: 70 tests passed, two intentionally ignored; Clippy passed without warnings. Converter safety and orientation-inclusion checks passed. The [evidence manifest](../bench/results/2026-10-18-round2-manifest.md) identifies the input and tooling files. Only summaries and this documentation are committed; tooling edits remain in the worktree, as requested.

Dataset attribution: Lallier, Vézard, Pinaud and Blin (2022), *Nesting Tasks Dataset for 2D-Nesting Efficiency Estimation*, [Zenodo 7030786](https://doi.org/10.5281/zenodo.7030786), version 1.1, CC BY 4.0. Converted coordinates are in mm; orientation sets may be restricted. No raw geometry is included.
