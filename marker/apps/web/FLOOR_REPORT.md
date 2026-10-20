# Cutting floor implementation report

Branch: `marker/astra-cutroom`, based on `marker/mvp` at `25e1233`.
All edits are under `marker/` in the assigned worktree. No other branch was
merged or pushed. The audit was committed first, followed by the requested
priority sequence. Apply the complete series: the shared Floor UI depends on
the later locale and manual assets.

## Delivered status

| Priority | Result |
| --- | --- |
| Audit | All 14 modules and every operator, report and ERP workflow in V04 section 5 are covered in `marker/docs/CUTTING-ROOM-STATUS.md`. Library/CLI/screens and test scope are separate. Remaining gaps are explicit. |
| Orders and rolls | Original version-1 CSV/XLSX templates; bounded preview; column mapping; row errors; unit/date/integer checks; literal Excel cells; atomic imports; stable external-ID replay and conflict detection. PO/colour quantities feed Cut plan; receiving and inspected Rolls registers have explicit hand-offs. |
| Spreading | Checked marker and stock allocation, roll sequence and per-roll plies, safe splice zones, each-end allowance, full lot/dye/shade identity, printable HTML lay sheet, documented generic spreader CSV/JSON, and ply-first sticker manifest. |
| Tracking / dashboard | Planned → spread → cut → bundled; measured issue/returns and actual plies; fewer plies regenerate bundles and block obsolete machine releases; immutable event history; date-filtered fabric, weighted efficiency, end bits, re-cuts and sewing issues. |
| Re-cuts | Exact physical part selection, duplicate damage rejection, shade-separated replacement jobs, job-to-Editor hand-off, Cut/Replaced states and ledger export. |
| Sewing | Keyboard/USB-scanner input, exact registered QR validation, bundled-only issue, duplicate-safe same-line scans, conflicting-line rejection and versioned BETA Time WIP CSV. |
| Hindi / Tamil | Main navigation and Floor labels, language switch and persistent preference. Both catalogues explicitly say machine-translated draft and require native operator review. Detailed instructions/errors remain English. |
| Manual | Self-contained printable HTML, 13 sections, full receiving-to-sewing flow, terminology, quantities/units, stock/actuals rules, re-cuts, CSV mapping, backup and troubleshooting. Open it in Floor, download it or print/save PDF through the browser. |

Rust owns every import rule, stock calculation, physical count, state transition,
scan check and export. TypeScript owns forms, presentation, file transport and
accepted-state persistence. The new standalone `beta-marker-floor` crate keeps
its requested `[workspace]` table. It provides a JSON CLI and the exact same
JSON boundary in local WASM. No new server route, online service, root npm
dependency or core contract field was added.

Existing tabs retain their styles. The small edits to `features.ts` and
`cutting-room.ts` only register the Floor tab, expose the order/register bridges
and capture floor state in native project snapshots. The desktop WebView uses
the same local WASM and native download adapter; no desktop window was opened.

## Validation

Every Cargo command used the assigned external `CARGO_TARGET_DIR` and
`CARGO_BUILD_JOBS=3`. WASM used the assigned matching wasm-bindgen 0.2.129.
Commands were run from the worktree root:

```text
npm install --cache <assigned npm cache>
cargo test --manifest-path marker/crates/beta-marker-floor/Cargo.toml -p beta-marker-floor
cargo clippy --manifest-path marker/crates/beta-marker-floor/Cargo.toml -p beta-marker-floor --all-targets -- -D warnings
npm run build:floor-wasm -w @beta/marker-web
npm run build:wasm -w @beta/marker-wasm
npm run build:pattern-wasm -w @beta/marker-wasm
npm run typecheck -w @beta/marker-wasm -w @beta/marker-web -w @beta/marker-desktop
npx vitest run marker
npm run build -w @beta/marker-web
cargo test --manifest-path marker/Cargo.toml --workspace --no-fail-fast
cargo clippy --manifest-path marker/Cargo.toml --workspace --all-targets
git diff --check
```

The floor Rust suite has **25 passing tests**, including the actual JSON
command flow from intake through measured production, bundle issue and every
export. It covers CSV/XLSX round trips, formulas, oversized files, mapping,
invalid dates/quantities, replay conflicts and atomicity, exact allowances,
splices, stock reservations, defects/shrinkage, folded physical parts, shorter
actual lays, stale tickets, forged backups, damage and scan identity. The final
regression keeps negative fabric variance numeric while protecting identifier
columns against spreadsheet formulas.

