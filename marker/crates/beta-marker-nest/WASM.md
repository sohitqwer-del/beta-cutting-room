# Browser nesting

## Feasibility

**Sparrow v0.3.0 runs in `wasm32-unknown-unknown` without a fork.**
The pinned revision is `c5e64b3b7a0e102de851f44c1f8dd2520a30633b`.
Jagua remains the unmodified crates.io `=1.0.0` dependency. No upstream
source, vendored copy, workspace dependency version or patch was changed.
All 81 Jagua source files match the cached crates.io archive, whose SHA-256
matches `Cargo.lock`; the Sparrow checkout has no tracked modifications.
The chosen browser path needs no upstream patch: it first publishes a checked
BETA marker, then runs Sparrow inside a disposable Worker and keeps the best checked result.

Integration base: `marker/mvp` at `4c90060`, including Quick Marker and the
layer-aware fold update in `91c32f5`. The additional clock import in
`v02/layers.rs` keeps that new path usable in WASM too.

The initial target check and a real headless Chrome Worker search passed.
The demo returned all eight placements, passed the independent core validator,
and finished after the local asset server had been stopped. No nesting server
was running. This feasibility check completed within the first hour.

| Possible blocker | Evidence and solution |
|---|---|
| `std::time::Instant` | Sparrow and Jagua already use `jagua_rs::Instant`, an alias for MIT/Apache-2.0 `web_time::Instant`. BETA's adapter, profiler, tuning and bounded packer now use the same alias. `std::time::Duration` is safe and stays unchanged. |
| Thread creation / Rayon | Sparrow's `Separator::new` already skips native pools on wasm32. The pinned Rayon core 1.13.0 provides a serial global fallback when thread creation is unsupported. Set one separator worker and one run. BETA's native low-core fallback no longer diverts ordinary WASM jobs away from Sparrow. No WebAssembly threading, shared memory, COOP or COEP headers are needed. |
| Randomness | Upstream already enables getrandom 0.4 `wasm_js`. Explicit seeded Xoshiro streams work. Omitted seeds use browser randomness and are restricted to JSON-safe integers. |
| File I/O / signals | The library optimizer does not need CLI logging, file listeners, SVG export or Ctrl-C. BETA supplies its own in-memory listener. Sparrow cfg-gates its Ctrl-C dependency off wasm32. |
| Browser event loop | Synchronous search runs inside a module Worker. Round 0 uses the BETA-owned bounded packer and publishes a complete checked seed. Sparrow uses its own first construction; subsequent rounds warm-start from the best marker. The host acknowledges progress before the next round. |
| Time limits | A bare Sparrow round on Lectra 14851 ran for over 14 minutes despite a two-second allowance. The production helper now has a foreground host watchdog at the total budget plus five seconds; Stop has a five-second watchdog too. It terminates a stalled Worker and returns a private copy of the last checked marker. Before any checkpoint exists, it rejects instead. Browser timer scheduling is still outside Rust's control. |
| Stack / memory | The ordinary wasm32 stack suffices for tested inputs. Linear memory and per-round times are recorded by the browser harness. No phone OOM limit or maximum practical job size is claimed. Disposing a nester terminates its Worker and releases the WASM instance. |
| Garment rules | The shared adapter handles rotation, handed demand, gap, shrinkage and bundles. Existing BETA bounded packing handles locked placements, defects and fold constraints. Every returned incumbent passes `beta_marker_core::validate` on the original job. A search that cannot produce a checked complete marker returns an error; constraints are never relaxed. |

Changes to the fold-owned files are limited to clock imports and the WASM
low-core routing condition. Session state and browser controls are in the
separate `browser.rs` file.

## API and integration

```ts
import { createMarkerNester, nest } from '@beta/marker-wasm';

// Convenience API: creates and disposes one Worker.
const result = await nest(job, {
  time_limit_s: 30, round_time_s: 2, seed: 1,
}, progress => draw(progress.result));

// Reuse loaded assets for repeat jobs; all subsequent search is local.
const nester = createMarkerNester();
await nester.ready;
const pending = nester.nest(job, { time_limit_s: 60 }, progress => {
  draw(progress.result); // complete, independently validated incumbent
});
// Stop button: nester.stop(); retains the latest checked marker.
// If nester.closed is true after a watchdog event, create a new instance.
const best = await pending;
nester.dispose();
```

