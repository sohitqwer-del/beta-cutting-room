# Cutting-room module audit

> Historical module audit, 19 October. Browser cutting-room parity arrived in
> [0.7](BROWSER-CUTROOM-API.md). Development 0.8 adds bounded area preview and an
> open-fabric width/ratio/order-cost screen, so the old CLI-only/no-screen rows
> below are superseded for those workflows. See [current features](FEATURES.md)
> and [the exact new limits](WORKFLOW-IMPROVEMENTS-2026-10-25.md).

Audit date: 19 October 2026. Branch: `marker/astra-cutroom`, based on
`marker/mvp` at `25e1233`. The proposal's original “missing” labels predate much
of the current implementation. This audit checked source, CLI registration,
web/desktop reports and the new floor tests. It does not claim hardware or
factory acceptance.

**Evidence key:** **Base** means a prior report documents the workflow; it was
not separately repeated in this task's one browser session. **Rust** means the
new floor command integration tests exercise the actual Rust engine and exports.
**UI** refers to the completed single browser smoke, recorded in
`marker/apps/web/FLOOR_REPORT.md`. Desktop shares the new UI/WASM but was not
launched again. Full desktop installer/hardware verification remains for the lead.

## The 14 modules in CUTTING-ROOM-PLAN section 3

| # | Module | Library exists? | CLI exists? | Web / desktop screen? | End-to-end evidence and remaining bounds |
| --- | --- | --- | --- | --- | --- |
| 1 | Pattern making and grading | Yes: `beta-pattern-core`, tools, grading, import/export | Yes: Pattern CLI and native adapters | Pattern drafting, measurements, grading, output; shared desktop UI | Base: Pattern UI report and native routes. Limited Seamly subset; fixed topology-changing operations need grading review. Not a full commercial PDS parity claim. |
| 2 | Digitizing paper patterns | Yes: `beta-pattern-digitize` | Yes: target/scan commands and routes | Digitize: target, photo, review, accept | Base: synthetic A3 photo through recovered pieces. No physical digitizer tablet or arbitrary calibration target support; camera/printer calibration needs operator verification. |
| 3 | Costing / consumption | Yes: cutroom costing + production report pack; plan cost comparison | Yes: `cutroom cost`; report packs and cost/width trials via typed queue tasks | Bundles quick estimate; richer packs remain CLI/library | Base: actual nesting and report tests. Quick model is uncalibrated unless a fitted model is supplied. Full merchandiser report configuration UI remains missing. |
| 4 | Marker making | Yes: core/nest/match/edit/dxf/export | Yes: import, nest, validate, export, batch | Editor, Compare; shared desktop | Base: Editor4 and desktop reports. Current task reuses a checked marker for floor release. Hardware calibration and commercial parity are unverified. |
| 5 | Cut order planning | Yes: plan and persistent queue | Yes: plan, pipeline, batch, queue | Cut plan; imported PO/colour hand-off added in Floor | Base plan tests; new Rust quantity hand-off. Width/cost alternatives and queue management still lack complete dedicated screens. |
| 6 | Fabric inspection | Yes: roll four-point scores, defect mapping | Yes: `roll inspect`, defect commands | Rolls defect entry and buyer policy; Floor receiving bridge | Base UI/route tests. Floor rechecks default policy and blocks unresolved consumed defects. No automatic defect-repair execution. |
| 7 | Shade and shrinkage | Yes: roll groups + core rules | Yes: roll group / pipeline | Rolls group and apply compensation; floor release gate | Base plus Rust: mismatched compensation/stock rejected. Replacement stock must match full lot/dye/shade identity. Lab results remain measured inputs. |
| 8 | Roll allocation and spreading plan | Yes: roll allocator + new floor release | Yes: roll allocate/spread; standalone floor `prepare` | Rolls allocator; new Floor Spreading | Rust/UI: audited roll sequence, per-roll plies, safe splice coordinates, allowance, stock reservation and end bits. Floor reserves an opened roll to one lay; verified remnants need new IDs. |
| 9 | Spreader and cutter output | Yes: export ISO; floor generic spreader writer | Yes: export ISO; floor export CSV/JSON/HTML | Editor cutter export; Floor spreading release files | Rust/UI: printable lay sheet, generic CSV/JSON and actual bytes. No vendor spreader dialect, speed/tension control or physical cutter test. |
| 10 | Ply numbering, bundling and ticketing | Yes: cutroom bundles/tickets, reused by floor | Yes: cutroom bundle/tickets/stickers; floor exports | Bundles; Floor A4 tickets, stickers, ply-order CSV | Base plus Rust/UI: exact physical identity, folded expansion, smaller actual-lay bundle regeneration. Reprint identity is stable; reprint audit/thermal workflow UI remains limited. |
| 11 | Re-cut and replacement | Yes: cutroom exact damage/scheduling, floor lifecycle | Yes: cutroom recut; floor damage/schedule/advance | New Floor Re-cuts, exact part selection, job-to-Editor hand-off, status buttons | Rust/UI: exact damaged quantities, duplicate prevention, original shade retained, forward status. Replacement nesting uses existing Editor. Physical replacement/return is an operator attestation; no automatic sensor or cancellation/reversal. |
| 12 | Cutting reports | Yes: cutroom measured/production reports; floor daily dashboard | Yes: cutroom report, queue Pipeline packs and floor consumption export | Bundles measured report; new Floor dashboard/tracking | Rust/UI: planned vs measured scope, actual issue/return, weighted efficiency, end bits, re-cuts and dates. Full buyer/monthly report-pack configuration remains CLI-only. |
| 13 | Link to sewing | Yes: existing QR contract + new floor issue ledger | Yes: floor issue and WIP CSV export | New Floor keyboard/USB scanner input | Rust/UI: registered bundled-only issue, exact payload validation, duplicate-safe same-line replay, conflicting-line rejection. BETA Time receiving importer, transfers and sewing receipt are not implemented here. |
| 14 | ERP/PLM link | Yes: floor CSV/XLSX intake/templates and consumption export | Yes: standalone floor import/template/export | New Floor preview, column mapping, row errors, reviewed import | Rust/UI: original PO/size/colour and packing-list templates, date/unit validation, idempotent external IDs, conflict rejection. CSV/JSON exchange exists; Tally/SAP posting, monitored outbox and acknowledgements do not. |

