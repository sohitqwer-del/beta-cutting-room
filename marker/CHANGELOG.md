# Changelog

Versions below describe development milestones. They do not imply that 0.1-0.3
were publicly shipped. Linked reports distinguish executed checks from limits.

## 0.4.0 - launch candidate

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
