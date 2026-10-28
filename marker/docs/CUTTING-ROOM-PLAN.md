# Project BETA: complete cutting-room software plan

Date: 19 October 2026. Status: historical proposal. The `pattern/` product now
exists. See [current features](FEATURES.md) and [roadmap](ROADMAP.md). Market
claims, gap lists and proposed approvals below describe the original research,
not a current implementation or supplier comparison.

## 1. Open-source pattern-making software (checked on GitHub, 19 October 2026)

| Project | Licence | State | What it does | Can we use the code? |
|---|---|---|---|---|
| Seamly2D | GPL-3.0 | Active (push Oct 2026), about 1,000 stars, C++/Qt | Parametric drafting from body measurements: points, lines, curves, formulas, pieces, seam allowance, layout, DXF-AAMA export | **No.** GPL code cannot go into Apache-2.0 BETA. We can read and write its files, and send it fixes upstream. |
| Valentina | GPL-3.0 | Active (Oct 2026) | Same family as Seamly2D (Seamly2D was forked from Valentina). Grain-aware layout. | **No** (GPL) |
| FreeSewing | MIT | Moved from GitHub to Codeberg; JavaScript | Made-to-measure patterns written as code; good library of blocks (shirt, T-shirt, trouser, hoodie...) | **Yes**: as a block library and test data (already in `testdata/open/`) |
| GarmentCode | MIT | Research (ETH Zurich), Python | Garment patterns as programs, for 3D simulation and AI | **Yes**: ideas and data; not a factory tool |
| patternForge | BSD-3-Clause | Tiny (1 star) | Simplified blocks, AAMA/ASTM export | Ideas only |
| LibreCAD | GPL-2.0 | Active | General 2D CAD, not garment-specific | **No** (GPL) |

**Conclusion.** No permissively licensed, industrial pattern CAD exists. Seamly2D is the best open tool, but:
- it is GPL;
- it is built for made-to-measure drafting, not factory grading by rule tables;
- it has no digitizing of paper patterns;
- it has no marker connection.

**Recommendation: build BETA Pattern from scratch in Rust** (Apache-2.0). Reuse BETA Marker's geometry core and DXF code, and use FreeSewing blocks (MIT) as starter patterns. Read Seamly2D files so its users can move to BETA easily.

## 2. What BETA Pattern must do (target: TUKAcad, Gerber AccuMark PDS, Lectra Modaris, Optitex PDS)

| Area | Functions |
|---|---|
| Drafting | Points, lines, curves (Bezier/spline), arcs, measurements, formulas, parametric blocks, construction lines, snap, undo |
| Pattern tools | Dart create/rotate/close, slash-and-spread, pleats, tucks, fullness, yoke and panel split, mirror/fold, add/remove seam allowance (per edge, corner types), notches (types), drills, buttons/buttonholes, grain line, walk and true seams, measure seam lengths, curve smoothing |
| Pieces | Piece table (name, quantity, pairs, material, fabric rules), annotations, piece checks (closed, self-crossing, notch counts match) |
| Grading | Rule-based grading (x/y delta per point per size; Gerber/Tukatech style), measurement-based grading (Seamly2D style), grade rule library, nest view of all sizes, grade checks |
| Digitizing | Photo/scan of a paper pattern on a calibration board (checkerboard/ArUco) -> outline, notches, grain -> DXF. Most small Indian units still keep paper patterns. Optional digitizer tablet support. |
| Import/export | DXF-AAMA/ASTM with grade rules (in and out), Seamly2D/Valentina files (read), SVG/PDF, plotter PLT, FreeSewing blocks |
| Made-to-measure | Size from body measurements (formula patterns) |
| Spec and costing | Measurement chart (POM), tech-pack sheet, fabric consumption estimate (piece area / expected marker efficiency) |
| Link to marker | One click: send graded pieces with rules to BETA Marker |
| Later | 3D try-on (GarmentCode research, open 3D simulation), AI pattern suggestions |

## 3. The complete cutting room: modules and status

| # | Module | What it does | Status |
|---|---|---|---|
| 1 | Pattern making and grading | Section 2 | **Missing**: BETA Pattern (new) |
| 2 | Digitizing paper patterns | Photo/scan to DXF | **Missing** (part of BETA Pattern) |
| 3 | Costing / consumption | Fabric per garment before the order, from pattern area and predicted efficiency (the Lectra dataset can train the predictor) | **Missing** |
| 4 | Marker making | Nesting with garment rules, editor, output | v0.1 done; v0.2 in progress |
| 5 | Cut order planning | Order -> lays (ratio + plies) | In progress (agent `plan`) |
| 6 | Fabric inspection | 4-point system, defect map per roll -> marker defects | **Missing** |
| 7 | Shade and shrinkage | Shade groups per roll/lot, shrinkage test -> marker shrinkage | **Missing** |
| 8 | Roll allocation and spreading plan | Which rolls go into which lay, splice points, end bits, ply count per roll, least waste | **Missing** |
| 9 | Spreader and cutter output | Cutter ISO files, spreader instructions | Cutter ISO in progress; spreader missing |
| 10 | Ply numbering, bundling and ticketing | Bundle tickets, QR codes, sticker printing (prevents shade mix-up in sewing) | Labels in progress; bundle system missing |
| 11 | Re-cut and replacement | Track damaged pieces, small re-cut markers | **Missing** |
| 12 | Cutting reports | Planned vs actual fabric, end-bit reconciliation, cutting efficiency per lay | **Missing** |
| 13 | Link to sewing | Bundles -> sewing line WIP and BETA Time line balancing | **Missing** |
| 14 | ERP/PLM link | CSV/Excel import of orders and rolls, export of consumption | **Missing** |

## 4. Proposed build order

1. **Now (running):** marker v0.2, Lectra benchmark.
2. **Next wave (cutting room, inside BETA Marker):** fabric inspection + shade/shrinkage + roll allocation/spreading plan (modules 6-8), bundling/ticketing + re-cut + reports (10-12), costing (3).
3. **BETA Pattern (needs a new folder `pattern/`):** start with the geometry and grading engine in Rust, DXF-AAMA with grade rules, piece table, then the browser drafting UI, then pattern tools, then photo digitizing, then Seamly2D import.
4. **Integration:** pattern -> marker -> cut plan -> spreading -> bundles -> sewing (BETA Time).
