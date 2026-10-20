import type { CutOrder, CutPlan, Job, MarkerResult, NestOptions, PlanOptions, Pt, Zone } from '@beta/marker-wasm';
import type { DocumentState } from './state';

/** Copy locks exactly; never let a later editor change mutate a submitted request. */
export function nestPayload(state: DocumentState, options: NestOptions, unlocked = false, warm = false): { job: Job; options: NestOptions } {
  return structuredClone({ job: state.job, options: { ...options,
    fixed: unlocked ? state.result.placements.filter(p => p.locked) : [],
    warm_start: warm && state.result.placements.length ? state.result : null } });
}
export interface PlanForm {
  quantities: Record<string, number>; maxPlies: number; maxGarments: number; overcut: number;
  time: number; finalTime?: number; endAllowance: number; layCost: number;
}
/** Validate operator input before asking Rust to compute a plan. */
export function planErrors(form: PlanForm): string[] {
  const errors: string[] = [];
  const whole = (n: number): boolean => Number.isInteger(n) && n >= 0 && n <= 4294967295;
  if (!Object.values(form.quantities).length || Object.values(form.quantities).some(n => !whole(n)) || !Object.values(form.quantities).some(n => n > 0)) errors.push('Enter a whole garment quantity for at least one size.');
  if (!whole(form.maxPlies) || form.maxPlies < 1) errors.push('Max plies must be a positive whole number.');
  if (!whole(form.maxGarments) || form.maxGarments < 1) errors.push('Max garments per marker must be a positive whole number.');
  for (const [label, n] of [['Over-cut', form.overcut], ['End allowance', form.endAllowance], ['Lay cost', form.layCost]] as const) if (!Number.isFinite(n) || n < 0) errors.push(`${label} must be zero or more.`);
  if (!Number.isFinite(form.time) || form.time <= 0) errors.push('Time per marker must be greater than zero.');
  if (!Number.isFinite(form.finalTime ?? 0) || (form.finalTime ?? 0) < 0 || (form.finalTime ?? 0) > 86400) errors.push('Final search time must be between 0 and 86400 seconds.');
  return errors;
}
export function planPayload(job: Job, form: PlanForm): { order: CutOrder; options: PlanOptions } {
  const errors = planErrors(form); if (errors.length) throw new Error(errors.join(' '));
  return structuredClone({ order: { format: 'beta-marker.cutorder/v1', name: job.name, job, quantities: form.quantities,
    max_plies: form.maxPlies, max_garments_per_marker: form.maxGarments, overcut_pct: form.overcut },
  options: { time_per_marker_s: form.time, final_time_s: form.finalTime ?? 0, end_allowance_mm: form.endAllowance } });
}
/** A saved partial result is useful evidence, but it is not an order ready to cut. */
export function planStatus(plan: Pick<CutPlan, 'complete' | 'shortfall'>): string {
  if (plan.complete) return 'Plan ready. Review each marker before cutting.';
  const missing = Object.entries(plan.shortfall).map(([size, quantity]) => `${size}: ${quantity}`).join(', ');
  return `Incomplete plan.${missing ? ` Missing garments — ${missing}.` : ''} Do not cut this as a complete order. Review the warnings and try other limits or more search time.`;
}
/** Savings may be negative. A zero or invalid reference has no defined saving. */
export function compareLengths(theirs: number, ours: number): { metres: number; percent: number } | null {
  if (!Number.isFinite(theirs) || !Number.isFinite(ours) || theirs <= 0 || ours <= 0) return null;
  return { metres: (theirs - ours) / 1000, percent: (1 - ours / theirs) * 100 };
}
/** Drawing coordinates stay within the fabric and the chosen drawing length. */
export function clampDefectPoint(p: Pt, length: number, width: number): Pt {
  return [Math.max(0, Math.min(length, p[0])), Math.max(0, Math.min(width, p[1]))];
}
export function rectangleDefect(a: Pt, b: Pt, length: number, width: number): Pt[] {
  const p = clampDefectPoint(a, length, width), q = clampDefectPoint(b, length, width);
  if (Math.abs(p[0] - q[0]) < 0.01 || Math.abs(p[1] - q[1]) < 0.01) return [];
  return [[Math.min(p[0], q[0]), Math.min(p[1], q[1])], [Math.max(p[0], q[0]), Math.min(p[1], q[1])],
    [Math.max(p[0], q[0]), Math.max(p[1], q[1])], [Math.min(p[0], q[0]), Math.max(p[1], q[1])]];
}
/** Only a drawing input check; Rust remains responsible for geometric validation. */
export function defectZone(points: Pt[], id: string): Zone {
  if (points.length < 3 || points.some(p => p.length !== 2 || p.some(n => !Number.isFinite(n)))) throw new Error('Draw at least three finite points.');
  if (new Set(points.map(p => p.join(','))).size < 3) throw new Error('A defect needs three different corners.');
  return { id, polygon: structuredClone(points), note: null };
}
export const issueHelp: Record<string, string> = {
  defect: 'A piece crosses a fabric flaw. Move it clear of the marked area.',
  defect_overlap: 'A piece crosses a fabric flaw. Move it clear of the marked area.',
  shade: 'Keep all pieces of one garment within the same shade band.',
  shade_band: 'Keep all pieces of one garment within the same shade band.',
  match: 'The named seam points do not line up with the stripe or check repeat.',
  repeat: 'Check the stripe or check repeat period and tolerance.',
  match_rule: 'Check the named points and the fabric repeat settings.',
  stripe_match: 'The named seam points do not line up with the fabric repeat.',
  fold: 'Place the symmetry line on a fold edge of folded or tubular fabric.',
  on_fold: 'Place the symmetry line on a fold edge of folded or tubular fabric.',
  fold_line: 'Set a valid symmetry line before placing this piece on a fold.',
  bundle: 'Assign each piece to its garment bundle.',
  bundle_nap: 'All pieces in this garment must face the same way.',
  nap: 'Check the direction of the pieces against the fabric nap rule.',
  block: 'Keep the full fusing rectangle and its margin clear.',
  shrinkage: 'Check the length and width shrinkage percentages.',
  surplus_piece: 'Extra copies come from cutting through both fabric layers.',
};
export function issueExplanation(code: string): string {
  return issueHelp[code] ?? (code.includes('shade') ? issueHelp.shade! : code.includes('match') ? issueHelp.match! : code.includes('fold') ? issueHelp.fold! : code.includes('bundle') ? issueHelp.bundle! : '');
}
export interface Comparison { job: Job; theirs: MarkerResult; ours?: MarkerResult }
