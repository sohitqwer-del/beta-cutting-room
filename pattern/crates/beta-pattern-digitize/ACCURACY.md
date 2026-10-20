# Synthetic accuracy report

Measured 19 October 2026 on the shared Windows x86-64 development machine,
Rust `1.98.1 (48a229cea 2026-09-01)`. The digitizer package was compiled with
optimization level 3 in the dev profile. No physical camera or printer was
used. These results establish a reproducible synthetic regression, not a
factory accuracy guarantee.

## Results

Every input is a **4000 × 3000 JPEG (12 MP)**. The target is landscape A3,
420 × 297 mm, with 14 original BETA square codes. Rectification uses
0.5 mm/pixel, followed by 0.25 mm polygon simplification. All six scans found
14 codes, retaining all 56 corner correspondences.

| FreeSewing outline | Photo | Mean error, mm | Maximum error, mm | Area error, % | Notches | False notches | Processing, s |
|---|---|---:|---:|---:|---:|---:|---:|
| Bella back | Scan-like | 0.064 | 0.838 | 0.017 | 2/2 | 0 | 2.387 |
| Bella back | Phone-like | 0.059 | 0.655 | 0.017 | 2/2 | 0 | 1.434 |
| Bella front side dart | Scan-like | 0.049 | 0.832 | 0.009 | 2/2 | 0 | 2.608 |
| Bella front side dart | Phone-like | 0.045 | 0.860 | 0.012 | 2/2 | 0 | 2.242 |
| Bella sleeve | Scan-like | 0.090 | 1.125 | 0.201 | 2/2 | 0 | 1.751 |
| Bella sleeve | Phone-like | 0.074 | 1.079 | 0.118 | 2/2 | 0 | 2.532 |

All six meet **mean < 1 mm and sampled maximum < 2 mm**. The inserted drill
dot was found in all six. Grain angle error was at most 0.0061 degrees.
The notch result is 12/12 (100%) with zero false positives **on this small
synthetic set**. It does not measure handwritten annotations or worn paper.

Processing time includes marker detection, lens fitting when enabled,
rectification, segmentation, contours, mark recognition, and overlay creation.
It excludes file decoding/writing and synthetic-photo generation. Times vary
with other work on the shared machine. The algorithm is single-threaded;
`CARGO_BUILD_JOBS=2` limits build work, not scan execution.

## Photo construction and scoring

The source is the first three pieces in
`marker/testdata/open/freesewing/bella-M.job.json`. The outlines are MIT,
copyright Joost De Cock and FreeSewing contributors. Each is scaled to fit
the A3 working area: back **0.475154×**, front **0.440748×**, sleeve **0.322951×**.
Their tested heights are 197 mm. Full-size garment pieces generally require
a larger tiled target or table border; those have not had an accuracy trial.

Each source receives an independently specified 3 mm-deep, 4 mm-wide physical
V cut on a long edge, a 4 mm drawn tick, a 2.6 mm drill dot, and a 0.65 mm-wide
grain line. The paper is rendered at 8 pixels/mm on the original target.
The camera image is inverse-warped from that scene, illuminated, given seeded
noise, blurred, saved as JPEG, and decoded by the actual scan entry point.

- **Scan-like:** frontal quadrilateral; Gaussian blur sigma 0.4 photo pixels;
  uniform noise ±1 intensity level; smooth illumination loss parameter 0.08;
  zero radial distortion; lens fitting disabled.
- **Phone-like:** oblique quadrilateral at normalized corners
  `(0.080,0.130), (0.915,0.060), (0.955,0.880), (0.105,0.940)`;
  blur sigma 0.9 pixels; noise ±4 levels; illumination loss parameter 0.28;
  radial `k1=0.035`; lens fitting enabled. It recovered `k1=0.035`.
- Noise seed is 42. Lighting combines a spatial gradient and a low-frequency
  sinusoidal variation. This is uneven light, not a simulated sharp fold shadow.
- Mean and maximum distances sample both polygons at no more than 0.5 mm
  along each edge, measuring nearest-segment distance to the other polygon.
  No alignment or scale correction is applied after recovery.
- The maximum is a sampled symmetric contour distance, not a proven continuous
  Hausdorff bound. Area error is absolute relative error. Notches match
  one-to-one within 2 mm of their true positions, with extra candidates counted
  separately. Drill centres use the same 2 mm matching radius.

Calibration reprojection RMS was 0.816 pixels for scan-like cases and 0.699
pixels for phone-like cases. A low calibration RMS alone cannot detect a wrong
print scale, non-flat paper, or an incorrectly spaced target assembly.

## Reproduce

From `pattern/`, with `BETA_DATA_DIR` set to the caller's external data directory:

