# Launch engine fixes and rule-freedom study

The fixed-piece search now checks its deadline inside candidate, obstacle and
contact-search loops. It builds a complete conservative layout first, then
compacts one movable piece at a time. Every accepted full candidate passes the
adapter checks and the independent core validator. Locked transforms and copy
identities remain unchanged. A cancelled or expired search retains its best
checked complete marker.

Sparrow 0.3.0 and unmodified Jagua 1.0.0 remain in use. There is no engine upgrade,
new dependency, model change, or change to ordinary-job search defaults.

## API and implementation

`nest` and `nest_with_progress` automatically use the bounded packer for fixed
pieces, defects and fold-edge jobs. The new `nest_with_cancellation` entry point
accepts an `AtomicBool` and a progress callback. Set the flag from another thread
or the callback. This explicit cancellable entry point uses the bounded packer
even for an ordinary job. It does not promise cancellation inside Sparrow's
unmodified cold-start constructor.

Preparation and independent validation still add overhead. The quick constructor
uses bounding boxes and switches to a simple trailing shelf if its allowance
expires. It never returns a partial layout. A strict maximum length can prevent
this conservative constructor from finding a marker; an error is not proof that
the job is impossible. Supplying a valid warm start avoids losing a known fit.

The search caches transformed outlines, bounding boxes and polygons. The first
construction puts fold halves first and compares each allowed half with a full
two-layer cut. Bounded exact compaction follows. Full cuts can create surplus;
useful-demand yield excludes that surplus. Randomized fresh constructions retain
the checked incumbent. On folded fabric, later passes vary which half/full
orientation receives the first search slice and can move a full piece away from
the single fold edge without lengthening the marker.

The matching adapter now retains its already checked snapped layout as the warm
incumbent during fixed-piece refinement. Ordinary engine defaults are unchanged.

Set `BETA_NEST_PROFILE=1` to print phase timings without pattern coordinates.
The result records preparation, initial construction and search/check times for
the bounded path. Progress callbacks report complete independently valid markers.

## QA-006 reproduction

The exact public QA input and saved 114-placement Simon result were copied into
the evidence directory. Input: S1/M2/L2/XL1, 1500 mm, one-way nap, 3 mm piece gap,
5 mm edge gap. Only the first placement is locked. Seed 42, two workers, one run.

The instrumented baseline at `9461f54` exceeded the 300-second cutoff. Individual
placement phases grew to more than 20 seconds, still inside the first complete
construction. The supervisor terminated it at 301.53 seconds. No baseline result
was scored. The earlier QA report's independent cutoff was 300.045 seconds.

Final build, same input and fixed placement:

| Search budget s | Wall s | Length m | Efficiency % | Valid; lock exact |
|---:|---:|---:|---:|---|
| 1 | 1.027 | 8.818486 | 74.03 | Yes |
| 5 | 5.046 | 8.818486 | 74.03 | Yes |
| 15 | 15.074 | 8.818486 | 74.03 | Yes |
| 60 | 60.058 | 8.814411 | 74.07 | Yes |

All four tested budgets met budget plus ten seconds. This is evidence for these
inputs and budgets, not a hard real-time guarantee for arbitrary geometry or a
host suspension. The short regression uses 114 detailed discs and a 0.02-second
budget. Cancellation tests cover cancellation from progress, from another thread
during search, and an already-cancelled call with a valid warm incumbent. The
public Simon budget matrix is an ignored integration test.

The final release CLI was then tested as a separate process, including startup,
JSON reads/writes and shutdown. The one-second and sixty-second calls returned
in **3.337 s** and **60.846 s**, respectively. With a full warm start, the one-second
call returned in **2.379 s**, retaining **8.200657 m / 79.6117%**. All exited zero;
the separate CLI validator also exited zero with no errors or warnings. The
locked `SIMON-BACK-L` copy 0 retained its exact hand, angle, translation, bundle
and lock flag. Both fixed-only calls returned all 114 placements at 8.818486 m.

