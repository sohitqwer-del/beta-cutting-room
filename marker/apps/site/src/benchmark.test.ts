import { describe, expect, it } from 'vitest';
import { readdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';
import { describeStratum, findForbiddenKeys, headlineGroup, loadRuns, parseRun, parseRunText, selectRuns, type Run } from './benchmark.ts';

const dataDir = fileURLToPath(new URL('../data/benchmark/', import.meta.url));

function minimal(over: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    schema: 'beta-site-benchmark/1', id: 'lectra-60s', title: 'Lectra, 60 s', kind: 'lectra', status: 'final', date: '2026-10-19',
    software: { beta_version: '0.4.0', engine_revision: 'abc1234' },
    reference: { name: 'Lectra', type: 'commercial-engine' },
    conditions: { budget_label: '60 s', budget_s: 60, machine: 'Test machine' },
    assumptions: ['Sheet type 0 is an open strip.'],
    groups: [{ label: 'Tier B', headline: true, attempts: 10, valid: 10, mean_diff_pp: -1.5, median_diff_pp: -1.2, win: 3, tie: 1, loss: 6 }],
    reproduce: { commands: ['cargo run'], doc: 'marker/docs/benchmarks.md' },
    ...over,
  };
}

function run(over: Record<string, unknown> = {}): Run {
  const r = parseRun(minimal(over), 'test.json');
  if (!r.ok) throw new Error(r.errors.join('\n'));
  return r.run;
}

describe('parseRun', () => {
  it('accepts a minimal valid file and fills defaults', () => {
    const r = run();
    expect(r.order).toBe(100);
    expect(r.groups[0]!.too_long_losses).toBe(0);
    expect(r.groups[0]!.other_failures).toBe(0);
    expect(r.items).toEqual([]);
    expect(headlineGroup(r).label).toBe('Tier B');
  });

  it('rejects another schema version', () => {
    const r = parseRun(minimal({ schema: 'beta-site-benchmark/2' }), 'x.json');
    expect(r.ok).toBe(false);
  });

  it('rejects win + tie + loss that does not equal valid', () => {
    const r = parseRun(minimal({ groups: [{ label: 'A', attempts: 10, valid: 9, mean_diff_pp: 0, median_diff_pp: 0, win: 3, tie: 1, loss: 6 }] }), 'x.json');
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.errors.join()).toMatch(/must equal valid/);
  });

  it('keeps too_long losses separate from valid-layout losses', () => {
    const r = run({ groups: [{ label: 'A', attempts: 12, valid: 10, mean_diff_pp: -1, median_diff_pp: -1, win: 3, tie: 1, loss: 6, too_long_losses: 2 }] });
    expect(r.groups[0]!.too_long_losses).toBe(2);
  });

  it('rejects valid > attempts and mixed null counts', () => {
    expect(parseRun(minimal({ groups: [{ label: 'A', attempts: 5, valid: 6, mean_diff_pp: 0, median_diff_pp: 0, win: null, tie: null, loss: null }] }), 'x').ok).toBe(false);
    expect(parseRun(minimal({ groups: [{ label: 'A', attempts: 5, valid: 5, mean_diff_pp: 0, median_diff_pp: 0, win: 5, tie: null, loss: null }] }), 'x').ok).toBe(false);
  });

  it('requires progress and a note for provisional runs', () => {
    expect(parseRun(minimal({ status: 'provisional' }), 'x').ok).toBe(false);
    expect(parseRun(minimal({ status: 'provisional', status_note: 'Running' }), 'x').ok).toBe(false);
    expect(parseRun(minimal({ status: 'provisional', status_note: 'Running', progress: { completed: 126, planned: 300 } }), 'x').ok).toBe(true);
    expect(parseRun(minimal({ status: 'provisional', status_note: 'Running', progress: { completed: 301, planned: 300 } }), 'x').ok).toBe(false);
  });

  it('rejects geometry anywhere in the file', () => {
    const r = parseRun(minimal({ extensions: { tasks: [{ id: '1', polygon: [[0, 0]] }] } }), 'x.json');
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.errors.join()).toMatch(/geometry/);
    expect(findForbiddenKeys({ a: { Points: 1 } })).toEqual(['$.a.Points']);
  });

  it('ignores unknown extension fields', () => {
    expect(run({ x_runtime: { wall_s: 3600 }, extensions: { load: 'high' } }).id).toBe('lectra-60s');
  });

  it('accepts a release engine baseline without calling it commercial', () => {
    const r = run({ reference: { name: 'BETA Cutting Room 0.7.0', type: 'engine-baseline' } });
    expect(r.reference.type).toBe('engine-baseline');
    expect(headlineGroup(r).median_diff_pp).toBe(-1.2);
  });

  it('requires null differences when there is no reference', () => {
    const bad = parseRun(minimal({ reference: { name: 'None', type: 'none' } }), 'x');
    expect(bad.ok).toBe(false);
    const good = parseRun(minimal({ reference: { name: 'None', type: 'none' }, groups: [{ label: 'A', attempts: 3, valid: 3, mean_diff_pp: null, median_diff_pp: null, win: null, tie: null, loss: null }] }), 'x');
    expect(good.ok).toBe(true);
  });

  it('turns JSON syntax errors into messages', () => {
    const r = parseRunText('{ not json', 'broken.json');
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.errors[0]).toMatch(/invalid JSON/);
  });

  it('checks quantile order and correlation range', () => {
    expect(parseRun(minimal({ quantiles_pp: { min: 1, p05: 0, p50: 2, p95: 3, max: 4 } }), 'x').ok).toBe(false);
    expect(parseRun(minimal({ correlation: { pairs: 3, pearson: 1.2, spearman: 0.5 } }), 'x').ok).toBe(false);
  });
});

