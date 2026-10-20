# BETA Cutting Room

One offline app for Pattern, Digitize, Editor, Cut plan, Rolls, Bundles, Compare and Help. Geometry, grading, nesting, inspection, allocation, numbering, costing and reports run in Rust. TypeScript handles forms, file entry and drawing.

## Build and run

Install root workspaces with the assigned npm cache. Set `CARGO_TARGET_DIR` outside the checkout, `CARGO_BUILD_JOBS` for the shared machine, and `WASM_BINDGEN` to a matching wasm-bindgen CLI. Add the Rust `wasm32-unknown-unknown` target if needed.

From the repository root, build the marker bindings with the marker target directory selected:

```sh
npm run build:wasm -w @beta/marker-wasm
```

Select the assigned pattern target directory, then build the standalone pattern bindings:

```sh
npm run build:pattern-wasm -w @beta/marker-wasm
npm run build -w @beta/marker-web
```

Restore the marker target directory. From `marker/`, run:

```sh
cargo run -p beta-marker-cli -- serve
```

Open the printed loopback address. The server binds only to `127.0.0.1`. All assets, WASM, PDFs and operations stay on the computer. For development, `npm run dev -w @beta/marker-web` proxies `/api` to the local server; `BETA_MARKER_API_URL` can select another loopback port.

## Layout and design

- **Shell:** a left sidebar groups the screens (Design, Marker and planning, Cutting floor, Reports and help). A top bar shows the current screen, server status, language, theme, server token (browser only) and About. A workflow bar links the eight steps: pattern, marker, cut plan, rolls, spread, cut, bundles, sewing. The address keeps the screen (`#/marker`, `#/rolls`, …), so a reload returns to it. Below 900 px wide the sidebar becomes a menu.
- **Home:** open a file or project, start a new pattern, open the demo, continue the current marker or saved pattern draft, desktop recent files, and the four most common tasks.
- **Reports:** one page that lists every report and where it is made.
- **Design system:** `src/style.css` holds the tokens (type scale, spacing, colours) for light and dark themes. Dark follows the system unless the user picks a theme. Drawing canvases use the same tokens. Fonts are local system fonts only; no web fonts or CDN assets are loaded. Text meets WCAG AA contrast. Tables with more than eight rows get a row filter and sticky headers. Print styles give clean A4 pages for cut plans, lay sheets and reports.
- **Branding:** `src/brand.ts` (original SVG mark, product name, version from `package.json`, credits) and `public/favicon.svg`.
- **Seamly2D import warnings (QA-004):** the Pattern screen shows `Document.meta.seamly_import_warnings` in a persistent panel. It stays through edits, saves, reloads and exports. Sending the pattern to Marker copies the warnings to `Job.meta.seamly_import_warnings`, and the Marker editor shows them under the statistics and after each export.
- **Server token:** when `beta-marker serve` requires a token, the browser asks for it once per tab (`sessionStorage`, never in the URL) and sends `Authorization: Bearer …` on every `/api` request. The desktop app does not use it.

## Screens

- **Pattern:** open beta-pattern JSON or import the supported Seamly2D subset with optional measurements. Edit a measurement, variable or drawing formula. Rust re-evaluates all sizes before accepting an edit; failed edits retain the accepted drawing. Review the size overlay, cut and stitch lines, grain and notches. Save JSON, export all sizes as DXF-AAMA, or send a size ratio to Editor. The original teaching bodice needs a fit test before production. WASM works without the HTTP server; server evaluation is a fallback.
- **Digitize:** print the original single-sheet A3 calibration target at 100%, check its scale, upload PNG/JPEG up to 30 MB, review the overlay and warnings, select pieces, assign a size and explicitly accept them to Pattern or Marker. Other board sizes remain available through the digitizer CLI.
- **Editor:** review fabric and piece rules, move/rotate/lock pieces, validate every change in WASM, and nest through the server. Exports: SVG, PLT, PDF, ISO, DXF, XLSX, labels PDF, mini-marker PDF and text report. ISO uses the crate's generic default profile; calibrate physical machines separately.
- **Cut plan:** enter order quantities, ply and marker limits, over-cut, short/final search time and end allowance. Review each saved lay marker before cutting. Lay cost is displayed as an estimate.
- **Rolls:** import the CSV template or register JSON, edit receiving/shrinkage data, add scored defect observations, and inspect against a buyer limit in the correct area unit. Display both scores and compatible lot/dye/shade/shrinkage groups. Apply group shrinkage before nesting. Allocate one group to the current cut plan or valid editor marker. Rust recomputes lengths, includes both end allowances, keeps the roll contract’s laid-flat width, and validates stock balance. Failed rolls are excluded unless explicitly approved. The UI uses whole plies; no automatic splicing or defect repair is implied.
- **Bundles:** send an audited spreading lay with its full inventory shade identity, or choose a saved lay or editor marker, enter actual shade runs and identifiers, expand physical layers in Rust, make bundles, and print A4 or 100 x 50 mm tickets/stickers. Record exact damaged pieces and save the ledger. Reconcile issued fabric, returned end bits, cut pieces and actual cut area. Report geometry is recomputed in Rust, including folded-layer area. Costing uses an uncalibrated heuristic and shows its limitations. One active bundle lay is retained; save JSON backups before replacing it. Damage history is retained across lays.
- **Compare:** import PLT, validate and re-nest the same pieces and width. Show savings only when both layouts validate.
- **Help:** explains garment terms and includes a printable one-page day-in-the-cutting-room walkthrough, from pattern through sewing.

