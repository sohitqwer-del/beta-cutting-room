import { describe, expect, it, vi } from 'vitest';
import type { Job, MarkerCore, PatternCore, PatternDocument, StyleCore } from '@beta/marker-wasm';
import { formatAngle, formatLength, fromMm, normaliseAngle, parseLength, step, toMm, widthProblem } from './units';
import { boundsOf, fitView, pan, rulerTicks, toScreen, toWorld, zoomAt } from './view';
import { chainLoop, inBox, inside, snapPoint } from './snap';
import * as S from './state';
import { joinCores, loadStyleEngine, matchStatus, practiceRequested } from './adapter';
import { clipLine } from './mock';
import en from './locales/en.json';
import hi from './locales/hi.json';
import ta from './locales/ta.json';
import { t } from './i18n';
import { planPayload } from '../workflows';
import { demoJob } from '../demo';

const doc = (pieces = 1, sizes = ['S', 'M', 'L']): PatternDocument => ({ format: 'beta-pattern.doc/v1', name: 'Tee', sizes, base_size: 'M', measurements: Object.fromEntries(sizes.map(s => [s, {}])), variables: {}, drawing: {},
  pieces: Array.from({ length: pieces }, (_, i) => ({ id: `p${i}`, name: `P${i}`, edges: [], material: i === 2 ? 'lining' : 'shell' })) });

describe('units', () => {
  it('converts cm and inch to millimetres and back', () => {
    expect(toMm(1.5, 'cm')).toBe(15);
    expect(toMm(60, 'inch')).toBeCloseTo(1524, 9);
    expect(fromMm(1524, 'inch')).toBeCloseTo(60, 9);
    expect(formatLength(1524, 'inch', true)).toBe('60 in');
    expect(formatLength(12.34, 'cm')).toBe('1.2');
    expect(formatLength(-0.01, 'cm')).toBe('0');
  });
  it('reads what people type: comma decimals, inch fractions and units', () => {
    expect(parseLength('1,5', 'cm')).toBe(15);
    expect(parseLength('1 1/2', 'inch')).toBeCloseTo(38.1, 9);
    expect(parseLength('3/8"', 'inch')).toBeCloseTo(9.525, 9);
    expect(parseLength('2 cm', 'cm')).toBe(20);
    expect(parseLength('-2', 'cm')).toBeNull();
    expect(parseLength('-2', 'cm', true)).toBe(-20);
    expect(parseLength('abc', 'cm')).toBeNull();
    expect(parseLength('1/0', 'inch')).toBeNull();
    expect(parseLength('', 'cm')).toBeNull();
  });
  it('uses the unit the person typed, not the screen unit', () => {
    expect(parseLength('58 in', 'cm')).toBeCloseTo(1473.2, 9);
    expect(parseLength('58"', 'cm')).toBeCloseTo(1473.2, 9);
    expect(parseLength('150 cm', 'inch')).toBe(1500);
    expect(parseLength('1-1/2 in', 'cm')).toBeCloseTo(38.1, 9);
    expect(parseLength('1-1/2', 'inch')).toBeCloseTo(38.1, 9);
    expect(parseLength('.5', 'cm')).toBe(5);
  });
  it('refuses lengths that could be read two ways or are not plain numbers', () => {
    expect(parseLength('1,500', 'cm')).toBeNull();
    expect(parseLength('1e3', 'cm')).toBeNull();
    expect(parseLength('0x10', 'cm')).toBeNull();
    expect(parseLength('Infinity', 'cm')).toBeNull();
    expect(parseLength('in', 'inch')).toBeNull();
  });
  it('keeps angles in (-180, 180] and limits widths and steppers', () => {
    expect(normaliseAngle(270)).toBe(-90);
    expect(normaliseAngle(-180)).toBe(180);
    expect(formatAngle(450)).toBe('90°');
    expect(widthProblem(150)).toBe('too_small');
    expect(widthProblem(5000)).toBe('too_large');
    expect(widthProblem(1524)).toBeNull();
    expect(step(1, -5, 0, 10)).toBe(0);
    expect(step(9, 5, 0, 10)).toBe(10);
  });
});

