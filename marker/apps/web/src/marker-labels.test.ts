import { afterEach, describe, expect, it } from 'vitest';
import type { Job, MarkerExportCore, MarkerResult, Pt } from '@beta/marker-wasm';
import { labelsSvg, placedLabels, useLabelEngine } from './marker-labels';

const rect = (x: number, y: number, w: number, h: number): Pt[] => [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
const job = { format: 'beta-marker.job/v1', name: 'j', fabric: { width: 1500 }, pieces: [
  { id: 'front', name: 'Front', size: 'M', outline: rect(0, 0, 600, 400), quantity: 1 },
  { id: 'rib', name: 'Neck rib', size: 'M', outline: rect(0, 0, 30, 20), quantity: 1 },
] } as unknown as Job;
const result = { placements: [{ piece_id: 'front', copy: 0, rotation_deg: 0, mirrored: false, translate: [0, 0] }, { piece_id: 'rib', copy: 0, rotation_deg: 0, mirrored: false, translate: [700, 0] }] } as unknown as MarkerResult;
const outlines = [rect(0, 0, 600, 400), rect(700, 0, 30, 20)];
const engine = (clearance: number[]): MarkerExportCore => ({
  exportMarker: () => new Uint8Array(),
  labels: () => [
    { placement: 0, piece_id: 'front', name: 'Front', size: 'M', anchor: [300, 200], rotation_deg: 0, clearance_mm: clearance[0]! },
    { placement: 1, piece_id: 'rib', name: 'Neck rib', size: 'M', anchor: [715, 10], rotation_deg: 90, clearance_mm: clearance[1]! },
  ],
});
afterEach(() => useLabelEngine(null));

describe('marker labels', () => {
  it('without the engine: fits names inside the outlines and leaves tiny pieces bare', () => {
    const labels = placedLabels(job, result, 0.5, outlines, () => 'M');
    expect(labels.map(l => l.placement)).toEqual([0]);
    expect(labels[0]).toMatchObject({ name: 'Front', showDetail: true, rotation: 0 });
  });
  it('with the engine: uses its anchors and angles, sizes text from the clearance', () => {
    useLabelEngine(engine([200, 10]));
    const far = placedLabels(job, { ...result } as MarkerResult, 0.5, outlines, () => 'M');
    expect(far.map(l => l.placement)).toEqual([0]);
    expect(far[0]!.at).toEqual([300, 200]);
    // Zoomed in, the rib gets its name along the engine's angle.
    const near = placedLabels(job, { ...result } as MarkerResult, 6, outlines, () => 'M');
    const rib = near.find(l => l.placement === 1)!;
    expect(rib.rotation).toBe(90); expect(rib.name.startsWith('Ne')).toBe(true);
  });
  it('draws with y down and turns engine angles for the screen', () => {
    useLabelEngine(engine([200, 10]));
    const svg = labelsSvg(placedLabels(job, { ...result } as MarkerResult, 6, outlines, () => 'M'), 1500, 1 / 6, s => s);
    expect(svg).toContain('y="1300.0"'); // 1500 - 200
    expect(svg).toContain('rotate(-90.0 715.0 1490.0)');
    // Font size in user units = screen px × mm per px (the same size on screen at any zoom).
    const far = labelsSvg(placedLabels(job, { ...result } as MarkerResult, 0.5, outlines, () => 'M'), 1500, 2, s => s);
    expect(Number(/font-size="([\d.]+)"/.exec(far)![1]) / 2).toBeCloseTo(Number(/font-size="([\d.]+)"/.exec(svg)![1]) * 6, 0);
  });
  it('falls back when the engine refuses the job', () => {
    useLabelEngine({ exportMarker: () => new Uint8Array(), labels: () => { throw new Error('unknown piece'); } });
    expect(placedLabels(job, { ...result } as MarkerResult, 0.5, outlines, () => 'M').map(l => l.placement)).toEqual([0]);
  });
});
