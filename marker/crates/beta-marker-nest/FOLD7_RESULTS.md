# Folded and tubular markers: round 0.7 (fold7)

## Summary

Folded and tubular results now report **useful yield** and **surplus copies**
next to footprint efficiency. The engine wastes much less fabric where the
waste was avoidable, and explains the cases where the order itself sets the length.

| Case (browser session, 20 s, 3 seeds) | m/garment before | after | Useful yield before → after |
|---|---:|---:|---:|
| Shirt DXF, folded 736.6 mm, S2 M3 L3 XL2, as imported | 3.339 | **1.678** | 30.1% → 59.9% |
| Same, fabric may be cut face down (`allow_flip`) | 1.752 | **1.401** | 57.4% → 71.8% |
| BETA T-shirt S–XXL × 1, tube 736.6 mm | 0.960 | **0.901** | 66.5% → 70.7% |
| Teagan S1 M2 L2 XL1, tube 750 mm | 0.927 | **0.872** | 64.6% → 68.6% |
| Simon, folded 750 mm (hypothetical) | 2.127 | **1.879** | 51.2% → 57.9% |
| BETA T-shirt S–XXL × 1, folded 736.6 mm | 1.392 | 1.392 | 45.7% → 45.7% |
| Teagan, folded 750 mm | 0.968 | 0.970 | 61.8% → 61.7% |
| Open lays (5 cases) | — | within ±0.3% | no regression |

The ~5–8% target against the open lay is **not** met overall:

- **Close:** Titan folded (+1.4%) and the shirt with flip permission (+9.6% at 20 s).
- **Tubes:** about +11 to +12%.
- **Folded T-shirt and Teagan:** +23% to +74%. With odd counts per size, every body is
  a fold half or a full cut too deep to sit beside one, so the body lengths add up along
  the marker (see below). These markers were already near that limit; the engine
  now says so with a `Fold edge:` warning.
- **Shirt without flip (+31%) and Simon (+48%):** they keep one unusable mirrored copy
  of each asymmetric Quantity-1 piece. This is reported as surplus.

## What changed

**Useful yield in every result.** `MarkerResult`, `ValidationReport` and
`Metrics` (native, WASM and `@beta/marker-wasm` types) now carry:

| Field | Meaning |
|---|---|
| `useful_piece_area` | mm² of full piece copies that serve the requested garments, summed over all layers. Surplus copies are excluded. |
| `useful_yield` | `useful_piece_area / (layers × width × length)`, a fraction. Layers: 1 open, 2 folded or tubular. |
| `surplus_copies` | Full copies cut through the layers that serve no garment. |

`efficiency` keeps its old meaning (occupied footprint). The validator
recomputes all three fields, so old saved results get them from
`validate`. Sums run in a fixed order, so native and WASM agree to the bit.

**Symmetric pieces serve both layers.** `rules::mirror_symmetric` tests whether
an outline equals its own mirror image within 0.2 mm (vertex sampling, start
point and winding do not matter). It checks the grain axis, and the cross-grain
axis when nap and rotation rules allow a half turn. On folded and tubular
fabric such a piece's mirrored layer can serve a normal demand, even with
`allow_flip = false`. The allocator, the validator and the metrics share this
rule (`rules::layer_hands_interchangeable`). A declared fold line still counts
as symmetric. This is not flip permission on open fabric.

**Fold halves inside Sparrow.** Before, any job with a fold half used the
bounded obstacle packer, which packs much less densely than Sparrow. Sparrow cannot fix a
piece's y position, so `v02/pinned.rs` joins each half into an outline that
spans the usable width to within 0.004 mm:

- folded fabric: the half plus a 1 mm bar to the selvedge;
- a tube: a half on the top fold joined by a short bar to a half on the bottom
  fold, centred on each other, so both folds are used at the same place.

The bar length is calibrated against Sparrow's own inflated collision outline
(simplification is switched off for these items so their extreme edges stay
exact). After the search the bars are dropped and each half is snapped onto its
fold; the snap (about 0.01 mm) is smaller than the engine's clearance guard.
If pinning cannot be set up or fails, the bounded packer uses the rest of the
time. Warm starts are not mapped into pinned outlines (the bar would cross the
pieces under a half); callers keep their checked incumbent.

**Choosing the allocation by fabric length.** On folded fabric a shorter marker
sometimes cuts a single fold piece as a full two-layer piece (one surplus
layer) beside the halves. Sparrow always keeps halves on the fold, so the
bounded packer, which can mix both, also runs on folded jobs. When the cheap
seed shows that fold halves and wide pieces already fill the fold edge, all the
time goes to the bounded packer. The shortest valid marker wins, ties go to
fewer surplus copies. Surplus copies cost fabric length, so length is the
objective. The round-0.7 WIP had added a surplus-area penalty to the bounded
packer's local scoring. It forced every T-shirt half onto the fold and made the
marker 3.6% longer (7.21 m instead of 6.96 m), so it was removed.

**Browser rounds.** A warm start fixes the two-layer allocation. The browser
session now starts every third round of a folded or tubular job fresh, so a
poor allocation from the first 0.1 s round is not kept for the whole search.

