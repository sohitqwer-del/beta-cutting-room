import { describe, expect, it } from 'vitest';
import { BACKUP_KEY, card, listOf, liveLine, nextStep, NUMBERED, ORDER, recoverInterrupted, SAMPLE, STYLE_KEY, stepNumber } from './steps';
import { t, tourLanguages } from './i18n';
import type { StyleFacts } from '../style/ui';

const facts: StyleFacts = {
  name: 'Sample T-shirt', pieces: 4, pieceNames: ['Front', 'Back', 'Sleeve', 'Neck rib (85% neckline length)'], sizes: ['S', 'M', 'L', 'XL'], baseSize: 'M',
  seamChecks: 5, seamChecksOk: 5, widthMm: 1524, layout: 'open', plies: 40, passes: 40, garmentsPerMarker: 4,
  lengthMm: 3059, efficiency: 0.773, checksOk: true, fabricMm: 123_970, garmentsInLay: 160,
};
function memory(initial: Record<string, string> = {}) {
  const data = new Map(Object.entries(initial));
  return { data, getItem: (k: string) => data.get(k) ?? null, setItem: (k: string, v: string) => void data.set(k, v), removeItem: (k: string) => void data.delete(k) };
}

describe('sample tour steps', () => {
  it('walks Pattern, Grade, Lay, Marker and Cut plan in order, numbered 1 to 5', () => {
    expect(ORDER).toEqual(['loading', 'pattern', 'grade', 'lay', 'marker', 'cut', 'end']);
    expect(NUMBERED.map(stepNumber)).toEqual([1, 2, 3, 4, 5]);
    expect(nextStep('lay')).toBe('marker');
    expect(nextStep('end')).toBeNull();
    expect(stepNumber('loading')).toBe(0);
  });
  it('uses the real T-shirt block in four sizes on an open 60-inch lay and a short search', () => {
    expect(SAMPLE.block).toBe('t_shirt');
    expect(SAMPLE.sizes).toEqual(['S', 'M', 'L', 'XL']);
    expect(SAMPLE.lay.widthMm).toBeCloseTo(60 * 25.4);
    expect(SAMPLE.searchS).toBeLessThanOrEqual(20);
  });
  it('fills every caption with the real numbers, in the chosen unit', () => {
    expect(card('pattern', facts, 'cm').text).toContain('4 pieces (Front, Back, Sleeve and Neck rib)');
    expect(card('grade', facts, 'cm').text).toContain('5 of 5 match');
    expect(card('lay', facts, 'cm').text).toContain('152.4 cm wide');
    expect(card('lay', facts, 'inch').text).toContain('60 in wide');
    expect(card('lay', facts, 'cm').next).toBe('Make the marker');
    expect(card('marker', facts, 'cm').text).toContain('3.06 m of fabric; 77.3% of it is in pieces');
    expect(card('marker', facts, 'inch').text).toContain('3.35 yd');
    expect(card('cut', facts, 'cm').text).toBe('For 40 plies you need 123.97 m of fabric and you get 160 garments. Plotter, cutter and PDF files are on this page.');
    expect(card('cut', { ...facts, layout: 'folded', passes: 20 }, 'cm').text).toContain('(20 lengths from the roll)');
  });
  it('says so plainly when the validator is not happy, and offers to stop the search while it runs', () => {
    expect(card('marker', { ...facts, checksOk: false }, 'cm').text).toContain('found something to fix');
    const running = card('marker', { ...facts, lengthMm: null, efficiency: null }, 'cm');
    expect(running.title).toBe('The marker, live');
    expect(running.next).toBe('Use the best so far');
  });
  it('shows the live best length, efficiency and time', () => {
    expect(liveLine({ elapsedS: 3.7, budgetS: 15, bestLengthMm: 3583, efficiency: 0.66 }, 'cm')).toBe('Best so far: 3.58 m, 66.0% of the fabric used · 3 of 15 s');
    expect(liveLine({ elapsedS: 0.4, budgetS: 15, bestLengthMm: null, efficiency: null }, 'cm')).toBe('Looking for the first marker · 0 of 15 s');
  });
  it('lists names in plain English', () => {
    expect(listOf(['S'])).toBe('S');
    expect(listOf(['S', 'M', 'L'])).toBe('S, M and L');
  });
});

describe('the visitor keeps their own style', () => {
  it('puts the own style back after a tour was cut short', () => {
    const store = memory({ [STYLE_KEY]: '{"sample":true}', [BACKUP_KEY]: JSON.stringify({ snapshot: '{"own":true}' }) });
    expect(recoverInterrupted(store)).toBe(true);
    expect(store.data.get(STYLE_KEY)).toBe('{"own":true}');
    expect(store.data.has(BACKUP_KEY)).toBe(false);
  });
  it('removes the sample when there was no own style', () => {
    const store = memory({ [STYLE_KEY]: '{"sample":true}', [BACKUP_KEY]: JSON.stringify({ snapshot: null }) });
    recoverInterrupted(store);
    expect(store.data.has(STYLE_KEY)).toBe(false);
  });
  it('does nothing without a backup and survives a broken one', () => {
    const store = memory({ [STYLE_KEY]: '{"own":true}' });
    expect(recoverInterrupted(store)).toBe(false);
    expect(store.data.get(STYLE_KEY)).toBe('{"own":true}');
    const broken = memory({ [BACKUP_KEY]: '{oops' });
    expect(recoverInterrupted(broken)).toBe(false);
    expect(broken.data.has(BACKUP_KEY)).toBe(false);
    expect(recoverInterrupted(null)).toBe(false);
  });
});

describe('tour strings', () => {
  it('has a draft for every language, marked for review, with the same placeholders', () => {
    const en = tourLanguages.en;
    for (const lang of ['hi', 'ta'] as const) {
      const other = tourLanguages[lang];
      expect(other._status).toMatch(/draft/);
      for (const key of Object.keys(en)) {
        expect(other[key], `${lang}.${key}`).toBeTruthy();
        const holes = (s: string) => (s.match(/\{\w+\}/g) ?? []).sort().join();
        if (key !== '_status') expect(holes(other[key]!), `${lang}.${key}`).toBe(holes(en[key]!));
      }
    }
  });
  it('fills placeholders and falls back to English', () => {
    expect(t('step_of', { n: 2, total: 5 }, 'en')).toBe('Step 2 of 5');
    expect(t('end_time', { s: 41 }, 'hi')).toContain('41');
  });
});
