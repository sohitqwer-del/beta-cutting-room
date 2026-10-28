// Loader for site benchmark files (schema beta-site-benchmark/1, see data/benchmark/SCHEMA.md).
// Pure functions: the caller reads the files and passes their text in.

export const SCHEMA_TAG = 'beta-site-benchmark/1';

export type RunKind = 'lectra' | 'academic' | 'freesewing' | 'other';
export type RunStatus = 'final' | 'provisional' | 'sample';
export type ReferenceType = 'commercial-engine' | 'published-paper' | 'engine-baseline' | 'none';

export interface Group {
  label: string;
  headline: boolean;
  attempts: number;
  valid: number;
  mean_diff_pp: number | null;
  median_diff_pp: number | null;
  win: number | null;
  tie: number | null;
  loss: number | null;
  too_long_losses: number;
  mean_length_saving_pct: number | null;
  median_length_saving_pct: number | null;
  other_failures: number;
  mean_beta_eff_pct: number | null;
  mean_reference_eff_pct: number | null;
  task_ids: string[];
}

export interface Item {
  id: string;
  group: string | null;
  seed: number | null;
  valid: boolean;
  beta_eff_pct: number | null;
  reference_eff_pct: number | null;
  diff_pp: number | null;
  runtime_s: number | null;
  failure: string | null;
}

export interface Run {
  file: string;
  id: string;
  title: string;
  kind: RunKind;
  status: RunStatus;
  status_note: string | null;
  order: number;
  date: string;
  summary_text: string | null;
  progress: { completed: number; planned: number } | null;
  software: { beta_version: string; engine_revision: string | null; engine: string | null };
  reference: { name: string; type: ReferenceType; citation: string | null; url: string | null; licence: string | null };
  conditions: {
    budget_label: string; budget_s: number | null; machine: string;
    jobs: number | null; workers_per_task: number | null; seeds: number[];
    background_load: string | null; notes: string | null;
  };
  assumptions: string[];
  groups: Group[];
  strata: Group[];
  items: Item[];
  task_ids: string[];
  correlation: { pairs: number; pearson: number; spearman: number } | null;
  quantiles_pp: { min: number; p05: number; p50: number; p95: number; max: number } | null;
  reproduce: { commands: string[]; doc: string };
  sources: { label: string; url: string }[];
}

export type ParseResult = { ok: true; run: Run } | { ok: false; file: string; errors: string[] };

/** Object keys that would mean geometry slipped into a public file. */
export const FORBIDDEN_KEYS = ['geometry', 'points', 'polygon', 'polygons', 'outline', 'outlines', 'vertices', 'shape', 'shapes', 'coordinates'];

const KINDS: RunKind[] = ['lectra', 'academic', 'freesewing', 'other'];
const STATUSES: RunStatus[] = ['final', 'provisional', 'sample'];
const REFERENCE_TYPES: ReferenceType[] = ['commercial-engine', 'published-paper', 'engine-baseline', 'none'];

type Obj = Record<string, unknown>;
const isObj = (v: unknown): v is Obj => typeof v === 'object' && v !== null && !Array.isArray(v);

/** Collects validation errors with a path prefix, so messages say where the problem is. */
class Checker {
  errors: string[] = [];
  fail(path: string, message: string): void { this.errors.push(`${path}: ${message}`); }

  str(o: Obj, key: string, path: string, required = true): string | null {
    const v = o[key];
    if (v === undefined || v === null) { if (required) this.fail(`${path}.${key}`, 'is required'); return null; }
    if (typeof v !== 'string' || (required && v.trim() === '')) { this.fail(`${path}.${key}`, 'must be a non-empty string'); return null; }
    return v;
  }

  num(o: Obj, key: string, path: string, opts: { required?: boolean; nullable?: boolean; int?: boolean; min?: number } = {}): number | null {
    const v = o[key];
    if (v === undefined) { if (opts.required) this.fail(`${path}.${key}`, 'is required'); return null; }
    if (v === null) { if (!opts.nullable) this.fail(`${path}.${key}`, 'must not be null'); return null; }
    if (typeof v !== 'number' || !Number.isFinite(v)) { this.fail(`${path}.${key}`, 'must be a finite number'); return null; }
    if (opts.int && !Number.isInteger(v)) { this.fail(`${path}.${key}`, 'must be an integer'); return null; }
    if (opts.min !== undefined && v < opts.min) { this.fail(`${path}.${key}`, `must be >= ${opts.min}`); return null; }
    return v;
  }