The existing CLI reads only locked placements from `--fixed`. It does not pass
the other placements as an incumbent. Supply both `--fixed` and `--warm-start`
to preserve the quality of an existing full marker. Automatically using the
fixed-result file as a warm start is a possible CLI/editor follow-up, outside
this change's owned files.

## Folded Teagan

The historical 40.90% folded, 69.83% tubular and 78.56% open figures have different
layouts and are not interchangeable targets. The current baseline already contains
fold-first and half/full-choice code from v0.3; this change does not claim those
earlier additions as new work.

Two frozen executables ran back-to-back on each seed, at nominal 60 seconds,
750 mm folded width, three requested workers and one run. The adapter uses one
thread for constrained packing. Seed 1 ran baseline first; seed 42 ran the new
engine first. All four markers passed independent validation. Both arms had the
same 120-second supervisor cap. The old constructor exceeded its nominal budget;
the wall times below are included rather than hidden or treated as equal CPU work.

| Seed | Before useful % | After useful % | Gain pp | Before m | After m | Saved m | Before / after wall s |
|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | 42.6223 | 42.6815 | +0.0592 | 8.420873 | 8.409189 | +0.011685 | 106.331 / 60.055 |
| 42 | 42.6223 | 42.8212 | +0.1989 | 8.420873 | 8.381756 | +0.039117 | 77.439 / 60.055 |

Mean paired gain: **+0.1291 percentage point and 0.025401 m per marker**. This is
a small gain on two seeds, not a broad efficiency breakthrough. Using the
0.1-point tie threshold gives one win and one tie. Historical all-half yield was
40.90%; the new two-seed mean is 42.75%, but most of that historical difference
already exists in the v0.3 baseline. Only the paired 0.1291 point is attributable
to this comparison.

Occupied efficiency was 59.8713% before and 60.3853% / 59.8125% after. Those values
include surplus cut area and must not be presented as useful yield. The folded
quality gap remains. Earlier exploratory measurements and the interrupted
seed-42 baseline are retained outside the headline comparison.

A full two-layer placement still assigns both layers to one bundle. It cannot
share two identical torso cuts between different garment bundles. That contract
limits useful yield and can create surplus. No demand or validation rule was
relaxed to improve the score.

One additional diagnostic changed only the per-piece flip permission of Teagan's
declared fold pieces. The same final executable ran both arms at 60 seconds,
seed 1, control first. Useful yield changed from 42.6815% to 42.9046%, and length
from 8.409189 m to 8.365471 m: **+0.2231 point / 0.043718 m**. Both markers were
valid for their respective inputs. This is not a gain under the original rules
and is excluded from the main folded comparison.

Core follow-up: `layer_copies` counts an on-fold cut as one normal piece, but
the generic mirror gate in `beta-marker-core/src/validate.rs` rejects the opposite-half
representation when mirror demand and flip permission are absent. Review whether
a declared symmetric on-fold cut may select either source half without granting
optional face-down cutting to a full piece. If that policy is approved, update
the core gate and the adapter's half-only hand choices together, keeping full-cut
permissions intact. No such exception is introduced here.

## Rule-freedom protocol

The Rust `launch_study` example runs adjacent paired trials in isolated child
processes. It saves each input, options, result, progress, child validation,
parent validation and wall time. Each contrast has a fresh control; arm order
alternates. Every child has a 120-second wall cap for the nominal 60-second
search. Timeouts and invalid results remain failures with no numeric score.

Selection: the first 20 rows of the existing frozen v0.3 development file,
plus FreeSewing Simon, Teagan and Titan S1/M2/L2/XL1. No confirmation tasks are
used. Task 12127's known infeasible fixed-angle baseline remains in the denominator.
Seed 1; two workers per trial; two concurrent pairs. The sample describes these
tasks, not the full Lectra population.

