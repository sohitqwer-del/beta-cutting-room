# BETA Cutting Room audit, round 0.7

Date: 2026-10-22. Auditor: audit7 (two sessions: GPT, then Claude).
Question: why does the factory owner say "the software is still not great"?

## Short answer

The app looks finished, but three things break trust when a factory person uses it for real work:

1. **The browser edition stops halfway.** It is the version people try first, but a cutting in-charge can only get a marker and its files. The cut plan, roll register, spreading sheet, bundle tickets and marker report all need the desktop app or a server.
2. **Folded and tube lays waste fabric, and until this round the screen hid it.** On folded fabric, BETA's markers used up to 2.6 times the fabric of an open lay. The screen still showed "Checks passed" and about 60% efficiency. Folded and tube lays are the normal case for knitwear units.
3. **Some numbers were wrong in quiet ways.** Examples: "58 in" typed on a cm screen became 58 cm, and a DXF "Cut 2" front became two right fronts. "Plies" also means different things on different screens.

Most of the screen problems were fixed during this round (see the status column below). The remaining work is in the engine (folded lays) and in bringing cut planning and the cutting floor into the browser.

## How the audit was done

- Builds tested headless (Playwright over CDP; never a visible window). All at 1366x768, or 390x844 for the phone user:
  - **Base:** static build of `marker/mvp` bd2a230 (0.6.0, the live app).
  - **Round:** bd2a230 plus `marker/export7` 46799f1 plus `marker/fit7` fb09b6c, merged on a temporary branch.
  - **ux7 head:** `marker/ux7` 5c3efe6 with the round WASM.
- Live app <https://sohitqwer-del.github.io/beta-cutting-room/app/>: read-only checks only.
- Correctness was checked with small independent scripts. They do not call BETA code for the quantity being checked (`marker/out/audit7/`, git-ignored):
  - `marker-check.mjs`: piece areas, copies, useful yield.
  - `plan-check.mjs`: cut plan totals.
  - `independent-numbers.mjs`: grading and seams.
  - `units-check.mts`: cm/inch.
- Screenshots: `round07-shots/audit7/` in the project data folder (outside git). Prefixes: `round-` (round build), `ux7-` (ux7 head), `live-` (live app). Unprefixed shots are base, except `master-01/02/04`, which a re-run replaced with round-build shots.
- Times below are automated runs. A person needs longer. Treat them as a lower bound and use the click counts to compare.

## Top 10 problems, ranked by user impact

