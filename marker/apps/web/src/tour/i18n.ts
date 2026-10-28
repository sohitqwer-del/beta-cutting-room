import en from './locales/en.json';
import hi from './locales/hi.json';
import ta from './locales/ta.json';

/** Tour strings. English is the source; Hindi and Tamil are drafts marked by `_status` for native review.
 * The app keeps the chosen language on <html data-language> (set by the language switch). */
export type TourLanguage = 'en' | 'hi' | 'ta';
export type TourKey = Exclude<keyof typeof en, '_status'>;
export const tourLanguages: Record<TourLanguage, Record<string, string>> = { en, hi, ta };

export function currentLanguage(): TourLanguage {
  const value = typeof document === 'undefined' ? '' : document.documentElement.dataset.language;
  return value === 'hi' || value === 'ta' ? value : 'en';
}
/** Translate and fill {placeholders}. Missing draft strings fall back to English. */
export function t(key: TourKey, values: Record<string, string | number> = {}, language: TourLanguage = currentLanguage()): string {
  const text = tourLanguages[language][key] || en[key] || key;
  return text.replace(/\{(\w+)\}/g, (_, name: string) => String(values[name] ?? `{${name}}`));
}