- Tilt: exact grain versus symmetric ±1 degree and versus ±2 degrees. Both
  fabric and per-piece tilt fields are set explicitly. Other rules are retained.
  The native sampler uses zero, ±0.5 degrees and the range endpoints. It does not
  exhaustively search the interval; the ±2-degree run does not retain every
  ±1-degree sample. Pieces already allowed continuous rotation gain no freedom
  from changing the tilt field. The input-only audit records those cases.
  In this selected 20-task subset, every demanded piece type has restricted
  rotations; there are no continuous-rotation types to make the treatment inert.
- Faceless: `allow_flip` off versus on, with all per-piece overrides set to the
  corresponding value. Both sides use five sequential allocation searches
  sharing the same 60-second allowance. This tests mirror choices without
  changing production defaults. It is not the single-run default's score.
- Tubular: open spread versus a two-layer tube at half the open width. Teagan's
  symmetric front/back receive the same declared fold lines as the existing
  feature benchmark. Other pieces keep their outlines and demand. Fabric type
  is unknown for the Lectra rows. Simon is a shirt and Titan a trouser block;
  neither is claimed to be a knit. Teagan is the T-shirt scenario. These are
  hypothetical material scenarios, not blanket recommendations or comparisons
  with Lectra's achieved efficiencies. The mixed tubular mean is not an estimate
  for tubular knits; many converted jobs create unused mirrored layer copies.

Positive gain means higher useful-demand yield in percentage points. Positive
saved metres means a shorter marker for the same requested products and total
fabric width. Occupied efficiency and surplus area are also saved. Mean gains
use only valid pairs; failures are listed separately and one-sided failures
count as losses or wins. A tie is within 0.1 percentage point.
The contrasts change one rule family at a time; their gains must not be added.

Tilt and optional flipping enlarge the permitted set, so they cannot worsen
the true optimum. A negative paired result measures this finite-budget search's
performance, not a physical penalty for allowing the freedom. A previous checked
control can still be retained as a warm incumbent. Tubular spreading is different:
it adds fold/layer constraints and can create surplus, so it is not simply a
larger feasible set.

The host is a shared Intel i9-11900H, eight cores and sixteen logical processors.
Other builds and searches overlap the study. CPU and compiler-process samples
are retained. Equal wall budgets do not imply equal CPU work. One seed cannot
establish a reliable expected factory saving. Negative results are retained.

An initial study prefix was interrupted by the PowerShell harness treating a
progress line on stderr as an error. That prefix is preserved separately and
excluded wholesale. The complete study uses direct child-process redirection
in a fresh output directory; no pair is selected by its score.

The host then had an unexplained timing gap from 19 October 17:05:57 UTC to
20 October 03:14:27 UTC in the load samples. Titan's two tilt pairs were in
flight, with one arm in each exceeding 36,000 seconds despite the supervisor's
120-second allowance. Both complete pairs are replayed with the original frozen
executable and arm order. Their raw interrupted records remain; the replacement
manifest records the superseding pairs. A folded seed-42 baseline spanning the
same gap is also excluded and repeated. No machine-sleep cause is assumed.
This interruption prevents a claim that the work finished within three elapsed
wall-clock hours.

## Rule-freedom results

All **92 planned pairs completed**: four contrasts on three FreeSewing jobs and
20 Lectra development tasks. After replacing the two interrupted Titan pairs,
the selected analysis contains **184 trial calls: 172 valid markers and 12
failed calls**. There are **84 valid pairs out of 92**. Every accepted marker
passed child and parent validation. Both replay input jobs and options match
the original files byte for byte; the original solver executable was reused.
No supervised timeout remains in the selected analysis. Completion and replays
finished within the lead's additional 60-minute cap.

### FreeSewing, all 12 pairs valid

Each row has its own fresh control. Flip uses the five-search schedule described
above. Positive saved metres means less fabric length; negative means more.