Focused `cargo clippy -p beta-marker-floor --all-targets -- -D warnings`, with
the standalone manifest path above, passes with **zero warnings**. The focused
tests and Clippy were rerun after the restart against the final source.

Vitest: **49 passed in 9 files**, including 8 new floor tests. They check accepted
state retention after Rust/storage failure, backup validation, snapshot isolation,
bounded file entry, scanner terminators and catalogue coverage. All three
TypeScript checks and all three WASM builds pass. The production web build passes
with the floor, marker and pattern assets included. Floor WASM is **3,700,483
bytes** before compression (about 910 kB gzip in the Vite report).

The full workspace test build was interrupted by the restart while compiling
the desktop dependency graph; it did not reach a test result. Full workspace
Clippy was waiting for that build lock and also did not complete. On the lead's
instruction, final verification uses the focused floor crate instead. The full
workspace and installed desktop flow still need verification after integration.

## One browser session and measured evidence

One hidden T3 collaborative preview tab was used and then closed. No visible
desktop or repeated browser window was opened. The native HTTP endpoint was
deliberately unavailable, so successful floor operations prove local WASM use.
Existing marker and pattern WASM assets were also built. The original shared
20 mm square fixture was loaded through the actual Editor file handlers.

The smoke uses real form/click handlers and file entry, with no mocked Rust API.
It imported an order CSV and an XLSX roll template, replayed the order, sent stock
to Rolls, passed PO quantities to Cut plan, prepared a lay, downloaded release
files, recorded Spread/Cut/Bundled, scheduled one exact damaged-part replacement,
issued a bundle and scanned it again. It checked all three language selections
and the manual's 13 sections. Wide-browser and 390 px mobile layouts had scroll width
equal to viewport width.

The browser exposed a missing `rust_xlsxwriter/wasm` feature: native XLSX tests
passed but browser creation initially trapped on the system clock. The feature
is now enabled and real browser XLSX generation/import passes. The smoke driver
also now waits for asynchronous form completion instead of submitting two actions
in the same JavaScript task. These corrections were made in the same hidden
session. There was no second browser launch.

Measured synthetic result: **2 plies**, marker **21 mm**, **10 mm at each end**,
planned cloth **0.082 m**. Measured issue **0.100 m** less returned **0.018 m**
equals **0.082 m**. Repeating the bundle scan retained **1 issue / 2 physical
parts**. One re-cut job contains exactly one damaged part. These are contract
smoke numbers, not factory efficiency or speed benchmarks.

Actual UI downloads: lay HTML **3,049 bytes**, spreader CSV **320 bytes**,
QR ticket PDF **8,808 bytes**, WIP CSV **316 bytes**. The WIP file was read back;
it contains one data row and preserves full lot/dye/shade identity.

Screenshots were saved and visually inspected. Local evidence, relative to this
worktree (ignored, not committed):

```text
marker/apps/web/.qa/floor/dashboard.png
marker/apps/web/.qa/floor/manual.png
marker/apps/web/.qa/floor/mobile-tamil-scan.png
marker/apps/web/.qa/floor/smoke-results.json
marker/apps/web/.qa/floor/lay-sheet.html
marker/apps/web/.qa/floor/spreader.csv
marker/apps/web/.qa/floor/tickets.pdf
marker/apps/web/.qa/floor/wip.csv
marker/apps/web/.qa/floor/floor-tests.txt
marker/apps/web/.qa/floor/floor-clippy.txt
marker/apps/web/.qa/floor/workspace-tests.txt
marker/apps/web/.qa/floor/workspace-clippy.txt
```

The repeatable browser driver is `src/floor-smoke.mjs`. Physical USB hardware,
printer stock, a spreader/cutter controller, replacement nesting and physical
completion were not re-tested in the browser smoke. Native tests cover the
replacement status transitions; inherited workflows are labelled Base in the
audit instead of being presented as newly tested.

## Dependencies and licences

No npm package was added. Cargo metadata was checked for every resolved floor
dependency. All offer the brief's allowed licence terms. New direct intake
families and notable reused packages are below. The complete 60-package table is
in `marker/crates/beta-marker-floor/DEPENDENCIES.md`.

