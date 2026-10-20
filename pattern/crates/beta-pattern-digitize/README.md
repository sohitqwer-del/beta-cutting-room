# BETA paper-pattern digitizer

Offline, pure Rust photo/scan input and millimetre `beta_marker_core::Piece`
output. No camera SDK, OpenCV, ArUco, AprilTag, remote service, or runtime network
is used. The code is Apache-2.0. This is an initial implementation; review the
debug overlay before using recovered geometry for cutting.

## Factory setup

1. Generate a target and keep its `.target.json` manifest with it.
2. Print landscape at **100% / actual size**. Disable “fit to page”, borderless
   enlargement, and printer scaling. A3 is 420 × 297 mm; A2 is 594 × 420 mm.
3. Use a ruler to check **ten checker cells = 50 mm**, in both directions, at
   several positions. The outside of every coded square is **24 × 24 mm**.
   A wrong print scale produces the same wrong pattern scale; the camera cannot
   discover it. Reject a print if either 50 mm check differs by more than 0.2 mm.
4. Tape the sheets flat to a rigid surface. Adjacent tiles overlap by 10 mm.
   Trim 5 mm from each **shared** edge and butt those cut edges. The PDF footer
   gives the tile column/row and its original board origin. Do not change the
   spacing to make a marker look square. Check a ruler across every join.
5. Use the printed grey field, or a uniform **dark matte backing**. White paper
   on a white table is not supported. In table mode, cut away the blank centre
   of perimeter pages and retain the outside 36 mm of the assembled board.
   Lay those strips around a dark cutting mat at their manifest positions.
   Do not move individual tags without making a matching manifest.
6. Flatten the paper/card. Separate pieces by at least 3 mm. Keep all pieces
   inside an **empty 36 mm border** on each side. Do not cover the coded squares.
   Draw a clear, long grain line (the fabric direction) and small edge ticks
   where needed. A drill mark is a small circular dot inside the piece.
7. Light evenly. Avoid glossy reflections and hard shadows. Use the normal or
   tele camera lens, rather than an ultra-wide lens. Keep the board near the
   image centre and photograph the entire board, as nearly overhead as possible.
   Four well-spread codes are the minimum; keep eight or more visible for
   optional lens correction. A3 with a sharp 12 MP photo is the tested setup.
8. Scan, inspect the overlay, correct uncertain grain/marks, then export a job.
   Green is the outline, blue the grain, orange notches, and red drill candidates.

## Commands

Run from the `pattern` workspace. The shared-machine build environment must set
`CARGO_TARGET_DIR` to the assigned data-volume directory and `CARGO_BUILD_JOBS=2`.
The examples use a caller-supplied `BETA_DATA_DIR`; no data directory is embedded
in the software. Do not save large test images in the repository.

```sh
cargo run -p beta-pattern-digitize --bin beta-digitize -- target --size a3 -o target.pdf
cargo run -p beta-pattern-digitize --bin beta-digitize -- target --size a2 --tiles 2x2 -o tiled.pdf
cargo run -p beta-pattern-digitize --bin beta-digitize -- target --size a3 --tiles 4x6 --table -o table.pdf
cargo run -p beta-pattern-digitize --bin beta-digitize -- target --size a3 -o target.svg

cargo run --release -p beta-pattern-digitize --bin beta-digitize -- scan photo.jpg \
  --target target.target.json --radial --debug overlay.png --report scan.json -o pieces.json
cargo run --release -p beta-pattern-digitize --bin beta-digitize -- scan photo.png \
  --job --width 1500 --grain 130,75,130,210 -o job.json
```

`scan` defaults to one landscape A3 sheet. Pass the manifest for A2, tiled, or
table targets. `--grain` is an explicit pair of **board mm, y down**, used only
for the piece that contains both endpoints. Output mm use **y up**, with origin
at the lower-left of the board; pieces retain their common board coordinates.
Grain is undirected when detected automatically (PCA cannot infer arrowheads).
Missing grain leaves `None`, which the Marker contract interprets as the x axis;
the scan report always warns. Quantity is one; size, material, and pairing are
not inferred. A Piece list contains no report; use `--report` or `--job` to retain it.

Other controls: `--mm-per-pixel 0.5`, `--tolerance 0.25` (Douglas-Peucker chord
tolerance, mm), `--min-area 100` (mm²), `--threshold 170` (normalized brightness),
`--rectified metric.png`, and `--no-smooth`. Supported input formats are PNG/JPEG.
Exif auto-rotation is unnecessary for code orientation; mirrored images are not
supported. Image limits are 64 MP input and 24 MP rectified output. Decoder
allocations are limited to 256 MiB. Total process memory can exceed that amount.

## Original target and calibration

Our own code has 8 × 8 cells: a black outer ring; an inner white ring with its
top-left cell black for orientation; a central 4 × 4 payload. Payload bits are
row-major, least significant first: eight ID bits and eight CRC bits. The CRC
polynomial is 0x1d with initial state 0xa7. The complete expected grid must match;
we do not repair damaged bits. Codes are unique within a manifest, with at most
256 codes. Duplicate detected IDs are discarded as ambiguous.

Detection uses adaptive dark thresholding, connected components, convex hulls,
four-sided reduction, and total-least-squares edge refinement. It samples the
grid through a projective map and checks its orientation and checksum.
Detection is capped at a 2400-pixel long edge; coordinates are returned in
original photo pixels. Small/distant or partially covered codes can be missed.

