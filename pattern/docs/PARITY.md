# Pattern making and grading: gap review

Reviewed 20 October 2026 against the code at `0d7b298`. This is a feature review,
not a test of commercial products. Public supplier pages were read only to
identify features. No supplier text, code, specifications, sample patterns or
file formats were copied. Factory value and gap size are BETA engineering
judgements, not measured savings. BETA remains short of full commercial parity.

## Baseline and priorities

Small = a usable workflow needs refinement. Medium = an important part is
missing. Large = a substantial workflow is absent.

| Feature and public evidence | BETA before this work | Gap before | Value to an Indian knit/woven factory | Priority / intended work |
| --- | --- | --- | --- | --- |
| Parametric drafting and measurements [S1, T1] | Points, lines, curves, safe formulas, size measurements, CSV, tracing and a tool catalog | Small | Repeat knit blocks and buyer measurement changes | Preserve existing tools |
| Per-edge seam and hem allowances, corner choices [G1, C1] | Width/formula per edge; mitre, bevel, square and mirrored corners | Medium | Different overlock, joining and hem widths | Verify export; reverse and fold-back corner workflows remain gaps |
| Notches and pattern annotations [C1] | Slit, V, T, castle, crossing-path notches, drills, labels, buttons and internal paths | Small/medium | Assembly matching, pocket positions and plackets | Test marks through marker/DXF; add buttonhole and repeated placement workflow |
| Darts, pleats and style changes [O1] | Dart creation/transfer/closure, knife/box pleats, slash-and-spread; outputs use fixed geometry | Medium | Woven fitting and style variation | Keep; associative edits, fold-in darts and dedicated tucks remain |
| Mirror and symmetric pieces [C2] | Object reflection and fixed piece reflection; piece copy loses marks/grain; fold line expects a full outline | Large | Left/right parts; front/back cut on fold | **High:** preserve piece details; unfold a half pattern into a full symmetric piece |
| Seam walking and verification [G2, O1] | Compare two draft paths in one size; only a length difference message | Large | Check side seams, sleeve/armhole ease and collar joins before sampling | **High:** compare selected piece-edge chains in every graded size, with ease, tolerance and a downloadable report |
| Seam shape/length correction [G2, O1] | Scale a separate path to a target length; true dart-leg construction | Medium | Remove sewing mismatch after edits | Retain; automatic curve/corner truing remains |
| Factory point rules and grade nest [T1, O1] | Relative/incremental x/y rules, point attachments, all-size overlay and reference alignment | Small/medium | Buyer size breaks, size-set checking | Verify actual graded seam lengths; make attachments easier to review |
| Reusable grade-rule library [T1] | Rust library functions; no file controls in Pattern tab | Medium | Reuse approved size increments across related styles | **High:** validated JSON library import/export in Grading |
| DXF plus rule-table interchange [O2, L1] | All-size DXF; BETA rule comments. Marker has a limited ASCII RUL parser | Large | Exchange supplier patterns without retyping every rule | **High:** reuse existing parser for bounded RUL library import/export and round-trip tests. No claim of universal vendor RUL compatibility |
| Measurement-chart grading [T1] | Measurement table drives formulas for each size; regular increments | Medium | Buyer specification changes | Preserve; finished-garment measurement constraints/automatic fitting remain |
| Piece information into marker [T1] | Name, fabric, grain, normal/mirror quantity, annotations and full-piece fold axis | Small/medium | Correct cut quantities, fabric separation and grain | Regression tests including transformed pieces |
| Photo digitizing [T1, O2] | Rust calibration, contour extraction, validation and editable Pattern handoff | Medium | Convert paper blocks without a digitizing table | Preserve; camera setup and manual checking remain necessary |
| 3D fit, collaboration and commercial CAD breadth [O1, L1] | No 3D fit simulation or shared approval workflow | Large | Sample review and multi-site production | Outside this bounded implementation |

## Public feature sources

Only feature names and broad workflow categories inform the table. A feature
documented by one supplier is not attributed to every supplier.

- **G1:** [AccuMark seam update](https://help.gerbertechnology.com/AccuMark/PDS/Function_Update.htm).
- **G2:** [AccuMark overview](https://help.gerbertechnology.com/AccuMark/Welcome.htm).
- **T1:** [TUKAcad product page](https://tukatech.com/products/tukacad).
- **O1:** [Optitex 2D/3D pattern design](https://optitex.com/products/2d-and-3d-cad-software/).
- **O2:** [Optitex apparel modules](https://optitex.com/fashion-apparel-solutions-bundles/).
- **L1:** [Lectra Modaris workflow overview](https://www.lectra.com/en/library/the-modaris-upgrade-working-at-the-speed-of-fashion).
- **C1:** [CLO pattern information help index](https://support.clo3d.com/hc/en-us/sections/115003578728-2D-Pattern-Information).
- **C2:** [CLO unfold help](https://support.clo3d.com/hc/en-us/articles/115000337267-Unfold).
- **S1:** [Seamly documentation index](https://wiki.seamly.io/wiki/Main_Page/en).

## Implementation boundary

All calculation remains in Rust. The browser collects inputs and displays
results. Reuse BETA's existing DXF and RUL implementation, with original test
geometry. No new dependencies are planned. Keep the public comparison's
“partly” assessment; a handful of closed gaps cannot establish commercial parity.

## Result after implementation

| Selected gap | Before | After | Remaining gap |
| --- | --- | --- | --- |
| Graded seam checks | Single-size draft-path difference; piece inspector ignored factory deformation | Rust retains each graded stitch edge. A new Seam checks mode measures connected runs in every size, applies ease/tolerance, aligns paths and exports CSV/JSON | Medium: no rolling animation, sewn-fit test or notch-to-notch ease distribution |
| Reusable rules | Core library functions without file controls | JSON and bounded ASCII RUL export, preview and import; explicit collision replacement; all-size validation; numeric rule ID editing | Small for local library reuse; large for universal vendor-native exchange |
| Mirror pieces | Fixed outline; marks/grain lost; nonuniform allowances not retained | Linked geometry; reflected point rules; marks, paths, anchors, grain, labels, unequal allowances and quantities retained | Medium: reflected rule values are copied, not live shared rules |
| Symmetric pieces | Fold axis expected a full piece; no half-to-full tool | Linked unfold from a straight zero-allowance edge; mirrored marks; on-fold marks kept once; full-piece axis sent to Marker | Medium: measurement grading only; factory-rule half patterns are rejected |
| Repeated production marks | Individual buttons/drills/lines | Parametric rows of buttons, drills and buttonholes with end bars; retained in mirrored/unfolded pieces and exports | Small/medium: no full button component library or automatic placement checks |
| Fold guide output | Direct job axis existed; full-piece fold guide absent from pattern DXF/print output | DXF internal fold guide, dashed PDF guide and PLT guide; valid importer round trip for the full cut outline | Medium: DXF reimport retains the guide, but does not restore the fold cutting constraint |

An original native regression makes the correctness difference concrete: a
100 mm draft edge grades to **90, 100 and 120 mm**. The old inspector reported
the draft length; the new result uses each size's actual graded stitch path.
This is a test of BETA geometry, not a claimed production saving.

The existing drafting, per-edge allowances, shaped notches, darts, pleats,
slash/spread, measurement tables, grade nest and digitizing remain available.
General projection of seam-anchored notches onto an allowance cut edge, new
reverse/fold-back corner types, fold-in darts, dedicated tucks and associative
style tools were not implemented in this round.

See [the operator guide](PRODUCTION-TOOLS.md) for limits and exact workflows,
and [the implementation report](PATTERN2-REPORT.md) for executed checks.
