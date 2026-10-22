import type { BlockKind, Job, Layout, MarkerOverlay, MarkerResult, MatchRule, Nap, PatternDocument, RepeatBand, StyleJobSettings, ValidationReport } from '@beta/marker-wasm';
import type { LengthUnit } from './units';
import { widthProblem } from './units';

/** The Style flow: one guided job from a new style to a cut-ready marker.
 * Pure state and guards; the screen (ui.ts) renders it and the adapter calls the Rust engine. */
export type Step = 'style' | 'pattern' | 'grade' | 'lay' | 'gaps' | 'stripes' | 'marker' | 'cut';
export const STEPS: Step[] = ['style', 'pattern', 'grade', 'lay', 'gaps', 'stripes', 'marker', 'cut'];

export interface StyleInfo { name: string; number: string; garment: string; season: string; fabric: string; notes: string }
export type Face = 'up' | 'down' | 'face_to_face';
export interface LaySetup {
  layout: Layout; face: Face; nap: Nap; allowFlip: boolean;
  /** Flat spread width in mm (folded: the folded width). */
  widthMm: number; widthUnit: LengthUnit;
  plies: number; endAllowanceMm: number;
  /** Garments of each size in one marker. */
  ratio: Record<string, number>;
  /** Garments ordered per size (for the cut plan). */
  order: Record<string, number>;
  /** Fabric group for this marker (shell, lining, fusing …); null when the style uses one fabric. */
  material: string | null;
}
export interface GapSetup { pieceGap: number; edgeGap: number; buffers: Record<string, number>; blocks: Record<string, number | null> }
export type RepeatKind = 'none' | 'stripes' | 'checks';
export interface RepeatSetup {
  kind: RepeatKind;
  /** Stripes that run along the fabric length repeat across the width (Y); stripes across the width repeat along the length (X). */
  direction: 'along' | 'across';
  periodX: number; periodY: number; offsetX: number; offsetY: number; tolerance: number;
  bands: RepeatBand[]; matches: MatchRule[];
}
export interface MarkerState {
  status: 'idle' | 'running' | 'done' | 'failed';
  job?: Job; result?: MarkerResult; overlay?: MarkerOverlay; report?: ValidationReport;
  previous?: { job: Job; result: MarkerResult };
  /** Settings changed after this marker was made. */
  stale: boolean; error?: string;
}
export interface Session {
  step: Step; visited: Step[];
  info: StyleInfo; unit: LengthUnit;
  doc: PatternDocument | null; start: { kind: 'block'; block: BlockKind } | { kind: 'blank' } | { kind: 'open' } | null;
  lay: LaySetup; gaps: GapSetup; repeat: RepeatSetup; marker: MarkerState;
}

export function initialSession(): Session {
  return {
    step: 'style', visited: ['style'],
    info: { name: '', number: '', garment: '', season: '', fabric: '', notes: '' }, unit: 'cm',
    doc: null, start: null,
    lay: { layout: 'open', face: 'up', nap: 'none', allowFlip: false, widthMm: 1524, widthUnit: 'inch', plies: 40, endAllowanceMm: 20, ratio: {}, order: {}, material: null },
    gaps: { pieceGap: 0, edgeGap: 5, buffers: {}, blocks: {} },
    repeat: { kind: 'none', direction: 'across', periodX: 10, periodY: 10, offsetX: 0, offsetY: 0, tolerance: 1, bands: [], matches: [] },
    marker: { status: 'idle', stale: false },
  };
}

export type Blocker = 'no_name' | 'no_start' | 'no_piece' | 'lay_width' | 'lay_plies' | 'lay_ratio' | 'repeat' | 'no_marker' | 'busy';
/** Why a step cannot open yet, or null. Earlier steps are always open again. */
export function blocker(s: Session, step: Step): Blocker | null {
  if (s.marker.status === 'running' && step !== 'marker') return 'busy';
  const at = STEPS.indexOf(step);
  if (at >= 1) { if (!s.info.name.trim()) return 'no_name'; if (!s.doc) return 'no_start'; }
  if (at >= 2 && !(s.doc?.pieces.length)) return 'no_piece';
  if (at >= 6) { const lay = layProblem(s.lay); if (lay) return lay; if (repeatProblem(s.repeat)) return 'repeat'; }
  if (at >= 7 && (s.marker.status !== 'done' || !s.marker.result)) return 'no_marker';
  return null;
}
export const MAX_PLIES = 500;
/** Why a ply count cannot be used: not a whole number of 1 or more, more than a table takes, or odd on a lay that cuts
 * two layers together. */
