# QA for BETA Cutting Room 0.8

Date: 2026-10-28. Tester: qa8. Base: `345f42e`.

This is a launch audit, not factory or physical cutter acceptance. Inputs are original synthetic fixtures and public repository samples. All browsers ran headless. No private files were accessed. No push, publication or deployment was performed.

## Evidence and method

External evidence is under the launch8-shots/qa8 folder in the shared project-data area. `logs/` holds complete gate output; `tools/` holds the headless drivers; `visitor.json`, `first-result.json` and `features.json` hold structured observations. Screens are 390x844 and 1366x768. The existing round-07 smoke and journey drivers were copied with only worktree/output paths changed.

Cold visitor timings use fresh Chromium profiles, disabled cache, 200,000 bytes/s download (1.6 Mbps), 93,750 bytes/s upload and 150 ms network latency. The phone uses mobile viewport/touch emulation; this does not emulate phone CPU performance. Timing is an automated lower bound, not a human usability study.

## Gates

All required base gates passed. Skips are disclosed separately from passes. All native gates use the locked dependency graph, offline Cargo, three compiler jobs and an external target directory. Release WASM builds use the release path-remapping wrapper.


| Gate | Result | Evidence log |
|---|---|---|
| Marker `cargo test --workspace --offline --locked` (desktop included) | 490 passed, 0 failed, 9 ignored | marker-test.log |
| Pattern `cargo test --workspace --offline --locked` | 127 passed, 0 failed, 0 ignored | pattern-test.log |
| Marker strict Clippy, workspace / all targets / `-D warnings` | Pass | marker-clippy.log |
| Pattern strict Clippy, workspace / all targets / `-D warnings` | Pass | pattern-clippy.log |
| Root Vitest, generated real WASM available | 51 files / 498 tests passed, no skips | vitest-wasm.log |
| All workspace typechecks | Six workspace scripts passed | typecheck.log |
| Fresh release WASM through path-remapping wrapper | All four modules built | marker/pattern/floor/cutroom-wasm.log |
| Binary path-leak scan | All four shipped WASM binaries passed | verification.json and tool output |
| `npm run build:static -w @beta/marker-web` | Pass; 29 files, 24,903,849 raw bytes / 5,403,243 aggregate gzip bytes | static.log |
| Native/WASM export parity | 46/46 factory files byte-identical; 6/6 DXF import reports identical | export-parity.log |
| Native/WASM cutting-room parity | 33/33 identical; XLSX creation timestamp excluded by the established script | cutroom-parity.log |
| Existing Style browser smoke | Original 14 + folded-plan check: 15/15 | smoke.log |
| Existing cutting-room journey | Original 14 + 2 display checks: 16/16 | journey.log |
| Direct real-WASM Style smoke | Pass, including 5,403 block checks | wasm-style.log |
| Desktop crate compilation | Pass in full marker workspace tests and strict all-target Clippy | marker-test.log; marker-clippy.log |

Toolchain: Rust/Cargo 1.98.1, Node 24.16.0, npm 11.13.0. The nine marker ignores are declared long studies, performance/report-generation tests and a child-process helper; none is counted as a pass. No ignored test was forced to run. The initial pre-WASM Vitest run passed 493 with five skips; the final 498/498 run supersedes it.

The standalone desktop `cargo check -p beta-cutting-room --offline --locked` was intentionally cancelled while rebuilding a different dependency-feature graph, after both the complete workspace test build and strict all-target Clippy had already compiled the desktop successfully. Its cancellation is not reported as a pass or as a product defect. All processes started for this audit were finished or explicitly stopped before handoff. There was no non-UI bug requiring a source fix; this commit contains the QA report only.

## First minute

The landing page identifies garment factories, open source, offline use and local data clearly. Both sizes have no horizontal page overflow. The phone load event was 1.61 s for the site and 3.88 s for the app; laptop app was 3.09 s. These are load events, not completed Rust work. GitHub's load event took 16.41 s on the phone and 12.45 s on the laptop under the same throttle; its large platform chrome delays the README.

The phone Home screen puts the eight-step style route ahead of the file-free sample. The sample action is below the first viewport. A sample also appears under Continue in a fresh browser. Both were reported to launch-ui before the first hour.

