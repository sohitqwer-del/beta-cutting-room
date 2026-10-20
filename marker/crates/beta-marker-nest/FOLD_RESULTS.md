# Folded and tubular markers: round 0.5

Teagan folded useful-demand yield rose from **42.75% to 62.94% at 60 s**
and from **42.79% to 64.86% at 300 s**, averaged over three paired seeds.
The marker shortened by **2.692 m** and **2.848 m**, respectively.
The synthetic 600 mm polo-body tube gained **3.16** and **2.94 percentage
points**. Teagan tubular was nearly unchanged. The other tube cases and the
hypothetical Simon/Titan folded cases showed little change.

This change adds useful-copy allocation across fabric layers. It also lets a
declared symmetric on-fold piece use either source half. Full pieces retain
their existing flip permissions. Open-fabric search defaults are unchanged.

## Contract and implementation

`Placement.layer_allocations` is optional and defaults to an empty list. Old
JSON keeps its legacy layer/bundle meaning. New layered results use one entry
for an on-fold cut, or two entries for a full cut: physical normal layer first,
mirrored layer second. Each entry names the demanded hand and garment bundle,
or is `null` for surplus. The primary `bundle` is the first useful assignment.

Both garments must have the piece's size. Asymmetric pieces keep their physical
handedness unless flipping is permitted. A declared fold line identifies a
symmetric piece, whose two layer copies can serve two normal demands. This
does not authorize a mirrored full placement when the piece has no mirror
demand or flip permission.

The independent validator checks exact useful demand globally and per garment.
It rejects wrong hands, missing or extra useful copies, wrong-size assignments,
invalid layer counts, and mixed explicit/legacy layered results. Shade, nap and
matching checks include every garment served by a cut. Surplus is reported and
never assigned to useful demand. Legacy surplus warnings remain supported.

The shared `rules::layer_assignments` helper feeds the cutting-room adapter,
the CLI physical-lay helper, and the XLSX bundle report. Their tests check that
shared layers receive separate garment identities and surplus receives no label.
The TypeScript contract and shared JSON fixture include the new field.
In the XLSX bundle table, a shared cut appears once for each garment it serves;
the per-bundle placement counts must not be summed as physical cut counts.

Layer allocation happens before geometry search. The packer compares fold
halves with paired full cuts, using the same total deadline. It keeps the best
validated marker while constructing and compacting alternatives. Near-equal lengths
(within 0.01 mm) prefer less occupied cut area, avoiding surplus for an
insignificant numerical length gain.

Folded jobs first try shared full cuts for 60% of the remaining allowance, then
the fold halves. Tubular jobs retain the existing fold search for
half the allowance, then divide the remaining allowance 80/20 between
fold halves and paired full cuts. The extra alternatives only run when
they produce distinct allocations. Narrow symmetric mirror pairs can use two
halves when a full cut will not fit. Full-only jobs retain the native Sparrow
adapter when its usual constraints permit it.

Fixed explicit assignments consume their exact garment demand. Warm starts
retain their allocation and checked geometry. Shared layers couple garment
directions under one-way-per-bundle nap. Legacy fixed/warm markers keep their
old allocation semantics; a fresh nest is needed to introduce cross-garment
allocation into such a marker.

## Measurement protocol

Baseline: `0d7b298cd0b31b92617d7d866efa8fcfb37baad9`. Separate release executables
are frozen before paired measurement. All inputs are public. There are no
factory inputs in the compute-server study.

- Seeds: **1, 42, 2026**. Budgets: **60 s and 300 s**.
- Real FreeSewing fixtures: Teagan, Simon and Titan, S1/M2/L2/XL1.
  Open width is 1,500 mm; folded/tubular flat width is 750 mm.
  Only Teagan's declared symmetric front/back receive fold lines.
- These FreeSewing scenarios retain the source fixtures' zero piece and edge
  gaps, zero tilt, no nap, and no optional flipping. The separate QA-006 check
  uses 3 mm piece gaps and 5 mm edge gaps.
