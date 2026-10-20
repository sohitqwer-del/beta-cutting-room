import { describe, expect, it, vi } from 'vitest';
import type { Job, MarkerResult, MarkerNester } from '@beta/marker-wasm';
import { better, exportFile, isPractice, loadQuickEngine, mockRequested, nestAvailable, nestInRounds, nestPractice, wrapCore } from './adapter';
import { mockCore, mockToJob } from './mock';

const job: Job = { format: 'beta-marker.job/v1', name: 'q', fabric: { width: 1500 }, pieces: [] };
const result = (length: number, efficiency = 0.8): MarkerResult => ({ format: 'beta-marker.result/v1', job_name: 'q', width: 1500, length, piece_area: 1, efficiency, engine: 'sparrow', placements: [] });
const json = (body: unknown, status = 200) => new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } });

describe('quick engine adapter', () => {
  it('reports a missing engine instead of pretending', async () => {
    const r = await loadQuickEngine({ module: {} });
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.reason).toBe('missing');
  });
  it('loads the real loader when the WASM package provides it', async () => {
    const loader = vi.fn(async () => mockCore());
    const r = await loadQuickEngine({ module: { loadQuickMarkerCore: loader } });
    expect(loader).toHaveBeenCalledOnce();
    expect(r.ok && r.engine.kind).toBe('wasm');
  });
  it('reports a loader failure with its message', async () => {
    const r = await loadQuickEngine({ module: { loadQuickMarkerCore: async () => { throw new Error('quick_scan is not exported'); } } });
    expect(r).toEqual({ ok: false, reason: 'failed', error: 'quick_scan is not exported' });
  });
  it('uses the mock only when asked', async () => {
    const r = await loadQuickEngine({ mock: true });
    expect(r.ok && r.engine.kind).toBe('mock');
    expect(mockRequested('?quick-mock=1')).toBe(true);
    expect(mockRequested('?x=1&quick-mock=1')).toBe(true);
    expect(mockRequested('?quick-mock=0')).toBe(false);
  });
  it('turns engine exceptions into rejected promises', async () => {
    const engine = wrapCore(mockCore(), 'mock');
    await expect(engine.scan(new Uint8Array(), { reference: 'a4-sheet' })).rejects.toThrow('empty');
    await expect(engine.toJob([], { width_mm: 1500 }, { M: 1 })).rejects.toThrow('Accept at least one piece');
  });
  it('mock job follows the engine contract: sizes must match the order', async () => {
    const engine = wrapCore(mockCore(), 'mock');
    const scan = await engine.scan(new Uint8Array([1]), { reference: 'a4-sheet', photoId: 'photo-1' });
    const session = await engine.merge([scan]);
    expect(session.pieces[0]!.id).toBe('photo-1:p1');
    const pieces = session.pieces.map(p => ({ ...p, size: 'M', grain: [[0, 0], [100, 0]] as [[number, number], [number, number]] }));
    expect(() => mockToJob(session.pieces.map(p => ({ ...p, size: 'M' })), { width_mm: 1500 }, { M: 1 })).toThrow('no confirmed grain');
    const j = mockToJob(pieces, { width_mm: 1117.6, nap: 'one_way', layout: 'folded' }, { M: 2 });
    expect(j.fabric).toEqual({ width: 1117.6, nap: 'one_way', layout: 'folded' });
    expect(j.order).toEqual({ sizes: { M: 2 } });
    expect(() => mockToJob(pieces, { width_mm: 1500 }, { M: 1, L: 1 })).toThrow('Size L has no pieces');
    const pdf = await engine.tiledPdf(j, result(1000), { paper: 'A4' });
    expect(new TextDecoder().decode(pdf.slice(0, 8))).toBe('%PDF-1.4');
  });
});