```powershell
$env:CARGO_TARGET_DIR="$env:BETA_DATA_DIR/cargo-target/digitize"
$env:CARGO_BUILD_JOBS='2'
cargo test --workspace
cargo clippy --workspace --all-targets
cargo build --package beta-pattern-digitize --bin beta-digitize --config 'profile.dev.package.beta-pattern-digitize.opt-level=3'
& "$env:CARGO_TARGET_DIR/debug/beta-digitize.exe" benchmark --input '../marker/testdata/open/freesewing/bella-M.job.json' --output-dir "$env:BETA_DATA_DIR/datasets/digitize/bella"
```

The data directory contains six JPEG photos, six overlays, true/recovered JSON
geometry, the printable target and its manifest, and complete `accuracy.json`
measurements. Large assets are deliberately excluded from git. The benchmark
returns a nonzero status if any outline misses the 1 mm mean / 2 mm maximum goal.
Notch/drill/grain measurements are reported, rather than hidden behind that gate.

## Validation and remaining work

- `cargo test --workspace`: **9 tests passed**; binary and doc-test targets pass.
  Tests cover RANSAC outliers, degenerate calibration, radial inversion/fitting,
  rotated codes, payload corruption, duplicate IDs, invalid options, contour
  separation, multiple pieces, manual grain, border rejection, and a degraded
  synthetic photo with scored outline, drill, notch, and grain recovery.
- `cargo clippy --workspace --all-targets`: **passed with no warnings**.
- CLI smoke checks: A3 PDF/SVG; 2 × 2 A2 tiled PDF; 4 × 6 A3 table-border PDF;
  JPEG scan with overlay, metric image, report, manual grain and a 1500 mm job.
- Poppler identified A3 pages as 1190.55 × 841.89 points and A2 pages as
  1683.78 × 1190.55 points. The A2 target has four pages; the table target emits
  only its sixteen perimeter pages. First-page renders were visually checked
  for scale layout, code visibility, checker border, and printable labels.

Next validation should use ruler-measured physical paper on a checked printer,
several phone models, actual drill holes and slit cuts, shadows, text near
edges, mixed paper/card colours, and multiple pieces on large assembled boards.
Folds and wrinkles violate the planar model. White-on-white and overlapping
pieces are unsupported. Very narrow cuts can be closed by the ink-bridging
step. Text can still look like marks; these six clean examples do not establish
production mark-recognition precision. See [README.md](README.md) for operator
instructions and the complete limitations.

No shared contract changes are needed. The lead must regenerate the workspace
lockfile after merging; it is deliberately not included in this commit.


# Quick Marker phone path: 20 October 2026

These are **synthetic** measurements from the new bounded Quick Marker API,
not real phone photographs. The older table above is the original target-only
path and remains a separate baseline. Inputs below are 4000 x 3000 JPEG (12 MP),
decoded by `quick::scan_bytes`, then reduced to a 2000 pixel long edge.
Processing includes decode, segmentation, assist and PNG overlay encoding.
It does not include TypeScript JSON transfer or photo rendering.

Same three MIT FreeSewing Bella outlines as above. A4 scenes place the actual
210 x 297 mm reference on an 800 x 600 mm dark textured floor beside one piece.
The piece is scaled to fit 330 x 420 mm. Its left edge is 110 mm from the sheet's
right edge; far edges extend several hundred mm beyond it. Target scenes still
scale each piece to the A3 working area, about 197 mm high. **The two modes do
not test equal physical piece sizes**; do not read this as a controlled ranking
of their absolute performance on full garment pieces.

Coordinates are compared directly against independent renderer truth, using
bidirectional samples every 0.5 mm. No translation, rotation or rescaling is
fitted to the recovered outline. All 18 scans found one piece and the expected
reference. Raw measurements, including warnings, are in `QUICK-ACCURACY.json`.

## A4 mode

| Piece | Photo case | Mean mm | Sampled max mm | Area error % | Native seconds |
|---|---|---:|---:|---:|---:|
| back | frontal | 0.096 | 0.317 | 0.036 | 0.826 |
| back | perspective-shadow-texture | 0.120 | 0.564 | 0.047 | 0.915 |
| back | blur-brown | 0.643 | 1.631 | 0.606 | 0.909 |
| back | radial-lens | 3.725 | 11.163 | 5.135 | 0.766 |
| frontSideDart | frontal | 0.070 | 0.363 | 0.061 | 0.875 |
| frontSideDart | perspective-shadow-texture | 0.108 | 0.537 | 0.105 | 1.003 |
| frontSideDart | blur-brown | 0.537 | 1.672 | 0.831 | 1.160 |
| frontSideDart | radial-lens | 3.707 | 13.652 | 6.538 | 1.114 |
| sleeve | frontal | 0.084 | 0.397 | 0.038 | 1.118 |
| sleeve | perspective-shadow-texture | 0.107 | 0.388 | 0.048 | 1.061 |
| sleeve | blur-brown | 0.629 | 1.521 | 0.777 | 1.111 |
| sleeve | radial-lens | 3.225 | 8.509 | 4.887 | 1.065 |

