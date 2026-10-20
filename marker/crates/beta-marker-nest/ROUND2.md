# Round 2: CAD nesting safety and real garment benchmarks

## Root cause and evidence

The zero-gap path did not reserve any numerical clearance. The adapter rounded
f64 cut outlines to f32 before jagua centred, simplified and placed them. That
conversion can move an edge inward. A collision-free engine layout can therefore
contain a positive overlap between the original cut lines. The adapter's old
check eroded both outlines by 0.005 mm, which hid such overlaps; the new core
validator correctly rejects them.

This is a non-conservative precision conversion in the adapter, not evidence of
a validator false positive or a jagua surrogate defect. All 35 windbreaker
outlines passed the original simple-polygon check. A diagnostic comparison of
original outlines and unguarded engine collision polygons measured positive
missing area for 33 shapes, up to 0.014748 mm2. The reported four-vertex sleeve
under-panel loses coverage even with simplification and concavity closure
turned off. This is asserted directly in the regression test.

The smallest own fixture uses a rectangle with right edge x=10.0000004. It rounds
to x=10 in f32. Two engine rectangles separated by 10 mm touch, while the true
rectangles overlap by 0.0000004 mm, or about 0.000004 mm2. Core validation rejects
that overlap. The old eroded check misses it. Its area is checked analytically:
polygon Boolean clipping can snap away a sliver this small.

Two unfixed 60 s windbreaker runs did not repeat the exact reported overlap
pair. Wall-clock cutoffs and worker scheduling vary placements despite seed 1.
The coverage loss and minimal overlap mechanism are reproduced deterministically.

## Fix

- Remove consecutive duplicate vertices, repeated closing vertices and redundant
  collinear vertices before engine import. Normalize winding. Reject crossings
  and backtracking spikes with the piece ID and geometry validation details.
- Inflate every collision shape, including zero-gap shapes, by the required
  clearance plus a scale-aware f32 guard. Keep original cut lines unchanged.
- Check coverage of the entire original polygon and clearance after every engine
  modification. Retry without simplification or concavity closure if needed;
  reject any outline whose collision shape still loses coverage.
- Run the real independent validator on returned results and progress improvements.
  Remove the `not_implemented` exception and the old eroded overlap check.
- If final contacts remain, try 0.01, 0.1 and 1 mm lengthwise nudges, then place
  remaining offending copies in disjoint boxes at the marker end. Recompute
  metrics and revalidate after repair. Report repairs as warnings. Return an
  error if validation still fails. No validator error is ignored.

Regression coverage includes the own four-vertex fixture, the actual two reported
Open+Pattern pieces (CC BY 4.0 with attribution), duplicate/collinear cleanup,
self-intersection and spike rejection, overlapping identical-copy repair,
clearance/edge/count/grain checks, and the max-length failure path.

## Run conditions and evidence

Release build, seed 1, three separator workers, early termination disabled.
At most four benchmark processes ran concurrently on the 16-core machine.
Workspace test compilation overlapped the first portion of the batch; Clippy
compilation (two build jobs) overlapped later runs. Treat these as shared-machine
measurements, not isolated-machine speed estimates. Every run below passed
`beta_marker_core::validate` with no warnings and no final placement repairs.

Search budgets exclude some startup/final-check overhead. Timings are measured
wall times returned by the adapter. There are no claims of a proven optimum.

Raw results, validation reports, progress records, logs and variant jobs are in
`$BETA_DATA_DIR/marker-bench/nest-round2/<run-name>/`. Source jobs under
`testdata/open/` were not changed. The example `garment_bench` writes all four
JSON evidence files and validates its own result.

## Requested garment results

The windbreaker test mixes shell, lining and reflective pieces as requested; it
is not a single-material production marker.

| Job | Width mm | Budget s | Length mm | Efficiency | Runtime s | Valid |
|---|---:|---:|---:|---:|---:|---|
| Windbreaker M (35 pieces) | 1500 | 60 | 2692.95 | 84.11% | 63.64 | Yes |
| Simon ratio | 1500 | 120 | 7336.16 | 88.99% | 123.33 | Yes |
| Teagan ratio | 1500 | 120 | 4572.92 | 78.49% | 122.79 | Yes |
| Titan ratio | 1500 | 120 | 7168.67 | 76.30% | 121.64 | Yes |
| Hugo ratio | 1500 | 120 | 6675.35 | 82.87% | 122.95 | Yes |
| Bella ratio | 1500 | 120 | 3450.46 | 84.07% | 121.19 | Yes |

## Simon time to solution: one 300 s run

| Elapsed wall time s | Best valid length mm | Efficiency |
|---:|---:|---:|
| 10 | 7556.59 | 86.40% |
| 30 | 7369.93 | 88.59% |
| 60 | 7340.50 | 88.94% |
| 120 | 7340.50 | 88.94% |
| 300 | 7310.04 | 89.31% |

