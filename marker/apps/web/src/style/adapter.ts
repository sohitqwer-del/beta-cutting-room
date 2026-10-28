import * as wasm from '@beta/marker-wasm';
import type { BlockInfo, CreateStyle, GradedPiece, Hit, Job, MarkerCore, MarkerOverlay, MarkerResult, Measured, Measurement, OverlayOptions, PatternCore,
  PatternDocument, PatternGrade, PatternInspection, Pt, QuickMarkerCore, SeamCheck, SeamReport, SizeChart, SpecFile, SpecSheet, StyleCore, StyleEditOperation, StyleQualityCore, MatchSummary, StyleJobSettings, ValidationReport } from '@beta/marker-wasm';
import { apiFetch } from '../transport';
import { exportFile, isPractice, nestInRounds, nestPractice, type NestProgress } from '../quick/adapter';

/** The only place the Style screens touch the engine (STYLE-API v1, `pattern/docs/STYLE-API.md`).
 * - Style calls: Rust `loadStyleCore()` (Pattern WASM `style_command`). `mock.ts` is for unit tests only.
 * - Drawing, grading, snapping to geometry and exports: Rust `PatternCore`. Marker checks: Rust `MarkerCore`.
 * - Nesting: Rust in a browser Web Worker, or the desktop app / `beta-marker serve`. */
export type EngineKind = 'style';
/** Where a match point landed on the marker and whether it meets its partner (error in mm when known). */
export interface MatchStatus { rule: number; garment: number | null; a: Pt; b: Pt | null; error_mm: number; pass: boolean }
export interface StyleEngine {
  kind: EngineKind;
  blocks(): Promise<BlockInfo[]>;
  sizeChart(preset?: 'indian_adult' | 'iso_example'): Promise<SizeChart>;
  create(request: CreateStyle): Promise<PatternDocument>;
  /** All operations or none. The caller keeps the previous document for undo. */
  edit(document: PatternDocument, operations: StyleEditOperation[]): Promise<PatternDocument>;
  inspect(document: PatternDocument, size: string): Promise<PatternInspection>;
  grade(document: PatternDocument): Promise<PatternGrade>;
  /** All sizes of each piece aligned at a reference point (stacked "nest" view). */
  nestSizes(document: PatternDocument, reference: string): Promise<Record<string, GradedPiece[]>>;
  snap(document: PatternDocument, size: string, at: Pt, radius: number, grid: number): Promise<{ point: Pt; object: string | null }>;
  hitTest(document: PatternDocument, size: string, at: Pt, radius: number): Promise<Hit[]>;
  /** Same Rust hit test, answered at once: the canvas picks points, lines, piece edges and pieces with it while the pointer moves. */
  hitTestNow(document: PatternDocument, size: string, at: Pt, radius: number): Hit[];
  measure(document: PatternDocument, size: string, request: Measurement): Promise<Measured>;
  seamReport(document: PatternDocument, check: SeamCheck): Promise<SeamReport>;
  blockSeams(document: PatternDocument): Promise<SeamReport[]>;
  toJob(document: PatternDocument, settings: StyleJobSettings): Promise<Job>;
  overlay(job: Job, result: MarkerResult, options?: OverlayOptions): Promise<MarkerOverlay>;
  validate(job: Job, result: MarkerResult): Promise<ValidationReport>;
  /** Where each match point landed and whether the Rust validator accepts it (used when the overlay has no `matches`). */
  matchStatus(job: Job, result: MarkerResult, report: ValidationReport): Promise<MatchStatus[]>;
  save(document: PatternDocument): Promise<string>;
  load(source: string): Promise<PatternDocument>;
  exportPatternPdf(document: PatternDocument, paper: 'A4' | 'A3'): Promise<Uint8Array>;
  exportPatternDxf(document: PatternDocument): Promise<string>;
  exportPatternPlt(document: PatternDocument): Promise<string>;
  /** Points of measure for every size, from the graded stitch paths (fit7). Absent on older engine builds. */
  specSheet?(document: PatternDocument): Promise<SpecSheet>;
  exportSpecSheet?(document: PatternDocument, format: 'pdf' | 'csv' | 'xlsx'): Promise<SpecFile>;
}
export type LoadResult = { ok: true; engine: StyleEngine; note?: string } | { ok: false; error: string };

