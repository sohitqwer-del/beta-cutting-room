export * from './pattern';
export * from './export';
export * from './import';
export * from './cutroom';
export { createMarkerNester, nest } from './nest';
export type { MarkerNester, MarkerNesterConfig, BrowserNestOptions, NestProgress } from './nest';
export type * from './types';
import type { AllowedRotations, Demand, Issue, Job, MarkerDrawing, MarkerResult, Metrics, Placement, Pt, ValidationReport } from './types';
interface RawWasm {
  default: (options?: { module_or_path: string | URL }) => Promise<unknown>;
  validate: (job: string, result: string) => string;
  metrics: (job: string, placements: string) => string;
  allowed_rotations: (job: string, id: string, mirrored: boolean) => string;
  demands: (job: string) => string;
  check_job: (job: string) => string;
  effective_job: (job: string) => string;
  placed_outline: (job: string, placement: string) => string;
  layer_copies: (job: string, placement: string) => string;
  drawing: (job: string, placements: string) => string;
  transform: (points: string, mirrored: boolean, rot: number, translate: string) => string;
}
/** Typed calls into the Rust core. Parse errors throw an exception. */
export interface MarkerCore {
  validate(job: Job, result: MarkerResult): ValidationReport;
  metrics(job: Job, placements: Placement[]): Metrics;
  allowed_rotations(job: Job, id: string, mirrored: boolean): AllowedRotations;
  demands(job: Job): Demand[];
  check_job(job: Job): Issue[];
  effective_job(job: Job): Job;
  placed_outline(job: Job, placement: Placement): Pt[];
  layer_copies(job: Job, placement: Placement): [number, number];
  drawing(job: Job, placements: Placement[]): MarkerDrawing;
  transform(points: Pt[], mirrored: boolean, rot: number, translate: Pt): Pt[];
}
/** Load generated wasm-bindgen files from a local URL. No server API is required. */
export async function loadMarkerCore(moduleUrl: string | URL = new URL('wasm/beta_marker_wasm.js', document.baseURI)): Promise<MarkerCore> {
  const url = new URL(String(moduleUrl), document.baseURI).href;
  const raw = await import(/* @vite-ignore */ url) as RawWasm;
  await raw.default({ module_or_path: new URL('./beta_marker_wasm_bg.wasm', url) });
  const json = JSON.stringify;
  return {
    validate: (job, result) => JSON.parse(raw.validate(json(job), json(result))),
    metrics: (job, placements) => JSON.parse(raw.metrics(json(job), json(placements))),
    allowed_rotations: (job, id, mirrored) => JSON.parse(raw.allowed_rotations(json(job), id, mirrored)),
    demands: job => JSON.parse(raw.demands(json(job))),
    check_job: job => JSON.parse(raw.check_job(json(job))),
    effective_job: job => JSON.parse(raw.effective_job(json(job))),
    placed_outline: (job, placement) => JSON.parse(raw.placed_outline(json(job), json(placement))),
    layer_copies: (job, placement) => JSON.parse(raw.layer_copies(json(job), json(placement))),
    drawing: (job, placements) => JSON.parse(raw.drawing(json(job), json(placements))),
    transform: (points, mirrored, rot, translate) => JSON.parse(raw.transform(json(points), mirrored, rot, json(translate))),
  };
}

export * from './quick';
export * from './style';