| # | Sev | Problem | Evidence | Status at end of audit |
|---|---|---|---|---|
| 1 | S1 | **Browser edition dead-ends the cutting journey.** Cut plan, roll register (even "Save roll"), spreading sheet, bundles and tickets, marker report TXT/mini PDF and Compare all need a server. The in-charge reaches step 2 of the 8-step workflow. | `round-cut-plan.png`, `round-rolls.png`, `round-bundles-made.png`; every call returns SERVER_FEATURE_MESSAGE | ux7 now says so before the form and disables the buttons. **The features are still missing in the browser.** |
| 2 | S1 | **Folded/tube markers waste fabric while the screen looked fine.** Shirt DXF, 29 in folded, 10 garments: 33.39 m, 3.34 m per garment, "Efficiency 60.2%", green "Checks passed". The validator found 160 wasted copies, and the useful yield was 30.1%. The same order on an open 58 in lay: 1.30 m per garment. BETA's own T-shirt, 100 garments: 140.0 m folded vs 82.3 m open. The folded result has 9 wasted copies, and only 6 of 10 fold pieces are on the fold. A 60 s search gives the identical marker. | `nest-variants.json`, `round-style-folded.*`, `folded60.*`; `round-cut-05-export-plt.png`, `round-style-folded-cut.png` | The UI part is fixed in ux7: amber "N pieces cut but not used". With the pair default, the shirt drops to 21.40 m (2.14 m per garment). **Still open (engine, export7):** symmetric pieces waste a layer, fold pieces are not preferred on the fold, and the headline efficiency is footprint (58.7%) where the useful yield is 47.0%. |
| 3 | S1 | **DXF "Cut 2" became two same-hand pieces.** "Left/right pairs" was off and hidden. An open face-up lay would cut two right fronts and no left front, with no warning. | Imported M pieces: Front 2+0, Sleeve 2+0 | **Fixed in ux7.** DXF open asks; "pair" is the default (verified). |
| 4 | S1 | **Style dart made two production fronts.** Adding a dart kept the original Front active (cut 1+1) next to the new piece (cut 1+1). Seam checks stayed green for the old piece. | `master-03-grade.png`; saved doc quantities | **Fixed in fit7** (verified: front 0+0, front_dart1 1+1). New S2: all 8 seam checks vanish and the screen says "No seam checks yet". |
| 5 | S1 | **Typed units were ignored.** "58 in" on a cm screen became 58 cm; "150 cm" on an inch screen became 150 in (3.81 m); "1,500" became 1.5 cm. All of these passed the width check. | `units-check.mts`: 5 of 19 failed | **Fixed in audit7** f68d4f4 and 8bdf615 (+5 tests), merged into ux7. |
| 6 | S1 | **"Plies" means two things.** The Style Lay step counts fabric layers (folded 40 = 20 spreads). The cut planner and Floor count spread passes. Style hands its layer count to the planner as "Max plies per lay", so a folded plan may stack twice the layers intended. | `style/ui.ts:695`; `beta-marker-plan` `Lay.plies`; `floor.rs` "folded passes have two physical layers" | **Open.** The cut plan intro still says plies are "fabric layers". |
| 7 | S2 | **Too many screens and too little room** (lead's test): 14 menu items, ~310 px of chrome, all screens in the page. | Base Home 3,329 DOM nodes, Style 3,371 | **Fixed in ux7:** five-job menu, slim top bar, Style canvas 784x356 → 1122x601 px (ux7's measure), DOM 705 (Home) and 951 (Style). |
| 8 | S2 | **Phone photo flow prints nothing real.** The sample ends in a 623-byte "practice" PDF and a 619-byte chalk guide with no pieces. One photo set is one size; no way to make S/M/L. A real printable 3-size T-shirt marker is only possible through the Style flow. | `phone-07-use.png`, `round-phone-07-use.png` | Partly addressed: ux7 says "one photo is one size". The real nest for the sample is **open**. |
| 9 | S2 | **The phone Back button leaves the app.** Screen changes use replaceState, and a hash change after load is ignored (URL #/style, screen Home). Same on the live app. | `live-deeplink.mjs`; history.length constant | **Open.** |
| 10 | S2 | **Marker had no labels, no live view, and long markers were unreadable.** A 33 m marker drew as a 10 px strip of overlapping text. | `round-cut-05-export-plt.png` | **Fixed in ux7:** engine label placement, live best-so-far, readable fit (`ux7-folded-done.png`). |

Other findings, all in `round07-findings.md` in the shared agent-brief folder (outside git):

- Hidden screens leaked text to screen readers (fixed by ux7 unmounting).
- Hindi UI partly English (open; needs a native-speaking cutter).
- Feedback recorded "Unknown" screen (fixed by ux7).
- Plies field accepted 10^12, gave the wrong reason for 0, and kept a stale error message (fixed by ux7).
- Marker editor accepts a 100 km fabric width (S3, open).
- Ctrl+Z right after nesting did nothing in one run; the toolbar Undo/Redo buttons work (1.470 → 1.416 → 1.470 → 1.416 m).
- The demo sample works offline after the first load (service worker), including Style.

## Task results

| User and task | Base 0.6.0 (live) | Round build / ux7 head | Finished? |
|---|---|---|---|
| **Small unit owner, phone 390x844:** photo to marker with sample images | 28 taps, 32 s automated. Size chips label pieces, no grading. Practice PDF has no pieces. | Same (30 s, 28 taps) | Practice only |
| Phone: marker for 3 sizes of a T-shirt | Through Style: 19 taps, 26 s to the Cut step (20 s search), sizes S, M, L on 58 in | n/a (not re-run) | Yes, via Style only |
| Phone: something to print | Tiled A4 marker PDF 190 KB, pattern 1:1 PDF, DXF, PLT | Same | Yes |
| **Pattern master 1366x768:** shirt block, collar point and sleeve length change, notch, 12 mm allowance, dart, grade S–XXL from an edited chart, check seams, save and reopen | 19 clicks, 12 s automated, 7/7 steps. Dart made a second production Front (S1). Seam table all "0 cm" incl. sleeve cap ease ("should be close to 0" is wrong guidance for caps). | Dart fixed (front 0+0). Script stopped: the seam table is gone after the dart. | Base yes (with S1); round no |
| **Cutting in-charge 1366x768:** open DXF | Blocked at step 1: "needs desktop/server" | Works: "Imported 32 pieces" | Round yes |
| Width 58 in, folded, ratio 2:3:3:2, 1 cm gap, stripes | n/a | Base editor: mm only (typed 736.6). ux7: 58″ preset sets 73.7 cm with a hint. | Yes |
| Make the marker | n/a | 20 s in browser. Base/round: 33.39 m, "Checks passed" (S1 waste). ux7: 21.40 m, amber "20 pieces cut but not used". | Yes, but wasteful |
| Plotter file and other exports | Disabled | PLT 171 KB, DXF 584 KB, PDF 7 pages, ISO, SVG; 0.6 s each. PLT length 33,390.2 mm = marker. | Yes |
| Cut plan | Server only | Server only (ux7: disabled with note) | **No** |
| Spreading sheet, bundles, tickets | Server only | Server only ("Save roll" too) | **No** |
| Floor tracking, reports | Floor dashboard opens with zeros. Marker report TXT/mini PDF disabled. | Same | **No** |
| Hindi UI | Most Style text translated; notices and status English | ux7 added drafts for new strings | Partly |
| Offline after first load | Works (service worker) | Works | Yes |
| Bad inputs | Plies 10^12 accepted; wrong reason for 0/−5; stale message across steps. Editor width 0 → browser "≥ 1"; 100 km accepted. | Plies fixed in ux7; editor 100 km open | Partly |

Live app first load (fresh context, read-only): 3.44 MB transferred, 13 requests. The load event fired in 1.3 s with 4G throttling (9 Mbps, 150 ms) and 0.6 s on Wi-Fi.

## Correctness results

"Pass" means the number BETA shows or writes equals an independent calculation.

| Check | Method | Result |
|---|---|---|
| Graded measurements vs size chart (shirt S–XXL, edited M sleeve 62 cm, L chest 105 cm) | Point distances in `grade` output vs chart; chest = chart + 80 mm ease | 15/15 pass |
| Seam lengths (8 shirt seam pairs × 5 sizes) | Polyline length of the named seam edges vs `block_seams` rows and differences | 120/120 pass |
| Marker length and efficiency, open and folded (T-shirt) | Length = max x of drawn outlines; efficiency = Σ outline area / (width × length); spread = length + 2 × end | 6/6 pass |
| Folded "efficiency" as fabric yield | Useful piece area / (2 layers × width × length) | **Fail by definition.** Shown 60.2% vs useful 30.1% (shirt DXF); 60.8% vs 45.8% (Style tee); 58.7% vs 47.0% on ux7 head. |
| PLT export | Parse HPGL, extents in 40 units/mm | Length 33,390.2 mm = marker. Width +9 mm, only from the header label. Pass. |
| Style Cut step fabric per lay | (length + 2 × end) × spreads; garments = ratio × spreads | Pass (140.01 m = 20 × 7.0003 m; 100 garments). The meaning of "plies" differs from the planner (problem 6). |
| Cut plan totals vs order (native `beta-marker plan`; T-shirt S200 M300 L300 XL200; max 50 plies; max 10 per marker; 2% over-cut; open 58 in and folded 29 in) | Planned = Σ ratio × plies; shortfall and over-cut; plies ≤ max; garments ≤ max; lay fabric = (L + 2e) × plies; total = Σ | 74/74 pass. Open 801.4 m, folded 888.8 m. Every size exactly as ordered. |
| cm/inch conversion and parsing | 19 typed values incl. "58 in" on a cm screen, "1-1/2", "1,500" | 14/19 before the fix → 19/19 after (f68d4f4) |
| Marker validator vs screen status | Rust `validate` on the saved job and result | 160 `surplus_copy` warnings while the screen said "Checks passed" (base, round). ux7 shows the count. |

Test suites on `marker/audit7` 8bdf615:
- Vitest **427/427** (base 425, plus 2 new test cases holding 11 assertions).
- Style smoke **14/14** on the static build: 0 console errors, 0 external requests (`round07-shots/audit7/smoke-audit7-static.log`).
- No Rust changed. The base Rust suite passed **441/441** in the first session's log.

## What factories expect that BETA lacks or does worse

Sources: public product pages and press (Gerber AccuMark/AccuPlan, Lectra Modaris/Diamino/Optiplan, Tukatech TUKAcad, Optitex, Audaces, Gemini, Richpeace, Indian marketplace listings), collected 2026-10-22. Vendors rarely publish details, so "lacks" means "lacks in BETA", not "all vendors have it".

| Expected (most common headline features first) | Commercial tools | BETA now |
|---|---|---|
| Auto nesting with high fabric use | All five families | Open lays are good (benchmarked vs the Lectra dataset). **Folded/tube is clearly worse** (problem 2). Optitex and Lectra advertise tubular/folded markers. |
| Grading to a full size range | All (rule tables, size charts) | Size-chart grading and .rul import. No grade-rule table editor UI. |
| Consumption, yield and cost reports | All | Marker numbers yes. Cost and order consumption only in the desktop/server cut plan. |
| Cut order planning from order quantities | Gerber AccuPlan, Lectra Optiplan, Gemini Planner, Audaces | Correct in native (74/74 checks). **Not in the browser.** Cannot force a given ratio (e.g. 2:3:3:2) as a lay. |
| Interactive marker editing with auto nesting | All | Yes (bump, compact, flip, lock, sections). |
| DXF-AAMA/ASTM exchange | All | Import and export in the browser after export7 (46 byte-identical files vs native). |
| Stripe/plaid matching | Tukatech, Audaces, Gerber, Lectra | Yes (repeat plus match points, per-rule report). |
| Plotter/cutter output | All (own cutter formats) | PLT/HPGL, ISO, DXF, PDF. No vendor cutter formats (by policy). |
| Digitizing | Gerber AccuScan (camera), Audaces, board digitizers | Phone photo with A4 calibration. Not yet measured on real factory photos. |
| Spec sheet / points of measure | Via PLM (Gerber/YuniquePLM, Audaces Idea) | fit7 API ready. **No screen yet.** |
| 3D fit | Gerber, Lectra, Tukatech, Optitex | None (low priority for small units). |
| Spreading, bundles, barcodes | Partial in most | Rich in desktop (tickets, QR, re-cuts). **None in the browser.** |
| Hindi/Indian languages | Richpeace claims 12 languages; Tukatech has Hindi videos | Hindi and Tamil drafts, not yet reviewed by native users. A possible advantage. |
| Price | TUKAcad Professional US$199/month; Indian CAD listings ₹0.98–2.5 lakh (unverified); marker bureaus ₹100–400 per marker plus ~₹90–200 per metre of plotting | Free and open source |

For a small unit, the real competitor is the local **marker bureau** (₹300 per marker). BETA wins only if a person can get a correct, printable folded/tube knit marker and a cut plan in a few minutes, in the browser.

## Recommended next round

1. **Folded/tube engine quality** (export7 owner):
   - Treat a mirror-symmetric piece's second layer as usable.
   - Prefer on-fold placement for fold pieces when the fold edge has room.
   - Count wasted copies as waste in the search objective.
   - Report `useful_yield`.
   - Target: folded within 5% of open for the same garments (today +65%).
2. **Cutting floor in the browser:** compile `beta-marker-plan`, `beta-marker-roll` and the bundle/ticket PDF parts of `beta-marker-cutroom` to WASM (the export7 pattern). Let the Style Lay step create a cut plan for a given ratio (2:3:3:2) as well as an optimised one.
3. **One meaning for plies:** use "spreads" and "layers" on every screen, in the handoff and in files. Fix `style/ui.ts:695`.
4. **Real photo test:** run the 10 real phone photos the user still owes. Make the Photo sample a real nest and print.
5. **Pattern master:**
   - Spec sheet screen (fit7 API).
   - "Seam checks need review" state after topology edits.
   - Sleeve-cap ease shown as a target range, not "should be 0".
6. **Phone basics:** Back button (pushState per screen and Style step); Hindi review by a cutter.
7. **Pilot:** time one real order end to end with two factories (bureau marker vs BETA), and record fabric used per garment.

## Reproduce

- Merge build: in a worktree at bd2a230, `git merge marker/export7 marker/fit7`, then `npm run build:static -w @beta/marker-web` with `wasm-bindgen` on PATH.
- Journeys: `marker/out/audit7/{cutting,style-folded,editor-inputs,phone-round,master}.mjs` over `host2.mjs`, which serves base on 4177, round on 4178, and a file-server offline switch on 4179.
- Numbers: `nest-variants.mjs`, `marker-check.mjs`, `validate-file.mjs`, `plan-make-order.mjs` and `plan-check.mjs` (with `beta-marker plan --time-per-marker 5 --end-allowance 20 --max-markers 8`), `independent-numbers.mjs`, and `units-check.mts` (kept with the audit scripts, outside git).
