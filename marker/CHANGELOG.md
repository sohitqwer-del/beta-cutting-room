# Changelog

## 0.6.0 - From scratch to cut

- Start a style with a blank sheet or one of seven editable basic blocks.
  Follow eight steps: Style, Pattern, Grade, Lay, Gaps, Stripes, Marker and Cut.
- Draw and edit patterns on a canvas. Grade from a size chart, view sizes
  together, and check seam lengths before making a marker.
- Set fabric lay, cutting gaps and stripe or check repeats. See the overlay
  and each match rule's measured error in millimetres.
- Rust now handles edge picking, line and curve offsets, line-to-line extend
  and trim, linked-piece grading, and missing fold marks. Browser workers
  can nest jobs with matching rules. Servers accept `/api/pattern/style`.
- The video tools support local Kokoro narration and recording your own voice.
  The website keeps its existing demo until a voice is chosen.
- Browser status is clearer and the service worker follows the app version.

Basic blocks need a fit review by a pattern master. Extend and trim currently
work between straight lines. In the browser, marker PLT, full-size PDF and DXF
need the desktop app or a server; tiled PDF works locally. Browser nesting is
weaker on big jobs. Quick Marker real-photo accuracy is untested. Hindi and
Tamil drafts need native review. The Windows installer is unsigned.


## 0.5.0 - 21 October 2026

- Redesigned the app screens, navigation and welcome. Added feedback on every screen, a new logo, and a new website. Desktop feedback links use a scoped opener.
- Added graded seam checks, RUL/JSON rule libraries, mirror and unfold tools, button and drill rows, and fold guides to Pattern.
- Improved folded-fabric nesting and physical-layer allocation. The public Teagan test rose from 42.8% to 62.9% useful yield. Folded layouts still trail open fabric.
- Added Quick Marker: review paper patterns from a photo with A4 or a BETA target, then make a marker. Export tiled print pages, a chalk guide or a calibrated projector layout. Real-photo accuracy has not been tested.
- Added local browser nesting in a WASM worker, an offline PWA, and a static app that needs no Rust server. Browser nesting is best for small jobs; desktop search is much stronger on large markers.
- Fixed audit findings in validation, imports, exports and production workflows. Added a repeatable demo video pipeline.
- Preserved exact locked coordinates across JSON round trips, including long decimal values.
- Remapped build paths, refreshed third-party notices and added a combined website/app build for GitHub Pages.

See [the release record](docs/RELEASE-0.5.0.md) for checks, artifacts and known limits. This is a fun project created with the help of modern AI tools.


Versions below describe development milestones. They do not imply that 0.1-0.3
were publicly shipped. Linked reports distinguish executed checks from limits.

## Unreleased

- Re-inspect rolls after inspection-policy/grouping changes, including changes
  made during a request. Preserve audited lay identity when switching bundle sources.
- Name cut-plan shortfalls instead of declaring incomplete orders ready.
- Reject duplicate roll CSV columns, accept uppercase JSON filenames, protect
  spreadsheet CSV text, and explain missing benchmark load evidence.
- Open explicit desktop feedback links with the official Tauri opener, scoped
  to the BETA repository and maintainer email. Copy the link when opening fails.
- Release build wrappers remap source/Cargo paths and scan the CLI executable.
  Older-glibc and repeatability gates are documented. See the
  [20 October audit](docs/IMPROVEMENTS-2026-10-20.md) for evidence and open issues.

## 0.4.0

- BETA Cutting Room desktop for Windows: native Rust operations, projects,
  autosave/recovery, file associations, progress, cancellation and NSIS packaging.
  [Desktop report](apps/desktop/DESKTOP_REPORT.md).
- Marker editing: bump/slide, grouping, local compaction, swapping, reinsert,
  rulers, shortcuts and a local marker library. General region optimization and
  shared approvals remain incomplete. [Editor](crates/beta-marker-edit/README.md).
- Drafting/grading tools, reviewed Seamly import/export, photo digitizing,
  actual-size PDF and plotter output. Seamly compatibility is a supported subset.
  [Pattern report](../pattern/crates/beta-pattern-core/REPORT.md).
- Cutting-floor intake, roll receiving, spreading sheets, measured actuals,
  bundles, exact re-cut identity and sewing scan ledger. ERP exchange is reviewed
  CSV/XLSX/JSON, without live SAP/Tally posting or sewing receipt integration.
  [Module audit](docs/CUTTING-ROOM-STATUS.md).
- Persistent queues, time tiers, report packs and fabric-width/cost trials.
  [Operations report](crates/beta-marker-plan/OPS_REPORT.md).
- Experimental cluster, learned-guidance and GPU work stays opt-in research.
  The production default remains CPU nesting with independent validation.
- Self-hosting: explicit IP bind address and bearer-token authentication.
  Non-loopback binding requires a token. HTTPS remains a reverse-proxy duty.
- Replaced the HTTP transport with bounded Hyper: early oversized-length
  rejection, streaming body limits, header/upload/write deadlines and connection
  and compute caps. Rejected bodies no longer use `tiny_http` cleanup.
- Install, privacy, self-hosting and licence documentation; portable Windows
  package tooling and Linux user-service examples.
- Deadline checks for locked-piece and folded-fabric nesting. Searches retain a
  complete checked marker on timeout or cancellation. Sparrow 0.3.0 and Jagua
  1.0.0 remain unchanged. [Engine evidence](crates/beta-marker-nest/LAUNCH_FIX.md).
- QA fixes: pending recovery preservation, local-only academic fixtures and
  grading-cache refresh on pattern changes. The QA report records remaining
  import-warning and partial re-nest limits. [QA](docs/QA-2026-10-19.md).

## 0.3 - cutting-room workflows and benchmark review

- Pattern/digitizer engines, roll inspection/shade/shrinkage, spreading,
  physical-layer bundles, labels, re-cuts and measured cutting reports.
- Browser screens and HTTP adapters for the expanded workflow.
  [UI report](apps/web/UI3_REPORT.md).
- More faithful Lectra orientation conversion, transparent exclusions, matched
  time comparisons and independent validation. Comparisons remain conditional
  on source-data assumptions; they do not establish commercial parity.
  [Benchmark protocol](docs/benchmarks.md).
- Planning/pipeline integration and queue/report work.
  [Integration report](crates/beta-marker-plan/INTEGRATE_REPORT.md).

## 0.2 - garment rules and planning

- Folded/tubular layouts, face flips and tilt, locked placements, warm starts,
  seeded runs, defects, shrinkage and fusing blocks.
- Stripe/check matching, shade bands, garment identities and one-way bundles.
- Cut-order planning, batch/width trials, HPGL marker comparison and grading.
- Marker DXF, generic ISO, XLSX and label exports.
  [Version 0.2 contract](SPEC.md#7-version-02-full-cutting-room-features).

## 0.1 - end-to-end marker planner

- DXF-AAMA/ASTM import into the Rust job contract.
- Sparrow on unmodified jagua-rs, garment orientation/copy rules and an
  independent geometry validator.
- SVG, HPGL and actual-size PDF output, browser editing, WASM bindings and a
  loopback HTTP API.
- Open test inputs and a reproducible benchmark workflow.