The nine cases without lens distortion meet the proposed mean < 1 mm / sampled
max < 2 mm engineering target. The three radial-distortion cases **fail** it.
One plain sheet supplies no lens calibration; errors reach **13.652 mm** here.
The returned `expectedErrorMm` models corner sampling, distance from the sheet
and perspective only. These lens errors exceed it. It is not a statistical
confidence bound and cannot account for curled paper, wrinkles or height offsets.
Use the BETA target and physical checks when accuracy matters.

- Frontal: blur sigma 0.5 photo pixels, smooth lighting loss 0.08, noise +/-1.
- Perspective/shadow/texture: quadrilateral corners `(0.05,0.10), (0.93,0.04),
  (0.97,0.92), (0.08,0.96)` of the photo; sigma 1.2; lighting loss 0.40;
  seeded noise +/-5. Background has fine crossed sinusoidal cloth texture.
  A smooth localized shadow crosses the piece; this is not every kind of hard shadow.
- Blur/brown: same perspective; paper intensity 185 rather than 231; sigma 3.0;
  lighting loss 0.30; noise +/-4.
- Radial lens: same perspective; k1=0.035; sigma 0.9; lighting loss 0.28;
  noise +/-4. This mode deliberately does not fit a radial coefficient.
- World renderer uses 4 pixels/mm and inverse projective sampling. Noise seed 42.
- The A4 shape itself is 242 grey on a nominal 38-grey floor. This controlled
  contrast does not establish performance on worn cards, glossy paper or busy floors.

## BETA target through Quick Marker

| Piece | Photo case | Mean mm | Sampled max mm | Area error % | Native seconds |
|---|---|---:|---:|---:|---:|
| back | frontal | 0.045 | 0.655 | 0.032 | 0.638 |
| back | phone | 0.050 | 0.655 | 0.050 | 0.783 |
| frontSideDart | frontal | 0.065 | 0.865 | 0.007 | 0.913 |
| frontSideDart | phone | 0.061 | 0.856 | 0.071 | 0.953 |
| sleeve | frontal | 0.048 | 1.025 | 0.034 | 0.964 |
| sleeve | phone | 0.049 | 1.094 | 0.065 | 1.005 |

All six target cases meet the same engineering target. The phone cases use
k1=0.035 and fit it using the BETA tags. These use the original target renderer
(8 pixels/mm, dark marks and printed codes), with the PhotoOptions defaults
except the stated front/phone perspective and radial setting. Detection sees the
downsampled photo. There is still no physical-camera or printer validation.

## Assist and reproduction

The shape classifier scores **17/78 (21.79%)** with whole styles held out; this
is below the global majority-class baseline. All name suggestions are low
confidence. See `model/MODEL.md` and `model/evaluation.json`, including per-class
counts, duplicate exclusions and the important limits of this very small sample.
Symmetry, mirror pairing, duplicate checks and grain fallback are geometry
heuristics with unit tests; they have no measured real-photo precision yet.

From the repository root, with the assigned build directory and three build jobs:

```sh
cargo run --manifest-path pattern/Cargo.toml -p beta-pattern-digitize \
  --example quick_benchmark --config 'profile.dev.package.beta-pattern-digitize.opt-level=3' \
  -- marker/testdata/open/freesewing/bella-M.job.json \
  pattern/crates/beta-pattern-digitize/tmp/full
```

`--quick` selects 1600 x 1200 input for iteration; it is not the table above.
Large generated JPEG/PNG files are ignored under the crate's `tmp/` directory.
Runtime varies on the shared i9-11900H laptop. Native scans above take
0.638-1.160 seconds. Browser/WASM timings and memory are reported in
`QUICK-MARKER-API.md` after the headless WASM check. Phone timing is only an
estimate until an actual phone run is made.

Follow `REAL-PHOTO-TEST.md` for the ten-photo physical test. A4-shaped pattern
pieces remain ambiguous; inspect the highlighted reference or select its corners.
Two sheets provide a consistency check, not a full two-reference lens calibration.
Touching/overlapping pieces and arbitrary interior shadows can still merge or
split outlines; the engine warns heuristically and never claims automatic repair.


The final RGB WASM run took 4.00 s cold and 3.41-3.61 s warm. Its linear memory
high-water mark was 140.1 MiB; total Node peak was 288.4 MB. See
`WASM-PERFORMANCE.json`. This is one synthetic RGB JPEG, not an entire phone
browser memory guarantee. A mid-range phone estimate is 10-25 s for a comparable
photo, pending physical testing. The geometry benchmark above used grayscale JPEG
encoding; the separate RGB fixture was used to test decoder memory realistically.

The new Quick API leaves grain unaccepted even when a drawn line is detected.
Consequently the raw contour benchmark's `grain_error_deg` can be null; that is
not a measured grain-recognition failure. The drawn-line regression checks its
candidate and visible blue overlay, while the old target-only benchmark above
still reports accepted native grain-angle measurements. No real-photo grain,
notch or drill precision is claimed for Quick mode.
