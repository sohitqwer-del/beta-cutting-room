// Test-only page entry, bundled by browser-test.mjs. No app backend.
import { createMarkerNester } from '../src/nest';
import type { Job, MarkerResult } from '../src/types';
import type { NestProgress } from '../src/nest-types';
import { demoJob } from '../../../apps/web/src/demo';

const nester = createMarkerNester({
  workerFactory: () => new Worker('/worker.js', { type: 'module' }),
  moduleUrl: new URL('/wasm/beta_marker_wasm.js', location.href),
});
const stalled = createMarkerNester({
  workerFactory: () => new Worker('/stall-worker.js', { type: 'module' }),
  moduleUrl: new URL('/wasm/beta_marker_wasm.js', location.href),
});
let raw: any;
let memory: WebAssembly.Memory;
const events: NestProgress[] = [];
let heartbeats = 0;
setInterval(() => { heartbeats++; }, 20);

async function init() {
  raw = await import(/* @vite-ignore */ new URL('/wasm/beta_marker_wasm.js', location.href).href);
  const wasm = await raw.default();
  memory = wasm.memory;
  await nester.ready;
  await stalled.ready;
}
async function watchdogDemo() {
  const start = performance.now(), beat = heartbeats;
  let deadline: ReturnType<typeof setTimeout> | undefined;
  try {
    const result = await Promise.race([
      stalled.nest(demoJob, { time_limit_s: 1, seed: 1 }),
      new Promise<never>((_, reject) => { deadline = setTimeout(() => reject(new Error('Watchdog did not stop the stalled fixture')), 9000); }),
    ]);
    return { result, validation: checked(demoJob, result), wall_s: (performance.now() - start) / 1000,
      heartbeats: heartbeats - beat, closed: stalled.closed };
  } finally { clearTimeout(deadline); stalled.dispose(); }
}
function checked(job: Job, result: MarkerResult) {
  const report = JSON.parse(raw.validate(JSON.stringify(job), JSON.stringify(result)));
  if (!report.ok) throw new Error(JSON.stringify(report.errors));
  return report;
}
async function workerDemo(stop = false) {
  const start = performance.now();
  const beat = heartbeats;
  events.length = 0;
  const job = demoJob;
  const result = await nester.nest(job, { time_limit_s: stop ? 60 : 2, round_time_s: 0.2, seed: 1 }, p => {
    events.push(p);
    document.querySelector('output')!.textContent =
      `Round ${p.round}: ${(p.result.efficiency * 100).toFixed(2)}% · ${p.result.length.toFixed(1)} mm`;
    if (stop && events.length === 1) nester.stop();
  });
  return { result, validation: checked(job, result), events: [...events],
    wall_s: (performance.now() - start) / 1000, heartbeats: heartbeats - beat };
}
async function benchmark(job: Job, time_limit_s: number, round_time_s = 2) {
  const start = performance.now();
  const rounds: { seconds: number; memory_bytes: number }[] = [];
  let session: any;
  try {
    session = new raw.NestSession(JSON.stringify(job), JSON.stringify({
      time_limit_s, round_time_s, seed: 1,
    }));
    for (;;) {
      const p = JSON.parse(session.step()) as NestProgress;
      p.memory_bytes = memory.buffer.byteLength;
      rounds.push({ seconds: p.round_seconds, memory_bytes: p.memory_bytes });
      // CDP binding lets the external benchmark watchdog retain the same
      // checked checkpoint that the production UI receives from its Worker.
      (globalThis as any).betaBenchProgress?.(JSON.stringify(p));
      if (p.done) return { result: p.result, validation: checked(job, p.result), rounds,
        wall_s: (performance.now() - start) / 1000, memory_bytes: memory.buffer.byteLength };
      await new Promise(resolve => setTimeout(resolve, 0));
    }
  } catch (error) {
    return { error: String(error), validation: { ok: false }, rounds,
      wall_s: (performance.now() - start) / 1000, memory_bytes: memory.buffer.byteLength };
  } finally { session?.free(); }
}
async function workerRules() {
  const fold: Job = {
    format: 'beta-marker.job/v1', name: 'Public folded rectangle', fabric: { width: 60, layout: 'folded' },
    pieces: [{ id: 'panel', name: 'Panel', quantity: 3,
      outline: [[0, 0], [60, 0], [60, 40], [0, 40]], fold_line: [[0, 20], [60, 20]] }],
  };
  const folded = await nester.nest(fold, { time_limit_s: 0.2, round_time_s: 0.1, seed: 1 });
  checked(fold, folded);
  if (folded.placements.some(p => !p.layer_allocations?.length)) throw new Error('Missing folded layer allocation');
  const defect = structuredClone(demoJob);
  defect.fabric.defects = [{ id: 'test-hole', polygon: [[0, 0], [40, 0], [40, 1500], [0, 1500]] }];
  const around = await nester.nest(defect, { time_limit_s: 0.2, round_time_s: 0.1, seed: 1 });
  checked(defect, around);
  const fixed = { ...around.placements[0]!, locked: true };
  const locked = await nester.nest(defect, {
    time_limit_s: 0.2, round_time_s: 0.1, seed: 1, fixed: [fixed],
  });
  checked(defect, locked);
  if (!locked.placements.some(p => JSON.stringify(p) === JSON.stringify(fixed))) throw new Error('Locked placement moved');
  return { folded, defects: around, locked };
}
// Public test hooks only in this standalone test bundle.
Object.assign(globalThis, { harness: { init, workerDemo, workerRules, watchdogDemo, benchmark, events,
  dispose: () => { nester.dispose(); stalled.dispose(); }, memoryBytes: () => memory.buffer.byteLength } });
