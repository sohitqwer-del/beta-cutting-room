# BETA Cutting Room: UX audit and design principles

Status: round 0.5 design pass. Scope: the web app in `marker/apps/web` (the desktop app shows the same pages).
This is a design-layer change. App logic, data contracts, transport and tests stay the same.

## Who uses it

- **CAD operators and marker makers.** They sit at an ordinary Windows PC, often 1366 x 768. They open DXF pieces, nest, fix a few pieces by hand and export to the plotter or cutter.
- **Cutting-room staff and supervisors.** They plan orders, check rolls, print lay sheets, record the cut and print bundle tickets. Some use a large floor screen (1920 x 1080) or a tablet near the table.
- Many users read English as a second or third language. Hindi and Tamil labels are drafts and are longer than English.

## How we looked

1. Heuristic review of every screen against Nielsen's ten heuristics.
2. Five task walk-throughs on the built app with demo data: first marker in 3 minutes, plan a cut order, inspect rolls, print a spreading sheet, make bundles.
3. Headless screenshots at 1366 x 768, 1920 x 1080 and 390 px wide (Floor and Bundles). Before shots: `<data-dir>/ux-shots/before/`.
4. Comparison with how commercial cutting-room tools *structure* their screens, from public product pages and manuals only. Nothing was copied. The common pattern in marker software is: a large marker canvas in the centre, a piece list or "piece bin" for unplaced parts, one toolbar of grouped tools, and an always-visible information bar with marker length, efficiency and placed/total pieces. Cut planning tools put an order grid (size x quantity) first, a short list of constraints next, and a result table of lays last. Shop-floor tools use large tiles, one task per page and scan-first input.

## Problems found

Priority: **P1** blocks or slows the main task; **P2** causes errors or confusion; **P3** polish.

| # | Problem | Where | Heuristic | Priority |
|---|---|---|---|---|
| 1 | At 1366 x 768 the marker canvas is below the fold. The top of the page is filled by statistics, two rows of tool buttons and a third toolbar. | Marker editor | Aesthetic and minimalist design; visibility | P1 |
| 2 | The main action, **Nest marker**, is about 1000 px down the left panel, under DXF import settings. New users do not find it. | Marker editor | Recognition over recall; visibility | P1 |
| 3 | About 30 tool buttons have the same size and weight. Daily tools (undo, rotate, lock) look the same as rare ones (section lines, numeric measure). | Marker editor | Minimalist design; consistency | P1 |
| 4 | There is no single place that shows the selected piece and its tools. Selection details are a line of small text under the canvas. | Marker editor | Visibility of system status | P2 |
| 5 | Length, efficiency and validity are split over three places (statistics, canvas footer, "Live checks" panel far below). | Marker editor | Visibility of system status | P2 |
| 6 | Nine export buttons in a row at the bottom, in capitals (LABELS, MINI). Save JSON buttons sit next to Open with the same weight. | Marker editor | Match with the real world; minimalist design | P2 |
| 7 | Long forms show every option at once: cut plan shows 7 limits with the same weight; the Fabric tab shows shade bands and stripes fields even when they are switched off. | Cut plan, Fabric rules, Rolls, Bundles | Minimalist design; flexibility | P2 |
| 8 | The 8-step workflow bar shows where you are, but not what to do next. | All screens | Visibility; user control | P2 |
| 9 | Buttons are 32-34 px high in tool rows and tables. This is small for a mouse on a busy day and too small for a floor tablet. | All screens | Error prevention (Fitts) | P2 |
| 10 | The sidebar uses 236 px on a 1366 px screen. It cannot be collapsed. | Shell | Flexibility and efficiency | P2 |
| 11 | On a phone the top bar wraps into two rows, and the roll check boxes on the Floor page show above their text. | Floor, Bundles at 390 px | Consistency | P2 |
| 12 | The theme follows the operating system. A dark Windows setting gives a dark app by default, which prints and photographs poorly on the floor. | Shell | Match with the real world | P3 |
| 13 | Units are part of label text ("Fabric width (mm)"). Users must read the label to know the unit while typing. | All forms | Recognition over recall | P3 |
| 14 | Icons exist only in the sidebar. Toolbar buttons are text only, so long Hindi or Tamil labels will not fit. | Editor, top bar | Consistency | P3 |
| 15 | Nesting progress pushes the canvas down when it appears. | Marker editor | Aesthetic design | P3 |

