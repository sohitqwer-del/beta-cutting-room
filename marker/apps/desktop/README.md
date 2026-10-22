# BETA Cutting Room desktop 0.7.0

A Windows x64 Tauri 2 application. The existing Cutting Room interface runs in the Windows WebView2 control. Rust calls the BETA crates directly. There is no HTTP server, runtime CDN, telemetry or updater. A strict content security policy and a navigation guard block remote pages. Only local event permissions are granted to the main window. Native file commands reject UNC paths.

## Install and use

Run the NSIS setup executable. It installs for the current Windows user and registers `.dxf`, `.plt`, `.betajob`, `.betamarker` and `.betapattern` files. Start **BETA Cutting Room** from the Start menu. WebView2 is normally present on Windows 11. This installer uses `webviewInstallMode: skip`: Windows 10 machines must have the Microsoft Evergreen WebView2 Runtime installed first. For an offline factory, provision Microsoft's standalone offline runtime installer through your normal software process. Setup and the app never download it. Windows 10 without WebView2 cannot launch the app. This build is unsigned.

Use the existing Pattern, Digitize, Editor, Cut plan, Rolls, Bundles, Compare and Help screens. **Open native file** and Editor's **Open file** use a Windows dialog. Dropped files and Explorer file associations use the same import path. Open a matching job before a standalone marker result. Native saves use `.betajob`, `.betamarker` and `.betapattern` names; their contents remain the shared JSON formats. Other existing file inputs use the WebView's system file picker.

**Cancel search** stops the actual solver process and waits for it to exit. The last accepted marker stays available; in-progress candidates are not recovered from a killed solver. A Windows Job Object also stops it if the parent crashes. Nesting, planning and pipeline searches share one permit. Other expensive operations run on blocking worker threads. Nest progress events contain elapsed seconds and the best feasible length/efficiency seen so far. Rule-based searches and planning currently provide elapsed time, then final nesting metrics where available; the underlying crates do not expose intermediate placements for those paths.

Save a PDF, then use **Open last PDF to print**. The default Windows PDF viewer supplies its print dialog. Print at actual size and check the calibration square. No physical printer or cutter has been calibrated by this work.

## Projects and recovery

**Save project** asks for a parent directory and writes `Cutting Room.betaproj/project.json`. A `.betaproj` is a folder, so it is easy to copy or back up. **Open project** selects that folder. It holds the current editor job/marker, live pattern/roll/bundle/damage drafts, the current cut plan with its lay markers, and desktop settings. Project files are limited to 50 MB. The live drafts are captured even if the WebView cache is full. Opening a project stages its data in Rust and reloads the interface. It does not use browser session storage to transfer the project. This is a single active project, not a multi-document library or a job history. Save separate copies in different parent folders to keep versions.

Autosave runs every five seconds when operations are idle and state changes. A synced temporary file and the previous good backup protect replacement. **Recover autosave** appears when a recovery snapshot is available; it restores the saved project explicitly. Up to five seconds of idle edits, or changes during an active operation, may be absent after a crash. There is no background cloud storage. Recovery is retained across normal closes too.

Settings, recovery, the WebView profile and 15 recent paths live under the app's local data directory. `BETA_DATA_DIR` redirects them into its `desktop` subdirectory. Settings include the default import fabric width in mm, a dimension summary in mm/cm/inch, plotter units per mm, a generic cutter ISO profile, nesting worker threads and English. Planning keeps its existing two-worker budget. Other geometry entry fields retain their labelled units. Hindi and Tamil are visible future placeholders. ISO commands must be verified for the actual controller.

## Build

Prerequisites: a Windows x64 Rust toolchain and matching linker/build tools, Node/npm, `wasm32-unknown-unknown`, a matching wasm-bindgen CLI and WebView2. Use external directories for shared-machine tools, Cargo output and npm cache. From the repository root in PowerShell, set your assigned paths:

```powershell
$env:CARGO_TARGET_DIR = '<external Cargo output>'
$env:CARGO_BUILD_JOBS = '4'
$env:WASM_BINDGEN = '<tools>/marker-web/bin/wasm-bindgen.exe'
$env:BETA_TOOLS_DIR = '<tools>'
npm install --cache <npm-cache>
npm install --prefix "$env:BETA_TOOLS_DIR/desktop" @tauri-apps/cli@2.12.3 --cache <npm-cache>
npm run build -w @beta/marker-desktop
```

NSIS compression is explicitly zlib. The optional NSIS LZMA compressor uses CPL-1.0 and is not used in this installer.

The npm workspace is already matched by root `marker/apps/*`; no root package edit is needed. The build checks dependency licences, builds both Rust WASM modules, builds the shared web assets, compiles the native executable and produces an x64 NSIS installer under `$env:CARGO_TARGET_DIR/release/bundle/nsis/`. Tauri's NSIS tools cache is redirected under `$env:BETA_TOOLS_DIR/tauri-bundler`. This machine uses Rust 1.98.1 with the Windows GNU toolchain. The GNU package includes WebView2Loader.dll; the executable imports no MinGW runtime DLL. MSVC is also a standard Tauri build option. The first build needs network access to fetch build dependencies and installer tools; the installed product works offline. `npm run dev -w @beta/marker-desktop` starts the usual local Vite development server only for development.

The desktop crate currently has its own workspace. When moving it into the Marker workspace, the lead must also move both `[patch.crates-io]` entries to the Marker root. See `DESKTOP_REPORT.md` for the exact lines. Do not drop these patches: the unmodified Tauri graph brings additional MPL-2.0 packages that the project licence policy excludes. Our original Apache-2.0 `dom_query` and `option-ext` adapters replace only the small interfaces Tauri/dirs-sys use. The HTML adapter delegates parsing and escaping to MIT/Apache-2.0 html5ever/RcDom; its selector subset is tested and Tauri codegen versions are pinned. Re-audit and extend those tests before a Tauri upgrade. No jagua-rs source is patched.

## API and verification

`transport.ts` preserves the web API's `Response` interface. Browser mode keeps `fetch('/api/...')`; desktop mode invokes `desktop_request` with method, path and raw bytes. Rust's `api.rs` preserves every current `serve.rs` API route, including pattern editor tools and binary outputs. It deliberately has no static-file or HTTP-server implementation. Keep the route parity tests in sync when adding server routes.

Desktop also accepts `POST /api/nest/cancel` and `POST /api/pipeline` (alias `/api/plan/pipeline`). Pipeline input is `{order, options, register, group_index?, colour?, bundle_rules?}`. It selects one compatible inspected roll group, applies its shrinkage, plans, spreads with both end allowances, maps defects, validates markers and creates physical lays/bundles. It returns `{order, inspection, plan, spread, bundles, markers}`. Use the existing export/ticket routes for print files. Whole plies and one selected group are deliberate limits. The normal interface already exposes the stages through its existing tabs; the composite pipeline is an IPC API.

```powershell
npx vitest run marker
npm run typecheck -w @beta/marker-web -w @beta/marker-desktop
cargo test --manifest-path marker/apps/desktop/src-tauri/Cargo.toml --workspace
cargo clippy --manifest-path marker/apps/desktop/src-tauri/Cargo.toml --workspace --all-targets -- -D warnings
cargo test --manifest-path marker/Cargo.toml --workspace
cargo clippy --manifest-path marker/Cargo.toml --workspace --all-targets
node marker/apps/desktop/scripts/check-licenses.mjs
```

For one bounded smoke run, set `BETA_DESKTOP_SMOKE_DIR` to an empty external directory and `BETA_DATA_DIR` to isolated test data, then launch the built executable once. The real frontend opens its original demo, nests for five seconds, validates, exports PLT through the export button, starts and cancels a second search, writes `smoke-result.json` and closes. This mode skips save dialogs only inside the explicit smoke output directory. It does not run during normal launches. The start measurement is Rust process entry to frontend bootstrap, including WebView creation and JavaScript loading, without flushing OS caches.

Dependency details and measured results are in `DESKTOP_REPORT.md` and `DEPENDENCY_LICENSES.md`.