The final result was 7306.55 mm / 89.35% at 307.53 s. The 300 s checkpoint is the
best validated callback available by exactly 300 elapsed seconds, not the final
result. From 60 to 300 s this trajectory saves 30.45 mm (about 0.41% of length)
and gains 0.37 percentage points. A 60-120 s wait is a useful starting point for
this job; longer waits buy smaller gains in this single run.

The optimizer splits the chosen budget 80/20 between exploration and compression.
Thus the 120 s checkpoint of a 300 s run is still exploration; it is not equivalent
to an independent 120 s run that already entered compression. Phase budgets and
machine scheduling matter when comparing trajectories.

## Titan width sensitivity (120 s each)

| Job | Width mm | Budget s | Length mm | Efficiency | Runtime s | Valid |
|---|---:|---:|---:|---:|---:|---|
| Titan 1500 mm | 1500 | 120 | 7168.67 | 76.30% | 121.64 | Yes |
| Titan 1600 mm | 1600 | 120 | 6498.91 | 78.91% | 121.29 | Yes |
| Titan 1800 mm | 1800 | 120 | 5958.64 | 76.50% | 121.78 | Yes |

Wider fabric shortens the marker but does not guarantee higher area efficiency.
1600 mm is best among these three seed-1 runs. No repeated-seed confidence
interval or optimality claim is implied.

## Additional diagnostics

| Job | Width mm | Budget s | Length mm | Efficiency | Runtime s | Valid |
|---|---:|---:|---:|---:|---:|---|
| Simon, longer search | 1500 | 300 | 7306.55 | 89.35% | 307.53 | Yes |
| Titan, longer search | 1500 | 300 | 7135.67 | 76.66% | 301.48 | Yes |
| Teagan, quarter-turn variant | 1500 | 120 | 4176.66 | 85.93% | 120.58 | Yes |
| Titan, quarter-turn variant | 1500 | 120 | 7294.55 | 74.99% | 120.72 | Yes |

Quarter-turn rows validate against the altered variant jobs only. They relax the
original grain rule and are diagnostic counterfactuals, not production results.
All width variants and rotation variants remain outside `testdata/open/`.

Teagan's M front/back are each about 719.64 mm along grain and 579.50 mm across
it; the sleeve spans 429.09 mm across grain. Two torso bounding boxes use about
1159 mm across a 1500 mm strip, leaving about 341 mm, less than one normal sleeve
span. Quarter-turns can use those spaces differently: they improve this run by
7.45 percentage points. Width, shapes and grain constraints therefore account
for much of the low T-shirt figure. Within the original 120 s run, Teagan improves
from 76.96% at 60 s to 78.49% at the end, so time still helps. No tubular fabric,
folded cutting or paired layers are modelled; the fixtures are unfolded outlines.

Titan's M back/front span about 1235/1218 mm along grain and 429/342 mm across it.
Each uses only about 71% of its bounding rectangle because of its leg/crotch
shape. Two back/front bounding-box pairs need about 1542 mm across; 1600 mm
admits this simple lane arrangement while 1500 mm does not. This is a packing
explanation, not a geometric bound: concave outlines can interlock. An 1800 mm
strip adds width without necessarily adding another useful lane.

Titan's 300 s run improves only 0.35 percentage points over its independent
120 s result. Quarter-turns produce a worse 120 s solution (74.99%). Additional
rotations enlarge the search space and do not guarantee a better finite-time
solution. These tests point mainly to shape/width packing and search quality,
not an accidental adapter rotation restriction or a time limit alone. The adapter
correctly preserves the job's two grain-aligned directions; changing the
production rule would require fabric/design approval.

## Verification and owner requests

`cargo test --workspace`: 62 passed, two existing long tests ignored.
`cargo clippy --workspace --all-targets -- -D warnings`: passed with no warnings.
All Cargo commands used the prescribed external target directory.
A rebuilt release CLI also nested the full 35-piece windbreaker in a 2 s smoke
run; the separate `beta-marker validate --json` command returned `ok: true`
with no errors or warnings.

No core, validator, importer, workspace manifest, source open-pattern job or
engine dependency was changed. No new dependency or lockfile change is needed.

SPEC/UI owner: please reconcile any text promising a warning-only `max_length`
result. To satisfy the requirement that every returned result pass the real
validator, nesting now treats `too_long` as an error, matching core validation.
No validator or importer fix is requested for the reported overlap.

Reproduce from `marker/` after setting `CARGO_TARGET_DIR` and `BETA_DATA_DIR`:

```text
cargo run --release -p beta-marker-nest --example garment_bench -- <job.json> 120 <output-directory>
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```