`BrowserNestOptions` accepts `time_limit_s` (0.001–3600, default 30),
`round_time_s` (0.05–30, default 2), a non-negative safe-integer `seed`,
`early_termination`, `fixed`, and `warm_start`. Browser execution is always one
run with one separator worker; unknown options are rejected by Rust.
The BETA seed and first Sparrow attempt use the supplied seed; later attempts increment it. The selected result records
its winning seed. Wall-clock cutoffs preclude byte-identical replay.

`NestProgress` contains the best `result`, `round`, `seconds`, `round_seconds`,
`improved`, `done`, `cancelled` and `memory_bytes`. Memory is allocated WASM
linear memory sampled after completed rounds, not browser process RSS or live heap.
Round 0 is the BETA seed. Memory can be higher during an interrupted round.
Watchdog recovery changes only runtime and warning metadata; it preserves the
checked placements and dimensions. Callback payloads cannot mutate the retained copy.

`NestSession` in the generated WASM bindings exposes `step()`, `cancel()` and
`free()`. A low-level synchronous `nest(jobJson, optionsJson, callback)` is also
exported; its callback receives a JSON progress record and may return `false`
to stop. These synchronous raw APIs have no host watchdog. UI callers must use
the TS Worker helper for bounded waiting and responsive controls.

Quick Marker's `adapter.ts` loads the Worker first. Desktop/serve remains a
fallback if the Worker cannot initialize. Search or validation errors do not
silently trigger server work. PDF sharing uses local Quick Marker exports;
HPGL still needs the native exporter. The production Vite build bundles the
Worker, and its existing PWA precache must include that asset and the WASM.

No new licence category was introduced. The direct `js-sys` dependency is
MIT/Apache-2.0 and was already present transitively. BETA code remains Apache-2.0.

## Measurement method

Inputs: public FreeSewing Simon, Teagan and Titan `-M.job.json` fixtures and
the first ten tasks in the existing frozen Lectra development-40 JSONL.
Selection occurs before results are examined. Geometry and rules are unchanged.
Lectra geometry stays outside committed files. Dataset attribution:
Lallier et al., Zenodo 7030786, CC BY 4.0. These are development cases, not a
held-out commercial comparison; unknown original sheet semantics still apply.

Native: Sparrow, 60-second allowance, seed 1, one separator worker.
WASM: independent 30- and 60-second runs, seed 1, two-second warm-start rounds,
one worker, and a budget-plus-five-second host watchdog. WASM's total budget includes repeated setup and validation, whereas
native's allowance is the optimizer search budget. Wall times cover nesting
calls, including preparation and validation. Asset loading, initial WASM module
initialization and native input-file reading are excluded.

Hardware: Intel Core i9-11900H laptop, 32 GB RAM, Windows. Toolchain:
Rust/Cargo 1.98.1, wasm-bindgen 0.2.129, headless Chrome 155.0.8059.85.
This is a single-pass development comparison on a shared laptop. Other users'
background load is not controlled; no other build or benchmark from this task
runs alongside the measurements.
The native cases were collected in two sessions after an interruption, using
the same executable and per-case controls. Each completed case is retained once.

