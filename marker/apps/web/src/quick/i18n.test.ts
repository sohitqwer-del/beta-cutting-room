import { describe, expect, it } from 'vitest';
import en from './locales/en.json';
import hi from './locales/hi.json';
import ta from './locales/ta.json';
import { pieceName, sizeName, t, warningText } from './i18n';

const holes = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map(m => m[1]).sort();
describe('quick strings', () => {
  it('has every key in Hindi and Tamil with the same placeholders, marked as drafts', () => {
    for (const [name, table] of [['hi', hi], ['ta', ta]] as const) {
      expect(table._status, name).toMatch(/draft.*review/i);
      for (const [key, text] of Object.entries(en)) {
        if (key === '_status') continue;
        const other = (table as Record<string, string>)[key];
        expect(other, `${name}.${key}`).toBeTruthy();
        expect(holes(other!), `${name}.${key}`).toEqual(holes(text));
      }
    }
  });
  it('fills placeholders and falls back to the engine message for unknown warnings', () => {
    expect(t('photo_found', { n: 3 }, 'en')).toBe('3 pieces found');
    expect(t('photo_found', { n: 3 }, 'hi')).toContain('3');
    expect(warningText('tilt', '', 'en').fix).toContain('flat');
    expect(warningText('new_code', 'Engine says hello', 'en')).toEqual({ title: 'Engine says hello', fix: 'Check the photo and try again.' });
    expect(pieceName('Sleeve', 'ta')).toBe('கை');
    expect(pieceName('Gusset', 'hi')).toBe('Gusset');
    expect(pieceName('skirt', 'en')).toBe('Skirt');
    expect(pieceName('gusset', 'en')).toBe('Gusset');
    expect(sizeName('FREE', 'en')).toBe('Free size');
  });
});
