# BETA Cutting Room 0.8.0 release record

Prepared locally from launch8 at 9f02ada. BETA is a fun project built with modern AI tools. These are unsigned Windows packages. No push, tag, publication, GitHub change or external server use is part of this work.

## What changed since 0.7.0

The nester can try stronger layouts around damage and fixed pieces. It can mix full cuts and fold halves per piece and size. Only complete, independently valid candidates replace the current marker.

The 24 October leading-defect pilot used four public FreeSewing families, three seeds and five requested seconds per run. Mean fabric savings were hoodie 12.8%, shirt 9.5%, T-shirt 12.9% and trouser 22.8%. These are selected synthetic-damage cases, not a production promise. The clean T-shirt control saved -0.78% on average. Actual wall times differed. See [the paired study](NESTING-EFFICIENCY-2026-10-24.md).

The later six-second, two-seed constrained pilot saved 5.66% on the shirt original-lock case and 2.29% on its interior-defect case. T-shirt constrained cases were flat or slightly worse; hoodie folded regressed 1.90%. A synthetic mixed-layer fixture fell from 123.002 to 83.002 mm (32.52% less fabric). It is not a garment result. See [all cases and controls](NESTING-THREE-IMPROVEMENTS-2026-10-24.md).

Keep a valid shorter marker after a new search. Improve one area with Preview, Apply and Undo. Compare widths and total order cost, including GSM, kilogram pricing and allowances. Paper-pattern sets have tape checks and separate fabric groups with GSM.

New visitors get a file-free 60-second tour and a clear loading screen. The new website includes share cards and launch videos. FEATURES, FAQ, ROADMAP, GETTING-STARTED, CONTRIBUTING and CODE_OF_CONDUCT explain the platform and how to help.

## Benchmark (bench8)

The [28 October paired report](../bench/results/2026-10-28-public-0.8.md) covers 300 Lectra tasks at 60 seconds. Against Lectra, 0.8 has mean -1.39 percentage points (pp), median -1.57 pp, and 77 wins / 12 ties / 211 losses. In the same run, 0.7 has mean -1.37 pp, median -1.62 pp, and 77 / 13 / 210.

On those ordinary open jobs, 0.8 and 0.7 are essentially tied: mean -0.02 pp, median -0.0004 pp, and 47 / 187 / 66. This does not show a broad gain. On three garment jobs with a flaw at the roll start, open-defect useful yield gains mean +15.52 pp, median +14.46 pp, and 3 / 0 / 0. Titan falls from 9.55 m to 7.17 m. Three Simon folded/tube cases slightly regress. The large defect gains are not typical garment savings.

All 636 scored layouts passed independent validation. This is one seed, three requested workers per task and shared load, with four tasks at once. Wall-clock search varies with scheduling. The 20 October run is older, with different engine and scheduling conditions. Its figures are retained as history, not as the current headline.

## Honest limits

Folded fabric still trails open lays for small orders. Blocks need sewn samples. Real photo accuracy and physical plotter/cutter output are untested. The installer is unsigned. Hindi and Tamil are drafts. Strategy comparison is experimental and off by default. Search results vary with time and scheduling. This is launch verification, not factory acceptance.

## Build and verification

All four WASM modules are rebuilt from source with source and Cargo path remapping. Native CLI and desktop builds use the same wrapper, portable CPU settings and three Cargo jobs. The path scanner checks UTF-8 and both UTF-16 alignments with even-length slices, avoiding the Node odd-length decoder crash.

Linux is not built here. The lead uses the prepared linux-inputs folder and must fill the Linux download size and SHA-256 before enabling its link. No stale Linux hash is carried forward.

Final gate results and artifact hashes are recorded in the external release folder. The release record is excluded from its own package to avoid self-referencing hashes.

## Release gates and evidence

- Marker workspace: 490 passed, 0 failed, 9 declared ignores. Pattern: 127 passed, 0 failed or ignored. Both strict all-target workspace Clippy runs passed.
- Post-bench8 Vitest: 529/529 across 54 files. All six workspace typechecks passed. Native web, standalone static and Pages builds passed.
- Fresh remapped WASM: Marker, Pattern, Floor and cutting room; each passed release-build verification. Release-helper tests: 4/4, including odd-length binary buffers.
- Export parity: 46/46 files and 6/6 DXF import reports. Cutting-room parity: 33/33 (established XLSX timestamp exclusion).
- Style: 15/15. Cutting-room journey: 16/16. Tour: 12/12 on phone and 12/12 on laptop. Zero console, external or API errors in these journeys.
- Windows CLI audit passed 81/81 calls. QA-006 kept all 114 placements and the exact lock in a valid marker in 1.027 seconds. This is not a real-time guarantee. CLI server checks passed. The server verifier now reads the release version from config.
- Offline Pages-path flow passed 21/21, including reload, scanned photo, required paper review, nesting, PDFs and projector. The updated driver uses simulated tape values from synthetic geometry; it does not test physical camera accuracy. Earlier driver failures are retained in logs and superseded by the completed run.
- CLI, desktop executable, installer and all WASM copies passed path scans in UTF-8 and both UTF-16 alignments, including worktree and Cargo-target paths.
- Fresh app and site screenshots are 1920 by 1200. Site phone checks show no MP4 download before playback. The home and benchmark page show the current run with median and win/tie/loss, and label historical 20 October results.
- Portable archive: every entry matches staging, CRCs pass and paths use forward slashes. No installer launch, signing, physical output or physical photo acceptance is claimed.

## Artifacts

The external releases/0.8.0 folder contains the unsigned NSIS installer, Windows portable ZIP, CLI, notices, licence inventory, Pages and Linux handoff inputs. Artifact sizes and SHA-256 are in artifact-manifest.json and SHA256SUMS (LF). The source record is excluded from its own archive. Re-signing or changing any artifact requires new hashes and matching site download fields.
