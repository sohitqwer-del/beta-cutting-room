import type { Job, MarkerResult } from './types';
import type { BrowserNestOptions, NestProgress, NestWorkerRequest, NestWorkerResponse } from './nest-types';
export type { BrowserNestOptions, NestProgress } from './nest-types';

export interface MarkerNester {
  /** Wait until JS and WASM are loaded; nesting then needs no network. */
  ready: Promise<void>;
  /** A timeout or worker failure closes this instance. Create another to retry. */
  readonly closed: boolean;
  nest(job: Job, options?: BrowserNestOptions, onProgress?: (progress: NestProgress) => void): Promise<MarkerResult>;
  /** Stop after the current round, with a five-second host watchdog, and resolve with the last valid marker.
   * Before the first round, cancellation rejects because no marker exists yet. */
  stop(): void;
  /** Release worker and WASM memory. Rejects any pending search. */
  dispose(): void;
}
export interface MarkerNesterConfig {
  moduleUrl?: string | URL;
  /** Useful for bundlers with a custom worker entry, and for host tests. */
  workerFactory?: () => Worker;
}

export function createMarkerNester(config: MarkerNesterConfig = {}): MarkerNester {
  const base = typeof document === 'undefined' ? globalThis.location?.href : document.baseURI;
  const url = new URL(String(config.moduleUrl ?? 'wasm/beta_marker_wasm.js'), base).href;
  const worker = config.workerFactory?.() ?? new Worker(new URL('./nest.worker.ts', import.meta.url), { type: 'module' });
  let resolveReady!: () => void;
  let rejectReady!: (reason: Error) => void;
  const ready = new Promise<void>((resolve, reject) => { resolveReady = resolve; rejectReady = reject; });
  // The public promise still rejects; this prevents a delayed caller from creating
  // an unhandled rejection when initialization fails before it starts a search.
  void ready.catch(() => {});
  let sequence = 0;
  let disposed = false;
  let failed: Error | undefined;
  let pending: { id: number; resolve: (r: MarkerResult) => void; reject: (e: Error) => void;
    progress?: (p: NestProgress) => void; latest?: NestProgress; started: number;
    timer?: ReturnType<typeof setTimeout>; cancelled: boolean } | undefined;
  const post = (message: NestWorkerRequest) => worker.postMessage(message);
  const fail = (error: Error) => {
    failed = error;
    disposed = true;
    worker.terminate();
    rejectReady(error);
    clearTimeout(pending?.timer);
    pending?.reject(error);
    pending = undefined;
  };
  const retain = (message: string): boolean => {
    const current = pending;
    if (!current?.latest) return false;
    clearTimeout(current.timer);
    pending = undefined;
    disposed = true;
    worker.terminate();
    const progress = current.latest;
    progress.seconds = Math.max(progress.seconds, (performance.now() - current.started) / 1000);
    progress.result.runtime_s = progress.seconds;
    progress.result.warnings = [...(progress.result.warnings ?? []), message];
    progress.done = true;
    progress.cancelled = current.cancelled;
    progress.improved = false;
    try { current.progress?.(structuredClone(progress)); current.resolve(progress.result); }
    catch (error) { current.reject(error instanceof Error ? error : new Error(String(error))); }
    return true;
  };
  const arm = (milliseconds: number, message: string) => {
    const current = pending;
    if (!current) return;
    clearTimeout(current.timer);
    current.timer = setTimeout(() => {
      if (pending !== current) return;
      if (!retain(message)) fail(new Error('Search stopped before a complete valid marker was available.'));
    }, milliseconds);
  };
  worker.onmessage = ({ data }: MessageEvent<NestWorkerResponse>) => {
    if (disposed) return;
    if (data.type === 'ready') { resolveReady(); return; }
    if (data.type === 'error' && data.id === undefined) { fail(new Error(data.message)); return; }
    if (!pending || data.id !== pending.id) return;
    if (data.type === 'progress') {
      const current = pending;
      try {
        // Keep a private copy: a drawing callback may mutate its own payload.
        current.latest = structuredClone(data.progress);
        current.progress?.(data.progress);
        if (!disposed && !current.latest.done) post({ type: 'continue', id: data.id, round: current.latest.round });
      }
      catch (error) {
        clearTimeout(current.timer);
        current.reject(error instanceof Error ? error : new Error(String(error)));
        pending = undefined;
        disposed = true;
        worker.terminate();
      }
    } else if (data.type === 'done') {
      clearTimeout(pending.timer);
      pending.resolve(data.result); pending = undefined;
    } else if (data.type === 'error') {
      if (!retain(`Browser search failed; last independently checked marker retained. ${data.message}`)) fail(new Error(data.message));
    }
  };
  worker.onerror = event => {
    if (disposed) return;
    if (!retain('Nesting worker failed; last independently checked marker retained.')) fail(new Error(event.message || 'Nesting worker failed.'));
  };
  worker.onmessageerror = () => {
    if (disposed) return;
    if (!retain('Worker response failed; last independently checked marker retained.')) fail(new Error('Could not read nesting worker response.'));
  };
  post({ type: 'init', moduleUrl: url });
  return {
    ready,
    get closed() { return disposed || !!failed; },
    nest(job, options = {}, onProgress) {
      if (disposed || failed) return Promise.reject(failed ?? new Error('Nesting worker is disposed.'));
      if (pending) return Promise.reject(new Error('A search is already running.'));
      if (options.seed != null && (!Number.isSafeInteger(options.seed) || options.seed < 0)) {
        return Promise.reject(new Error('seed must be a non-negative JavaScript safe integer.'));
      }
      const limit = options.time_limit_s ?? 30;
      if (!Number.isFinite(limit) || limit < 0.001 || limit > 3600) return Promise.reject(new Error('time_limit_s must be between 0.001 and 3600.'));
      const id = ++sequence;
      return new Promise<MarkerResult>((resolve, reject) => {
        pending = { id, resolve, reject, progress: onProgress, started: performance.now(), cancelled: false };
        void ready.then(() => {
          if (pending?.id !== id) return;
          pending.started = performance.now();
          if (!pending.cancelled) arm((limit + 5) * 1000, 'Browser time limit reached; last independently checked marker retained.');
        }, () => {});
        try { post({ type: 'nest', id, job, options }); }
        catch (error) { pending = undefined; reject(error); }
      });
    },
    stop() {
      if (!pending) return;
      pending.cancelled = true;
      post({ type: 'cancel', id: pending.id });
      arm(5000, 'Browser search stopped; last independently checked marker retained.');
    },
    dispose() {
      if (disposed) return;
      fail(new Error('Nesting worker is disposed.'));
    },
  };
}

/** One-shot convenience API. Reuse createMarkerNester for repeated searches.
 * AbortSignal requests cooperative stop, retaining an available valid marker. */
export async function nest(
  job: Job,
  options: BrowserNestOptions = {},
  onProgress?: (progress: NestProgress) => void,
  config: MarkerNesterConfig & { signal?: AbortSignal } = {},
): Promise<MarkerResult> {
  if (config.signal?.aborted) throw new Error('Search cancelled before starting.');
  const nester = createMarkerNester(config);
  const stop = () => nester.stop();
  config.signal?.addEventListener('abort', stop, { once: true });
  try {
    return await nester.nest(job, options, onProgress);
  } finally {
    config.signal?.removeEventListener('abort', stop);
    nester.dispose();
  }
}
