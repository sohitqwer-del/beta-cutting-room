/** Cutting room in the browser: cut plans, rolls, spreading, bundles, tickets,
 * marker reports and PLT import. The same Rust routes as `beta-marker serve`,
 * in a separate WASM module that loads only when a cutting-room screen needs it. */
import type { CutPlan, PlanRequest } from './types';

/** Current cut order format: `max_plies` counts plies (layers). */
export const CUT_ORDER_FORMAT = 'beta-marker.cutorder/v2';
export const CUT_ROOM_MODULE = 'wasm/beta_marker_cutroom_wasm.js';

export interface CutRoomReply { status: number; mime: string; extension: string | null; body: Uint8Array }
export interface CutRoomCore {
  /** POST one local API route, for example `/api/roll/spread` or `/api/import?format=plt&width=1500`. */
  request(url: string, body: Uint8Array): CutRoomReply;
}
interface RawReply { readonly status: number; readonly mime: string; readonly extension: string | undefined; readonly body: Uint8Array; free(): void }
interface RawCutRoom {
  default(options: { module_or_path: URL }): Promise<unknown>;
  api_request(url: string, body: Uint8Array): RawReply;
  plan(request: string, onProgress: (json: string) => boolean): string;
}
export function bindCutRoomCore(raw: Pick<RawCutRoom, 'api_request'>): CutRoomCore {
  return {
    request(url, body) {
      const reply = raw.api_request(url, body);
      try { return { status: reply.status, mime: reply.mime, extension: reply.extension ?? null, body: reply.body }; }
      finally { reply.free(); }
    },
  };
}
function moduleHref(moduleUrl?: string | URL): string {
  const base = typeof document === 'undefined' ? globalThis.location?.href : document.baseURI;
  return new URL(String(moduleUrl ?? CUT_ROOM_MODULE), base).href;
}
async function loadRaw(url: string): Promise<RawCutRoom> {
  const raw = await import(/* @vite-ignore */ url) as RawCutRoom;
  await raw.default({ module_or_path: new URL('./beta_marker_cutroom_wasm_bg.wasm', url) });
  return raw;
}
/** Load local assets on demand. No HTTP API or external service is used. */
export async function loadCutRoomCore(moduleUrl?: string | URL): Promise<CutRoomCore> {
  return bindCutRoomCore(await loadRaw(moduleHref(moduleUrl)));
}

export type CutPlanWorkerRequest =
  | { type: 'init'; moduleUrl: string }
  | { type: 'plan'; id: number; request: PlanRequest };
export type CutPlanWorkerResponse =
  | { type: 'ready' }
  | { type: 'progress'; id: number; plan: CutPlan }
  | { type: 'done'; id: number; plan: CutPlan }
  | { type: 'error'; id?: number; message: string };
export interface CutPlanWorkerPort {
  onmessage: ((event: MessageEvent<CutPlanWorkerRequest>) => void) | null;
  postMessage(message: CutPlanWorkerResponse): void;
}
/** Worker side, separate from the entry file so it can be unit tested. Planning
 * blocks the Worker; progress messages still reach the page after each marker. */
export function installCutPlanWorker(port: CutPlanWorkerPort, load: (url: string) => Promise<Pick<RawCutRoom, 'plan'>> = loadRaw): void {
  let runtime: Promise<Pick<RawCutRoom, 'plan'>> | undefined;
  port.onmessage = ({ data }) => {
    if (data.type === 'init') {
      if (runtime) return;
      runtime = load(data.moduleUrl);
      void runtime.then(() => port.postMessage({ type: 'ready' }), error => port.postMessage({ type: 'error', message: String(error) }));
      return;
    }
    const loaded = runtime;
    if (!loaded) { port.postMessage({ type: 'error', id: data.id, message: 'Worker is not initialized.' }); return; }
    void loaded.then(raw => {
      const json = raw.plan(JSON.stringify(data.request), progress => {
        port.postMessage({ type: 'progress', id: data.id, plan: JSON.parse(progress) as CutPlan });
        return true;
      });
      port.postMessage({ type: 'done', id: data.id, plan: JSON.parse(json) as CutPlan });
    }).catch(error => port.postMessage({ type: 'error', id: data.id, message: String(error instanceof Error ? error.message : error) }));
  };
}

export interface CutPlanner {
  /** JS and WASM loaded; planning then needs no network. */
  ready: Promise<void>;
  /** `onProgress` receives each checked partial plan (one per measured marker). */
  plan(request: PlanRequest, onProgress?: (plan: CutPlan) => void): Promise<CutPlan>;
  /** Stop now and resolve with the last checked partial plan, or reject if none exists yet.
   * The returned plan carries a warning and may be incomplete: check `complete`. */
  stop(): void;
  dispose(): void;
}
export interface CutPlannerConfig { moduleUrl?: string | URL; workerFactory?: () => Worker }
export const STOPPED_PLAN_WARNING = 'Stopped before the search ended. This is the best checked plan found so far.';

export function createCutPlanner(config: CutPlannerConfig = {}): CutPlanner {
  const worker = config.workerFactory?.() ?? new Worker(new URL('./cutroom.worker.ts', import.meta.url), { type: 'module' });
  let resolveReady!: () => void, rejectReady!: (error: Error) => void;
  const ready = new Promise<void>((resolve, reject) => { resolveReady = resolve; rejectReady = reject; });
  void ready.catch(() => {});
  let sequence = 0, closed = false;
  let pending: { id: number; resolve: (plan: CutPlan) => void; reject: (error: Error) => void; progress?: (plan: CutPlan) => void; latest?: CutPlan } | undefined;
  const close = (error: Error) => {
    if (closed) return;
    closed = true; worker.terminate(); rejectReady(error);
    pending?.reject(error); pending = undefined;
  };
  worker.onmessage = ({ data }: MessageEvent<CutPlanWorkerResponse>) => {
    if (closed) return;
    if (data.type === 'ready') { resolveReady(); return; }
    if (data.type === 'error' && data.id === undefined) { close(new Error(data.message)); return; }
    if (!pending || data.id !== pending.id) return;
    if (data.type === 'progress') { pending.latest = data.plan; pending.progress?.(structuredClone(data.plan)); }
    else if (data.type === 'done') { const current = pending; pending = undefined; current.resolve(data.plan); }
    else { const current = pending; pending = undefined; current.reject(new Error(data.message)); }
  };
  worker.onerror = event => close(new Error(event.message || 'Cut plan worker failed.'));
  worker.postMessage({ type: 'init', moduleUrl: moduleHref(config.moduleUrl) } satisfies CutPlanWorkerRequest);
  return {
    ready,
    plan(request, onProgress) {
      if (closed) return Promise.reject(new Error('Cut planner is closed.'));
      if (pending) return Promise.reject(new Error('A cut plan is already running.'));
      const id = ++sequence;
      return new Promise<CutPlan>((resolve, reject) => {
        pending = { id, resolve, reject, progress: onProgress };
        worker.postMessage({ type: 'plan', id, request } satisfies CutPlanWorkerRequest);
      });
    },
    stop() {
      const current = pending;
      if (!current) return;
      pending = undefined;
      close(new Error('Cut planner is closed.'));
      if (!current.latest) { current.reject(new Error('Stopped before any marker was measured.')); return; }
      const plan = current.latest;
      plan.warnings = [...plan.warnings, STOPPED_PLAN_WARNING];
      current.resolve(plan);
    },
    dispose() { close(new Error('Cut planner is closed.')); },
  };
}
