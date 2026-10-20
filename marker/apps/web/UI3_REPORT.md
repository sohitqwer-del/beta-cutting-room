# UI3 implementation report

Branch: `marker/ui3`, based on `31297f1`. UI checkpoint: `67ca81a`.
Merged `marker/integrate` at `55c0123` through merge commit `3b19b05`.
Its integration-owned tests were reported passing; its documented workspace failures included the stale UI fixtures and two nesting-owner issues. The UI fixtures are fixed here. No quality branch was merged.

The app now connects Pattern, Digitize, Editor, Cut plan, Rolls and Bundles. Compare and Help remain available. Pattern measurements, variables and drawing values re-evaluate through Rust WASM. Failed edits retain the accepted drawing. All sizes overlay in document size order; piece detail shows cut/stitch outlines, grain and notches. Seamly2D import, DXF-AAMA export and Pattern-to-Marker conversion work through the native engine.

The digitizer prints the original A3 target, uploads bounded PNG/JPEG bytes, returns recovered pieces and an overlay PNG, requires explicit review, and accepts selected pieces into Pattern or Marker. Roll entry/CSV/JSON import, scored defects, buyer-unit pass/fail, shade/shrinkage groups, audited allocation and end-bit tables work. The shared spread adapter carries complete lot/dye/shade identity into physical bundles. Manual shade runs remain available for an already-cut lay. Tickets/stickers, exact damaged-piece records, fabric costing and measured cutting reports work. Help includes the nine-step, printable day walkthrough.

The server stays on loopback, bounds JSON bodies at 50 MB and photos at 30 MB, and returns JSON errors. Every marker export now uses `beta_marker_export::export`, including its MIME and extension. Report downloads are TXT; explicit job/result JSON saves remain available. `PlanOptions.final_time_s` is preserved/defaulted to zero and exposed in Cut plan. Spreading recomputes marker lengths and end allowances in Rust; roll widths follow the shared laid-flat contract. Cutting reports recompute physical geometry, including folded-layer width/area.

Wire types are in `marker/packages/marker-wasm/src/types.ts` and the separate `pattern.ts`. Native and TypeScript tests share pattern, marker, roll, physical-lay and actuals fixtures. The new `pattern/crates/beta-pattern-wasm` is an independent small workspace pending the lead's member change. No source file outside ownership was edited directly; the integration merge is the only imported work.

## Verification

Every Cargo command used `CARGO_BUILD_JOBS=4` and the assigned marker or pattern `CARGO_TARGET_DIR`. Build output stayed outside the source tree. Fast integration tests also need `BETA_DATA_DIR`; it was directed into ignored `.qa/data` for this worktree.

Commands run from the repository root unless another directory is stated:

```text
npm install --cache <assigned npm cache>
npm run build:wasm -w @beta/marker-wasm
npm run build:pattern-wasm -w @beta/marker-wasm
npm run typecheck -w @beta/marker-wasm -w @beta/marker-web
npx vitest run marker
npm run build -w @beta/marker-web
cargo test --manifest-path marker/Cargo.toml --workspace --no-fail-fast
cargo clippy --manifest-path marker/Cargo.toml --workspace --all-targets -- -D warnings
cargo build --manifest-path marker/Cargo.toml -p beta-marker-cli
```

From `pattern/`, with its assigned target directory:

```text
cargo test --workspace
cargo clippy --workspace --all-targets
cargo test --manifest-path crates/beta-pattern-wasm/Cargo.toml
cargo clippy --manifest-path crates/beta-pattern-wasm/Cargo.toml --all-targets
```

Results after the integration merge:

- Vitest: **22 passed** in four files. Both TypeScript workspaces typecheck. Production Vite build passes.
- HTTP route tests: **12 passed**, including all nine export formats/MIME/extension, measurement edits, Seamly import, audited spread-to-bundle identity, tickets/stickers, costing, physical reports, malformed inputs and a real synthetic-photo scan.
- Marker full workspace: **220 passed, 2 failed, 4 ignored**. Both failures are in `beta-marker-nest`: `tests::examples_are_reproducible` (stale example JSON missing optional piece fields) and `v02::tests::no_order_keeps_distinct_sizes_in_distinct_garments` (old no-order bundle semantics disagree with the shared contract). All CLI, validator, export, plan, roll, cutroom and WASM tests pass. The failing files were left to their owner.
- Marker Clippy, all targets with `-D warnings`: passes with zero warnings. CLI build passes.
- Pattern workspace: **34 passed**, including its doc test. Standalone pattern WASM: **1 passed**. Both Clippy commands pass without warnings.
- `git diff --check`: passes. Lockfiles and generated output are excluded from commits.

