# Pattern and plotted-marker import

## Compare a factory marker (HPGL / PLT)

```sh
beta-marker import their-marker.plt --format plt --width 1500 -o job.json --result-out their-marker.json
beta-marker validate job.json their-marker.json
beta-marker nest job.json --time 120 -o beta-marker.json
beta-marker validate job.json beta-marker.json
```

Compare the two validation reports. Both recompute length, area and efficiency
with BETA's formula: sum of cut-outline areas / (width * maximum cut x).
Use identical fabric, spacing, quantities and materials. Import preserves the
plot and does not repair overlaps. Positive plot margins remain in consumption;
negative coordinates remain visible to validation. Review warnings before cutting.

`import_hpgl(bytes, &HpglOptions)` returns a reusable `Job` and a `MarkerResult`
with engine `imported`. Piece translations are removed and stored in placements.
Grain cannot be inferred reliably: each distinct plotted orientation is assumed
grain-aligned along +x, with rotation `flip180`. Mirrored and half-turned duplicate
outlines share normal/mirrored quantities. Quarter-turned shapes stay separate
because their inferred grain differs. Matching ignores winding, starting vertex
and straight-edge subdivision. It checks 0.05 mm tolerance, then shares geometry
only for lossless matches. Near matches stay separate to preserve the plotted
cut lines exactly; the core has no per-copy outline override. Grouped copies use
the first copy's marks and labels. Check per-copy differences in the plot.

Default scale is **40 plotter units/mm**; override with `--plu-per-mm N`.
Plot x must follow marker length and y must run across fabric width. Export
rotated-feed plots in these axes before import; the reader does not swap axes.
`--format` accepts auto/dxf/plt/hpgl; auto uses the extension. `--width` wins.
Otherwise an enclosing rectangular frame supplies width, else maximum cut-line
y supplies it with a warning. A frame must contain two substantial loops, or
one loop and a matching explicit width. This heuristic can confuse a rectangular
garment containing cutouts. The plotted origin is retained. An offset frame uses
maximum y, with a warning. Footer labels do not affect length or inferred width.

Supported commands: IN, DF, SP, PU, PD, PA, PR, LB, DT, SI, CI, AA/AR and EA/ER.
Arcs use chord sag below 0.01 mm. Same-pen endpoints join in either direction
within 0.05 mm. Closed outer loops become cut pieces. Exact repeated coordinates
recover closed cycles when an attached notch leaves an open tail in a joined
pen path. Contained loops become
internal marks; rings up to 6 mm diameter become drills. Open paths up to 10 mm
touching an edge become notches. Other paths inside/near pieces become internal
lines. All pens except pen 0 contribute. Open paths outside pieces warn; long
open boundaries are never closed by inventing a straight cut.

LB uses ETX or the DT delimiter. Text inside a piece supplies its name;
`Piece Name: ...` / `Size: ...` and `name | size | ...` labels are recognized.
Assigned labels survive as text marks. Text appearance commands are accepted
without generating glyph strokes. UTF-8 labels survive; invalid bytes use
replacement characters. LF/CRLF, missing command semicolons and adjacent signed
coordinates work. LB needs its delimiter; PE needs a semicolon.
Geometry after LB must use an absolute pen-up reposition, because label advance
depends on the plotter font. Relative movement from an unknown label end errors.

