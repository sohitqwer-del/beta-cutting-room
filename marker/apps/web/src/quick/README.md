# Quick Marker (photo to marker)

Phone-first flow: photo of paper pattern pieces → check pieces → fabric and order → nest → print, chalk, projector or share. Design notes: `../../UX.md` ("Quick Marker").

| File | Role |
|---|---|
| `adapter.ts` | The only place that calls the engine. Real: `loadQuickMarkerCore()` from `@beta/marker-wasm` (Quick Marker API v1, `marker/docs/QUICK-MARKER-API.md`); scans run in `scan-worker.ts`. Nesting: local Sparrow WASM in a module Web Worker, in short validated warm-start rounds. If the worker cannot load, `/api/nest` provides the desktop/server fallback. |
| `mock.ts` | Practice engine: original synthetic shirt pieces, a drawn sample photo, shelf layout, tiny PDFs. Used for tests, the smoke and "Try a sample photo". |
| `state.ts` | Steps, guards and review actions as pure functions. Nothing from a photo is used without a user action. |
| `units.ts`, `geometry.ts`, `projector.ts` | Unit conversion, savings, homography for outlines over the photo, projector calibration and parts. |
| `ui.ts`, `quick.css`, `art.ts` | The screen. |
| `i18n.ts`, `locales/` | English source; Hindi and Tamil drafts for native review. |
| `pwa.ts`, `../../public/sw.js`, `../../public/manifest.json` | Offline install. The Vite plugin `offline-precache` writes `dist/precache.json`. |

Checks: `npx vitest run marker/apps/web/src/quick` (worktree root). Smoke in the Vite app opened at `?quick-mock=1#/quick`:
`const s = await import('/src/quick/quick-smoke.mjs'); await s.runQuickSmoke();` (nesting uses the local Rust WASM worker when available, then the native fallback; practice results remain labelled).
