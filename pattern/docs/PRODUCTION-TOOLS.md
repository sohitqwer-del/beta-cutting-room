# Daily pattern checks and production details

These tools run locally in Rust, through the existing Pattern WASM module or
local server. No account, runtime internet access or extra service is needed.
All lengths use millimetres.

## Check sewing seams

Open **Pattern → Seam checks**. Choose two pieces and their connected edge
numbers. The lists below the inputs show the edge names. A run such as `3,4,1`
can wrap around the end of a piece. Edges must follow piece order without repeats.

Enter the required ease and allowed error. Ease is extra length to distribute
while sewing. Positive ease means A should be longer than B. For example,
10 mm ease and 1 mm allowed error passes when A minus B is between 9 and 11 mm.

**Check all sizes** measures graded stitch paths, before seam allowance. It
includes factory point rules. The result lists both lengths, their difference,
the error from the required ease, and PASS or CHECK. It uses 0.05 mm curve
sampling; this is not a bound on total measured-length error.

The preview translates both starts to one point and aligns their first tangents.
It does not stretch a seam. Reverse either run if the sewing direction needs
it. This is a length and alignment aid, not a rolling walk animation or a fit
simulation. Export CSV for a size-set review or JSON to retain inputs and paths.
Changing inputs invalidates the visible report. Editing the pattern clears it.

## Reuse grade rules

Open **Grading → Reuse grade rules**. JSON keeps relative or incremental modes.
RUL export converts every rule to cumulative x/y movement from the base size.
Both formats keep size order. A library must match the current size order and
base size before it can be applied.

Choose a file and review the table. **Apply reviewed rules** commits the import.
Existing IDs are rejected unless **Replace matching rule IDs** is checked.
Every affected size is evaluated before the change is accepted. Undo restores
the previous document. Imported numeric rule IDs can be edited in the rule table.

Importing a library does not attach its rules to points. Use the existing
piece/point/rule selectors to attach them. Attachments and formulas stay in
pattern JSON. All-size DXF continues to include BETA's comment-based rule table.

RUL support reuses `beta-marker-dxf::GradeTable::parse`. Its bounded ASCII subset
uses explicit units, size order, base size, mode and one x/y pair per size.
See the [existing reader contract](../../marker/crates/beta-marker-dxf/README.md).
This is not a claim of universal AAMA/ASTM or vendor-native RUL compatibility.
No vendor format specification was used to add these controls.

## Mirror a complete piece

In **Piece → Piece tools**, choose **mirror**, give the new piece an ID, and
choose two drawing points for the reflection axis. The copy retains unequal
edge allowances, marks, internal paths, grain, labels, anchors, material,
normal/mirrored counts, rotation and clearance.

The geometry stays linked to source constructions. Factory rule displacement
vectors are reflected about the base-size axis and copied to new rule IDs.
Those copied rule values are independent: recreate or edit them after changing
the source rules or reflection-axis direction. Other existing shape tools still
make fixed outlines in one evaluated size.

## Unfold a half pattern

Use **unfold** with the edge number that lies on the fold. That edge must be
straight and have zero allowance in every size. The half pattern must lie on
one side of the axis. This version accepts measurement-graded pieces and rejects
factory-rule half patterns explicitly.

The result is a full symmetric piece, as required by the Marker contract.
It retains the original exterior and a reversed reflected exterior. The fold
edge becomes an internal fold axis. Explicit edge notches on the fold must be
removed first. Exterior notches, drills, internal paths and buttonholes are
mirrored. Each crossing-notch guide is restricted to its own half; intersections
with the removed fold edge are omitted. Marks and guides exactly on the fold
are kept once.

Geometry references stay linked, but the mark list is copied when the tool runs.
Recreate the unfolded piece after adding/removing source marks or moving an
on-fold mark away from the fold. This is not a live whole-piece symmetry constraint.

The source remains in the document. If it is only a construction template, set
**both Quantity and Extra mirrored copies to zero** before sending to Marker.
Check the new full piece's quantities separately. A fold line does not choose
the fabric mode; set the intended open/folded fabric in Marker.

## Repeated buttons, buttonholes and drills

Open **Piece → Button, buttonhole or drill row**. Select a first centre, count,
spacing and row direction. Set button diameter or buttonhole length. The
buttonhole angle is independent of the row direction.

The tool inserts native parametric point constructions and ordinary production
marks. Buttonholes are internal lines with end bars, not cut-outline holes.
Buttons export as labelled drill centres. The UI accepts numeric dimensions;
the Rust model also accepts formulas. Edit generated constructions with the
existing draft tools. Check that the marks lie in the intended part of the piece.

## What travels to production

The **Send sizes to marker** action and native `to-job` retain size ratios,
normal/mirrored counts, grain, material, marks and full-piece fold axes.
DXF round trips retain outlines, shaped notches, drills, buttonhole lines,
labels and copy counts through BETA's existing importer.

A full symmetric piece's DXF fold axis is an internal guide with a fold label.
It is not a layer-6 half-pattern instruction: sending a full outline with that
instruction would ask the reader to unfold it twice. The current DXF importer
does not restore `Piece.fold_line` from this guide. Use pattern JSON or the
direct Marker handoff to preserve that cutting constraint. PDF and PLT now
include fold guides too.

## Remaining gaps

- No factory-rule half-pattern unfolding or live shared grade rules on reflected
  copies.
- No full vendor RUL dialect coverage, combined external DXF/RUL point-attachment
  reconstruction, or certified cross-vendor exchange.
- No automatic curved-seam truing, animated rolling walk, sewn fit validation,
  or notch-to-notch ease distribution.
- Existing seam-anchored notch placement is retained; general projection onto
  the allowance cut edge is not added here.
- Reverse/fold-back allowance corner workflows, fold-in darts, dedicated tucks,
  and associative dart/pleat/slash edits remain incomplete.
- No finished-garment measurement constraint solver, 3D fit or shared approvals.

No new third-party crate is introduced. The WASM adapter now directly uses the
workspace's existing `anyhow` dependency (MIT OR Apache-2.0).