describe('canvas view', () => {
  it('maps screen and world both ways with y up', () => {
    const v = { left: 100, top: 500, scale: 2 };
    expect(toWorld(v, [0, 0])).toEqual([100, 500]);
    expect(toWorld(v, [20, 40])).toEqual([110, 480]);
    expect(toScreen(v, toWorld(v, [33, 77]))).toEqual([33, 77]);
  });
  it('zooms around the pointer and pans by pixels', () => {
    const v = { left: 0, top: 100, scale: 1 };
    const z = zoomAt(v, [50, 50], 2);
    expect(toWorld(z, [50, 50])).toEqual(toWorld(v, [50, 50]));
    expect(z.scale).toBe(2);
    expect(toWorld(pan(v, 10, 0), [10, 0])).toEqual([0, 100]);
  });
  it('fits bounds into the screen with a margin', () => {
    const v = fitView([0, 0, 1000, 500], 600, 400, 50);
    expect(v.scale).toBeCloseTo(0.5, 9);
    expect(toScreen(v, [500, 250])).toEqual([300, 200]);
    expect(boundsOf([[1, 2], [-3, 5]])).toEqual([-3, 2, 1, 5]);
    expect(boundsOf([])).toBeNull();
  });
  it('labels ruler ticks in cm or inch with readable spacing', () => {
    const cm = rulerTicks(0, 1000, 1, 'cm');
    expect(cm.filter(t => t.major).map(t => t.label).slice(0, 3)).toEqual(['0', '10', '20']);
    const inch = rulerTicks(0, 1000, 1, 'inch');
    expect(inch.filter(t => t.major)[1]!.mm).toBeCloseTo(152.4, 6);
    expect(rulerTicks(0, 1e9, 1e-6, 'cm').length).toBeLessThanOrEqual(2001);
  });
});

describe('snapping', () => {
  const targets = [{ id: 'a', at: [0, 0] as [number, number] }, { id: 'b', at: [100, 50] as [number, number] }];
  const base = { radius: 5, points: true, grid: 10, angleStep: 15, align: true };
  it('prefers an existing point inside the radius', () => {
    const r = snapPoint([98, 52], targets, base);
    expect(r).toMatchObject({ kind: 'point', target: 'b', at: [100, 50] });
    expect(r.guides[0]!.kind).toBe('ring');
  });
  it('snaps the direction to 15° steps from the anchor and tidies the length to the grid', () => {
    const r = snapPoint([99, 3], [], { ...base, anchor: [0, 0], align: false });
    expect(r.kind).toBe('angle');
    expect(r.angle).toBe(0);
    expect(r.at[0]).toBeCloseTo(100, 9); expect(r.at[1]).toBeCloseTo(0, 9);
    const r45 = snapPoint([70, 72], [], { ...base, anchor: [0, 0], grid: 0, align: false });
    expect(r45.angle).toBe(45);
  });
  it('aligns with other points and shows a guide, then falls back to the grid', () => {
    const r = snapPoint([100.4, 20.2], targets, { ...base, anchor: null });
    expect(r.kind).toBe('align'); expect(r.at).toEqual([100, 20]); expect(r.guides).toHaveLength(1);
    expect(snapPoint([33, 27], [], base)).toMatchObject({ kind: 'grid', at: [30, 30] });
    expect(snapPoint([33, 27], [], { ...base, grid: 0 })).toMatchObject({ kind: 'free', at: [33, 27] });
    expect(snapPoint([1, 1], targets, { ...base, ignore: ['a'] }).kind).not.toBe('point');
  });
  it('tests inside outlines and boxes', () => {
    const sq = [[0, 0], [10, 0], [10, 10], [0, 10]] as [number, number][];
    expect(inside(sq, [5, 5])).toBe(true); expect(inside(sq, [15, 5])).toBe(false);
    expect(inBox([5, 5], [10, 0], [0, 10])).toBe(true);
  });
  it('orders selected lines into one closed loop for "Make a piece"', () => {
    const loop = chainLoop([{ id: 'a', start: [0, 0], end: [10, 0] }, { id: 'c', start: [0, 10], end: [10, 10] }, { id: 'b', start: [10, 0], end: [10, 10] }, { id: 'd', start: [0, 10], end: [0, 0] }]);
    expect(loop).toEqual([{ object: 'a', reversed: false }, { object: 'b', reversed: false }, { object: 'c', reversed: true }, { object: 'd', reversed: false }]);
    expect(chainLoop([{ id: 'a', start: [0, 0], end: [10, 0] }, { id: 'b', start: [10, 0], end: [10, 10] }])).toBeNull();
  });
});

