import { describe, expect, it } from 'vitest';
import { minRefinements, readRatios, trialOrder, trialReason } from './cost-comparison';
describe('operator ratio menus', () => {
  it('preserves exact size names and multiple menus without computing a plan', () => {
    expect(readRatios('S=1,M=2\nM=1,L=2')).toEqual([{S:1,M:2},{M:1,L:2}]);
    expect(readRatios('')).toEqual([]);
  });
  it('rejects duplicate, fractional, missing, zero and overflowing quantities', () => {
    for (const value of ['M=1,M=2','M=1.5','M=','M=0','M=4294967296','M=1=2']) expect(() => readRatios(value)).toThrow();
  });
  it('does not turn special size names into object prototype changes', () => {
    const ratio = readRatios('__proto__=1,constructor=2')[0]!;
    expect(Object.getPrototypeOf(ratio)).toBeNull();
    expect(Object.keys(ratio)).toEqual(['__proto__','constructor']);
  });
});

describe('cost comparison screen helpers', () => {
  it('asks for at least one measured ratio per ordered size', () => {
    expect(minRefinements({ M: 4, L: 4 })).toBe(2);
    expect(minRefinements({ S: 0, M: 4 })).toBe(1);
    expect(minRefinements({})).toBe(1);
    expect(minRefinements(Object.fromEntries(Array.from({ length: 20 }, (_, i) => [`S${i}`, 1])))).toBe(16);
  });
  it('explains an impossible width as a width problem when other widths planned the same order', () => {
    const others = [{ width_mm: 1500, feasible: true }, { width_mm: 10, feasible: false }];
    expect(trialReason('Order quantities are incomplete', 10, others)).toMatch(/No marker fits this width/);
    expect(trialReason('Order quantities are incomplete', 10, [{ width_mm: 10, feasible: false }])).toBeNull();
    expect(trialReason('Stock shortage for roll R1', 10, others)).toBeNull();
    expect(trialReason(null, 1500, others)).toBeNull();
  });
  it('lists the recommended choice first, then checked, short and failed ones by total', () => {
    const t = (feasible: boolean, final_evaluated: boolean, total: number | null) => ({ feasible, final_evaluated, cost: total == null ? null : { total } });
    expect(trialOrder([t(false, false, null), t(true, true, 900), t(true, false, 700), t(true, true, 800)], 3)).toEqual([3, 1, 2, 0]);
    expect(trialOrder([t(true, true, 900), t(true, true, 800)], null)).toEqual([1, 0]);
  });
});
