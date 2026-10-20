# beta-marker-cutroom

Version 0.4 adds [merchandiser and cutting-room report packs](PRODUCTION_REPORTS.md)
in XLSX, CSV and A4 PDF, using one reconciled Rust result.

Rust operations for the steps after nesting and for early fabric costing. The
crate has no network calls or storage service. The CLI wires these APIs.

After cutting a lay (a stack of fabric plies), workers number each garment's
pieces so they sew together the same ply and shade. They group one size and part
into bundles, normally 20 pieces, and attach a ticket. A damaged piece gets a
replacement from the same shade group. Managers reconcile fabric and piece counts.
Merchandisers estimate consumption before they accept an order.

## Numbering and bundles

`bundle_lay(&Lay, BundleRules)` accepts physical cut parts per ply, plus inclusive
shade runs. The runs must cover plies 1 through the final ply, in order, with no
gaps or overlaps. A marker garment id identifies exactly one size. Its parts must
share that id; `slot` distinguishes repeated parts such as two pockets.

The input is deliberately small. Adapters from `MarkerResult`, `CutPlan` and
spreading records belong to integration work. For folded/tubular fabric, expand
physical layers and mirror states before constructing this input. Do not treat a
marker placement as one physical piece when it cuts two layers. On-fold pieces
cut one full piece. Use core `rules::layer_copies` in that adapter.

A floor bundle always contains one part, size, handedness, colour, shade and
contiguous ply range. These are mandatory rules. `pieces_per_bundle` is a maximum;
the last bundle of each shade run can be smaller. A bundle may end partway through
a ply when several marker garments use that part. The explicit piece list is the
source of truth. Floor bundle numbers start at `first_bundle` and increase in
stable size/part/handedness/shade-run order. The caller reserves non-overlapping
bundle ranges across lays.

For each size, garment ids are sorted. The sticker number is:

```text
offset + (ply - 1) * garments_of_this_size_in_marker + garment_rank + 1
```

Every part of that garment and ply gets the same number. `1-60` means garment
number 1 out of the final number 60 in this size's numbering run. It does NOT mean
a bundle contains plies 1 through 60. The offset applies equally to every size;
use zero for numbering within a lay. A complete physical identity also includes
order, lay, piece id, ply, marker garment, slot and handedness. Do not use the
printed number alone as a database key. Repeated parts can share a printed number.

## Tickets, stickers and WIP scans

`ticket_pdf` and `sticker_pdf` return PDF 1.4 bytes. `TicketPaper::A4` has ten
100 x 50 mm labels (2 columns, 5 rows). `Label100x50` has one label per page.
Both ticket and piece sticker formats use these sizes. Print at 100% scale, with
fit-to-page disabled. The writer draws vector QR modules, a four-module quiet
zone and Helvetica text. Medium error correction is used. It rejects identifiers
that force modules below 0.30 mm or text below 4.5 points. Human text uses ASCII;
unsupported characters appear as `?`. The QR preserves exact UTF-8 identifiers.
Full Unicode human text needs a future embedded-font implementation.

Scan payload is compact JSON, version 1. `ScanPayload::decode` validates it.

```json
{"v":1,"s":"TEE","o":"O1","z":"M","c":"Blue","l":"L1","h":"A","part":"front","m":false,"b":1,"p":[1,20],"q":20}
```

| Key | Meaning |
| --- | --- |
| v | Schema version (1); reject unsupported versions |
| s, o, z, c | Style, order, size, colour |
| l, h, part | Lay, inventory shade group, source piece id |
| m | Physical handedness (`true` means mirrored) |
| b | Sewing-floor bundle number, not `Placement.bundle` |
| p | Inclusive physical ply range |
| q | Physical quantity in this ticket; 1 on a piece sticker |
| piece, n | Optional exact `PieceKey` and printed garment number on a sticker |

For a piece sticker, `p` is `[ply,ply]`. `piece` has `piece_id`, `ply`, `garment`,
`slot` and `mirrored`; `n` is its number. For BETA Time / sewing WIP, use
`(o,l,b)` as the bundle identity, and the complete physical piece key for a sticker.
Scans have no URL and require no online lookup to read their basic fields. Treat
scan data as an identifier, never as authorization. Unknown keys are rejected;
new schema versions should get their own decoder. JSON is not signed.

## Damaged pieces and re-cuts