describe('style flow state', () => {
  it('opens steps in order and says why a step is blocked', () => {
    let s = S.initialSession();
    expect(S.blocker(s, 'pattern')).toBe('no_name');
    s = S.setInfo(s, { name: 'Tee' });
    expect(S.blocker(s, 'pattern')).toBe('no_start');
    s = S.setDoc(s, doc(0), { kind: 'blank' });
    expect(S.blocker(s, 'pattern')).toBeNull();
    expect(S.blocker(s, 'grade')).toBe('no_piece');
    s = S.setDoc(s, doc(1));
    expect(S.blocker(s, 'gaps')).toBeNull();
    expect(S.blocker(s, 'cut')).toBe('no_marker');
    expect(S.go(s, 'cut')).toBe(s);
    expect(S.go(s, 'lay').step).toBe('lay');
    expect(S.nextStep('stripes')).toBe('marker'); expect(S.prevStep('style')).toBeNull();
  });
  it('checks the lay: width, even plies for folded and face-to-face lays, ratio', () => {
    let s = S.setDoc(S.setInfo(S.initialSession(), { name: 'Tee' }), doc(1));
    expect(s.lay.ratio).toEqual({ S: 1, M: 1, L: 1 });
    s = S.setLay(s, { layout: 'folded', plies: 41 });
    expect(S.blocker(s, 'marker')).toBe('lay_plies');
    s = S.setLay(s, { plies: 40, face: 'face_to_face', layout: 'open' });
    expect(S.layProblem(s.lay)).toBeNull();
    expect(S.layProblem({ ...s.lay, widthMm: 100 })).toBe('lay_width');
    expect(S.layProblem({ ...s.lay, ratio: { S: 0 } })).toBe('lay_ratio');
    expect(S.repeatProblem({ ...s.repeat, kind: 'stripes', direction: 'along', periodY: 0 })).toBe('period');
  });
  it('keeps the marker but marks it out of date when settings change', () => {
    let s = S.setDoc(S.setInfo(S.initialSession(), { name: 'Tee' }), doc(1));
    s = { ...s, marker: { status: 'done', stale: false, result: {} as never } };
    expect(S.blocker(s, 'cut')).toBeNull();
    expect(S.setGaps(s, { pieceGap: 2 }).marker.stale).toBe(true);
    expect(S.blocker({ ...s, marker: { ...s.marker, status: 'running' } }, 'lay')).toBe('busy');
  });
  it('builds engine settings: stripes along the length repeat across the width', () => {
    let s = S.setDoc(S.setInfo(S.initialSession(), { name: 'Tee' }), doc(3));
    s = S.setRepeat(S.setGaps(s, { pieceGap: 3, buffers: { p0: 2 }, blocks: { p1: 10 } }), { kind: 'stripes', direction: 'along', periodY: 10, offsetY: 2 });
    s = S.setLay(s, { ratio: { S: 0, M: 2, L: 1 }, endAllowanceMm: 15, plies: 20 });
    const j = S.jobSettings(s);
    expect(j.fabric.repeat).toEqual({ period_x: null, period_y: 10, offset_x: 0, offset_y: 2, tolerance_mm: 1 });
    expect(j.ratio).toEqual({ M: 2, L: 1 });
    expect(j.piece_settings).toEqual({ p0: { buffer: 2 }, p1: { block_margin: 10 } });
    expect(j.lay).toEqual({ face: 'up', plies: 20, end_allowance_mm: 15 });
    expect(j.material).toBe('shell');
    expect(S.materials(s.doc)).toEqual(['shell', 'lining']);
    expect(S.jobSettings(S.setRepeat(s, { kind: 'none' })).fabric.repeat).toBeNull();
  });
  it('works out spread length and fabric for open and folded lays', () => {
    expect(S.layFabric(5000, { endAllowanceMm: 20, plies: 40, layout: 'open' })).toEqual({ perPlyMm: 5040, totalMm: 201600, passes: 40 });
    expect(S.layFabric(5000, { endAllowanceMm: 20, plies: 40, layout: 'folded' })).toEqual({ perPlyMm: 5040, totalMm: 100800, passes: 20 });
    expect(S.layFabric(5000, { endAllowanceMm: 20, plies: 40, layout: 'tubular' }).passes).toBe(20);
  });
  it('hands the cut planner plies (layers), never passes: cut order v2 halves folded plies itself', () => {
    // marker/docs/PLIES.md: 40 plies of folded fabric = 20 passes. Handing over 20 would halve twice (10 passes).
    const lay = { ...S.initialSession().lay, layout: 'folded' as const, plies: 40, endAllowanceMm: 15, order: { M: 120, L: 40 } };
    expect(S.layFabric(5000, lay).passes).toBe(20);
    const form = S.cutPlanHandoff(lay, ['S', 'M', 'L']);
    expect(form).toEqual({ quantities: [0, 120, 40], maxPlies: 40, endAllowanceMm: 15 });
    const payload = planPayload(demoJob, { quantities: { M: 120, L: 40 }, maxPlies: form.maxPlies, maxGarments: 4, overcut: 0, time: 1, endAllowance: form.endAllowanceMm, layCost: 0 });
    expect(payload.order.format).toBe('beta-marker.cutorder/v2');
    expect(payload.order.max_plies).toBe(40);
    const open = { ...lay, layout: 'open' as const };
    expect(S.layFabric(5000, open).passes).toBe(40);
    expect(S.cutPlanHandoff(open, ['M']).maxPlies).toBe(40);
  });
  it('saves and restores the session, never the marker', () => {
    const store = new Map<string, string>();
    const storage = { getItem: (k: string) => store.get(k) ?? null, setItem: (k: string, v: string) => void store.set(k, v), removeItem: (k: string) => void store.delete(k) };
    let s = S.setDoc(S.setInfo(S.initialSession(), { name: 'Tee' }), doc(1));
    s = { ...S.go(s, 'grade'), marker: { status: 'done', stale: false, result: {} as never } };
    expect(S.save(s, storage)).toBe(true);
    const back = S.restore(storage)!;
    expect(back.step).toBe('grade'); expect(back.info.name).toBe('Tee'); expect(back.marker.status).toBe('idle');
    store.set('beta-style-v1', JSON.stringify({ ...s, step: 'cut' }));
    expect(S.restore(storage)!.step).toBe('marker');
    store.set('beta-style-v1', '{bad'); expect(S.restore(storage)).toBeNull();
    S.clearSaved(storage); expect(S.restore(storage)).toBeNull();
  });
});

