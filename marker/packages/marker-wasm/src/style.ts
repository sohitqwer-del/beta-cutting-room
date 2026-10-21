/** From scratch to cut. Rust owns validation and geometry; edits return snapshots. */
import type { DrawingObject, PatternDocument, PatternPiece, PatternValue, Annotation, GradeRule, PatternGrade } from './pattern';
import type { Fabric, Job, Json, MarkerResult, MatchRule, MatchReport, MatchSummary, Piece, Pt, Spacing } from './types';

export type BlockKind = 't_shirt' | 'shirt' | 'trouser' | 'a_line_skirt' | 'kurta' | 'polo' | 'track_pant';
export interface SizeChart {
  name: string; sizes: string[]; base_size: string;
  /** Size -> measurement name -> millimetres. These are editable example charts. */
  measurements: Record<string, Record<string, number>>;
}
export interface BlockEase {
  chest?: number; waist?: number; hip?: number;
  /** Sleeve cap length minus total armhole length, mm. */
  sleeve_cap?: number;
  seam_allowance?: number; hem_allowance?: number;
}
export interface BlockInfo { id: BlockKind; name: string; pieces: string[]; note: string }
export interface CreateStyle { name?: string; block?: BlockKind; chart?: SizeChart; ease?: BlockEase }
export interface SeamRun { piece: string; edges: number[]; reversed?: boolean }
export interface SeamCheck { name: string; a: SeamRun; b: SeamRun; ease_mm: number; tolerance_mm: number }
export interface SeamReport {
  check: SeamCheck; sampling_mm: number;
  rows: { size: string; a_mm: number; b_mm: number; difference_mm: number; error_mm: number;
    required_ease_mm?: number;
    pass: boolean; a_path: Pt[]; b_path: Pt[] }[];
}
export interface MarkRow {
  piece: string; id: string; origin: string; count: number; spacing: PatternValue;
  direction: PatternValue; kind: 'button' | 'buttonhole' | 'drill'; length: PatternValue; angle: PatternValue;
}
/** A transaction is validated in all sizes after every operation has been applied. */
export type StyleOperation =
  | { op: 'put_object'; id: string; object: DrawingObject; replace?: boolean }
  | { op: 'delete_object'; id: string }
  | { op: 'rename_object'; id: string; new_id: string }
  | { op: 'move_point'; id: string; size: string; target: Pt; preserve_formulas?: boolean }
  | { op: 'put_piece'; piece: PatternPiece; replace?: boolean }
  | { op: 'delete_piece'; id: string }
  | { op: 'set_allowance'; piece: string; edges: number[]; width: PatternValue; corner: 'miter' | 'bevel' | 'square' | 'mirrored' }
  | { op: 'add_annotation'; piece: string; annotation: Annotation }
  | { op: 'mark_row'; row: MarkRow }
  | { op: 'set_chart'; chart: SizeChart }
  | { op: 'set_variable'; name: string; value: PatternValue }
  | { op: 'set_rule'; id: string; rule: GradeRule }
  | { op: 'attach_rule'; piece: string; point: string; rule: string | null }
  | { op: 'offset_piece'; piece: string; id: string; size: string; distance: number }
  | { op: 'join_pieces'; pieces: [string, string]; id: string; size: string }
  /** Existing piece_action schema: mirror, unfold, split, dart, pleat, slash_spread. */
  | { op: 'piece_action'; request: Record<string, Json> };
/** New path edits are opt-in so existing exhaustive StyleOperation switches still compile. */
export type StyleEditOperation = StyleOperation
  | { op: 'offset_path'; id: string; source: string; distance: PatternValue; side: 'left' | 'right' }
  | { op: 'extend_trim'; id: string; source: string; target: string; end: 0 | 1 };
export interface Hit {
  id: string; kind: 'point' | 'path' | 'piece'; point: Pt; distance_mm: number;
  piece_id?: string; edge?: number; along_mm?: number; parameter?: number;
  /** Native t follows the source object direction, even for a reversed piece edge. */
  parameter_kind?: 'native' | 'arc_length_fraction' | 'edge_length_fraction';
}
export type Measurement = { kind: 'length'; object: string } | { kind: 'distance'; a: string; b: string }
  | { kind: 'angle'; a: string; vertex: string; b: string };