What already works and is kept: the screen grouping in the sidebar, the address per screen (`#/marker`), keyboard shortcuts in the editor, visible focus, the toast that keeps errors until dismissed, empty-state messages, print styles, and plain-English messages that say what to do next.

## Task flows (before → after)

| Task | Before | After |
|---|---|---|
| First marker in 3 minutes | Home → Open a file → scroll the left panel → find Nest under DXF import → scroll back up to see the marker. | Home → **Start a new marker** (or Try the demo) → set width and ratio in the Setup panel → **Nest marker** is always visible at the bottom of the panel → the result shows in the status bar under a canvas that fills the screen. A 4-step guide on Home links to each control. |
| Plan a cut order | All 7 limits shown together; the result is a table and a line of text. | Garments per size first, then the two main limits; 5 advanced limits are folded under "More options". The result shows totals as tiles above the lay table. |
| Inspect rolls | One long page. | Same page, but cards, wider fields and bigger buttons. "Next: Spread" in the workflow bar leads on. |
| Print a spreading sheet | Floor → Spreading → fill 9 fields → check boxes laid out wrong → Release files. | Same steps; roll choices are clear check rows; release files are grouped; the page works on a tablet. |
| Make bundles | Long page of equal-weight sections. | Prepare, print, table, re-cut and report are separate cards. Buttons are 48 px on touch screens. |

## Design principles

1. **The work area comes first.** On the marker editor the canvas fills the screen at 1366 x 768. Panels can be hidden.
2. **One primary action per view.** It is the only filled green button (Nest marker, Make cut plan, Inspect and group rolls, Prepare lay, Make bundles, Record bundle issue).
3. **Show the next step.** The workflow bar always offers "Next: …" for the current step.
4. **Simple first, advanced folded.** Rare options sit under "More options" or are hidden while their feature is switched off. Nothing is removed.
5. **Status is always visible.** Length, efficiency, copies and checks sit in one status bar under the marker.
6. **Big, labelled targets.** Every control is at least 40 px high; 48 px on touch screens. Icon buttons have a text label or a tooltip with the keyboard key.
7. **Plain words with units.** Labels name the thing; the unit shows inside the field (mm, m, %, s).
8. **Same parts everywhere.** One set of tokens and components for all screens, light by default, dark on request.
9. **Readable in any language.** Labels and buttons can wrap; nothing depends on English text width.
10. **Accessible by default.** Keyboard path for every action, visible focus, AA contrast, names for icon buttons, reduced motion respected.

## Design system

Tokens live at the top of `src/style.css`; icons in `src/icons.ts` (original SVG paths, Apache-2.0, no icon font, no CDN).

- **Colour:** green accent from the brand; neutral grey-green surfaces; status colours for ok, warning, danger and info. Light is the default. Dark and "follow system" are choices in the top bar. All text pairs meet WCAG AA (4.5:1); input borders meet 3:1.
- **Type scale:** 12 / 13 / 14 / 15 (body) / 17 / 20 / 26 / 32 px, system fonts only (Segoe UI on Windows, Noto/Nirmala for Hindi and Tamil). Hindi and Tamil get a taller line height.
- **Spacing:** 4, 8, 12, 16, 24, 32, 48 px.
- **Radius:** 6, 8, 12 px and pill.
- **Elevation:** 3 shadow levels (card, menu, dialog).
- **Motion:** 120 ms and 200 ms with one easing curve; off when the system asks for reduced motion.
- **Components:** buttons (primary, secondary, ghost, danger), icon buttons with tooltip and key hint, segmented controls, fields with units, cards, tables with sticky headers and a row filter, menus, toasts, empty states, dialogs, progress, chips and status badges.

## Information architecture

