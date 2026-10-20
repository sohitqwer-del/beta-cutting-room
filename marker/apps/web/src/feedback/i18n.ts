import en from './locales/en.json';
import hi from './locales/hi.json';
import ta from './locales/ta.json';

/** Feedback strings. Same pattern as the app labels: English source, Hindi and Tamil drafts marked by `_status`. */
export type FeedbackLanguage = 'en' | 'hi' | 'ta';
export type FeedbackKey = Exclude<keyof typeof en, '_status'>;
export const feedbackLanguages: Record<FeedbackLanguage, Record<string, string>> = { en, hi, ta };

/** The shell keeps the chosen language on <html data-language>. */
export function currentLanguage(): FeedbackLanguage {
  const value = typeof document === 'undefined' ? '' : document.documentElement.dataset.language;
  return value === 'hi' || value === 'ta' ? value : 'en';
}
export function t(key: FeedbackKey, language: FeedbackLanguage = currentLanguage()): string {
  return feedbackLanguages[language][key] || en[key];
}

/** Elements carry `data-fb="key"` for text and `data-fb-attr="aria-label:key;title:key"` for attributes. */
export function translateFeedback(root: HTMLElement, language: FeedbackLanguage = currentLanguage()): void {
  const all = (selector: string): HTMLElement[] => [...(root.matches(selector) ? [root] : []), ...Array.from(root.querySelectorAll<HTMLElement>(selector))];
  all('[data-fb]').forEach(element => {
    element.textContent = t(element.dataset.fb as FeedbackKey, language);
    if (language === 'en') element.removeAttribute('lang'); else element.lang = language;
  });
  all('[data-fb-attr]').forEach(element => {
    for (const pair of element.dataset.fbAttr!.split(';')) {
      const [attribute, key] = pair.split(':');
      if (attribute && key) element.setAttribute(attribute, t(key as FeedbackKey, language));
    }
  });
}
