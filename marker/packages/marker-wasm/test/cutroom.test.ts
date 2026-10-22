import { describe, expect, it, vi } from 'vitest';
import { bindCutRoomCore, createCutPlanner, installCutPlanWorker, STOPPED_PLAN_WARNING,
  type CutPlanWorkerPort, type CutPlanWorkerRequest, type CutPlanWorkerResponse } from '../src/cutroom';
import type { CutPlan, PlanRequest } from '../src/types';

const plan = (n: number, complete = false) => ({ markers_evaluated: n, complete, warnings: [] as string[], lays: [] }) as unknown as CutPlan;
type RawPlan = { plan(request: string, progress: (json: string) => boolean): string };

/** A planner client wired to the real worker code, in one thread. */
function pair(raw: RawPlan) {
  const port: CutPlanWorkerPort = { onmessage: null, postMessage: m => queueMicrotask(() => client.onmessage?.({ data: structuredClone(m) } as MessageEvent<CutPlanWorkerResponse>)) };
  const client = {
    onmessage: null as ((e: MessageEvent<CutPlanWorkerResponse>) => void) | null, onerror: null, terminate: vi.fn(),
    postMessage: (m: CutPlanWorkerRequest) => queueMicrotask(() => port.onmessage?.({ data: structuredClone(m) } as MessageEvent<CutPlanWorkerRequest>)),
  };
  installCutPlanWorker(port, async () => raw);
  return createCutPlanner({ workerFactory: () => client as unknown as Worker, moduleUrl: 'http://local/wasm/x.js' });
}
function fakeWorker() {
  return { onmessage: null as ((e: MessageEvent<CutPlanWorkerResponse>) => void) | null, onerror: null, postMessage: vi.fn(), terminate: vi.fn() };
}

describe('cutting-room WASM bindings', () => {
  it('copies reply fields and frees the Rust object', () => {
    const free = vi.fn();
    const core = bindCutRoomCore({ api_request: () => ({ status: 200, mime: 'application/json', extension: undefined, body: new Uint8Array([123, 125]), free }) });
    expect(core.request('/api/roll/inspect', new Uint8Array())).toEqual({ status: 200, mime: 'application/json', extension: null, body: new Uint8Array([123, 125]) });
    expect(free).toHaveBeenCalledOnce();
  });
  it('streams checked partial plans and resolves with the final plan', async () => {
    const planner = pair({ plan(request, progress) {
      expect(JSON.parse(request).order.max_plies).toBe(40);
      progress(JSON.stringify(plan(1)));
      progress(JSON.stringify(plan(2)));
      return JSON.stringify(plan(3, true));
    } });
    await planner.ready;
    const seen: number[] = [];
    const result = await planner.plan({ order: { max_plies: 40 } } as unknown as PlanRequest, p => seen.push(p.markers_evaluated));
    expect(seen).toEqual([1, 2]);
    expect(result.complete).toBe(true);
  });
  it('reports Rust errors as rejections', async () => {
    const planner = pair({ plan() { throw new Error('Folded and tube fabric lays two plies in each pass'); } });
    await expect(planner.plan({} as PlanRequest)).rejects.toThrow('two plies');
  });
  it('stop before any marker rejects and closes the worker', async () => {
    const worker = fakeWorker();
    const planner = createCutPlanner({ workerFactory: () => worker as unknown as Worker, moduleUrl: 'http://local/wasm/x.js' });
    const running = planner.plan({} as PlanRequest);
    planner.stop();
    await expect(running).rejects.toThrow('Stopped before any marker');
    expect(worker.terminate).toHaveBeenCalled();
    await expect(planner.plan({} as PlanRequest)).rejects.toThrow('closed');
  });
  it('stop keeps the last checked plan and marks it', async () => {
    const worker = fakeWorker();
    const planner = createCutPlanner({ workerFactory: () => worker as unknown as Worker, moduleUrl: 'http://local/wasm/x.js' });
    const running = planner.plan({} as PlanRequest);
    worker.onmessage!({ data: { type: 'progress', id: 1, plan: plan(5) } } as MessageEvent<CutPlanWorkerResponse>);
    planner.stop();
    const kept = await running;
    expect(kept.markers_evaluated).toBe(5);
    expect(kept.warnings).toContain(STOPPED_PLAN_WARNING);
    expect(worker.terminate).toHaveBeenCalled();
  });
});
