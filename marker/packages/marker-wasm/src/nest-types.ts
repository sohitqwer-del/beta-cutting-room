import type { Job, MarkerResult, Placement } from './types';

/** Browser search is always single threaded. All geometry runs in Rust. */
export interface BrowserNestOptions {
  time_limit_s?: number;
  seed?: number | null;
  /** Search allowance per round (0.05–30 seconds); setup/checks add overhead. Default 2. */
  round_time_s?: number;
  early_termination?: boolean;
  fixed?: Placement[];
  warm_start?: MarkerResult | null;
}
export interface NestProgress {
  /** Within-round update; no continue acknowledgement is required. */
  checkpoint?: boolean;
  round: number;
  seconds: number;
  round_seconds: number;
  improved: boolean;
  done: boolean;
  cancelled: boolean;
  /** Best complete marker, checked by the independent Rust validator. */
  result: MarkerResult;
  /** Allocated WASM linear memory, not whole-browser memory. */
  memory_bytes: number;
}
export type NestWorkerRequest =
  | { type: 'init'; moduleUrl: string }
  | { type: 'nest'; id: number; job: Job; options: BrowserNestOptions }
  | { type: 'continue'; id: number; round: number }
  | { type: 'cancel'; id: number };
export type NestWorkerResponse =
  | { type: 'ready' }
  | { type: 'progress'; id: number; progress: NestProgress }
  | { type: 'done'; id: number; result: MarkerResult }
  | { type: 'error'; id?: number; message: string };