PE supports base 64, base 32 (`7`), relative coordinates, next-pair absolute (`=`),
pen-up (`<`), pen selection (`:`), and fractional bits (`>`), with bounded values.
Written independently using the public
[HP PCL 5 Technical Reference, Part II, vector group](https://www.hp.com/ctg/Manual/bpl13211.pdf).
Coordinate transforms SC/IP/IR/RO, clipping, polygon mode, filled shapes, Beziers
and page changes return errors. Remove PCL/printer wrappers before importing.
Unknown commands warn. Limits: 64 MiB, two million points, 100,000 paths/labels.

## Experimental grading

```sh
beta-marker import base.dxf --width 1500 --rules sizes.rul -o graded-job.json
```

`GradeTable::parse(bytes)` and `import_dxf_with_rules(bytes, options, table)` use
a documented ASCII subset inspired by public AAMA/ASTM exchange concepts.
This is **experimental**, not full standard or vendor-dialect compliance. No
paywalled standard text, vendor rule files or vendor code were copied.

Our original example (see the generated fixtures):

```text
SIZES: S M L
BASE: M
UNITS: MM
MODE: CUMULATIVE
RULE: 1 2,1 0,0 -2,-1
RULE: 2 -2,1 0,0 2,-1
RULE: 3 -2,-1 0,0 2,1
RULE: 4 2,-1 0,0 -2,1
```

Each rule has an ID and one x,y pair per size, in declared order. Commas, spaces
and tabs separate fields. Rules can continue onto following lines. `//` and
leading `#` lines are comments. Aliases: SIZE LIST, SAMPLE SIZE, BASE SIZE, and
`RULE: DELTA id ...`. Offset units are MM/METRIC (default), CM or INCH/ENGLISH,
independent of DXF drawing units.

CUMULATIVE offsets are from BASE; the base entry must be zero. INCREMENTAL
entry i is the step from size i-1 to size i; entry 0 is unused. Steps accumulate
in size order and rebase at BASE. Missing MODE uses DELTA or zero base entries
as evidence for cumulative, otherwise incremental is assumed. Either inference
warns. Zero base increments are ambiguous; specify MODE explicitly.

Anchor labels are `#id`, `RULE: id`, `GRADE: id`, `GRADE RULE: id` or `RULE ID: id`
in TEXT/MTEXT or ATTRIB/ATTDEF. Contour anchors lie within 0.1 mm of the cut
boundary or a closed internal line. Turn-point text
can carry a rule at the same position; bare numbers are not rules. Anchors
project onto the boundary and insert vertices where needed. Unnumbered points
interpolate displacement by **distance along the outline** between numbered
neighbours, including the closing segment. One anchor translates the piece;
explicit zero-growth rules keep anchors fixed.

One piece per size retains quantities and material. Cut and internal contours
grade independently. Internal reference points follow their exact rules; other
marks and match points follow the nearest graded contour. This is an approximation
for unnumbered internal points. Unnumbered grain lines retain their direction.
Bad tables, unknown IDs, missing/conflicting contour anchors,
non-base sizes, collapsed outlines and self-intersecting outlines error. Only garment BLOCKs are graded.
Layer-6 half patterns must be supplied as full base outlines. Rule axes must
match expanded drawing axes; rotated/scaled INSERTs return an error when grading.
Verify fit, measurements and self-intersection before cutting. A plausible area
does not prove a correct grade. Limits: 8 MiB, 100 sizes, 100,000 rules.

## Materials, folds and matching references

Material text takes precedence, then Category text. The CLI infers missing
materials from names: interlining/interfacing/fusing/fusible -> `interlining`,
lining -> `lining`, pocket bag -> `pocket bag`, reflective -> `reflective`,
otherwise `shell`. The Open+Pattern fixture's `pocker bag` and `refective`
misspellings are included. Inferred classifications warn. Names such as "Under
Reflective Panel" do not establish the fabric reliably; check the bill of
materials. The library exposes `classify_materials` as an explicit step.
Nest separate materials as separate jobs; the combined import is for inspection.

```sh
beta-marker import pattern.dxf --width 1500 --material lining -o lining.json
beta-marker import pattern.dxf --width 1500 --split-materials materials -o all.json
beta-marker import pattern.dxf --width 1500 --material-keywords keywords.json -o all.json
```

Keyword JSON maps material names to lists, e.g.
`{"mesh":["mesh","net"],"lining":["lining","liner"]}`. Entries replace built-in
words for that material; explicit DXF metadata wins. Sorted keys define keyword
precedence (interlining precedes lining). Filters use exact case-insensitive
material names. Numbered, sanitized split filenames retain width, spacing,
order and relevant match rules. A filtered PLT result retains original positions.

Layer-6 half patterns unfold and store the axis as `Piece.fold_line`, in unfolded
piece coordinates. A layer name or text/attribute starting with MATCH, PLAID or
STRIPE supplies a `match_points` entry at its first point/text anchor. Full text
is the ID; duplicates get -2, -3, etc. Ordinary notches are not match points.
No pairwise matching rules or fabric repeat lengths are invented.

## ASCII DXF basics

`import_dxf(bytes, &ImportOptions::default())` reads an offline ASCII DXF into
millimetre `beta_marker_core::Piece` values. `export_aama(&pieces)` returns a
fallible R12 ASCII string. No new dependencies are used.

The reader accepts LF or CRLF, padded group codes, R12 and later ASCII section
layouts, unknown sections, block base points, nested INSERT translation, separate
X/Y scales, rotation, and layer-0 inheritance. Supported entities are LINE,
POLYLINE/VERTEX/SEQEND, LWPOLYLINE, ARC, CIRCLE, POINT, TEXT, plain MTEXT, and SPLINE.
Unknown entities are ignored with a warning. INSERT cycles and missing blocks
return errors. Binary DXF returns an explicit error.

Circular arcs and bulges use a chord sag below 0.01 mm after unit and INSERT
transforms. Rational and non-rational control-point B-splines use homogeneous
de Boor evaluation. Each knot span is seeded at twice the control degree, then
recursively sampled until quarter-point,
midpoint and three-quarter-point interpolation errors are below 0.01 mm. This
is a sampling criterion, not a proof of the maximum error of every possible
high-degree spline. Fit-point-only splines become a warned polyline through
their fit points. Inputs that exceed convergence or allocation limits return
errors. Export uses eight decimal places.

## Garment mapping

Each top-level inserted BLOCK is a piece. Nested INSERTs supply that piece's
geometry. Layer 1 is the cut boundary; segments join in either direction within
0.05 mm. A repeated closing point is removed. Open boundaries are closed by a
straight segment with a warning. The largest connected boundary is the outline;
additional disconnected layer-1 loops become warned internal lines. Pieces with
no usable layer-1 boundary are skipped with a warning.

Layer 7 supplies the grain, preserving the longest line's direction. Layer 4
supplies notches, including single POINT notches. Two-point notch lines start
at the endpoint nearest the cut boundary. Layer 13 supplies drill centres
from POINT or CIRCLE. Layers 8, 11 and 14 are internal lines (including sew lines).
Layer 2 turn points and layer 3 curve points are retained as Line reference
marks; they do not become physical cut notches. A layer-6 axis unfolds a half
pattern whose straight fold edge lies on that axis. Marks off the fold are
mirrored too. Text and annotations are retained as Text marks.

Case-insensitive TEXT/MTEXT labels read Piece Name, Size, Quantity, Material,
Style Name, Sample Size, Units, Category, Rule Table and Annotation. The core
has no category or grade-table field, so those labels remain Text marks.
Supplied blocks of every size are imported as drawn. Without `--rules`, Rule
Table text warns that only supplied geometry is imported. A base piece without Size uses Sample Size.
An explicit ASTM comment or file text sets dialect `astm`; otherwise numbered
piece blocks use `aama`. Geometry alone cannot reliably distinguish them.

Auto units resolve `$INSUNITS` (1 inch, 4 mm, 5 cm), then AAMA Units text
(`ENGLISH`/`METRIC` and common unit names), then `$MEASUREMENT` with a warning.
Other `$INSUNITS` values produce a warning and continue resolution. With no
unit evidence, a largest piece dimension below 80 source units is assumed to
be inches, otherwise millimetres. This heuristic always warns. Use an explicit
unit override for small trims or ambiguous drawings. Coordinates are not
translated to an arbitrary local origin.

IDs are sanitized `STYLE-PIECE-SIZE`, with suffixes for collisions. Missing style
and size use `STYLE` and `BASE` in the ID. Missing grain warns and leaves the
core's documented x-axis grain default. Quantity defaults to 1. By default,
Quantity N gives N normal copies and zero mirrored copies. `ImportOptions.pairs`
or CLI `--pairs` splits **every positive even** N into N/2 normal and N/2 mirrored
copies. Odd quantities stay normal. The rule cannot infer which pieces are
paired; this is the caller's explicit choice.

A drawing without garment blocks uses `generic`: every outer closed polygon
is a piece; closed outlines fully nested inside it are Line marks. Layer-7 grain
lines wholly inside the outer piece are retained. Other loose annotations are
not assigned to generic pieces. Quantity defaults to 1.

## Export and limits

Export writes R12 POLYLINE outlines and marks, grain LINEs, notch LINEs/POINTs,
drill POINTs, layer-15 TEXT, `$INSUNITS=4`, and one inserted BLOCK per piece.
Optional `BETA:` TEXT preserves the ID, style, normal/mirrored quantity, rotation
and buffer, which standard AAMA cannot represent. Other CAD readers can ignore
that field. It is trusted only as non-geometric metadata and is validated.
Registered `BETA_MARK` XDATA preserves mark text (including empty text, line
breaks and labels that resemble piece metadata) without changing its geometry.

This is a planar garment reader, not a general CAD modeler. 3D meshes,
non-XY extrusion, array INSERT, binary DXF, legacy non-UTF-8 text encodings,
MTEXT formatting/layout, hatch, dimensions, and general vendor grade-rule dialects
are outside version 0.1. Non-XY extrusion and unsupported INSERT/mesh forms
return errors. MTEXT supports plain strings and paragraph breaks, not rich text.
The reader limits input to 64 MiB, four million physical lines, 32 INSERT levels,
100,000 expanded INSERTs, two million output points, and 65,536 samples per curve.

Reproduce original fixtures with `cargo run -p beta-marker-dxf --example make_testdata`
from the marker workspace. See `testdata/dxf/README.md` for the data licence.

## Grading contours

Rule references on the cut line grade that contour. Closed internal lines carry
their own rules, interpolated by arc length. Exact internal reference points
follow their own rules. Other marks follow the nearest graded contour. If
cut-line rules are absent, cut vertices follow the nearest graded internal
contour displacement, with an explicit seam-allowance approximation warning.
This is not an exact offset reconstruction. Verify measurements before cutting.
Unnumbered grain lines move without changing direction. Two explicitly numbered
grain endpoints follow their own rules.

For a local measurement report and overlaid cut-outline SVG, run:

```sh
cargo run -p beta-marker-dxf --example inspect_grades -- job.json report-directory
```

The output directory contains source geometry and identifiers. Keep reports from
private jobs outside the repository.


## Original Seamly2D export regressions

`testdata/dxf/seamly/` contains original Apache-2.0 shirt exports, with provenance
linked to the native inputs in `pattern/testdata/seamly/`. Tests cover actual
R12 and modern-writer AAMA, baseline missing-unit inference, explicit mm units,
configured/default/hidden labels, hidden grain, quantities, sizes, cotton labels,
internal paths, notch segments and explicit pairs. No Seamly process is run.

A free-text "mirrored pair" note is kept as a text mark. Without an explicit
pair rule it now raises a warning: Quantity N stays N normal copies. `pairs`
remains an opt-in convention that splits positive even totals. An explicit
BETA round-trip copy count already supplies a rule and needs no note warning.
Empty metadata fields are skipped so they cannot hide a later populated label.
Stable export fields remain independent of decorative labels. Missing material
stays None in the core report; any caller that assigns a default fabric must
warn and obtain bill-of-materials confirmation. Hidden grain stays unknown.
The baseline R12 file has no explicit units and still warns about its size
heuristic. The reader does not silently certify unitless data as millimetres.
