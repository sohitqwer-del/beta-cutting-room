import { describe, expect, it } from 'vitest';
import { FOLD_EDGE_TEXT, foldEdgeWarning, markerYield } from './yield';
import { dxfImportPath } from './workflows';

const surplusWarning = { code: 'surplus_copy', message: 'x' };
describe('folded and tube lays: useful yield, surplus copies, fold edge (fold7)', () => {
  it('leads with useful yield on folded and tube lays, efficiency on open lays', () => {
    expect(markerYield('folded', { efficiency: 0.82, useful_yield: 0.61, surplus_copies: 2 })).toEqual({ headline: 'yield', usefulYield: 0.61, efficiency: 0.82, surplus: 2 });
    expect(markerYield('tubular', { efficiency: 0.8, useful_yield: 0.7 }).headline).toBe('yield');
    expect(markerYield('open', { efficiency: 0.8, useful_yield: 0.8 }).headline).toBe('efficiency');
    expect(markerYield(undefined, { efficiency: 0.8, useful_yield: 0.8 }).headline).toBe('efficiency');
  });
  it('never estimates: old results without the fields keep footprint efficiency as the headline', () => {
    expect(markerYield('folded', { efficiency: 0.82 })).toEqual({ headline: 'efficiency', usefulYield: null, efficiency: 0.82, surplus: 0 });
    expect(markerYield('folded', { efficiency: 0.82, useful_yield: Number.NaN }).usefulYield).toBeNull();
  });
  it('prefers the fresh validator report, then the result, then counts surplus warnings', () => {
    const y = markerYield('folded', { efficiency: 0.8, useful_yield: 0.5, surplus_copies: 4 }, { warnings: [], useful_yield: 0.55, surplus_copies: 1 });
    expect([y.usefulYield, y.surplus]).toEqual([0.55, 1]);
    expect(markerYield('folded', { efficiency: 0.8 }, { warnings: [surplusWarning, surplusWarning] }).surplus).toBe(2);
    expect(markerYield('folded', { efficiency: 0.8, surplus_copies: 0 }, { warnings: [surplusWarning] }).surplus).toBe(0);
  });
  it('finds the engine fold-edge warning and has plain words for it', () => {
    const w = 'Fold edge: 6 pieces cut on the fold, or too deep to sit beside them, measure about 6.90 m end to end; this marker is 6.96 m.';
    expect(foldEdgeWarning(['Other', w])).toBe(w);
    expect(foldEdgeWarning(['Other'])).toBeNull();
    expect(foldEdgeWarning(undefined)).toBeNull();
    expect(FOLD_EDGE_TEXT).toMatch(/^This order is limited by the fold length, not by search time\./);
  });
  it('always sends the DXF "Cut 2" choice, pairs by default like the engine', () => {
    expect(dxfImportPath(1500, 'auto', true)).toBe('import?format=dxf&width=1500&units=auto&pairs=true');
    expect(dxfImportPath(1500, 'mm', false)).toBe('import?format=dxf&width=1500&units=mm&pairs=false');
  });
});
