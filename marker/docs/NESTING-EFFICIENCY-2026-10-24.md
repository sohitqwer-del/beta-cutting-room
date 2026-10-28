# Marker efficiency experiments, 24 October 2026

Two measured engine changes are implemented on `marker/efficiency8` (engine commit `9f07d28`), based on `marker/mvp` `372ed68`. They are local experiments, not a published release or factory acceptance result.

## Measured result

Four public FreeSewing garment jobs were given an original synthetic 20 ? 20 mm defect at the fabric start. This represents a narrow failure in the current routing: any defect selects the simpler obstacle packer for the entire job. The new candidate compares that checked solution with Sparrow on the clean section after the damaged prefix.

Each pair uses identical width, pieces, quantities, mirror/grain rules and gaps. Three preselected seeds: 1, 7, 42. Both variants request 5 seconds of search. Actual wall time includes preparation and validation and differs; it is shown explicitly. Measurements were run sequentially after compilation, with before/after order alternated for seed 7.

| Job | Old efficiency | New efficiency | Mean fabric saved | Saving range, 3 seeds | Mean wall seconds, old / new |
| --- | ---: | ---: | ---: | ---: | ---: |
| Hugo hoodie | 65.3% | 74.9% | 12.8% | 12.0?14.0% | 5.04 / 6.09 |
| Simon shirt | 75.0% | 82.9% | 9.5% | 9.0?10.2% | 5.03 / 6.64 |
| Teagan T-shirt | 62.9% | 72.2% | 12.9% | 11.9?13.7% | 5.03 / 5.41 |
| Titan trouser block | 55.4% | 71.7% | 22.8% | 19.9?24.8% | 5.02 / 5.32 |

Fabric saved is `100 ? (1 - new_length / old_length)` for the same required pieces, not the efficiency-point difference. The whole reserved prefix is included in marker length and efficiency. Every targeted layout passed the core geometric/quantity/grain/gap validator. These are four selected public pattern families with a synthetic flaw, not a broad production benchmark. [Full paired values](efficiency8/summary.json).

The original locked U-shaped regression needs 110.002 mm before the change and 100 mm afterwards, with a 10 mm piece placed inside the existing pocket instead of at the tail. The locked piece stays exactly in place. This is a synthetic correctness regression, excluded from the garment table.

## Controls and uncertainty

The clean Teagan control uses the unchanged Sparrow route. Its three-seed mean was **74.41% before / 73.92% after**, corresponding to **-0.78% mean fabric saved**; individual pairs ranged from **-5.28% to +2.91%**. Repeating seed 1 on the candidate changed the result from 5.106 m to 4.781 m without source changes. Time-limited stochastic search is variable even with a recorded seed. The control therefore does not establish a general improvement or prove non-regression for ordinary clean markers.

The Teagan interior-defect control was 65.84% in both variants (one seed), with equal marker lengths. [Control values](efficiency8/controls-summary.json). A larger clean/interior/folded matrix and factory inputs are required before promoting this to a release.

## Implementation

- `v02/defects.rs`: run a short obstacle search, then compare a clean-suffix candidate. Keep the shorter independently valid complete result. Only open, unlocked jobs with short leading defects qualify. The prefix must be within both 5% of the area lower bound and one quarter of fabric width. Matching/shade/repeat cases and cooperative-cancellation calls keep their established path. Explicit maximum length is reduced for the clean search, then checked again after restoring coordinates.
- `v02/bounded/horizontal.rs`: enumerate horizontal vertex/edge and endpoint-clearance contact events for small concave pairs. Binary search assumes one collision interval and can skip an internal pocket. All candidate positions still use exact overlap/distance checks and final validation. The sweep is limited to at most 1,024 vertex pairs to protect the bounded search slices; larger curves retain the inexpensive search.
- The existing complete incumbent, locked transforms, demand validation, per-piece clearances, allowed rotations and layer allocation remain mandatory. No dependency sources, licenses or public API fields changed.

## Verification

- `cargo test --release -p beta-marker-nest --lib --offline`: **62 passed**, zero failed or ignored.
- `cargo clippy --release -p beta-marker-nest --all-targets --offline -- -D warnings`: passed.
- `cargo check -p beta-marker-nest --target wasm32-unknown-unknown --offline`: passed. This is a compatibility check, not a measured browser-speed result.
- 15 target pairs, four control pairs and one repeated control pair: **40 valid final layouts**. Initial exploratory runs are excluded from the three-seed table.
- Rust 1.98.1, Windows 11, same laptop and one requested worker. Sparrow may use coordinator/helper threads. No concurrent builds during final timing. No private factory data, external compute server or GUI launches were used.

## Reproduce

The Rust example owns case generation, nesting, validation, metrics and aggregation. Public input geometries remain under `testdata/open/freesewing`; their original notices and metadata apply. In the commands below, use the full path to the built `efficiency_study` example (with `.exe` on Windows), under `target/release/examples` or the configured external Cargo target directory.

```sh
cargo build --release -p beta-marker-nest --example efficiency_study --offline
# Run from marker/; OUT is an existing output directory.
efficiency_study --prepare . OUT/cases.json
efficiency_study OUT/cases.json OUT/new-1.jsonl 5 1
# Repeat with seeds 7 and 42. Use the baseline engine plus this same driver
# in an isolated worktree at 372ed68 for the before variant.
efficiency_study --summarize BEFORE_FILES_COMMA_SEPARATED AFTER_FILES_COMMA_SEPARATED OUT/summary.json
```

Raw layouts, public cases, binaries, dependency evidence, timing logs and the interactive comparison are retained in the task's external `efficiency8` data folder. The baseline binary was captured before the new modules were compiled; its dependency file has no horizontal-contact module. Aggregate results are committed here; no customer patterns were uploaded or published.

## Next improvements, in order

1. **Folded and tubular allocation:** search mixed half/full-cut policies per piece and size. The current layer allocator tests global symmetric-pairing policies; a mixture can use scarce fold-edge length more effectively. Judge useful full-piece yield and metres per garment, with exact layer/copy validation.
2. **Interior defects and locked layouts:** let the stronger optimizer work in valid free regions, followed by exact local repair, rather than routing the whole job through the simpler packer. Preserve fixed transforms and the complete incumbent. Benchmark scattered defects and several locked-piece densities.
3. **Browser search continuity:** retain useful exploration/compression state across rounds. Current browser rounds restart from placements and reconstruct search state. Measure real WASM at 20/60/120 seconds, including time to first complete marker and cancellation behavior.
4. **Ratio/width alternatives:** use the existing cut planner to compare legal size ratios and usable widths by total order metres, including overcut and spreading end allowances. A higher marker percentage alone need not minimize order consumption.

These are engineering priorities, not promised savings. Alternative search families are also worth measuring: [Sparrow](https://github.com/JeroenGar/sparrow) describes exploration and compression; [PackingSolver's irregular algorithms](https://fontanf.github.io/packingsolver/internals/irregular.html) describe tree-search methods. Neither source proves a gain on our garment rules without a paired test.
