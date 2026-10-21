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

## Style flow: from scratch to cut (round 0.6)

The factory owner: "The app is half-baked. I need to make patterns from scratch and work with patterns, then size-scale them, then select the lay and the cut gaps, the stripes, the overlay." Screen `#/style` (sidebar "Style: pattern to cut", first card on Home, workflow step 1). Code: `src/style/` (see its README).

**Users:** pattern masters and cutting masters. They know garments, grading and lays well. They are not CAD experts and do not think in formulas or object ids. They work in cm or inch, at a 1366 x 768 PC or a 10-inch tablet.

### Audit of the Pattern tab and the Pattern → Marker path (before)

Walk-through as a pattern master: make a T-shirt in S–XL, add notches and seam allowance, choose a folded lay with 1 cm stripes, and get a marker. Before shots: `<data-dir>/style-ui-shots/before/`.

| # | Half-baked or blocked | Where | Priority |
|---|---|---|---|
| 1 | The drawing starts 660 px down at 1366 x 768, under file inputs, a status line, a toolbar and 7 mode buttons. | Pattern | P1 |
| 2 | No garment blocks. The only start is a blank formula draft or a teaching bodice. | Pattern | P1 |
| 3 | Drawing a point opens a dialog that asks for an "Object ID", empty x/y text and a "measurement / variable picker". 35 tools are text buttons named after geometry ("Perpendicular foot", "Two circles", "Circle tangent contact"); none has an icon. | Draft | P1 |
| 4 | No direct drawing: no click-click lines, no rubber band, no typed length at the cursor, no curve handles to drag (only fixed numeric points move). Snapping is two check boxes with no guides. | Draft | P1 |
| 5 | A piece is made by "Start a new piece", then clicking paths in order and reversing edges by hand. | Piece | P1 |
| 6 | Notch, drill and button marks need the edge number ("starts at 1") and the distance along it typed in a form. Dart, pleat, split and mirror share one 13-field form. | Piece | P2 |
| 7 | Sizes and measurements are in mm only, in a raw table under "Measurements"; grading is a separate mode with dx/dy fields and a "Nest reference point" list of ids. | Measurements, Grading | P2 |
| 8 | The status line keeps saying "Loading local draft tools…" after the tools have loaded. | Pattern | P3 |
| 9 | "Send sizes to marker" sits in Output. The marker opens as a thin empty strip with "4 issues · 4 not placed" and no lay, gap or stripe settings carried over. | Pattern → Marker | P1 |
| 10 | Lay, gaps, stripes and match points are spread over three Setup tabs of the marker editor. Spread is a plain list ("open", "folded", "tubular"); match points are typed as x/y or clicked on a small outline; rules read "front/seam-1 → back/seam-1". | Marker setup | P2 |
| 11 | Stripes show only as faint lines behind the marker, not on the pieces. There is no "matched / not matched". | Marker | P2 |
| 12 | The hand-off is one way. The editor has no way back to the pattern, and sending again replaces the marker. | Marker | P2 |
| 13 | Nothing guides the user from pattern to lay to gaps to stripes. The workflow bar jumps from Pattern to Marker. | Shell | P2 |

What already worked and is kept: the Rust engine (formulas, grading, piece tools, DXF/PDF/PLT, seam lengths), the formula Pattern screen for advanced users (now "Pattern formulas" in the sidebar), the marker editor and the cut plan.

### Design

One guided job in 8 steps with the same step bar as the cutting-room workflow and a "Next" on every step. Every step can be revisited; a blocked step says why next to the Next button.

| Step | What the user does | Design choices |
|---|---|---|
| 1 Style | Name (only required field), style number, garment, season, fabric, notes; size chart preset and the sizes to include; start from a block gallery (T-shirt, shirt, trouser, skirt, kurta, polo, track pant) or a blank sheet, or open a style file | Pictures for every block. Buyer and order details stay out. The name defaults to the block name. |
| 2 Pattern | Draw and change pieces on a CAD-like sheet | Left: 19 tools with icon, name and key (V P L C A R · O M X J D K · N H B T S G F) plus "Rectangle piece". Centre: grid paper, rulers in cm or inch, live guides for point, line-up and 15° snaps, a rubber band with live length and angle, curve handles to drag, box select. The exact-value popover sits in the corner of the drawing (length and angle for lines; width for seam allowance, dart, pleat; count for button rows) so it never covers the next click. Under the drawing: what to do next for the current tool, and the cursor position. Right: inspector for the selected point, line or piece (name, X/Y, "Free this point", quantity, pair, fabric, seam allowance per edge, marks) and a Pieces list. Closing a line chain makes a piece. |
| 3 Grade | Edit the size chart; see all sizes stacked; click a corner for a per-point rule; read the seam check | Chart in cm/inch with base size, add and remove size (a new size starts one grade step on). Seam check shows the length difference per size as a pass/fail chip. |
| 4 Lay | Width (inch/cm with preset chips), spread (open / folded / tube), face (up / down / face to face), nap (any / one way / one way per garment), plies with steppers, garments per size and order | Big picture cards. Even plies enforced where two layers are cut together. "Lays needed" per size. Fabric group chooser when a style mixes fabrics. |
| 5 Gaps | Piece gap, edge gap, end allowance, per-piece buffer and fusing block | Live preview to scale with two real pieces, dimension arrows, shaded end allowances. |
| 6 Stripes | Plain / stripes / checks, direction, repeat, offset, tolerance, stripe colours; match points | Stripes drawn over the real pieces with graded sizes dashed on top. Match: tap a point on piece A, then on piece B; each rule shows "Not checked yet", "Matched" or "Not matched by …". |
| 7 Marker | Choose Fast / Better / Best and make the marker | The marker fills the view with stripe lines clipped to the pieces, match circles (green matched, red not) and the previous marker as a ghost. Status: length, efficiency, per garment, spread length, checks, stripe match. Changing a setting marks the marker out of date. |
| 8 Cut | Files and the next floor steps | Pattern PDF (A4/A3, 1:1), DXF-AAMA, PLT; marker PLT, PDF, DXF (desktop or server) and tiled PDF (everywhere); stripe guide lines as an option. "Make a cut plan" fills the order and plies; "Print a spreading sheet", "Open in marker editor". |