const call = <T>(f: () => T): Promise<T> => new Promise((resolve, reject) => {
  try { resolve(f()); } catch (error) { reject(error instanceof Error ? error : new Error(String(error))); }
});
/** Join the Style, Pattern and Marker cores into one async engine. Exported for contract tests. */
export function joinCores(style: StyleCore<StyleEditOperation, MatchSummary>, pattern: PatternCore, marker: MarkerCore | undefined, kind: EngineKind): StyleEngine {
  return {
    kind,
    blocks: () => call(() => style.blocks()),
    sizeChart: preset => call(() => style.size_chart(preset)),
    create: request => call(() => style.create(request)),
    edit: (doc, ops) => call(() => style.edit(doc, ops)),
    inspect: (doc, size) => call(() => pattern.inspect(doc, size)),
    grade: doc => call(() => style.grade(doc)),
    nestSizes: (doc, reference) => call(() => pattern.nest(doc, reference).sizes),
    snap: (doc, size, at, radius, grid) => call(() => pattern.snap(doc, size, { at, radius, grid, points: true, paths: true })),
    hitTest: (doc, size, at, radius) => call(() => style.hit_test(doc, size, at, radius)),
    hitTestNow: (doc, size, at, radius) => style.hit_test(doc, size, at, radius),
    measure: (doc, size, request) => call(() => style.measure(doc, size, request)),
    seamReport: (doc, check) => call(() => style.seam_report(doc, check)),
    blockSeams: doc => call(() => style.block_seams(doc)),
    toJob: (doc, settings) => call(() => style.to_job(doc, settings)),
    overlay: (job, result, options) => call(() => style.overlay(job, result, options)),
    validate: (job, result) => call(() => { if (!marker) throw new Error('Marker checks are not loaded.'); return marker.validate(job, result); }),
    matchStatus: (job, result, report) => call(() => matchStatus(marker, job, result, report)),
    save: doc => call(() => style.save(doc)),
    load: source => call(() => style.load(source)),
    exportPatternPdf: (doc, paper) => call(() => pattern.export_pdf(doc, paper)),
    exportPatternDxf: doc => call(() => pattern.export_dxf(doc)),
    exportPatternPlt: doc => call(() => pattern.export_plt(doc)),
    ...(hasQuality(style) ? {
      specSheet: (doc: PatternDocument) => call(() => style.spec_sheet(doc)),
      exportSpecSheet: (doc: PatternDocument, format: 'pdf' | 'csv' | 'xlsx') => call(() => style.export_spec_sheet(doc, format)),
    } : {}),
  };
}
const hasQuality = (style: object): style is StyleQualityCore => typeof (style as Partial<StyleQualityCore>).spec_sheet === 'function' && typeof (style as Partial<StyleQualityCore>).export_spec_sheet === 'function';

/** Match points on the marker: positions from Rust `transform`, pass or fail from the Rust validator's `match` issues
 * ("Bundle N does not satisfy matching point piece/point"). No phase arithmetic happens in the browser. */