describe('engine adapter', () => {
  const pattern = { inspect: vi.fn(() => ({ objects: {}, pieces: [] })), nest: vi.fn(() => ({ sizes: { M: [] } })), export_dxf: vi.fn(() => 'DXF') } as unknown as PatternCore;
  const style = { blocks: vi.fn(() => [{ id: 't_shirt', name: 'T-shirt', pieces: [], note: '' }]), edit: vi.fn(() => { throw new Error('Edge 9 does not exist.'); }) } as unknown as StyleCore;
  it('uses the real Style core when the WASM has it', async () => {
    const r = await loadStyleEngine({ loaders: { style: async () => style, pattern: async () => pattern, marker: async () => ({}) as MarkerCore } });
    expect(r.ok && r.engine.kind).toBe('style');
    if (r.ok) expect((await r.engine.blocks())[0]!.id).toBe('t_shirt');
  });
  it('reports a Pattern WASM without style tools as an error (no stand-in engine)', async () => {
    const r = await loadStyleEngine({ loaders: { style: async () => { throw new Error('Update the local Pattern WASM files to use style tools.'); }, pattern: async () => pattern, marker: async () => ({}) as MarkerCore } });
    expect(r).toEqual({ ok: false, error: 'Update the local Pattern WASM files to use style tools.' });
  });
  it('reports a missing Pattern WASM instead of pretending', async () => {
    const r = await loadStyleEngine({ loaders: { pattern: async () => { throw new Error('no wasm'); } } });
    expect(r).toEqual({ ok: false, error: 'no wasm' });
  });
  it('turns engine exceptions into rejected promises and routes geometry to the Pattern core', async () => {
    const e = joinCores(style, pattern, undefined, 'style');
    await expect(e.edit(doc(1), [])).rejects.toThrow('Edge 9 does not exist.');
    await expect(e.validate({} as never, {} as never)).rejects.toThrow(/not loaded/);
    expect(await e.nestSizes(doc(1), 'p0.a')).toEqual({ M: [] });
    expect(await e.exportPatternDxf(doc(1))).toBe('DXF');
  });
  it('offers the spec sheet only when the engine measures one', async () => {
    expect(joinCores(style, pattern, undefined, 'style').specSheet).toBeUndefined();
    const sheet = { schema: 'beta-pattern.spec/v1', style: 's', block: 't_shirt', unit: 'mm', sizes: ['M'], base_size: 'M', rows: [], notes: [] };
    const quality = { ...style, block_checks: vi.fn(), spec_sheet: vi.fn(() => sheet), export_spec_sheet: vi.fn(() => ({ filename: 'spec.pdf', mime: 'application/pdf', bytes: [37, 80] })) };
    const e = joinCores(quality as never, pattern, undefined, 'style');
    expect(await e.specSheet!(doc(1))).toBe(sheet);
    expect((await e.exportSpecSheet!(doc(1), 'pdf')).filename).toBe('spec.pdf');
    quality.spec_sheet.mockImplementation(() => { throw new Error('Topology changed.'); });
    await expect(e.specSheet!(doc(1))).rejects.toThrow('Topology changed.');
  });
  it('reads the practice switch from the address', () => {
    expect(practiceRequested('?style-practice=1')).toBe(true);
    expect(practiceRequested('?x=1&style-practice=1')).toBe(true);
    expect(practiceRequested('?style-practice=10')).toBe(false);
  });
  it('reads match status from the Rust validator and positions from Rust transform', () => {
    const marker = { transform: vi.fn((pts: [number, number][], _m: boolean, _r: number, t: [number, number]) => pts.map(([x, y]) => [x + t[0], y + t[1]])) } as unknown as MarkerCore;
    const job = { format: 'j', name: 'j', fabric: { width: 1000 }, pieces: [{ id: 'f-M', name: 'F', outline: [], match_points: [{ id: 'side', point: [1, 2] }] }, { id: 'b-M', name: 'B', outline: [], match_points: [{ id: 'side', point: [3, 4] }] }],
      match_rules: [{ a: { piece_id: 'f-M', point_id: 'side' }, b: { piece_id: 'b-M', point_id: 'side' }, axes: 'y' as const }] } as unknown as Job;
    const pl = (piece_id: string, bundle: number, x: number) => ({ piece_id, copy: 0, mirrored: false, rotation_deg: 0, translate: [x, 0] as [number, number], bundle });
    const result = { format: 'r', job_name: 'j', width: 1000, length: 1, piece_area: 1, efficiency: 1, engine: 'sparrow', placements: [pl('f-M', 0, 10), pl('b-M', 0, 20), pl('f-M', 1, 30), pl('b-M', 1, 40)] };
    const report = { ok: false, length: 1, piece_area: 1, efficiency: 1, warnings: [], errors: [{ code: 'match', message: 'Bundle 1 does not satisfy matching point f-M/side.', piece_ids: ['f-M'] }] };
    const rows = matchStatus(marker, job, result, report);
    expect(rows.map(r => [r.garment, r.pass])).toEqual([[0, true], [1, false]]);
    expect(rows[0]!.a).toEqual([11, 2]); expect(rows[0]!.b).toEqual([23, 4]);
    expect(matchStatus(undefined, job, result, report)).toEqual([]);
  });
  it('clips stripe lines to an outline', () => {
    const sq = [[0, 0], [10, 0], [10, 10], [0, 10]] as [number, number][];
    expect(clipLine(sq, 'y', 5)).toEqual([[[0, 5], [10, 5]]]);
    expect(clipLine(sq, 'x', 20)).toEqual([]);
  });
});