describe('nesting through the local Rust nester', () => {
  const local = (): MarkerNester => ({
    ready: Promise.resolve(),
    closed: false,
    nest: vi.fn(async (_job, _options, progress) => {
      const best = result(1900);
      progress?.({ round: 1, seconds: 1, round_seconds: 1, done: true,
        cancelled: false, improved: true, memory_bytes: 65536, result: best });
      return best;
    }),
    stop: vi.fn(), dispose: vi.fn(),
  });
  it('uses the browser engine without even probing the server', async () => {
    const fetcher = vi.fn(async () => { throw new Error('offline'); });
    const nester = local();
    expect(await nestAvailable(fetcher, () => nester)).toBe(true);
    const progress = vi.fn();
    const best = await nestInRounds(fetcher, job, { budgetS: 30, browserNester: () => nester, onProgress: progress });
    expect(best!.length).toBe(1900);
    expect(progress.mock.calls[0]![0].best).toEqual(best);
    expect(fetcher).not.toHaveBeenCalled();
    expect(nester.nest).toHaveBeenCalledWith(job, { time_limit_s: 30, round_time_s: 2, seed: 42 }, expect.any(Function));
  });
  it('forwards Stop to the Worker and retains its marker', async () => {
    const nester = local();
    const abort = new AbortController();
    const fetcher = vi.fn();
    const best = await nestInRounds(fetcher, job, { budgetS: 30, browserNester: () => nester,
      signal: abort.signal, onProgress: () => abort.abort() });
    expect(nester.stop).toHaveBeenCalledOnce();
    expect(best!.length).toBe(1900);
    expect(fetcher).not.toHaveBeenCalled();
  });
  it('does not hide a browser geometry failure behind a server request', async () => {
    const nester = local();
    nester.nest = async () => { throw new Error('invalid grain'); };
    const fetcher = vi.fn();
    await expect(nestInRounds(fetcher, job, { budgetS: 30, browserNester: () => nester })).rejects.toThrow('invalid grain');
    expect(fetcher).not.toHaveBeenCalled();
  });
  it('falls back only when the browser engine cannot load', async () => {
    const fetcher = vi.fn(async () => json(result(2000)));
    let clock = 0;
    const best = await nestInRounds(fetcher, job, { budgetS: 1, pollMs: 0, now: () => (clock += 1000),
      browserNester: () => { throw new Error('WebAssembly unavailable'); } });
    expect(best!.length).toBe(2000);
    expect(fetcher).toHaveBeenCalledOnce();
  });
  it('checks whether a nester is reachable', async () => {
    expect(await nestAvailable(async () => json({ ok: true }))).toBe(true);
    expect(await nestAvailable(async () => { throw new Error('offline'); })).toBe(false);
  });
  it('runs short rounds, warm-starts from the best and keeps the shortest marker', async () => {
    let clock = 0;
    const lengths = [2400, 2300, 2350];
    const bodies: { options: { time_limit_s: number; seed: number; warm_start: MarkerResult | null } }[] = [];
    const fetcher = vi.fn(async (_url: string, init?: RequestInit) => {
      bodies.push(JSON.parse(String(init!.body))); clock += 10000;
      return json(result(lengths[bodies.length - 1]!));
    });
    const progress: number[] = [];
    const best = await nestInRounds(fetcher, job, { budgetS: 30, roundS: 10, pollMs: 0, now: () => clock, onProgress: p => progress.push(p.best!.length) });
    expect(fetcher).toHaveBeenCalledTimes(3);
    expect(best!.length).toBe(2300);
    expect(bodies[0]!.options.warm_start).toBeNull();
    expect(bodies[2]!.options.warm_start!.length).toBe(2300);
    expect(bodies.map(b => b.options.seed)).toEqual([42, 43, 44]);
    expect(progress).toEqual([2400, 2300, 2300]);
  });
  it('stops after the running round when the user presses Stop', async () => {
    const stop = new AbortController();
    let clock = 0;
    const fetcher = vi.fn(async () => { clock += 1000; stop.abort(); return json(result(2000)); });
    const best = await nestInRounds(fetcher, job, { budgetS: 60, pollMs: 0, signal: stop.signal, now: () => clock });
    expect(fetcher).toHaveBeenCalledOnce();
    expect(best!.length).toBe(2000);
  });
  it('explains server errors in plain words', async () => {
    await expect(nestInRounds(async () => json({ error: 'Piece too wide for the fabric' }, 400), job, { budgetS: 5, pollMs: 0 })).rejects.toThrow('Piece too wide');
    await expect(nestInRounds(async () => json({}, 401), job, { budgetS: 5, pollMs: 0 })).rejects.toThrow('access token');
  });
  it('compares markers by length, then efficiency', () => {
    expect(better(result(2000), null)).toBe(true);
    expect(better(result(1990), result(2000))).toBe(true);
    expect(better(result(2000, 0.81), result(2000, 0.8))).toBe(true);
    expect(better(result(2010), result(2000))).toBe(false);
  });
  it('marks practice layouts so they are never shown as real', async () => {
    const pieces = (await wrapCore(mockCore(), 'mock').merge([mockCore().quickScan(new Uint8Array([1]), { reference: 'a4-sheet' })])).pieces.map(p => ({ ...p, size: 'M', grain: [[0, 0], [100, 0]] as [[number, number], [number, number]] }));
    const r = await nestPractice(mockToJob(pieces, { width_mm: 1500 }, { M: 1 }), { budgetS: 1 });
    expect(isPractice(r)).toBe(true);
    expect(r.placements.length).toBeGreaterThan(0);
    expect(isPractice(result(1))).toBe(false);
  });
  it('exports plotter files through the Rust exporter', async () => {
    const fetcher = vi.fn(async (_url: string, _init?: RequestInit) => new Response('IN;', { status: 200 }));
    const blob = await exportFile(fetcher, job, result(1000), 'plt');
    expect(await blob.text()).toBe('IN;');
    expect(fetcher.mock.calls[0]![0]).toBe('/api/export?format=plt');
  });
});