The public repository README identifies purpose, licence, privacy, downloads, benchmark and feedback. The root lacks contribution and code-of-conduct guides. A fresh `npm ci` fails because this checkout has no lockfile; dependency installation used `npm install --package-lock=false` without changing tracked package files. Source setup needs a clear documented command and an explicit lockfile policy.

## Acceptance limits

No physical paper photographs, printer scale, plotter, cutter, sewn fit, installer launch or signed installer acceptance is claimed. Phone testing uses desktop Chromium viewport emulation. The prior 0.7 audit contains historical open findings; they are not automatically counted as new failures here. The requested old journey drivers are rerun against this base to check regression in their actual behavior.


## Feature tasks

| Task | 390x844 | 1366x768 | Evidence |
|---|---|---|---|
| Open two-fabric paper set; separate inventory | Pass | Pass | features.json, paper-open screenshots |
| 100 mm edge: paper 95 mm fails at 3 mm tolerance; 100 mm passes | Pass | Pass | paper-fail-pass screenshots; independent-numbers.json |
| Main 180 GSM / 1000 mm; lining 80 GSM / 1100 mm restore independently | Pass | Pass | paper-groups-gsm screenshots |
| A check on main does not unlock lining; lining gets its own passing check | Pass | Pass | features.json; supplemental.json |
| Separate main and lining markers, spacing 2 mm / edge 5 mm | Pass | Pass | paper-*-job/result.json; lining-*-job/result.json |
| Download real PLT and tiled PDF from paper marker | Pass | Pass | 390/1366 download files in evidence folder |
| Save set, reopen and require new review | Pass | Pass | saved-*.paper-set.json; paper-editor-save-reopen screenshots |
| Region Preview / Apply / Undo | Pass | Pass | region-preview screenshots; region-applied/undone JSON |
| Cancel stops region preview | Pass | Pass | region-stop screenshots; closed dialog, no Apply |
| Keep-shorter-marker notice after fresh search | Pass | Pass | keep-shorter-notice screenshots; features.json |
| Compare 1500 mm metre-priced and 1400 mm kilogram-priced offers | Pass | Pass | cost-*.json, cost-features.json |
| Impossible 10 mm width remains failed; cannot win | Pass | Pass | cost-*.json |
| Stock shortage cannot win | Pass | Pass | cost-stock-shortage screenshots: no eligible stock |
| Stop keeps prior completed cost comparison | Pass | Pass | cost-features.json; retained=true |
| Real photo-worker scan of original synthetic A4 scene | Pass | Not repeated | synthetic-scan-390.png; one piece, weak scale and grain warnings retained |

The initial custom harness runs contained two driver mistakes: typing millimetres into centimetre inputs, and filling a collapsed advanced section. Those failures are retained in the logs but excluded from product failures. Corrected runs and supplemental checks provide the acceptance evidence. A first cost run deliberately exposed the allowed refinement value 1 failing a two-size order; the valid comparison uses 4 refinements. Short 0.05-second cost searches test workflow and arithmetic, not attainable production efficiency.

## Independent number checks

**183 checks passed:** 15 tape/GSM/region/incumbent checks, 80 paper-marker and cost checks, 18 lining/Apply/Undo checks, and 70 cost checks on the separate saved-plan handoff run. The arithmetic scripts do not call BETA for the expected values. WASM provides the actual values; independent expectations use Euclidean distance, shoelace area, explicit rotation/translation, quantity multiplication and cost formulas.