export interface Measured { value: number; unit: 'mm' | 'deg'; points: Pt[] }
export interface LaySettings {
  /** Physical face on the first layer; pairs alternate. Does not grant piece flip permission. */
  face: 'up' | 'down' | 'face_to_face';
  /** Physical plies. Folded/tubular and face-to-face lays require an even count. */
  plies: number;
  /** Waste at EACH end of the spread. Does not change marker coordinate origin. */
  end_allowance_mm: number;
}
export interface RepeatBand {
  axis: 'x' | 'y'; offset_mm: number; width_mm: number; colour: string;
}
export interface StyleJobSettings {
  /** Choose one fabric group for the marker; required when a style mixes materials. */
  material?: string;
  fabric: Fabric; spacing: Spacing; ratio: Record<string, number>; lay?: LaySettings;
  repeat_bands?: RepeatBand[];
  /** Keys are unsuffixed document piece IDs. */
  piece_settings?: Record<string, { buffer?: number; block_margin?: number | null }>;
  /** References use unsuffixed document piece IDs; Rust expands rules per selected size. */
  matches?: MatchRule[];
}
export interface OverlayLine { placement: number; piece_id: string; axis: 'x' | 'y'; points: [Pt, Pt]; colour: string; width_mm: number }
export interface OverlayOutline { piece_id: string; size: string | null; points: Pt[]; placement?: number }
export interface MarkerOverlay<TMatch extends MatchSummary = MatchSummary> {
  /** Same phase measurements as the independent validator. Absent on older engines. */
  matches?: TMatch[];
  stripes: OverlayLine[]; outlines: OverlayOutline[]; graded: OverlayOutline[];
  previous: { kind: 'added' | 'removed' | 'moved' | 'unchanged'; piece_id: string; before: Pt[]; after: Pt[] }[];
  /** Cut extent plus two end allowances. */
  spread_length_mm: number;
}
export interface OverlayOptions { bands?: RepeatBand[]; graded?: Piece[]; previous?: { job: Job; result: MarkerResult } }
export interface StyleCore<TOperation = StyleOperation, TMatch extends MatchSummary = MatchSummary> {
  blocks(): BlockInfo[];
  size_chart(preset?: 'indian_adult' | 'iso_example'): SizeChart;
  create(request?: CreateStyle): PatternDocument;
  edit(document: PatternDocument, operations: TOperation[]): PatternDocument;
  save(document: PatternDocument): string;
  load(source: string): PatternDocument;
  grade(document: PatternDocument): PatternGrade;
  hit_test(document: PatternDocument, size: string, at: Pt, radius: number): Hit[];
  measure(document: PatternDocument, size: string, request: Measurement): Measured;
  seam_report(document: PatternDocument, check: SeamCheck): SeamReport;
  /** Default checks supplied by a generated block; recomputed after edits/grading. */
  block_seams(document: PatternDocument): SeamReport[];
  to_job(document: PatternDocument, settings: StyleJobSettings): Job;
  overlay(job: Job, result: MarkerResult, options?: OverlayOptions): MarkerOverlay<TMatch>;
}
export interface RawStyle { style_command(request: string): string }
/** Runtime engines support the extended edits and complete validator measurements. */
export type NativeStyleCore = StyleCore<StyleEditOperation, MatchReport>;
export type NativeMarkerOverlay = MarkerOverlay<MatchReport>;
/** Transport-only adapter, also usable by an in-process native host. */
export function bindStyleCore(raw: RawStyle): NativeStyleCore {
  const call = <T>(request: object): T => JSON.parse(raw.style_command(JSON.stringify(request))) as T;
  return {
    blocks: () => call({ action: 'blocks' }),
    size_chart: (preset = 'indian_adult') => call({ action: 'size_chart', preset }),
    create: (request = {}) => call({ action: 'create', request }),
    edit: (document, operations) => call({ action: 'edit', document, operations }),
    save: document => call({ action: 'save', document }),
    load: source => call({ action: 'load', source }),
    grade: document => call({ action: 'grade', document }),
    hit_test: (document, size, at, radius) => call({ action: 'hit_test', document, size, at, radius }),
    measure: (document, size, request) => call({ action: 'measure', document, size, request }),
    seam_report: (document, check) => call({ action: 'seam_report', document, check }),
    block_seams: document => call({ action: 'block_seams', document }),
    to_job: (document, settings) => call({ action: 'to_job', document, settings }),
    overlay: (job, result, options = {}) => call({ action: 'overlay', job, result, options }),
  };
}
/** Offline local WASM. New calls fail clearly if a stale WASM build is installed. */
export async function loadStyleCore(moduleUrl?: string | URL): Promise<NativeStyleCore> {
  const url = new URL(moduleUrl ?? 'wasm/beta_pattern_wasm.js', document.baseURI);
  const raw = await import(/* @vite-ignore */ url.href) as RawStyle & { default(options: { module_or_path: URL }): Promise<unknown> };
  await raw.default({ module_or_path: new URL('./beta_pattern_wasm_bg.wasm', url) });
  if (typeof raw.style_command !== 'function') throw new Error('Update the local Pattern WASM files to use style tools.');
  return bindStyleCore(raw);
}
