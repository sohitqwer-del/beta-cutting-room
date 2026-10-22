import { describe, expect, it } from 'vitest';
import { appUnit, fromMm, longLength, parseShort, shortLength, shortUnit, toMm } from './units-pref';

describe('one unit for the app', () => {
  it('starts in cm when nothing is saved', () => { expect(appUnit()).toBe('cm'); });
  it('shows short lengths in cm or inch', () => {
    expect(shortLength(1524, 'cm')).toBe('152.4 cm');
    expect(shortLength(1524, 'inch')).toBe('60 in');
    expect(shortLength(5, 'inch', false)).toBe('0.2');
    expect(shortLength(-0.01, 'cm')).toBe('0 cm');
    expect(shortUnit('inch')).toBe('in');
  });
  it('shows long lengths in metres or yards', () => {
    expect(longLength(2825, 'cm')).toEqual({ value: '2.825', unit: 'm' });
    expect(longLength(914.4 * 3, 'inch', 2)).toEqual({ value: '3.00', unit: 'yd' });
  });
  it('reads typed lengths in the chosen unit', () => {
    expect(parseShort('152,4', 'cm')).toBeCloseTo(1524, 6);
    expect(parseShort('60 in', 'inch')).toBeCloseTo(1524, 6);
    expect(parseShort('-1', 'cm')).toBeNull();
    expect(parseShort('-1', 'cm', true)).toBe(-10);
    expect(parseShort('abc', 'cm')).toBeNull();
    expect(fromMm(toMm(2.5, 'inch'), 'inch')).toBeCloseTo(2.5, 9);
  });
});
