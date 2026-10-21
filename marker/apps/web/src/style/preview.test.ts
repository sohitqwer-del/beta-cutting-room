import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { beforeAll, describe, expect, it } from 'vitest';
import type { MarkerCore, PatternCore, PatternDocument, StyleCore } from '@beta/marker-wasm';
import { previewCore } from './mock';
import { mockNest } from '../quick/mock';
import { jobSettings, initialSession, setDoc, setRepeat } from './state';

/** Preview engine against the real local Rust WASM (skipped when the WASM has not been built). */
const pkg = fileURLToPath(new URL('../../../../packages/marker-wasm/pkg/', import.meta.url));
const built = existsSync(`${pkg}beta_pattern_wasm_bg.wasm`) && existsSync(`${pkg}beta_marker_wasm_bg.wasm`);
let core: StyleCore, pattern: PatternCore;

beforeAll(async () => {
  if (!built) return;
  const p = await import(/* @vite-ignore */ `${pkg}beta_pattern_wasm.js`) as Record<string, (...a: unknown[]) => string> & { initSync(o: { module: Buffer }): void };
  p.initSync({ module: readFileSync(`${pkg}beta_pattern_wasm_bg.wasm`) });
  const m = await import(/* @vite-ignore */ `${pkg}beta_marker_wasm.js`) as Record<string, (...a: unknown[]) => string> & { initSync(o: { module: Buffer }): void };
  m.initSync({ module: readFileSync(`${pkg}beta_marker_wasm_bg.wasm`) });
  const j = JSON.stringify, P = JSON.parse;
  pattern = {
    inspect: (d: PatternDocument, s: string) => P(p.inspect!(j(d), s)), grade: (d: PatternDocument) => P(p.grade!(j(d))), nest: (d: PatternDocument, r: string) => P(p.nest!(j(d), r)),
    move_point: (d: PatternDocument, id: string, s: string, t: unknown) => P(p.move_point!(j(d), id, s, j(t))), piece_action: (d: PatternDocument, r: unknown) => P(p.piece_action!(j(d), j(r))),
    to_job: (d: PatternDocument, f: unknown, r: unknown) => P(p.to_job!(j(d), j(f), j(r))),
  } as unknown as PatternCore;
  const marker = { drawing: (job: unknown, pl: unknown) => P(m.drawing!(j(job), j(pl))), transform: (pts: unknown, mi: boolean, r: number, t: unknown) => P(m.transform!(j(pts), mi, r, j(t))),
    validate: (job: unknown, res: unknown) => P(m.validate!(j(job), j(res))) } as unknown as MarkerCore;
  core = previewCore(pattern, marker);
});

