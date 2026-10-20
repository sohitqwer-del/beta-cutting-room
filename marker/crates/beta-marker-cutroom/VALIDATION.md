# Cutroom validation and handoff

All commands ran from the `marker` workspace, with `CARGO_BUILD_JOBS=2`, a
worktree-specific `CARGO_TARGET_DIR`, and `BETA_DATA_DIR` pointing to the data
root. Only the `beta-marker-cutroom` crate is part of this change. No branch was
merged and no core contract change is requested.

## Final checks

| Command | Result |
| --- | --- |
| `cargo check -p beta-marker-cutroom` | Passed |
| `cargo fmt -p beta-marker-cutroom --check` | Passed |
| `cargo test --workspace` | 86 passed, 0 failed, 2 existing long tests ignored |
| `cargo clippy --workspace --all-targets` | Passed with no warnings after fixing one range-loop lint |
| `cargo run -p beta-marker-cutroom --example calibrate -- crates/beta-marker-cutroom/calibration` | 24 valid nests, 0 failed trials; model and observations saved |
| `cargo run -p beta-marker-cutroom --example demo -- crates/beta-marker-cutroom/output` | 12 bundles, 240 pieces, 1 checked re-cut job; PDFs, ledger and reports generated |

The 17 cutroom unit tests cover shade/size/handedness separation, matching garment
numbers across parts, offsets, partial bundles, invalid input and overflow,
versioned Unicode scan payloads, PDF pagination/escaping, exact mirrored re-cut
demands, atomic scheduling, status transitions, repeated lay ids across orders,
fabric/end-bit reconciliation, area-weighted order efficiency, consumption
features and shrinkage, regression/MAPE math, and a checked short real nest.

Existing ignored tests are `beta-marker-core::validate::tests::validation_speed`
and the nesting integration test `shirt_sixty_seconds`. They were not enabled.
No TypeScript files changed.

## Measured costing error

Each calibration search used 1 second and exactly two workers. Widths were
1400/1600 mm and ratio multipliers were 1/2. All successful markers passed the
independent core validator. Six source patterns/tasks produced 24 observations:
16 for fitting, 8 from two complete held-out sources.

- Training consumption MAPE: **6.6266%**.
- Held-out consumption MAPE: **2.4201%**.
- Held-out GarmentCodeData source: **1.1619%**.
- Held-out Lectra task `37729`: **3.6783%**.
- No failed trials. No Lectra geometry was copied into the crate.

See `calibration/summary.json`, `observations.json`, `model.json`, and its README
for the measurements, split, fixed ridge penalty, source attribution and limits.
These are short-run baselines on a small convenience sample, not a production
accuracy claim. The default Rust model stays explicitly uncalibrated; callers
can deserialize the saved experimental model or fit their own observations.

## PDF and scan QA

Poppler `pdfinfo` confirmed PDF 1.4, A4 pages for the sheet and exact 100 x 50 mm
label pages (283.465 x 141.732 pt): 2 A4 pages, 12 bundle-label pages and 20
piece-sticker pages in the demo. `pdftoppm` rendered both A4 pages at 120 dpi and
the first ticket/sticker at 300 dpi. Visual review found no clipping, overlap or
missing label text. Poppler emitted local display-font configuration messages
for Symbol/ArialUnicode; all used Helvetica/ASCII text rendered correctly.

OpenCV independently decoded the rendered ticket and sticker QR codes. Their
order/lay/bundle, quantity, ply range, printed number and full physical-piece
reference matched `bundles.json`. No new QR-decoder dependency was added to Rust.
Physical printer calibration and shop-floor scanner tests remain integration QA.
Generated PDFs and PNGs are ignored under the crate's `output/` directory.

The demo report reconciles 61.200 m planned, 61.500 m actual, +0.300 m variance,
1.000 m returned end bits, 240/240 pieces and 58.5366% cutting efficiency.

## Dependencies and integration work

New external dependency: `qrcode` 0.14.1, **MIT OR Apache-2.0**, with default
features disabled and no new transitive dependency. Licence checked in the
upstream tagged manifest and downloaded package. `beta-marker-nest` is an
existing Apache-2.0 workspace dependency; existing nesting dependency policy is
unchanged. PDF writing, hull extraction, regression and CSV export add no crate.
The workspace lockfile is intentionally not included; the lead regenerates it.

The README defines the small `Lay`/spreading inputs, QR/WIP schema and proposed
CLI commands. Integration still needs planner/spreading adapters, external
inventory shade enforcement, atomic ledger persistence and CLI wiring. Expand
folded placements into physical layers before bundling/reporting; costing
currently accepts open fabric only. Human-readable PDF text is ASCII, with `?`
for unsupported characters; QR data retains UTF-8. xlsx export is optional and
was not added.

After merge, re-test re-cut jobs and accurate costing against the merged v0.2
nesting/validator rules (shrinkage, fold expansion, defects, matching, shade bands,
fusing blocks and garment assignments). Re-run calibration because engine changes
change its labels. Verify adapter handling of normal/mirror counts, per-size
garment ids, physical plies and returned end-bit accounting. No contract change
is required; inventory shade identity is carried explicitly and in re-cut metadata.
