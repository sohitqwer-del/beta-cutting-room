import type { Job, Pt } from '@beta/marker-wasm';

/** One garment's match as reported after nesting. Newer engines send this on `overlay.matches` (STYLE-API match report:
 * status, signed offsets and error in mm); older ones give pass/fail only (`adapter.matchStatus`, error NaN). */
export interface MarkerMatch {
  rule: number; garment: number | null; status?: 'matched' | 'off' | 'not_placed';
  a: Pt | null; b: Pt | null; error_mm: number | null; offset_mm?: [number | null, number | null]; pass: boolean;
}

/** One style match rule after nesting: matched, off (with the worst offset in mm when the engine reports it) or not placed.
 * Display only: pass/fail and offsets come from the Rust validator or the engine's match report, never from browser maths. */
export type RuleState = 'matched' | 'off' | 'not_placed';
export interface RuleReport {
  rule: number; state: RuleState;
  /** Garments checked, garments off and garments whose pieces are not in this marker. */
  checked: number; off: number; missing: number;
  /** Largest offset in mm among the garments that are off; null when the engine gives pass/fail only. */
  worstMm: number | null;
  /** Largest measured offset over all placed garments (matched ones too); null when not measured. */
  maxMm: number | null;
  /** Marker positions of the two points for each garment (a on piece A, b on piece B or null for a fabric-repeat rule). */
  marks: Array<{ a: Pt; b: Pt | null; pass: boolean; errorMm: number | null; size: string }>;
}
/** The engine expands each style rule once per size in the marker ratio, in order (`to_job`). */
export function styleRuleOf(job: Job | undefined, jobRule: number, styleRules: number): number {
  const total = job?.match_rules?.length ?? 0;
  if (!styleRules || !total) return jobRule;
  const perRule = Math.max(1, Math.round(total / styleRules));
  return Math.floor(jobRule / perRule);
}
const sizeOf = (job: Job | undefined, jobRule: number): string => {
  const id = job?.match_rules?.[jobRule]?.a.piece_id ?? '';
  return job?.pieces.find(p => p.id === id)?.size ?? id.slice(id.lastIndexOf('-') + 1);
};
export const placed = (m: MarkerMatch): m is MarkerMatch & { a: Pt } => m.status !== 'not_placed' && !!m.a;
export function ruleReports(styleRules: number, job: Job | undefined, statuses: MarkerMatch[]): RuleReport[] {
  return Array.from({ length: styleRules }, (_, rule) => {
    const all = statuses.filter(s => styleRuleOf(job, s.rule, styleRules) === rule), rows = all.filter(placed);
    const off = rows.filter(s => !s.pass), finite = off.map(s => s.error_mm).filter((v): v is number => v != null && Number.isFinite(v));
    return {
      rule, state: !rows.length ? 'not_placed' : off.length ? 'off' : 'matched', checked: rows.length, off: off.length, missing: all.length - rows.length,
      worstMm: finite.length ? Math.max(...finite) : null,
      maxMm: ((all: number[]) => all.length ? Math.max(...all) : null)(rows.map(s => s.error_mm).filter((v): v is number => v != null && Number.isFinite(v))),
      marks: rows.map(s => ({ a: s.a, b: s.b, pass: s.pass, errorMm: s.error_mm, size: sizeOf(job, s.rule) })),
    };
  });
}
/** Stripe lines drawn on the marker: keep every n-th line so neighbours stay at least `minPx` apart on screen. */
export function stripeStep(periodMm: number, mmPerPx: number, minPx = 5): number {
  if (!(periodMm > 0) || !(mmPerPx > 0)) return 1;
  return Math.max(1, Math.ceil(minPx * mmPerPx / periodMm));
}
