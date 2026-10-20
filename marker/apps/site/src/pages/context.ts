import type { Run } from '../benchmark.ts';

/** Inputs every page renderer may use. */
export interface BuildContext {
  /** File names present in public/assets/screens (e.g. "marker-editor.png"). */
  screens: Set<string>;
  /** True when the self-hosted demo video file is present in public/. */
  video?: boolean;
  /** Benchmark runs selected for display. */
  runs: Run[];
}
