# Desktop implementation report

Branch: `marker/desktop`, based on `marker/mvp` at `de68343`. App/package version: **0.4.0**. All source changes are under `marker/apps/desktop/` and the small shared web transport/native integration. The Marker workspace manifest is unchanged. No branch was merged or pushed.

## Implementation

The Tauri 2 Rust shell calls the existing BETA marker, pattern and digitizer crates directly. Every `/api/*` route in the current `serve.rs`, including the newer drafting tools, is mirrored by the native dispatcher. Browser mode retains HTTP. The frontend keeps its existing tabs and styles. Native mode uses Tauri IPC and real export bytes; no localhost HTTP server runs.

Long searches run on background threads supervising a solver mode of the same executable. The engine has no cooperative cancellation API. Cancel kills and waits for that worker. A Windows Job Object kills it if the parent crashes. One permit covers nest/plan/pipeline searches. Events carry elapsed seconds, best feasible length and efficiency. Rule-based nesting/planning supply elapsed time when their underlying crates do not provide intermediate improvements.

Native features include Windows open/save dialogs, supported file associations, drag-and-drop, 15 recent files, folder-based `.betaproj` projects, five-second idle autosave with a previous good copy, explicit crash recovery and opening saved PDFs in the default viewer for printing. Projects capture the current pattern and floor drafts, editor job/marker, cut plan and settings. Live panel snapshots are saved independently of the WebView cache. A Rust-held one-shot project payload transfers recovery data across renderer reloads, including projects above browser session-storage quotas. Files are bounded at 50 MB. They are single active projects, not a multi-job document library. Native `.betapattern` opens as JSON through the existing pattern workflow.

Settings persist the default import fabric width, a dimensions summary in mm/cm/inch, plotter units per mm, cutter ISO profile, nesting worker count and English. The planner keeps its current two-worker budget. Geometry fields retain their labelled units. Hindi and Tamil are placeholders. Saved marker and pattern PLTs use the chosen plotter units. ISO uses the chosen generic profile.

The additional native pipeline API selects one compatible inspected roll group, applies its shrinkage, plans, allocates whole plies with end allowances, maps cloth defects, validates and makes physical lays/bundles. Existing export and ticket commands produce print files. The normal interface exposes these stages in its existing tabs; the composite pipeline is available through IPC.

## Build and measurements

Build platform: Windows x64, Rust 1.98.1 GNU toolchain. The package contains WebView2Loader.dll and no MinGW runtime DLL is imported. `npm run build -w @beta/marker-desktop` builds both WASM modules, the shared UI, the Rust executable and a current-user NSIS installer with the original B icon. The installer uses zlib compression and explicitly skips WebView2 download.

Installer: `$CARGO_TARGET_DIR/release/bundle/nsis/BETA Cutting Room_0.4.0_x64-setup.exe`. Final size: **12,645,059 bytes (12.06 MiB)**. SHA-256: `d34137476a0a1a0e0823ebe5331b3ae8f5c752ff276c3e28ca6eb8f67c99fc6b`.

Exactly one UI smoke run passed. It loaded the original demo through the real button, requested a five-second nest, independently validated the result, exported PLT through the real export button, started and cancelled a second search, and closed the app. No app window was left open. Measured Rust entry to frontend bootstrap: **1,771.82 ms**. Nest plus PLT save: **7,259.40 ms**; the engine reported **6.625 s**, including construction/checking outside the five-second search budget. The marker was **1,231.992 mm** long at **75.7906%** efficiency, with all eight placements valid. Second-search start through cancellation completion: **472 ms**. These are single-run observations on the shared machine, with OS caches left intact.

The subsequent project-size recovery correction is covered by Rust staging/round-trip tests and TypeScript payload tests, including data above 5 MB. The requested one-run UI limit was preserved; that correction did not receive a second GUI launch. Smoke evidence and the PLT are in the assigned external QA directory, under `smoke-0.4.0/`.

Validation commands run with the assigned Cargo target, four build jobs, npm cache and wasm-bindgen tool:

```text
npm install --cache <assigned npm cache>
npm run build -w @beta/marker-desktop
npm run typecheck -w @beta/marker-web -w @beta/marker-desktop
npx vitest run marker
cargo test --manifest-path marker/Cargo.toml --workspace --no-fail-fast
cargo clippy --manifest-path marker/Cargo.toml --workspace --all-targets
cargo test --manifest-path marker/apps/desktop/src-tauri/Cargo.toml --workspace
cargo clippy --manifest-path marker/apps/desktop/src-tauri/Cargo.toml --workspace --all-targets -- -D warnings
node marker/apps/desktop/scripts/check-licenses.mjs
```