## V04-RESEARCH section 5.1: operator workflows

| Workflow | Library / CLI | Screen and end-to-end status |
| --- | --- | --- |
| Bump / slide to contact | Edit `bump`/`apply`; WASM bindings | Editor four directions/group bump. Base Editor4 tests/smoke; diagonal geometry tests. |
| Compact region / whole marker | Edit bounded compaction | Whole marker and selected groups exposed. Arbitrary region optimizer and persistent optimizer clusters remain incomplete. Base. |
| Swap / local reinsert | Edit exact two-piece swap and bounded fill-gaps | Editor selection/unplaced tray. Base; not general bundle-group neighbourhood optimization. |
| Flip / reverse / tilt | Core permissions + edit transforms | Editor keyboard/buttons; rules enforced. Base. |
| Temporary overlap / tolerance | Independent core and editor inspection | Invalid draft indication and export gate. No factory-authorized sacrificial-allowance overlap model. Base. |
| Split piece | Pattern operations | Pattern split with source retained and grading review. No automatic marker optimization that changes garment seams. Base. |
| Sections / splice lines / window cuts | Editor hard section metadata, roll splice zones, export tiling | Editor sections + new Floor safe splice zone form/sheet. Rust checks full-width clearance. Shared server/CLI section enforcement still needs lead review as documented by Editor4. |
| Multi-width trials | Batch and plan cost comparison | CLI/library only for complete stock/cost comparison. No dedicated alternative-selection screen. |
| Marker library / lay-rule reuse | Edit reuse with compatible job/rule checks | Local Editor library and JSON backup. No shared approval/revision library. Base. |
| Group / marry / repeated blocks | Edit selection/group actions, core blocks | Multi-select and group transforms; no full mandatory-sewing relationship workspace. Base. |
| Matching workspace | Core/match anchor rules; native nesting | Fabric/piece rule controls exist. No full fabric-photo overlay/residual-error workspace. Base. |
| Piece tray / counts / keyboard | Edit missing-copy model | Editor tray, numeric placement, rulers, shortcuts and undo. Base. |
| Preview / compare | Import PLT, nest, validate, reuse | Compare and best-valid retention exist. Searchable approval/checkpoint provenance remains partial. Base. |

## V04-RESEARCH section 5.2: merchandiser and cutting-room documents

| Document | Library / CLI | Screen / delivery / test status |
| --- | --- | --- |
| Style costing / consumption | Cutroom `production::build`, XLSX/CSV/PDF writers | Quick Bundles estimate plus native report packs; full metadata/material input UI absent. Base report tests, not re-marketed as calibrated quotation. |
| Width / ratio comparison | Plan cost request and width trials | Complete comparison remains CLI/library; no dedicated selection/reason screen. Base tests. |
| Lay / cutting ticket | Production report pack, floor sheets, export mini-marker | Floor lay HTML, roll sequence/plies/splices/allowances and A4 QR tickets. Rust/UI. Single combined lay-sheet QR/mini-marker composition remains in the existing native report pack. |
| Roll issue / end-bit register | Production reports and floor actual roll rows | Floor tracking measures issue/returns; consumption and spread exports. Rust/UI. No approval signature database, automatic remnant receiving or actual kg entry here. |
| Fabric reconciliation / cutting average | Cutroom measured and buyer report packs | Bundles report and Floor daily dashboard/CSV. Rust/UI. Monthly buyer filters and good-garment yield configuration remain native report inputs. |
| Bundle / re-cut ledger | Cutroom bundles/damage + floor sewing ledger | Floor exact part selection, re-cut scheduling/status, scan history and CSV/JSON. Rust/UI. Reprint audit, line receipt and transfer/reversal UI remain missing. |

## V04-RESEARCH section 5.3: ERP integration

| Connection | Implemented boundary | Missing / tested scope |
| --- | --- | --- |
| Excel / CSV first | Versioned original order and roll templates, XLSX/CSV preview/mapping/errors, duplicate-safe intake, consumption and WIP export | Rust/UI. Actuals are entered in tracking, not imported from a third template. No monitored pending-outbox folder or acknowledgements. |
| TallyPrime | Generic reviewed exchange files only | No Tally XML/JSON/ODBC adapter. Site-specific mapping and replay/reversal tests remain. |
| SAP | Generic reviewed exchange files only | No authenticated PO reader, material-document posting, retry/duplicate connector. |
| Other ERP / PLM | Documented versioned CSV/JSON contract and stable external IDs | No live connector, monitored-folder daemon or BOM revision service. WIP counts parts, not completed garments. |

The new work does not fill other agents' owned pattern/editor/native report
implementation gaps. Those remaining items are explicit lead follow-ups, rather
than a claim that all 14 modules have complete factory acceptance. See the floor
report for exact tests, screenshots, dependencies and integration lines.