- Three synthetic knit-body scenarios use unmodified FreeSewing Teagan M
  outlines: T-shirt at 450 mm tube width, four garments; sleeveless body study
  at 525 mm, six garments; polo-body study at 600 mm, six garments. The
  sleeveless scenario omits sleeves. The polo scenario omits collar/placket.
  These are packing studies, not complete new sewing patterns.
- Every layered case has an adjacent before/after pair at each budget and seed.
  Arm order alternates. Corresponding open jobs run once per budget/seed using
  the new executable. Open regression has its own paired comparison: the first
  ten tasks of the frozen Lectra development file, 60 s, all three seeds.
- Two trials run concurrently, each with one separator worker. Processes run
  at nice level 10 and are restricted to eight logical CPUs. The supervisor
  permits budget plus 30 s, records child-process wall time, and retains failures.
  It polls every 50 ms. The additional parent validation is outside that timer.
- Host: shared Linux server, AMD Ryzen 9 9900X3D, 12 cores / 24 logical CPUs.
  Rust 1.98.1, release profile. Affinity restricts this study to CPUs 0–7.
- Child and parent validators must both accept a scored marker. The raw job,
  options, result, progress, validation, wall time and errors are retained.
  Expected surplus warnings do not make a marker invalid.

Useful-demand yield is requested full-piece area divided by
`layers × flat width × marker length`. Occupied efficiency uses the area drawn
on the spread, including surplus. Surplus area is reported in square metres
of flat spread area; multiply by two for physical two-layer fabric area.
No surplus contributes to useful yield.

An exploratory matrix prefix exposed a tubular regression from spending too
little time on the established half-based search. That entire prefix is
excluded from the final matrix and retained as exploratory evidence. The
revised policy retains the best validated result from that search. No pair is selected
for its score. Shared-server load can still change finite-budget results.
The development rows compare BETA before and after this change; they do not
compare against Lectra's recorded efficiencies.

The eight modified core/nesting runtime files match the measured source
snapshot byte for byte. Later edits only affect tests, documentation, and
study-output handling. The frozen Linux executables are:

| Executable | SHA-256 |
|---|---|
| Before | `bd81b2c8e24c53b678ba7445a059d61cd597b9ec46b6bd75366102db7713839e` |
| After | `c839a776bb2fcad8c6ba9daa52e41980c443c904805338a31b4166ed240c8950` |

The frozen development-file SHA-256 is
`b31f996fd1e0dfd831980165949eda8a0ec4537c40febc5e0ffe6c1403bce088`.

## Results

All **180 planned calls** finished: **174 returned independently valid markers**
and **six failed to nest**. There were **69 valid pairs out of 72 attempted
pairs**, plus **36 valid open comparators**. All **42 layered before/after
pairs** were valid. The six failures were Task 12127, both builds, all three
seeds; the reported error was “sparrow could not construct or optimise a
marker.” These calls have no numeric score. No call reached the supervisor cap.

Real folded/tubular fixtures use 750 mm flat width. Synthetic widths are shown
in their labels. Open comparisons use twice the corresponding flat width.

### Layered jobs, 60 s

Three-seed arithmetic means. Surplus is flat spread area in m². Quality means use valid pairs; wall-time means include all attempts.

