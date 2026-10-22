import type { ImportReport, Units } from './types';

export interface DxfImportOptions { units?: Units; pairs?: boolean }
export interface MarkerImportCore {
  /** Optional .rul text uses the existing experimental Rust grading rules. */
  importDxf(bytes: Uint8Array, options?: DxfImportOptions, rules?: string): ImportReport;
}
interface RawImport {
  default(options: { module_or_path: URL }): Promise<unknown>;
  import_dxf(bytes: Uint8Array, options: string, rules?: string): string;
}
export async function loadMarkerImportCore(moduleUrl?: string | URL): Promise<MarkerImportCore> {
  const base = typeof document === 'undefined' ? globalThis.location.href : document.baseURI;
  const url = new URL(moduleUrl ?? 'wasm/beta_marker_wasm.js', base);
  const raw = await import(/* @vite-ignore */ url.href) as RawImport;
  await raw.default({ module_or_path: new URL('./beta_marker_wasm_bg.wasm', url) });
  return { importDxf: (bytes, options = {}, rules) => JSON.parse(raw.import_dxf(bytes, JSON.stringify(options), rules)) };
}