The full marker test log is `marker/apps/web/.qa/marker-workspace-tests.txt`. The first attempt after merge lacked `BETA_DATA_DIR` for two integration tests; the final run supplies it, and those tests pass. The remaining two failures are the nesting issues above.

## Browser evidence and measured numbers

Native collaborative browser smoke tests used the built app and loopback server. A measurement edit changed the graded outlines with **zero API requests** during that edit. Four sizes rendered. The sample marker passed validation at about **0.430 m** and **62.5%** efficiency. A four-ply L plan and spread both accounted for **1.841 m**, including end allowances; its measured sample report reconciled **16 pieces**, one requested re-cut and zero fabric variance. After merging integration, a four-ply M plan with 0.1-second short/final budgets returned **1.801 m**. These are smoke measurements on the original teaching pattern, not quality benchmark claims.

The original synthetic **1600 x 1200** A3 photo recovered one piece using **14 codes**, with an **840-pixel-wide** overlay. Accept-to-Pattern produced a valid one-size document. A post-merge shared-adapter hand-off made four part/handedness bundles across two plies with shade identity `["LOT1","DYE1","A"]`. Its compressed ticket/sticker PDFs were **33,183 / 99,571 bytes**. The unified report downloaded as `marker-report.txt`. Arrow navigation moved selection and focus together. All eight tabs fit a **390 x 844** viewport without page-level horizontal overflow. Bundle and damage drafts survived reload.

Screenshots, relative to this worktree:

```text
marker/apps/web/.qa/screenshots/pattern.png
marker/apps/web/.qa/screenshots/digitize.png
marker/apps/web/.qa/screenshots/editor.png
marker/apps/web/.qa/screenshots/cut_plan.png
marker/apps/web/.qa/screenshots/rolls.png
marker/apps/web/.qa/screenshots/bundles.png
marker/apps/web/.qa/screenshots/compare.png
marker/apps/web/.qa/screenshots/help.png
marker/apps/web/.qa/screenshots/integrated_bundles.png
marker/apps/web/.qa/screenshots/day_walkthrough.png
marker/apps/web/.qa/screenshots/mobile_pattern.png
```

Details are in `.qa/smoke-results.json` and `.qa/post-merge-smoke.json`. The source photo and downloaded PDFs are in the same ignored QA directory. Screenshots are local review artifacts, not committed datasets.

## Dependencies and handoff

New direct server dependencies: `beta-pattern-core` and `beta-pattern-digitize` (Apache-2.0), `image` 0.25.8 and `base64` 0.22.1 (MIT OR Apache-2.0). The new WASM crate reuses `serde_json` and `wasm-bindgen` (MIT OR Apache-2.0) plus the local cores. No runtime npm dependency was added. Integration brings its documented `flate2` edge (MIT OR Apache-2.0) and existing `zlib-rs` (Zlib). Licences were checked in Cargo metadata and the integration report. No GPL/vendor code or private data is included.

Lead request: add `crates/beta-pattern-wasm` to `pattern/Cargo.toml` workspace members and remove the empty `[workspace]` table in the crate manifest at that time. Regenerate lockfiles after merging branches. No model change is requested. Resolve/merge the quality owner's nesting fixes, then re-run the full marker workspace and the browser pattern -> plan -> rolls -> bundles flow. Recheck per-piece flip/tilt, canonical no-order garment IDs, folded/on-fold parts, block metrics, final warm starts and full inventory shade identities after that merge.

Current limits: one A3 digitizer target in this UI; the documented Seamly2D subset; synchronous photo processing temporarily occupies the request loop; whole-ply spreading without automatic splice/defect repair; one active bundle lay with JSON backups and retained damage history; re-cut entry records damage while scheduling/status transitions remain in the CLI; costing is explicitly uncalibrated. Printer/cutter hardware calibration was not performed. Base-font PDF text is ASCII-limited; QR identifiers preserve UTF-8.
