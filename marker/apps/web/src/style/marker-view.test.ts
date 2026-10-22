import { describe, expect, it } from 'vitest';
import type { Job, MarkerResult, Pt } from '@beta/marker-wasm';
import { boxText, fitMarker, mmPerPx, panBox, pieceFacts, zoomBox } from './marker-view';

const rect = (x: number, y: number, w: number, h: number): Pt[] => [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];

describe('marker view', () => {
  it('fits the whole marker with a margin and keeps the screen shape', () => {
    const b = fitMarker(4000, 1500, 1200, 400, 12);
    expect(b.w / b.h).toBeCloseTo(1200 / 400, 6);
    expect(b.x).toBeLessThan(0); expect(b.x + b.w).toBeGreaterThan(4000);
    expect(b.y).toBeLessThanOrEqual(0); expect(b.y + b.h).toBeGreaterThanOrEqual(1500);
    const u = mmPerPx(b, 1200);
    expect(Math.min(-b.x, b.x + b.w - 4000) / u).toBeGreaterThanOrEqual(11.9);
  });
  it('keeps a free strip under the marker for the controls', () => {
    const b = fitMarker(3000, 1500, 1000, 400, 12, 40), u = mmPerPx(b, 1000);
    // The near selvedge (svg y = fabric width) sits above the bottom strip.
    expect((1500 - b.y) / u).toBeLessThanOrEqual(400 - 40 - 12 + 0.5);
    expect(-b.y / u).toBeGreaterThanOrEqual(11.5);
  });
  it('zooms around the pointer and pans by screen pixels', () => {
    const b = fitMarker(4000, 1500, 1000, 500), at: Pt = [1000, 700];
    const z = zoomBox(b, 2, at, 1000);
    expect(z.w).toBeCloseTo(b.w / 2, 6);
    // The point under the pointer stays at the same place on screen.
    expect((at[0] - z.x) / z.w).toBeCloseTo((at[0] - b.x) / b.w, 9);
    expect((at[1] - z.y) / z.h).toBeCloseTo((at[1] - b.y) / b.h, 9);
    const p = panBox(z, 100, -50, 1000), u = mmPerPx(z, 1000);
    expect(p.x).toBeCloseTo(z.x - 100 * u, 6); expect(p.y).toBeCloseTo(z.y + 50 * u, 6);
    expect(boxText(p).split(' ')).toHaveLength(4);
  });
  it('stops zooming at sensible limits', () => {
    let b = fitMarker(4000, 1500, 1000, 500);
    for (let i = 0; i < 60; i++) b = zoomBox(b, 2, [2000, 750], 1000);
    expect(mmPerPx(b, 1000)).toBeCloseTo(0.05, 6);
  });
});

describe('piece details', () => {
  const job = { format: 'beta-marker.job/v1', name: 'j', fabric: { width: 1500 }, pieces: [{ id: 'front', name: 'Front', size: 'M', outline: rect(0, 0, 10, 10), quantity: 1 }] } as unknown as Job;
  const result = { placements: [{ piece_id: 'front', copy: 0, rotation_deg: 180, mirrored: true, translate: [0, 0], bundle: 2 }, { piece_id: 'front', copy: 1, rotation_deg: 0, mirrored: false, translate: [0, 0], bundle: null }] } as unknown as MarkerResult;
  it('counts copies and garments from 1 and uses the placement the engine names', () => {
    expect(pieceFacts(job, result, 'front', 0)).toMatchObject({ name: 'Front', size: 'M', copy: 1, bundle: 3, rotation: 180, mirrored: true });
    expect(pieceFacts(job, result, 'front', 1)).toMatchObject({ copy: 2, bundle: null, mirrored: false });
    expect(pieceFacts(job, result, 'gone').name).toBe('gone');
  });
});

describe('readable fit for long markers', () => {
  it('shows a short marker whole and starts a long one readable', async () => {
    const { readableMarker } = await import('./marker-view');
    expect(readableMarker(3000, 1500, 1200, 400, 12, 40).whole).toBe(true);
    const long = readableMarker(33000, 740, 1200, 400, 12, 40);
    expect(long.whole).toBe(false);
    // The fabric fills 45% of the drawing height and the view starts at the marker start.
    expect(740 / mmPerPx(long.box, 1200)).toBeCloseTo(0.45 * 400, 3);
    expect(long.box.x).toBeLessThanOrEqual(0); expect(long.box.x).toBeGreaterThan(-1000);
  });
});
