import * as markerWasm from '@beta/marker-wasm';
import type { Job, MarkerResult, MarkerNester, NestOptions } from '@beta/marker-wasm';
import type { ChalkGuideOptions, ProjectorLayout, QuickFabric, QuickMarkerCore, QuickPiece, QuickScan, QuickScanOptions, QuickSession, TiledMarkerOptions } from '@beta/marker-wasm';
import { mockCore, mockNest, MOCK_ENGINE } from './mock';

/** The only place the Quick Marker screens touch the engine.
 * - Photo, assist, job, PDF and projector calls: Rust WASM `loadQuickMarkerCore()` from @beta/marker-wasm
 *   (Quick Marker API v1). The mock is used only when asked for (tests, smoke, "Try a sample photo").
 * - Nesting: local Rust WASM in a Web Worker first. Desktop/serve is a fallback
 *   when that worker cannot load. A search/validation error is never hidden. */
export type EngineKind = 'wasm' | 'mock';
export interface QuickEngine {
  kind: EngineKind;
  scan(photo: Uint8Array, options: QuickScanOptions): Promise<QuickScan>;
  assist(pieces: QuickPiece[]): Promise<QuickPiece[]>;
  merge(scans: Pick<QuickScan, 'photoId' | 'pieces' | 'warnings'>[]): Promise<QuickSession>;
  toJob(pieces: QuickPiece[], fabric: QuickFabric, garments: Record<string, number>): Promise<Job>;
  tiledPdf(job: Job, result: MarkerResult, options: TiledMarkerOptions): Promise<Uint8Array>;
  chalkPdf(job: Job, result: MarkerResult, options?: ChalkGuideOptions): Promise<Uint8Array>;
  projector(job: Job, result: MarkerResult): Promise<ProjectorLayout>;
}
export type LoadResult = { ok: true; engine: QuickEngine } | { ok: false; reason: 'missing' | 'failed'; error: string };

/** Wrap a synchronous core as async calls, so moving the scan into a Web Worker later changes nothing here. */
export function wrapCore(core: QuickMarkerCore, kind: EngineKind): QuickEngine {
  const call = <T>(f: () => T): Promise<T> => new Promise((resolve, reject) => { try { resolve(f()); } catch (error) { reject(error instanceof Error ? error : new Error(String(error))); } });
  return {
    kind,
    scan: (photo, options) => call(() => core.quickScan(photo, options)),
    assist: pieces => call(() => core.quickAssist(pieces)),
    merge: scans => call(() => core.quickMerge(scans)),
    toJob: (pieces, fabric, garments) => call(() => core.quickPiecesToJob(pieces, fabric, garments)),
    tiledPdf: (job, result, options) => call(() => core.tiledMarkerPdf(job, result, options)),
    chalkPdf: (job, result, options) => call(() => core.chalkGuidePdf(job, result, options)),
    projector: (job, result) => call(() => core.projectorLayout(job, result)),
  };
}

type Loader = (moduleUrl?: string | URL) => Promise<QuickMarkerCore>;
/** Load the real engine. `module` is injectable for tests. */
export async function loadQuickEngine(options: { mock?: boolean; module?: Record<string, unknown> } = {}): Promise<LoadResult> {
  if (options.mock) return { ok: true, engine: wrapCore(mockCore(), 'mock') };
  const loader = (options.module ?? (markerWasm as unknown as Record<string, unknown>)).loadQuickMarkerCore as Loader | undefined;
  if (typeof loader !== 'function') return { ok: false, reason: 'missing', error: 'This build has no photo reader yet.' };
  try { return { ok: true, engine: wrapCore(await loader(), 'wasm') }; }
  catch (error) { return { ok: false, reason: 'failed', error: error instanceof Error ? error.message : String(error) }; }
}

/** Real engine only: run scans in a module Web Worker. If the worker cannot start, scans stay on the main thread. */
export function scanInWorker(engine: QuickEngine, makeWorker: () => Worker, moduleUrl: string): QuickEngine {
  let worker: Worker | null = null, broken = false, next = 0;
  const waiting = new Map<number, { resolve: (s: QuickScan) => void; reject: (e: Error) => void; fallback: () => void }>();
  const start = (): Worker | null => {
    if (broken) return null;
    if (worker) return worker;
    try { worker = makeWorker(); } catch { broken = true; return null; }
    worker.onmessage = (event: MessageEvent<{ id: number; scan?: QuickScan; error?: string }>) => {
      const job = waiting.get(event.data.id); if (!job) return; waiting.delete(event.data.id);
      if (event.data.scan) job.resolve(event.data.scan); else job.reject(new Error(event.data.error ?? 'Scan failed.'));
    };
    worker.onerror = () => { broken = true; worker?.terminate(); worker = null; for (const job of waiting.values()) job.fallback(); waiting.clear(); };
    return worker;
  };
  return {
    ...engine,
    scan(photo, options) {
      const w = start();
      if (!w) return engine.scan(photo, options);
      return new Promise((resolve, reject) => {
        const id = ++next;
        waiting.set(id, { resolve, reject, fallback: () => { engine.scan(photo, options).then(resolve, reject); } });
        w.postMessage({ id, bytes: photo, options, moduleUrl });
      });
    },
  };
}

/** `?quick-mock=1` in the address forces practice mode (headless smoke and screenshots). */
export function mockRequested(search: string): boolean { return /[?&]quick-mock=1\b/.test(search); }

// ---------- Nesting ----------
export type Fetcher = (url: string, init?: RequestInit) => Promise<Response>;
export interface NestProgress { elapsedS: number; round: number; best: MarkerResult | null; statusLengthMm?: number | null; statusEfficiency?: number | null }
export type BrowserNesterFactory = () => MarkerNester;
export interface NestRun { budgetS: number; roundS?: number; seed?: number; signal?: AbortSignal; onProgress?: (p: NestProgress) => void; now?: () => number; pollMs?: number;
  /** Host override for tests or deployments with a custom Worker entry. */
  browserNester?: BrowserNesterFactory }

