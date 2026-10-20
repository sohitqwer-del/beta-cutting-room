# Cutting floor operations

This independent Rust 2024 crate adds versioned CSV/XLSX receiving, spreading
release sheets, floor status and measured fabric use, re-cut workflow and sewing
bundle issue. The new **Floor** tab uses this exact crate through WASM. It works
offline in the browser and in the desktop WebView. No new HTTP route is required.

The [printable manual](../../docs/CUTTING-ROOM-MANUAL.html) covers the complete
operator flow. The [module audit](../../docs/CUTTING-ROOM-STATUS.md) distinguishes
implemented screens from library/CLI capabilities and untested hardware.

## Build and test

Set `CARGO_TARGET_DIR` outside the source tree, `CARGO_BUILD_JOBS=3` and
`WASM_BINDGEN` to the matching wasm-bindgen CLI. At the repository root:

```text
cargo test --manifest-path marker/crates/beta-marker-floor/Cargo.toml
cargo clippy --manifest-path marker/crates/beta-marker-floor/Cargo.toml --all-targets -- -D warnings
npm run build:floor-wasm -w @beta/marker-web
npm run typecheck -w @beta/marker-web
npx vitest run marker
npm run build -w @beta/marker-web
```

The floor WASM output is copied into the existing ignored Marker WASM package
folder. Vite serves/copies it with the other local WASM assets. A viewer build is
still allowed without generated WASM, but Floor explicitly reports that its
engine is unavailable. The installer build must run `build:floor-wasm` before
packaging the web assets. No runtime npm dependency was added.

## CLI and JSON contract

```text
cargo run --manifest-path marker/crates/beta-marker-floor/Cargo.toml -- request.json
```

Omit the file to read stdin. One response is written to stdout only after the
request succeeds. Errors go to stderr and exit 1. Do not redirect output over
the input file. Write a new file and retain the previous accepted state.

```json
{
  "op": "dashboard",
  "state": {"version":1,"orders":[],"register":{"rolls":[]},"lays":[],"issues":[]},
  "data": {"date":"2026-10-19"}
}
```

Response: `{state, data, download}`. `download`, when present, contains `name`,
`mime` and a byte array. The client saves `state` atomically before showing a
successful mutation. The web controller leaves its old snapshot intact if Rust
or local storage rejects the change. A backup restore replaces, rather than
merges, the active floor. Desktop project snapshots include floor records.

| Operation | Data |
| --- | --- |
| `validate` | No data. Checks a restored snapshot. |
| `template` | `kind: orders|rolls`, `format: csv|xlsx`. Returns download bytes. |
| `import` | `kind`, `format`, file `bytes`, optional `mapping` (canonical column to source header). Re-parses and validates the file before committing. |
| `sync_register` | A complete existing `RollRegister`. Merges inspection changes only into unreserved rolls; reserved stock cannot change. |
| `quantities` | `order`, `style`, `colour`, current `job`. Returns size quantities for the existing planner; missing pattern sizes fail. |
| `prepare` | `Prepare` fields in `floor.rs`: immutable job/marker, lay/order/colour/date/table, plies, end allowance, selected roll IDs, optional safe splice zones and overlap, bundle size. |
| `advance` | `id`, next `status`, `date`, `operator`, `note`, optional measured `actuals`. |
| `damage` | `lay`, damage `id`, floor `bundle`, exact `piece` key, `reason`. |
| `schedule` | `lay`, new `batch` id. Uses the existing exact re-cut job builder. |
| `recut_advance` | `lay`, damage `id`, `status: cut|replaced`. Records physical operator attestation. |
| `issue` | Exact JSON `payload`, stable `event_id`, `date`, `line`, `operator`. Returns `added: false` for an identical same-line replay. |
| `dashboard` | Calendar `date`. |
| `export` | `kind: backup|consumption|wip|lay_sheet|machine_csv|machine_json|sticker_order|tickets|stickers|recuts`. Lay exports also need `lay`. |

WASM exposes `floor_request(json)` and `floor_preview(bytes, format, kind,
mapping_json)`. Preview never mutates saved records. This boundary is bounded to
50 MB per command, 5 MB per intake file, 20 MB expanded XLSX, 10,000 input rows,
64 columns and 4,096 bytes per cell. XLSX formulas, unsupported cell types,
wrong sheet names, oversized cell coordinates and unknown template versions
fail explicitly. CSV is UTF-8 and supports quoted commas/quotes/line endings.

## Receiving templates

Orders sheet/CSV columns:

```text
schema_version,external_id,purchase_order,buyer,style,colour,size,quantity,due_date
```

Rolls sheet/CSV columns:

```text
schema_version,id,lot,dye_lot,shade_group,ticket_length_m,measured_length_m,usable_width_mm,length_pct,width_pct
```

Schema version is `1`. Dates are ISO calendar date text; blank order due dates
are allowed. Garment quantities are positive integers up to one million. Roll
lengths are metres, width is laid-flat usable mm, shrinkage is percentage points.
Unknown columns are retained in preview but do not become domain fields. Map
names explicitly; unit conversion is never inferred. Identical ID/content
replays add nothing. Changed content for an existing identity fails the whole
import. XLSX cells are written as literal text. CSV exports prefix formula-like
text with an apostrophe; import identifier fields as text in downstream systems.

Packing lists do not assert a completed inspection. The receiving-to-Rolls
bridge preserves existing measurements rather than silently overwriting them.
Bring the inspected register back with **Use inspected Rolls register**.

## Physical release and accounting

Marker geometry is independently validated. The floor recomputes length and
area, checks stock shrinkage compensation, uses the existing roll allocator,
audits stock balance and converts the spread into physical cut parts using the
existing integration adapter. Folded/on-fold expansion is not duplicated.

