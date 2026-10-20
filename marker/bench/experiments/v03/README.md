# Version 0.3 paired development protocol

The 40 development IDs are frozen before outcomes are inspected. They are
stratified tasks outside both confirmation ID lists, selected with seed 7030789.
The round-2 300 IDs and matched-duration 60 IDs stay unchanged. Conversion records
the split and every original/retained orientation set outside git.

The original plan had eight variants. The lead reduced it after five completed
comparisons, before shuffle, adaptive and faster-shrink trials started. The
active and deferred lists are frozen in `study-plan.json`. The five measured
variants each get a fresh baseline: 200 pairs, 400 trials, plus load-only replays.
Each trial uses 60 seconds, seed 1 and three separator workers. Two task pairs
run concurrently. Within a pair, baseline and variant run next to each other.
Even original task indices run baseline first; odd indices run variant first.
The same task, seed and time budget occur in both arms. This is 40 unique tasks,
not 400 independent tasks. The source order is retained for load reruns.

The user explicitly authorized shared-machine measurements with unrelated
background programs left running. Startup waits only for our own marker-quality
builds/benchmarks to be idle. It does not wait for low total CPU. The wrapper
records three background samples and CPU samples every five seconds in
`conditions.json`, and sets `RAYON_NUM_THREADS=3`. Unrelated programs are never
stopped. Two tasks with three separator workers each keep our active search near
six workers plus coordinators. Raw layouts, jobs, validation and epoch timestamps
are saved for every trial.

The Rust load audit flags a trial whose mean CPU exceeds its original run's
median by `max(15 percentage points, 3 * median absolute deviation)`. The latter
is the typical absolute distance from the median. A flagged development task
repeats both arms, in the same original order. Cutoffs stay fixed to the original
run. Up to three load reruns are made. The first rerun below the cutoff is used;
if none qualifies, the last is retained and clearly marked as a persistent load
outlier. Scores never select reruns or replacements. Raw results are preserved;
`effective/selection.json` records every chosen source. Confirmation uses the
same load-only audit and replays only flagged tasks, with their saved settings.

During the second comparison, sampling stopped for about eight minutes and two
60-second trials reported more than 520 seconds. Before computing that study's
paired result, the audit was extended to flag any trial with a telemetry gap
over 30 seconds. This also repeats both arms, without looking at their scores.
After the reduced matrix, all five studies are re-audited with the tested new audit;
replays use the original executable, whose SHA-256 is
`B07AC596E626CAC9D4DB09E301C4E61636BA85C827FBDF33B4FE7B4E24AD09DB`.
The frozen executable contains source commit `ace25d5`. Later documentation and
audit changes do not change that development engine. Sampling-gap reruns use
retry directories 11-13, preserving any original CPU-only retries 01-03. Final
confirmation applies both the CPU and sampling-gap rules from the start.

The revised scope runs only Simon, Teagan and Titan at 60 seconds after these
checks. The 300-task, matched-time, academic and longer feature confirmations
are deferred to the lead on a quiet machine. Their absence is not evidence of
commercial parity. Unmeasured search controls remain options, not recommended
defaults.

Report every variant, including failures. Rank by arithmetic mean paired
variant-minus-baseline efficiency, not raw means across different periods.
A candidate needs valid pairs on every geometrically eligible task. Keep the
baseline unless paired gain exceeds 0.1 point. If a candidate fails on an
eligible task, report it and compare the remaining candidates.
Do not tune on either confirmation set.

The original gate required 40 valid pairs. Before computing aggregate scores,
the first comparison exposed a geometry failure in both arms: task 12127 has
fixed-angle pieces spanning 1286 mm and 1244 mm across a 1243 mm strip. Source
records permit only zero degrees and no tilt or reflection. The frozen proof in
`eligibility.json` excludes this task only from search selection. All 40 original
tasks are still attempted and reported; all remaining 39 pairs must be valid.
No search-dependent failure is excluded and no development ID is replaced.

Configurations are hypotheses, not claimed improvements. Compression shares of
30% and 40% test more refinement at short budgets. Sequential restarts keep all
separator workers. Race-then-focus gives 30% to fresh probes before refining the
best. Adaptive compression raises its share to 40% for at least 150 demanded
copies. The lighter sampler trades breadth per move for more moves. The faster
shrink/pool variant tests larger disruptions. Shuffle changes enumeration and
tie order; Sparrow still controls its construction algorithm.

These experiments follow the exploration/compression distinction in Gardeyn,
Vanden Berghe and Wauters, [arXiv:2509.13329v4](https://arxiv.org/html/2509.13329v4),
sections 7 and 9 and table 1. The paper's 80/20 split was tuned for 20-minute
academic runs. It does not establish the best split for 60-second garment jobs.
Only public Sparrow parameters are changed; engine source is not patched.

`SearchTuning` is separate from `NestOptions`. To run a pair, use
`bench lectra --search-config <variant.json> --paired-baseline <baseline.json>`.
Use `--runs 3` for sequential3 and race3; all others use one. `bench audit`,
`bench replay` and `bench finalize` apply the load-only policy. Fixed geometry,
collision coverage, copy rules and independent validation remain required.
