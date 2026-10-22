import { afterEach, describe, expect, it, vi } from 'vitest';
import { createMarkerNester } from '../src/nest';
import { installNestWorker, type NestWorkerPort, type NestRuntime } from '../src/nest-worker';
import type { NestWorkerRequest, NestWorkerResponse, NestProgress } from '../src/nest-types';
import type { Job, MarkerResult } from '../src/types';
import fixture from './fixtures/contract.json';

const job = fixture.job as unknown as Job;
const result = fixture.result as unknown as MarkerResult;
const tick = () => new Promise<void>(r => setTimeout(r, 0));
afterEach(() => vi.useRealTimers());
async function until(test: () => boolean) {
  for (let i = 0; i < 100; i++) { if (test()) return; await tick(); }
  throw new Error('Worker test timed out');
}
function runtime(doneAfter = 2) {
  const freed = vi.fn();
  let calls = 0;
  const raw: NestRuntime = {
    memory: { buffer: { byteLength: 65536 } },
    NestSession: class {
      cancelled = false;
      step() {
        calls++;
        return JSON.stringify({ round: calls, seconds: calls, round_seconds: 1, improved: true,
          done: this.cancelled || calls >= doneAfter, cancelled: this.cancelled, result });
      }
      cancel() { this.cancelled = true; }
      free() { freed(); }
    },
  };
  return { raw, freed, calls: () => calls };
}
function harness(raw: NestRuntime) {
  const messages: NestWorkerResponse[] = [];
  const port: NestWorkerPort = { onmessage: null, postMessage: m => {
    messages.push(m);
    if (m.type === 'progress' && !m.progress.done) setTimeout(() =>
      send({ type: 'continue', id: m.id, round: m.progress.round }), 0);
  } };
  installNestWorker(port, async () => raw);
  const send = (data: NestWorkerRequest) => port.onmessage!({ data } as MessageEvent<NestWorkerRequest>);
  send({ type: 'init', moduleUrl: 'http://localhost/wasm.js' });
  return { send, messages };
}
describe('nesting worker', () => {
  it('posts within-round layouts before the step returns', async () => {
    const rt = runtime(1);
    const Session = rt.raw.NestSession;
    rt.raw.NestSession = class extends Session {
      step_with_progress(callback: (json: string) => void) {
        callback(JSON.stringify({ result, round: 0, seconds: 0.1, improved: true, done: false }));
        return this.step();
      }
    };
    const h = harness(rt.raw);
    h.send({ type: 'nest', id: 1, job, options: {} });
    await until(() => h.messages.some(m => m.type === 'done'));
    const events = h.messages.filter(m => m.type === 'progress');
    expect(events).toHaveLength(2);
    expect(events[0]!.progress.checkpoint).toBe(true);
    expect(events[0]!.progress.result.placements).toEqual(result.placements);
    expect(events[1]!.progress.done).toBe(true);
  });
  it('loads once, yields, reports marker payloads and frees the session', async () => {
    const rt = runtime();
    const h = harness(rt.raw);
    h.send({ type: 'nest', id: 1, job, options: {} });
    await until(() => h.messages.some(m => m.type === 'done'));
    const progress = h.messages.filter(m => m.type === 'progress');
    expect(progress).toHaveLength(2);
    expect(progress[0]?.type === 'progress' && progress[0].progress.memory_bytes).toBe(65536);
    expect(rt.freed).toHaveBeenCalledOnce();
  });
  it('handles stop at a round boundary and ignores stale stop IDs', async () => {
    const rt = runtime(100);
    const h = harness(rt.raw);
    h.send({ type: 'nest', id: 2, job, options: {} });
    h.send({ type: 'cancel', id: 1 });
    await until(() => rt.calls() >= 1);
    h.send({ type: 'cancel', id: 2 });
    await until(() => h.messages.some(m => m.type === 'done'));
    const events = h.messages.filter(m => m.type === 'progress');
    expect(events.at(-1)?.progress.cancelled).toBe(true);
    expect(rt.calls()).toBeLessThan(100);
    expect(rt.freed).toHaveBeenCalledOnce();
  });
  it('rejects concurrent jobs and stop before the first marker', async () => {
    const rt = runtime();
    const h = harness(rt.raw);
    h.send({ type: 'nest', id: 1, job, options: {} });
    h.send({ type: 'nest', id: 2, job, options: {} });
    h.send({ type: 'cancel', id: 1 });
    await until(() => h.messages.filter(m => m.type === 'error').length === 2);
    expect(rt.calls()).toBe(0);
    expect(h.messages.some(m => m.type === 'done')).toBe(false);
  });
  it('reports Rust errors and releases memory', async () => {
    const rt = runtime();
    rt.raw.NestSession.prototype.step = () => { throw new Error('invalid job'); };
    const h = harness(rt.raw);
    h.send({ type: 'nest', id: 1, job, options: {} });
    await until(() => h.messages.some(m => m.type === 'error'));
    expect(rt.freed).toHaveBeenCalledOnce();
  });
});