| Package | Resolved version | Allowed licence used |
| --- | --- | --- |
| calamine | 0.31.0 | MIT |
| csv / csv-core | 1.4.0 / 0.1.13 | MIT option |
| quick-xml | 0.38.4 | MIT |
| zip | 4.6.1 and existing 7.4.0 | MIT |
| rust_xlsxwriter | 0.95.0 | MIT OR Apache-2.0 |
| wasm-bindgen / js-sys | 0.2.129 / 0.3.106 | MIT OR Apache-2.0 |
| futures-core / task / util | 0.3.34 | MIT OR Apache-2.0 |
| pin-project-lite / slab | 0.2.17 / 0.4.12 | MIT OR Apache-2.0 / MIT |
| codepage / atoi_simd / fast-float2 / debug_unsafe | 0.1.3 / 0.16.1 / 0.2.4 / 0.1.4 | MIT OR Apache-2.0 |
| encoding_rs | 0.8.42 | (MIT OR Apache-2.0) AND BSD-3-Clause |
| qrcode / flate2 / zlib-rs | 0.14.1 / 1.1.10 / 0.6.8 | MIT OR Apache-2.0 / MIT OR Apache-2.0 / Zlib |
| Local floor/core/roll and reused cutroom source | current repository | Apache-2.0 |

Existing serde, serde_json, anyhow, proc-macro/wasm support, ZIP compression and
numeric support use MIT, Apache-2.0, Unicode or Zlib options. `ryu` is used under
its Apache-2.0 option; `byteorder` and `memchr` under MIT. No prohibited copyleft
dependency is introduced into the floor graph. No vendor code, proprietary file
specification, private folder, generated dataset, lockfile or build output is in
the commit series.

## Exact lead integration lines

1. Add this member in `marker/Cargo.toml`, then remove the new crate's standalone
   `[workspace]` table:

   ```toml
   "crates/beta-marker-floor",
   ```

2. Keep the `rust_xlsxwriter` **`wasm` feature** on the floor dependency. Add this
   command to the desktop build before building/packaging the web assets:

   ```text
   npm run build:floor-wasm -w @beta/marker-web
   ```

3. The shared cutroom ticket writer has a wasm32 compile issue. This branch
   adapts its source at build time, without editing that other owner's file.
   Make the permanent one-line change in `beta-marker-cutroom/src/tickets.rs`:

   ```rust
   // old: *offset <= 9_999_999_999
   (*offset as u64) <= 9_999_999_999u64
   ```

   Then replace the generated `tickets` include in floor `lib.rs` with
   `#[path = "../../beta-marker-cutroom/src/tickets.rs"] pub mod tickets;`
   and remove floor `build.rs`. Bundles, re-cuts and the physical adapter reuse
   the exact existing sources by path to avoid pulling native nesting into WASM.
   A future lightweight shared crate can give those types one normal dependency.

4. Regenerate lockfiles. No root CLI registration or core model change is needed.
   The standalone JSON CLI is usable now; a `beta-marker floor` alias is optional.

5. After merge, rerun the floor and full workspace checks, all WASM builds,
   TypeScript/Vitest and the desktop installer build. Recheck native project
   restore/save and the installed Floor tab. Keep the ticket portability fix and
   workbook WASM feature when consolidating dependencies.

## Important operating limits

- Floor opened-roll reservation is intentionally conservative. Verified returned
  ends need new remnant IDs; there is no automatic multi-lay return/reallocation.
- Pending consumed defects stop release. Floor uses the default inspection
  policy and has no buyer-rejection override or automatic defect repair.
- Fewer actual plies need a note and regenerated labels. Extra plies require
  replanning. Frozen actuals have no silent edit/reversal workflow.
- Generic spreader exchange is not a vendor machine protocol. No hardware
  calibration is claimed.
- WIP is an issued-parts exchange for BETA Time, not a connected importer,
  sewing receipt, line transfer or ERP accounting post. Re-cuts and physical
  status transitions rely on operator records.
- Draft Hindi/Tamil need native review. Full translation and embedded Unicode
  PDF fonts remain outside this increment.
- Report-pack and width/ratio library/queue capabilities already exist, but their
  complete merchandiser/queue screens, Tally/SAP connectors and monitored outbox
  remain explicit gaps in the audit.
