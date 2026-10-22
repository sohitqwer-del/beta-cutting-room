// Production PWA check with a public synthetic photo, real scan and real nesting.
// Usage: node quick-offline-test.mjs <web-dist> <synthetic-photo.png> <output-dir> [subpath=/app/]
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { join, resolve, sep, extname } from 'node:path';
const [distArg, photoArg, outArg, prefix = '/app/'] = process.argv.slice(2);
if (!outArg) throw new Error('Pass web-dist, public synthetic PNG, and output directory.');
if (!prefix.startsWith('/') || !prefix.endsWith('/') || prefix.includes('..')) throw new Error('Use an absolute sub-path ending in /.');
const dist = resolve(distArg), out = resolve(outArg);
mkdirSync(out, { recursive: true });
const report = { cases: [], requests: [], exceptions: [] };
const server = createServer((request, response) => {
  let name = decodeURIComponent(new URL(request.url, 'http://localhost').pathname);
  if (!name.startsWith(prefix)) { response.writeHead(404); response.end(); return; }
  name = name.slice(prefix.length) || 'index.html';
  const file = resolve(dist, name);
  if (!file.startsWith(dist + sep) || !existsSync(file)) { response.writeHead(404); response.end(); return; }
  response.setHeader('Content-Type', ({ '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css',
    '.wasm': 'application/wasm', '.json': 'application/json', '.png': 'image/png', '.svg': 'image/svg+xml' })[extname(file)] || 'application/octet-stream');
  response.end(readFileSync(file));
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const url = `http://127.0.0.1:${server.address().port}${prefix}`;
const chrome = [process.env.CHROME_PATH, 'C:/Program Files/Google/Chrome/Application/chrome.exe',
  'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe'].find(p => p && existsSync(p));
if (!chrome) throw new Error('Set CHROME_PATH.');
const profile = mkdtempSync(join(out, 'chrome-profile-'));
const browser = spawn(chrome, ['--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`,
  '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--disable-background-networking',
  'about:blank'], { windowsHide: true, stdio: 'ignore' });
const sleep = ms => new Promise(r => setTimeout(r, ms));
let ws, sequence = 0;
const pending = new Map();
function send(method, params = {}) {
  const id = ++sequence;
  return new Promise((resolve, reject) => { pending.set(id, { resolve, reject }); ws.send(JSON.stringify({ id, method, params })); });
}
async function evaluate(expression) {
  const response = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (response.exceptionDetails) throw new Error(JSON.stringify(response.exceptionDetails));
  return response.result.value;
}
async function wait(expression, timeout = 60000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) { if (await evaluate(expression)) return; await sleep(100); }
  throw new Error(`Timed out: ${expression}; page: ${await evaluate('document.body.innerText.slice(-4000)')}`);
}
function check(ok, label) { if (!ok) throw new Error(label); report.cases.push(label); }
async function click(selector) {
  await evaluate(`(()=>{const e=document.querySelector(${JSON.stringify(selector)});if(!e)throw Error('missing control');e.click()})()`);
}
async function shot(name) {
  const result = await send('Page.captureScreenshot', { format: 'png' });
  writeFileSync(join(out, name), Buffer.from(result.data, 'base64'));
}
try {
  let port;
  for (let i = 0; i < 100; i++) {
    try { port = readFileSync(join(profile, 'DevToolsActivePort'), 'utf8').split('\n')[0]; } catch {}
    if (port) break;
    await sleep(100);
  }
  if (!port) throw new Error('Headless Chrome failed to start.');
  const tab = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: 'PUT' })).json();
  ws = new WebSocket(tab.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = reject; });
  ws.onmessage = ({ data }) => {
    const m = JSON.parse(data);
    if (m.id) { const p = pending.get(m.id); if (!p) return; pending.delete(m.id); m.error ? p.reject(new Error(m.error.message)) : p.resolve(m.result); }
    else if (m.method === 'Network.requestWillBeSent') report.requests.push(m.params.request.url);
    else if (m.method === 'Runtime.exceptionThrown') report.exceptions.push(m.params.exceptionDetails);
  };
  await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable');
  await send('Browser.setDownloadBehavior', { behavior: 'allow', downloadPath: out });
  await send('Emulation.setDeviceMetricsOverride', { width: 390, height: 844, deviceScaleFactor: 1, mobile: true });
  await send('Page.navigate', { url: url + '#/quick' });
  await wait("document.querySelector('#q-take') && navigator.serviceWorker.controller");
  report.cached = await evaluate("(async()=>{const keys=await caches.keys();return (await Promise.all(keys.map(async k=>(await (await caches.open(k)).keys()).map(r=>r.url)))).flat()})()");
  check(report.cached.some(u => /nest\.worker-.*\.js/.test(u)), 'production precache includes the nesting Worker');
  check(report.cached.some(u => u.endsWith('beta_marker_wasm_bg.wasm')), 'production precache includes the full Rust WASM');
  report.static_mode = await evaluate("!!document.querySelector('#static-mode-note')");
  if (report.static_mode) {
    check(report.cached.some(u => u.endsWith('beta_pattern_wasm_bg.wasm'))
      && report.cached.some(u => u.endsWith('beta_marker_floor_bg.wasm')), 'static build includes Pattern and Floor WASM');
    await click('[data-screen="0"]'); await click('#demo');
    await wait("document.querySelectorAll('#canvas [data-index]').length === 8");
    check(true, 'normal marker editor loads the eight-piece demo');
    await evaluate("document.querySelector('#time').value='1'; document.querySelector('#seed').value='1'");
    await click('#nest');
    await wait("!document.querySelector('#nest').disabled && /sparrow|bounded/.test(document.querySelector('#engine').textContent)", 30000);
    check(await evaluate("document.querySelector('#check-status').textContent === 'Checks passed'"), 'normal marker editor nests and validates without an API');
    await shot('static-marker-demo.png');
    // Since 0.7 only the board-photo digitizer needs a server; only the open screen is in the page.
    await click('[data-screen="5"]');
    check(await evaluate("[...document.querySelectorAll('.static-feature-note')].some(n => /digitizing panel needs the desktop app or server/.test(n.textContent))"), 'the digitizer explains that it needs the desktop app or a server');
    await click('[data-screen="1"]');
    check(await evaluate("!document.querySelector('.static-feature-note') && !document.querySelector('#make-plan').disabled"), 'cut plan works in the browser without a server note');
    await click('[data-screen="11"]');
    check(!report.requests.some(u => new URL(u).pathname.includes('/api/')), 'static mode sends no API requests');
  }
  server.closeAllConnections(); await new Promise(resolve => server.close(resolve));
  await send('Network.emulateNetworkConditions', { offline: true, latency: 0, downloadThroughput: 0, uploadThroughput: 0 });
  await send('Page.reload');
  await wait("document.querySelector('#q-take') && navigator.serviceWorker.controller");
  check(true, 'app reloads offline under a GitHub-Pages-style subpath');
  const photo = readFileSync(resolve(photoArg)).toString('base64');
  await evaluate(`(()=>{const bytes=Uint8Array.from(atob(${JSON.stringify(photo)}),c=>c.charCodeAt(0));const dt=new DataTransfer();dt.items.add(new File([bytes],'public-synthetic-a4.png',{type:'image/png'}));const input=document.querySelector('#q-gallery');input.files=dt.files;input.dispatchEvent(new Event('change',{bubbles:true}))})()`);
  await wait("document.querySelector('#q-to-review') && !document.querySelector('.q-status.q-reading')");
  check(!await evaluate("!!document.querySelector('.q-practice-chip')"), 'real photo scan runs offline without practice mode');
  await click('#q-to-review');
  await wait("document.querySelectorAll('.q-piece').length > 0");
  report.scanned_pieces = await evaluate("document.querySelectorAll('.q-piece').length");
  await click('#q-accept-all'); await click('#q-to-order');
  await wait("!!document.querySelector('#q-width')");
  await evaluate("(()=>{const e=document.querySelector('#q-width');e.value='150';e.dispatchEvent(new Event('input',{bubbles:true}))})()");
  // Use the explicit centimetre control to avoid guessing the persisted unit.
  await click('#q-unit-cm');
  await evaluate("(()=>{const e=document.querySelector('#q-width');e.value='150';e.dispatchEvent(new Event('input',{bubbles:true}))})()");
  await click('#q-budget-20'); await click('#q-make');
  await wait("document.querySelector('#screen-11')?.dataset.step === 'use' || !!document.querySelector('#q-preview .q-marker')", 120000);
  if (await evaluate("document.querySelector('#screen-11')?.dataset.step === 'nest'")) await click('#q-stop');
  await wait("document.querySelector('#screen-11')?.dataset.step === 'use'", 120000);
  report.marker_summary = await evaluate("[...document.querySelectorAll('.q-result strong')].map(e=>e.textContent)");
  check(report.marker_summary.length === 3, 'real scanned pieces nest offline into a complete marker');
  check(!report.requests.some(u => /\/api\/nest(?:[/?]|$)/.test(u)), 'Quick Marker makes no nesting API request');
  await shot('quick-offline-result.png');
  await click('#q-tiles');
  for (let i = 0; i < 100 && !existsSync(join(out, 'marker-tiles-A4.pdf')); i++) await sleep(100);
  check(existsSync(join(out, 'marker-tiles-A4.pdf')), 'A4 marker PDF exports offline');
  await click('#q-share-pdf');
  for (let i = 0; i < 100 && !existsSync(join(out, 'marker.pdf')); i++) await sleep(100);
  check(existsSync(join(out, 'marker.pdf')), 'PDF sharing uses the local exporter');
  check(!report.requests.some(u => /\/api\/export(?:[/?]|$)/.test(u)), 'PDF output makes no export API request');
  await click('#q-projector-btn');
  await wait("!!document.querySelector('.q-proj-piece')");
  await shot('quick-offline-projector.png');
  check(true, 'projector geometry is available offline');
  if (report.static_mode) {
    await click('#q-proj-exit');
    await evaluate("window.confirm = () => true");
    await click('#q-restart'); await click('#q-sample');
    await wait("document.querySelectorAll('.q-photo-card').length === 2 && !document.querySelector('.q-status.q-reading')");
    await click('#q-to-review'); await click('#q-accept-all'); await click('#q-to-order');
    await click('#q-unit-cm');
    await evaluate("(()=>{const e=document.querySelector('#q-width');e.value='150';e.dispatchEvent(new Event('input',{bubbles:true}))})()");
    await click('#q-budget-20'); await click('#q-make');
    await wait("document.querySelector('#screen-11')?.dataset.step === 'use' || !!document.querySelector('#q-preview .q-marker')", 45000);
    if (await evaluate("document.querySelector('#screen-11')?.dataset.step === 'nest'")) await click('#q-stop');
    await wait("document.querySelector('#screen-11')?.dataset.step === 'use'", 30000);
    await shot('static-quick-sample.png');
    await click('#q-editor');
    await wait("!document.querySelector('.workspace').hidden");
    check(await evaluate("/sparrow|bounded/.test(document.querySelector('#engine').textContent) && document.querySelector('#check-status').textContent === 'Checks passed'"),
      'built-in Quick Marker sample completes with a real validated browser nest');
  }
  check(report.exceptions.length === 0, 'no uncaught app exception');
  if (report.static_mode) check(!report.requests.some(u => new URL(u).pathname.includes('/api/')), 'full static flow stays API-free');
  report.passed = true;
} finally {
  writeFileSync(join(out, 'quick-offline-report.json'), JSON.stringify(report, null, 2));
  ws?.close(); server.closeAllConnections(); server.close(); browser.kill();
  await sleep(800);
  if (resolve(profile).startsWith(out + sep)) {
    try { rmSync(profile, { recursive: true, force: true, maxRetries: 3 }); } catch {}
  }
}
console.log(JSON.stringify(report.cases));
