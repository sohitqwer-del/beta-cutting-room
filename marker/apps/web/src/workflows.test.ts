import { describe, expect, it } from 'vitest';
import { demoJob, demoResult } from './demo';
import { readDocument } from './state';
import { clampDefectPoint, compareLengths, defectZone, nestPayload, planErrors, planPayload, rectangleDefect, type PlanForm } from './workflows';
import contract from '../../../packages/marker-wasm/test/fixtures/contract.json';

describe('v0.2 operator workflows', () => {
  it('re-nests around exact locked placements and sends portfolio / warm start', () => {
    const state = structuredClone({ job: demoJob, result: demoResult });
    state.result.placements[0]!.locked = true;
    state.result.placements[0]!.bundle = 4;
    state.result.placements[0]!.on_fold = true;
    const payload = nestPayload(state, { time_limit_s: 0.1, workers: 2, runs: 3 }, true, true);
    expect(payload.options.fixed).toEqual([state.result.placements[0]]);
    expect(payload.options.warm_start).toEqual(state.result);
    expect(payload.options.runs).toBe(3);
    payload.options.fixed![0]!.translate[0] = 999;
    expect(state.result.placements[0]!.translate[0]).not.toBe(999);
    expect(nestPayload(state, { time_limit_s: 1 }).options).toMatchObject({ fixed: [], warm_start: null });
  });
  const form: PlanForm = { quantities: { M: 120, L: 40 }, maxPlies: 40, maxGarments: 4, overcut: 2, time: 0.1, endAllowance: 10, layCost: 200 };
  it('validates a cut order and sends only the Rust planner contract', () => {
    expect(planErrors(form)).toEqual([]);
    const payload = planPayload(demoJob, form);
    expect(payload.order.quantities).toEqual({ M: 120, L: 40 });
    expect(payload.order.max_plies).toBe(40);
    expect(payload.options).toEqual({ time_per_marker_s: 0.1, final_time_s: 0, end_allowance_mm: 10 });
    expect(payload.options).not.toHaveProperty('layCost');
    expect(planPayload(demoJob, {...form,finalTime:3}).options.final_time_s).toBe(3);
    for (const patch of [{ quantities: { M: 0 } }, { quantities: { M: 1.5 } }, { maxPlies: 0 }, { maxGarments: -1 }, { overcut: -1 }, { time: NaN }, { finalTime: -1 }, { finalTime: Infinity }, { endAllowance: -1 }, { layCost: Infinity }]) {
      expect(() => planPayload(demoJob, { ...form, ...patch })).toThrow();
    }
  });
  it('compares lengths in mm, preserving negative savings and guarding zero', () => {
    expect(compareLengths(5000, 4000)?.metres).toBe(1);
    expect(compareLengths(5000, 4000)?.percent).toBeCloseTo(20);
    expect(compareLengths(4000, 5000)).toEqual({ metres: -1, percent: -25 });
    expect(compareLengths(5000, 5000)).toEqual({ metres: 0, percent: 0 });
    for (const pair of [[0, 2], [2, 0], [NaN, 2], [2, Infinity], [-1, 2]]) expect(compareLengths(pair[0]!, pair[1]!)).toBeNull();
  });
  it('draws rectangles in either direction, clamps fabric bounds and rejects incomplete zones', () => {
    expect(rectangleDefect([300, 200], [100, 50], 250, 150)).toEqual([[100, 50], [250, 50], [250, 150], [100, 150]]);
    expect(clampDefectPoint([-10, 600], 500, 300)).toEqual([0, 300]);
    expect(rectangleDefect([10, 10], [10, 20], 100, 100)).toEqual([]);
    const points: [number, number][] = [[0, 0], [10, 0], [5, 10]];
    const zone = defectZone(points, 'flaw'); points[0]![0] = 999;
    expect(zone.polygon[0]).toEqual([0, 0]);
    expect(() => defectZone([[0, 0], [1, 1]], 'bad')).toThrow();
    expect(() => defectZone([[0, 0], [1, 1], [NaN, 2]], 'bad')).toThrow();
    expect(() => defectZone([[0, 0], [0, 0], [1, 1]], 'bad')).toThrow();
  });
  it('loads all v0.2 fields and rejects malformed new drawing geometry', () => {
    expect(readDocument(JSON.stringify(contract.v02.job))).toEqual(contract.v02.job);
    expect(readDocument(JSON.stringify(contract.v02.result))).toEqual(contract.v02.result);
    const job = structuredClone(contract.v02.job); job.fabric.defects[0]!.polygon = [[1, 2]];
    expect(() => readDocument(JSON.stringify(job))).toThrow();
  });
});
