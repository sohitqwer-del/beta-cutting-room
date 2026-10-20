import type { Job, Layout, MarkerResult, Pt } from '@beta/marker-wasm';
import type { QuickPiece, QuickScan, QuickSession, QuickWarning } from '@beta/marker-wasm';
import { longAxis, turnLine } from './geometry';
import { widthProblem, type WidthUnit } from './units';

/** Quick Marker flow: plain data and pure functions so every rule is testable without a browser.
 * Rule: nothing from a photo is used until the user confirms it (accept, or tap a suggestion). */
export type Step = 'start' | 'capture' | 'review' | 'order' | 'nest' | 'use';
export const STEPS: Step[] = ['start', 'capture', 'review', 'order', 'nest', 'use'];
export type Reference = 'a4-sheet' | 'beta-target';

export interface Photo { id: string; name: string; size: string; status: 'reading' | 'done' | 'failed'; error?: string; scan?: QuickScan }
export type PieceStatus = 'pending' | 'accepted' | 'deleted';
export interface ReviewPiece { piece: QuickPiece; status: PieceStatus; dismissed: SuggestionKind[]; removedAsPairOf?: string; removedAsDuplicateOf?: string }
export interface NestState {
  status: 'idle' | 'running' | 'done' | 'stopped' | 'failed';
  budgetS: number; elapsedS: number; bestLengthMm: number | null; bestEfficiency: number | null;
  job?: Job; result?: MarkerResult; error?: string;
}
export interface QuickState {
  step: Step; reference: Reference;
  photos: Photo[]; pieces: ReviewPiece[]; duplicates: QuickSession['duplicates']; warnings: QuickWarning[];
  selected: string | null;
  defaultSize: string;
  fabric: { widthMm: number | null; unit: WidthUnit; nap: boolean; layout: Layout };
  garments: Record<string, number>;
  nest: NestState;
  usualM: number | null;
}

export const SIZE_PRESETS = ['FREE', 'S', 'M', 'L', 'XL', 'XXL'];
export function initialState(): QuickState {
  return {
    step: 'start', reference: 'a4-sheet', photos: [], pieces: [], duplicates: [], warnings: [], selected: null,
    defaultSize: 'FREE',
    fabric: { widthMm: 44 * 25.4, unit: 'inch', nap: false, layout: 'open' },
    garments: {},
    nest: { status: 'idle', budgetS: 30, elapsedS: 0, bestLengthMm: null, bestEfficiency: null },
    usualM: null,
  };
}

// ---------- Steps and guards ----------
export type Blocker = 'no_photo' | 'reading' | 'no_pieces' | 'pending' | 'width' | 'no_garments' | 'size_without_pieces' | 'no_result';
export function accepted(state: QuickState): ReviewPiece[] { return state.pieces.filter(p => p.status === 'accepted'); }
export function pending(state: QuickState): ReviewPiece[] { return state.pieces.filter(p => p.status === 'pending'); }
/** Sizes that have accepted pieces, in a stable shop order (FREE, S, M, L, XL, XXL, then others A–Z). */
export function orderSizes(state: QuickState): string[] {
  const sizes = [...new Set(accepted(state).map(p => p.piece.size ?? state.defaultSize))];
  const rank = (s: string) => { const i = SIZE_PRESETS.indexOf(s); return i < 0 ? 100 : i; };
  return sizes.sort((a, b) => rank(a) - rank(b) || a.localeCompare(b));
}
/** Garments to nest: only sizes with pieces and a count above zero. */
export function garmentsFor(state: QuickState): Record<string, number> {
  const out: Record<string, number> = {};
  for (const size of orderSizes(state)) { const n = state.garments[size] ?? 0; if (n > 0) out[size] = n; }
  return out;
}
export function totalGarments(state: QuickState): number { return Object.values(garmentsFor(state)).reduce((a, b) => a + b, 0); }

/** Why a step cannot open yet, or null when it can. */
export function blocker(state: QuickState, step: Step): Blocker | null {
  if (step === 'start' || step === 'capture') return null;
  const done = state.photos.filter(p => p.status === 'done');
  if (!done.length) return state.photos.some(p => p.status === 'reading') ? 'reading' : 'no_photo';
  if (step === 'review') return null;
  if (!accepted(state).length) return 'no_pieces';
  if (pending(state).length) return 'pending';
  if (step === 'order') return null;
  if (widthProblem(state.fabric.widthMm)) return 'width';
  if (!totalGarments(state)) return 'no_garments';
  if (step === 'nest') return null;
  return state.nest.result ? null : 'no_result';
}
export function go(state: QuickState, step: Step): QuickState {
  return blocker(state, step) ? state : { ...state, step };
}
export function nextStep(step: Step): Step | null { const i = STEPS.indexOf(step); return STEPS[i + 1] ?? null; }
export function previousStep(step: Step): Step | null { const i = STEPS.indexOf(step); return i > 0 ? STEPS[i - 1]! : null; }