| Job | Contrast | Control useful % | Variant useful % | Gain pp | Saved m |
|---|---|---:|---:|---:|---:|
| Simon | 0 to ±1° | 87.6415 | 86.3015 | -1.3400 | -0.115669 |
| Simon | 0 to ±2° | 87.4415 | 87.2940 | -0.1475 | -0.012614 |
| Simon | Flip off to on | 86.7399 | 84.6152 | -2.1247 | -0.189000 |
| Simon | Open to tubular, hypothetical | 87.7236 | 51.1119 | -36.6117 | -5.330991 |
| Teagan | 0 to ±1° | 77.9862 | 77.4245 | -0.5616 | -0.033385 |
| Teagan | 0 to ±2° | 77.5131 | 77.3808 | -0.1323 | -0.007920 |
| Teagan | Flip off to on | 77.1843 | 75.5262 | -1.6582 | -0.102094 |
| Teagan | Open to tubular | 77.4297 | 66.3481 | -11.0816 | -0.774215 |
| Titan block | 0 to ±1° | 76.5239 | 76.7168 | +0.1928 | +0.017967 |
| Titan block | 0 to ±2° | 75.4573 | 77.2858 | +1.8285 | +0.171503 |
| Titan block | Flip off to on | 75.6820 | 74.6796 | -1.0024 | -0.097013 |
| Titan block | Open to tubular, hypothetical | 75.3302 | 73.9235 | -1.4067 | -0.138174 |

In these runs, Titan improved with tilt; Simon and Teagan did not. The faceless
allocation policy lost length on all three. These are single-seed observations,
not a prediction for other styles. Titan is a trouser block without the waistband,
fly or pockets of a finished trouser. Teagan is the relevant T-shirt tube scenario;
the other two tubular rows are not claims about knit production.

### Lectra development subset, all 20 tasks attempted

| Contrast | Valid pairs / attempted | Mean gain pp | Mean saved m | Win / tie / loss | Failed control / variant |
|---|---:|---:|---:|---|---|
| 0 to ±1° | 19 / 20 | -0.2588 | -0.036171 | 7 / 4 / 8 | 1 / 1 |
| 0 to ±2° | 19 / 20 | +0.0410 | -0.024271 | 11 / 2 / 6 | 1 / 1 |
| Flip off to on, five searches | 19 / 20 | -1.1164 | -0.080261 | 4 / 5 / 10 | 1 / 1 |
| Open to tubular, hypothetical | 15 / 20 | -39.4638 | -4.482197 | 0 / 0 / 19 | 1 / 5 |

Task 12127 failed on both sides of every contrast. The tubular variants also
failed on tasks 8411, 18896, 77136 and 58904; their valid controls remain recorded.
These failures are not given numeric efficiency scores. The four one-sided
tubular failures count as losses, explaining why its loss count exceeds its
15 numeric pairs. A both-failed pair is recorded separately, not treated as a tie.

The ±2° mean point gain is near zero, while mean metres saved is negative.
The averages can have different signs because a point of efficiency on a long
marker represents more metres than a point on a short marker. Both measures are
computed per pair before averaging. This study does not demonstrate a general
fabric saving from either tilt setting or the tested faceless allocation policy.
The large hypothetical tubular penalties include unused mirrored layer copies;
they must not be generalized to correctly specified knit jobs.

Mean wall seconds, control / variant, across all attempted Lectra calls were
57.83 / 57.75 for ±1°, 57.49 / 57.79 for ±2°, 58.69 / 70.60 for flip, and
57.89 / 52.32 for tubular. Fast failures lower these means; they are not
valid-marker latency guarantees. Per-trial wall times and all errors remain in
`pairs.csv`, `selected-pairs.json` and `failures.json` under the evidence root.
The replacement manifest identifies the two whole-pair replays. Defaults remain
unchanged; a factory should use only permissions its material and quality rules allow.

## Verification

