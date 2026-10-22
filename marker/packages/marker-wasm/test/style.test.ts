import { describe, expect, it } from 'vitest';
import { bindStyleCore, type StyleOperation, type StyleEditOperation, type StyleJobSettings } from '../src/style';
import type { PatternDocument } from '../src/pattern';
import type { Job, MarkerResult } from '../src/types';

describe('style loader wire contract', () => {
  const document = { format: 'beta-pattern.doc/v1', name: 'Test', sizes: ['M'], base_size: 'M',
    measurements: {}, variables: {}, drawing: {}, pieces: [] } satisfies PatternDocument;
  it('sends an atomic operation batch without changing the input snapshot', () => {
    let request: unknown;
    const core = bindStyleCore({ style_command(raw) { request = JSON.parse(raw); return JSON.stringify({ ...document, name: 'Edited' }); } });
    const operations: StyleOperation[] = [{ op: 'put_object', id: 'origin', object: { kind: 'point', operation: { op: 'absolute', x: 0, y: 'm.chest/4' } } }];
    expect(core.edit(document, operations).name).toBe('Edited');
    expect(request).toEqual({ action: 'edit', document, operations });
    expect(document.name).toBe('Test');
    expect(document.drawing).toEqual({});
  });
  it('preserves physical lay, match rules, repeat colours and per-piece gaps', () => {
    let request: unknown;
    const core = bindStyleCore({ style_command(raw) { request = JSON.parse(raw); return '{"format":"beta-marker.job/v1"}'; } });
    const settings: StyleJobSettings = { fabric: { width: 750, layout: 'folded', repeat: { period_x: 10 } },
      spacing: { piece_gap: 3, edge_gap: 5 }, ratio: { S: 1, M: 2 },
      lay: { face: 'up', plies: 20, end_allowance_mm: 15 },
      repeat_bands: [{ axis: 'x', offset_mm: 2, width_mm: 1, colour: '#224466' }],
      piece_settings: { front: { buffer: 2, block_margin: null } },
      matches: [{ a: { piece_id: 'front', point_id: 'underarm' }, b: { piece_id: 'back', point_id: 'underarm' }, axes: 'x' }] };
    core.to_job(document, settings);
    expect(request).toEqual({ action: 'to_job', document, settings });
  });
  it('wraps each request in the native dispatcher schema', () => {
    const requests: Record<string, unknown>[] = [];
    const core = bindStyleCore({ style_command(raw) { requests.push(JSON.parse(raw)); return 'null'; } });
    core.blocks(); core.size_chart(); core.create({ block: 't_shirt' }); core.load('{}'); core.save(document); core.grade(document);
    core.hit_test(document, 'M', [1, 2], 4); core.measure(document, 'M', { kind: 'length', object: 'side' });
    core.block_seams(document);
    const job = { format: 'beta-marker.job/v1' } as Job, result = { placements: [] } as unknown as MarkerResult;
    core.overlay(job, result);
    expect(requests.map(r => r.action)).toEqual(['blocks', 'size_chart', 'create', 'load', 'save', 'grade', 'hit_test', 'measure', 'block_seams', 'overlay']);
    expect(requests[1]).toEqual({ action: 'size_chart', preset: 'indian_adult' });
    expect(requests[2]).toEqual({ action: 'create', request: { block: 't_shirt' } });
    expect(requests[9]).toEqual({ action: 'overlay', job, result, options: {} });
  });
  it('propagates engine rejection and decodes saved JSON as text', () => {
    expect(bindStyleCore({ style_command: () => JSON.stringify('{"format":"beta-pattern.doc/v1"}') }).save(document)).toBe('{"format":"beta-pattern.doc/v1"}');
    const core = bindStyleCore({ style_command() { throw new Error('Piece path is not closed.'); } });
    expect(() => core.edit(document, [])).toThrow('Piece path is not closed');
  });
  it('passes linked offsets and line intersections through the atomic edit transport', () => {
    let request: unknown;
    const core = bindStyleCore({ style_command(raw) { request = JSON.parse(raw); return JSON.stringify(document); } });
    const operations: StyleEditOperation[] = [
      { op: 'offset_path', id: 'facing', source: 'curve', distance: 'm.neck/10', side: 'right' },
      { op: 'extend_trim', id: 'extension', source: 'side', target: 'hem', end: 1 },
    ];
    core.edit(document, operations);
    expect(request).toEqual({ action: 'edit', document, operations });
  });
  it('preserves zero curve parameters and null unchecked match axes from Rust', () => {
    const hit = { id: 'curve', kind: 'path', piece_id: 'front', edge: 0, along_mm: 0,
      parameter: 0, parameter_kind: 'native', point: [2, 3], distance_mm: 1 };
    const match = { rule: 0, garment: 0, status: 'not_placed', a_ref: { piece_id: 'front-M', point_id: 'hem' },
      b_ref: null, a: null, b: [0, 0], a_placement: null, b_placement: null,
      offset_mm: [null, null], error_mm: null, tolerance_mm: 1, pass: false };
    const core = bindStyleCore({ style_command(raw) {
      return JSON.stringify(JSON.parse(raw).action === 'hit_test' ? [hit] : { matches: [match] });
    } });
    expect(core.hit_test(document, 'M', [2, 4], 5)[0]).toEqual(hit);
    expect(core.overlay({} as Job, {} as MarkerResult).matches).toEqual([match]);
  });
  it('keeps null spec targets, grade order and binary exports through the native adapter', () => {
    const values = [{ size: 'S', measured_mm: 500, target_mm: null, grade_step_mm: null, pass: null },
      { size: 'M', measured_mm: 530, target_mm: null, grade_step_mm: 30, pass: null }];
    const file = { filename: 'measurement-spec.pdf', mime: 'application/pdf', bytes: [37, 80, 68, 70] };
    const core = bindStyleCore({ style_command(raw) {
      const request = JSON.parse(raw);
      if (request.action === 'spec_sheet') return JSON.stringify({ rows: [{ values }] });
      if (request.action === 'block_checks') return JSON.stringify({ pass: false, sizes: [{ size: 'M', pass: false, errors: ['Review topology'] }] });
      expect(request).toEqual({ action: 'export_spec_sheet', document, format: 'pdf' });
      return JSON.stringify(file);
    } });
    expect(core.spec_sheet(document).rows[0]?.values).toEqual(values);
    expect(core.block_checks(document).pass).toBe(false);
    expect(core.export_spec_sheet(document, 'pdf')).toEqual(file);
  });

});
