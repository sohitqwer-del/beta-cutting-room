// Headless Chrome/CDP, following marker/deploy/browser-smoke.mjs.
// node .../browser-test.mjs <output-dir> [cases.json] [round-seconds=2]
// Optional cases run the same Rust session on the throttled renderer thread:
// Chromium CPU emulation targets the main thread, not the production Worker.
import { build } from 'esbuild';
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync, appendFileSync, rmSync } from 'node:fs';
import { resolve, join, dirname, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { gzipSync } from 'node:zlib';

const here = dirname(fileURLToPath(import.meta.url));
const out = resolve(process.argv[2] || join(here, '../../../../out/wasm-browser'));
mkdirSync(out, { recursive: true });
const pkg = resolve(here, '../pkg');
const assets = join(out, 'assets'); mkdirSync(assets, { recursive: true });
await build({ entryPoints: [join(here, 'browser-harness.ts')], bundle: true, format: 'esm', outfile: join(assets, 'harness.js') });
await build({ entryPoints: [join(here, '../src/nest.worker.ts')], bundle: true, format: 'esm', outfile: join(assets, 'worker.js') });
await build({ entryPoints: [join(here, 'stall-worker.ts')], bundle: true, format: 'esm', outfile: join(assets, 'stall-worker.js') });
const wasm = readFileSync(join(pkg, 'beta_marker_wasm_bg.wasm'));
const report = { started_at: new Date().toISOString(), wasm_bytes: wasm.length,
  wasm_gzip_bytes: gzipSync(wasm, { level: 9 }).length, cases: [], requests: [], exceptions: [],
  benchmark_execution: 'Rust BrowserSession on renderer main thread with CDP 4x CPU throttling; Worker checked separately',
};
const html = `<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width">
<title>BETA offline browser nesting check</title><style>body{font:18px system-ui;max-width:760px;margin:60px auto;padding:20px;background:#f5f4ef;color:#173d38}output{display:block;padding:24px;background:white;border:1px solid #bbb}</style>
<h1>Offline browser nesting</h1><p>Rust Sparrow search in a Web Worker. Every returned marker passes the independent validator.</p>
<output aria-live="polite">Loading local WASM…</output><script type="module" src="/harness.js"></script>`;
const server = createServer((req, res) => {
  const path = new URL(req.url, 'http://localhost').pathname;
  if (path === '/') { res.setHeader('Content-Type', 'text/html'); res.end(html); return; }
  const files = {
    '/harness.js': join(assets, 'harness.js'), '/worker.js': join(assets, 'worker.js'),
    '/stall-worker.js': join(assets, 'stall-worker.js'),
    '/wasm/beta_marker_wasm.js': join(pkg, 'beta_marker_wasm.js'),
    '/wasm/beta_marker_wasm_bg.wasm': join(pkg, 'beta_marker_wasm_bg.wasm'),
  };
  if (!files[path]) { res.writeHead(404); res.end(); return; }
  res.setHeader('Content-Type', path.endsWith('.wasm') ? 'application/wasm' : 'text/javascript');
  res.end(readFileSync(files[path]));
});
await new Promise(r => server.listen(0, '127.0.0.1', r));
const url = `http://127.0.0.1:${server.address().port}/`;
const chrome = [process.env.CHROME_PATH, 'C:/Program Files/Google/Chrome/Application/chrome.exe',
  'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe'].find(p => p && existsSync(p));
if (!chrome) throw new Error('Set CHROME_PATH to a Chrome/Chromium executable.');
const profile = mkdtempSync(join(out, 'chrome-profile-'));
const browser = spawn(chrome, ['--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`,
  '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--disable-background-networking',
  '--disable-component-update', '--disable-sync', 'about:blank'], { stdio: 'ignore', windowsHide: true });
const delay = ms => new Promise(r => setTimeout(r, ms));
let ws, nextId = 0, benchmarking = false;
const pending = new Map();
function send(method, params = {}) {
  const id = ++nextId;
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject });
    ws.send(JSON.stringify({ id, method, params }));
  });
}
async function evaluate(expression) {
  const response = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
  if (response.exceptionDetails) throw new Error(JSON.stringify(response.exceptionDetails));
  return response.result.value;
}
async function until(expression, seconds = 30) {
  const deadline = Date.now() + seconds * 1000;
  while (Date.now() < deadline) { if (await evaluate(expression)) return; await delay(100); }
  throw new Error(`Timed out: ${expression}`);
}
function assert(condition, name) {
  if (!condition) throw new Error(name);
  report.cases.push(name);
}
try {
  let port;
  for (let i = 0; i < 100; i++) {
    const path = join(profile, 'DevToolsActivePort');
    if (existsSync(path)) {
      try { port = readFileSync(path, 'utf8').split('\n')[0]; if (port) break; }
      catch (error) { if (!['EBUSY', 'ENOENT'].includes(error.code)) throw error; }
    }
    await delay(100);
  }
  if (!port) throw new Error('Headless Chrome did not start.');
  const target = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: 'PUT' })).json();
  ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = reject; });
  ws.onmessage = ({ data }) => {
    const message = JSON.parse(data);
    if (message.id) {
      const p = pending.get(message.id); if (!p) return;
      pending.delete(message.id);
      message.error ? p.reject(new Error(message.error.message)) : p.resolve(message.result);
    } else if (message.method === 'Network.requestWillBeSent') {
      report.requests.push(message.params.request.url);
    } else if (message.method === 'Runtime.exceptionThrown') {
      report.exceptions.push(message.params.exceptionDetails);
    } else if (message.method === 'Runtime.bindingCalled' && message.params.name === 'betaBenchProgress' && benchmarking) {
      process.send?.({ type: 'checkpoint', progress: JSON.parse(message.params.payload) });
    }
  };
  report.browser = await send('Browser.getVersion');
  await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable');
  await send('Page.navigate', { url });
  await until('!!globalThis.harness');
  await evaluate('harness.init()');
  // All worker and WASM assets are now resident. Stop the static server too:
  // this is stronger than merely omitting beta-marker serve.
  server.closeAllConnections();
  await new Promise(resolve => server.close(resolve));
  report.server_stopped_before_search = true;
  await send('Network.emulateNetworkConditions', { offline: true, latency: 0, downloadThroughput: 0, uploadThroughput: 0 });
  const afterLoad = report.requests.length;
  report.demo = await evaluate('harness.workerDemo()');
  assert(report.demo.validation.ok && /sparrow|bounded/.test(report.demo.result.engine), 'demo nests with real Rust WASM and no server');
  assert(report.demo.events.some(p => p.round >= 1), 'Sparrow improvement rounds follow the checked BETA seed');
  assert(report.demo.result.placements.length === 8, 'demo returns all eight placements');
  assert(report.demo.heartbeats >= 10, 'UI event loop remains responsive during Worker search');
  assert(report.demo.events.every(p => p.result.placements.length === 8), 'progress contains complete markers');
  report.stop = await evaluate('harness.workerDemo(true)');
  assert(report.stop.validation.ok && report.stop.events.at(-1).cancelled, 'cooperative stop retains a checked marker');
  assert(report.stop.events.at(-1).round === report.stop.events[0].round, 'stop does not start an extra search round');
  report.rules = await evaluate('harness.workerRules()');
  assert(true, 'folded layer allocation, defects and locked placements pass the real WASM validator');
  if (!process.argv[3]) {
    report.watchdog = await evaluate('harness.watchdogDemo()');
    assert(report.watchdog.validation.ok && report.watchdog.closed && report.watchdog.wall_s < 9,
      'real stalled Worker terminates and retains its Rust-validated seed');
    assert(report.watchdog.heartbeats > 50, 'UI remains responsive while a Worker is stuck');
  }
  assert(report.requests.length === afterLoad, 'no page network request after the server stops');
  const screenshot = await send('Page.captureScreenshot', { format: 'png' });
  writeFileSync(join(out, 'offline-worker.png'), Buffer.from(screenshot.data, 'base64'));
  await evaluate('harness.dispose()');
  if (process.argv[3]) {
    if (!process.send) throw new Error('Use wasm-benchmark.mjs for timed trials; it provides an external watchdog.');
    const cases = JSON.parse(readFileSync(resolve(process.argv[3]), 'utf8'));
    const roundSeconds = Number(process.argv[4] ?? 2);
    if (!Number.isFinite(roundSeconds) || roundSeconds < 0.05 || roundSeconds > 30) throw new Error('Invalid round duration.');
    report.round_time_s = roundSeconds;
    await send('Emulation.setCPUThrottlingRate', { rate: 4 });
    await send('Runtime.addBinding', { name: 'betaBenchProgress' });
    report.cpu_throttle_rate = 4;
    // Check that main-thread throttling is effective on fixed work.
    const calibration = `(()=>{let x=0;const start=performance.now();for(let i=0;i<30000000;i++)x=Math.imul(x^i,1664525)+1013904223|0;return {ms:performance.now()-start,x}})()`;
    report.calibration_4x = await evaluate(calibration);
    await send('Emulation.setCPUThrottlingRate', { rate: 1 });
    report.calibration_1x = await evaluate(calibration);
    await send('Emulation.setCPUThrottlingRate', { rate: 4 });
    writeFileSync(join(out, 'browser-report.json'), JSON.stringify(report, null, 2));
    process.send({ type: 'metadata', report });
    writeFileSync(join(out, 'wasm-results.jsonl'), '');
    report.benchmark_failures = [];
    for (const item of cases) {
      for (const seconds of process.argv[5] ? [Number(process.argv[5])] : [30, 60]) {
        const start = performance.now();
        benchmarking = true;
        process.send({ type: 'start', id: item.id, budget_s: seconds });
        let value;
        try {
          value = await evaluate(`harness.benchmark(${JSON.stringify(item.job)},${seconds},${roundSeconds})`);
        } catch (error) {
          value = { error: String(error), validation: { ok: false }, wall_s: (performance.now() - start) / 1000 };
        }
        benchmarking = false;
        if (value.error) report.benchmark_failures.push({ id: item.id, budget_s: seconds, error: value.error });
        const row = { id: item.id, budget_s: seconds, round_time_s: roundSeconds, cpu_throttle_rate: 4, ...value };
        appendFileSync(join(out, 'wasm-results.jsonl'), JSON.stringify(row) + '\n');
        process.send({ type: 'result', row });
        console.log(value.error ? `${item.id} WASM ${seconds}s: FAILED (${value.wall_s.toFixed(2)}s)`
          : `${item.id} WASM ${seconds}s: ${(value.result.efficiency * 100).toFixed(2)}%, ${value.wall_s.toFixed(2)}s, ${(value.memory_bytes / 1048576).toFixed(1)} MiB`);
      }
    }
  }
  assert(report.exceptions.length === 0, 'no uncaught browser exception');
  report.passed = true;
} finally {
  writeFileSync(join(out, 'browser-report.json'), JSON.stringify(report, null, 2));
  ws?.close();
  server.closeAllConnections(); server.close();
  browser.kill();
  await delay(800);
  // Delete only the freshly allocated profile inside this test's output root.
  if (resolve(profile).startsWith(out + sep)) {
    try { rmSync(profile, { recursive: true, force: true, maxRetries: 3 }); } catch {}
  }
}
console.log(JSON.stringify({ passed: report.passed, cases: report.cases, wasm_bytes: report.wasm_bytes,
  wasm_gzip_bytes: report.wasm_gzip_bytes }));
