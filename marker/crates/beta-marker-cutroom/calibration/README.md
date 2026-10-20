# Consumption model calibration

This is an exploratory fit on BETA Marker's own short real nests. It is not a
factory accuracy claim or a comparison with commercial CAD. The training labels
are feasible short-run results, not proven best marker lengths.

The reproducible Rust runner is `examples/calibrate.rs`. Run it from the marker
workspace root, with `CARGO_BUILD_JOBS=2`, the worktree's `CARGO_TARGET_DIR`, and
`BETA_DATA_DIR` for optional Lectra tasks:

```sh
cargo run -p beta-marker-cutroom --example calibrate -- crates/beta-marker-cutroom/calibration
```

It uses widths 1400 and 1600 mm, ratio multipliers 1 and 2, one second of search
per nest, exactly two workers, and seeds starting at 7100 (plus the number of
prior successful observations). Construction and validation can exceed the search
budget. The independent validator checks every successful result. Failed trials
are retained with their error, rather than silently counted as successful.

Training patterns: FreeSewing Teagan M and Titan M; GarmentCodeData
`rand_07UUJUMCB1`; and the first eligible Lectra sample task with at most 25 pieces.
Held-out patterns: GarmentCodeData `rand_0B1IDD2JL0` and the second eligible Lectra
task. Both widths and both ratios of each held-out pattern remain outside fitting.
The first two comparable tasks are read only from the local
`BETA_DATA_DIR/marker-bench/lectra/sample-300.jsonl`, if present.

All source quantities, grain rules, handedness and size ratios are retained;
ratios multiply the complete original set. A Lectra task without an order ratio
is treated as one complete set (its real garment count is not established).
This convention does not change its relative consumption error. Lectra strip
eligibility and orientation-conversion caveats from the local conversion audit
still apply. No diagnostic or vendor source files are used.

`observations.json` contains the five features, validated efficiency, real marker
length, width and garment count, plus failed trials. It contains no geometry.
`model.json` contains the fitted coefficients. `summary.json` records sample
counts, train/holdout fabric-consumption MAPE and per-source holdout error. Ridge
penalty is fixed at 0.01; it is not tuned on the holdout. Prediction is clamped to
20-95%. MAPE is `mean(abs(actual_efficiency / predicted_efficiency - 1)) * 100`,
which is relative marker-length error for equal area and width.

## Data attribution and limits

Measured on this branch: **24 valid real nests, zero failed trials**, with 16
training observations and 8 observations from two held-out sources. Training
consumption MAPE is **6.63%**; held-out consumption MAPE is **2.42%**. Held-out
GarmentCodeData error is **1.16%** and held-out Lectra task `37729` error is
**3.68%**. Training uses Lectra task `53060`. These small-sample figures must not
be described as general production accuracy. In particular, a lower holdout
error than training error does not make the small holdout representative.

- **FreeSewing**, Joost De Cock and contributors, package version 4.10.2, MIT.
  [FreeSewing code](https://codeberg.org/freesewing/freesewing). These are the
  repository's existing Teagan/Titan jobs; no new source geometry was copied.
- **GarmentCodeData**, Maria Korosteleva, Timur Levent Kesdogan, Fabian Kemper,
  Stephan Wenninger, Jasmin Koller, Yuhan Zhang, Mario Botsch and Olga
  Sorkine-Hornung, ECCV 2024, CC BY 4.0.
  [Dataset record](https://doi.org/10.3929/ethz-b-000690432).
  The repository's shifted AIpparel mirror examples retain their documented
  upstream provenance limitation. See `testdata/open/garmentcode/README.md` and
  `testdata/open/SOURCES.md` for the full attribution, mirror pin and caveats.
- **Nesting Tasks Dataset for 2D-Nesting Efficiency Estimation**, Lallier, Vezard,
  Pinaud and Blin (2022), version 1.1, CC BY 4.0.
  [Zenodo record](https://doi.org/10.5281/zenodo.7030786).
  Only the existing local converted tasks are read. Original geometry is not
  committed. Reported derived observations retain this attribution.

This small convenience sample does not cover factories, fabrics, garment types,
widths, ratios, shrinkage, matching rules or roll conditions in general. Some
features may vary very little in it, making coefficients poorly identified even
with regularization. Wall-clock search and worker scheduling make results vary
between runs with the same seed. Record new measured error after a nesting-engine
merge; do not assume these coefficients remain accurate. For quotations, gather
representative local observations and hold out complete styles/orders.
