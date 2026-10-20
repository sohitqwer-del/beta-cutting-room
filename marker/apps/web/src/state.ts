import type { Job, MarkerResult, Placement } from '@beta/marker-wasm';
/** Editable document. View position and selection are deliberately outside undo. */
export interface DocumentState { job: Job; result: MarkerResult }
/** Bounded, immutable snapshot history. One drag is one operation. */
export class History<T> {
  private past: T[] = [];
  private future: T[] = [];
  constructor(public current: T, private limit = 100) { this.current = structuredClone(current); }
  get canUndo(): boolean { return this.past.length > 0; }
  get canRedo(): boolean { return this.future.length > 0; }
  commit(next: T): T {
    if (JSON.stringify(next) === JSON.stringify(this.current)) return this.current;
    this.past.push(structuredClone(this.current));
    if (this.past.length > this.limit) this.past.shift();
    this.current = structuredClone(next); this.future = []; return this.current;
  }
  undo(): T {
    const previous = this.past.pop();
    if (previous !== undefined) { this.future.push(structuredClone(this.current)); this.current = previous; }
    return structuredClone(this.current);
  }
  redo(): T {
    const next = this.future.pop();
    if (next !== undefined) { this.past.push(structuredClone(this.current)); this.current = next; }
    return structuredClone(this.current);
  }
}
/** Change a placement; locked placements only accept a lock toggle. */
export function changePlacement(state: DocumentState, index: number, patch: Partial<Placement>): DocumentState {
  const placement = state.result.placements[index];
  if (!placement || (placement.locked && Object.keys(patch).some(key => key !== 'locked'))) return state;
  const next = structuredClone(state);
  next.result.placements[index] = { ...placement, ...patch };
  next.result.engine = 'manual'; return next;
}
/** Start a job with no placements. */
export function emptyResult(job: Job): MarkerResult {
  return { format: 'beta-marker.result/v1', job_name: job.name, width: job.fabric.width, length: 0,
    piece_area: 0, efficiency: 0, engine: 'manual', placements: [], warnings: [] };
}
/** Parse enough of the wire shape to keep malformed files out of the viewer. Rust checks garment rules. */
export function readDocument(text: string): Job | MarkerResult {
  const value = JSON.parse(text) as Record<string, unknown>;
  if (!value || typeof value !== 'object') throw new Error('Choose a BETA Marker job or result JSON file.');
  const finite = (n: unknown): n is number => typeof n === 'number' && Number.isFinite(n);
  const point = (p: unknown): boolean => Array.isArray(p) && p.length === 2 && p.every(finite);
  const optional = (v: unknown, check: (v: unknown) => boolean): boolean => v === undefined || check(v);
  const nullableText = (v: unknown): boolean => v === undefined || v === null || typeof v === 'string';
  const count = (v: unknown): boolean => finite(v) && Number.isInteger(v) && v >= 0 && v <= 4294967295;
  const object = (v: unknown): v is Record<string, unknown> => v !== null && typeof v === 'object' && !Array.isArray(v);
  const strings = (v: unknown): boolean => Array.isArray(v) && v.every(s => typeof s === 'string');
  if (value.format === 'beta-marker.job/v1') {
    const job = value as unknown as Job;
    if (typeof job.name !== 'string' || !job.fabric || !finite(job.fabric.width) || job.fabric.width <= 0 || !Array.isArray(job.pieces)) throw new Error('Job needs a name, positive fabric width and pieces.');
    if (job.pieces.some(p => !p || typeof p.id !== 'string' || typeof p.name !== 'string' || !Array.isArray(p.outline) || p.outline.length < 3 || !p.outline.every(point))) throw new Error('Every piece needs an id, name and at least three finite outline points.');
    if (!optional(job.fabric.nap, v => ['none', 'one_way', 'one_way_per_bundle'].includes(String(v))) || !optional(job.fabric.tilt_deg, finite) || !optional(job.fabric.allow_flip, v => typeof v === 'boolean') || !(job.fabric.max_length == null || finite(job.fabric.max_length))) throw new Error('Invalid fabric rules.');
    const fabric = job.fabric;
    if (!optional(fabric.layout, v => ['open', 'folded', 'tubular'].includes(String(v)))
      || !optional(fabric.defects, v => Array.isArray(v) && v.every(z => object(z) && optional(z.id, v => typeof v === 'string') && nullableText(z.note) && Array.isArray(z.polygon) && z.polygon.length >= 3 && z.polygon.every(point)))
      || !(fabric.shade == null || object(fabric.shade) && ['length', 'width'].includes(String(fabric.shade.axis)) && finite(fabric.shade.band_mm) && fabric.shade.band_mm > 0)
      || !(fabric.repeat == null || object(fabric.repeat) && ['period_x', 'period_y'].every(k => fabric.repeat![k as 'period_x'] == null || finite(fabric.repeat![k as 'period_x']) && fabric.repeat![k as 'period_x']! > 0) && ['offset_x', 'offset_y', 'tolerance_mm'].every(k => optional(fabric.repeat![k as 'offset_x'], finite)))
      || !optional(fabric.shrinkage, v => object(v) && optional(v.length_pct, finite) && optional(v.width_pct, finite))) throw new Error('Invalid fabric layout, defect, shade, repeat or shrinkage.');
    const ref = (v: unknown): boolean => object(v) && typeof v.piece_id === 'string' && typeof v.point_id === 'string';
    if (!optional(job.match_rules, v => Array.isArray(v) && v.every(r => object(r) && ref(r.a) && (r.b == null || ref(r.b)) && ['x', 'y', 'both'].includes(String(r.axes)) && optional(r.offset, point)))) throw new Error('Invalid match rules.');
    if (job.pieces.some(p => !(p.fold_line == null || Array.isArray(p.fold_line) && p.fold_line.length === 2 && p.fold_line.every(point))
      || !(p.block_margin == null || finite(p.block_margin) && p.block_margin >= 0)
      || !optional(p.match_points, v => Array.isArray(v) && v.every(m => object(m) && typeof m.id === 'string' && point(m.point))))) throw new Error('Invalid fold line, block margin or match points.');
    if (job.spacing !== undefined && (!object(job.spacing) || !optional(job.spacing.piece_gap, finite) || !optional(job.spacing.edge_gap, finite))) throw new Error('Gaps must be finite numbers.');
    if (job.order != null && (!object(job.order) || !object(job.order.sizes) || !Object.values(job.order.sizes).every(count))) throw new Error('Size counts must be whole numbers from 0 to 4294967295.');
    if (job.pieces.some(p => !nullableText(p.size) || !nullableText(p.style) || !nullableText(p.material) || !optional(p.quantity, count) || !optional(p.mirror_quantity, count) || !optional(p.buffer, finite) || !optional(p.rotation, v => ['inherit', 'fixed', 'flip180', 'quarter', 'free'].includes(String(v))) || !(p.grain == null || Array.isArray(p.grain) && p.grain.length === 2 && p.grain.every(point)) || !optional(p.marks, v => Array.isArray(v) && v.every(m => object(m) && ['notch', 'drill', 'line', 'text'].includes(String(m.kind)) && Array.isArray(m.points) && m.points.every(point) && nullableText(m.text))))) throw new Error('Invalid piece quantities, rules, grain or marks.');
    return job;
  }
  if (value.format === 'beta-marker.result/v1') {
    const result = value as unknown as MarkerResult;
    if (Array.isArray(result.placements) && result.placements.some(p => !(p.bundle == null || count(p.bundle)) || !optional(p.on_fold, v => typeof v === 'boolean'))) throw new Error('Invalid bundle or fold placement.');
    if (typeof result.job_name !== 'string' || !Array.isArray(result.placements) || !finite(result.width) || result.width <= 0 || !finite(result.length) || !finite(result.efficiency) || !finite(result.piece_area)) throw new Error('Result needs dimensions, metrics and placements.');
    if (result.placements.some(p => !p || typeof p.piece_id !== 'string' || !point(p.translate) || !finite(p.rotation_deg) || typeof p.mirrored !== 'boolean' || !count(p.copy) || !optional(p.locked, v => typeof v === 'boolean'))) throw new Error('Result has an invalid placement.');
    if (typeof result.engine !== 'string' || !optional(result.runtime_s, finite) || !(result.seed == null || Number.isSafeInteger(result.seed) && result.seed >= 0) || !optional(result.warnings, strings)) throw new Error('Invalid result metadata. Seeds must be safe JavaScript integers.');
    return result;
  }
  throw new Error('Expected beta-marker.job/v1 or beta-marker.result/v1.');
}