export function matchStatus(marker: MarkerCore | undefined, job: Job, result: MarkerResult, report: ValidationReport): MatchStatus[] {
  if (!marker || !job.match_rules?.length) return [];
  const failed = report.errors.filter(e => e.code === 'match');
  const at = (piece: string, point: string, p: MarkerResult['placements'][number]): Pt | null => {
    const local = job.pieces.find(q => q.id === piece)?.match_points?.find(m => m.id === point)?.point;
    return local ? marker.transform([local], p.mirrored, p.rotation_deg, p.translate)[0] ?? null : null;
  };
  const out: MatchStatus[] = [];
  job.match_rules.forEach((rule, index) => {
    for (const pa of result.placements.filter(p => p.piece_id === rule.a.piece_id)) {
      const a = at(rule.a.piece_id, rule.a.point_id, pa); if (!a) continue;
      const pb = rule.b ? result.placements.find(p => p.piece_id === rule.b!.piece_id && p.bundle === pa.bundle) : undefined;
      const b = rule.b && pb ? at(rule.b.piece_id, rule.b.point_id, pb) : null;
      const bad = failed.some(e => e.piece_ids?.includes(rule.a.piece_id) && e.message.includes(`${rule.a.piece_id}/${rule.a.point_id}`) && (pa.bundle == null || e.message.includes(`Bundle ${pa.bundle} `)));
      out.push({ rule: index, garment: pa.bundle ?? null, a, b, error_mm: Number.NaN, pass: !bad });
    }
  });
  return out;
}
export interface Loaders {
  style?: () => Promise<StyleCore<StyleEditOperation, MatchSummary>>; pattern?: () => Promise<PatternCore>; marker?: () => Promise<MarkerCore>;
}
/** Load the Rust Style engine with the Pattern and Marker cores. A Pattern WASM without style tools is reported, not replaced.
 * Loaders are injectable for tests. */
export async function loadStyleEngine(options: { loaders?: Loaders } = {}): Promise<LoadResult> {
  const l = { style: () => wasm.loadStyleCore(), pattern: () => wasm.loadPatternCore(), marker: () => wasm.loadMarkerCore(), ...options.loaders };
  // Both downloads start together (about 1 MB and 1.4 MB compressed): on a slow phone connection this halves the wait.
  const markerLoading = l.marker().catch(() => undefined);
  let pattern: PatternCore;
  try { pattern = await l.pattern(); }
  catch (error) { return { ok: false, error: error instanceof Error ? error.message : String(error) }; }
  const marker = await markerLoading;
  try { return { ok: true, engine: joinCores(await l.style(), pattern, marker, 'style') }; }
  catch (error) { return { ok: false, error: error instanceof Error ? error.message : String(error) }; }
}
/** Fast practice layout for the headless smoke (`?style-practice=1`): instant, marked "not a real marker". */
export const practiceRequested = (search: string): boolean => /[?&]style-practice=1\b/.test(search);
export const isPracticeResult = (result: MarkerResult | undefined): boolean => isPractice(result);

export type { NestProgress };
/** Nest a marker: Rust in this browser first (Web Worker, warm-start rounds), else the desktop app or server. */
export async function nestMarker(job: Job, run: { budgetS: number; signal?: AbortSignal; onProgress?: (p: NestProgress) => void; practice?: boolean }): Promise<MarkerResult | null> {
  if (run.practice) return nestPractice(job, run);
  return nestInRounds(apiFetch, job, { budgetS: run.budgetS, roundS: Math.min(5, Math.max(1, run.budgetS / 4)), signal: run.signal, onProgress: run.onProgress });
}

let quick: Promise<QuickMarkerCore> | undefined;
/** Marker files use shared Rust writers through WASM, desktop IPC or the server. */
export async function exportMarker(job: Job, result: MarkerResult, format: 'plt' | 'pdf' | 'dxf' | 'tiled-a4' | 'tiled-a3'): Promise<Blob> {
  if (format === 'tiled-a4' || format === 'tiled-a3') {
    quick ??= wasm.loadQuickMarkerCore();
    const bytes = (await quick).tiledMarkerPdf(job, result, { paper: format === 'tiled-a4' ? 'A4' : 'A3', overlap_mm: 10 });
    return new Blob([new Uint8Array(bytes)], { type: 'application/pdf' });
  }
  if (format === 'dxf') {
    const response = await apiFetch('/api/export?format=dxf', { method: 'POST', body: JSON.stringify({ job, result }), headers: { 'Content-Type': 'application/json' } });
    if (!response.ok) throw new Error(((await response.json().catch(() => null)) as { error?: string } | null)?.error ?? `Server error ${response.status}.`);
    return response.blob();
  }
  return exportFile(apiFetch, job, result, format);
}
