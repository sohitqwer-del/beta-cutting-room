// Headless release screenshots using only built-in original sample data and real app forms.
// Usage: BETA_TOKEN=<temporary token> node marker/deploy/app-screenshots.mjs <loopback URL> <output dir> <CLI binary>
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
const url = new URL(process.argv[2]);
if (!['127.0.0.1', 'localhost', '[::1]'].includes(url.hostname)) throw new Error('Smoke accepts only loopback URLs.');
if (!process.argv[4]) throw new Error('Supply the final CLI binary to capture a verified release.');
const out = resolve(process.argv[3]); mkdirSync(out, { recursive: true });
const token = process.env.BETA_TOKEN;
if (!token) throw new Error('Set a temporary BETA_TOKEN for the smoke server.');
const browser = [process.env.CHROME_PATH, 'C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe'].find(p => p && existsSync(p));
if (!browser) throw new Error('No headless Chrome/Edge found.');
const server = process.argv[4] ? spawn(resolve(process.argv[4]), ['serve', '--bind', '127.0.0.1', '--port', url.port, '--web-dir', resolve('marker/apps/web/dist')], { stdio: 'ignore', windowsHide: true }) : null;
const profile = mkdtempSync(join(out, 'browser-profile-'));
const proc = spawn(browser, ['--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', 'about:blank'], { stdio: 'ignore', windowsHide: true });
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const pending = new Map(); let nextId = 0; let ws;
const report = { viewport: { cssWidth: 1920, cssHeight: 1200, deviceScale: 1, pngWidth: 1920, pngHeight: 1200 }, startedAt: new Date().toISOString(), binarySha256: process.argv[4] ? createHash('sha256').update(readFileSync(resolve(process.argv[4]))).digest('hex') : null, cases: [], requests: [], externalRequests: [], exceptions: [], screenshots: [] };
function send(method, params = {}) {
  const id = ++nextId; ws.send(JSON.stringify({ id, method, params }));
  return new Promise((resolve, reject) => pending.set(id, { resolve, reject }));
}
async function evaluate(expression) {
  const value = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (value.exceptionDetails) throw new Error(value.exceptionDetails.text);
  return value.result.value;
}
async function until(expression, label) {
  const deadline = Date.now() + 60000;
  while (Date.now() < deadline) { if (await evaluate(expression)) { report.cases.push(label); return; } await delay(100); }
  throw new Error(`Timed out: ${label}: ${await evaluate("document.body.innerText.slice(-8000)")}`);
}
async function shot(name) {
  const result = await send('Page.captureScreenshot', { format: 'png' });
  const file = join(out, name); writeFileSync(file, Buffer.from(result.data, 'base64')); report.screenshots.push(file);
}
try {
  if (server) {
    let ready = false;
    for (let i = 0; i < 50; i++) {
      if (server.exitCode !== null) throw new Error('Owned smoke server exited.');
      try { const response = await fetch(new URL('/api/health', url), { headers: { Authorization: `Bearer ${token}` }, signal: AbortSignal.timeout(1000) }); if (response.ok) { ready = true; break; } } catch {}
      await delay(100);
    }
    if (!ready) throw new Error('Owned smoke server did not start.');
  }
  let port;
  for (let i = 0; i < 100; i++) { const file = join(profile, 'DevToolsActivePort'); if (existsSync(file)) { port = readFileSync(file, 'utf8').split('\n')[0]; if (port) break; } await delay(100); }
  if (!port) throw new Error('Headless browser did not start.');
  const target = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: 'PUT' })).json();
  ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { ws.onopen = resolve; ws.onerror = reject; });
  ws.onmessage = event => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) { const p = pending.get(message.id); pending.delete(message.id); message.error ? p.reject(new Error(message.error.message)) : p.resolve(message.result); return; }
    if (message.method === 'Network.responseReceived') {
      const response = message.params.response;
      if (response.url.startsWith(url.origin)) report.requests.push({ path: new URL(response.url).pathname, status: response.status });
      else if (!/^(data|blob|about):/.test(response.url)) report.externalRequests.push(response.url);
    }
    if (message.method === 'Runtime.exceptionThrown') report.exceptions.push(message.params.exceptionDetails.text);
  };
  await send('Page.enable'); await send('Network.enable'); await send('Runtime.enable');
  await send('Emulation.setDeviceMetricsOverride', { width: 1920, height: 1200, deviceScaleFactor: 1, mobile: false });
  await send('Page.navigate', { url: url.href });

  await until('document.querySelector("#token-dialog")?.open === true', 'token dialog ready');
  await evaluate(`document.querySelector('#token-input').value = ${JSON.stringify(token)}; document.querySelector('#token-save').click();`);
  await until('document.querySelector("#server-status")?.classList.contains("connected") && document.querySelector("#wasm-status")?.textContent.includes("Rust WASM is active")', 'real server and WASM ready');
  async function click(selector) { await evaluate(`document.querySelector(${JSON.stringify(selector)}).click()`); await delay(150); }
  async function fill(id, value) { await evaluate(`{ const e=document.getElementById(${JSON.stringify(id)});e.value=${JSON.stringify(String(value))};e.dispatchEvent(new Event('input',{bubbles:true})); }`); }
  async function submit(id) { await evaluate(`document.getElementById(${JSON.stringify(id)}).requestSubmit()`); }
  async function screenshot(name) { await evaluate('document.activeElement?.blur(); document.querySelector(".toast-close")?.click(); for(const e of document.querySelectorAll("*")) if(e.scrollTop) e.scrollTop=0; window.scrollTo(0,0)'); await delay(350); await shot(`${name}.png`); }
  await fill('app-theme','light'); await evaluate('document.querySelector("#app-theme").dispatchEvent(new Event("change",{bubbles:true}))'); await click('[data-screen="0"]'); await click('#demo'); await fill('time',1); await submit('nest-form');
  await until('!document.querySelector("#app").hasAttribute("aria-busy") && document.querySelectorAll("#canvas [data-index]").length === 8 && !document.querySelector("#check-status").textContent.includes("error")', 'sample shirt nested');
  await click('#fit'); await screenshot('marker-editor');
  await click('[data-screen="4"]');
  await until('document.querySelectorAll("[data-mode]").length > 3', 'pattern engine ready');
  await click('#pattern-starter'); await click('[data-mode="measurements"]'); await evaluate(`{ const e=document.querySelector('[data-pd-measure="chest"][data-size="M"]'); e.value='940'; e.dispatchEvent(new Event('change',{bubbles:true})); }`); await delay(200); await click('[data-mode="grading"]'); await click('#pd-fit');
  await until('document.querySelectorAll("#pd-canvas polygon").length >= 4', 'teaching bodice graded');
  await screenshot('pattern');
  await click('[data-screen="1"]'); await fill('quantity-0',40); await fill('quantity-1',40);
  await fill('max-plies',20); await fill('max-garments',2); await fill('plan-time',0.2); await fill('end-allowance',10); await submit('plan-form');
  await until('document.querySelector("#plan-error").textContent.includes("Plan ready")', 'real cut plan ready');
  await screenshot('cut-plan');
  await click('[data-screen="6"]');
  for (const [id,length] of [['DEMO-001',100],['DEMO-002',85]]) {
    await fill('roll-id',id); await fill('roll-lot','SAMPLE-LOT'); await fill('roll-dye','DEMO-NAVY');
    await fill('roll-ticket',length); await fill('roll-length',length); await submit('roll-form');
    await until(`document.querySelector('#roll-status').textContent.includes(${JSON.stringify(`Roll ${id} saved`)})`, `sample roll ${id} saved`);
  }
  await click('#inspect-rolls');
  await until('document.querySelector("#roll-status").textContent.includes("Inspection ready")', 'sample rolls inspected');
  await evaluate('document.querySelector("#screen-6 details").open=false'); await screenshot('rolls');
  await click('[data-screen="8"]');
  await until('!document.querySelector("#floor-status").textContent.includes("Loading") && !document.querySelector("#screen-8").hasAttribute("aria-busy")', 'floor engine ready');
  await click('#floor-use-rolls'); await click('[data-floor-page="spreading"]');
  await until('document.querySelectorAll("[data-floor-roll]").length === 2', 'sample stock received on floor');
  await fill('floor-lay-id','DEMO-LAY-01'); await fill('floor-order','SAMPLE-ORDER'); await fill('floor-plies',20); await fill('floor-date','2026-10-20');
  await evaluate('document.querySelector("[data-floor-roll]").checked=true'); await submit('floor-prepare-form');
  await until('document.querySelector("#floor-sheet-summary").textContent.includes("DEMO-LAY-01")', 'checked sample floor lay saved');
  await screenshot('spreading');
  await click('[data-screen="7"]'); await click('#make-bundles');
  await until('document.querySelector("#bundle-status").textContent.includes("bundles ready")', 'real bundle tickets ready');
  await screenshot('bundles');
  await click('[data-screen="12"]');
  await until('!!document.querySelector("[data-block=t_shirt]") && !document.querySelector("[data-block=t_shirt]").disabled', 'style blocks ready');
  await click('[data-block="t_shirt"]');
  await until('JSON.parse(localStorage.getItem("beta-style-v1") || "null")?.doc?.pieces?.length === 4', 'style T-shirt created');
  await click('#st-next');
  await until('!document.querySelector("#st-canvas").hidden && document.querySelectorAll("#sc-static path").length > 3', 'style pattern canvas ready');
  await click('#sc-fit'); await screenshot('style-pattern');
  report.sampleData = 'Original built-in shirt demo, teaching bodice and editable T-shirt block; synthetic rolls DEMO-001/002. No factory data.';
  if (report.requests.some(r => !r.path.startsWith('/api/') && r.status >= 400)) throw new Error('A bundled web asset failed to load.');
  if (report.externalRequests.length || report.exceptions.length) throw new Error('External request or uncaught browser exception.');
  report.passed = true;
} finally {
  try { ws?.close(); } catch {}
  proc.kill(); // Only this script's headless browser.
  server?.kill(); // Only the CLI child created by this script, if any.
  await delay(500);
  // Resolved target is a unique child of the explicitly supplied output directory.
  if (resolve(profile).startsWith(`${out}\\`) || resolve(profile).startsWith(`${out}/`)) {
    try { rmSync(profile, { recursive: true, force: true }); } catch {}
  }
  writeFileSync(join(out, 'app-screenshots-report.json'), JSON.stringify(report, null, 2));
}
console.log(`PASS: ${report.cases.length} browser checks; ${report.externalRequests.length} external requests; ${report.exceptions.length} uncaught exceptions.`);
