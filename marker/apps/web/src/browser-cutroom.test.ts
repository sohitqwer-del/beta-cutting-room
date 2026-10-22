import { describe, expect, it, vi } from 'vitest';
import { browserCutRoom, type CutRoomDeps } from './browser-cutroom';
import type { CutPlan, CutRoomCore } from '@beta/marker-wasm';

function deps(overrides: Partial<CutRoomDeps> = {}): CutRoomDeps & { request: ReturnType<typeof vi.fn> } {
  const request = vi.fn().mockReturnValue({ status: 200, mime: 'application/pdf', extension: null, body: new Uint8Array([37, 80, 68, 70]) });
  return { request, core: async () => ({ request } as CutRoomCore), plan: vi.fn(), nest: vi.fn(), ...overrides };
}

describe('browser cutting-room transport', () => {
  it.each(['/api/roll/inspect', '/api/roll/spread', '/api/cutroom/bundles', '/api/cutroom/tickets', '/api/cutroom/report-marker', '/api/validate',
    '/api/import?format=plt&width=1500', '/api/export?format=report', '/api/export?format=xlsx', '/api/export?format=mini', '/api/export?format=labels'])(
    'answers %s with the Rust module bytes', async url => {
      const d = deps();
      const response = await browserCutRoom(url, { method: 'POST', body: new Uint8Array([1, 2, 255]) }, d);
      expect(response?.status).toBe(200);
      expect(response?.headers.get('Content-Type')).toBe('application/pdf');
      expect(Array.from(new Uint8Array(await response!.arrayBuffer()))).toEqual([37, 80, 68, 70]);
      expect(d.request).toHaveBeenCalledWith(url, new Uint8Array([1, 2, 255]));
    });
  it('passes Rust status, extension and JSON errors through', async () => {
    const d = deps();
    d.request.mockReturnValue({ status: 400, mime: 'application/json', extension: 'txt', body: new TextEncoder().encode('{"error":"Paper must be a4"}') });
    const response = await browserCutRoom('/api/cutroom/tickets', { method: 'POST', body: '{}' }, d);
    expect(response?.status).toBe(400);
    expect(response?.headers.get('X-Beta-Extension')).toBe('txt');
    expect(await response?.json()).toEqual({ error: 'Paper must be a4' });
    const broken = await browserCutRoom('/api/roll/spread', { method: 'POST', body: '{}' }, deps({ core: async () => { throw new Error('WASM missing'); } }));
    expect(broken?.status).toBe(400);
    expect(await broken?.json()).toEqual({ error: 'WASM missing' });
  });
  it('plans in the worker and re-nests with the browser nester', async () => {
    const plan = { complete: true, lays: [] } as unknown as CutPlan;
    const d = deps({ plan: vi.fn().mockResolvedValue(plan), nest: vi.fn().mockResolvedValue({ placements: [], length: 5 }) });
    const request = { order: { format: 'beta-marker.cutorder/v2' }, options: { time_per_marker_s: 1 } };
    const planned = await browserCutRoom('/api/plan', { method: 'POST', body: JSON.stringify(request) }, d);
    expect(await planned?.json()).toEqual(plan);
    expect(d.plan).toHaveBeenCalledWith(request, expect.any(Function));
    const nested = await browserCutRoom('/api/nest', { method: 'POST', body: JSON.stringify({ job: { name: 'j' }, options: { time_limit_s: 3, seed: 42, workers: 2, runs: 1 } }) }, d);
    expect(await nested?.json()).toEqual({ placements: [], length: 5 });
    expect(d.nest).toHaveBeenCalledWith({ name: 'j' }, expect.objectContaining({ time_limit_s: 3, seed: 42 }));
    const failed = await browserCutRoom('/api/plan', { method: 'POST', body: '{}' }, deps({ plan: vi.fn().mockRejectedValue(new Error('two plies in each pass')) }));
    expect(failed?.status).toBe(400);
    expect(await failed?.json()).toEqual({ error: 'two plies in each pass' });
  });
  it('leaves other routes and methods to the other transports', async () => {
    const d = deps();
    for (const url of ['/api/export?format=pdf', '/api/export?format=plt', '/api/import?format=dxf', '/api/digitize', '/api/pattern/grade']) {
      expect(await browserCutRoom(url, { method: 'POST', body: '{}' }, d)).toBeNull();
    }
    expect(await browserCutRoom('/api/roll/inspect', undefined, d)).toBeNull();
    expect(d.request).not.toHaveBeenCalled();
  });
});
