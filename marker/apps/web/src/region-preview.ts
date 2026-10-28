import type { DocumentState } from './state';
import type { EditorOutput } from './editor-core';
/** One disposable Worker per region preview; abort never modifies the document. */
export function previewRegion(state:DocumentState, action:Record<string,unknown>, signal:AbortSignal):Promise<EditorOutput> {
  if(signal.aborted) return Promise.reject(new Error('Area preview cancelled.'));
  return new Promise((resolve,reject)=>{
    const worker = new Worker(new URL('./region.worker.ts',import.meta.url),{type:'module'});
    const finish = () => {worker.terminate();signal.removeEventListener('abort',abort);};
    const abort = () => {finish();reject(new Error('Area preview cancelled.'));};
    signal.addEventListener('abort',abort,{once:true});
    worker.onmessage = ({data}:MessageEvent<{output?:EditorOutput;error?:string}>)=>{
      finish();if(data.output)resolve(data.output);else reject(new Error(data.error??'Area preview failed.'));
    };
    worker.onerror = event => {finish();reject(new Error(event.message||'Area preview worker failed.'));};
    worker.postMessage({state,action,moduleUrl:new URL('wasm/beta_marker_wasm.js',document.baseURI).href});
  });
}
