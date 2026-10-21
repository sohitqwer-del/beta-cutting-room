import { describe, expect, it } from 'vitest';
import type { Job, Pt } from '@beta/marker-wasm';
import { chordWidth, edgeDistance, isProductionLabel, labelAnchor, planLabel, shorten, textEms } from './labels';
import { placed, ruleReports, stripeStep, styleRuleOf, type MarkerMatch } from './match-report';
import { errorText } from './ui';

const box = (w: number, h: number, x = 0, y = 0): Pt[] => [[x, y], [x + w, y], [x + w, y + h], [x, y + h]];
// An L shape: the box centre (50, 50) lies outside the piece.
const ell: Pt[] = [[0, 0], [100, 0], [100, 20], [20, 20], [20, 100], [0, 100]];

describe('piece labels', () => {
  it('anchors the label inside the piece, even when the box centre is outside', () => {
    const a = labelAnchor(ell)!;
    expect(a.at[0] >= 0 && a.at[0] <= 100 && a.at[1] >= 0 && a.at[1] <= 100).toBe(true);
    expect(edgeDistance(ell, a.at)).toBeGreaterThan(5);
    expect(a.clearance).toBeLessThan(15);
    const big = labelAnchor(box(400, 600))!;
    expect(big.at[0]).toBeCloseTo(200, -1); expect(big.at[1]).toBeCloseTo(300, -1);
  });
  it('measures the free width across the piece', () => {
    expect(chordWidth(box(400, 600), [200, 300])).toBeCloseTo(400, 6);
    expect(chordWidth(ell, [10, 60])).toBeCloseTo(20, 6);
    expect(chordWidth(ell, [60, 60])).toBe(0);
  });
  it('scales with zoom: full label when large, name only when small, nothing when tiny', () => {
    const piece = box(500, 700);
    const near = planLabel(piece, 'Front', 'M · cut 1', 1)!;
    expect(near.showDetail).toBe(true); expect(near.name).toBeLessThanOrEqual(17);
    const mid = planLabel(piece, 'Front', 'M · cut 1 + 1 mirrored · lining', 0.12)!;
    expect(mid.showDetail).toBe(false); expect(mid.text).toBe('Front');
    expect(planLabel(piece, 'Front', 'M · cut 1', 0.03)).toBeNull();
  });
  it('never makes a label wider than the piece', () => {
    for (const scale of [0.08, 0.2, 0.5, 1, 3]) {
      const piece = box(300, 40), plan = planLabel(piece, 'Neck rib (15% stretch)', 'M · cut 1', scale);
      if (!plan) continue;
      expect(textEms(plan.text) * plan.name).toBeLessThanOrEqual(300 * scale + 0.5);
      expect(plan.name * 1.2).toBeLessThanOrEqual(40 * scale + 0.5);
    }
  });
  it('shortens long names with an ellipsis and hides the engine production label', () => {
    expect(shorten('Neck rib (15% stretch)', 6)).toMatch(/…$/);
    expect(textEms(shorten('Neck rib (15% stretch)', 6))).toBeLessThanOrEqual(6);
    expect(shorten('Front', 40)).toBe('Front');
    expect(shorten('Front', 1)).toBe('');
    expect(isProductionLabel('Front | size M | cut 1 + 0 mirrored | fabric shell', 'Front')).toBe(true);
    expect(isProductionLabel('Pocket here', 'Front')).toBe(false);
  });
});

describe('match report', () => {
  const job = { pieces: ['S', 'M'].flatMap(s => [{ id: `front-${s}`, size: s }, { id: `back-${s}`, size: s }]),
    match_rules: [0, 1].flatMap(() => ['S', 'M'].map(s => ({ a: { piece_id: `front-${s}`, point_id: 'h' }, b: { piece_id: `back-${s}`, point_id: 'h' }, axes: 'y' }))) } as unknown as Job;
  const row = (rule: number, pass: boolean, extra: Partial<MarkerMatch> = {}): MarkerMatch => ({ rule, garment: rule, a: [1, 2], b: [3, 4], error_mm: pass ? 0 : Number.NaN, pass, ...extra });
  it('maps the per-size job rules back to the style rule', () => {
    expect([0, 1, 2, 3].map(i => styleRuleOf(job, i, 2))).toEqual([0, 0, 1, 1]);
    expect(styleRuleOf(undefined, 3, 2)).toBe(3);
  });
  it('reports matched, off with the worst mm, and not placed', () => {
    const r = ruleReports(3, { ...job, match_rules: [...job.match_rules!, ...job.match_rules!.slice(0, 2)] } as Job,
      [row(0, true), row(1, true), row(2, false, { error_mm: 1.5 }), row(3, false, { error_mm: 3.25 })]);
    expect(r.map(x => x.state)).toEqual(['matched', 'off', 'not_placed']);
    expect(r[1]!.worstMm).toBe(3.25); expect(r[1]!.off).toBe(2); expect(r[0]!.worstMm).toBeNull(); expect(r[0]!.maxMm).toBe(0); expect(r[2]!.maxMm).toBeNull();
    expect(r[0]!.marks.map(m => m.size)).toEqual(['S', 'M']);
  });
  it('accepts the engine match report: not placed rows and missing offsets', () => {
    const r = ruleReports(2, job, [row(0, true), { ...row(1, false), status: 'not_placed', a: null, error_mm: null }, row(2, false), row(3, false, { status: 'off', error_mm: null })]);
    expect(r[0]).toMatchObject({ state: 'matched', checked: 1, missing: 1 });
    expect(r[1]).toMatchObject({ state: 'off', off: 2, worstMm: null });
    expect(placed({ ...row(0, true), status: 'not_placed' })).toBe(false);
  });
  it('thins dense stripes so drawn lines stay apart', () => {
    expect(stripeStep(10, 5, 6)).toBe(3);
    expect(stripeStep(50, 1, 6)).toBe(1);
    expect(stripeStep(0, 5)).toBe(1);
  });
});

describe('error text', () => {
  it('turns a list of engine issues into a count and the first reasons', () => {
    const text = errorText(new Error('Validate this marker before export: [{"code":"grain","message":"Piece \'front-L\' turned."},{"code":"bundle","message":"Bundles differ."},{"code":"x","message":"Third."}]'));
    expect(text).toMatch(/^Validate this marker before export\. 3 problems/);
    expect(text).toContain('front-L'); expect(text).not.toContain('Third'); expect(text).not.toContain('{');
    expect(errorText(new Error('Plain message.'))).toBe('Plain message.');
  });
});
