# Install BETA Cutting Room 0.4.0

This release is a launch candidate. Public download links follow only after
the [release checklist](RELEASE-CHECKLIST.md) is approved. Nothing here installs
a service or publishes a release automatically.

## Windows desktop

Use the x64 NSIS installer `BETA-Cutting-Room_0.4.0_x64-setup.exe`. It installs
for the current user. Compare its SHA-256 with the release record before running
it. The candidate is unsigned; signing and clean-PC acceptance remain gates.

Microsoft Edge WebView2 Runtime must already be installed. The installer skips
network bootstrap so installation can remain offline. If missing, an
administrator must transfer Microsoft's standalone offline runtime installer.
Windows 10 and a clean PC still need acceptance testing. A successful build on
the development laptop does not prove either.

Start **BETA Cutting Room**, load an open demo, check units and fabric width,
and run a short nest. Save a project in your own folder. Reopen it before using
factory data. Test recovery and export. Keep a separate backup; autosave is
recovery assistance, not a complete backup system.

The desktop calls Rust through Tauri IPC. It needs no local HTTP server or cloud
account. Hindi and Tamil text need native-speaker review.

## Windows portable CLI and browser app

Extract `BETA-Cutting-Room_0.4.0_windows-x64-portable.zip` to a writable folder.
Run `start-server.cmd`, then open `http://127.0.0.1:7878`. Leave the console open;
Ctrl+C stops it. Do not open `web/index.html` directly from disk. The CLI also
runs as `.\beta-marker.exe --help` in PowerShell.

Use [self-hosting](SELF-HOSTING.md) for remote access or authentication. Browser
drafts belong to the browser profile. Export them before removing the profile,
changing the site address or clearing site data.

## Linux server

The Linux package is the CLI plus static browser assets. The Windows desktop
installer is separate. A Linux desktop GUI is not part of this release's package
or permissive-licence claim.

Extract `BETA-Cutting-Room_0.4.0_linux-x86_64.tar.gz`, enter the extracted folder,
then run:

```sh
chmod +x ./beta-marker
./beta-marker --help
./beta-marker serve --bind 127.0.0.1 --port 7878 --web-dir ./web
```

Use a build compatible with the destination's architecture and system libraries.
The build machine's glibc version is recorded with the artifact; an x86_64 Linux
build does not promise compatibility with older distributions. Review the
user-service and HTTPS examples before using them.

## Build from source

Use Rust 1.90 or newer, target `wasm32-unknown-unknown`, Node.js 22 or newer, npm,
and a wasm-bindgen CLI matching the resolved crate. Initial dependency/tool
installation needs a network connection or prepared caches. The built product
runs offline. Set `CARGO_TARGET_DIR` outside the checkout and keep
`CARGO_BUILD_JOBS=3` on a shared machine. Set an external npm cache too.

From the repository root:

```sh
npm install
npm run build:wasm -w @beta/marker-wasm
npm run build:pattern-wasm -w @beta/marker-wasm
npm run build:floor-wasm -w @beta/marker-web
npm run build -w @beta/marker-web
cargo build --manifest-path marker/Cargo.toml --release -p beta-marker-cli
```

On Windows, the desktop wrapper also needs `BETA_TOOLS_DIR` pointing to a tool
directory containing `desktop/node_modules/@tauri-apps/cli` and cached
`tauri-bundler` tools. Set `WASM_BINDGEN` if its CLI is outside PATH. Run
`npm run build -w @beta/marker-desktop`. Find the installer under
`$CARGO_TARGET_DIR/release/bundle/nsis/`. See the
[desktop report](../apps/desktop/DESKTOP_REPORT.md) for toolchain/licence bounds.
Build Floor WASM first: the desktop pre-build currently builds only Marker and
Pattern WASM.

On Linux, build `-p beta-marker-cli` for a server. The entire workspace also
selects desktop platform dependencies. Preserve the permissive patches and
audit optional platforms separately.

## Before cutting cloth

Check scale on paper, grain, mirrored pairs, quantities, shrinkage, matching,
roll defects and the independent validator. Generic HPGL and ISO output must be
calibrated for the actual plotter/cutter. Acceptance needs a real device and a
trained operator.
