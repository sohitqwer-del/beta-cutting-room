# BETA Cutting Room 0.4.0 release record

This is an unpublished, unsigned launch candidate. The runtime source is
`4078d36` on `marker/mvp`, including engine fix `e437b34`. The release branch
adds packaging records, public sample screenshots and checks. No engine upgrade:
Sparrow 0.3.0 and unmodified jagua-rs 1.0.0 remain in use.

## Builds and verification

Windows x86_64 GNU: Rust 1.98.1, Node 24.16.0, wasm-bindgen 0.2.129.
Cargo uses three jobs, release optimization level 3, thin LTO, four codegen
units and stripped debug information. No `target-cpu=native` is used.

Linux x86_64: Rust 1.98.1, Ubuntu glibc 2.43. The isolated build and temporary
loopback probes ran at nice level 10. No service was installed. Older Linux
distributions need a compatible rebuild; this artifact does not promise an
older glibc baseline. Windows requires the separate Microsoft WebView2 Runtime.

The final Windows checks passed:

| Check | Result |
|---|---|
| Marker full workspace tests, including desktop | 385 passed; 0 failed; 9 ignored |
| Pattern full workspace tests | 62 passed; 0 failed; 0 ignored |
| Marker strict Clippy, all targets | Passed; 0 warnings |
| Pattern strict Clippy, all targets | Passed; 0 warnings |
| Marker Vitest suite, including site | 72 passed in 12 files |
| TypeScript: marker-wasm, web, desktop, site | All four passed |
| Marker, Pattern and Floor WASM | All rebuilt; all three in web assets |
| Linux help and authenticated loopback serve | Passed; owned server stopped |

Commands, run from the repository root with `CARGO_BUILD_JOBS=3`:

```sh
cargo test --manifest-path marker/Cargo.toml --workspace --no-fail-fast -- --test-threads=2
cargo test --manifest-path pattern/Cargo.toml --workspace --no-fail-fast -- --test-threads=2
cargo clippy --manifest-path marker/Cargo.toml --workspace --all-targets -- -D warnings
cargo clippy --manifest-path pattern/Cargo.toml --workspace --all-targets -- -D warnings
npx vitest run marker
npm run typecheck -w @beta/marker-wasm
npm run typecheck -w @beta/marker-web
npm run typecheck -w @beta/marker-desktop
npm run typecheck -w @beta/marker-site
```

Ignored tests are opt-in evidence/longer runs; they are not counted as passes.
The full Windows Marker run includes the HTTP wire regressions and three new
deadline/cancellation regressions. Independent security review remains open.

The exact public QA-006 Simon job has 114 placements and one locked placement.
The final Linux CLI with `--fixed --time 1 --workers 2 --seed 42` completed in
**1.018 seconds**, returned all 114 placements and retained the exact lock.
A separate `validate --json` call passed. This is an observed result on that
fixture, not a hard real-time guarantee for arbitrary geometry or suspension.

The final Windows CLI completed the same QA-006 call in **1.410 seconds**,
with all 114 placements, an unchanged lock and independent validation success.
Both final CLIs return a marker of 8.818486 m / 74.0341% for this fixed-only call.
Use `--warm-start` as well as `--fixed` to retain an existing complete incumbent;
`--fixed` alone only retains locked placements.

The final Windows help/server probe passed. Headless token E2E passed **7/7**:
401 opens the dialog, a wrong token fails, the correct token connects, storage
is session-only, About shows 0.4.0, reload retains the token, and Forget clears it.
The six screenshot workflows passed **13 readiness checks**. Both browser runs
reported **zero external page requests** and **zero uncaught exceptions**. Their
owned server and browser processes stopped. This does not measure platform
background diagnostics or replace a target-PC network audit.

After installing the six PNGs and installer checksum, the static site build
passed, found all six screenshots, and the final Vitest rerun again passed all
**72 tests**, including **17 site tests**. The site build includes its TypeScript
check. All six images were inspected before commit.

## Artifacts

