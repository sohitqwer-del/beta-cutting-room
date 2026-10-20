import en from './locales/en.json';
import hi from './locales/hi.json';
import ta from './locales/ta.json';

/** Quick Marker strings. English is the source; Hindi and Tamil are drafts marked by `_status` for native review.
 * The app keeps the chosen language on <html data-language> (set by the language switch). */
export type QuickLanguage = 'en' | 'hi' | 'ta';
export type QuickKey = Exclude<keyof typeof en, '_status'>;
export const quickLanguages: Record<QuickLanguage, Record<string, string>> = { en, hi, ta };

export function currentLanguage(): QuickLanguage {
  const value = typeof document === 'undefined' ? '' : document.documentElement.dataset.language;
  return value === 'hi' || value === 'ta' ? value : 'en';
}
/** Translate and fill {placeholders}. Missing draft strings fall back to English. */
export function t(key: QuickKey, values: Record<string, string | number> = {}, language: QuickLanguage = currentLanguage()): string {
  const text = quickLanguages[language][key] || en[key] || key;
  return text.replace(/\{(\w+)\}/g, (_, name: string) => String(values[name] ?? `{${name}}`));
}
/** Piece names from the engine's shape classifier are English words; show them in the user's language when known. */
export function pieceName(name: string, language: QuickLanguage = currentLanguage()): string {
  const key = `piece_${name.trim().toLowerCase()}` as QuickKey;
  return key in en ? t(key, {}, language) : name.charAt(0).toUpperCase() + name.slice(1);
}
export function sizeName(size: string, language: QuickLanguage = currentLanguage()): string {
  return size === 'FREE' ? t('size_FREE', {}, language) : size;
}
/** Plain words and a fix for an engine warning code. Unknown codes keep the engine's English message. */
export function warningText(code: string, message: string, language: QuickLanguage = currentLanguage()): { title: string; fix: string } {
  const key = `w_${code}` as QuickKey;
  if (key in en) return { title: t(key, {}, language), fix: t(`w_${code}_fix` as QuickKey, {}, language) };
  return { title: message, fix: t('w_other_fix', {}, language) };
}
