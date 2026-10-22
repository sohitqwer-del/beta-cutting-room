/** One unit system for the whole app: the user picks cm or inch once (top bar) and every screen follows.
 * Engines always work in millimetres; this only decides how lengths are shown and typed.
 * Long lengths (marker length, fabric) are shown in metres for cm and in yards for inch. */
export type AppUnit = 'cm' | 'inch';
const KEY = 'beta-unit';
export const MM_PER_INCH = 25.4, MM_PER_YARD = 914.4;

export function appUnit(): AppUnit {
  try { return localStorage.getItem(KEY) === 'inch' ? 'inch' : 'cm'; } catch { return 'cm'; }
}
export function setAppUnit(unit: AppUnit): void {
  try { localStorage.setItem(KEY, unit); } catch { /* this session only */ }
  if (typeof document !== 'undefined') document.documentElement.dataset.unit = unit;
  if (typeof window !== 'undefined') window.dispatchEvent(new CustomEvent<AppUnit>('beta-unit', { detail: unit }));
}
export function onUnit(listener: (unit: AppUnit) => void): void {
  window.addEventListener('beta-unit', event => listener((event as CustomEvent<AppUnit>).detail));
}

/** Short lengths (widths, gaps, allowances): cm with one decimal, or inch with up to two decimals. */
export function shortLength(mm: number, unit: AppUnit = appUnit(), withUnit = true): string {
  const value = unit === 'inch' ? mm / MM_PER_INCH : mm / 10;
  const rounded = Number(value.toFixed(unit === 'inch' ? 2 : 1));
  const text = Object.is(rounded, -0) ? '0' : String(rounded);
  return withUnit ? `${text} ${unit === 'inch' ? 'in' : 'cm'}` : text;
}
/** Long lengths (marker length, fabric per garment, total fabric): metres for cm, yards for inch. */
export function longLength(mm: number, unit: AppUnit = appUnit(), digits = 3): { value: string; unit: string } {
  return unit === 'inch' ? { value: (mm / MM_PER_YARD).toFixed(digits), unit: 'yd' } : { value: (mm / 1000).toFixed(digits), unit: 'm' };
}
export const shortUnit = (unit: AppUnit = appUnit()): string => (unit === 'inch' ? 'in' : 'cm');
export function toMm(value: number, unit: AppUnit = appUnit()): number { return unit === 'inch' ? value * MM_PER_INCH : value * 10; }
export function fromMm(mm: number, unit: AppUnit = appUnit()): number { return unit === 'inch' ? mm / MM_PER_INCH : mm / 10; }
/** A typed short length in the app unit; accepts a comma decimal mark. Null for empty or nonsense. */
export function parseShort(text: string, unit: AppUnit = appUnit(), allowNegative = false): number | null {
  const clean = text.trim().replace(',', '.').replace(/\s*(cm|in|inch|")\s*$/i, '');
  if (!clean) return null;
  const value = Number(clean);
  if (!Number.isFinite(value) || (!allowNegative && value < 0)) return null;
  return toMm(value, unit);
}