  strings(o: Obj, key: string, path: string): string[] {
    const v = o[key];
    if (v === undefined || v === null) return [];
    if (!Array.isArray(v) || v.some((x) => typeof x !== 'string')) { this.fail(`${path}.${key}`, 'must be an array of strings'); return []; }
    return v as string[];
  }

  oneOf<T extends string>(o: Obj, key: string, path: string, allowed: readonly T[]): T | null {
    const v = o[key];
    if (typeof v !== 'string' || !allowed.includes(v as T)) { this.fail(`${path}.${key}`, `must be one of ${allowed.join(', ')}`); return null; }
    return v as T;
  }
}

/** Returns the paths of any forbidden geometry keys anywhere in the value. */
export function findForbiddenKeys(value: unknown, path = '$'): string[] {
  const found: string[] = [];
  if (Array.isArray(value)) value.forEach((v, i) => found.push(...findForbiddenKeys(v, `${path}[${i}]`)));
  else if (isObj(value)) {
    for (const [k, v] of Object.entries(value)) {
      if (FORBIDDEN_KEYS.includes(k.toLowerCase())) found.push(`${path}.${k}`);
      found.push(...findForbiddenKeys(v, `${path}.${k}`));
    }
  }
  return found;
}

function parseGroup(c: Checker, raw: unknown, path: string, hasReference: boolean): Group | null {
  if (!isObj(raw)) { c.fail(path, 'must be an object'); return null; }
  const before = c.errors.length;
  const attempts = c.num(raw, 'attempts', path, { required: true, int: true, min: 0 }) ?? 0;
  const valid = c.num(raw, 'valid', path, { required: true, int: true, min: 0 }) ?? 0;
  if (valid > attempts) c.fail(`${path}.valid`, `(${valid}) must not exceed attempts (${attempts})`);
  for (const key of ['mean_diff_pp', 'median_diff_pp', 'win', 'tie', 'loss']) {
    if (!(key in raw)) c.fail(`${path}.${key}`, 'is required (use null if not measured)');
  }
  const win = c.num(raw, 'win', path, { nullable: true, int: true, min: 0 });
  const tie = c.num(raw, 'tie', path, { nullable: true, int: true, min: 0 });
  const loss = c.num(raw, 'loss', path, { nullable: true, int: true, min: 0 });
  const counts = [win, tie, loss];
  if (counts.some((x) => x !== null) && counts.some((x) => x === null)) c.fail(path, 'win, tie and loss must all be numbers or all be null');
  if (win !== null && tie !== null && loss !== null && win + tie + loss !== valid) {
    c.fail(path, `win + tie + loss (${win + tie + loss}) must equal valid (${valid})`);
  }
  const mean = c.num(raw, 'mean_diff_pp', path, { nullable: true });
  if (!hasReference && (mean !== null || win !== null)) c.fail(path, 'differences and win/tie/loss must be null when reference.type is none');
  if (raw.headline !== undefined && typeof raw.headline !== 'boolean') c.fail(`${path}.headline`, 'must be a boolean');
  const group: Group = {
    label: c.str(raw, 'label', path) ?? '',
    headline: raw.headline === true,
    attempts, valid,
    mean_diff_pp: mean,
    median_diff_pp: c.num(raw, 'median_diff_pp', path, { nullable: true }),
    win, tie, loss,
    too_long_losses: c.num(raw, 'too_long_losses', path, { int: true, min: 0 }) ?? 0,
    mean_length_saving_pct: c.num(raw, 'mean_length_saving_pct', path, { nullable: true }),
    median_length_saving_pct: c.num(raw, 'median_length_saving_pct', path, { nullable: true }),
    other_failures: c.num(raw, 'other_failures', path, { int: true, min: 0 }) ?? 0,
    mean_beta_eff_pct: c.num(raw, 'mean_beta_eff_pct', path, { nullable: true }),
    mean_reference_eff_pct: c.num(raw, 'mean_reference_eff_pct', path, { nullable: true }),
    task_ids: c.strings(raw, 'task_ids', path),
  };
  return c.errors.length === before ? group : null;
}