Artifacts are stored under `$CARGO_TARGET_DIR/releases/0.4.0/`. The installer also
remains in `release/bundle/nsis/`. This record is excluded from its own archives,
so its checksum table does not create a circular dependency. Archive names,
byte counts and SHA-256 values are:

| Package | Bytes | SHA-256 |
|---|---:|---|
| `BETA Cutting Room_0.4.0_x64-setup.exe` (published as `BETA-Cutting-Room_0.4.0_x64-setup.exe`) | 8241490 | `6066499c315cda9c60cf34b01c0398de119dd338cd9b5003ad8e94a7f1804df7` |
| `BETA-Cutting-Room_0.4.0_windows-x64-portable.zip` | 8830651 | `c5ae4817fe7913b61089c91cc5dab66f2e663d4bd5dc522ce4c4a2f6ef65c0ca` |
| `BETA-Cutting-Room_0.4.0_linux-x86_64.tar.gz` | 8309493 | `a146aeeb1aadc8035c7421ace4690ab32eefaaf129e50c582ff2e7b4822edfb1` |

Windows ZIP CRC validation passed. Both archives were read in full: 44 Windows
files and 43 Linux files. Every web asset matched the built output, executable
hashes matched the probed binaries, notices were present, and the Linux CLI
mode was 0755. No dataset, private directory, token, browser profile or build
output directory was found. `SHA256SUMS.txt` and `artifact-manifest.json` accompany
the artifacts outside the repository. The Linux archive was assembled locally
from the server-built ELF and the same verified web assets.

Underlying executable checksums:

| Executable | Bytes | SHA-256 |
|---|---:|---|
| Windows `beta-marker.exe` | 21077032 | `689350cb58b7e5396bc56bb069b3ca6442ae58a51152af5244798dfdae30d9bc` |
| Linux `beta-marker` | 18415744 | `a2a43188bbe16db8d52bb450041134ad68302ec78fc80e0f61ca207858a02357` |

Portable packages contain the matching CLI, complete web assets (all three WASM
modules), deployment examples, documentation, Apache licence, NOTICE and full
third-party licence texts. No datasets, raw test logs, access token or browser
profile is included.

## Licence confirmation

Regeneration after merging the engine fix produced **510 selected packages**,
**95 metadata-only packages outside the shipped target graphs**, and **zero
review-required selected packages**. The inventory is unchanged. Selected Rust
and npm licences are permissive, except unmodified jagua-rs 1.0.0 under MPL-2.0.
The original Apache-2.0 `dom_query` and `option-ext` adapters remain in place.
The separately audited WebView2 SDK loader uses BSD three-clause terms; its
Runtime is not bundled. See [the inventory](../THIRD-PARTY-LICENSES.md).

Resolved lockfiles are retained with the private build evidence. Lockfile edits
are deliberately not committed in this release branch, as directed by the lead.
Regeneration and review in the integration branch remain a publication gate.

## Public sample screenshots

All six PNGs are under `apps/site/public/assets/screens/`, relative to `marker/`:
`marker-editor.png`, `pattern.png`, `cut-plan.png`, `rolls.png`, `spreading.png`
and `bundles.png`. They show original built-in BETA examples and synthetic stock.
No factory, buyer or private geometry appears. The headless script drives real
forms, Rust WASM and the local authenticated API. No visible app windows open.
The site's repository, contact and download URLs remain placeholders.

## Remaining public-launch gates

This record proves build and automated checks, not clean-PC installation or
production readiness. Complete the [release checklist](RELEASE-CHECKLIST.md):
clean-PC and Windows 10 acceptance, actual printer/plotter/cutter calibration,
Hindi/Tamil operator review, independent server/security review, monitored
private reporting contact, approved public URLs, and installer signing.
Signing changes the installer hash; rebuild its checksum and site field then.
The tiny_http allocation finding was mitigated by the bounded Hyper transport;
413/400/431 wire regressions remain required. No independent audit is claimed.

No push, publication, public release upload, deployment, service installation or release tag
was performed. The isolated Linux verification copy is a build workspace.