Safe splice zones use table metres, including the head allowance. They must
clear every effective cut boundary, including fusing blocks. The full zone is
checked conservatively against piece x extents. The allocator checks overlap
coverage and physical roll sequence. Pending consumed defects stop floor
release, even when a more advanced defect-repair workflow might salvage them.
Default four-point inspection policy is used here; this UI does not override
buyer rejection to force a release.

An opened roll is reserved for one floor lay. Its unconsumed remainder is
reported, not automatically returned to available stock. Receive a physically
verified remnant under a new roll ID with original lot/dye/shade. The more
general existing roll allocator supports shared multi-lay stock; this floor
ledger deliberately avoids pretending that measured returns have occurred.

Actuals contain `plies`, `rolls: [{roll_id, issued_m, returned_m}]`, and `note`.
Issue cannot exceed stock; return cannot exceed issue. Every opened roll must
be accounted for. Net use must cover actual marker passes. Fewer actual plies
require a note, regenerate the physical bundles/stickers and invalidate old
machine release exports. Reprint before bundling. More plies than released
require replanning. Frozen actuals/status history prevent silent corrections.

Dashboard scope is explicit: lays by scheduled date; issues by issue date.
Variance compares only measured lays with their own plans. Marker efficiency
is area weighted. Cutting yield uses actual passes and actual roll width,
including physical folded layers once; damaged parts still count as geometric
cut area. It is not good-garment yield. Re-cut fabric belongs to its own lay.

## Generic spreader contract

JSON envelope `format = beta-marker.spreader/v1` contains a note and `rows`.
CSV carries `schema_version = 1` on every row. Both formats contain the same
execution sequence:

```text
sequence,lay,table,ply,roll,lot,dye_lot,shade,
roll_start_m,table_start_m,length_m,
head_allowance_mm,tail_allowance_mm,splice_overlap_m,direction,face
```

Ply and sequence start at 1. Roll offset is measured from the start of that
received roll. Table offset starts before the marker head allowance. Direction
is `forward`; face is `up`. Incoming splice segments report the overlap; the
outgoing segment includes it in consumed length. Summary rows count contributing
plies separately from uninterrupted whole plies. Sticker order is ply, size,
garment, part, repeated slot, handedness.

This is a generic documented exchange, not a machine controller dialect. There
is no speed/tension control or controller command. Factory adapter review and
hardware dry run are required. No proprietary specification was used.

## Re-cut and sewing identity

Re-cuts reuse `Damage::record`, `schedule_recuts` and `Damage::advance` from the
cutroom crate. Duplicate physical damage is rejected. Jobs retain source rules
and full inventory shade identity. Preparing a re-cut marker on mismatched
lot/dye/shade stock fails. Marking a replacement Cut/Replaced is a human record;
the app does not infer physical completion from a computed marker.

Sewing uses the existing version-1 `ScanPayload`. It checks the whole registered
bundle, not only `(order, lay, number)`. Unknown, altered or piece-sticker
payloads fail. A lay must be Bundled. Same-line repeat scans are no-ops; another
line is a conflict. WIP CSV retains event ID, date, line, operator, PO/style/
colour, lay/bundle/size/part/handedness/shade, ply range and issued physical part
quantity. This is an exchange for BETA Time; a receiving importer and factory
field mapping remain separate. It does not count finished garments or post ERP
stock transactions.

## Integration and licences

The crate retains its own `[workspace]` as requested. The lead should add
`"crates/beta-marker-floor",` to `marker/Cargo.toml` members, then remove this
crate's `[workspace]`. Its explicit dependency versions can remain or be moved
to workspace dependencies. Regenerate lockfiles; none are committed here.

To avoid importing native nesting into WASM, `bundles.rs`, `recuts.rs` and
`integration.rs` compile directly from the existing cutroom source via `#[path]`.
There is one implementation. The ticket writer is also reused, through a tiny
build-time portability adaptation. Its existing PDF xref comparison infers the
10-digit limit as `usize`, which cannot compile on wasm32. The lead can change
this exact comparison in `beta-marker-cutroom/src/tickets.rs`:

```rust
// Before
*offset <= 9_999_999_999
// After
(*offset as u64) <= 9_999_999_999u64
```

Then replace the generated `tickets` include with a direct path module and
remove `build.rs`. A later lightweight shared floor-contracts crate can replace
all these source inclusions. No core model or CLI registration change is needed
for the standalone driver or the web tab. A main-CLI alias is optional.

Dependencies were checked using `cargo metadata`. New direct external families:
`calamine 0.31` (MIT), `csv 1.4` (MIT option), `quick-xml 0.38` (MIT), and
`zip 4.6` (MIT, already in the wider project graph). Reused direct crates:
anyhow, serde, serde_json, rust_xlsxwriter, wasm-bindgen, qrcode (MIT OR Apache-2.0),
flate2 (MIT OR Apache-2.0 with zlib-rs/Zlib), and local core/roll (Apache-2.0).
The spreadsheet writer's `wasm` feature is required for browser clock access;
its js-sys and futures-core/task/util, pin-project-lite and slab dependencies
all offer MIT or Apache-2.0 terms. Without that feature, native template tests
pass but Excel creation traps in a browser. The actual browser smoke caught
and verified this configuration fix.
New intake transitives include csv-core (MIT option), calamine's codepage,
atoi_simd, fast-float2, debug_unsafe (MIT OR Apache-2.0), encoding_rs
((MIT OR Apache-2.0) AND BSD-3-Clause). The complete resolved graph is recorded
in the implementation report. No GPL/vendor code, runtime network dependency,
new npm package or private sample is included.
