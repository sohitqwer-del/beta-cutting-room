import type { DocumentState } from './state';
/** Local editing geometry runs off the UI thread. The host can terminate a preview. */
self.onmessage = async ({data}: MessageEvent<{moduleUrl:string; state:DocumentState; action:Record<string,unknown>}>) => {
  try {
    const raw = await import(/* @vite-ignore */ data.moduleUrl);
    await raw.default({module_or_path:new URL('./beta_marker_wasm_bg.wasm',data.moduleUrl)});
    const output = raw.edit(JSON.stringify(data.state.job),JSON.stringify(data.state.result),JSON.stringify({selected:[],...data.action}));
    self.postMessage({output:JSON.parse(output)});
  } catch(error) { self.postMessage({error:error instanceof Error ? error.message : String(error)}); }
};
