# BETA Cutting Room 0.7.0 release record

This release builds on `marker/mvp` at `a143140` (all of round 0.7 merged).
BETA is a fun project created with the help of modern AI tools. These are
unsigned local release packages. No push, public upload, publication or
deployment is part of this work, and no server was used.

## What is new: make it good

**Room to work.** The menu has five jobs: Home, Make a style, Photo to marker,
Marker from files and Cutting floor, plus More tools. Only the open screen is
in the page, so hidden screens no longer slow the app or leak text to screen
readers. One cm/inch switch sets every screen. The phone Back button goes back
inside the app.

**A marker you can read.** The marker draws live while the search runs. Each
piece shows its name and size. Zoom in and tap a piece for its details.

**The browser edition is complete for the cutting room.** Only the
board-photo digitizer still needs the desktop app or a server. In the browser
you can now make plotter PLT/HPGL, DXF-AAMA, full-size and tiled PDF, ISO and
SVG files. They are byte-identical with the desktop app on all 46 test files.
DXF import reads .rul grading. Cut plans run in a worker with "Stop and keep
the best plan". Rolls, spreading, bundles and tickets, floor tracking, marker
reports and Compare also run locally: 33 of 33 test outputs are identical with
the desktop app. Work is kept in browser storage; a backup file saves it all.

**One meaning of plies.** A ply is one fabric layer. A pass is one length laid
from the roll (open fabric: 1 ply; folded or tube: 2 plies). Cut order and cut
plan files are now format v2. Version 1 files keep their old meaning. See
[PLIES.md](PLIES.md).

**Honest folded and tube lays.** Results now show useful yield (fabric that
ends up in garments) next to the footprint, and count copies cut but not used.
Symmetric pieces serve both layers. Fold halves stay on the fold inside the
main search. On the worst test (shirt DXF, folded, 10 garments) fabric fell
from 3.34 to 1.68 m per garment. When the order itself sets the length, the
screen says so. See [the fold results](../crates/beta-marker-nest/FOLD7_RESULTS.md).

**Blocks you can sew.** The trouser, track pant and shirt yoke were redrafted;
the 0.6.0 trousers could not be sewn. 10,806 block checks pass. A new spec
sheet gives points of measure per size as PDF, CSV or Excel. See
[block checks](../../pattern/docs/BLOCK-CHECKS.md).

**Serious bugs fixed.** A dart no longer makes a second production front.
Typed units such as "58 in" on a cm screen are honoured. DXF "Cut 2" makes a
left and right pair by default. Folded lays are no longer halved twice in the
cut plan.

**Release polish.** Engine search notes (passes, seeds, timings) sit under a
closed "Technical details". The lay cost tile shows only when a cost is
entered. Rolls, Bundles and Floor table headers and the Passes tooltip follow
the chosen language. The Photo-to-marker "Print A3 target" button now gives
the true reason in the browser: the target comes from the board-photo
digitizer, which is not in the browser edition.

## Limits that matter

- **Folded fabric is still behind open fabric.** Folded T-shirt and Teagan
  orders with odd counts per size need 23% to 74% more fabric than an open lay;
  tubes about 11% to 12% more. The shirt without "cut face down" permission
  keeps one unusable mirrored copy of each one-sided piece. The engine now
  reports these cases instead of hiding them.
- **Blocks need sewn samples.** Automatic checks prove the drafts agree with
  themselves and with the size chart. They do not prove fit. A pattern master
  must sew and review samples before production.
- **Browser planning is slower.** The browser plans on one thread, about
  2 seconds per marker. Big jobs remain much better on the desktop app.
- Quick Marker real-photo accuracy is untested. Check dimensions against the
  physical paper.
- Hindi and Tamil text, including the new table headers, are drafts that need
  native-speaker review.
- The Windows installer is unsigned. WebView2 is a separate prerequisite.
  Clean-PC and Windows 10 acceptance, real plotter and cutter calibration and
  an independent security review remain open.
- No commercial pattern-making parity or general win over Lectra is claimed.
  Tukatech, Gerber and Optitex have not been compared.

Public desktop benchmark evidence is unchanged: Lectra 60 s, 300 tasks,
mean -0.73 pp, median -1.22 pp, win/tie/loss 84/12/204; recorded durations,
60 tasks, mean +0.42 pp, median -0.30 pp, win/tie/loss 25/3/32.

## Build and verification method

`deploy/release-build.mjs` remaps source and Cargo paths to `/build` and
`/cargo` for all four WASM modules (Marker, Pattern, Floor and the new
cutting-room module), the CLI and the desktop app. Three Cargo jobs and
portable CPU settings are kept. Executable and WASM bytes are scanned in
UTF-8 and both UTF-16 alignments. This is path-leak verification, not proof
of bit-for-bit reproducibility. This record is kept out of its own archives.

`deploy/site/build-pages.mjs` builds the website and the static app at
`/beta-cutting-room/app/` from the same remapped WASM
(`BETA_SKIP_WASM_BUILD=1`). The seven website screenshots were retaken at
1920 x 1200 against the local Rust server, using only the built-in demo
shirt, the teaching bodice, the BETA T-shirt block and synthetic rolls. The
Pages folder keeps the unchanged v1 web demo video.

