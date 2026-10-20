# Version 0.3 development experiments

Five paired development comparisons and their sampling-gap re-audits completed.
Every measured variant had a negative observed mean paired change. Keep the existing 80/20
default. The lead deferred three variants and all confirmation runs. No full
confirmation result is available.

The protocol and eight frozen parameter sets are in
[`bench/experiments/v03/README.md`](../experiments/v03/README.md). Every comparison
uses the same 40 development IDs, seed 1, and 60 seconds per arm. Two tasks run
at once with three separator workers each. Baseline and variant run next to each
other, with alternating order. These IDs are disjoint from both confirmation
sets. Selection uses paired differences, never confirmation scores.

The user authorized measurements on a shared machine. Unrelated programs remain
running. Each run records CPU samples in `conditions.json`. High-load reruns use
the fixed, score-independent rule in the protocol; raw evidence is retained.

## Construction failures

Task `12127` failed in both arms of the first comparison before search. The
converted fabric width is 1243 mm. Piece `10` spans 1286 mm across the width and
piece `13` spans 1244 mm. Both have a fixed zero-degree orientation, zero tilt,
and no permitted reflection. The saved source orientation records contain only
zero degrees (with the equivalent 360-degree endpoint). A translation cannot
make either piece fit the converted strip. This is an input feasibility issue
to investigate against the public source, not evidence for either search arm.
Both failed attempts remain in the table and evidence. No input was widened,
rotated, removed, or replaced after inspecting its outcome.

Every returned layout is independently validated. A failure to return a layout
is recorded separately from a valid quality comparison. The initial gate required
all 40 pairs to be valid. Before computing aggregate scores, the geometry proof
above was frozen in [`eligibility.json`](../experiments/v03/eligibility.json).
Selection now requires valid pairs on all other 39 tasks. No search-dependent
failure is excluded, and the original 40 IDs are still attempted. If no variant
passes that gate, the baseline remains the default. This amendment is independent
of quality scores.

## Evidence

Raw layouts, jobs, validation reports, settings, paired CSVs, load samples and
score-independent rerun provenance are under
`$BETA_DATA_DIR/marker-bench/quality-v03/paired-*`. Interrupted earlier attempts
and short tooling smoke trials are retained separately and excluded.

## Results

Gains are arithmetic mean variant-minus-baseline efficiency in percentage points
on valid eligible pairs. Every arm still attempts all 40 original IDs.

| Experiment | Valid eligible pairs | Paired gain, pp | Load-selected rerun tasks | Decision |
|---|---:|---:|---:|---|
| Compression 30% | 39 / 39 | -0.0338 | 0 | Reject; no improvement |
| Compression 40% | 39 / 39 | -0.1281 | 2 | Reject; worse |
| Sequential three | 39 / 39 | -0.5514 | 0 | Reject; worse |
| Race then focus | 39 / 39 | -0.8068 | 0 | Reject; worse |
| Lighter samples | 39 / 39 | -0.0925 | 0 | Reject; no improvement |
| Shuffled order | Not run | Not measured | — | Deferred by lead |
| Adaptive compression | Not run | Not measured | — | Deferred by lead |
| Faster shrink / wider pool | Not run | Not measured | — | Deferred by lead |

The first study's median trial-window CPU load was 99.89%, and no trial exceeded
its fixed high-load cutoff. Total CPU at this level is saturated: it cannot
distinguish additional competition above 100%. Adjacent paired trials reduce
this uncertainty but do not establish equal CPU time. These are shared-machine
wall-time measurements, not quiet-machine or hardware-equivalent comparisons.
Both construction failures for task 12127 remain in raw and effective evidence.

## Sampling interruption

In the 40% comparison, samples jump from 10:35:21 to 10:43:39 UTC. Baseline
task 92929 reports 524.361 seconds; variant task 81475 reports 521.540 seconds.
The cause is unknown. These are not accepted as ordinary 60-second trials.
Both complete pairs were replayed with the frozen development executable.
The four replayed layouts passed validation and the load/sampling audit. Every
study was checked for gaps over 30 seconds before selection. No persistent
outlier remains. Original interrupted trials remain available. No quality score
selected a replay or its replacement.

## Earlier excluded baseline

An earlier unpaired `development-baseline` folder has 40 rows, 39 valid, and mean
BETA-minus-Lectra -3.2468 points among valid layouts. It used three tasks in
parallel, a different executable hash and the earlier quiet-start wrapper, and
has no completion.json or load audit. It remains in raw evidence but is excluded
from paired selection and confirmation. Short tooling trials and interrupted
baseline attempts are also excluded, as recorded in the validation report.
