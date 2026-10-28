# Features and where to find them

Scope: the 0.8 development base, including public 0.7. **Experimental 0.8**
means implemented but unreleased, with limited acceptance evidence. A function
is not a claim of fit, machine compatibility or commercial parity.
Names follow the English base; launch navigation may change. **More tools**
holds separate Pattern, Grading, Digitize, Compare and cutting-room screens.

## Pattern

| Function | Screen or menu |
|---|---|
| Start blank or from seven editable garment blocks. | Make a style → Style |
| Name a style and select/edit an example measurement chart. | Make a style → Style / Grade |
| Draw points, lines, cubic curves and arcs with numeric or formula inputs. | Make a style → Pattern; More tools → Pattern |
| Drag points/handles, snap, measure, zoom, pan, select and undo/redo. | Make a style → Pattern |
| Rename pieces and set normal/mirrored quantities, fabric group and grain. | Pattern → piece controls |
| Add internal paths, labels, shaped notches, drills, buttons and buttonholes. | Pattern → marks / piece tools |
| Set per-edge seam/hem allowances and supported corner treatments. | Pattern → allowances |
| Mirror, split, join, offset, add/transfer/close darts, pleat and slash/spread. | Pattern → shape tools; Style → Pattern |
| Retain supported linked Style derivatives and re-evaluate them in each size. | Make a style → Pattern; review new allowances/marks |
| Unfold a measurement-graded half pattern from a straight zero-allowance edge. | More tools → Pattern → Piece tools → unfold |
| Place repeated button, buttonhole or drill rows. | More tools → Pattern → Piece |
| Trace a background image and export measurement tables. | More tools → Pattern → Draft / Measurements |
| Grade by measurement charts and show stacked sizes. | Make a style → Grade; More tools → Grading |
| Attach relative/incremental point rules and reuse JSON/RUL libraries. | More tools → Grading → Reuse grade rules; bounded RUL subset is experimental |
| Check connected seam runs across sizes against ease/tolerance, with CSV/JSON. | More tools → Pattern → Seam checks |
| Check generated block shapes/seams and export points-of-measure PDF, CSV, XLSX. | Make a style → Grade → block checks / spec sheet |
| Import supported DXF-AAMA/ASTM geometry and optional .rul grading. | Marker from files → Open DXF; dialect coverage is limited |
| Import a supported Seamly2D file subset with warnings. | More tools → Pattern → import |
| Send selected sizes, quantities and fabric group to nesting. | Make a style → Marker; Grading → Send sizes to marker |
| Export patterns as DXF-AAMA, PLT and tiled PDF. | Make a style → Cut; Pattern → output |
| Save/reopen local pattern and Style documents. | Pattern / Make a style → file controls |

Evidence: [Style API](../../pattern/docs/STYLE-API.md),
[production tools](../../pattern/docs/PRODUCTION-TOOLS.md),
[block checks](../../pattern/docs/BLOCK-CHECKS.md). Blocks need sewn samples.

## Marker