Marker workspace: **231 passed, 4 ignored, 0 failed**. Workspace Clippy finished with no warnings. Both TypeScript checks and the production frontend build pass. Vitest: **35 passed in 7 files**. The Windows GNU licence gate passes for **329 Cargo packages**; the MSVC graph was also checked during initial setup. The headless native executable/loader/worker-pipe preflight passed. Native command tests cover all exports, pattern/roll/cutting-room routes, a real synthetic photo, pipeline, actual process termination, file replacement backups and projects. Final native workspace: **28 passed, 0 failed** (25 command/storage/pipeline/adaptor tests, two HTML compatibility tests and one Option helper test). Strict Clippy finishes with zero warnings; both native command exit records are zero. The final build exit record is zero. PLT smoke output: **1,412 bytes**, with the real HPGL initialization header.

The installer targets Windows x64 through NSIS and installs for the current user. It is unsigned. WebView2 is a system prerequisite: the installer explicitly skips network bootstrap. Windows 10 machines without WebView2 need Microsoft's separate standalone offline runtime installer before launch. Windows 11 normally has the runtime. No Windows 10 VM or physical printer/cutter was available for this task.

## Dependency licences

The new direct Tauri crates, plugins, support crates and external npm CLI are Apache-2.0/MIT or MIT. BETA crates and the two original compatibility adapters are Apache-2.0. `windows-sys`, html5ever, markup5ever_rcdom and xml5ever are MIT OR Apache-2.0. No npm runtime package was added. NSIS 3.13 core and the selected zlib compressor use zlib/libpng terms. Its optional CPL-1.0 LZMA compressor is excluded. WebView2 is a separately installed Microsoft Windows prerequisite.

The initial unmodified Tauri graph included MPL-only `dom_query` dependencies (`selectors`, `cssparser`, `cssparser-macros`, `dtoa-short`) and `option-ext`. This conflicts with the common brief. Original narrow Apache-2.0 adapters replace those interfaces. HTML parsing/escaping uses permissive html5ever/RcDom; no upstream helper source was copied, no CSP checks were removed, and jagua-rs remains unmodified. Tauri/codegen/utils versions are pinned. The licence gate checks the Windows Cargo graph on every desktop build and permits only the common brief's allowlist, plus the existing jagua-rs exception. Sparrow's MIT LICENSE was inspected because its manifest omits a licence field. The exact package/version table is in `DEPENDENCY_LICENSES.md`; upstream licence texts are bundled in `THIRD_PARTY_NOTICES.txt`.

## Lead integration lines

The root npm workspaces already include `"marker/apps/*"`. No root package line is needed. The new package declares:

```json
"name": "@beta/marker-desktop",
"version": "0.4.0"
```

Add these exact Marker workspace member lines:

```toml
    "apps/desktop/src-tauri",
    "apps/desktop/src-tauri/permissive/dom-query",
    "apps/desktop/src-tauri/permissive/option-ext",
```

Remove the desktop crate's standalone `[workspace]` table and its `members` line at that time. Move its patches to `marker/Cargo.toml` with these exact entries (merge with an existing section if present):

```toml
[patch.crates-io]
dom_query = { path = "apps/desktop/src-tauri/permissive/dom-query" }
option-ext = { path = "apps/desktop/src-tauri/permissive/option-ext" }
```

Keep the crate's explicit `version = "0.4.0"`. Remove the local `[profile.release]` table when joining the workspace. The Marker root already uses opt-level 3 and four codegen units. Add this exact package override to retain stripping:

```toml
[profile.release.package.beta-cutting-room]
strip = true
``` Regenerate lockfiles at the workspace root. No core model, engine cancellation hook or HTTP contract change is required for this implementation.

After merge, re-run the full Marker workspace, desktop tests and strict Clippy, both TypeScript checks, Vitest, the licence gate and NSIS build. Recheck route parity when changing `serve.rs`. Re-audit every Tauri HTML call/selector before upgrading the pinned codegen stack. Recheck the native pattern -> marker -> plan -> rolls -> bundles flow after other agents' changes. Preserve the permissive patches when integrating; dropping them reintroduces disallowed licences.


## Current bounds

- The active editor job and current cut plan are captured; separate project folders keep separate versions. This is not a multi-job library interface.
- Display units format the native dimensions summary. Existing geometry fields keep their labelled units. Worker preferences apply to nesting; planning stays at its crate's existing cap.
- Intermediate efficiency is unavailable for folded/tubular jobs until a checked result exists. Rule-based searches and planning may provide only elapsed time before completion. Cancel keeps the last accepted marker; it does not recover candidate placements from a killed worker.
- The pipeline operates on one selected compatible roll group and whole plies. Its composite endpoint is native IPC; the normal UI exposes the stages through the existing tabs.
- Installer install/uninstall, Explorer association activation and Windows 10 were not exercised on a clean VM. The generated NSIS script was checked for all associations, loader/notices, zlib compression and skipped runtime download. No physical printer/cutter or future language was tested.
