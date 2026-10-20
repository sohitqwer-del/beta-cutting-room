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
