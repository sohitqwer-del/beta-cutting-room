import { describe, expect, it } from 'vitest';
import { demoJob, demoResult } from './demo';
import { changePlacement, emptyResult, History, readDocument } from './state';
import { fitView, markerPoint, rulerTicks, screenPoint, zoomAt } from './view';
describe('editor history and files', () => {
  it('moves immutably, protects locks, undoes and clears redo on a new edit', () => {
    const start = { job: demoJob, result: demoResult };
    const history = new History(start);
    const moved = changePlacement(history.current, 0, { translate: [20, 30] });
    history.commit(moved);
    expect(demoResult.placements[0]!.translate).toEqual([10, 10]);
    expect(history.undo()).toEqual(start);
    expect(history.redo()).toEqual(moved);
    history.undo(); history.commit(changePlacement(history.current, 0, { locked: true }));
    expect(history.canRedo).toBe(false);
    expect(changePlacement(history.current, 0, { translate: [999, 0] })).toBe(history.current);
    expect(changePlacement(history.current, 0, { locked: false }).result.placements[0]!.locked).toBe(false);
  });
  it('bounds history and ignores no-op changes', () => {
    const history = new History(0, 2); history.commit(0); expect(history.canUndo).toBe(false);
    history.commit(1); history.commit(2); history.commit(3);
    expect(history.undo()).toBe(2); expect(history.undo()).toBe(1); expect(history.undo()).toBe(1);
  });
  it('accepts wire fixtures and rejects malformed viewer geometry', () => {
    expect(readDocument(JSON.stringify(demoJob))).toEqual(demoJob);
    expect(readDocument(JSON.stringify(demoResult))).toEqual(demoResult);
    expect(emptyResult(demoJob).placements).toEqual([]);
    expect(() => readDocument('{"format":"other"}')).toThrow();
    expect(() => readDocument(JSON.stringify({ ...demoJob, fabric: { width: -1 } }))).toThrow();
    expect(() => readDocument(JSON.stringify({ ...demoJob, pieces: [{ ...demoJob.pieces[0], quantity: 'bad' }] }))).toThrow();
    expect(() => readDocument(JSON.stringify({ ...demoJob, order: { sizes: { M: 1.5 } } }))).toThrow();
    expect(() => readDocument(JSON.stringify({ ...demoResult, seed: 1e20 }))).toThrow();
  });
});
describe('view coordinates and rulers', () => {
  it('fits both dimensions at laptop aspect ratios', () => {
    const view = fitView(3000, 1500, 1.8);
    expect(view.width / view.height).toBeCloseTo(1.8);
    expect(view.x).toBeLessThan(0); expect(view.y).toBeLessThan(0);
    expect(view.x + view.width).toBeGreaterThan(3000);
    expect(view.y + view.height).toBeGreaterThan(1500);
  });
  it('keeps the cursor anchor stationary when zooming and flips y once', () => {
    const view = { x: 10, y: 20, width: 1000, height: 500 };
    const anchor = screenPoint(view, [250, 125], 1000, 500);
    const zoom = zoomAt(view, anchor, 0.5);
    expect(screenPoint(zoom, [250, 125], 1000, 500)).toEqual(anchor);
    expect(markerPoint([100, 200], 1500)).toEqual([100, 1300]);
    expect(rulerTicks(view)).toEqual([200, 400, 600, 800, 1000]);
  });
});