// ---------- Photos and scans ----------
export function addPhoto(state: QuickState, id: string, name: string): QuickState {
  return { ...state, photos: [...state.photos, { id, name, size: state.defaultSize, status: 'reading' }] };
}
export function photoFailed(state: QuickState, id: string, error: string): QuickState {
  return { ...state, photos: state.photos.map(p => p.id === id ? { ...p, status: 'failed', error } : p) };
}
export function photoScanned(state: QuickState, id: string, scan: QuickScan): QuickState {
  return { ...state, photos: state.photos.map(p => p.id === id ? { ...p, status: 'done', scan } : p) };
}
export function removePhoto(state: QuickState, id: string): QuickState {
  return { ...state, photos: state.photos.filter(p => p.id !== id), pieces: state.pieces.filter(p => p.piece.photoId !== id), selected: null };
}
/** Fold a merged session into the review list. Pieces the user already reviewed keep their edits and status. */
export function applySession(state: QuickState, session: QuickSession): QuickState {
  const old = new Map(state.pieces.map(p => [p.piece.id, p]));
  const sizeOf = new Map(state.photos.map(p => [p.id, p.size]));
  const pieces = session.pieces.map(piece => old.get(piece.id) ?? { piece: { ...piece, size: piece.size ?? sizeOf.get(piece.photoId) ?? state.defaultSize }, status: 'pending' as const, dismissed: [] });
  return { ...state, pieces, duplicates: session.duplicates, warnings: session.warnings };
}
/** Changing the size of a photo changes its pieces that the user has not set one by one. */
export function setPhotoSize(state: QuickState, id: string, size: string): QuickState {
  const clean = cleanSize(size); if (!clean) return state;
  const before = state.photos.find(p => p.id === id)?.size;
  return {
    ...state, defaultSize: clean,
    photos: state.photos.map(p => p.id === id ? { ...p, size: clean } : p),
    pieces: state.pieces.map(p => p.piece.photoId === id && (p.piece.size ?? before) === before ? { ...p, piece: { ...p.piece, size: clean } } : p),
  };
}
export function cleanSize(size: string): string { return size.trim().toUpperCase().replace(/\s+/g, ' ').slice(0, 12); }