class FakeWorker {
  onmessage: ((event: MessageEvent<NestWorkerResponse>) => void) | null = null;
  onerror: ((event: ErrorEvent) => void) | null = null;
  onmessageerror: (() => void) | null = null;
  messages: NestWorkerRequest[] = [];
  terminate = vi.fn();
  postMessage(m: NestWorkerRequest) { this.messages.push(m); }
  emit(data: NestWorkerResponse) { this.onmessage?.({ data } as MessageEvent<NestWorkerResponse>); }
}
function client() {
  const worker = new FakeWorker();
  const nester = createMarkerNester({ moduleUrl: 'http://localhost/wasm.js', workerFactory: () => worker as unknown as Worker });
  return { worker, nester };
}
describe('typed worker helper', () => {
  const checkpoint = (): NestProgress => ({ round: 0, seconds: 0.1, round_seconds: 0.1,
    done: false, cancelled: false, improved: true, memory_bytes: 65536, result: structuredClone(result) });
  it('does not acknowledge within-round checkpoints or let callbacks corrupt recovery', async () => {
    vi.useFakeTimers();
    const { worker, nester } = client();
    worker.emit({ type: 'ready' }); await nester.ready;
    const pending = nester.nest(job, { time_limit_s: 1 }, p => { p.result.placements.length = 0; });
    worker.emit({ type: 'progress', id: 1, progress: { ...checkpoint(), checkpoint: true } });
    expect(worker.messages.filter(m => m.type === 'continue')).toHaveLength(0);
    await vi.advanceTimersByTimeAsync(6000);
    expect((await pending).placements).toEqual(result.placements);
  });
  it('terminates a stalled worker and returns an untouched private checkpoint at the budget', async () => {
    vi.useFakeTimers();
    const { worker, nester } = client();
    worker.emit({ type: 'ready' }); await nester.ready;
    const pending = nester.nest(job, { time_limit_s: 1 }, progress => {
      progress.result.placements[0]!.translate[0] = 999;
    });
    worker.emit({ type: 'progress', id: 1, progress: checkpoint() });
    await vi.advanceTimersByTimeAsync(6000);
    const saved = await pending;
    expect(saved.placements).toEqual(result.placements);
    expect(saved.warnings?.at(-1)).toContain('time limit');
    expect(nester.closed).toBe(true);
    expect(worker.terminate).toHaveBeenCalledOnce();
  });
  it('bounds Stop latency even if the running round never yields', async () => {
    vi.useFakeTimers();
    const { worker, nester } = client();
    worker.emit({ type: 'ready' }); await nester.ready;
    const progress = vi.fn();
    const pending = nester.nest(job, { time_limit_s: 60 }, progress);
    worker.emit({ type: 'progress', id: 1, progress: checkpoint() });
    nester.stop();
    await vi.advanceTimersByTimeAsync(5000);
    expect((await pending).placements).toEqual(result.placements);
    expect(progress.mock.calls.at(-1)?.[0].cancelled).toBe(true);
  });
  it('rejects a watchdog expiry when no complete checkpoint exists', async () => {
    vi.useFakeTimers();
    const { worker, nester } = client();
    worker.emit({ type: 'ready' }); await nester.ready;
    const observed = nester.nest(job, { time_limit_s: 1 }).catch(error => error as Error);
    await vi.advanceTimersByTimeAsync(6000);
    expect(await observed).toBeInstanceOf(Error);
    expect(nester.closed).toBe(true);
  });
  it('retains the checked checkpoint if the worker fails during improvement', async () => {
    const { worker, nester } = client();
    worker.emit({ type: 'ready' }); await nester.ready;
    const pending = nester.nest(job);
    worker.emit({ type: 'progress', id: 1, progress: checkpoint() });
    worker.emit({ type: 'error', id: 1, message: 'optimizer trapped' });
    const saved = await pending;
    expect(saved.placements).toEqual(result.placements);
    expect(saved.warnings?.at(-1)).toContain('optimizer trapped');
    expect(nester.closed).toBe(true);
    expect(worker.terminate).toHaveBeenCalledOnce();
  });
  it("routes progress, ignores stale responses, stops and returns the worker's incumbent", async () => {
    const { worker, nester } = client();
    worker.emit({ type: 'ready' });
    await nester.ready;
    const progress = vi.fn();
    const promise = nester.nest(job, {}, progress);
    const p = { result } as NestProgress;
    worker.emit({ type: 'progress', id: 99, progress: p });
    worker.emit({ type: 'progress', id: 1, progress: p });
    expect(progress).toHaveBeenCalledOnce();
    nester.stop();
    expect(worker.messages.at(-1)).toEqual({ type: 'cancel', id: 1 });
    worker.emit({ type: 'done', id: 1, result });
    expect(await promise).toEqual(result);
    nester.dispose();
    expect(worker.terminate).toHaveBeenCalledOnce();
  });
  it('rejects overlapping work, unsafe seeds and disposal during work', async () => {
    const { nester } = client();
    await expect(nester.nest(job, { seed: 2 ** 53 })).rejects.toThrow('safe integer');
    const pending = nester.nest(job);
    await expect(nester.nest(job)).rejects.toThrow('already running');
    nester.dispose();
    await expect(pending).rejects.toThrow('disposed');
    await expect(nester.nest(job)).rejects.toThrow('disposed');
  });
  it('propagates initialization failure and rejects later jobs', async () => {
    const { worker, nester } = client();
    worker.emit({ type: 'error', message: 'WASM unavailable' });
    await expect(nester.ready).rejects.toThrow('WASM unavailable');
    await expect(nester.nest(job)).rejects.toThrow('WASM unavailable');
    nester.dispose();
  });
  it('terminates and rejects if a progress callback throws', async () => {
    const { worker, nester } = client();
    const pending = nester.nest(job, {}, () => { throw new Error('callback failed'); });
    worker.emit({ type: 'progress', id: 1, progress: { result } as NestProgress });
    await expect(pending).rejects.toThrow('callback failed');
    expect(worker.terminate).toHaveBeenCalledOnce();
  });
});