**Fold-edge note.** A folded result gets a `Fold edge:` warning when fold
halves and pieces wider than half the table, which cannot sit side by side,
take at least 85% of the marker length. In that case the order (odd counts per
size) or the lay type sets the length, and more search time will not help.

**DXF pairs.** An omitted `pairs` option now means `true` in Rust, WASM, native
`serve` and the static browser route. An even Quantity of an *asymmetric* piece
is split into left and right copies; a symmetric piece (or an unfolded half
pattern) keeps same-hand copies. `ImportReport.pairs_assumed` is `true` when at
least one piece was split, and one summary warning lists the split and kept
counts. `pairs: false` (CLI `--pairs=false`) keeps every Quantity same-hand.

## Why some folded lays cannot reach the open lay

A folded lay has one fold edge. Every fold half needs it, and two pieces wider
than half the table cannot sit side by side across the width. So their lengths
add up along the marker.

- **T-shirt S–XXL × 1, 736.6 mm folded.** Each size has one front and one back:
  ten body pieces, each about 0.7 m long, so the marker is at least about 6.4 m.
  The open lay at 1473.2 mm puts two bodies across the width and needs 4.05 m.
  No allocation or search time can close this gap. The same garments on a tube
  (two fold edges) or with two of each size per lay (full two-layer cuts) do.
- **Teagan S1 M2 L2 XL1, 750 mm folded.** S and XL bodies are single fold
  halves; M and L are full cuts 579 mm wide. All eight body cuts are wider than
  half the table or on the fold: about 5.6 m minimum against 4.53 m open.