`Damage::record` checks that the exact piece exists in the bundle.
`schedule_recuts` validates the complete batch before changing any record. It
rejects duplicate damage ids and repeated records for one physical piece. It
groups open records by order, style, colour and inventory shade. Every output has
a `beta_marker_core::Job` with exactly the damaged normal/mirrored quantities,
no order multiplier, and `shade_group` plus source identifiers in `job.meta`.
An external stock allocator MUST supply fabric from that inventory shade. Core
`fabric.shade` is a geometric band constraint, not an inventory shade identity.

Re-cut jobs use open fabric so a single damaged piece does not produce paired
surplus. Original marker-coordinate defects are removed. Fabric shrinkage, grain,
repeat and other source rules remain. Match rules with a missing counterpart are
removed; surviving matching rules remain. The caller can supply new fabric defect
coordinates before nesting. Source piece ids and sizes must match the source job.

Status: `open -> scheduled -> cut -> replaced`. Scheduling assigns a job id;
`advance` accepts only the last two transitions. Persist the ledger in the calling
application. Scheduling a previously scheduled ledger does not schedule it again.
Use a new batch id for each later re-cut batch. Cancellation and repeated damage
to a replacement need a future explicit workflow.

## Cutting reports and ERP export

`lay_report` takes planned geometry, actual pieces/area and roll spreading records.
The width is a physical fabric width and the plies are physical layers; adapters
must expand folded accounting. All inputs use mm or mm2.

```text
planned length = (marker length + head/tail allowance PER PLY) * plies
actual length  = sum(issued roll length - reusable returned end bits)
variance       = actual length - planned length
marker efficiency  = planned outline area / marker fabric area * 100
cutting efficiency = actual cut outline area / actual consumed fabric area * 100
```

Actual use includes non-reusable waste. End bits are reusable inventory returns,
reported separately; do not subtract them twice in a spreading adapter.
`actual_pieces` and `actual_piece_area_mm2` must use the same scope. Include re-cut
output in both only if the underlying spreading records include its fabric use.
`recut_count` records requested replacements and is not added to the piece count.
A lay with no actual use has no actual efficiency. Impossible areas, NaN, infinity,
negative values, excessive end-bit returns and duplicate lays are errors.

`order_report` sums original lay measurements, checks the order id, and weights
efficiency by total fabric area, including orders with several widths. It does
not average percentages. `reports_csv` emits quoted identifiers, unit-labelled
columns and CRLF rows. `report_text` shows lengths in metres. CSV is the ERP/PLM
boundary for now; no xlsx dependency was added. Spreadsheet clients must import
identifier columns as text when identifiers can start with formula characters.

## Costing

`estimate(job, width, garments, model)` uses total demanded outline area after
shrinkage, including mirror quantities and size ratios. The caller supplies the
number of complete garments; it must equal the sum of a job's size ratio when
one exists. Jobs without a ratio may represent a complete pattern set, but only
the caller knows the real garment count. Mixed materials need separate jobs.

```text
marker length = total piece area / (fabric width * expected efficiency)
consumption per garment = marker length / garments
```

`features` extracts copy-weighted piece count, mean area, largest grain-allowed
width ratio, convexity (outline area / convex-hull area), and rotation freedom.
`fit` uses a six-coefficient ridge regression (five features plus intercept).
Features are scaled as documented in the Rust type. Prediction is clamped to
20-95%. `EfficiencyModel::default` is an explicitly uncalibrated heuristic;
load `calibration/model.json` for this branch's experimental fitted model. Keep
its provenance and held-out error with it. Refit on representative factory jobs
before using it for a commercial quotation.

`accurate_estimate` runs the real nesting engine with exactly two workers, a
caller-selected seed and search budget of at most 20 s, then independently checks
the result. Import, construction and validation add wall-clock overhead. Tests
use 0.05 s. Costing currently requires open fabric. Quick mode does not model
allowances, spacing, material mixing, roll loss, stripes/checks, defect maps,
shade bands or fusing blocks as features. It is not a feasible placement or an
optimality guarantee. Accurate mode applies the source job constraints supported
by the nesting engine; integration must re-test the v0.2 rules after merge.

`consumption_mape` computes mean absolute percentage error in fabric length,
not efficiency percentage-point error. See `calibration/README.md` and its JSON
summary for real measured results and limitations. The calibration example holds
out whole source patterns/tasks across both widths and ratios, avoiding leakage
from the same geometry into training and evaluation.

## Examples and proposed CLI (lead wiring)

Run cargo commands from the marker workspace, with `CARGO_BUILD_JOBS=2` and a
worktree-specific `CARGO_TARGET_DIR`. Set `BETA_DATA_DIR` to enable the optional
Lectra sample reader. Keep generated labels out of git.

