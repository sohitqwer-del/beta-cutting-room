/** Offline Quick Marker contract. Geometry is computed in Rust. */
import type { Fabric, Job, Mark, MarkerResult, Piece, Pt } from './types';
export type QuickConfidence = 'high' | 'medium' | 'low';
export interface QuickWarning { code: string; message: string; pieceId?: string; photoId?: string }
export interface QuickSuggestion<T> { value: T; confidence: QuickConfidence; reason: string }
export interface QuickSuggestions {
  name?: QuickSuggestion<string>;
  pair: 'mirror' | 'symmetric' | 'single';
  pairPieceId?: string;
  quantity?: QuickSuggestion<{ normal: number; mirrored: number }>;
  symmetryAxis?: QuickSuggestion<[Pt, Pt]>;
  cutOnFold?: QuickSuggestion<boolean>;
  grain?: QuickSuggestion<[Pt, Pt]>;
}
/** Existing Piece fields are accepted settings; suggestions never modify them. */
export interface QuickPiece extends Piece {
  photoId: string;
  suggestions: QuickSuggestions;
  expectedErrorMm: number;
  /** Convenience views of marks. Editing must update marks, the authoritative field. */
  notches: Mark[];
  drills: Mark[];
}
export interface QuickScanOptions {
  reference: 'a4-sheet' | 'beta-target';
  photoId?: string;
  targetManifest?: string;
  maxPieces?: number;
  /** 640..2400, default 2000; Rust downsamples before vision. */
  maxLongEdge?: number;
  /** Optional operator-selected A4 corners in ORIGINAL photo pixels (before EXIF rotation).
   * Clockwise in image coordinates, beginning at either end of a 210 mm edge. */
  a4Corners?: [Pt, Pt, Pt, Pt];
  /** Use a second detected A4 sheet as a scale-consistency check. Default true. */
  useSecondSheet?: boolean;
}
export interface QuickScale {
  mmPerPx: number;
  confidence: QuickConfidence;
  method: 'a4-sheet' | 'beta-target';
  /** Heuristic estimate, not a statistical bound or factory accuracy guarantee. */
  expectedErrorMm: number;
  tiltScore: number;
  /** Original photo pixels. Empty for BETA target mode. */
  referenceCorners: Pt[][];
  /** Row-major projective map from original photo pixels to output mm, y up.
   * BETA mode also has lens correction; use overlay rather than this map there. */
  imageToMm: number[];
}
export interface QuickScan {
  photoId: string;
  pieces: QuickPiece[];
  /** Source-photo overlay, downscaled to processedSize. */
  overlayPng: Uint8Array;
  scale: QuickScale;
  warnings: QuickWarning[];
  sourceSize: [number, number];
  processedSize: [number, number];
  /** Rust compute time; wrapper measures elapsed time including WASM crossing. */
  runtimeMs: number;
}
export interface QuickSession {
  pieces: QuickPiece[];
  warnings: QuickWarning[];
  /** Suggestions only. All pieces remain until the operator removes duplicates. */
  duplicates: { pieceId: string; duplicateOf: string; confidence: QuickConfidence }[];
}
export interface QuickFabric { width_mm: number; nap?: Fabric['nap']; layout?: Fabric['layout'] }
export interface TiledMarkerOptions { paper: 'A4' | 'A3'; overlap_mm?: number }
export interface ChalkGuideOptions { language?: 'en' | 'hi' | 'ta' }
export interface ProjectorLayout {
  format: 'beta-marker.projector/v1';
  units: 'mm';
  lengthMm: number;
  widthMm: number;
  calibrationSquareMm: number;
  pieces: { pieceId: string; name: string; size: string | null; copy: number; outline: Pt[];
    referencePoint: Pt; rotationDeg: number; mirrored: boolean; onFold: boolean }[];
}
export interface QuickMarkerCore {
  quickScan(photo: Uint8Array, options: QuickScanOptions): QuickScan;
  quickAssist(pieces: QuickPiece[]): QuickPiece[];
  quickMerge(scans: Pick<QuickScan, 'photoId' | 'pieces' | 'warnings'>[]): QuickSession;
  /** Requires explicit size on every piece. Never grades or copies one outline into other sizes. */
  quickPiecesToJob(pieces: QuickPiece[], fabric: QuickFabric, garments: Record<string, number>): Job;
  tiledMarkerPdf(job: Job, result: MarkerResult, options: TiledMarkerOptions): Uint8Array;
  chalkGuidePdf(job: Job, result: MarkerResult, options?: ChalkGuideOptions): Uint8Array;
  projectorLayout(job: Job, result: MarkerResult): ProjectorLayout;
}
interface RawQuick {
  default(options: { module_or_path: URL }): Promise<unknown>;
  quick_scan(photo: Uint8Array, options: string): string;
  quick_assist(pieces: string): string;
  quick_merge(scans: string): string;
  quick_pieces_to_job(pieces: string, fabric: string, garments: string): string;
  tiled_marker_pdf(job: string, result: string, options: string): Uint8Array;
  chalk_guide_pdf(job: string, result: string, options: string): Uint8Array;
  projector_layout(job: string, result: string): string;
}
/** Adapt wasm-bindgen's wire representation; exported for contract tests. */
export function bindQuickMarkerCore(raw: Omit<RawQuick, 'default'>): QuickMarkerCore {
  const json = JSON.stringify;
  return {
    quickScan(photo, options) {
      const start = performance.now();
      const scan = JSON.parse(raw.quick_scan(photo, json(options))) as Omit<QuickScan, 'overlayPng'> & { overlayPng: number[] };
      return { ...scan, overlayPng: Uint8Array.from(scan.overlayPng), runtimeMs: performance.now() - start };
    },
    quickAssist: pieces => JSON.parse(raw.quick_assist(json(pieces))),
    // Do not serialize large PNG arrays when combining scans.
    quickMerge: scans => JSON.parse(raw.quick_merge(json(scans.map(({photoId, pieces, warnings}) => ({photoId, pieces, warnings}))))),
    quickPiecesToJob: (pieces, fabric, garments) => JSON.parse(raw.quick_pieces_to_job(json(pieces), json(fabric), json(garments))),
    tiledMarkerPdf: (job, result, options) => raw.tiled_marker_pdf(json(job), json(result), json(options)),
    chalkGuidePdf: (job, result, options = {}) => raw.chalk_guide_pdf(json(job), json(result), json(options)),
    projectorLayout: (job, result) => JSON.parse(raw.projector_layout(json(job), json(result))),
  };
}
/** Loads local assets. Pass a module URL in Web Workers (where document is absent). */
export async function loadQuickMarkerCore(moduleUrl?: string | URL): Promise<QuickMarkerCore> {
  const base = typeof document === 'undefined' ? globalThis.location.href : document.baseURI;
  const url = new URL(moduleUrl ?? 'wasm/beta_marker_wasm.js', base);
  const raw = await import(/* @vite-ignore */ url.href) as RawQuick;
  await raw.default({ module_or_path: new URL('./beta_marker_wasm_bg.wasm', url) });
  return bindQuickMarkerCore(raw);
}