**Seamless hand-off:** the marker goes to the editor, cut plan, spreading and bundles screens directly (no file). In the editor a banner offers "Back to style" and "Use this edited marker".

**Language and access:** every string is in `src/style/locales` with Hindi and Tamil drafts marked for review. All controls have names; pictures are decorative with text labels; the drawing has a role description and a keyboard path (tool keys, arrows move selected points by 1 mm or 10 mm with Shift, Delete, Esc, +/−/0) and the inspector gives exact values for every point; touch: one finger pans, two fingers zoom, 48 px targets.

**Engine:** `src/style/adapter.ts` is the only file that calls the engine (STYLE-API v1, `pattern/docs/STYLE-API.md`): the Rust Style, Pattern and Marker cores in WASM and the Rust nester (browser worker with matched nesting, or the desktop app / `beta-marker serve`). If the local Pattern WASM has no style tools, the screen says so and gives the build command; there is no stand-in engine in the app (`mock.ts` is a test double for unit tests).

### Checks (Style flow)

- Vitest: `npx vitest run marker/apps/web/src/style`: units and inch fractions, view maths and ruler ticks, snapping (points, 15° angles, line-up guides, grid), edge picking, loop chaining, the step state machine and its guards, engine settings, spread fabric, saving, the adapter (real engine, preview fallback, missing WASM, validator-based match status), stripe clipping, and string drafts (same placeholders in Hindi and Tamil). `preview.test.ts` runs the preview engine on the local Rust WASM: all 7 blocks valid in all sizes, formula-preserving point moves, blank-sheet piece, job with gaps/buffers/per-size match rules, stripe overlay.
- Web, WASM and desktop typecheck; web build; `npm run build:static -w @beta/marker-web`.
- Headless end-to-end with real mouse and keyboard input (Chrome DevTools protocol) against the real Style engine and the real Rust browser nester, in the dev app and in the static browser build. Steps: blank-sheet piece drawn with the Line tool and a typed length/angle, closed into a piece; T-shirt block in S–XL; armhole curve handle dragged (formula kept); notches on front and back side seams (with their fold twins); 1.5 cm seam allowance (both fold twin edges); size chart edit; folded 60-inch lay; 0.5 cm piece gap and a sleeve buffer; 1 cm stripes along the length with front/back side hems matched; 20 s nest; cut-step files; editor hand-off and back. Real engine result: all marker checks passed, 4 of 4 side-hem matches, stripes clipped to the pieces; pattern PDF, DXF-AAMA and PLT and a tiled marker PDF exported and checked by file signature. No console errors, no external requests.
- Accessibility spot check on every step: no unnamed controls; hit areas at least 32 px (chart base-size radios and table check boxes were enlarged after the first run).
- Screenshots: `<data-dir>/style-ui-shots/before/` (old Pattern tab and hand-off), `after-1366`, `after-1920`, `after-1024-touch`, `lang-hi`, `lang-ta`, `dark-1366`, `static-1366` (browser edition).

### Open issues (Style flow)

1. Per-rule match status with the phase error in mm is not in the overlay yet (contract question 1). The UI shows "Matched / Not matched" from the Rust validator and marks the points; it shows no mm for real-engine markers.
2. The engine's blocks start with all pieces at the origin. The UI lays them apart once with "Arrange" (move_point, formulas kept, undoable). If the engine lays blocks out itself, Arrange does nothing.
3. "Offset" works on whole pieces only (offset_piece); offsetting a single line or curve needs an engine operation (contract question 3).
4. Edge picks (notch, dart, seam allowance, fold) are worked out in the browser from the engine's stitch paths (contract question 2). On fold pieces the twin edge is i ↔ n−1−i and the twin notch sits at the twin length minus the distance; removing one of a pair by hand makes a folded hand-off fail with the engine's message.
5. Hindi and Tamil strings are drafts for native review.
6. Marker PLT, DXF and full-size PDF need the desktop app or the shop server; the browser edition offers the tiled PDF.
7. The previous-marker overlay lasts for the session; markers are not stored with the style.
8. The formula Pattern screen ("Pattern formulas") stays for advanced drafting and is not merged with the new canvas.
9. The desktop window was not opened this round (typecheck and shared web build only).