export function pliesProblem(plies: number, lay: Pick<LaySetup, 'layout' | 'face'>): 'plies_whole' | 'plies_max' | 'plies_even_error' | null {
  if (!Number.isInteger(plies) || plies < 1) return 'plies_whole';
  if (plies > MAX_PLIES) return 'plies_max';
  if (pliesMustBeEven(lay) && plies % 2) return 'plies_even_error';
  return null;
}
export function layProblem(lay: LaySetup): 'lay_width' | 'lay_plies' | 'lay_ratio' | null {
  if (widthProblem(lay.widthMm)) return 'lay_width';
  if (pliesProblem(lay.plies, lay)) return 'lay_plies';
  if (!Object.values(lay.ratio).some(n => n > 0)) return 'lay_ratio';
  return null;
}
/** Folded and tubular lays and face-to-face pairs always cut two layers together. */
export function pliesMustBeEven(lay: Pick<LaySetup, 'layout' | 'face'>): boolean { return lay.layout !== 'open' || lay.face === 'face_to_face'; }
export function repeatProblem(r: RepeatSetup): 'period' | 'tolerance' | null {
  if (r.kind === 'none') return null;
  const needX = r.kind === 'checks' || r.direction === 'across', needY = r.kind === 'checks' || r.direction === 'along';
  if ((needX && !(r.periodX > 0)) || (needY && !(r.periodY > 0))) return 'period';
  if (!(r.tolerance >= 0)) return 'tolerance';
  return null;
}
export function go(s: Session, step: Step): Session {
  if (blocker(s, step)) return s;
  return { ...s, step, visited: s.visited.includes(step) ? s.visited : [...s.visited, step] };
}
export function nextStep(step: Step): Step | null { return STEPS[STEPS.indexOf(step) + 1] ?? null; }
export function prevStep(step: Step): Step | null { return STEPS[STEPS.indexOf(step) - 1] ?? null; }

/** Settings changes after a marker keep it visible but mark it out of date (and keep it as "previous"). */
function touched(s: Session): MarkerState { return s.marker.status === 'done' ? { ...s.marker, stale: true } : s.marker; }
export function setInfo(s: Session, patch: Partial<StyleInfo>): Session { return { ...s, info: { ...s.info, ...patch } }; }
export function setLay(s: Session, patch: Partial<LaySetup>): Session { return { ...s, lay: { ...s.lay, ...patch }, marker: touched(s) }; }
export function setGaps(s: Session, patch: Partial<GapSetup>): Session { return { ...s, gaps: { ...s.gaps, ...patch }, marker: touched(s) }; }
export function setRepeat(s: Session, patch: Partial<RepeatSetup>): Session { return { ...s, repeat: { ...s.repeat, ...patch }, marker: touched(s) }; }
/** A new pattern document: keep ratio and order in step with its sizes (new sizes start at 1 garment). */
export function setDoc(s: Session, doc: PatternDocument, start: Session['start'] = s.start): Session {
  const ratio: Record<string, number> = {}, order: Record<string, number> = {};
  for (const size of doc.sizes) { ratio[size] = s.lay.ratio[size] ?? 1; order[size] = s.lay.order[size] ?? 0; }
  const ids = new Set(doc.pieces.map(p => p.id)), groups = materials(doc);
  const material = groups.length > 1 ? (s.lay.material && groups.includes(s.lay.material) ? s.lay.material : groups[0]!) : null;
  const keep = <T>(r: Record<string, T>) => Object.fromEntries(Object.entries(r).filter(([k]) => ids.has(k)));
  return { ...s, doc, start, lay: { ...s.lay, ratio, order, material }, gaps: { ...s.gaps, buffers: keep(s.gaps.buffers), blocks: keep(s.gaps.blocks) }, marker: touched(s),
    repeat: { ...s.repeat, matches: s.repeat.matches.filter(m => ids.has(m.a.piece_id) && (!m.b || ids.has(m.b.piece_id))) } };
}
/** Fabric groups used by the pieces, shell first. A marker is made for one group at a time. */
export function materials(doc: PatternDocument | null): string[] {
  const set = new Set((doc?.pieces ?? []).map(p => p.material || 'shell'));
  return [...set].sort((a, b) => (a === 'shell' ? -1 : b === 'shell' ? 1 : a.localeCompare(b)));
}
export function garmentsPerMarker(lay: LaySetup): number { return Object.values(lay.ratio).reduce((a, b) => a + (b > 0 ? b : 0), 0); }