function parseItem(c: Checker, raw: unknown, path: string): Item | null {
  if (!isObj(raw)) { c.fail(path, 'must be an object'); return null; }
  const before = c.errors.length;
  if (typeof raw.valid !== 'boolean') c.fail(`${path}.valid`, 'must be a boolean');
  if (!('beta_eff_pct' in raw)) c.fail(`${path}.beta_eff_pct`, 'is required (use null if not valid)');
  const item: Item = {
    id: typeof raw.id === 'number' ? String(raw.id) : c.str(raw, 'id', path) ?? '',
    group: c.str(raw, 'group', path, false),
    seed: c.num(raw, 'seed', path, { nullable: true, int: true }),
    valid: raw.valid === true,
    beta_eff_pct: c.num(raw, 'beta_eff_pct', path, { nullable: true, min: 0 }),
    reference_eff_pct: c.num(raw, 'reference_eff_pct', path, { nullable: true, min: 0 }),
    diff_pp: c.num(raw, 'diff_pp', path, { nullable: true }),
    runtime_s: c.num(raw, 'runtime_s', path, { nullable: true, min: 0 }),
    failure: c.str(raw, 'failure', path, false),
  };
  if (item.valid && item.beta_eff_pct === null) c.fail(`${path}.beta_eff_pct`, 'is required for a valid item');
  return c.errors.length === before ? item : null;
}

