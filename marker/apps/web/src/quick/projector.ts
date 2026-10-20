/** Projector mode maths. A CSS pixel is not a millimetre on the table, so the user measures a
 * projected square with a tape and we correct each axis. Everything here is pure and tested. */
export interface Calibration { pxPerMmX: number; pxPerMmY: number; measured: boolean }
export const SQUARE_MM = 100;
/** CSS reference pixel: 96 per inch. Only a starting guess; real projectors differ a lot. */
export const NOMINAL_PX_PER_MM = 96 / 25.4;
export const DEFAULT_CALIBRATION: Calibration = { pxPerMmX: NOMINAL_PX_PER_MM, pxPerMmY: NOMINAL_PX_PER_MM, measured: false };

/** Size of the calibration square on screen, in CSS pixels. */
export function squarePx(cal: Calibration, squareMm = SQUARE_MM): { width: number; height: number } {
  return { width: squareMm * cal.pxPerMmX, height: squareMm * cal.pxPerMmY };
}

export type MeasureProblem = 'missing' | 'too_small' | 'too_large';
/** A tape reading must be plausible: a 100 mm square shown between 20 mm and 2 m on the table. */
export function measureProblem(value: number | null, squareMm = SQUARE_MM): MeasureProblem | null {
  if (value == null || !Number.isFinite(value)) return 'missing';
  if (value < squareMm * 0.2) return 'too_small';
  if (value > squareMm * 20) return 'too_large';
  return null;
}

/** The square should be `squareMm` but measured `measuredX` × `measuredY` mm. Scale each axis so it becomes exact. */
export function correct(cal: Calibration, measuredX: number, measuredY: number, squareMm = SQUARE_MM): Calibration {
  if (measureProblem(measuredX, squareMm) || measureProblem(measuredY, squareMm)) throw new Error('Measured size is not plausible.');
  return { pxPerMmX: cal.pxPerMmX * squareMm / measuredX, pxPerMmY: cal.pxPerMmY * squareMm / measuredY, measured: true };
}

/** Error left after correction if the user measures again, in mm over the square. */
export function residual(cal: Calibration, measuredX: number, measuredY: number, squareMm = SQUARE_MM): number {
  return Math.max(Math.abs(measuredX - squareMm), Math.abs(measuredY - squareMm));
}

export interface Section { index: number; row: number; col: number; x0: number; x1: number; y0: number; y1: number }
/** Split the marker into screen-sized sections. X runs along the length from the marker start; Y across from the
 * selvedge (y = 0). Neighbouring sections overlap by `overlapMm` so the cutter can line them up. */
export function sections(lengthMm: number, widthMm: number, viewport: { width: number; height: number }, cal: Calibration, overlapMm = 50): Section[] {
  const spanX = viewport.width / cal.pxPerMmX, spanY = viewport.height / cal.pxPerMmY;
  const starts = (total: number, span: number): number[] => {
    if (total <= span) return [0];
    // Overlap is at most a tenth of the screen, so small screens are not mostly overlap.
    const stride = span - Math.min(overlapMm, span * 0.1);
    const out: number[] = [];
    for (let s = 0; s + span < total; s += stride) out.push(s);
    out.push(total - span);
    return out;
  };
  const xs = starts(lengthMm, spanX), ys = starts(widthMm, spanY);
  const out: Section[] = [];
  ys.forEach((y0, row) => xs.forEach((x0, col) => out.push({ index: out.length, row, col, x0, x1: x0 + spanX, y0, y1: y0 + spanY })));
  return out;
}

/** mm on the marker → CSS pixels on the projector screen for one section. Y is up on the marker, down on screen. */
export function toScreen(section: Section, cal: Calibration, viewportHeight: number): (p: [number, number]) => [number, number] {
  return ([x, y]) => [(x - section.x0) * cal.pxPerMmX, viewportHeight - (y - section.y0) * cal.pxPerMmY];
}

const KEY = 'beta-quick-projector';
export function loadCalibration(store: Pick<Storage, 'getItem'> | null): Calibration {
  try {
    const value = JSON.parse(store?.getItem(KEY) ?? 'null') as Calibration | null;
    if (value && value.pxPerMmX > 0.1 && value.pxPerMmX < 100 && value.pxPerMmY > 0.1 && value.pxPerMmY < 100) return { ...value, measured: !!value.measured };
  } catch { /* default */ }
  return { ...DEFAULT_CALIBRATION };
}
export function saveCalibration(store: Pick<Storage, 'setItem'> | null, cal: Calibration): void {
  try { store?.setItem(KEY, JSON.stringify(cal)); } catch { /* this session only */ }
}