## Style flow polish (round 0.6, polish)

The lead walked the Style flow on the screenshots and asked for it to "feel finished". Before shots: `<data-dir>/style-polish-shots/before-1366/` and `before-1024-touch/` (real engine, same smoke as round 0.6).

| # | Rough edge (before) | Fix |
|---|---|---|
| 1 | Pattern canvas: the engine's production label ("Front \| size M \| cut 1 + 0 mirrored \| fabric shell") ran across other pieces; small pieces had names wider than the piece. | `labels.ts` places the name and "M · cut 1 (+ 1 mirrored, fabric if not shell)" at the roomiest point inside the piece, sized for the zoom (10–17 px). Too small: name only, then shortened with "…", then hidden (the piece keeps a tooltip and its row in Pieces). The production label is not drawn on the canvas (it is still in the exported files). User text marks are shortened to the piece width. |
| 2 | "Viewer mode · local server offline" next to a green "Rust style engine" chip. | One status in the top bar: "Working offline in this browser" (tooltip: what needs the desktop app or a shop server), in Hindi and Tamil too. The Style header shows a chip only while loading or on failure. |
| 3 | Match rules showed "Not checked yet" or "Not matched" with raw point ids. | Stripes step: "Pairs to match" with a summary and a colour key; each pair shows Matched (green), Off by up to N mm (red, from the engine's match report when present), Not in the marker (amber) or Out of date, plus **Show on marker**. Point names read "Front · Side hem". |
| 4 | Marker: 1 cm stripes merged into grey; match dots without a key; no way to find one pair. | Dense stripes are thinned so drawn lines stay 6 px apart (the key says "every 5th line shown"); pieces are paler under stripes; outlines on top. Match points are rings with a light halo (green matched, red off). The overlay switches carry their key swatch, followed by sizes and the fold edge, in one row. Show on marker highlights that pair (others dimmed, partners linked) with a banner and "Show all". The drawing takes the free height, so length, efficiency and checks stay in view at 1366 x 768. |
| 5 | No hints beyond one long line; empty and error states were bare. | "How this works" strip per step: the step's purpose and three numbered actions. It opens the first time a step is shown and folds into a header button after that. Empty states for no match pairs, previews without a closed piece, the size nest, loading blocks and a running search. Engine and export errors in plain words (a JSON list of issues becomes "N problems, for example: …"); a failed marker says what to check. |
| 6 | Walk as a new pattern master at 1366 x 768 and 1024 touch. | The whole page scrolled by about 20 px at 768 px height (icon rail and an off-screen live region were taller than the window), so the top bar slid away when a field got focus. Fixed in the shell. The step bar shows numbers only for the other steps whenever names would overflow (1024 px, Tamil). The size chart and gap tables no longer have their own scroll inside the page. The stacked sizes stay in view while the chart scrolls. Gap table headers wrap; the edge-gap label sits below the fabric. Focus stays on a grade or match point after a re-render. |
| 7 | Strings. | Every new string has Hindi and Tamil drafts (`src/style/locales`, `src/locales` for the status), marked for native review. |

Engine polish (`marker/polish-engine`, merged on mvp bb79b01, STYLE-API "Polish additions") is wired in: the per-garment match report (`overlay.matches`, or the validator's `matches`) gives each pair its status and offset in mm ("Matched · 4 of 4 · within 0.2 mm", "Off by up to 3 mm"); all canvas picking (points, lines, piece edges with `edge` and `along_mm`, piece interiors) is the Rust `hit_test`, and the browser edge maths is gone; darts pass `seam_edge`; Offset on a line, cubic or arc makes a linked parallel copy (`offset_path`) and the inspector says what it follows; a new **Extend / trim** tool (key E) makes a copy of a straight line that ends on another line (`extend_trim`); linked derived pieces say they regrade from their source; blocks arrive laid out (Arrange stays for later edits); the canvas no longer adds twin notches and drill holes on fold pieces because the engine adds the mirrored partners on a folded or tube hand-off (seam allowance still changes both twin edges, as the engine requires symmetric widths).

Checks: Vitest (`polish.test.ts`: label placement inside L shapes, zoom levels, width limits, shortening; match report grouping, worst mm, not placed, engine report shape; stripe thinning; plain error text), web typecheck, web build and `build:static`, real-engine smoke 14/14 (the 11 round-0.6 steps plus Offset on a line, Extend / trim, and "match report and Show on marker") at 1366 x 768, 1024 touch, Hindi, Tamil and dark. Screenshots: `<data-dir>/style-polish-shots/after-*`, `lang-*`, `dark-1366`, `static-1366`.
