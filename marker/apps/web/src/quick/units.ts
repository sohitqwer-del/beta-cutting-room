/** Units for the Quick Marker screens. The engine always works in millimetres. */
export type WidthUnit = 'cm' | 'inch';
export const MM_PER_INCH = 25.4;

/** Common fabric widths in Indian shops. Inch widths are how cloth is sold; cm for mills that mark cm. */
export const WIDTH_PRESETS: Record<WidthUnit, number[]> = { inch: [36, 44, 58, 60], cm: [90, 112, 150, 160] };

export function toMm(value: number, unit: WidthUnit): number {
  return unit === 'inch' ? value * MM_PER_INCH : value * 10;
}
export function fromMm(mm: number, unit: WidthUnit): number {
  return unit === 'inch' ? mm / MM_PER_INCH : mm / 10;
}
/** Show a width in the chosen unit: whole numbers stay whole, otherwise one decimal. */
export function formatWidth(mm: number, unit: WidthUnit): string {
  const value = fromMm(mm, unit);
  const rounded = Math.round(value * 10) / 10;
  return Number.isInteger(rounded) ? String(rounded) : rounded.toFixed(1);
}
/** Parse what the user typed. Accepts a comma as the decimal mark. Returns null for empty, zero or nonsense. */
export function parseNumber(text: string): number | null {
  const value = Number(text.trim().replace(',', '.'));
  return Number.isFinite(value) && value > 0 ? value : null;
}
/** Fabric width must fit real looms and the engine: 20 cm to 4 m. */
export function widthProblem(mm: number | null): 'missing' | 'too_small' | 'too_large' | null {
  if (mm == null) return 'missing';
  if (mm < 200) return 'too_small';
  if (mm > 4000) return 'too_large';
  return null;
}

export function metres(mm: number, digits = 2): string { return (mm / 1000).toFixed(digits); }
export function percent(fraction: number, digits = 1): string { return (fraction * 100).toFixed(digits); }

/** Metres of fabric per garment for one marker. Folded and tube fabric still use the marker length once. */
export function metresPerGarment(lengthMm: number, garments: number): number | null {
  return garments > 0 && lengthMm > 0 ? lengthMm / 1000 / garments : null;
}

export interface Saving { savedM: number; savedPct: number; better: boolean }
/** Compare with the user's usual fabric per garment (metres). Positive savedM means BETA uses less. */
export function savingVsUsual(perGarmentM: number | null, usualM: number | null): Saving | null {
  if (perGarmentM == null || usualM == null || usualM <= 0) return null;
  const savedM = usualM - perGarmentM;
  return { savedM, savedPct: savedM / usualM, better: savedM > 0.0005 };
}

/** Steppers: keep counts whole and inside limits. */
export function step(value: number, delta: number, min = 0, max = 999): number {
  const next = Math.round((Number.isFinite(value) ? value : min) + delta);
  return Math.min(max, Math.max(min, next));
}