/** Everything the engine needs to build a marker job from the pattern. Units: mm. */
/** STYLE-API v1 settings plus the additive `material` filter (one fabric group per marker). */
export type JobSettings = StyleJobSettings & { material?: string };
export function jobSettings(s: Session): JobSettings {
  const r = s.repeat;
  const useX = r.kind === 'checks' || (r.kind === 'stripes' && r.direction === 'across');
  const useY = r.kind === 'checks' || (r.kind === 'stripes' && r.direction === 'along');
  const piece_settings: NonNullable<StyleJobSettings['piece_settings']> = {};
  for (const id of new Set([...Object.keys(s.gaps.buffers), ...Object.keys(s.gaps.blocks)])) {
    const buffer = s.gaps.buffers[id], block = s.gaps.blocks[id];
    if ((buffer ?? 0) > 0 || block != null) piece_settings[id] = { ...(buffer ? { buffer } : {}), ...(block != null ? { block_margin: block } : {}) };
  }
  return {
    fabric: {
      width: s.lay.widthMm, layout: s.lay.layout, nap: s.lay.nap, allow_flip: s.lay.allowFlip,
      repeat: r.kind === 'none' ? null : { period_x: useX ? r.periodX : null, period_y: useY ? r.periodY : null, offset_x: useX ? r.offsetX : 0, offset_y: useY ? r.offsetY : 0, tolerance_mm: r.tolerance },
    },
    spacing: { piece_gap: s.gaps.pieceGap, edge_gap: s.gaps.edgeGap },
    ratio: Object.fromEntries(Object.entries(s.lay.ratio).filter(([, n]) => n > 0)),
    lay: { face: s.lay.face, plies: s.lay.plies, end_allowance_mm: s.lay.endAllowanceMm },
    repeat_bands: r.kind === 'none' ? [] : r.bands,
    piece_settings,
    matches: r.kind === 'none' ? [] : r.matches,
    ...(s.lay.material ? { material: s.lay.material } : {}),
  };
}
/** Spread length per ply (cut extent plus waste at each end) and total fabric for the lay.
 * One meaning (marker/docs/PLIES.md): a ply is one fabric layer; a pass is one length laid from the roll.
 * Open: one ply per pass. Folded and tube: two plies per pass, so fabric = length per pass x passes. */
export function layFabric(markerLengthMm: number, lay: Pick<LaySetup, 'endAllowanceMm' | 'plies' | 'layout'>): { perPlyMm: number; totalMm: number; passes: number } {
  const perPlyMm = markerLengthMm + 2 * lay.endAllowanceMm;
  const passes = lay.layout === 'open' ? lay.plies : Math.ceil(lay.plies / 2);
  return { perPlyMm, totalMm: perPlyMm * passes, passes };
}
/** What "Make a cut plan" puts in the Cut plan form: garments per size (in `sizes` order), max plies and end allowance.
 * Cut order v2 counts max plies in plies (layers); the planner halves them into passes on folded/tube lays itself. */
export function cutPlanHandoff(lay: Pick<LaySetup, 'plies' | 'endAllowanceMm' | 'order'>, sizes: readonly string[]): { quantities: number[]; maxPlies: number; endAllowanceMm: number } {
  return { quantities: sizes.map(s => lay.order[s] ?? 0), maxPlies: lay.plies, endAllowanceMm: lay.endAllowanceMm };
}

const KEY = 'beta-style-v1';
/** Keep the session on this computer between visits (browser storage). Markers are not stored. */
export function save(s: Session, storage: Pick<Storage, 'setItem'> | undefined = globalThis.localStorage): boolean {
  try { storage?.setItem(KEY, JSON.stringify({ ...s, marker: { status: 'idle', stale: false } })); return true; } catch { return false; }
}
export function restore(storage: Pick<Storage, 'getItem'> | undefined = globalThis.localStorage): Session | null {
  try {
    const raw = storage?.getItem(KEY); if (!raw) return null;
    const v = JSON.parse(raw) as Partial<Session>, base = initialSession();
    if (!v || typeof v !== 'object' || !STEPS.includes(v.step as Step)) return null;
    if (v.doc && (v.doc.format !== 'beta-pattern.doc/v1' || !Array.isArray(v.doc.pieces))) return null;
    const s: Session = { ...base, ...v, info: { ...base.info, ...v.info }, lay: { ...base.lay, ...v.lay }, gaps: { ...base.gaps, ...v.gaps }, repeat: { ...base.repeat, ...v.repeat }, marker: { status: 'idle', stale: false } } as Session;
    // The marker is not stored, so the last two steps reopen at Marker.
    if (s.step === 'cut') s.step = 'marker';
    if (blocker(s, s.step)) s.step = 'style';
    return s;
  } catch { return null; }
}
export function clearSaved(storage: Pick<Storage, 'removeItem'> | undefined = globalThis.localStorage): void { try { storage?.removeItem(KEY); } catch { /* nothing saved */ } }