```sh
cargo run -p beta-marker-cutroom --example demo -- crates/beta-marker-cutroom/output
cargo run -p beta-marker-cutroom --example calibrate -- crates/beta-marker-cutroom/calibration
cargo test -p beta-marker-cutroom
```

The demo writes A4 tickets, label-printer tickets, piece stickers, bundles, a
re-cut job and CSV/text reports. The main CLI now uses the `cutroom` command group documented below. Historical proposed commands:

```text
beta-marker bundle lay.json --bundle-size 20 --first-bundle 1 -o bundles.json
beta-marker tickets bundles.json --paper a4|100x50 --kind bundle|piece -o labels.pdf
beta-marker recut source-job.json bundles.json damage-ledger.json --batch R1 -o recuts.json
beta-marker cutting-report lay-actuals.json --format csv|text -o report.csv
beta-marker consumption job.json --width 1500 --garments 4 --model model.json
beta-marker consumption job.json --width 1500 --garments 4 --accurate --time 5 --workers 2
```

The lead should design atomic ledger persistence around the re-cut command.
No core contract change is required. Future adapters need an inventory shade id,
physical layers, actual fabric withdrawals/returns, and actual piece counts/area.

## Dependencies and licences

- New external crate: [`qrcode` 0.14.1](https://github.com/kennytm/qrcode-rust/blob/v0.14.1/Cargo.toml),
  **MIT OR Apache-2.0**, checked in its published manifest and source. Default
  features are disabled, so no image dependency or new transitive crate is added.
- `beta-marker-nest` is an existing workspace crate, Apache-2.0. Its existing
  sparrow dependency is MIT; jagua-rs is the project's documented unmodified
  MPL-2.0 exception. It is not vendored or changed here.
- Existing `anyhow`, `serde`, `serde_json`: MIT OR Apache-2.0; existing core:
  Apache-2.0. PDF writing, convex hull and fitting add no external dependency.
- Data attribution is in `calibration/README.md`. No vendor samples, private
  directories, network calls at runtime, or Lectra geometry are included.

## Integrated CLI and physical-piece adapter

```text
beta-marker cutroom bundle lay.json --pieces-per-bundle 20 -o bundles.json
beta-marker cutroom tickets bundles.json --paper a4 -o tickets.pdf
beta-marker cutroom stickers bundles.json --paper label -o stickers.pdf
beta-marker cutroom recut source-job.json bundles.json damage.json --batch-id R1 -o recuts.json
beta-marker cutroom report actuals.json --format csv -o report.csv
beta-marker cutroom cost job.json --garments 6 --model model.json -o estimate.json
beta-marker cutroom cost job.json --garments 6 --accurate 5 -o estimate.json
```

Bundle input is the typed cutroom `Lay`. The pipeline writes these as
`L001-cutroom.json`, etc. Printing supports A4 or one 100 x 50 mm label per page.
Re-cut output writes the updated ledger and scheduled jobs in one
JSON object (`ledger`, `jobs`), rather than changing the input ledger in place.
Report input is an array of measured `LayActuals`; output includes each lay and
an area-weighted order total, in JSON, CSV or text. Quick costing retains its
uncalibrated warning when no fitted model is given. `--accurate SECONDS` uses
exactly two nesting workers and accepts at most 20 seconds.

`integration::from_spread(job, marker, register, lay_spread, order, colour)`
validates marker geometry, checks canonical `rules::bundle_sizes`, expands every
placement with `rules::layer_copies`, and assigns unique repeated-part slots.
An on-fold half cuts one full part; an ordinary folded/tubular placement cuts
normal and mirrored physical parts. The caller must audit the complete spread
with `beta_marker_roll::validate_plan` first. The adapter rejects incomplete
plies and different inventory shades inside one ply, including splices.
Contiguous ply runs retain the full lot/dye/shade identity as a compact JSON array `[lot,dye_lot,shade_group]` so
identical shade labels from different lots cannot be confused.

The integrated pipeline prints all bundles, reserves unique floor-bundle numbers
across lays, and keeps garment numbering within each lay. Planned reports do not
pretend to be measured actuals. New tests cover folded/on-fold physical counts
and identical garment numbers across normal/mirrored parts.

Print page streams now use lossless PDF FlateDecode compression. Outlines and
QR modules remain vector paths; payloads, label dimensions and scan identity
are unchanged. The regression test inflates a page and checks its escaped text.
Direct dependency `flate2` 1.1 (resolved 1.1.10) is MIT OR Apache-2.0; it uses
`zlib-rs` 0.6.8 (Zlib) with default flate2 features disabled. Both packages already
belong to the workspace XLSX dependency graph; no new transitive package is needed.