| Function | Screen or menu |
|---|---|
| Set width and open, folded or tube fabric. | Make a style → Lay; Marker → Fabric |
| Record lay face, physical plies and spread-end allowances. | Make a style → Lay; Cut plan |
| Enforce grain, rotation, nap, mirror and garment-direction rules. | Make a style → Lay; Marker → fabric/piece rules |
| Set piece gaps, edge gaps, piece buffers and fusing margins. | Make a style → Gaps; Marker → rules |
| Apply measured shrinkage, defects, shade and matching restrictions. | Rolls / Marker → fabric and piece rules |
| Set stripe/check repeats and match points with overlays and rule reports. | Make a style → Stripes / Marker |
| Show graded-size and previous-version geometry overlays. | Make a style → Pattern / Marker |
| Nest locally in Rust/WASM and show complete valid best-so-far layouts. | Make a style → Marker; Marker from files → Nest marker |
| Show piece names/sizes, zoom, length, footprint, useful yield and unused cut copies. | Marker results / Make a style → Cut |
| Validate overlap, quantity, clearances and garment rules independently. | Marker → Checks |
| Move, rotate, flip, lock, select groups and place missing copies. | Marker canvas, toolbar and piece tray |
| Bump to contact, compact, fill gaps and swap selected pieces. | Marker toolbar |
| Set hard cut sections and a marker/table-end limit. | Marker → section and boundary tools |
| Undo/redo edits and reuse compatible saved marker-library layouts. | Marker → history / library |
| Compare an imported PLT reference with a BETA marker. | More tools → Compare |
| Keep a previous valid shorter marker after fresh search. | Marker / Style → Nest; experimental 0.8 |
| Retain compatible browser search state across rounds and checked recovery on Stop. | Browser Nest; experimental 0.8 engine work |
| Compare mixed half/full-cut allocations for folded/tube jobs. | Nest with folded/tube fabric; experimental 0.8 |
| Try stronger checked candidates around eligible defects/locks. | Nest; experimental 0.8, no guaranteed saving |
| Preview a bounded area improvement, Apply once and Undo. | Marker → Improve area; experimental 0.8, 32 movable pieces / 1000 placements maximum |
| Try an optional native search strategy, off by default. | Marker → Compare search methods, desktop/server with at least 12 seconds; CLI `nest --adaptive`; experimental 0.8, excludes matching/shade jobs |

Evidence: [browser exports](BROWSER-EXPORT-API.md),
[folded results](../crates/beta-marker-nest/FOLD7_RESULTS.md),
[engine pilot](NESTING-THREE-IMPROVEMENTS-2026-10-24.md),
[workflow additions](WORKFLOW-IMPROVEMENTS-2026-10-25.md).

## Cutting room and files

| Function | Screen or menu |
|---|---|
| Export SVG, DXF, PLT/HPGL, roll PDF, A4/A3 tiles and generic ISO locally. | Marker export / Make a style → Cut |
| Export marker text/XLSX reports, mini-marker PDF and labels. | Reports / Marker export |
| Plan integer lays from quantities, ratios, limits and overcut. | More tools → Cut plan |
| Stop browser planning and retain a checked partial plan for review. | Cut plan → Stop; inspect shortages before release |
| Compare offered widths/prices, ratio menus and eligible stock with measured plans. | Cut plan → Compare fabric widths, ratios and order cost; experimental 0.8, open fabric only |
| Download comparison JSON and use complete final-checked alternatives. | Same comparison panel; experimental 0.8 |
| Record rolls, four-point inspection scores, shade groups and measured shrinkage. | More tools → Rolls |
| Allocate inspected rolls and prepare spreading work. | Rolls / Cutting floor → Spreading |
| Preview, map and validate original order/roll CSV/XLSX templates before import. | Cutting floor → Imports |
| Send reviewed purchase-order quantities to planning. | Cutting floor → Imports |
| Reserve rolls, specify safe splice zones and print lay instructions. | Cutting floor → Spreading |
| Export generic spreader CSV/JSON and ply sticker order. | Cutting floor → Spreading → Release files |
| Make physical-layer bundles, A4 QR tickets and 100×50 mm stickers. | Bundles; Cutting floor → Spreading |
| Record spread/cut/bundled status and measured issue/returns. | Cutting floor → Tracking |
| Select exact damaged parts, schedule replacement jobs and track replacement. | Cutting floor → Re-cuts |
| Issue registered bundles with keyboard/USB QR input and duplicate/conflict checks. | Cutting floor → Sewing |
| Show a daily dashboard and export consumption and part-based WIP. | Cutting floor → Dashboard / Sewing |
| Generate measured reports, estimates and report packs. | Bundles / Reports; richer buyer/monthly configuration is CLI/library-only |
| Read/print a self-contained operator manual. | Cutting floor → Manual |