describe.skipIf(!built)('preview style engine on the local Rust WASM', () => {
  it('makes every block valid in every size, with seam checks', () => {
    for (const block of core.blocks()) {
      const doc = core.create({ name: block.name, block: block.id, chart: core.size_chart() });
      expect(doc.pieces.length, block.id).toBe(block.pieces.length);
      for (const size of doc.sizes) expect(pattern.inspect(doc, size).pieces.every(p => p.ok), `${block.id} ${size}`).toBe(true);
      const seams = core.block_seams(doc);
      expect(seams.length, block.id).toBeGreaterThan(0);
      expect(seams[0]!.rows).toHaveLength(doc.sizes.length);
    }
  });
  it('grades: bigger sizes give longer side seams', () => {
    const doc = core.create({ block: 't_shirt', chart: core.size_chart() });
    const rows = core.block_seams(doc)[0]!.rows;
    expect(rows.at(-1)!.a_mm).toBeGreaterThan(rows[0]!.a_mm);
    expect(rows.every(r => r.pass)).toBe(true);
  });
  it('moves a point and keeps its grading formula', () => {
    const doc = core.create({ block: 't_shirt', chart: core.size_chart() });
    const before = pattern.inspect(doc, 'S').objects['front.arm1_r']!.points[0]!;
    const M = pattern.inspect(doc, 'M').objects['front.arm1_r']!.points[0]!;
    const next = core.edit(doc, [{ op: 'move_point', id: 'front.arm1_r', size: 'M', target: [M[0] + 12, M[1] - 5] }]);
    const after = pattern.inspect(next, 'S').objects['front.arm1_r']!.points[0]!;
    expect(after[0] - before[0]).toBeCloseTo(12, 3);
    expect(after[1] - before[1]).toBeCloseTo(-5, 3);
  });
  it('draws a rectangle piece on a blank sheet in one transaction', () => {
    const blank = core.create({ name: 'Pocket', chart: core.size_chart() });
    const ops = [['a', 0, 0], ['b', 150, 0], ['c', 150, 180], ['d', 0, 180]].map(([id, x, y]) => ({ op: 'put_object' as const, id: `pk.${id}`, object: { kind: 'point' as const, operation: { op: 'absolute' as const, x: x as number, y: y as number } } }));
    const lines = [['ab', 'a', 'b'], ['bc', 'b', 'c'], ['cd', 'c', 'd'], ['da', 'd', 'a']].map(([id, f, t]) => ({ op: 'put_object' as const, id: `pk.${id}`, object: { kind: 'line' as const, from: `pk.${f}`, to: `pk.${t}` } }));
    const doc = core.edit(blank, [...ops, ...lines, { op: 'put_piece', piece: { id: 'pocket', name: 'Pocket', quantity: 2, edges: ['ab', 'bc', 'cd', 'da'].map(e => ({ object: `pk.${e}`, allowance: 10 })) } },
      { op: 'set_allowance', piece: 'pocket', edges: [3], width: 30, corner: 'square' },
      { op: 'add_annotation', piece: 'pocket', annotation: { kind: 'shaped_notch', edge: 0, distance: 75, depth: 3, width: 3, shape: 'slit' } }]);
    expect(pattern.inspect(doc, 'M').pieces[0]!.ok).toBe(true);
    expect(() => core.edit(doc, [{ op: 'delete_object', id: 'pk.a' }])).toThrow(/used by/);
  });
  it('builds a job with gaps, buffers, lay and per-size match rules, then overlays stripes and match status', () => {
    let doc = core.create({ block: 't_shirt', chart: core.size_chart() });
    doc = core.edit(doc, ['front', 'back'].map(id => ({ op: 'put_piece' as const, replace: true, piece: { ...doc.pieces.find(p => p.id === id)!, anchors: { side_hem: `${id}.hem_r` } } })));
    let s = setDoc(initialSession(), doc);
    s = { ...s, lay: { ...s.lay, ratio: { M: 1, L: 1, S: 0, XL: 0, XXL: 0 } }, gaps: { ...s.gaps, pieceGap: 4, buffers: { sleeve: 2 } } };
    s = setRepeat(s, { kind: 'stripes', direction: 'along', periodY: 10, matches: [{ a: { piece_id: 'front', point_id: 'side_hem' }, b: { piece_id: 'back', point_id: 'side_hem' }, axes: 'y' }] });
    const job = core.to_job(doc, jobSettings(s));
    expect(job.spacing).toEqual({ piece_gap: 4, edge_gap: 5 });
    expect(job.pieces.find(p => p.id === 'sleeve-M')!.buffer).toBe(2);
    expect(job.match_rules).toHaveLength(2);
    expect(job.pieces.find(p => p.id === 'front-M')!.match_points![0]!.id).toBe('side_hem');
    expect(job.fabric.repeat).toMatchObject({ period_y: 10, period_x: null });
    const result = mockNest(job);
    const o = core.overlay(job, result) as ReturnType<StyleCore['overlay']> & { matches?: unknown[] };
    expect(o.stripes.length).toBeGreaterThan(10);
    expect(o.stripes.every(l => l.axis === 'y')).toBe(true);
    expect(o.spread_length_mm).toBeCloseTo(result.length + 40, 6);
    expect(o.matches?.length).toBeGreaterThan(0);
  });
});
