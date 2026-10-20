/// <reference lib="webworker" />
/** Photo scans can take seconds on a mid-range phone, so the Rust WASM scan runs here, off the main thread.
 * The adapter falls back to the main thread if this worker cannot start or load the engine. */
import * as markerWasm from '@beta/marker-wasm';
import type { QuickMarkerCore, QuickScanOptions } from '@beta/marker-wasm';

type Loader = (moduleUrl?: string | URL) => Promise<QuickMarkerCore>;
export interface ScanRequest { id: number; bytes: Uint8Array; options: QuickScanOptions; moduleUrl: string }
let core: Promise<QuickMarkerCore> | undefined;
const scope = self as unknown as DedicatedWorkerGlobalScope;

scope.onmessage = async (event: MessageEvent<ScanRequest>) => {
  const { id, bytes, options, moduleUrl } = event.data;
  try {
    const loader = (markerWasm as unknown as Record<string, unknown>).loadQuickMarkerCore as Loader | undefined;
    if (typeof loader !== 'function') throw new Error('This build has no photo reader yet.');
    // The package loaders resolve URLs against document.baseURI; a worker has no document.
    const g = globalThis as unknown as { document?: { baseURI: string } };
    g.document ??= { baseURI: moduleUrl };
    core ??= loader(moduleUrl);
    const scan = (await core).quickScan(bytes, options);
    scope.postMessage({ id, scan }, [scan.overlayPng.buffer as ArrayBuffer]);
  } catch (error) {
    core = undefined;
    scope.postMessage({ id, error: error instanceof Error ? error.message : String(error) });
  }
};