Evidence: [browser cutting room](BROWSER-CUTROOM-API.md), [plies](PLIES.md),
[module audit](CUTTING-ROOM-STATUS.md), and source
[floor controls](../apps/web/src/floor-ui.ts),
[marker controls](../apps/web/src/main.ts). Generic files need a real-device dry run.

## Paper patterns and local operation

| Function | Screen or menu |
|---|---|
| Extract paper outlines from a phone photo with an A4 reference. | Home → Digitize paper patterns (Photo to marker flow) |
| Review names, photographed size, copies, pairs and grain. | Photo to marker → piece review |
| Suggest labels and fold/grain hints for operator confirmation. | Photo to marker → review; experimental assist with low measured naming accuracy |
| Print tiled markers/chalk guides, plot or open the marker editor. | Photo to marker → Use |
| Calibrate projector display with a physical ruler. | Photo to marker → projector; experimental, check at the working surface |
| Save/open reusable paper sets without photos, restarting review. | Photo to marker → Save/Open pattern set; experimental 0.8 |
| Inventory parts by size/fabric and keep width/layout/GSM profiles per group. | Photo to marker → pattern set / fabrics; experimental 0.8 |
| Check outline-point distance against a paper tape measurement before nesting. | Photo to marker → tape check; experimental 0.8 |
| Digitize a supported board-target photo into editable patterns. | More tools → Digitize; desktop/server only, camera accuracy experimental |
| Work offline after successful browser caching, or use native desktop/CLI. | Browser / Windows desktop; initial downloads need connectivity |
| Save/restore browser backup files and local projects. | Browser notice → Save/Restore backup; project file controls |
| Switch cm/inch, light/dark/system theme and English/Hindi/Tamil. | Top bar; Hindi/Tamil are drafts; new paper panels remain English |
| Prepare feedback for review without automatic sending. | Feedback button |
| Run authenticated factory-server computation, batch jobs and persistent queues. | CLI `serve`, `batch`, `queue`; no queue management screen or per-user roles |

Evidence: [paper sets](PAPER-PATTERNS-MSME-2026-10-26.md),
[Quick Marker API](QUICK-MARKER-API.md), [install](INSTALL.md),
[self-hosting](SELF-HOSTING.md), [privacy](PRIVACY.md).
No automatic photo grading, 3D fit, live ERP connector or shared approval
library is included. Real-photo accuracy and production hardware acceptance remain open.

## Developer and research tools outside the app menu

These are included for completeness. They are not production UI features or
claims of improved default nesting.

| Function | Access and limits |
|---|---|
| Validate, import/export, nest and run pipelines without the UI. | `beta-marker` CLI; command help and [Marker quick start](../README.md#quick-start) |
| Run frozen public benchmark protocols and export aggregate evidence. | [Public benchmark runner](../bench/public/README.md); external inputs and explicit environment |
| Generate richer production, buyer/monthly and measured report packs. | CLI/library; [module audit](CUTTING-ROOM-STATUS.md), no complete configuration screen |
| Generate conservative GPU/CPU raster proposals and bounded gap-fill candidates. | [GPU experiment](../crates/beta-marker-gpu/README.md); separate opt-in crate, unsupported rules rejected; production remains CPU |
| Estimate recorded Lectra efficiency/length with an offline learned model. | [Learn API/tool](../crates/beta-marker-learn/README.md); experimental, plain zero-gap open jobs only, never a feasible marker or packing bound |
| Try pair clusters, aggregate, restart, gap-fill, two-stage and race strategies. | [Cluster library](../crates/beta-marker-cluster/README.md); experimental, Auto remains plain nesting |

Source review for the feature map used the screen handlers in
`apps/web/src/style/ui.ts`, `pattern-editor.ts`, `quick/ui.ts`, `main.ts`,
`floor-ui.ts` and their Rust/WASM contracts. The release reports establish
tested scope; this document does not claim a fresh app acceptance run.
