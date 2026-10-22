// Headless CDP smoke, following apps/site/scripts/screenshots.mjs. No npm dependencies.
// Usage: BETA_TOKEN=<test token> node marker/deploy/browser-smoke.mjs <loopback URL> <output dir> [CLI binary]
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
const url = new URL(process.argv[2]);
if (!['127.0.0.1', 'localhost', '[::1]'].includes(url.hostname)) throw new Error('Smoke accepts only loopback URLs.');
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
const report = { startedAt: new Date().toISOString(), binarySha256: process.argv[4] ? createHash('sha256').update(readFileSync(resolve(process.argv[4]))).digest('hex') : null, cases: [], requests: [], externalRequests: [], exceptions: [], screenshots: [] };
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
  const deadline = Date.now() + 15000;
  while (Date.now() < deadline) { if (await evaluate(expression)) { report.cases.push(label); return; } await delay(100); }
  throw new Error(`Timed out: ${label}`);
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
  await send('Emulation.setDeviceMetricsOverride', { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false });
  await send('Page.navigate', { url: url.href });
  await until('document.querySelector("#token-dialog")?.open === true', '401 opens token dialog');
  await shot('token-required.png');
  await evaluate(`document.querySelector('#token-input').value = 'wrong-0123456789abcdef0123456789abcdef'; document.querySelector('#token-save').click();`);
  await until('document.querySelector("#token-dialog")?.open === true && !document.querySelector("#server-status")?.classList.contains("connected")', 'wrong token stays unauthenticated');
  await evaluate(`document.querySelector('#token-input').value = ${JSON.stringify(token)}; document.querySelector('#token-save').click();`);
  await until('document.querySelector("#server-status")?.classList.contains("connected") && document.querySelector("#server-status").textContent.includes("0.7.0")', 'correct token connects to real Hyper server');
  if (!await evaluate(`sessionStorage.getItem('beta-server-token') === ${JSON.stringify(token)} && !Object.values(localStorage).includes(${JSON.stringify(token)})`)) throw new Error('Token storage scope failed.');
  report.cases.push('token is session-only; absent from localStorage');
  await evaluate(`document.querySelector('#about-button').click()`);
  await until('document.querySelector("#about-dialog")?.open && document.querySelector("#about-dialog").textContent.includes("0.7.0")', 'About displays package version 0.7.0');
  await shot('connected-about.png');
  await evaluate(`document.querySelector('#about-dialog [data-close]').click()`);
  await send('Page.reload');
  await until('performance.getEntriesByType("navigation")[0]?.type === "reload" && document.querySelector("#server-status")?.classList.contains("connected") && !document.querySelector("#token-dialog")?.open', 'tab reload retains token and reconnects');
  await evaluate(`document.querySelector('#token-button').click(); document.querySelector('#token-clear').click();`);
  await until('document.querySelector("#token-dialog")?.open && sessionStorage.getItem("beta-server-token") === null', 'Forget token clears session and reopens authentication');
  await shot('token-forgotten.png');
  const statuses = report.requests.filter(r => r.path === '/api/health').map(r => r.status);
  if (statuses.filter(s => s === 401).length < 3 || statuses.filter(s => s === 200).length < 2) throw new Error(`Incomplete real health transitions: ${statuses}`);
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
  writeFileSync(join(out, 'report.json'), JSON.stringify(report, null, 2));
}
console.log(`PASS: ${report.cases.length} browser checks; ${report.externalRequests.length} external requests; ${report.exceptions.length} uncaught exceptions.`);
