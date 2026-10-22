import type { NestProgress, NestWorkerRequest, NestWorkerResponse } from './nest-types';

export interface RawNestSession { step(): string; step_with_progress?(callback: (json: string) => void): string; cancel(): void; free(): void }
export interface NestRuntime {
  NestSession: new (job: string, options: string) => RawNestSession;
  memory: { buffer: { byteLength: number } };
}
export interface NestWorkerPort {
  onmessage: ((event: MessageEvent<NestWorkerRequest>) => void) | null;
  postMessage(message: NestWorkerResponse): void;
}
interface ActiveSearch { id: number; cancelled: boolean; round?: number; resume?: () => void }
export async function loadNestRuntime(url: string): Promise<NestRuntime> {
  const raw = await import(/* @vite-ignore */ url);
  if (typeof raw.NestSession !== 'function') throw new Error('This WASM build has no browser nester.');
  const wasm = await raw.default({ module_or_path: new URL('./beta_marker_wasm_bg.wasm', url) });
  return { NestSession: raw.NestSession, memory: wasm.memory };
}

/** Separate from the worker entry so lifecycle and stop races can be unit tested. */
export function installNestWorker(
  port: NestWorkerPort,
  load = loadNestRuntime,
  yieldToMessages = () => new Promise<void>(resolve => setTimeout(resolve, 0)),
): void {
  let runtime: Promise<NestRuntime> | undefined;
  let running: ActiveSearch | undefined;
  port.onmessage = ({ data }) => {
    if (data.type === 'init') {
      if (runtime) return;
      runtime = load(data.moduleUrl);
      void runtime.then(
        () => port.postMessage({ type: 'ready' }),
        error => port.postMessage({ type: 'error', message: String(error) }),
      );
    } else if (data.type === 'cancel') {
      if (running?.id === data.id) { running.cancelled = true; running.resume?.(); }
    } else if (data.type === 'continue') {
      if (running?.id === data.id && running.round === data.round) running.resume?.();
    } else if (data.type === 'nest') {
      if (running || !runtime) {
        port.postMessage({ type: 'error', id: data.id, message: running ? 'A search is already running.' : 'Worker is not initialized.' });
        return;
      }
      const active: ActiveSearch = running = { id: data.id, cancelled: false };
      const loaded = runtime;
      void (async () => {
        let session: RawNestSession | undefined;
        try {
          const raw = await loaded;
          await yieldToMessages();
          if (active.cancelled) throw new Error('Search cancelled before a complete marker was available.');
          session = new raw.NestSession(JSON.stringify(data.job), JSON.stringify(data.options));
          for (;;) {
            if (active.cancelled) session.cancel();
            const checkpoint = (json: string) => {
              const progress = JSON.parse(json) as NestProgress;
              progress.checkpoint = true;
              progress.memory_bytes = raw.memory.buffer.byteLength;
              port.postMessage({ type: 'progress', id: data.id, progress });
            };
            const progress = JSON.parse(session.step_with_progress ? session.step_with_progress(checkpoint) : session.step()) as NestProgress;
            progress.memory_bytes = raw.memory.buffer.byteLength;
            active.round = progress.round;
            const acknowledged = new Promise<void>(resolve => { active.resume = resolve; });
            port.postMessage({ type: 'progress', id: data.id, progress });
            if (progress.done) {
              port.postMessage({ type: 'done', id: data.id, result: progress.result });
              break;
            }
            // Backpressure lets the UI process progress (and its Stop callback)
            // before another round starts. No shared memory or isolation headers.
            await acknowledged;
            active.resume = undefined;
            await yieldToMessages();
          }
        } catch (error) {
          port.postMessage({ type: 'error', id: data.id, message: String(error) });
        } finally {
          session?.free();
          running = undefined;
        }
      })();
    }
  };
}
