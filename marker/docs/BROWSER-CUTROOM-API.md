# Browser cutting room

Round 0.7 (plan7). The browser edition now runs the whole cutting-room journey on
the device: cut plan, roll register and inspection, roll allocation (spreading
plan), bundles, tickets and stickers, cutting reports, marker reports (TXT, XLSX,
mini PDF, labels), floor tracking and PLT import for Compare. Nothing leaves the
device. All calculations are Rust.

## One implementation for every transport

```
                beta-marker-api (Rust, no transport)
         roll/*  cutroom/*  plan  import?format=plt  check_plan
          /                     |                         \
 beta-marker serve        desktop app (Tauri)      beta-marker-cutroom-wasm
 (HTTP, native threads)   (IPC, native threads)    (browser, lazy-loaded)
```

- `crates/beta-marker-api`: the route handlers. Before this round, `serve.rs` and
  the desktop `api.rs` each had a copy, and the copies had drifted (the desktop
  still bundled surplus folded layers). Both now call this crate.
- `crates/beta-marker-cutroom-wasm`: the browser module. `api_request(url, body)`
  answers the routes above plus `/api/validate` and `/api/export` (all formats,
  including report, xlsx, mini and labels). `plan(request, onProgress)` runs the
  planner with a one-thread browser nester; call it in a Worker.
- The planner can reuse measured markers: `PlanRequest.markers =
  [{ratio, result}]`. A known marker is used only if it passes validation for the
  order's job; otherwise that ratio is nested again. This makes re-planning fast and
  makes native/WASM parity testable.

## TypeScript

```ts
import { loadCutRoomCore, createCutPlanner } from '@beta/marker-wasm';
const core = await loadCutRoomCore();             // wasm/beta_marker_cutroom_wasm.js
const reply = core.request('/api/roll/spread', bytes); // {status, mime, extension, body}
const planner = createCutPlanner();               // Worker
const plan = await planner.plan(request, partial => show(partial));
planner.stop();  // resolves with the best checked partial plan (warning added)
```

In the static build, `apiFetch` sends these routes to `apps/web/src/browser-cutroom.ts`:
`/api/plan` (Worker; progress as the `beta-plan-progress` window event, stop with
`stopBrowserPlan()`), `/api/nest` (Compare re-nest, browser nester),
`/api/roll/*`, `/api/cutroom/*`, `/api/validate`, `/api/import?format=plt` and
`/api/export?format=report|xlsx|mini|labels`. Factory exports and DXF import stay on
export7's routes in the marker module. Only `/api/digitize` (board photo) still
needs the desktop app or a server.

## Storage and backup

`apps/web/src/browser-store.ts`: the cut plan is kept in IndexedDB (localStorage if
IndexedDB is blocked). Rolls, bundles, Style and Floor keep their existing
localStorage keys. `makeBackup()` writes one JSON file with every `beta-*`
localStorage entry and every IndexedDB record; `restoreBackup()` replaces this
browser's BETA work with it after checking the format. The browser-edition notice
has "Save a backup file" and "Restore from a backup file".

## Plies

See [PLIES.md](PLIES.md): a ply is one fabric layer; cut order and plan format v2.

## Repeatable parity test

From `marker/`, after the remapped release WASM build:

```sh
cargo run --release -p beta-marker-api --example cutroom_parity -- out/cutroom-parity
node packages/marker-wasm/scripts/cutroom-parity.mjs out/cutroom-parity
```

Inputs are the MIT FreeSewing Simon fixture (open 1500 mm and folded 800 mm) and the
synthetic demo roll register. Nesting is time-based, so it is not compared: a first
native run records the first marker measured for each ratio, then native and WASM
plan again with those markers. Each later step uses the previous step's output.
XLSX files are compared entry by entry except `docProps/core.xml` (creation time).

## Round 0.7 verification (22 October 2026)

| Result | Identical / compared |
| --- | ---: |
| Cut plans (open; folded with 12 plies = 6 passes) | 2 / 2 |
| Roll inspection, roll allocation (spreading plan) | 4 / 4 |
| Physical lay from spread, bundles | 4 / 4 |
| Tickets (A4, label 100x50), stickers | 6 / 6 |
| Re-cut record, cutting report, cost (open only) | 5 / 5 |
| Marker report TXT, mini PDF, labels PDF, XLSX | 8 / 8 |
| PLT export and PLT import (Compare) | 4 / 4 |
| **Total** | **33 / 33** |

export7's factory-file parity still passes 46/46 files and 6/6 import reports.

WASM sizes (bytes, gzip level 9). The cutting-room module loads only when a
cutting-room route is first used; the service worker still precaches it for offline use.

| Module | Raw | Gzip |
| --- | ---: | ---: |
| Cutting room (new): planner + nester, rolls, cutroom, reports, PLT import | 4,875,216 | 1,421,529 |
| Marker (unchanged sources) | 4,368,655 | 1,283,784 |
| Pattern | 5,028,630 | 954,468 |
| Floor | 3,799,359 | 932,068 |

All four packaged modules pass `node deploy/release-build.mjs --verify`.

Headless journey on the static build (1366x768, real Rust, no server): DXF open,
nest, marker report TXT/XLSX/mini/labels, cut plan (14 markers measured in the
Worker, progress shown), roll import and inspection, roll allocation, bundles,
tickets and stickers PDF, cutting report, Floor lay prepare and lay sheet, Floor
tracking spread -> cut -> bundled, Compare with the exported PLT, reload keeps the
plan, backup file. 14/14 steps; 0 `/api` requests, 0 external requests, 0 console
errors.
