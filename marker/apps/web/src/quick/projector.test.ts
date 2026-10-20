import { describe, expect, it } from 'vitest';
import { correct, DEFAULT_CALIBRATION, loadCalibration, measureProblem, NOMINAL_PX_PER_MM, residual, saveCalibration, sections, squarePx, toScreen, type Calibration } from './projector';
import { applyHomography, area, invertHomography, mmToOverlay, pathData, turnLine } from './geometry';
import { SAMPLE_IMAGE_TO_MM, SAMPLE_SIZE } from './mock';

describe('projector calibration', () => {
  it('starts from the CSS reference pixel', () => {
    expect(squarePx(DEFAULT_CALIBRATION).width).toBeCloseTo(100 * 96 / 25.4, 6);
    expect(DEFAULT_CALIBRATION.measured).toBe(false);
  });
  it('corrects each axis from the measured square', () => {
    // Projector shows the 100 mm square as 250 mm wide and 200 mm high: scale must shrink 2.5× and 2×.
    const cal = correct(DEFAULT_CALIBRATION, 250, 200);
    expect(cal.pxPerMmX).toBeCloseTo(NOMINAL_PX_PER_MM / 2.5, 9);
    expect(cal.pxPerMmY).toBeCloseTo(NOMINAL_PX_PER_MM / 2, 9);
    expect(cal.measured).toBe(true);
    // Projecting again now gives exactly 100 mm on the table (projector magnification is linear).
    const magX = 250 / squarePx(DEFAULT_CALIBRATION).width, magY = 200 / squarePx(DEFAULT_CALIBRATION).height;
    const sq = squarePx(cal);
    expect(sq.width * magX).toBeCloseTo(100, 6);
    expect(sq.height * magY).toBeCloseTo(100, 6);
  });
  it('refines a second measurement and reports what is left', () => {
    const first = correct(DEFAULT_CALIBRATION, 101, 99);
    const second = correct(first, 100.5, 100);
    expect(second.pxPerMmX).toBeCloseTo(first.pxPerMmX * 100 / 100.5, 9);
    expect(residual(second, 100.5, 99.2)).toBeCloseTo(0.8, 9);
  });
  it('rejects readings that cannot be right', () => {
    expect(measureProblem(null)).toBe('missing');
    expect(measureProblem(5)).toBe('too_small');
    expect(measureProblem(3000)).toBe('too_large');
    expect(() => correct(DEFAULT_CALIBRATION, 0, 100)).toThrow();
  });
  it('stores the calibration per device and ignores bad data', () => {
    const map = new Map<string, string>();
    const store = { getItem: (k: string) => map.get(k) ?? null, setItem: (k: string, v: string) => void map.set(k, v) };
    const cal: Calibration = { pxPerMmX: 2.1, pxPerMmY: 2.2, measured: true };
    saveCalibration(store, cal);
    expect(loadCalibration(store)).toEqual(cal);
    map.set('beta-quick-projector', '{"pxPerMmX":-1}');
    expect(loadCalibration(store)).toEqual(DEFAULT_CALIBRATION);
    expect(loadCalibration(null)).toEqual(DEFAULT_CALIBRATION);
  });
});

describe('projector sections', () => {
  const cal: Calibration = { pxPerMmX: 1, pxPerMmY: 1, measured: true };
  it('uses one section when the marker fits', () => {
    expect(sections(900, 600, { width: 1000, height: 800 }, cal)).toHaveLength(1);
  });
  it('covers the whole length with overlapping sections that end at the marker end', () => {
    const s = sections(2500, 600, { width: 1000, height: 800 }, cal, 50);
    expect(s.map(x => x.x0)).toEqual([0, 950, 1500]);
    expect(s.at(-1)!.x1).toBe(2500);
    for (let k = 1; k < s.length; k++) expect(s[k - 1]!.x1 - s[k]!.x0).toBeGreaterThanOrEqual(50);
  });
  it('adds rows when the fabric is wider than the screen', () => {
    const s = sections(1500, 1500, { width: 1000, height: 800 }, cal, 50);
    expect(new Set(s.map(x => x.row)).size).toBe(2);
    expect(Math.max(...s.map(x => x.y1))).toBe(1500);
  });
  it('maps marker mm to screen pixels with the selvedge at the bottom', () => {
    const [sec] = sections(2500, 600, { width: 1000, height: 800 }, { pxPerMmX: 2, pxPerMmY: 2, measured: true });
    const map = toScreen(sec!, { pxPerMmX: 2, pxPerMmY: 2, measured: true }, 800);
    expect(map([0, 0])).toEqual([0, 800]);
    expect(map([100, 50])).toEqual([200, 700]);
  });
});

describe('quick geometry', () => {
  it('inverts the photo map so outlines land on the overlay', () => {
    const toPx = mmToOverlay(SAMPLE_IMAGE_TO_MM, SAMPLE_SIZE, [800, 600])!;
    // mm (0,0) is pixel (120,1100) in the 1600 × 1200 photo, so (60,550) on a half-size overlay.
    const [x, y] = toPx([0, 0]);
    expect(x).toBeCloseTo(60, 6); expect(y).toBeCloseTo(550, 6);
    const h = [1.2, 0.1, 5, -0.05, 0.9, 3, 0.0001, 0.0002, 1];
    const inv = invertHomography(h)!;
    const p = applyHomography(inv, applyHomography(h, [123, 456]));
    expect(p[0]).toBeCloseTo(123, 6); expect(p[1]).toBeCloseTo(456, 6);
    expect(invertHomography([0, 0, 0, 0, 0, 0, 0, 0, 0])).toBeNull();
  });
  it('measures area, draws paths and turns grain lines', () => {
    expect(area([[0, 0], [100, 0], [100, 50], [0, 50]])).toBe(5000);
    expect(pathData([[0, 0], [10, 5]])).toBe('M0.0 0.0L10.0 -5.0Z');
    const [a, b] = turnLine([[0, 0], [100, 0]]);
    expect(a[0]).toBeCloseTo(50); expect(b[0]).toBeCloseTo(50);
    expect(Math.abs(a[1] - b[1])).toBeCloseTo(100);
  });
});
