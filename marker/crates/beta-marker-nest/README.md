# Garment nesting adapter

`nest(&Job, &NestOptions)` returns a checked marker. Ordinary jobs run Sparrow
0.3.0 as a library; fixed pieces, defects and fold edges use an f64 obstacle
packer. `nest_with_progress`
adds a callback with elapsed seconds and the best feasible cut-outline length.
Neither function writes files, installs a logger or handles Ctrl-C. Sparrow and
jagua send engine messages through `log`; the calling application controls them.

Version 0.2 adds fixed placements, defects, parallel portfolios, native warm
starts, garment bundles, faceless fabric, layered/folded layouts, fusing blocks
and shrinkage. See [V02.md](V02.md) for the design and limits, and
[V02_RESULTS.md](V02_RESULTS.md) for feature measurements and verification.
The engine mapping below describes the ordinary Sparrow path.

Jagua's strip height is fixed (marker y). Its width is minimised (marker x).
Each normal/mirrored demand becomes one engine item. Its outline is mirrored,
then aligned to its grain. The engine receives only relative allowed rotations.
Engine reflection is disabled; mirrored copies have separate demands.
Jagua export reverses its internal centring before our grain angle is added back.

Each collision outline is offset by half the piece gap plus its own buffer.
The engine strip height is `fabric.width - 2 * edge_gap + piece_gap`.
Translations gain `edge_gap - piece_gap / 2` on both axes. The extra space at the
end of the engine strip does not enter the reported length, which is recomputed
from original outlines. Every outline is cleaned of consecutive duplicates,
repeated closing points and redundant collinear vertices. Winding is normalized;
self-intersections and backtracking spikes are rejected with the piece ID and
validation details. Original cut lines remain unchanged in the job.

Round offsets get a 0.2% radius increase for chord approximation. Every collision
shape also gets `0.002 + 32 * f32::EPSILON * max_abs_coordinate` mm of numerical
guard, including when the requested gap is zero. This covers input rounding and
centring; mapped results are still checked independently. Full polygon coverage
of the original outline and requested clearance is checked after all engine
modifications. If simplification loses coverage, import retries without
simplification or concavity closure. A shape that still fails coverage is rejected
with its piece ID rather than sent to the optimizer.

Tilt samples the exact base direction, +/- min(0.5 degrees, tilt), and both tilt
endpoints. There are at most four extra directions per base. Free rotation uses
jagua's continuous mode. The narrow panel example requires -2 degrees to fit;
it cannot fit in the fixed width at zero degrees.

Sparrow's time limit is split 80/20 between exploration and compression, as in the
sparrow CLI. Import, initial construction, callbacks and final checks add runtime.
Early termination uses sparrow's default consecutive-failure and shrink-decay
settings. Worker counts configure both separator phases. A supplied seed fixes
the random streams; wall-clock stopping and scheduling mean exact final-output
replay is not guaranteed. Use the recorded seed and worker count for comparisons.

Every returned result passes `beta_marker_core::validate` on effective cut
boundaries, including shrinkage, folds and blocks. This includes counts, grain, edges, clearance, positive interior overlap
and `max_length`. No validator error code is ignored. If final validation finds
contacts on the ordinary path, the adapter tries bounded lengthwise nudges, then separates the
remaining offending copies into disjoint boxes at the marker end. Repairs are
rechecked and reported as warnings. Unrepairable errors return no result.
Fixed placements never enter that repair path.

Progress callbacks report only improvements that pass the independent validator.
They record elapsed wall time, including instance construction and validation.

## Reproduce examples and checks

Run Cargo from the `marker` directory. Set `CARGO_TARGET_DIR` to a suitable build
folder outside the source tree before every command, and set
`CARGO_BUILD_JOBS=3` and `BETA_DATA_DIR` to the assigned data location.

```text
cargo run -p beta-marker-nest --example generate_jobs -- examples/jobs
cargo test --workspace
cargo clippy --workspace --all-targets
cargo run --release -p beta-marker-cli -- nest examples/jobs/shirts-sml.json --time 60 --seed 1 --workers 3 -o result.json
```

The generated examples are original synthetic patterns, not production patterns.
They cover a shirt size ratio with left/right fronts and sleeves, one-way
trousers, unequal buffers and gaps, and tilt.

## Academic comparison

Build the unmodified sparrow CLI in release mode, using the same target folder.
If its source has no Cargo.lock, use a temporary copy rather than changing a
read-only checkout. A copy nested inside this workspace needs an empty
`[workspace]` table in its manifest; no engine source changes are needed.

```text
cargo run --release -p beta-marker-nest --example sanity -- <sparrow-source>/data/input/shirts.json <raw-sparrow-executable> crates/beta-marker-nest/out/shirts 60
cargo run --release -p beta-marker-nest --example sanity -- <sparrow-source>/data/input/trousers.json <raw-sparrow-executable> crates/beta-marker-nest/out/trousers 60
```

The example converts each input to a Job with fixed strip height as fabric width,
zero gaps, and 0/180 rotations. It runs the adapter and raw CLI sequentially, each
with seed 1, three workers and a 60-second search budget. It writes the converted
job, result, improvements and summary into the supplied output folder. It also
converts and independently validates the raw layout, writing
`raw-beta-result.json` and `raw-validation.json`. Raw CLI
SVGs and logs stay in that folder. No benchmark input or generated SVG is shipped
with the product. A difference worse than -0.5 percentage points exits with an
error. Saved raw output can be checked without another search:

```text
cargo run --release -p beta-marker-nest --example sanity -- --check-raw <job.json> <raw.json> <output-directory>
```

Historical results are in `BENCHMARKS.md`; current measurements, including both
shirts trials and their raw-comparison outcomes, are in `V02_RESULTS.md`.

## Direct dependency licences

Direct adapter dependencies include `rand` 0.10 (MIT OR Apache-2.0) and `geo` 0.33
(MIT OR Apache-2.0). Both were already in sparrow/jagua's resolved dependency
graph. No new transitive packages are needed by the adapter. Sparrow remains
MIT and jagua-rs remains an unmodified MPL-2.0 dependency. Their source is not
vendored or patched in the committed adapter.

## Real garment benchmarks

`garment_bench` runs one input job with seed 1 and three separator workers. It
writes `result.json`, `validation.json`, `progress.json` and `summary.json` to the
specified output directory. The summary records the best valid efficiency at
10, 30, 60, 120 and 300 elapsed seconds when those checkpoints are in the budget.

```text
cargo run --release -p beta-marker-nest --example garment_bench -- testdata/open/freesewing/simon-ratio.job.json 300 <output-directory>
```

See `ROUND2.md` for the zero-gap precision regression and real garment results.