- **Sidebar:** Home, then four groups (Design, Marker and planning, Cutting floor, Reports and help). It can shrink to an icon rail; the rail is the default below 1400 px.
- **Top bar:** screen title, server status, a slot for the Feedback button (added by the feedback module before About), language, theme, server token (browser only) and About. On a phone these sit in a "More" menu.
- **Workflow bar:** the 8 steps (pattern, marker, cut plan, rolls, spread, cut, bundles, sewing) and a "Next" button.
- **Home:** start actions, continue where you left off, a 4-step "first marker" guide, the workflow map and common tasks.
- **Marker editor:** command bar (open, demo, save, export) → Setup panel on the left (Job, Fabric rules, Piece rules; Nest docked at the bottom) → toolbar → canvas → status bar → Tools panel on the right for the selected pieces (move, turn, select, clearance, marker end, measure).

## Factory-friendly checks

- 1366 x 768: canvas visible without scrolling; main action visible without scrolling on every screen.
- 1920 x 1080: content width is capped so lines stay readable on floor screens.
- Touch (`pointer: coarse`): 48 px controls on all screens.
- 390 px: Floor and Bundles work in one column; the top bar is one row.
- Print: lay sheets, cut plans and reports print without app chrome.

## What changed in this pass

| Area | Change |
|---|---|
| Tokens and components | New type scale (15 px body), 40 px controls (48 px on touch), radius, elevation and motion tokens; buttons, icon buttons with tooltips, segmented controls, fields with units, disclosure, menus, cards, chips, tables, dialogs, toasts, progress and empty states. Original icon set in `src/icons.ts`. |
| Theme | Light by default; dark or "same as computer" on request. Print is always light. |
| Shell | Icon rail (default below 1400 px), "Next: …" in the workflow bar, one-row top bar on phones with a "More" panel, Feedback button slot kept before About. |
| Home | Start actions, continue list, "first marker in 3 minutes" guide, common tasks, workflow map. |
| Marker editor | Command bar with Save and Export menus; Setup panel with Nest docked; one grouped toolbar; canvas fills the window; status bar; Tools panel for the selection; hideable panels; floating progress; empty state. |
| Cut plan, Compare | Numbered card forms; rare options folded; plan totals as tiles. |
| Floor, Rolls, Bundles | Tab bar with 48 px targets; roll choices as tap rows; large scan field; cards; backups out of the way. |
| Fabric and piece rules | Options of a switched-off feature stay hidden until it is switched on. |

App logic, data contracts, transport, ids and handlers are unchanged. The only behaviour changes are presentational: light default theme, the Nest dock visible on every Setup tab, the measure mode opening the Tools panel, and remembered panel and rail choices.

## Checks

- Vitest (worktree root, `npx vitest run marker`), web, wasm and desktop typecheck, web build.
- Smoke scripts `editor-smoke.mjs`, `floor-smoke.mjs` (with the language and manual check) and `pattern-smoke.mjs` run headlessly against the Vite app and the local Rust server. Headless Chrome needs focus emulation for the editor smoke; without it the "resolve by bump" step fails on the old code as well.
- Contrast of 29 token pairs in light and dark: all meet AA (4.5:1 text, 3:1 borders and focus).
- No unnamed controls on Home, Marker editor, Cut plan, Rolls, Bundles, Floor and Pattern; Tab order follows the layout; menus open, close with Escape and return focus.
- Screenshots: `<data-dir>/ux-shots/before/` and `after/` (1366 x 768, 1920 x 1080, 390 px for Floor and Bundles, dark samples), Hindi and Tamil samples in `lang-hi/` and `lang-ta/`.

## Open issues

1. New strings on Home, the editor toolbar and the Cut plan are English only. The Hindi and Tamil files cover navigation, Floor and "Next". They need a translation pass with operators.
2. Rolls and Bundles keep their existing forms; only the styling changed. Folding rare fields there needs template changes in `cutting-room.ts`, which other agents edit.
3. Pattern and Digitize are restyled only through shared tokens (their logic belongs to the pattern work).
4. Steppers (+/−) for plies and ratios are not built yet. Number fields keep keyboard arrows; the browser spinners are hidden inside unit fields.
5. Between 900 and 1500 px the Tools panel floats over the right edge of the canvas. Close it, or hide Setup, to see that edge.
6. The desktop file bar is restyled with CSS only and was not run in the Tauri window this round.
7. The website screenshots in `marker/apps/site/public/assets/screens/` show the old design and should be retaken.

