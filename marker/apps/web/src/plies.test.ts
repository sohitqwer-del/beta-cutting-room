import { describe, expect, it } from 'vitest';
import { headCell, PASS_TIP, pliesAndPasses } from './plies';
import en from './locales/en.json';
import hi from './locales/hi.json';
import ta from './locales/ta.json';

const esc = (v: unknown): string => String(v).replace(/</g, '&lt;');
describe('plies and passes wording (marker/docs/PLIES.md)', () => {
  it('names both counts when a lay is folded or tube, one when they are the same', () => {
    expect(pliesAndPasses(40, 20)).toBe('40 plies · 20 passes');
    expect(pliesAndPasses(20, 20)).toBe('20 plies');
    expect(pliesAndPasses(20, undefined)).toBe('20 plies');
    expect(pliesAndPasses(2, 1)).toBe('2 plies · 1 pass');
  });
  it('gives the Passes column its tooltip and escapes other headers', () => {
    expect(headCell('Passes', esc)).toBe(`<th scope="col" data-floor-label="th_passes" title="${PASS_TIP}" data-floor-title="pass_tip">Passes</th>`);
    expect(headCell('<b>', esc)).toBe('<th scope="col">&lt;b></th>');
  });
  it('translates Rolls, Bundles and Floor table headers and the Passes tooltip (drafts)', () => {
    expect(headCell('Passes', esc, 'hi')).toBe(`<th scope="col" data-floor-label="th_passes" title="${hi.pass_tip}" data-floor-title="pass_tip" lang="hi">${hi.th_passes}</th>`);
    expect(headCell('Roll', esc, 'ta')).toBe(`<th scope="col" data-floor-label="th_roll" lang="ta">${ta.th_roll}</th>`);
    const heads = Object.keys(en).filter(k => k.startsWith('th_')) as (keyof typeof en)[];
    expect(heads.length).toBeGreaterThan(50);
    for (const key of heads) {
      expect(headCell(en[key], esc)).toContain(`data-floor-label="${key}"`);
      expect(hi[key]).toBeTruthy(); expect(ta[key]).toBeTruthy();
    }
    expect(PASS_TIP).toBe('1 pass = one length from the roll; folded or tube = 2 plies.');
  });
  it('has Hindi and Tamil drafts for the new Floor labels', () => {
    for (const key of ['passes', 'actual_passes', 'pass_tip', 'status_browser_tip', 'status_offline_tip'] as const) {
      expect(en[key]).toBeTruthy(); expect(hi[key]).toBeTruthy(); expect(ta[key]).toBeTruthy();
      expect(hi[key]).not.toBe(en[key]); expect(ta[key]).not.toBe(en[key]);
    }
    expect(en.pass_tip).toBe(PASS_TIP);
    expect(en.status_browser_tip).not.toMatch(/Plotter/); expect(en.status_browser_tip).toContain('Only the board-photo digitizer');
  });
});