// ---------- Piece review ----------
function edit(state: QuickState, id: string, change: (p: ReviewPiece) => ReviewPiece): QuickState {
  return { ...state, pieces: state.pieces.map(p => p.piece.id === id ? change(p) : p) };
}
const editPiece = (state: QuickState, id: string, patch: Partial<QuickPiece>) => edit(state, id, p => ({ ...p, piece: { ...p.piece, ...patch } }));
export function select(state: QuickState, id: string | null): QuickState { return { ...state, selected: id }; }
/** The grain arrow the user sees: confirmed, else Rust's suggestion, else the long axis. Dashed until confirmed. */
export function shownGrain(piece: QuickPiece): [Pt, Pt] { return piece.grain ?? piece.suggestions.grain?.value ?? longAxis(piece.outline); }
/** Accepting a piece also confirms the grain arrow shown on it (the engine needs a confirmed grain on every piece). */
const confirm = (p: ReviewPiece): ReviewPiece => ({ ...p, status: 'accepted', piece: { ...p.piece, grain: shownGrain(p.piece) } });
export function accept(state: QuickState, id: string): QuickState { return edit(state, id, confirm); }
export function remove(state: QuickState, id: string): QuickState { return edit(state, id, p => ({ ...p, status: 'deleted' })); }
export function restore(state: QuickState, id: string): QuickState { return edit(state, id, p => ({ ...p, status: 'pending', removedAsPairOf: undefined, removedAsDuplicateOf: undefined })); }
/** An explicit user action: accept every piece still waiting. Deleted pieces stay deleted. */
export function acceptAllPending(state: QuickState): QuickState {
  return { ...state, pieces: state.pieces.map(p => p.status === 'pending' ? confirm(p) : p) };
}
export function rename(state: QuickState, id: string, name: string): QuickState {
  const clean = name.trim().slice(0, 40); return clean ? editPiece(state, id, { name: clean }) : state;
}
export function setSize(state: QuickState, id: string, size: string): QuickState {
  const clean = cleanSize(size); return clean ? editPiece(state, id, { size: clean }) : state;
}
/** Copies per garment: normal and mirrored (left/right). At least one copy in total. */
export function setQuantity(state: QuickState, id: string, normal: number, mirrored: number): QuickState {
  const n = Math.max(0, Math.min(20, Math.round(normal))), m = Math.max(0, Math.min(20, Math.round(mirrored)));
  return n + m < 1 ? state : editPiece(state, id, { quantity: n, mirror_quantity: m });
}
/** Pair on: the copies are split into left and right. Pair off: all copies are the same side. */
export function setPair(state: QuickState, id: string, pair: boolean): QuickState {
  const p = state.pieces.find(r => r.piece.id === id)?.piece; if (!p) return state;
  const total = Math.max(1, (p.quantity ?? 1) + (p.mirror_quantity ?? 0));
  if (pair) { const half = Math.max(1, Math.floor(total / 2)); return setQuantity(state, id, half, half); }
  return setQuantity(state, id, total, 0);
}
export function isPair(piece: QuickPiece): boolean { return (piece.mirror_quantity ?? 0) > 0; }
export function copies(piece: QuickPiece): number { return (piece.quantity ?? 1) + (piece.mirror_quantity ?? 0); }
/** Cut on fold uses the symmetry axis Rust found. Without one, fold cannot be switched on here. */
export function setFold(state: QuickState, id: string, on: boolean): QuickState {
  const p = state.pieces.find(r => r.piece.id === id)?.piece; if (!p) return state;
  if (!on) return editPiece(state, id, { fold_line: null });
  const axis = p.suggestions.symmetryAxis?.value; return axis ? editPiece(state, id, { fold_line: axis }) : state;
}
export function canFold(piece: QuickPiece): boolean { return !!piece.suggestions.symmetryAxis; }
export function turnGrain(state: QuickState, id: string): QuickState {
  const p = state.pieces.find(r => r.piece.id === id)?.piece; if (!p) return state;
  return editPiece(state, id, { grain: turnLine(shownGrain(p)) });
}
export function replacePiece(state: QuickState, piece: QuickPiece): QuickState { return edit(state, piece.id, p => ({ ...p, piece: { ...piece, size: p.piece.size, name: p.piece.name } })); }

// ---------- Suggestions (chips the user taps) ----------
export type SuggestionKind = 'name' | 'pair' | 'quantity' | 'fold' | 'grain' | 'duplicate';
export interface Chip { kind: SuggestionKind; value: string; confidence: 'high' | 'medium' | 'low'; reason: string; other?: string }
export function chips(state: QuickState, id: string): Chip[] {
  const r = state.pieces.find(p => p.piece.id === id); if (!r || r.status === 'deleted') return [];
  const p = r.piece, s = p.suggestions, out: Chip[] = [];
  const open = (k: SuggestionKind) => !r.dismissed.includes(k);
  if (s.name && open('name') && s.name.value !== p.name) out.push({ kind: 'name', value: s.name.value, confidence: s.name.confidence, reason: s.name.reason });
  const mate = s.pairPieceId ? state.pieces.find(q => q.piece.id === s.pairPieceId) : undefined;
  if (s.pair === 'mirror' && mate && mate.status !== 'deleted' && open('pair')) out.push({ kind: 'pair', value: 'mirror', confidence: s.quantity?.confidence ?? 'medium', reason: 'Two pieces look like mirror images.', other: mate.piece.id });
  const q = s.quantity?.value;
  if (q && open('quantity') && !(s.pair === 'mirror' && mate && mate.status !== 'deleted') && (q.normal !== (p.quantity ?? 1) || q.mirrored !== (p.mirror_quantity ?? 0)))
    out.push({ kind: 'quantity', value: `${q.normal}+${q.mirrored}`, confidence: s.quantity!.confidence, reason: s.quantity!.reason });
  if (s.cutOnFold?.value && s.symmetryAxis && !p.fold_line && open('fold')) out.push({ kind: 'fold', value: 'fold', confidence: s.cutOnFold.confidence, reason: s.cutOnFold.reason });
  if (s.grain && !p.grain && open('grain')) out.push({ kind: 'grain', value: 'grain', confidence: s.grain.confidence, reason: s.grain.reason });
  const dup = state.duplicates.find(d => d.pieceId === id);
  const original = dup ? state.pieces.find(q2 => q2.piece.id === dup.duplicateOf) : undefined;
  if (dup && original && original.status !== 'deleted' && open('duplicate')) out.push({ kind: 'duplicate', value: 'duplicate', confidence: dup.confidence, reason: 'Same shape and size as a piece in another photo.', other: dup.duplicateOf });
  return out;
}
/** Tap on a chip: apply exactly what it says. */
export function applyChip(state: QuickState, id: string, chip: Chip): QuickState {
  const r = state.pieces.find(p => p.piece.id === id); if (!r) return state;
  const s = r.piece.suggestions;
  let next = state;
  switch (chip.kind) {
    case 'name': next = rename(state, id, chip.value); break;
    case 'pair':
      // Keep this piece as the one pattern; it is cut once normal and once mirrored. Its mate leaves the job.
      next = setQuantity(state, id, 1, 1);
      if (chip.other) next = edit(next, chip.other, p => ({ ...p, status: 'deleted', removedAsPairOf: id }));
      break;
    case 'quantity': if (s.quantity) next = setQuantity(state, id, s.quantity.value.normal, s.quantity.value.mirrored); break;
    case 'fold': next = setFold(state, id, true); break;
    case 'grain': if (s.grain) next = editPiece(state, id, { grain: s.grain.value as [Pt, Pt] }); break;
    case 'duplicate': next = edit(state, id, p => ({ ...p, status: 'deleted', removedAsDuplicateOf: chip.other })); break;
  }
  return edit(next, id, p => ({ ...p, dismissed: [...new Set([...p.dismissed, chip.kind])] }));
}
export function dismissChip(state: QuickState, id: string, kind: SuggestionKind): QuickState {
  return edit(state, id, p => ({ ...p, dismissed: [...new Set([...p.dismissed, kind])] }));
}

