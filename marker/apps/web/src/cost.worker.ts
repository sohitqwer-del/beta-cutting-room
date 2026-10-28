import type { CostComparisonRequest } from './cost-types';
interface RawCost {
  default(options: { module_or_path: URL }): Promise<unknown>;
  compare_cost(request: string, progress: (json: string) => boolean): string;
}
/** Each comparison owns its Worker. Stop terminates it without publishing a partial winner. */
self.onmessage = async ({ data }: MessageEvent<{ moduleUrl: string; request: CostComparisonRequest }>) => {
  try {
    const raw = await import(/* @vite-ignore */ data.moduleUrl) as RawCost;
    await raw.default({ module_or_path: new URL('./beta_marker_cutroom_wasm_bg.wasm', data.moduleUrl) });
    const result = raw.compare_cost(JSON.stringify(data.request), () => { self.postMessage({ type: 'progress' }); return true; });
    self.postMessage({ type: 'done', value: JSON.parse(result) });
  } catch (error) { self.postMessage({ type: 'error', message: error instanceof Error ? error.message : String(error) }); }
};
