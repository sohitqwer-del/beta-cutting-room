# Style flow (from scratch to cut)

One guided job in 8 steps: Style → Pattern → Grade → Lay → Gaps → Stripes → Marker → Cut. Screen `#/style` (index 12), sidebar "Style: pattern to cut", first card on Home, and step 1 of the cutting-room workflow. Design notes and the audit: `../../UX.md` ("Style flow").

| File | Role |
|---|---|
| `adapter.ts` | The only place that calls the engine (STYLE-API v1, `pattern/docs/STYLE-API.md`): `loadStyleCore()` for documents, blocks, charts, edits, seams, jobs and overlays; `PatternCore` for inspect, nest view, snap and pattern exports; `MarkerCore` for checks and match-point positions; nesting through the Quick Marker nester (browser Web Worker first, then desktop or `beta-marker serve`). |
| `mock.ts`, `mock-blocks.ts` | Test double of the Style engine for unit tests only (`preview.test.ts`): the same API on the Rust Pattern and Marker cores plus small TypeScript glue and original formula blocks. The app never loads it; a Pattern WASM without `style_command` shows a clear error with the build command. |
| `state.ts` | Steps, guards ("why is Next blocked"), lay/gap/stripe settings, engine job settings, spread fabric, saving on this computer. Pure functions. |
| `canvas.ts` | Drafting canvas: tools, snapping with guides, exact-value popover, inspector, piece list, Arrange. Picking (points, lines, piece edges, pieces) is the Rust `hit_test`. On a piece cut on the fold, seam allowance changes both twin edges (the engine rejects one-sided widths); mirrored notches and drill holes are added by the engine on a folded or tube hand-off. Turns pointer and key input into STYLE-API operations; draws only Rust geometry. |
| `labels.ts` | Where a piece label fits inside its own piece at the current zoom (name, then "size · cut", shortened or hidden when small). Display only. |
| `match-report.ts` | Groups the engine's per-garment match results into one report per style rule (matched, off with the worst mm, not in the marker) and thins dense stripe lines for drawing. Pass/fail and mm come from Rust. |
| `snap.ts`, `view.ts`, `units.ts` | Pointer snapping and picking, screen/world view maths and ruler ticks, cm/inch conversion and parsing (inch fractions, comma decimals). |
| `ui.ts`, `art.ts`, `style.css` | The screen and its pictures. `style.css` is imported by `main.ts` after the design system. |
| `i18n.ts`, `locales/` | English source; Hindi and Tamil drafts for every string, marked for native review. |
| `style-smoke.mjs` | Page helpers for a headless driver (positions, session, captured export files, accessibility spot check). |

Checks (worktree root): `npx vitest run marker/apps/web/src/style` (unit tests; `preview.test.ts` runs the test double on the local Rust WASM when it is built). Headless end-to-end: a CDP driver opens `#/style` (real Style engine and Rust nester), imports `/src/style/style-smoke.mjs`, and sends real mouse and keyboard input through every step.

Address switch: `?style-practice=1` replaces nesting with an instant practice layout that is marked "not a real marker" (headless checks only).