- **Asymmetric pieces with Quantity 1** (the shirt fixture's one-sided back,
  Simon's right and left fronts) give one useless mirrored copy per cut unless
  the fabric may be cut face down (`allow_flip`). This is reported as surplus,
  not hidden.

## Before and after

Before: `df1f0c3` (export7). After: this branch. Both run the same job files
(the shirt job comes from the new importer; its as-imported row uses the old
default, pairs off). "vs open" compares with the open lay at twice the width,
which uses the same fabric area. Means of valid runs; every run was valid.

### Browser session (web app path), 20 s, seeds 1, 2, 3

| Case | Lay | Valid before · after | m/garment before → after | Change | Useful % before → after | Surplus before → after | After vs open |
|---|---|---:|---:|---:|---:|---:|---:|
| Shirt DXF as imported (pairs off) | folded | 3/3 | 3.339 → (see next row) | — | 30.1 | 160 | — |
| Shirt DXF, new import (pairs on) | folded | 3/3 · 3/3 | 2.680 → 1.678 | −37.4% | 37.5 → 59.9 | 80 → 12 | +31.3% |
| Shirt DXF, pairs + flip | folded | 3/3 · 3/3 | 1.752 → 1.401 | −20.1% | 57.4 → 71.8 | 4 → 4 | +9.6% |
| Shirt DXF | open 1473.2 | 3/3 · 3/3 | 1.280 → 1.278 | −0.1% | 78.6 → 78.7 | 0 → 0 | — |
| T-shirt S–XXL × 1 | folded | 3/3 · 3/3 | 1.392 → 1.392 | 0.0% | 45.7 → 45.7 | 9 → 9 | +73.7% |
| T-shirt S–XXL × 1 | tube | 3/3 · 3/3 | 0.960 → 0.901 | −6.2% | 66.5 → 70.7 | 5 → 5 | +12.4% |
| T-shirt S–XXL × 1 | open 1473.2 | 3/3 · 3/3 | 0.801 → 0.802 | 0.0% | 79.5 → 79.5 | 0 → 0 | — |
| Teagan S1 M2 L2 XL1 | folded | 3/3 · 3/3 | 0.968 → 0.970 | +0.1% | 61.8 → 61.7 | 1 → 1 | +23.3% |
| Teagan | tube | 3/3 · 3/3 | 0.927 → 0.872 | −6.0% | 64.6 → 68.6 | 0 → 0 | +10.8% |
| Teagan | open 1500 | 3/3 · 3/3 | 0.787 → 0.787 | −0.1% | 76.0 → 76.0 | 0 → 0 | — |
| Simon (hypothetical) | folded | 3/3 · 3/3 | 2.127 → 1.879 | −11.7% | 51.2 → 57.9 | 30 → 14 | +47.9% |
| Simon | open 1500 | 3/3 · 3/3 | 1.267 → 1.270 | +0.3% | 85.9 → 85.7 | 0 → 0 | — |
| Titan (hypothetical) | folded | 3/3 · 3/3 | 1.235 → 1.234 | −0.1% | 73.8 → 73.9 | 0 → 0 | +1.4% |
| Titan | open 1500 | 3/3 · 3/3 | 1.218 → 1.218 | 0.0% | 74.8 → 74.9 | 0 → 0 | — |

### Native nester (desktop and serve path), 60 s, seed 1

| Case | Lay | m/garment before → after | Change | Useful % before → after | Surplus before → after | After vs open |
|---|---|---:|---:|---:|---:|---:|
| Shirt DXF, pairs on | folded | 2.678 → 1.622 | −39.4% | 37.6 → 62.0 | 80 → 12 | +31.3% |
| Shirt DXF, pairs + flip | folded | 1.752 → 1.378 | −21.4% | 57.4 → 73.0 | 4 → 4 | +11.5% |
| Shirt DXF | open | 1.235 → 1.235 | +0.1% | 81.5 → 81.4 | 0 → 0 | — |
| T-shirt × 1 | folded | 1.366 → 1.366 | 0.0% | 46.6 → 46.6 | 10 → 10 | +68.8% |
| T-shirt × 1 | tube | 0.912 → 0.882 | −3.3% | 69.8 → 72.2 | 5 → 5 | +9.0% |
| T-shirt × 1 | open | 0.808 → 0.809 | +0.1% | 78.8 → 78.7 | 0 → 0 | — |
| Teagan | folded | 0.968 → 0.968 | 0.0% | 61.8 → 61.8 | 1 → 1 | +24.8% |
| Teagan | tube | 0.892 → 0.860 | −3.5% | 67.1 → 69.5 | 0 → 0 | +10.9% |
| Teagan | open | 0.779 → 0.776 | −0.4% | 76.8 → 77.1 | 0 → 0 | — |
| Simon | folded | 2.085 → 1.804 | −13.4% | 52.2 → 60.3 | 30 → 14 | +47.6% |
| Simon | open | 1.223 → 1.223 | 0.0% | 89.0 → 89.0 | 0 → 0 | — |
| Titan | folded | 1.233 → 1.232 | −0.1% | 74.0 → 74.0 | 0 → 0 | +3.0% |
| Titan | open | 1.196 → 1.196 | 0.0% | 76.2 → 76.2 | 0 → 0 | — |

Notes:

- The T-shirt's 5 surplus copies on the tube are the five neck ribs: one per
  size, no fold line, so each cut leaves one spare rib (about 0.02 m² each).
- Before, the shirt kept 80 surplus copies with pairs on: its symmetric sleeves
  and cuffs (Quantity 2, kept same-hand) wasted a layer per cut. The 12 left
  are the 10 one-sided backs and the odd-count pockets in M and L.
- Folded T-shirt and Teagan are unchanged by design. When the fold edge is
  already full, the bounded packer keeps the whole budget, and it mixes fold
  halves with full cuts (that is how 6.96 m beats 7.21 m with every half on the
  fold).
- Single runs on a shared laptop; differences under about 1% are noise.

## Tests

| Check | Result |
|---|---|
| `cargo test --workspace --no-fail-fast` in `marker/` | 460 passed, 0 failed, 9 ignored (export7: 451) |
| `cargo test --workspace` in `pattern/` | 114 passed, 0 failed |
| `cargo clippy --all-targets -D warnings`: core, nest, dxf, CLI, WASM | Pass (with the pre-existing `labels.rs` `chunks_exact_to_as_chunks` lint in beta-marker-export allowed) |
| `npx vitest run` | 436 passed in 40 files |
| `npm run typecheck` (all workspaces) | Pass |
| Remapped WASM builds + `release-build.mjs --verify` | Pass for marker, pattern and floor WASM |
| Export parity (`export-parity.mjs`) | 46/46 files byte-identical; 6/6 DXF import reports identical |
| Static build (`BETA_SKIP_WASM_BUILD=1 npm run build:static -w @beta/marker-web`) | Pass |
| Real WASM nest session: T-shirt tube, T-shirt folded, shirt folded | Valid; result yield fields equal the validator's |

New tests: inferred symmetry (sampling, rounding, off-centre notch, half turn,
nap, rotation rule); legacy two-layer pooling in metrics; useful yield and
surplus in the validator; DXF pairs split only asymmetric pieces and are
explicit; pinned halves nest in Sparrow and sit exactly on the fold; tube pairs
use both folds at one place; too-deep tube halves stay single; the fold-edge
note fires only when fold pieces set the length.

Marker WASM: 4,491,499 bytes raw, 1,307,723 gzip -9 (export7: 4,368,636 /
1,283,856; +2.8% raw, +1.9% gzip, mostly `geo` polygon union for the pinned
outlines). Pattern WASM 5,028,138; floor WASM 3,832,102.

## Reproduction

All inputs are public: `testdata/dxf/shirt-aama-mm.dxf`, the FreeSewing
fixtures in `testdata/open/freesewing`, and BETA's own T-shirt block as
exported by the Style flow (audit7's `round-style-folded.job.json`). Teagan's
front and back get fold lines on their centre line, as in round 0.5.

```text
cargo build --release -p beta-marker-nest --example fold_yield
fold_yield JOB SECONDS SEED browser|native [open|folded|tubular] [WIDTH_MM] [RESULT_OUT]
```

`fold_yield` recomputes useful yield from the layer assignments, independent
of the result's own fields. Before and after binaries are frozen copies of this
example built from `df1f0c3` and from this branch. Runs alternate before/after
per case, so machine load hits both equally. This laptop is shared with other agents, so
finite-budget results carry load noise.
