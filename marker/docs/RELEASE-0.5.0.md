# BETA Cutting Room 0.5.0 release record

This release builds on `marker/mvp` at `3f0f162`. It is a fun project created
with the help of modern AI tools. The packages are unsigned release candidates.
No release was pushed, published, deployed or uploaded to a public service.
Only public source and verification inputs were copied to the assigned isolated
Linux build folder under the authorized server-build workflow.

## Changes

The app has new screens, navigation, a humble welcome and feedback on every
screen. The desktop uses a scoped Tauri opener for feedback links. The logo
and website are new. Pattern adds graded seam checks, RUL/JSON rule libraries,
mirror and unfold operations, button and drill rows, and fold guides.

Folded nesting now allocates physical fabric layers. On the public Teagan test,
useful-demand yield rose from 42.8% to 62.9% at 60 seconds. This is one fixture;
folded fabric still trails open fabric. See [fold evidence](../crates/beta-marker-nest/FOLD_RESULTS.md).

Quick Marker takes a photo of paper patterns with an A4 sheet or BETA target.
The operator reviews outlines, sizes, quantities and grain before nesting.
Outputs include tiled A4/A3 pages, a chalk guide and a projector layout.
The browser app runs local Rust WASM in workers, can be installed as a PWA,
and has a static build with no Rust server. A demo video pipeline uses only
public synthetic examples and offline narration. Audit fixes cover validation,
imports, exports and production workflows. Release checks also found a last-bit
JSON coordinate drift in a locked public Simon placement. Correct float
round-trip parsing now retains the exact coordinates; a core regression test
covers the observed value.

## Limits that matter

- Browser nesting is much weaker on large jobs. At a 4x laptop slowdown proxy
  and a 60-second setting, WASM averaged about 21.6 percentage points below
  native on nine valid Lectra task pairs, and 0.05 points below on three
  FreeSewing garments. A tenth Lectra task failed in both paths and was
  excluded from the mean. This is a selected subset and a slowdown proxy,
  not measured phone performance. Use the desktop app for big markers.
  See [the WASM report](../crates/beta-marker-nest/WASM.md).
- Quick Marker real-photo accuracy is untested. Synthetic tests do not prove
  camera or cutting accuracy. A4 mode cannot correct lens distortion. A4
  detection can be ambiguous, and its error estimate is a heuristic. Review
  dimensions against physical paper before cutting. A BETA target provides
  the calibrated workflow; it does not remove the need for physical checks.
- Piece-name assist reached only 22% accuracy in the recorded evaluation.
  Names, grain, pairs and cut-on-fold suggestions need operator confirmation.
- Folded and tubular jobs remain weaker than open fabric. The folded result
  above is not a claim of commercial parity.
- Hindi and Tamil text, guides and subtitle drafts need native-speaker review.
- The Windows installer is unsigned. Clean-PC and Windows 10 acceptance,
  actual printer/projector/plotter/cutter calibration, and independent
  security review remain open. WebView2 is a separate prerequisite.

Public desktop benchmarks remain qualified: Lectra 60 s, 300 tasks, mean
-0.73 pp, median -1.22 pp, win/tie/loss 84/12/204; recorded durations,
60 tasks, mean +0.42 pp, median -0.30 pp, 25/3/32. No general win over Lectra
is claimed. Tukatech, Gerber and Optitex have not been compared.

## Build and verification

Release builds use `deploy/release-build.mjs`: source and Cargo paths map to
`/build` and `/cargo`, including both Windows separator spellings. CPU settings
remain portable. Executable and WASM bytes are scanned in UTF-8 and both
UTF-16 alignments for user, home and worktree paths. No source-directory
reproducibility claim is made from a cached build.

The final check counts, measured Linux ABI and artifact hashes are recorded
below after the final packages pass verification. Full local evidence stays
outside source control. The current release record is excluded from its own
archives to avoid circular hashes.

## Website and screenshots

`node marker/deploy/site/build-pages.mjs --out <empty-output-folder>
--video <demo-video/beta-demo-en.web.mp4>` builds the website plus the static
app at `/beta-cutting-room/app/`. Omit `--video` when it is absent, or set
`BETA_DEMO_VIDEO`. The script also finds the optional `demo-video` sibling of
the external Cargo cache layout. The output is one local GitHub Pages folder; the script
never publishes. Set the usual external Cargo target, three build jobs,
wasm-bindgen tool and npm cache before running it. Build output is ignored.

The six website images are in `apps/site/public/assets/screens/`:
`marker-editor.png`, `pattern.png`, `cut-plan.png`, `rolls.png`,
`spreading.png` and `bundles.png`. They use only the original built-in shirt,
teaching bodice and synthetic sample stock. The headless capture script
checks the real local API, forms and Rust WASM. No visible browser is used.

## Licence inventory

Regenerate with `deploy/generate-licenses.py --include-optional --texts
<output>/THIRD-PARTY-NOTICES.txt --json <output>/licenses.json`. The inventory
includes the scoped opener, text shaping, Unicode and web-time dependencies.
Droid Sans Devanagari and Tamil fonts use Apache-2.0. The site's Bricolage
Grotesque font uses OFL-1.1. Unmodified jagua-rs 1.0.0 remains the sole accepted
MPL-2.0 code exception. Full notices accompany all three packages.

## Linux baseline

The final x86_64 ELF was built with Rust 1.98.1, cargo-zigbuild 0.23.4 and
Zig 0.17.0, targeting glibc 2.31. The highest required GLIBC symbol is 2.30.
All 81 CLI audit calls passed using the extracted Ubuntu libc6
2.31-0ubuntu9.18 loader. QA-006 (`--fixed --time 1 --workers 2 --seed 42`)
returned all 114 placements in 1.0075 seconds, retained the exact locked
placement and passed independent validation. Help and authenticated loopback
server checks passed with that loader and with the host's glibc 2.43.