The licence inventory was regenerated from resolved Cargo and npm metadata,
optional tooling included: **524 selected entries, 97 metadata-only entries,
0 needing review**. The two new entries are BETA's own `beta-marker-api` and
`beta-marker-cutroom-wasm` (Apache-2.0). Unmodified jagua-rs 1.0.0 remains the
only MPL-2.0 exception. The desktop licence gate passed for 386 Windows
packages. Full notices ship with the installer, the ZIP and the static app.

## Checks on the release commit

- Marker Rust workspace, including the desktop crate: **468 passed**, 0
  failed, 9 existing ignored tests (46 test binaries). Pattern Rust workspace:
  **127 passed**, 0 failed or ignored. Ignored tests are not counted as passes.
- Strict Clippy in both workspaces, all targets, warnings denied: passed.
  Three new lints from export7 and fit7 code (`chunks_exact`, an 8-argument
  helper, a collapsible `if`) are allowed in place, on their existing lines,
  so the tested binaries still match the source line for line. The two
  crates were re-tested after that change (export 27, pattern core passed).
- Vitest: **488 passed in 49 files** (whole repository). All npm workspace
  typechecks passed. Site tests: 44 passed.
- Native web, standalone static app and Pages builds passed.
- Export parity: **46/46** browser factory files byte-identical with native,
  and 6/6 DXF import reports identical.
- Cutting-room parity: **33/33** browser results byte-identical with native
  (XLSX: every entry except its creation time).
- Style smoke with real mouse and keys, the real Style engine and the browser
  nester: **14/14** in the development app and **14/14** in the static build
  at 1366 x 768, plus the folded cut-plan check (40 plies = 20 passes, yield
  shown). Zero console errors and zero external requests; the static run sent
  zero API requests.
- Cutting-room journey on the static build: **16/16** steps (DXF open, nest,
  marker files, cut plan with Stop, rolls, spreading, bundles and tickets,
  reports, floor tracking, Compare, reload, backup file, fold-edge display),
  with zero API, external or console errors.
- Release UI fixes on the static build: **9/9** (About 0.7.0; Live checks and
  cut plan keep engine log lines in a closed "Technical details"; no lay
  cost tile without a cost; Hindi Rolls headers and Passes tooltip; true A3
  target reason).
- Offline static flow at `/beta-cutting-room/app/`: **19/19**, including
  offline reload, a real scan of a public synthetic photo, nesting and PDF
  output; the digitizer is the only screen that asks for a server.
- Older workflow smokes (Editor, Floor, Pattern, production, Quick Marker,
  roll audit, incomplete plan, Compare, feedback): **27 checks passed**. Their
  runners were updated for parked screens and the one cm/inch unit.
- Browser WASM worker 12/12, token end-to-end 7/7, screenshot workflow 16/16,
  Style WASM smoke and Quick WASM smoke passed.
- Windows CLI: **81/81** CLI audit calls; QA-006 returned all 114 placements
  with the exact lock and a valid marker in 1.019 s (a one-second search plus
  start and check time, not a real-time guarantee). Server checks passed on
  the CLI and on the extracted portable package.
- Path scans: CLI, desktop executable, installer and all four Pages WASM
  modules had **zero** user, home, worktree or Cargo-target path matches, and
  every Pages WASM passed `node deploy/release-build.mjs --verify`.
  Release-helper tests: 3/3.
- The portable ZIP was read entry by entry: **97 files** match staging, CRCs
  pass, paths use forward slashes, all four WASM modules are present.

## Artifacts

Files are in `$CARGO_TARGET_DIR/releases/0.7.0/`.

- `BETA-Cutting-Room_0.7.0_x64-setup.exe`: **11,617,671 bytes**. SHA-256: `07e3e7e49332b3554555df46855498d08ed59126e48b17ff3f9348014ae7233e`.
- `BETA-Cutting-Room_0.7.0_windows-x64-portable.zip`: **13,472,269 bytes**. SHA-256: `ffb220923617fa9e0a5f590ddb71e84bd4b8d438d535dac33bc5bfecb4159eee`.
- **No Linux package yet.** The glibc 2.31 builder used for 0.5.0 and 0.6.0
  is on the shared compute server, which this release agent may not use. The
  website keeps the published 0.6.0 Linux server package until the lead
  builds 0.7.0 there (`deploy/package-linux.py` now also requires the
  cutting-room WASM).

The Pages folder `releases/0.7.0/pages/` has **63 files, 41,988,713 bytes**,
with the app at `/beta-cutting-room/app/` and the unchanged 15,741,940-byte
v1 demo video (SHA-256
`8e73ab92a8e4eb4a5c88bcd1a66c03799ee510dc62ba6560f58c6ace77f060d7`).
Browser WASM sizes: Marker 4,491,834; Pattern 5,193,833; Floor 3,833,325;
cutting room 4,992,959 bytes.

`artifact-manifest.json`, `SHA256SUMS` and `SHA256SUMS.txt` list the packages,
CLI, notices, licence JSON, this record and every Pages file. Signing or
changing any package needs new hashes and matching website download fields.
No push, tag, upload, publication or deployment was performed.
