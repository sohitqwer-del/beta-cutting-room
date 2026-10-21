/** Units for the Style screens. The engine always works in millimetres; people work in cm or inch. */
export type LengthUnit = 'cm' | 'inch';
export const MM_PER_INCH = 25.4;

export function toMm(value: number, unit: LengthUnit): number {
  return unit === 'inch' ? value * MM_PER_INCH : value * 10;
}
export function fromMm(mm: number, unit: LengthUnit): number {
  return unit === 'inch' ? mm / MM_PER_INCH : mm / 10;
}
/** Show a length in the chosen unit. cm: one decimal (0.1 cm = 1 mm). inch: up to two decimals (~0.25 mm). */
export function formatLength(mm: number, unit: LengthUnit, withUnit = false): string {
  const value = fromMm(mm, unit);
  const digits = unit === 'inch' ? 2 : 1;
  const rounded = Number(value.toFixed(digits));
  const text = Object.is(rounded, -0) ? '0' : String(rounded);
  return withUnit ? `${text} ${unit === 'inch' ? 'in' : 'cm'}` : text;
}
/** Parse a typed length. Accepts a comma decimal mark and simple inch fractions ("1 1/2", "3/8").
 * Returns millimetres, or null for empty or nonsense. Negative values are allowed only when asked for. */
export function parseLength(text: string, unit: LengthUnit, allowNegative = false): number | null {
  const clean = text.trim().replace(',', '.').replace(/\s*(cm|in|inch|")\s*$/i, '');
  if (!clean) return null;
  let value: number;
  const fraction = /^(-?)(?:(\d+)\s+)?(\d+)\/(\d+)$/.exec(clean);
  if (fraction) {
    const [, sign, whole, top, bottom] = fraction;
    if (Number(bottom) === 0) return null;
    value = (Number(whole ?? 0) + Number(top) / Number(bottom)) * (sign ? -1 : 1);
  } else value = Number(clean);
  if (!Number.isFinite(value) || (!allowNegative && value < 0)) return null;
  return toMm(value, unit);
}
/** Angles: the engine uses degrees counter-clockwise from +x with y up. Keep them in (-180, 180]. */
export function normaliseAngle(deg: number): number {
  let a = deg % 360;
  if (a <= -180) a += 360;
  if (a > 180) a -= 360;
  return Object.is(a, -0) ? 0 : a;
}
export function formatAngle(deg: number): string { return `${Number(normaliseAngle(deg).toFixed(1))}°`; }

/** Fabric widths are sold in inch in India; mills may mark cm. Same presets as the photo flow. */
export const WIDTH_PRESETS: Record<LengthUnit, number[]> = { inch: [36, 44, 58, 60], cm: [90, 112, 150, 160] };
/** Fabric width must fit real looms and the nester: 20 cm to 4 m. */
export function widthProblem(mm: number | null): 'missing' | 'too_small' | 'too_large' | null {
  if (mm == null || !Number.isFinite(mm)) return 'missing';
  if (mm < 200) return 'too_small';
  if (mm > 4000) return 'too_large';
  return null;
}
export function metres(mm: number, digits = 2): string { return (mm / 1000).toFixed(digits); }
export function percent(fraction: number, digits = 1): string { return (fraction * 100).toFixed(digits); }
/** Steppers keep counts whole and inside limits. */
export function step(value: number, delta: number, min = 0, max = 999): number {
  const next = Math.round((Number.isFinite(value) ? value : min) + delta);
  return Math.min(max, Math.max(min, next));
}