The final release test run for nest, match and CLI passed **111 tests**, with
zero failures and three ignored tests. The ignored Simon budget-matrix test was
then run explicitly and passed all four budgets. Strict Clippy for all targets
of those three packages passed with warnings denied. No desktop, TypeScript or
full-workspace pass is claimed for this crate-only change.

The new regressions check deadline-limited completion with 114 detailed polygons,
exact fixed transforms, cancellation during search, cancellation from progress,
and retention of an already valid warm incumbent. Existing folded, layered,
defect, gap, per-piece permission, matching and shade tests also pass.

## Reproduction and evidence

Set `CARGO_TARGET_DIR`, `CARGO_BUILD_JOBS=3` and `BETA_DATA_DIR` as assigned. From
`marker/`, use the following commands. `launch_study` denotes the executable in
`$CARGO_TARGET_DIR/release/examples/` (with `.exe` on Windows); `beta-marker` is
in `$CARGO_TARGET_DIR/release/`.

```text
cargo build --release -p beta-marker-cli --bin beta-marker -p beta-marker-nest --example launch_study --offline
cargo test --release -p beta-marker-nest -p beta-marker-match -p beta-marker-cli --offline -- --test-threads=2
cargo clippy --release -p beta-marker-nest -p beta-marker-match -p beta-marker-cli --all-targets --offline -- -D warnings
cargo test --release -p beta-marker-nest --test deadline --offline -- --ignored --exact simon_qa006_fixed_budget_matrix --nocapture
beta-marker nest <qa/factory-job.json> --time 1 --workers 2 --seed 42 --fixed <qa/locked-result.json> -o <qa/renested.json>
beta-marker validate <qa/factory-job.json> <qa/renested.json> --json
launch_study freedoms <development-40.jsonl> testdata/open/freesewing <fresh-output> 60 2
launch_study replay <frozen-executable> <interrupted-pair-directory> <replay-output>
launch_study summarize <output> <replay-output>
launch_study audit-tilt <development-40.jsonl>
launch_study trial <job.json> <options.json> <output>
```

Raw evidence lives under `$BETA_DATA_DIR/marker-bench/astra-fix/`. Source inputs,
geometry, executables, generated output and build products are not committed.
Engine code is committed as `e437b34`. The rule matrix uses a frozen working
implementation from before the final folded-only scheduling refinements; ordinary
open/tubular search settings were not changed afterward. The final fold trials
and CLI checks use the final engine. These frozen executable hashes distinguish
the measured artifacts from later analysis-tool builds:

| Artifact | SHA-256 |
|---|---|
| Baseline CLI, `9461f54` plus phase instrumentation | `deaeab0b31682087ef524b2bf96bdcb9c783f9c56f9c6ef2f04557e282f85666` |
| Baseline fold driver | `1d94651dcdf16c6e7aae28d15de9dcdbffa70d05db36f3c4cc418315a48be43c` |
| Frozen rule-study driver | `b34940762ee3c6ea7cef8603d5aa4a254bd0ab73a87056899a3492451a29e448` |
| Final CLI | `ae31e6e62e02b7349e9f791a238b43dad85b225a9a748239eb8a372092b18006` |
| Final fold driver | `e85d5164611aa29a1298cf5c79df0db60b67739b192ab15f0b04b90fcca85637` |

FreeSewing data
is MIT; the open Lectra dataset is Lallier et al., Zenodo 7030786 v1.1, CC BY 4.0.
No new third-party package is introduced.

After merge, repeat the portable Rust checks and the ignored Simon matrix with
the integrated CLI. Also run the existing long Simon production pipeline and
fixed-refinement callers such as the cluster strategies. UI cancellation must explicitly call the new cooperative
entry point to use its incumbent-return behavior; process-kill cancellation is
unchanged by this crate-only patch. No core contract change is required for the
deadline fix. Cross-bundle layer allocation remains a separate design issue.