// ---------- Fabric and order ----------
export function setWidth(state: QuickState, widthMm: number | null): QuickState { return { ...state, fabric: { ...state.fabric, widthMm } }; }
export function setUnit(state: QuickState, unit: WidthUnit): QuickState { return { ...state, fabric: { ...state.fabric, unit } }; }
export function setNap(state: QuickState, nap: boolean): QuickState { return { ...state, fabric: { ...state.fabric, nap } }; }
export function setLayout(state: QuickState, layout: Layout): QuickState { return { ...state, fabric: { ...state.fabric, layout } }; }
export function setGarments(state: QuickState, size: string, count: number): QuickState {
  return { ...state, garments: { ...state.garments, [size]: Math.max(0, Math.min(99, Math.round(count))) } };
}
/** First visit to the order step: one garment of each size, unless the user already chose. */
export function ensureGarments(state: QuickState): QuickState {
  const garments = { ...state.garments };
  for (const size of orderSizes(state)) if (garments[size] == null) garments[size] = 1;
  return { ...state, garments };
}
/** Pieces of a size that is not ordered are left out of the job (the engine rejects unknown sizes). */
export function jobPieces(state: QuickState): QuickPiece[] {
  const sizes = garmentsFor(state);
  return accepted(state).map(p => p.piece).filter(p => (sizes[p.size ?? ''] ?? 0) > 0);
}

// ---------- Nesting ----------
export function nestStarted(state: QuickState, budgetS: number): QuickState {
  return { ...state, step: 'nest', nest: { status: 'running', budgetS, elapsedS: 0, bestLengthMm: null, bestEfficiency: null } };
}
export function nestProgress(state: QuickState, elapsedS: number, bestLengthMm: number | null, bestEfficiency: number | null, job?: Job, result?: MarkerResult): QuickState {
  const n = state.nest;
  return { ...state, nest: { ...n, elapsedS, bestLengthMm: bestLengthMm ?? n.bestLengthMm, bestEfficiency: bestEfficiency ?? n.bestEfficiency, job: job ?? n.job, result: result ?? n.result } };
}
export function nestFinished(state: QuickState, stopped: boolean): QuickState {
  return { ...state, nest: { ...state.nest, status: state.nest.result ? (stopped ? 'stopped' : 'done') : 'failed', error: state.nest.result ? undefined : state.nest.error } };
}
export function nestFailed(state: QuickState, error: string): QuickState { return { ...state, nest: { ...state.nest, status: state.nest.result ? 'stopped' : 'failed', error } }; }
/** Editing pieces or the order makes an old marker out of date. */
export function clearNest(state: QuickState): QuickState {
  return { ...state, nest: { status: 'idle', budgetS: state.nest.budgetS, elapsedS: 0, bestLengthMm: null, bestEfficiency: null } };
}