/** Validates one parsed JSON value against beta-site-benchmark/1. */
export function parseRun(value: unknown, file: string): ParseResult {
  const c = new Checker();
  if (!isObj(value)) return { ok: false, file, errors: ['$: must be a JSON object'] };
  for (const p of findForbiddenKeys(value)) c.fail(p, 'geometry is not allowed in public benchmark files');
  if (value.schema !== SCHEMA_TAG) return { ok: false, file, errors: [`$.schema: must be "${SCHEMA_TAG}"`, ...c.errors] };

  const id = c.str(value, 'id', '$') ?? '';
  if (id && !/^[a-z0-9-]+$/.test(id)) c.fail('$.id', 'must match [a-z0-9-]+');
  const status = c.oneOf(value, 'status', '$', STATUSES);
  const statusNote = c.str(value, 'status_note', '$', status !== null && status !== 'final');
  const date = c.str(value, 'date', '$') ?? '';
  if (date && !/^\d{4}-\d{2}-\d{2}$/.test(date)) c.fail('$.date', 'must be YYYY-MM-DD');

  let progress: Run['progress'] = null;
  if (isObj(value.progress)) {
    const completed = c.num(value.progress, 'completed', '$.progress', { required: true, int: true, min: 0 }) ?? 0;
    const planned = c.num(value.progress, 'planned', '$.progress', { required: true, int: true, min: 1 }) ?? 1;
    if (completed > planned) c.fail('$.progress', 'completed must not exceed planned');
    progress = { completed, planned };
  } else if (value.progress !== undefined && value.progress !== null) c.fail('$.progress', 'must be an object');
  if (status === 'provisional' && !progress) c.fail('$.progress', 'is required when status is provisional');

  const sw = isObj(value.software) ? value.software : (c.fail('$.software', 'is required'), {});
  const ref = isObj(value.reference) ? value.reference : (c.fail('$.reference', 'is required'), {});
  const cond = isObj(value.conditions) ? value.conditions : (c.fail('$.conditions', 'is required'), {});
  const repro = isObj(value.reproduce) ? value.reproduce : (c.fail('$.reproduce', 'is required'), {});
  if (!('engine_revision' in sw)) c.fail('$.software.engine_revision', 'is required (use null if unknown)');
  if (!('budget_s' in cond)) c.fail('$.conditions.budget_s', 'is required (use null if per task)');
  const refType = c.oneOf(ref, 'type', '$.reference', REFERENCE_TYPES);
  const hasReference = refType !== 'none';

  if (!Array.isArray(value.assumptions) || value.assumptions.some((a) => typeof a !== 'string')) c.fail('$.assumptions', 'must be an array of strings');
  if (!Array.isArray(value.groups) || value.groups.length === 0) c.fail('$.groups', 'must be a non-empty array');
  const groups = (Array.isArray(value.groups) ? value.groups : []).map((g, i) => parseGroup(c, g, `$.groups[${i}]`, hasReference)).filter((g): g is Group => g !== null);
  if (groups.filter((g) => g.headline).length > 1) c.fail('$.groups', 'at most one group may be the headline');
  if (value.strata !== undefined && !Array.isArray(value.strata)) c.fail('$.strata', 'must be an array');
  const strata = (Array.isArray(value.strata) ? value.strata : []).map((g, i) => parseGroup(c, g, `$.strata[${i}]`, hasReference)).filter((g): g is Group => g !== null);
  if (value.items !== undefined && !Array.isArray(value.items)) c.fail('$.items', 'must be an array');
  const items = (Array.isArray(value.items) ? value.items : []).map((it, i) => parseItem(c, it, `$.items[${i}]`)).filter((it): it is Item => it !== null);

  let correlation: Run['correlation'] = null;
  if (isObj(value.correlation)) {
    const pairs = c.num(value.correlation, 'pairs', '$.correlation', { required: true, int: true, min: 0 });
    const pearson = c.num(value.correlation, 'pearson', '$.correlation', { required: true });
    const spearman = c.num(value.correlation, 'spearman', '$.correlation', { required: true });
    for (const [k, v] of [['pearson', pearson], ['spearman', spearman]] as const) if (v !== null && Math.abs(v) > 1) c.fail(`$.correlation.${k}`, 'must be between -1 and 1');
    if (pairs !== null && pearson !== null && spearman !== null) correlation = { pairs, pearson, spearman };
  }
  let quantiles: Run['quantiles_pp'] = null;
  if (isObj(value.quantiles_pp)) {
    const q = ['min', 'p05', 'p50', 'p95', 'max'].map((k) => c.num(value.quantiles_pp as Obj, k, '$.quantiles_pp', { required: true }));
    if (q.every((x): x is number => x !== null)) {
      if (q.some((x, i) => i > 0 && x < q[i - 1]!)) c.fail('$.quantiles_pp', 'must be in increasing order');
      quantiles = { min: q[0]!, p05: q[1]!, p50: q[2]!, p95: q[3]!, max: q[4]! };
    }
  }
  const sources: Run['sources'] = [];
  if (Array.isArray(value.sources)) {
    value.sources.forEach((s, i) => {
      if (!isObj(s)) { c.fail(`$.sources[${i}]`, 'must be an object'); return; }
      const label = c.str(s, 'label', `$.sources[${i}]`); const url = c.str(s, 'url', `$.sources[${i}]`);
      if (label && url) sources.push({ label, url });
    });
  }

  const run: Run = {
    file, id,
    title: c.str(value, 'title', '$') ?? '',
    kind: c.oneOf(value, 'kind', '$', KINDS) ?? 'other',
    status: status ?? 'sample',
    status_note: statusNote,
    order: c.num(value, 'order', '$') ?? 100,
    date,
    summary_text: c.str(value, 'summary_text', '$', false),
    progress,
    software: {
      beta_version: c.str(sw, 'beta_version', '$.software') ?? '',
      engine_revision: typeof sw.engine_revision === 'string' ? sw.engine_revision : null,
      engine: c.str(sw, 'engine', '$.software', false),
    },
    reference: {
      name: c.str(ref, 'name', '$.reference') ?? '',
      type: refType ?? 'none',
      citation: c.str(ref, 'citation', '$.reference', false),
      url: c.str(ref, 'url', '$.reference', false),
      licence: c.str(ref, 'licence', '$.reference', false),
    },
    conditions: {
      budget_label: c.str(cond, 'budget_label', '$.conditions') ?? '',
      budget_s: c.num(cond, 'budget_s', '$.conditions', { nullable: true, min: 0 }),
      machine: c.str(cond, 'machine', '$.conditions') ?? '',
      jobs: c.num(cond, 'jobs', '$.conditions', { nullable: true, int: true, min: 1 }),
      workers_per_task: c.num(cond, 'workers_per_task', '$.conditions', { nullable: true, int: true, min: 1 }),
      seeds: Array.isArray(cond.seeds) && cond.seeds.every((s) => Number.isInteger(s)) ? (cond.seeds as number[]) : (cond.seeds === undefined ? [] : (c.fail('$.conditions.seeds', 'must be an array of integers'), [])),
      background_load: c.str(cond, 'background_load', '$.conditions', false),
      notes: c.str(cond, 'notes', '$.conditions', false),
    },
    assumptions: Array.isArray(value.assumptions) ? value.assumptions.filter((a): a is string => typeof a === 'string') : [],
    groups, strata, items,
    task_ids: c.strings(value, 'task_ids', '$'),
    correlation,
    quantiles_pp: quantiles,
    reproduce: {
      commands: c.strings(repro, 'commands', '$.reproduce'),
      doc: c.str(repro, 'doc', '$.reproduce') ?? '',
    },
    sources,
  };
  return c.errors.length ? { ok: false, file, errors: c.errors } : { ok: true, run };
}

