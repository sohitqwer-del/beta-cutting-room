import en from './locales/en.json';
import hi from './locales/hi.json';
import ta from './locales/ta.json';

/** Style flow strings. English is the source; Hindi and Tamil are drafts marked by `_status` for native review.
 * The app keeps the chosen language on <html data-language> (set by the language switch). */
export type StyleLanguage = 'en' | 'hi' | 'ta';
export type StyleKey = Exclude<keyof typeof en, '_status'>;
export const styleLanguages: Record<StyleLanguage, Record<string, string>> = { en, hi, ta };

export function currentLanguage(): StyleLanguage {
  const value = typeof document === 'undefined' ? '' : document.documentElement.dataset.language;
  return value === 'hi' || value === 'ta' ? value : 'en';
}
export function has(key: string): key is StyleKey { return key !== '_status' && key in en; }
/** Translate and fill {placeholders}. Missing draft strings fall back to English. */
export function t(key: StyleKey, values: Record<string, string | number> = {}, language: StyleLanguage = currentLanguage()): string {
  const text = styleLanguages[language][key] || en[key] || key;
  return text.replace(/\{(\w+)\}/g, (_, name: string) => String(values[name] ?? `{${name}}`));
}