Calibration uses normalized DLT (a fixed final coefficient and pivoted normal
equations), deterministic 512-sample RANSAC, and an inlier refit. It requires
four distinct known tags, broad board coverage, at least 75% consistent corners,
and reprojection RMS below 3 pixels. Normal equations are adequate for these
normalized, well-spread points; this is not an SVD implementation. Optional
radial fitting searches one coefficient about the image centre, using eight or
more codes, then runs RANSAC. Tangential distortion, camera-centre estimation,
and full camera calibration are not included. The radial fit assumes most
decoded markers are correctly printed and coplanar; it is not an adversarial
robust estimator for a table assembled at wrong marker positions.

Metric resampling uses bilinear interpolation at 0.5 mm/pixel by default. Empty
left/right grey border strips estimate a row-wise illumination field. Otsu
thresholding separates light paper from it. A small morphological close bridges
thin ink marks. Marching squares interpolates grey crossings for subpixel edge
positions. One bounded local quadratic smoothing step is applied only at shallow
turns; Douglas-Peucker then reduces the polygon. The Piece contract stores a
polygon, so no Bezier curves are exported or claimed.

Small re-entrant V cuts and dark edge ticks/slits produce notch candidates.
Compact dark components produce drill candidates. A long, thin interior dark
component supplies a grain candidate through its principal axis. These are
geometric heuristics, not handwriting recognition. Keep text away from edges
and use a manual grain override when lettering joins the line.

## Accuracy and reproducibility

See [ACCURACY.md](ACCURACY.md) for the measured cases and limits. To regenerate:

```sh
cargo run --release -p beta-pattern-digitize --bin beta-digitize -- benchmark \
  --input ../marker/testdata/open/freesewing/bella-M.job.json \
  --output-dir "$BETA_DATA_DIR/datasets/digitize/bella"
```

This renders up to the first three source outlines, scaled to fit A3. It adds
one real V cut, one drawn edge tick, one drill dot, and one drawn grain line.
It generates 4000 × 3000 JPEG photos, scans those decoded files, and saves
ground truth, recovered pieces, overlays, and `accuracy.json`. `--quick` uses
1800 × 1350 for iteration. Rendering, transforms, blur, lighting, and seeded noise
are all Rust. Distances sample both true and recovered outlines every 0.5 mm;
no rigid alignment or rescaling is applied to conceal calibration errors.
The sampled maximum is not a formal continuous Hausdorff bound. Notches match
one-to-one within 2 mm, and false positives are reported separately.

FreeSewing source outlines are MIT, copyright Joost De Cock and FreeSewing
contributors. See `marker/testdata/open/freesewing/README.md` and `LICENSE.MIT`.
We use these existing files without copying a new dataset into this crate.

## Limits

Pieces bigger than the calibrated working area need a larger assembled target;
stitching multiple photos is not implemented. Contact with the usable border
or camera edge is rejected. Folds, wrinkles, curled card, and table height
changes break the planar model and can bias geometry without obvious camera
reprojection error. Flatten them; lens fitting cannot fix them.

White-on-white, translucent paper, touching/overlapping pieces, very dark card,
hard shadows, glare, or background texture can break segmentation. The border
normalizer handles smooth illumination gradients, not arbitrary interior
shadows. Closing can erase cuts narrower than roughly 1.5 mm; genuine thin
slits may appear as tick candidates rather than geometry. Small text dots can
look like drills, and short concave corners can look like V cuts. Internal
cutout holes are not represented by `Piece.outline`; drill dots are marks.

Synthetic accuracy is an engineering regression, not proof of real-camera
accuracy. No physical printer/camera validation has been performed. A2, large
tiled tables, multiple pieces, and strongly skewed photos need factory trials.
Review units, grain, notch direction, and piece topology before production.

## Dependencies and integration

See [DEPENDENCIES.md](DEPENDENCIES.md) for versions, checked licences, and sources.
The only new direct dependency is `image = 0.25.8` with only `jpeg` and `png`.
No workspace manifest or Piece contract change is required. The lead should
regenerate the workspace lockfile after merging this crate. Tests and lint:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets
```


## Quick Marker: unprinted A4 and phone memory limits

The `quick` module adds an offline JPEG/PNG entry point, `scan_bytes`, used by the
Marker and Pattern WASM crates. It detects a separate plain white A4 sheet on
dark matte backing, fits four subpixel corners, and maps outlines to mm. An
optional second sheet checks consistency away from the first. The coded BETA
target remains available in the same API as the accurate mode.

Quick mode accepts at most 12 MP and 20 MB encoded, then reduces to a 2000-pixel
long edge before vision. Coarse background normalization handles slow lighting
changes; contour extraction keeps subpixel grey crossings. Background texture
and contour-complexity caps reject scenes that would threaten phone memory.
Blur, shadow, glare, tilt, clipping, low contrast and possible touching pieces
produce review warnings. These are heuristics; a missing warning is not proof
of a good photograph. Brown paper is supported only with enough floor contrast.

A4 calibration cannot correct unknown lens distortion or non-flat paper. Its
error estimate excludes these effects, and the synthetic lens case reaches
13.652 mm maximum error. Read the new tables in `ACCURACY.md`. No real phone
accuracy is claimed. Keep separate pieces at least 10 mm apart and use a normal
camera lens. Select the sheet corners if an A4-shaped pattern causes ambiguity.

The shape model, geometric suggestions, multi-photo merge and outputs are
described in `marker/docs/QUICK-MARKER-API.md` (from repository root). Suggestions
never change cut quantities, grain, folds or delete duplicates automatically.
Every ordered size needs real patterns; Quick Marker does not grade them.
See `model/MODEL.md` for the weak held-out name classification result and
`REAL-PHOTO-TEST.md` for a practical physical validation protocol.