Buttons, form labels and visible focus support keyboard use. A skip link jumps to the main content. In the sidebar, arrow keys, Home and End move between screens. Pattern, roll, bundle and re-cut drafts are stored in this browser. Save JSON backups before clearing browser storage. Editor/plan state still uses explicit job/result saves.

## HTTP contracts

All failures return JSON with an `error` message. JSON bodies are limited to 50 MB; photo bodies are limited to 30 MB before reading. Static files reject encoded traversal, Windows absolute paths and links outside the web root. Nesting and planning share a worker permit; concurrent work receives 409. Photo scanning is synchronous and temporarily occupies the request loop.

Existing routes: `GET /api/health`, `GET /api/nest/status`, and `POST /api/import`, `/api/nest`, `/api/plan`, `/api/validate`, `/api/export?format=...`. Import accepts raw DXF or PLT. Export accepts `{job,result,options?}` and returns real bytes for every format.

New POST routes:

| Route | Input | Result |
|---|---|---|
| `/api/pattern/starter` | `{}` | Original document |
| `/api/pattern/import` | `{pattern,measurements?,size?}` XML strings | Document and warnings |
| `/api/pattern/grade` | `{document,sizes?,tolerance_mm?}` | Graded pieces by size |
| `/api/pattern/export-dxf` | Same pattern request | DXF bytes |
| `/api/pattern/to-job` | `{document,fabric,ratio,tolerance_mm?}` | Marker job |
| `/api/pattern/from-job` | Job | Fixed-geometry document |
| `/api/digitize/target` | `{}` | Single A3 target PDF |
| `/api/digitize` | Raw PNG/JPEG bytes | Pieces, scan report, data URL of overlay PNG |
| `/api/roll/inspect` | `{register,settings?,grouping?}` | Inspection and groups |
| `/api/roll/spread` | `{register,lays,settings?}` or `{register,markers,shade_rule?,eligible_roll_ids?,settings?}` | Independently audited spreading plan |
| `/api/cutroom/lay` | `{job,result,id,order,colour,plies}` | Physical lay; rejects unchecked/unassigned markers |
| `/api/cutroom/from-spread` | `{job,result,register,spread,lay_id,order,colour}` | Shared physical-layer and full shade identity adapter |
| `/api/cutroom/bundles` | `{lay,rules?}` | Bundles |
| `/api/cutroom/tickets` and `/stickers` | `{bundles,paper?}` | PDF bytes |
| `/api/cutroom/recut` | `{id,bundle,piece,reason}` | Damage record |
| `/api/cutroom/report` | `LayActuals` | Cutting report |
| `/api/cutroom/report-marker` | `{source,issued_mm,end_bits_mm,actual_piece_area_mm2,actual_pieces,recut_count,end_allowance_per_ply_mm}` | Report from validated source geometry |
| `/api/cutroom/cost` | `{job,garments,price_per_m}` | Consumption and fabric cost per garment |

Wire types are in `packages/marker-wasm/src/types.ts` and `pattern.ts`. Roll lengths use metres; marker geometry uses millimetres. The API requires explicit shade runs for bundles; verify these against physical spreading.

## Verification

```sh
npx vitest run marker
npm run typecheck -w @beta/marker-wasm -w @beta/marker-web
npm run build -w @beta/marker-web
# From marker/, with assigned build environment:
cargo test --workspace
cargo clippy --workspace --all-targets
# From pattern/, with its assigned build environment:
cargo test --workspace
cargo clippy --workspace --all-targets
cargo test --manifest-path crates/beta-pattern-wasm/Cargo.toml
cargo clippy --manifest-path crates/beta-pattern-wasm/Cargo.toml --all-targets
```

Shared fixtures cover the v0.2 model, native pattern document, receiving register, marker, physical lay and report. Route tests cover all exports, grading/editing/import, allocation with end allowance, bundle/sticker PDFs, damage, reports, costing, malformed inputs and photo limits. A synthetic original A3 photograph verifies real recovered pieces and overlay PNG; setting `BETA_UI_SMOKE_DIR` writes it there for browser smoke testing. Generated screenshots and logs belong in ignored `.qa/`, not in git.

New direct server dependencies: beta-pattern-core and beta-pattern-digitize (Apache-2.0), image 0.25 (MIT OR Apache-2.0), base64 0.22 (MIT OR Apache-2.0). No new runtime npm packages. Existing WASM bindings use serde, serde_json and wasm-bindgen (MIT OR Apache-2.0). No vendor/GPL code, private data, remote assets or telemetry is included.

Merged integration commit `55c0123`: HTTP marker exports use its unified `export` dispatcher, returned MIME and filename extension. Report output is plain text; job/result JSON saves remain available. Planner options preserve `final_time_s`, defaulting to zero.
