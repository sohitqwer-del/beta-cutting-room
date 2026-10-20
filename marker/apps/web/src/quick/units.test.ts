import { describe, expect, it } from 'vitest';
import { formatWidth, fromMm, metresPerGarment, parseNumber, savingVsUsual, step, toMm, widthProblem, WIDTH_PRESETS } from './units';

describe('quick units', () => {
  it('converts inch and cm widths to mm and back', () => {
    expect(toMm(44, 'inch')).toBeCloseTo(1117.6, 6);
    expect(toMm(150, 'cm')).toBe(1500);
    expect(fromMm(toMm(58, 'inch'), 'inch')).toBeCloseTo(58, 9);
    expect(formatWidth(1117.6, 'inch')).toBe('44');
    expect(formatWidth(1117.6, 'cm')).toBe('111.8');
    expect(formatWidth(1500, 'cm')).toBe('150');
  });
  it('has the common presets', () => {
    expect(WIDTH_PRESETS.inch).toEqual([36, 44, 58, 60]);
    for (const p of WIDTH_PRESETS.cm) expect(widthProblem(toMm(p, 'cm'))).toBeNull();
  });
  it('parses typed numbers with a decimal comma and rejects empty or zero', () => {
    expect(parseNumber('44,5')).toBe(44.5);
    expect(parseNumber(' 58 ')).toBe(58);
    expect(parseNumber('')).toBeNull();
    expect(parseNumber('0')).toBeNull();
    expect(parseNumber('abc')).toBeNull();
  });
  it('checks plausible widths', () => {
    expect(widthProblem(null)).toBe('missing');
    expect(widthProblem(150)).toBe('too_small');
    expect(widthProblem(5000)).toBe('too_large');
  });
  it('computes metres per garment and saving vs the usual length', () => {
    expect(metresPerGarment(3200, 2)).toBeCloseTo(1.6);
    expect(metresPerGarment(3200, 0)).toBeNull();
    const s = savingVsUsual(1.6, 1.8)!;
    expect(s.savedM).toBeCloseTo(0.2); expect(s.savedPct).toBeCloseTo(0.1111, 3); expect(s.better).toBe(true);
    expect(savingVsUsual(1.9, 1.8)!.better).toBe(false);
    expect(savingVsUsual(1.6, null)).toBeNull();
  });
  it('keeps stepper counts whole and in range', () => {
    expect(step(1, 1)).toBe(2);
    expect(step(0, -1)).toBe(0);
    expect(step(1, -1, 1)).toBe(1);
    expect(step(99, 1, 0, 99)).toBe(99);
    expect(step(Number.NaN, 1)).toBe(1);
  });
});
