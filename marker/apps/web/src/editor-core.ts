import type { Job, MarkerResult, Placement, Pt, ValidationReport } from '@beta/marker-wasm';
import type { DocumentState } from './state';
/** Local editor bindings. Geometry stays in Rust; the existing public package is unchanged. */
export interface EditorInspection {
  validation: ValidationReport;
  pairs: { a: number; b: number; overlap_mm2: number; distance_mm: number; required_mm: number }[];
  section_violations: number[];
  end_violations: number[];
  geometry_ok: boolean;
  unplaced: Placement[];
  table_length: number;
  table_efficiency: number;
}
export interface EditorOutput extends DocumentState { selected: number[]; measurement: number | null; notice: string }
export interface EditorCore {
  chooseSearch?(state: DocumentState, candidate: MarkerResult, fixed: Placement[]): {result:MarkerResult;retained:boolean};
  drawing?(state: DocumentState): { outlines: Pt[][] };
  apply(state: DocumentState, selected: number[], action: Record<string, unknown>): EditorOutput;
  inspect(state: DocumentState): EditorInspection;
  reuse(current: Job, saved: DocumentState): MarkerResult;
}
export async function loadEditorCore(): Promise<EditorCore> {
  const url = new URL('wasm/beta_marker_wasm.js', document.baseURI).href;
  const raw = await import(/* @vite-ignore */ url) as {
    default(input: {module_or_path: URL}): Promise<unknown>;
    edit(job: string, result: string, command: string): string;
    edit_inspect(job: string, result: string): string;
    edit_reuse(current: string, job: string, result: string): string;
    drawing(job: string, placements: string): string;
    edit_choose_search(job: string, incumbent: string, candidate: string, fixed: string): string;
  };
  await raw.default({ module_or_path: new URL('./beta_marker_wasm_bg.wasm',url) });
  if (typeof raw.edit !== 'function') throw new Error('Rebuild marker WASM to load the editing tools.');
  return {
    chooseSearch: (state,candidate,fixed) => JSON.parse(raw.edit_choose_search(JSON.stringify(state.job),JSON.stringify(state.result),JSON.stringify(candidate),JSON.stringify(fixed))),
    drawing: state => JSON.parse(raw.drawing(JSON.stringify(state.job), JSON.stringify(state.result.placements))),
    apply: (state, selected, action) => JSON.parse(raw.edit(JSON.stringify(state.job), JSON.stringify(state.result), JSON.stringify({ selected, ...action }))),
    inspect: state => JSON.parse(raw.edit_inspect(JSON.stringify(state.job), JSON.stringify(state.result))),
    reuse: (current, saved) => JSON.parse(raw.edit_reuse(JSON.stringify(current), JSON.stringify(saved.job), JSON.stringify(saved.result))),
  };
}
export interface EditorSettings { sections?: number[]; table_length?: number; removed?: Placement[] }
export function editorSettings(job: Job): EditorSettings {
  const raw = job.meta?.editor_v1;
  if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return {};
  // Drawing must remain safe while Rust reports malformed saved metadata.
  return {
    sections: Array.isArray(raw.sections) && raw.sections.every(x => typeof x === 'number' && Number.isFinite(x)) ? raw.sections as number[] : [],
    table_length: typeof raw.table_length === 'number' && Number.isFinite(raw.table_length) ? raw.table_length : undefined,
  };
}
/** Input event deltas are passed to Rust as coordinates, without doing polygon work in JS. */
export type EditorPoint = Pt;