describe('style strings', () => {
  it('has a draft for every language, marked for review, with the same placeholders', () => {
    for (const [name, lang] of Object.entries({ hi, ta }) as Array<[string, Record<string, string>]>) {
      expect(lang._status, name).toMatch(/review/i);
      for (const [key, text] of Object.entries(lang)) {
        if (key === '_status') continue;
        expect(key in en, `${name}.${key}`).toBe(true);
        const holes = (s: string) => (s.match(/\{\w+\}/g) ?? []).sort().join();
        expect(holes(text), `${name}.${key}`).toBe(holes((en as Record<string, string>)[key]!));
      }
    }
  });
  it('fills placeholders and falls back to English', () => {
    expect(t('n_pieces', { n: 3 }, 'en')).toBe('3 pieces');
    expect(t('step_of', { n: 2, total: 8 }, 'ta')).toMatch(/2/);
  });
});

describe('plies', () => {
  it('gives one clear reason for each problem', async () => {
    const S = await import('./state');
    const open = { layout: 'open' as const, face: 'up' as const }, folded = { layout: 'folded' as const, face: 'up' as const };
    expect(S.pliesProblem(40, open)).toBeNull();
    expect(S.pliesProblem(0, open)).toBe('plies_whole');
    expect(S.pliesProblem(-5, open)).toBe('plies_whole');
    expect(S.pliesProblem(2.5, open)).toBe('plies_whole');
    expect(S.pliesProblem(1e12, open)).toBe('plies_max');
    expect(S.pliesProblem(41, folded)).toBe('plies_even_error');
    expect(S.pliesProblem(41, open)).toBeNull();
  });
});