- Main and lining each contain exactly their own piece. Each synthetic rectangle has area 20,000 mm² and marker length 205 mm. GSM does not change the outline. Main efficiency is 20,000 / (1000 x 205) = 9.7561%.
- Region preview and Apply change 50 mm to 40 mm. The locked outside placement is identical. Undo restores all placement records exactly and the length to 50 mm.
- A cached candidate length of 1 mm cannot beat a valid 50 mm incumbent when its geometry actually extends to 90 mm. The UI also shows the shorter-marker retention notice on both viewports.
- Cost is checked as net marker metres plus two end allowances per pass, plus purchase allowance and charged remnants; kg price uses width in metres x GSM / 1000. Garments are ratio x passes. No stock is assumed when the stock option is off.
- Phone comparison: 1500 mm offer consumes 6.315908 m and purchases 6.631704 m after 5% allowance. At 10 INR/m plus 2 INR lay cost, total is 68.317039 INR. The 1400 mm / 180 GSM / 50 INR/kg rate is 12.60 INR/m; its total is 80.284665 INR. Both produce M4/L4. The cheaper final offer wins.
- Laptop results differ because the time-limited search differs: totals are 65.957269 and 79.082989 INR. All arithmetic is checked against that run's actual geometry. Cross-run layout equality is not required.

## First-result timing and regressions

The live 0.7 sample reaches a checked real marker in 27.8 s on the phone and 21.4 s on the laptop under the stated cold slow-4G profile, with a one-second search. The local base static build reaches the same milestone in 27.0 s and 14.8 s. These are scripted discovery-free lower bounds. The sample action's position remains the bigger first-minute risk.

The local harness serves uncompressed static assets: its Home load events are 11.85 s and 7.49 s. Do not compare these directly with the compressed public host as an engine-speed claim. Timings ran on a shared laptop while build work was active; they are indicative, not a controlled performance benchmark.

The established round-07 Style driver passes its original 14 tasks plus the added folded cut-plan task: **15/15**, with zero console errors, external requests or /api requests. The cutting-room driver passes its original 14 tasks plus two additional display checks: **16/16**. It covers DXF, nesting, exports, cut plan, rolls, spreading, bundles/tickets, reports, Floor, PLT comparison, persistence and backup. No regression was observed in these executed main journeys. This does not prove every 0.7 function or stochastic nesting result is unchanged.

## Ranked top 10: defects and remaining acceptance gaps

| Rank | Severity / kind | Finding | Next action |
|---|---|---|---|
| 1 | S2 UX | Phone file-free demo is below the first viewport | Put a real sample result route first; reported to launch-ui |
| 2 | S2 UX | Phone cost table hides total/action beyond a long horizontal scroll | Mobile offer cards or sticky fields and scroll cue; reported |
| 3 | Acceptance gap | Paper-camera accuracy has no physical repeatability trial | Measure real paper, lighting, perspective and curl before factory acceptance |
| 4 | Acceptance gap | Physical 1:1 output and installer/recovery integration are untested | Calibrated machine and installation trials; headless compilation and file generation are verified |
| 5 | S3 UX | Cost refinement minimum can be below the number of ordered sizes | Validate before starting; reported |
| 6 | S3 UX | Paper check mixes cm inputs and mm results | Keep one unit or explain conversion; reported |
| 7 | S3 UX | Fresh sample looks like saved work under Continue | Mark it as a sample; reported |
| 8 | S3 onboarding | Root contribution/code-of-conduct guides absent on public repo | docs8 owns documentation; reported |
| 9 | S3 reproducibility | npm ci cannot run without a lockfile | Lead chooses lock policy; document install command; no lock committed |
| 10 | S3 UX | Impossible width reports incomplete order quantities | Preserve the packing/width failure reason; reported |

No S1 wrong-number defect was found in the executed checks. Two entries above are acceptance gaps, not reproduced software defects. UI findings were appended to the shared findings file as they were found; this branch does not change frontend code. The benchmark and final launch-ui integration require their own post-merge gates.


### Saved-plan handoff and further wording

A separate short-search handoff run on both viewports compares M4/L4, then edits the visible order to M1/L1 before Use saved plan. The resulting plan correctly remains M4/L4 from the comparison's saved snapshot (`cost-handoff-features.json`). Its stochastic search values differ from the arithmetic run above and are stored separately as `cost-handoff-*.json`.

The impossible-width row reports `Order quantities are incomplete` even though the saved M4/L4 order is complete and the wider offers succeed. The alternative stays failed and cannot win, but the explanation directs the operator to the wrong input. This S3 wording issue was also reported. The cost-table overflow affects laptop width as well as phone width. The cost-refinement failure's initial evidence remains in `features.json`; `cost-features.json` contains the corrected successful run.
