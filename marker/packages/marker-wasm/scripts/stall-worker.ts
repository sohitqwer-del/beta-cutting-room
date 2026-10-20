// Regression fixture only. Publish a real Rust-validated seed, then simulate a
// non-yielding optimizer call. The host watchdog must preserve the seed.
import { loadNestRuntime } from '../src/nest-worker';
import type { NestWorkerRequest } from '../src/nest-types';
const scope = globalThis as any;
let runtime: ReturnType<typeof loadNestRuntime>;
scope.onmessage = async ({ data }: MessageEvent<NestWorkerRequest>) => {
  if (data.type === 'init') {
    runtime = loadNestRuntime(data.moduleUrl);
    await runtime;
    scope.postMessage({ type: 'ready' });
  } else if (data.type === 'nest') {
    const raw = await runtime;
    const session = new raw.NestSession(JSON.stringify(data.job), JSON.stringify(data.options));
    const progress = JSON.parse(session.step());
    progress.memory_bytes = raw.memory.buffer.byteLength;
    scope.postMessage({ type: 'progress', id: data.id, progress });
    // This Worker is created only by browser-test.mjs, never shipped in the app.
    for (;;) { /* deliberately blocked until Worker.terminate() */ }
  }
};