| Case | Valid pairs | Useful % before → after | Gain pp | Occupied % before → after | Surplus m² before → after | Length m before → after | Wall s before → after |
|---|---:|---:|---:|---:|---:|---:|---:|
| Simon folded (hypothetical) | 3/3 | 53.3493 → 53.2458 | -0.1034 | 82.7994 → 82.6389 | 2.7030 → 2.7030 | 12.2378 → 12.2615 | 60.7717 → 60.9107 |
| Synthetic polo body, 600 mm tube | 3/3 | 74.9859 → 78.1472 | 3.1612 | 74.9859 → 78.1472 | 0.0000 → 0.0000 | 5.7440 → 5.5116 | 60.0365 → 60.1770 |
| Synthetic tank body, 525 mm tube | 3/3 | 88.1589 → 88.1589 | 0.0000 | 88.1589 → 88.1589 | 0.0000 → 0.0000 | 4.5512 → 4.5512 | 60.0328 → 60.0232 |
| Synthetic T-shirt, 450 mm tube | 3/3 | 57.9531 → 57.9657 | 0.0125 | 57.9531 → 57.9657 | 0.0000 → 0.0000 | 6.6064 → 6.6050 | 60.0377 → 60.0280 |
| Teagan folded | 3/3 | 42.7472 → 62.9421 | 20.1949 | 59.8953 → 65.7067 | 1.0799 → 0.1199 | 8.3963 → 5.7046 | 60.0397 → 60.0775 |
| Teagan tubular | 3/3 | 68.2757 → 68.2357 | -0.0400 | 68.2757 → 68.2357 | 0.0000 → 0.0000 | 5.2576 → 5.2608 | 60.0432 → 60.0283 |
| Titan folded (hypothetical) | 3/3 | 73.9994 → 73.9983 | -0.0011 | 73.9994 → 73.9983 | 0.0000 → 0.0000 | 7.3919 → 7.3920 | 60.3239 → 60.2451 |

### Open comparators, 60 s

| Case | Valid / attempted | Useful and occupied % | Surplus m² | Length m | Wall s |
|---|---:|---:|---:|---:|---:|
| Simon, 1,500 mm open | 3/3 | 88.4382 | 0.0000 | 7.3824 | 61.3799 |
| Synthetic polo body, 1,200 mm open | 3/3 | 81.6113 | 0.0000 | 5.2777 | 60.2241 |
| Synthetic tank body, 1,050 mm open | 3/3 | 56.3685 | 0.0000 | 7.1179 | 60.1137 |
| Synthetic T-shirt, 900 mm open | 3/3 | 63.0814 | 0.0000 | 6.0694 | 60.1802 |
| Teagan, 1,500 mm open | 3/3 | 79.1870 | 0.0000 | 4.5327 | 60.3486 |
| Titan, 1,500 mm open | 3/3 | 75.5724 | 0.0000 | 7.2381 | 60.3767 |

### Layered jobs, 300 s

Three-seed arithmetic means. Surplus is flat spread area in m². Quality means use valid pairs; wall-time means include all attempts.

| Case | Valid pairs | Useful % before → after | Gain pp | Occupied % before → after | Surplus m² before → after | Length m before → after | Wall s before → after |
|---|---:|---:|---:|---:|---:|---:|---:|
| Simon folded (hypothetical) | 3/3 | 54.4135 → 54.3850 | -0.0285 | 84.4512 → 84.4070 | 2.7030 → 2.7030 | 11.9983 → 12.0046 | 300.8129 → 300.9840 |
| Synthetic polo body, 600 mm tube | 3/3 | 75.2590 → 78.1991 | 2.9401 | 75.2590 → 78.1991 | 0.0000 → 0.0000 | 5.7232 → 5.5080 | 300.0429 → 300.1854 |
| Synthetic tank body, 525 mm tube | 3/3 | 88.1589 → 88.1589 | 0.0000 | 88.1589 → 88.1589 | 0.0000 → 0.0000 | 4.5512 → 4.5512 | 300.0440 → 300.0520 |
| Synthetic T-shirt, 450 mm tube | 3/3 | 58.0223 → 58.0223 | 0.0000 | 58.0223 → 58.0223 | 0.0000 → 0.0000 | 6.5985 → 6.5985 | 300.0456 → 300.0400 |
| Teagan folded | 3/3 | 42.7934 → 64.8578 | 22.0644 | 59.9539 → 67.4663 | 1.0795 → 0.1082 | 8.3872 → 5.5394 | 300.0619 → 300.0731 |
| Teagan tubular | 3/3 | 69.1745 → 69.3093 | 0.1348 | 69.1745 → 69.3093 | 0.0000 → 0.0000 | 5.1887 → 5.1786 | 300.0344 → 300.0499 |
| Titan folded (hypothetical) | 3/3 | 74.0244 → 74.0348 | 0.0104 | 74.0244 → 74.0348 | 0.0000 → 0.0000 | 7.3894 → 7.3884 | 300.2606 → 300.1798 |

