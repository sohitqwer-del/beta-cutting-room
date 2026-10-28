# BETA Cutting Room 0.6.0 release record

> Historical release evidence. The browser export/server limitation below was
> removed in [0.7](RELEASE-0.7.0.md). These counts, packages and performance
> statements describe 0.6, not the current development branch.

This release builds on `marker/mvp` at `6b1f5dc`. BETA is a fun project
created with the help of modern AI tools. These are unsigned local release
packages. No push, public upload, publication or deployment is part of this work.
Only public tracked source was copied to the approved isolated Linux builder.

## From scratch to cut

The Style workspace joins eight steps: Style, Pattern, Grade, Lay, Gaps,
Stripes, Marker and Cut. Start with a blank sheet or an editable T-shirt,
shirt, trouser, A-line skirt, kurta/tunic, polo or track-pant block. Draft on
the canvas, grade from a size chart, view stacked sizes and check seam lengths.
Choose the lay, gaps and repeats, then nest, inspect and prepare cutting files.
The stripe/check overlay shows each match rule and its measured error in mm.

Rust provides style documents, path offsets, line intersections, hit testing,
linked-piece regrading, missing fold marks and matched nesting in the browser
worker. The server accepts `POST /api/pattern/style`. Canvas tools use the
Rust engine. The formula Pattern screen remains available for advanced work.

The offline video tools now support Kokoro narration and recording your own
voice. The Pages folder retains the exact existing v1 web demo video until
the user chooses a voice. Browser status is clearer and service-worker builds
carry an asset version so a changed app can update its offline cache.

## Limits that matter

- Basic blocks are starting drafts. A pattern master must review fit, buyer
  measurements, marks, allowances and construction details before production.
- Extend and trim currently work between straight lines. Curve offsets cover
  lines, cubics and circular arcs; unsafe or unsupported offsets return errors.
- In the browser, marker plotter PLT, full-size PDF and DXF need the desktop
  app or a server. Tiled marker PDF works locally. Pattern exports use the
  Pattern engine. Physical print, plotter and cutter calibration remain open.
- Browser nesting is weaker on big jobs. Use the desktop app for large markers.
  The 0.5.0 selected WASM comparison remains the available performance evidence;
  this release makes no new general speed or quality claim.
- Quick Marker real-photo accuracy is untested. Synthetic checks do not prove
  camera or cutting accuracy. A4 mode cannot correct lens distortion. Check
  dimensions against physical paper.
- Hindi and Tamil text and narration drafts need native-speaker review.
- The Windows installer is unsigned. WebView2 is a separate prerequisite.
  Clean-PC/Windows 10 acceptance and independent security review remain open.
- Folded and tubular results remain weaker than open fabric. No commercial
  pattern-making parity or general win over Lectra is claimed. Tukatech,
  Gerber and Optitex have not been compared.

Public desktop benchmark evidence remains qualified: Lectra 60 s, 300 tasks,
mean -0.73 pp, median -1.22 pp, win/tie/loss 84/12/204; recorded durations,
60 tasks, mean +0.42 pp, median -0.30 pp, win/tie/loss 25/3/32.

## Build and verification method

`deploy/release-build.mjs` remaps source and Cargo paths to `/build` and
`/cargo`. Portable CPU settings and three Cargo jobs are retained. Actual
executable and WASM bytes are checked in UTF-8 and both UTF-16 alignments.
This is path-leak verification, not proof of bit-for-bit reproducibility.
The current release record is excluded from its own archives to avoid
self-referencing hashes. Full local logs stay outside source control.

The Linux CLI targets glibc 2.31 using the existing isolated build tools.
The final measured symbol requirement and loader-level runtime checks will
be recorded with the artifacts. This does not establish clean-OS acceptance.

`deploy/site/build-pages.mjs` assembles the website and static app at
`/beta-cutting-room/app/`. The seven gallery images use only original built-in
patterns and synthetic sample stock. Screenshots and flows run headlessly.
Every WASM in the assembled Pages folder is checked with the release verifier.

The licence inventory is regenerated from resolved Cargo/npm metadata, with
optional tooling included. Unmodified jagua-rs 1.0.0 remains the sole accepted
MPL-2.0 code exception. Bundled fonts keep their Apache-2.0/OFL-1.1 notices.
The model/narration tool licences are documented in
[the audio sources](../deploy/video/AUDIO-SOURCES.md); weights and new audio
are not bundled in the application release.

## Final verification and artifacts

- Marker Rust workspace, including desktop: **441 passed**, 0 failed,
  9 existing ignored tests. Pattern Rust workspace: **114 passed**, 0 failed
  or ignored, including its doc test. Ignored tests are not counted as passes.
- Strict Clippy in both workspaces, all targets, warnings denied: passed.
- Vitest: **245 passed in 27 files**. A stale 0.5.0 download-URL expectation
  was updated for this release; the final full run passed.
- All npm workspace typechecks, all three release WASM builds, native web,
  standalone static app and site/Pages builds: passed.
- Release-remapping helper tests: **3 passed**. Final artifact path scans
  and final Pages WASM scans are recorded with the artifact manifest.
- Browser worker checks: **12 passed**, including progress, cancellation,
  locked/folded validation and recovery from a stalled worker.
- Real Style flow: **14/14 passed** in the development app and **14/14** in
  the standalone static browser build at 1366 x 768. These use real mouse and
  keyboard input, the Rust Style engine and matched browser nesting. A blank
  piece and T-shirt, curve edits, offsets, extend/trim, S/M/L/XL grading,
  fold marks, gaps, 1 cm stripes, four matching rules, PDF/DXF/PLT pattern
  exports, tiled marker PDF and editor handoff were exercised. Both runs had
  zero console errors and zero external requests.