## Quick Marker: photo to marker (round 0.5, phone first)

For tailoring shops and small units with paper patterns, no CAD and often only a phone. Screen `#/quick` (sidebar "Photo to marker", first card on Home). Code: `src/quick/` (see its README).

**User:** a tailor or cutting master on a mid-range Android phone, one hand, bright shop light, little English.

| Step | What the user does | Design choices |
|---|---|---|
| Start | Choose language, read the 3-picture guide (dark cloth, A4 sheet, phone flat above), choose Quick (A4 sheet) or Accurate (printed target), press **Take photo of pattern pieces** | One big button. Pictures say the same as the words. "Try a sample photo" shows the whole flow with practice pieces. |
| Photo | Take one or more photos; set the size of the pieces in each photo | `<input type=file accept=image/* capture=environment>` works on every phone. Each photo shows pieces found, a size-check chip and quality warnings in plain words **with a fix** ("Phone was not flat: hold the phone flat, straight above the pieces"). |
| Check | Tap a piece (on the photo or in the list); accept, delete, rename, size, copies (+/−), left/right pair, cut on fold, turn grain | Nothing is used until the user accepts it. Suggestions are chips the user taps ("Looks like: Sleeve · maybe", "Left/right pair?", "Cut on fold?", "Same as a piece in another photo: remove this copy?"), each with "Not this". Accepting a piece also confirms the grain arrow shown on it (dashed until confirmed), because the engine needs a confirmed grain on every piece. |
| Fabric | Width with inch/cm toggle and preset chips (36, 44, 58, 60 inch; 90, 112, 150, 160 cm), open / folded / tube pictures, one-direction switch, garments per size with steppers, search time | Big number field; the other unit is shown under it. Only sizes with accepted pieces appear. |
| Marker | Watch progress; see the best marker so far; **Stop and use best** | Nesting runs in short rounds that start from the best result, so a real marker shows after one round and Stop keeps it. |
| Use | Length, fabric used by pieces, metres per garment; optional "your usual fabric per garment" shows the saving; print A4/A3 tiles, chalk guide, projector, share PDF/PLT, open in Marker editor | Web Share API with the file where the phone supports it, otherwise download (save dialog on desktop). |

**Projector mode:** full screen, white on black (or black on white), 1:1 at the calibrated scale. A 100 mm square is shown; the user measures it with a tape and enters width and height; each axis is corrected (`projector.ts`, tested) and stored on that device. The marker is split into overlapping parts that fit the screen (Previous/Next, arrow keys). Selvedge (Y = 0) and marker start (X = 0) are labelled; reference points match the chalk guide. The screen is kept awake with the Wake Lock API where available; controls fade after 5 s.

**Offline install (PWA):** `public/manifest.json`, `public/sw.js` and PNG icons. The build writes `precache.json` (every built file, same server only); the service worker never caches `/api/`. Not registered in the desktop app or the dev server. Browsers allow service workers only on `https://` or `localhost`, so a phone that opens `http://<shop PC>:7878` works online but gets no offline copy.

**Accessibility and language:** all controls 48 px (44 px for segmented choices), names for every control (checked headlessly on every step), focus kept after each re-render, a live region announces results, reduced motion respected, Hindi and Tamil drafts for every new string (`src/quick/locales`, marked for native review).

**Honest limits:** shape reading, assist, job building, tiled/chalk PDFs and the projector layout are Rust WASM (`@beta/marker-wasm` Quick Marker API v1). Nesting and PLT/full-size PDF export need the desktop app or the shop's `beta-marker serve` on the same network; the browser WASM has no nester yet, so a phone alone cannot make the marker. Practice mode is labelled on every step and its layouts are marked "not a real marker".