### Open comparators, 300 s

| Case | Valid / attempted | Useful and occupied % | Surplus m² | Length m | Wall s |
|---|---:|---:|---:|---:|---:|
| Simon, 1,500 mm open | 3/3 | 89.0633 | 0.0000 | 7.3305 | 301.2561 |
| Synthetic polo body, 1,200 mm open | 3/3 | 81.6368 | 0.0000 | 5.2760 | 300.2019 |
| Synthetic tank body, 1,050 mm open | 3/3 | 56.3803 | 0.0000 | 7.1165 | 300.1953 |
| Synthetic T-shirt, 900 mm open | 3/3 | 63.2949 | 0.0000 | 6.0490 | 300.1804 |
| Teagan, 1,500 mm open | 3/3 | 79.6534 | 0.0000 | 4.5060 | 300.4081 |
| Titan, 1,500 mm open | 3/3 | 76.6394 | 0.0000 | 7.1373 | 300.5914 |

### Open development regression, 60 s

| Task | Valid pairs | Useful % before → after | Mean gain pp | Win / tie / loss | Failed before / after |
|---|---:|---:|---:|---:|---:|
| Task 8411 | 3/3 | 79.6370 → 79.4645 | -0.1725 | 0/2/1 | 0/0 |
| Task 1561 | 3/3 | 91.6601 → 91.6576 | -0.0025 | 0/3/0 | 0/0 |
| Task 88853 | 3/3 | 73.3564 → 73.3543 | -0.0021 | 0/3/0 | 0/0 |
| Task 14851 | 3/3 | 81.0070 → 81.1311 | 0.1242 | 3/0/0 | 0/0 |
| Task 18110 | 3/3 | 82.4323 → 82.5328 | 0.1005 | 1/2/0 | 0/0 |
| Task 4122 | 3/3 | 76.2996 → 76.3723 | 0.0726 | 1/2/0 | 0/0 |
| Task 18896 | 3/3 | 79.9797 → 79.9849 | 0.0051 | 0/3/0 | 0/0 |
| Task 58688 | 3/3 | 82.4954 → 82.5577 | 0.0623 | 1/2/0 | 0/0 |
| Task 12127 | 0/3 | — → — | — | 0/0/0 | 3/3 |
| Task 89590 | 3/3 | 75.1820 → 75.1312 | -0.0507 | 0/2/1 | 0/0 |

Mean gain across valid development pairs: 0.0152 pp (27 pairs).
Mean of 9 task means: 0.0152 pp; approximate 95% t interval [-0.0541, 0.0845] pp. This describes this selected subset, not the full benchmark population.
Mean wall time across valid development pairs: 60.9722 s before, 61.0218 s after.

The open check shows no meaningful regression in this selected subset:
**6 wins, 19 ties and 2 losses** over the 27 valid pairs, using a 0.1-point
tie threshold. The mean quality difference is close to zero, its interval
includes zero, and mean wall time differs by only 0.050 s. This is not a
claim of a general improvement over the full public dataset.

The main gain is reduced surplus and shorter markers for declared symmetric
torso pieces on folded fabric. At 300 s, Teagan folded still trails its open
comparator, **64.86% versus 79.65%**. Tubular Teagan gains only 0.13 point at
300 s and loses 0.04 point at 60 s. The tank-body tube already packed well
before this change; its open/tube difference is not a gain from this patch.
Simon still creates 2.703 m² of surplus spread area under the hypothetical
folded rules. No extra mirror or flip permission was added to remove it.
The search remains a heuristic and does not test every layer allocation.

## Verification and remaining integration work

Regression tests cover separate layer garments, exact handed demand, surplus,
wrong sizes, open-fabric rejection, symmetric half choice without full flipping,
shared-layer shade/nap/matching, narrow symmetric pairs, warm/fixed allocations,
faceless fixed demand, CLI part mapping, cutting-room labels, and XLSX bundles.