- All **17 existing smoke routines across nine workflows** passed in the
  expanded runner: Editor, Floor, Pattern, production tools, Quick Marker,
  roll audit, incomplete plan, Compare and feedback opener. The runner
  recorded **27 checks**, zero external requests and zero uncaught exceptions.
  Feedback opening and clipboard fallbacks used test doubles; no external
  application was opened and no real message was sent.
- Token E2E: **7 passed**. Screenshot workflows: **16 readiness checks**,
  with zero external requests and zero uncaught exceptions in both runs.
  All seven 1920 x 1200 images were visually reviewed and added to the site.
  The final site also passed 14 desktop/mobile/reduced-motion captures with
  no horizontal page overflow, hidden reveals, external requests or console errors.
- Direct Quick WASM scan/export smoke passed with the documented 4000 x 3000
  synthetic fixture, five PDF outputs and repeated-scan memory bounds. The
  source photo and all geometry are original public test inputs.
- Final Pages-path static flow: **18 checks passed** at
  `/beta-cutting-room/app/`, including offline reload, real synthetic-photo
  scanning, nesting, PDF export/sharing and projector geometry. No API
  requests or uncaught exceptions were observed. The final rebuilt app is
  checked byte-for-byte against this tested app before handoff.

The Windows CLI passed all **81 CLI audit calls**. QA-006 completed in
**1.0226 seconds**, with all **114 placements**, the exact lock retained and
independent validation success. This is an observed one-second search plus
startup/validation overhead, not a hard real-time guarantee.

The final Linux ELF was built with Rust 1.98.1, cargo-zigbuild 0.23.4 and
Zig 0.17.0 for `x86_64-unknown-linux-gnu.2.31`, with three Cargo jobs
and an explicit eight-CPU affinity cap. Its highest required GLIBC
symbol is **2.30**. All **81 CLI audit calls**, authenticated loopback server
checks and QA-006 passed using the extracted Ubuntu libc6
**2.31-0ubuntu9.18** loader. QA-006 returned 114 placements, retained the exact
lock and passed validation in **1.0090 seconds**. Help and authenticated server
checks also passed on the host's glibc 2.43. The wrapper hash in runtime logs
identifies that wrapper; the artifact manifest records the actual ELF hash.
The extracted libc is a test prerequisite and is not distributed.

Every archive entry was compared with staging: **87 Windows files** and
**86 Linux files**. ZIP CRCs passed, paths use forward slashes, and the Linux
CLI has mode 0755. Extracted Windows and Linux packages passed server probes;
the Linux package used the glibc 2.31 loader. The inventory contains **522
selected entries**, **97 metadata-only entries**, and **zero requiring review**.
Full notices accompany the desktop, ZIP, tar and static app.

The desktop executable, installer, both CLIs and all three Pages WASM modules
had zero Windows user, Linux home, known worktree or Cargo-target path matches
in UTF-8 and both UTF-16 alignments. Every WASM in the Pages folder also passed
`node deploy/release-build.mjs --verify`. Compressed payloads are covered by
the matching uncompressed executable and archive checks. This does not prove
bit-for-bit reproducibility or clean-PC installer acceptance.

## Artifacts and handoff

Files are under `$CARGO_TARGET_DIR/releases/0.6.0/`. The installer filename has
no spaces. The current release record stays outside its own archives.

- `BETA-Cutting-Room_0.6.0_x64-setup.exe`: **10,075,182 bytes**. SHA-256: `d9e6e1f887db829d842579b5ebc3f96ec3905e285387d71cd8fc571f984aed98`.

- `BETA-Cutting-Room_0.6.0_windows-x64-portable.zip`: **11,458,546 bytes**. SHA-256: `6ee86bb77ad9ff6bfe303edee1373bc6760f22d5f3c530bde44efb67ad5efb7a`.

- `BETA-Cutting-Room_0.6.0_linux-x86_64.tar.gz`: **10,316,030 bytes**. SHA-256: `d5b3e98d9298b33eaa7b7906058baaff4410d80b7d924735a63a62882030368b`.

The final Pages folder is `releases/0.6.0/pages/`: **58 files**,
**36,136,639 bytes (34.46 MiB)**. Its app is at
`/beta-cutting-room/app/`. It includes the seven refreshed screenshots and the
unchanged **15,741,940-byte** v1 demo video, SHA-256
`8e73ab92a8e4eb4a5c88bcd1a66c03799ee510dc62ba6560f58c6ace77f060d7`.

`artifact-manifest.json`, `SHA256SUMS` and `SHA256SUMS.txt` accompany the
packages. The checksum lists cover deliverables and every Pages file; raw
build inputs and QA logs remain separate local evidence. The manifest records
the release commit, build base, tools, lockfiles, Linux runtime, actual binary
hashes, Pages hashes and screenshot hashes. Package signing or changing any
file requires new hashes and matching site download fields.

`package-lock.json` is unchanged and is not committed. All resolved local
BETA Cargo versions are 0.6.0. The source, verification logs and packaged Linux
check remain in the approved isolated builder folder; its temporary 0.6.0
Cargo target was removed. No other server folder or service was changed.

No GitHub operation, release tag, public upload, push, publication, deployment
or service installation was performed. The local artifacts and Pages folder
are ready for the lead's review. Browser test counts cover the tested pages;
OS/browser background diagnostics were not audited.