The loader wrapper's checksum in raw runtime logs identifies the wrapper;
the artifact manifest and table identify the actual ELF. This verifies the
older runtime path, not an entire clean Ubuntu installation. The tools and
extracted libc stayed in the assigned user folder; no host service or system
package was installed. The compiler tools are MIT. The extracted LGPL libc
is a test prerequisite and is not part of the release archive.

## Final automated checks

- Marker Rust workspace, including desktop: 428 passed, 0 failed, 9 ignored.
- Pattern Rust workspace: 99 passed, 0 failed, 0 ignored, including one doc test.
- Strict Clippy, both workspaces and all targets: passed with warnings denied.
- TypeScript checks: all npm workspaces passed.
- Marker, Pattern and Floor WASM plus native web, static app and site builds: passed.
- Real browser WASM worker: 12 checks passed, including responsive progress,
  cancellation, stalled-worker recovery and folded/locked validation.
- Full static flow: 18 checks passed at `/beta-cutting-room/app/`, including
  offline reload, a real engine scan of a public synthetic photo, nesting,
  PDF output, sharing and projector geometry. No API requests or uncaught exceptions.
- Release remapping helper: 3 tests passed. Actual executable and WASM scans
  found no Windows user, Linux home or source-worktree paths.

Ignored tests are opt-in evidence or longer runs and are not counted as passes.
The first Windows run reproduced the new locked-coordinate regression before
its Cargo feature graph was rebuilt. The corrected full run above passed.

The five Vite workflows (Editor, Floor, Pattern, production tools, Quick Marker)
passed all 13 existing smoke routines, with zero external page requests and
zero uncaught exceptions. The runner uses headless focus emulation so keyboard
selection follows the active-tab path. Token E2E passed 7/7. The six screenshot
workflows passed 13 readiness checks, with zero external page requests and
zero uncaught exceptions. All six 1920 x 1200 images were visually inspected.
These counts do not measure browser or operating-system background diagnostics.

The final Windows CLI passed all 81 CLI audit calls. QA-006 completed in
1.0240 seconds with all 114 placements, the exact lock retained and independent
validation success. The one-second search has startup/validation overhead;
this observed time is not a hard real-time guarantee. The audit now checks
exact locked-placement membership, because valid output may reorder placements.

## Artifacts and handoff

Files are under `$CARGO_TARGET_DIR/releases/0.5.0/`. The installer name has no
spaces. Every archive entry was read and compared with staging: 73 Windows
files and 72 Linux files. ZIP CRCs passed and every entry uses forward slashes.
The Linux CLI has mode 0755. All three matching WASM modules and complete
third-party notices are present. Extracted Windows and Linux packages passed
loopback server probes; the Linux package used the glibc 2.31 loader.

- `BETA-Cutting-Room_0.5.0_x64-setup.exe`: 9,624,731 bytes. SHA-256: `4ceb1058ba57eb244594900be7b472005b279a299a2a670038c18ffa1581f2ea`.
- `BETA-Cutting-Room_0.5.0_windows-x64-portable.zip`: 10,385,459 bytes. SHA-256: `bb67647fa1b5f35dccfb75cc04f96077166e44788e50a04d05b78d127c60d566`.
- `BETA-Cutting-Room_0.5.0_linux-x86_64.tar.gz`: 9,346,910 bytes. SHA-256: `4854f75bcc3293939d554e59f0274b32d5c975a3cb75888a6b917ee842584694`.

The Windows CLI is 21,375,719 bytes, SHA-256
`7bce085f72e5dc1bb4acd008e899cf495f294574622ee05334b0035060651f61`.
The Linux ELF is 14,791,688 bytes, SHA-256
`c3bad877af7137d1a38df6985c739ce42b0f872726e6b1d1e5d9d7cbf95534c3`.
The desktop executable, installer, both CLIs and all three WASM files had
zero matches for Windows user, Linux home or any local worktree-directory path
in UTF-8 and both UTF-16 alignments. Compressed payloads are covered by the
matching uncompressed executable and archive checks. This does not establish
bit-for-bit reproducibility or a clean-PC installer acceptance test.

Vitest passed 197 tests in 23 files. The regenerated licence inventory has
522 selected entries (including bundled fonts), 97 metadata-only entries and
zero requiring review. Full notices are included in desktop, portable, Linux
and static-app distributions. `package-lock.json` is not part of this commit.
Cargo lockfiles contain the resolved 0.5.0 versions.

The final Pages folder has 57 files, 33,631,148 bytes (32.07 MiB), including the
six new screenshots, browser app and the exact 15,741,940-byte web demo video.
Build it with `deploy/site/build-pages.mjs`; use an empty output folder or one
previously created by that script. `BETA_SKIP_WASM_BUILD=1` can reuse the three
already remapped and verified WASM modules. It must not be used with stale assets.

`artifact-manifest.json`, `SHA256SUMS.txt`, the full licence texts and audit JSON
accompany the packages. The checksum list covers release deliverables and every
Pages file; raw build inputs and check logs are separate local evidence.
The manifest records the source commit, tool versions, Linux runtime, binary
hashes, Pages file hashes and the individual screenshot hashes.

The lead receives the local packages and Pages folder for review. No GitHub
operation, public release upload, publication, deployment, service installation
or release tag was performed. The isolated Linux build and verification folder
remain available for the lead. Signing or changing any package requires new
hashes and matching site download fields.