describe('selectRuns', () => {
  const sample = run({ status: 'sample', status_note: 'Sample', date: '2026-10-18' });
  const prov = (completed: number, date = '2026-10-19') => run({ status: 'provisional', status_note: 'Running', progress: { completed, planned: 300 }, date });

  it('shows a sample and the most complete provisional run side by side', () => {
    const chosen = selectRuns([prov(93), sample, prov(126)]);
    expect(chosen.map((r) => r.status)).toEqual(['sample', 'provisional']);
    expect(chosen[1]!.progress!.completed).toBe(126);
  });

  it('lets the newest final file replace everything else with that id', () => {
    const finalOld = run({ date: '2026-10-19', title: 'old' });
    const finalNew = run({ date: '2026-10-20', title: 'new' });
    const chosen = selectRuns([sample, prov(126), finalOld, finalNew]);
    expect(chosen).toHaveLength(1);
    expect(chosen[0]!.title).toBe('new');
  });

  it('sorts by order, then id', () => {
    const a = run({ id: 'b-run', order: 5 });
    const b = run({ id: 'a-run', order: 5 });
    const c = run({ id: 'first', order: 1 });
    expect(selectRuns([a, b, c]).map((r) => r.id)).toEqual(['first', 'a-run', 'b-run']);
  });
});

describe('shipped data files', () => {
  const files = readdirSync(dataDir).filter((f) => f.endsWith('.json')).map((name) => ({ name, text: readFileSync(join(dataDir, name), 'utf8') }));

  it('all validate against beta-site-benchmark/1', () => {
    const { errors } = loadRuns(files);
    expect(errors).toEqual([]);
    expect(files.length).toBeGreaterThan(0);
  });

  // No check on the sign of the result: the data decides that. Lectra headlines must
  // show mean, median and win/tie/loss together so a mean alone is never the claim.
  it('label every non-final file and give full Lectra headlines', () => {
    const { runs } = loadRuns(files);
    for (const r of runs) {
      if (r.status !== 'final') expect(r.status_note).toBeTruthy();
      if (r.kind === 'lectra') {
        const h = headlineGroup(r);
        for (const v of [h.mean_diff_pp, h.median_diff_pp, h.win, h.tie, h.loss]) expect(v).toEqual(expect.any(Number));
      }
    }
  });
});

describe('describeStratum', () => {
  it('writes strata in plain English', () => {
    expect(describeStratum('pieces:1-25;lectra:<75')).toBe('1–25 pieces, Lectra below 75%');
    expect(describeStratum('pieces:151+;lectra:85+')).toBe('151+ pieces, Lectra 85% or more');
    expect(describeStratum('pieces:26-75;lectra:75-85')).toBe('26–75 pieces, Lectra 75–85%');
    expect(describeStratum('Tier B')).toBe('Tier B');
  });
});
