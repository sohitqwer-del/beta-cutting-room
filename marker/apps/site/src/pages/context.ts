import type { Run } from '../benchmark.ts';

/** Inputs every page renderer may use. */
export interface BuildContext {
  /** File names present in public/assets/screens (e.g. "marker-editor.png"). */
  screens: Set<string>;
  /** Benchmark runs selected for display. */
  runs: Run[];
}