The browser test uses headless Chrome and never opens a desktop window.
Chromium's CPU throttle acts on the renderer main thread
([Chromium implementation](https://chromium.googlesource.com/chromium/src/+/99baeeafe29b54168bc18411791025d4a9419eea/third_party/blink/renderer/platform/scheduler/public/thread_cpu_throttler.h)).
The 4x comparison runs the **same Rust BrowserSession** on the throttled
renderer thread, so the throttle applies to the computation.
Actual Worker operation, cancellation and UI responsiveness are checked
separately. This is a laptop slowdown proxy, not a measurement on a phone.
Each final timed trial runs in a fresh headless process. Diagnostic partial
runs with bare Sparrow and a BETA warm-start were kept separately and are not
included in the final comparison. CDP sends every completed
Rust checkpoint to the external Node controller. That controller implements
the same budget-plus-five-second cutoff as the production Worker helper; a
stalled renderer is terminated and its checked checkpoint is retained. Every
returned benchmark marker is replayed through the native independent validator.
Round timings cover completed rounds; interrupted rounds are censored by the
host deadline. A fixed-work 1x/4x calibration is recorded for each trial.
The calibration values are saved alongside each trial. This is a coarse
slowdown proxy, not exact phone-equivalent hardware.

Reproduction, from repository root (output paths are examples):

```sh
# Set CARGO_TARGET_DIR, CARGO_BUILD_JOBS=3 and WASM_BINDGEN first.
node marker/packages/marker-wasm/scripts/build-wasm.mjs
node marker/packages/marker-wasm/scripts/prepare-wasm-bench.mjs \
  /path/to/development-40.jsonl marker/out/wasm-bench/cases.json
cargo run --release --manifest-path marker/Cargo.toml \
  -p beta-marker-nest --example wasm_bench -- \
  marker/out/wasm-bench/cases.json marker/out/wasm-bench/native.jsonl 60 1
node marker/packages/marker-wasm/scripts/wasm-benchmark.mjs \
  marker/out/wasm-bench/cases.json marker/out/wasm-bench/browser-final 2
# Replay every returned marker with the independent native validator:
cargo run --release --manifest-path marker/Cargo.toml \
  -p beta-marker-nest --example wasm_bench -- --validate \
  marker/out/wasm-bench/cases.json \
  marker/out/wasm-bench/browser-final/wasm-results.jsonl \
  marker/out/wasm-bench/wasm-validated.jsonl
```

Run native and browser benchmarks sequentially, not concurrently. Raw results,
geometry and browser profiles stay in ignored output directories.

The production app test serves the built app under `/app/`, waits for its
service worker cache, stops the server and reloads offline. It then uploads a
public synthetic A4 photograph through the visible file input, confirms the
pieces, nests, and checks local PDF/projector output. It does not enable mock
mode or call a nesting API:

```sh
cargo run --manifest-path marker/Cargo.toml -p beta-marker-wasm \
  --example browser_photo -- marker/out/wasm-bench/synthetic-a4.png
node marker/packages/marker-wasm/scripts/quick-offline-test.mjs \
  marker/apps/web/dist marker/out/wasm-bench/synthetic-a4.png \
  marker/out/wasm-bench/quick-offline
```

This synthetic flow verifies integration. It does not establish camera-scale
accuracy or performance on a real phone. Photo digitizing keeps its existing
operator review and calibration limits.

## Results and verification

The final merged module is **3,337,466 bytes**, or **1,042,456 bytes with gzip
level 9**. It includes the existing photo scanner, core/editor and Quick Marker
PDF exports as well as nesting. This is the full module shipped by this build.
WASM SHA-256: `8513ca2819cf78b421d7efbfc23860e2470d7d105d054b63bd3b7e5760d04bb4`.

| Check | Result |
|---|---|
| Rust tests, nesting and WASM crates | 59 passed; 2 pre-existing long/opt-in tests ignored |
| Strict Clippy, both crates and all targets | Passed with `-D warnings` |
| `wasm32-unknown-unknown` release + wasm-bindgen web build | Passed |
| Production Vite build / web typecheck | Passed; nesting Worker is in the precache manifest |
| Vitest, WASM package, Quick Marker and transport | 80 passed in 9 files |
| Headless Worker checks | 12 passed, including a deliberately stalled real Worker: its checked seed returned in 6.015 s for a 1 s budget, while 301 UI heartbeats ran |
| Full static app under a sub-path | 18 checks passed for both relative and explicit base paths, including the editor demo, real photo scan, Quick Marker built-in sample, offline reload, PDFs and projector |

The production check used a 390 × 844 viewport, a fresh browser profile and a
synthetic photo. No mock engine was enabled. It made no nesting or PDF-export
API request. Other screens' optional health/Pattern probes are outside this
check. Full cutting-room app offline coverage remains outside this task.

Local evidence (ignored build output, paths relative to `marker/`):

- `out/wasm-final-smoke/browser-report.json` and `offline-worker.png`
- `out/static-final-test/quick-offline-report.json`
- `out/static-final-test/static-marker-demo.png`
- `out/static-final-test/quick-offline-result.png`
- `out/static-final-test/static-quick-sample.png`
- `out/static-explicit-base-vite6-test/quick-offline-report.json`
- `out/wasm-bench/native.jsonl`
- `out/wasm-bench/browser-final/wasm-results.jsonl`
- `out/wasm-bench/wasm-validated.jsonl`

## Upstream timeout proposal for the lead

No patch was applied to Sparrow or Jagua. The bare-WASM diagnostic on task
14851 is retained in the ignored `out/wasm-bench/browser/` output. Source
inspection found this unchecked inner loop in the pinned Sparrow
`src/optimizer/separator.rs`. A minimal proposal is:

```diff
-            while n_iter_no_improvement < self.config.iter_no_imprv_limit {
+            while n_iter_no_improvement < self.config.iter_no_imprv_limit && !term.kill() {
```

This would check the deadline between worker batches. It does not interrupt a
single `move_items_multi()` call or initial construction. The exact hot loop
in the observed stall was not profiled, so this is an **unapplied proposal**,
not a verified upstream fix. The lead decides whether to pursue it. The
BETA seed and host watchdog make the shipped browser path independent of that
decision. No upstream source was vendored, forked, pushed or published.

## Static app handoff

From the repository root, with `CARGO_TARGET_DIR`, `CARGO_BUILD_JOBS=3`,
`WASM_BINDGEN` and the npm cache configured:

```powershell
# Relative URLs work when this folder is copied to any sub-path.
$env:BETA_WEB_BASE = './'
npm run build:static -w @beta/marker-web
```

The script builds Marker, Pattern and Floor WASM, checks TypeScript, and writes
**`marker/out/static-app/`**. Copy that folder's **contents** to the site's
`app/` directory. No installation, application server, API key, CDN, COOP or
COEP header is required. Serve over HTTPS for the offline service worker.
The script does not upload or publish anything.

`BETA_WEB_BASE=/beta-cutting-room/app/` is also supported and was tested with
Vite 6.4.4. The portable `./` build is the handoff artifact. Set
`BETA_SKIP_WASM_BUILD=1` only for UI-only rebuilds with current generated WASM;
missing modules are rejected. The normal `build` script keeps desktop/server
behaviour and writes the existing `dist` output.

Static mode selects the browser nester in the normal marker editor and Quick
Marker. It suppresses API requests, explains server-only panels, disables
native marker-export controls, and keeps local JSON and Quick Marker PDF
output available. Pattern and Floor modules are bundled too. The service
worker and its caches are scoped to the app sub-path, including its Worker
and WASM assets. The output includes the Apache licence, NOTICE and dependency
licence inventory.

Artifact size: **21 files, 11,471,930 bytes** uncompressed; **2,919,938 bytes**
when each file is gzipped at level 9. The gzip sum is a packaging comparison,
not a promise about the host's transfer encoding. Per-file sizes are in
`marker/out/static-build-report.json`.

```sh
node marker/packages/marker-wasm/scripts/quick-offline-test.mjs \
  marker/out/static-app marker/out/wasm-bench/synthetic-a4.png \
  marker/out/static-subpath-test /beta-cutting-room/app/
```

## Final paired measurements

**At the 60-second setting:** FreeSewing was 0.05 percentage points below
native on average; the nine valid Lectra pairs were 21.55 points below.
All 24 returned WASM markers passed native replay validation. Task 12127
failed in both paths; the browser error identifies piece 10 as unable to fit
its allowed width/orientation search. Failed attempts remain in the table.

The two heavy cases 14851 and 18110 retained very inefficient BETA checkpoints
when Sparrow did not finish a round. These results establish an offline
browser capability, **not native quality parity on complex jobs**. Watchdog
recoveries are included in the reported averages.

| Case | Native 60 s: efficiency / length mm | WASM 30 s: efficiency / length mm | WASM 60 s: efficiency / length mm |
|---|---:|---:|---:|
| freesewing-simon-M | 82.15% / 1280.4 | 79.94% / 1315.7 | 81.97% / 1283.2 |
| freesewing-teagan-M | 79.79% / 719.7 | 79.79% / 719.7 | 79.79% / 719.7 |
| freesewing-titan-M | 72.15% / 1235.7 | 72.18% / 1235.3 | 72.18% / 1235.3 |
| lectra-12127 | FAILED | FAILED | FAILED |
| lectra-14851 | 77.69% / 3644.8 | 10.78% / 26268.1 | 9.87% / 28696.9 |
| lectra-1561 | 91.66% / 720.1 | 91.67% / 720.0 | 91.67% / 720.0 |
| lectra-18110 | 81.87% / 4698.9 | 12.02% / 32002.4 | 12.45% / 30903.2 |
| lectra-18896 | 79.70% / 2286.3 | 71.42% / 2551.2 | 73.30% / 2485.8 |
| lectra-4122 | 72.10% / 5554.0 | 67.53% / 5929.5 | 67.60% / 5923.5 |
| lectra-58688 | 82.22% / 3521.6 | 74.88% / 3866.5 | 77.29% / 3746.0 |
| lectra-8411 | 78.91% / 1187.5 | 55.98% / 1673.8 | 56.81% / 1649.4 |
| lectra-88853 | 72.22% / 5460.3 | 70.61% / 5584.6 | 70.64% / 5582.8 |
| lectra-89590 | 71.91% / 6612.9 | 54.67% / 8698.8 | 54.67% / 8698.8 |

freesewing- at 30 s: mean efficiency difference from native = -0.72 percentage points (3 valid pairs; failures excluded from the mean).

lectra- at 30 s: mean efficiency difference from native = -22.08 percentage points (9 valid pairs; failures excluded from the mean).

freesewing- at 60 s: mean efficiency difference from native = -0.05 percentage points (3 valid pairs; failures excluded from the mean).

lectra- at 60 s: mean efficiency difference from native = -21.55 percentage points (9 valid pairs; failures excluded from the mean).

| Case | Native wall s | WASM wall s (30 / 60) | WASM memory MiB (30 / 60) | 60 s run: rounds / mean round s / max round s |
|---|---:|---:|---:|---:|
| freesewing-simon-M | 61.49 | 30.37 / 62.21 | 2.69 / 2.94 | 12 / 5.172 / 8.732 |
| freesewing-teagan-M | 1.15 | 30.32 / 60.13 | 3.69 / 3.75 | 63 / 0.935 / 2.792 |
| freesewing-titan-M | 0.12 | 30.43 / 60.33 | 2.00 / 2.00 | 110 / 0.529 / 0.836 |
| lectra-12127 | 0.33 | 0.03 / 0.03 | 1.69 / 1.69 | — |
| lectra-14851 | 64.17 | 35.00* / 65.01* | 7.62 / 7.62 | 1 / 1.693 / 1.693 |
| lectra-1561 | 61.37 | 30.19 / 61.47 | 2.94 / 2.94 | 9 / 6.824 / 9.781 |
| lectra-18110 | 67.86 | 35.01* / 65.01* | 8.00 / 8.00 | 1 / 1.087 / 1.087 |
| lectra-18896 | 60.77 | 30.33 / 62.68 | 2.69 / 2.69 | 13 / 4.797 / 6.390 |
| lectra-4122 | 64.28 | 32.76 / 64.35 | 5.44 / 5.44 | 5 / 12.492 / 16.835 |
| lectra-58688 | 62.27 | 33.39 / 62.25 | 3.31 / 3.38 | 13 / 4.765 / 6.540 |
| lectra-8411 | 61.94 | 32.94 / 65.00* | 2.50 / 2.50 | 8 / 7.215 / 8.578 |
| lectra-88853 | 60.94 | 35.01* / 65.01* | 2.81 / 2.81 | 4 / 13.302 / 19.239 |
| lectra-89590 | 63.25 | 35.00* / 65.01* | 4.62 / 4.62 | 1 / 0.255 / 0.255 |

* Host watchdog retained the last checked checkpoint. Round timings and memory samples cover completed rounds only.

Native wall time: 0.12–67.86 s.
WASM wall time (30/60 s budgets combined): 0.03–65.01 s.
WASM observed allocated linear memory: 1.69–8.00 MiB (fresh instance per trial; samples after completed rounds).
Round time: mean 2.181 s, median 0.669 s, max 19.239 s (370 rounds).
Valid native results: 12/13; valid WASM results: 24/26.
Watchdog recoveries: 9 / 26. Results whose winning marker came from the BETA bounded packer: 8.


### Round-length probe

A separate follow-up on task 8411 used eight-second rounds at the
same 4x setting and seed. At 30 s it reached **60.71% / 1543.5 mm** in 33.78 s;
at 60 s, **65.24% / 1436.4 mm** in 64.06 s. Both passed native replay validation.
The default two-second 60 s run reached 56.81%, so this one-case probe gained
8.43 percentage points. Longer rounds delay improvement checkpoints. This is
one development case, not evidence to change the default for every job.
The public API exposes `round_time_s` for callers that choose that trade-off.

Remaining limits: no real-phone timing or memory acceptance test; no physical
photo/print/projector calibration in this task; weak packing on some complex
jobs; a construction failure on the retained development case 12127. The
foreground watchdog preserves validity and responsiveness, not packing quality.
