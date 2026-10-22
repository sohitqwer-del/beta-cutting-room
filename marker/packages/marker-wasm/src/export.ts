/** Factory files from the same Rust writers as the native application. */
import type { Job, MarkerResult, Placement, Pt } from './types';

export type MarkerExportFormat = 'svg' | 'plt' | 'hpgl' | 'pdf' | 'dxf' | 'iso';
export interface MarkerExportOptions {
  marks?: boolean;
  labels?: boolean;
  /** Square PDF tile side, including margins, in mm. Omit for roll output. */
  pdf_tile_mm?: number | null;
  /** HPGL: feed marker length along plotter Y. */
  rotate?: boolean;
}
export interface MarkerLabel {
  placement: number;
  piece_id: string;
  name: string;
  size: string | null;
  /** Interior anchor in marker coordinates (mm, Y up). */
  anchor: Pt;
  /** Readable text baseline, degrees counter-clockwise in marker coordinates. */
  rotation_deg: number;
  /** Distance from anchor to nearest cut edge. Text still needs UI fitting. */
  clearance_mm: number;
}
export interface MarkerExportCore {
  exportMarker(job: Job, result: MarkerResult, format: MarkerExportFormat, options?: MarkerExportOptions): Uint8Array;
  labels(job: Job, placements: Placement[]): MarkerLabel[];
}
interface RawExport {
  default(options: { module_or_path: URL }): Promise<unknown>;
  export_marker(job: string, result: string, format: string, options: string): Uint8Array;
  marker_labels(job: string, placements: string): string;
}
export function bindMarkerExportCore(raw: Omit<RawExport, 'default'>): MarkerExportCore {
  const json = JSON.stringify;
  return {
    exportMarker: (job, result, format, options = {}) => raw.export_marker(json(job), json(result), format, json(options)),
    labels: (job, placements) => JSON.parse(raw.marker_labels(json(job), json(placements))),
  };
}
/** Load local assets on demand. No HTTP API or external service is used. */
export async function loadMarkerExportCore(moduleUrl?: string | URL): Promise<MarkerExportCore> {
  const base = typeof document === 'undefined' ? globalThis.location.href : document.baseURI;
  const url = new URL(moduleUrl ?? 'wasm/beta_marker_wasm.js', base);
  const raw = await import(/* @vite-ignore */ url.href) as RawExport;
  await raw.default({ module_or_path: new URL('./beta_marker_wasm_bg.wasm', url) });
  return bindMarkerExportCore(raw);
}