| Check | Result |
|---|---|
| `cargo test --workspace --exclude beta-cutting-room --offline -- --test-threads=2` | 375 passed, 0 failed, 9 ignored |
| Strict Clippy, all targets of core/nest/match/CLI/cutroom/export | Passed, warnings denied |
| Release `validation_speed`, run separately with `--ignored --nocapture` | 1 passed; 1,000 pieces × 100 vertices in 184.79 ms |
| `npx vitest run marker` | 73 passed in 12 files |
| `npm run typecheck -w @beta/marker-wasm` | Passed |

The final rebuilt Windows CLI completed the public Simon QA-006 case with
`--fixed --time 1 --workers 2 --seed 42` in **1.993 s**, including process startup,
JSON I/O and shutdown. It returned all **114 placements** at **8.818486 m** and
**74.0341%** occupied efficiency. The locked placement retained every numeric
transform and identity field exactly. A separate CLI validation returned no
errors or warnings. The earlier development build took 2.404 s; both records
remain in the evidence directory. This checks the requested budget-plus-ten
seconds behaviour for this fixture, not a hard real-time guarantee for all jobs.

Two initial Windows test assertions depended on a 50 ms optimizer tie between
one shared cut and two surplus-producing cuts. The cross-garment fixture now
gives the shared cut a clear geometric advantage. The other test checks exact
useful demand and honest surplus reporting. The search code was not changed
for these test corrections. The final full workspace run above includes them.

The editor's automatic replacement of deleted copies still assumes a single
bundle per placement when reconstructing new copies. Existing explicit
placements retain their assignments when moved, and the independent validator
blocks inconsistent replacement assignments. The editor owner should use the
new shared helpers before enabling automatic replacement for these markers.

The matching search still needs integrated multi-layer factory scenarios after
merge; independent matching checks already include both assigned garments.
Rebuild the CLI and WASM validator together when integrating this contract.
Simon and Titan are hypothetical material-layout comparisons, not claims that
these woven-shirt/trouser patterns are tubular knit garments. A declared
symmetry line is a caller assertion; the allocator does not infer new symmetry
or silently grant flip permission.

No dependency is added. FreeSewing data is MIT. The public Lectra benchmark
dataset is Lallier et al., Zenodo 7030786 v1.1, CC BY 4.0. Sparrow and its
unmodified Jagua dependency are unchanged.

## Reproduction

Set the assigned `CARGO_TARGET_DIR`, `CARGO_BUILD_JOBS=3` and `BETA_DATA_DIR`.
Build and freeze the baseline and updated `launch_study` executables separately.
From `marker/`:

```text
cargo build --release -p beta-marker-nest --example launch_study --offline
launch_study fold-matrix <before-exe> <after-exe> testdata/open/freesewing <development-40.jsonl> <fresh-60-output> 60 2
launch_study fold-matrix <before-exe> <after-exe> testdata/open/freesewing <development-40.jsonl> <fresh-300-output> 300 2
launch_study fold-summary <study-root-containing-60-and-300> <summary-output>
cargo test --workspace --exclude beta-cutting-room --offline -- --test-threads=2
cargo clippy -p beta-marker-core -p beta-marker-nest -p beta-marker-match -p beta-marker-cli -p beta-marker-cutroom -p beta-marker-export --all-targets --offline -- -D warnings
npx vitest run marker
npm run typecheck -w @beta/marker-wasm
```

Raw evidence is stored below `$BETA_DATA_DIR/marker-bench/fold-r05/`.
The checked `evidence.tar.gz` SHA-256 is
`1aaaf68cef2e38a555406b11e92a685bbd65451f7e81eb473097ba82c34fea26`.
It retains the original Linux trial directory names, including quoted task
IDs. Portable `study/60/rows.json`, `study/300/rows.json` and `final-summary/`
are extracted beside it. The current study driver creates Windows-safe
numeric task names. The exploratory prefix remains separate and excluded.
Datasets, executables and build output are not committed. No code is pushed
or published.