let localNester: MarkerNester | undefined;
async function browserNester(make?: BrowserNesterFactory): Promise<MarkerNester> {
  if (localNester?.closed) { localNester.dispose(); localNester = undefined; }
  const nester = make ? make() : (localNester ??= markerWasm.createMarkerNester());
  try { await nester.ready; return nester; }
  catch (error) {
    nester.dispose();
    if (localNester === nester) localNester = undefined;
    throw error;
  }
}
/** Native-only capabilities (for example HPGL export) still need this check. */
export async function serverAvailable(fetcher: Fetcher): Promise<boolean> {
  try { const r = await fetcher('/api/health'); return r.ok; } catch { return false; }
}
/** Prefer the offline browser engine; no health request is sent when it loads. */
export async function nestAvailable(fetcher: Fetcher, make?: BrowserNesterFactory): Promise<boolean> {
  try { await browserNester(make); return true; }
  catch { return serverAvailable(fetcher); }
}
async function errorOf(response: Response): Promise<string> {
  const body = await response.json().catch(() => null) as { error?: string } | null;
  if (response.status === 401) return 'The shop server needs an access token. Open More › Server token.';
  return body?.error || `Server error ${response.status}.`;
}
/** True when `a` is a better marker than `b` (shorter; ties broken by efficiency). */
export function better(a: MarkerResult, b: MarkerResult | null): boolean {
  return !b || a.length < b.length - 0.01 || (Math.abs(a.length - b.length) <= 0.01 && a.efficiency > b.efficiency);
}
/** Browser first, with stop checked between warm-start rounds. Every browser
 * progress marker is independently validated in Rust. Native fallback is used
 * only when the browser engine cannot initialize. */
export async function nestInRounds(fetcher: Fetcher, job: Job, run: NestRun): Promise<MarkerResult | null> {
  if (run.signal?.aborted) return null;
  let local: MarkerNester | undefined;
  try { local = await browserNester(run.browserNester); } catch { /* Native fallback below. */ }
  if (run.signal?.aborted) return null;
  if (local) {
    let best: MarkerResult | null = null;
    const stop = () => local!.stop();
    run.signal?.addEventListener('abort', stop, { once: true });
    try {
      return await local.nest(job, {
        time_limit_s: run.budgetS, round_time_s: run.roundS ?? 2, seed: run.seed ?? 42,
      }, progress => {
        best = progress.result;
        run.onProgress?.({ elapsedS: progress.seconds, round: progress.round, best });
      });
    } catch (error) {
      if (run.signal?.aborted) return best;
      throw error;
    } finally { run.signal?.removeEventListener('abort', stop); }
  }
  return nestOnServer(fetcher, job, run);
}
/** Native fallback. Stop keeps the completed round; polling is optional. */
async function nestOnServer(fetcher: Fetcher, job: Job, run: NestRun): Promise<MarkerResult | null> {
  const now = run.now ?? (() => performance.now());
  const start = now(), roundS = Math.max(1, run.roundS ?? 8);
  let best: MarkerResult | null = null, round = 0;
  const elapsed = () => (now() - start) / 1000;
  let polling = true;
  const poll = async () => {
    while (polling && !run.signal?.aborted) {
      await new Promise(r => setTimeout(r, run.pollMs ?? 1000));
      if (!polling) break;
      try {
        const s = await (await fetcher('/api/nest/status')).json() as { best_length_mm?: number | null; best_efficiency?: number | null };
        run.onProgress?.({ elapsedS: elapsed(), round, best, statusLengthMm: s.best_length_mm ?? null, statusEfficiency: s.best_efficiency ?? null });
      } catch { /* status is optional */ }
    }
  };
  const poller = run.pollMs === 0 ? Promise.resolve() : poll();
  try {
    while (!run.signal?.aborted) {
      const left = run.budgetS - elapsed();
      if (left < 0.5 && round > 0) break;
      const time = Math.max(1, Math.min(roundS, left));
      const options: NestOptions = { time_limit_s: Math.round(time * 10) / 10, seed: (run.seed ?? 42) + round, workers: 2, runs: 1, early_termination: false, warm_start: best };
      const response = await fetcher('/api/nest', { method: 'POST', body: JSON.stringify({ job, options }), headers: { 'Content-Type': 'application/json' } });
      if (!response.ok) throw new Error(await errorOf(response));
      const result = await response.json() as MarkerResult;
      if (!Array.isArray(result.placements)) throw new Error('The server did not return a marker.');
      round++;
      if (better(result, best)) best = result;
      run.onProgress?.({ elapsedS: elapsed(), round, best });
    }
  } finally { polling = false; void poller; }
  return best;
}
/** Practice mode: an instant shelf layout, clearly marked as not real. */
export async function nestPractice(job: Job, run: NestRun): Promise<MarkerResult> {
  const result = mockNest(job);
  run.onProgress?.({ elapsedS: 0.2, round: 1, best: result });
  return result;
}
export const isPractice = (result: MarkerResult | undefined): boolean => result?.engine === MOCK_ENGINE;

/** Plotter file from the native Rust exporter (needs the desktop app or server). */
export async function exportFile(fetcher: Fetcher, job: Job, result: MarkerResult, format: 'plt' | 'pdf'): Promise<Blob> {
  const response = await fetcher(`/api/export?format=${format}`, { method: 'POST', body: JSON.stringify({ job, result }), headers: { 'Content-Type': 'application/json' } });
  if (!response.ok) throw new Error(await errorOf(response));
  return response.blob();
}
