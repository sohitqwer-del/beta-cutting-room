# BETA Cutting Room

**Open-source, self-hosted cutting-room software for garment factories. Version 0.7.0.**

BETA takes a style from pattern to sewing line: draft and grade patterns, nest
markers, plan cuts, inspect rolls, print spreading sheets, cut and make bundles.
It runs offline on a Windows PC, or on your own factory server. No account,
no cloud, no licence fee. Your patterns never leave your factory.

- Website: https://sohitqwer-del.github.io/beta-cutting-room/
- **Try it now in your browser (no install):** https://sohitqwer-del.github.io/beta-cutting-room/app/
- Demo video (3 min): on the website home page
- Download: [Releases](https://github.com/sohitqwer-del/beta-cutting-room/releases)
- Benchmark: [marker/docs/benchmarks.md](marker/docs/benchmarks.md)
- Contact: sohitqwer@gmail.com

> **A fun project, built with the help of modern AI tools.** BETA is young and
> not perfect yet. **We need your help to improve it.** If something is wrong,
> confusing or missing, please tell us: every report helps factories like yours.
>
> - [Report a bug](https://github.com/sohitqwer-del/beta-cutting-room/issues/new?template=bug_report.yml)
> - [Tell us what is missing](https://github.com/sohitqwer-del/beta-cutting-room/issues/new?template=missing_feature.yml)
> - [Share feedback or an idea](https://github.com/sohitqwer-del/beta-cutting-room/issues/new?template=feedback.yml)
> - [Join the discussion](https://github.com/sohitqwer-del/beta-cutting-room/discussions)
>
> Please do not attach factory patterns unless you agree to make them public.
> The app also has a **Feedback** button on every screen; it never sends anything by itself.

## New in 0.7.0: easier to use, and the whole cutting room in the browser

- **Roomier, simpler screens.** One "Start here" path, a short menu (Make a style, Photo to marker, Marker from files, Cutting floor, More tools), one cm/inch switch for the whole app, and the phone Back button works.
- **Live marker.** The marker is drawn while it improves, with piece name and size on every piece, zoom, and tap for details.
- **Everything in the browser, no server:** plotter (PLT/HPGL), DXF-AAMA, full-size and tiled PDF, DXF import with grade rules, cut plan, rolls, spreading sheet, bundles and tickets, floor tracking, reports and compare. Work stays on your device; save a backup file to move it.
- **Honest folded and tube lays.** Results show the real fabric yield and pieces cut but not used. Waste on the worst test case is about halved, but folded fabric is still behind open fabric for small orders. See [FOLD7_RESULTS.md](marker/crates/beta-marker-nest/FOLD7_RESULTS.md).
- **Better blocks.** Trouser, track pant and shirt yoke redrafted; 10,806 shape and seam checks pass for all 7 blocks in every size; new measurement spec sheet (PDF, CSV, Excel). Blocks are still starting drafts: sew a sample before production.
- **Clear words.** A ply is one layer of fabric; a pass is one length laid from the roll (folded or tube = 2 plies per pass).

Full notes: [marker/docs/RELEASE-0.7.0.md](marker/docs/RELEASE-0.7.0.md). Real-user audit: [marker/docs/AUDIT-0.7.md](marker/docs/AUDIT-0.7.md).

### From 0.6.0: pattern to cut in one flow

Start from a blank sheet or a ready block (T-shirt, shirt, trouser, skirt, kurta, polo, track pant), draw and edit the pattern, grade it from a size chart, choose the lay (open, folded or tube), set the cut gaps, add stripes or checks with a live overlay and match points, make the marker and export the cut files.

## What is inside

| Folder | What it is |
|---|---|
| [`marker/`](marker/README.md) | BETA Marker: nesting, marker editor, cut plan, rolls, spreading, bundles, floor tracking. Rust workspace, web app, Windows desktop app (Tauri) and the `beta-marker` server. |
| [`pattern/`](pattern/README.md) | BETA Pattern: drafting, grading, DXF-AAMA and Seamly2D import, photo digitizing. Rust workspace. |

All geometry and production calculations run in Rust. Nesting uses
[sparrow](https://github.com/JeroenGar/sparrow) 0.3.0 (MIT) on
[jagua-rs](https://github.com/JeroenGar/jagua-rs) 1.0.0 (MPL-2.0, used unmodified),
plus BETA's garment rules and an independent validator that checks every marker.

## Benchmark in short

On the public Lectra dataset (Zenodo 7030786, CC BY 4.0, real client nesting tasks):

| Time per task | Tasks | Mean vs Lectra | Median | Win / tie / loss |
|---|---:|---:|---:|---|
| 60 s | 300 | -0.73 points | -1.22 | 84 / 12 / 204 |
| Lectra's recorded time (~5 min) | 60 | +0.42 points | -0.30 | 25 / 3 / 32 |

This is not a general win over Lectra: the median task is still behind.
Tukatech, Gerber and Optitex are not yet compared. Details and caveats:
[marker/bench/results/2026-10-20-public.md](marker/bench/results/2026-10-20-public.md).

## Install

- **Browser:** open the app link above on a PC or phone. It works offline after the first visit and keeps your data on the device. Best for trying and small to medium jobs; for big markers and faster planning use the desktop app.
- **Windows:** download `BETA-Cutting-Room_0.7.0_x64-setup.exe` from Releases
  and check its SHA-256. The installer is not code-signed yet, so SmartScreen
  may warn you. See [marker/docs/INSTALL.md](marker/docs/INSTALL.md).
- **Server:** see [marker/docs/SELF-HOSTING.md](marker/docs/SELF-HOSTING.md).
  The ready-made Linux build needs glibc 2.31 or newer (for example Ubuntu 20.04+).

## Build from source

You need Rust (with the `wasm32-unknown-unknown` target and `wasm-bindgen`) and Node.js 22.18 or newer.

```sh
npm install
npm test
cd marker && cargo test --workspace
cd ../pattern && cargo test --workspace
```

## Licence

Code: Apache-2.0 ([LICENSE](LICENSE)). Data: CC BY 4.0 where stated.
Third-party notices: [NOTICE](NOTICE). Security reports: [SECURITY.md](SECURITY.md) (private, not public issues).
