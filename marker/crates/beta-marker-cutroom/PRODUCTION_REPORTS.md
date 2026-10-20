# Merchandiser and cutting-room report packs

`production::build(&ReportInput) -> Result<ReportPack>` joins a source job, checked
cut plan, optional audited roll allocation, buyer/PO/style metadata and optional
measured actuals. It builds costing, per-lay tickets, width comparisons, roll/end
balances and ordered/planned/good-cut reconciliation. It rejects inconsistent
geometry, size totals, stock requests, issue/return balances and non-finite values.

The same full-precision result feeds `production::xlsx`, `production::pdf` and
`production::csv_files`. CSV returns a named file per table, with labelled units,
CRLF and escaped identifiers. Formula-leading text gets an import-safe apostrophe
in CSV only; JSON/XLSX keep exact identifiers. XLSX uses numeric cells, editable
assumptions, cached consumption formulas, frozen/filterable headers and wrapped
identifiers. A4 PDF repeats headers, splits wide registers into column groups,
and prints a ticket per lay. PDF Helvetica replaces non-ASCII human text with `?`;
CSV/XLSX preserve Unicode. Separate mini-marker/plot files retain cutter geometry.

Consumption denominators:

- Net marker: marker metres x plies / planned garments, including planned extras.
- Gross purchase: (marker + end/splice loss + purchase wastage) / ordered garments.
- Actual: measured issue minus reusable returns / measured good cut garments.

Each shows m/piece, m/dozen, kg/piece, kg/dozen and yd/piece. One yard is exactly
0.9144 m. GSM is grams per square metre. Estimated kg uses physical cloth width:
folded/tubular mass doubles laid-flat width once, while garment output keeps the
plan ratio unchanged. Actual kg uses each measured roll's width. Width / laid-flat
dia is also shown in inches, with 25.4 mm per inch.

Reusable end bits keep roll/lot/dye/shade identity and remain stock. Planned ends
are separate from measured reusable returns. Identified scrap is already inside
actual consumption and is not charged again. Missing actuals stay blank. Good-cut
garments exclude rejects and are distinct from counts of physical pattern parts.

`ReportInput` requires GSM, currency, metre price, lay price and purchase allowance;
none is guessed. `ReportMeta` carries PO, buyer, style/revision, colour, fabric,
report date, preparer and approver. A blank approver is not an approval record.
`OrderActuals` gives good-cut garments by size plus `RollActual` issue/return/scrap
movements. All units are in field names, and JSON retains unrounded calculations.

Queue Pipeline tasks can emit the entire pack through optional `ReportSettings`.
Direct callers can keep `ReportPack` JSON as a revisioned costing result and
regenerate any format offline. Width comparison remains a reproducible separate
`CostComparison`, not a change to measured production actuals.

New direct edges: internal `beta-marker-plan` (Apache-2.0), existing workspace
`rust_xlsxwriter` 0.95 (MIT OR Apache-2.0), and existing `zip` 7.x (MIT) for XLSX
test inspection. No new transitive package is needed. All calculations and writers
are Rust; they make no runtime network calls. Tests reconcile stock, quantities,
per-piece/per-dozen mass, yard conversion, formula caches and blank actuals.