/** Parses JSON text; syntax errors become validation errors instead of exceptions. */
export function parseRunText(text: string, file: string): ParseResult {
  let value: unknown;
  try { value = JSON.parse(text.replace(/^﻿/, '')); } catch (e) { return { ok: false, file, errors: [`invalid JSON: ${(e as Error).message}`] }; }
  return parseRun(value, file);
}

const byDateDesc = (a: Run, b: Run) => b.date.localeCompare(a.date);

/**
 * Chooses which runs to show when several files share an id (rules in SCHEMA.md):
 * newest final wins alone; otherwise one sample plus the most complete provisional.
 */
export function selectRuns(runs: Run[]): Run[] {
  const byId = new Map<string, Run[]>();
  for (const r of runs) byId.set(r.id, [...(byId.get(r.id) ?? []), r]);
  const chosen: Run[] = [];
  for (const list of byId.values()) {
    const finals = list.filter((r) => r.status === 'final').sort(byDateDesc);
    if (finals.length) { chosen.push(finals[0]!); continue; }
    const sample = list.filter((r) => r.status === 'sample').sort(byDateDesc)[0];
    const provisional = list.filter((r) => r.status === 'provisional')
      .sort((a, b) => (b.progress?.completed ?? 0) - (a.progress?.completed ?? 0) || byDateDesc(a, b))[0];
    if (sample) chosen.push(sample);
    if (provisional) chosen.push(provisional);
  }
  const statusRank: Record<RunStatus, number> = { final: 0, sample: 1, provisional: 2 };
  return chosen.sort((a, b) => a.order - b.order || a.id.localeCompare(b.id) || statusRank[a.status] - statusRank[b.status]);
}

export interface LoadResult { runs: Run[]; errors: { file: string; errors: string[] }[] }

/** Parses many files, keeps valid ones, and reports the rest. */
export function loadRuns(files: { name: string; text: string }[]): LoadResult {
  const runs: Run[] = []; const errors: LoadResult['errors'] = [];
  for (const f of files) {
    const r = parseRunText(f.text, f.name);
    if (r.ok) runs.push(r.run); else errors.push({ file: r.file, errors: r.errors });
  }
  return { runs: selectRuns(runs), errors };
}

/** The group shown large for a run: the flagged headline, else the first group. */
export function headlineGroup(run: Run): Group {
  return run.groups.find((g) => g.headline) ?? run.groups[0]!;
}

/** Turns a stratum label like "pieces:1-25;lectra:<75" into plain English. */
export function describeStratum(label: string): string {
  const parts = Object.fromEntries(label.split(';').map((p) => p.split(':') as [string, string]));
  if (!parts.pieces || !parts.lectra) return label;
  const pieces = parts.pieces.endsWith('+') ? `${parts.pieces.slice(0, -1)}+ pieces` : `${parts.pieces.replace('-', '–')} pieces`;
  const eff = parts.lectra.startsWith('<') ? `Lectra below ${parts.lectra.slice(1)}%`
    : parts.lectra.endsWith('+') ? `Lectra ${parts.lectra.slice(0, -1)}% or more`
    : `Lectra ${parts.lectra.replace('-', '–')}%`;
  return `${pieces}, ${eff}`;
}
