// Runs existing app smoke modules headlessly against real WASM and a loopback API.
// Usage: node marker/deploy/workflow-smoke.mjs <output dir> <verified CLI>
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { createRequire } from 'node:module';
import { createServer } from 'node:net';
async function freePort() { const probe = createServer(); await new Promise(resolve => probe.listen(0, '127.0.0.1', resolve)); const port = probe.address().port; await new Promise(resolve => probe.close(resolve)); return port; }
const require = createRequire(import.meta.url);
const out = resolve(process.argv[2]); mkdirSync(out, { recursive: true });
const cli = resolve(process.argv[3]);
if (!process.argv[3]) throw new Error('Pass output folder and verified CLI.');
const apiPort = await freePort(), webPort = await freePort();
const url = new URL(`http://127.0.0.1:${webPort}/`);
const browser = [process.env.CHROME_PATH, 'C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe'].find(p => p && existsSync(p));
if (!browser) throw new Error('No headless Chrome/Edge found.');
const env = { ...process.env, BETA_MARKER_API_URL: `http://127.0.0.1:${apiPort}` }; delete env.BETA_TOKEN;
const server = spawn(cli, ['serve', '--bind', '127.0.0.1', '--port', String(apiPort), '--web-dir', resolve('marker/apps/web/dist')], { env, stdio: 'ignore', windowsHide: true });
const vite = spawn(process.execPath, [join(dirname(require.resolve('vite/package.json')), 'bin/vite.js'), '--host', '127.0.0.1', '--port', String(webPort), '--strictPort'], { cwd: resolve('marker/apps/web'), env, stdio: 'ignore', windowsHide: true });
const profile = mkdtempSync(join(out, 'browser-profile-'));
const proc = spawn(browser, ['--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--disable-background-networking', 'about:blank'], { stdio: 'ignore', windowsHide: true });
const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
const pending = new Map(); let nextId = 0; let ws;
let loadResolve;
const loaded = () => new Promise(resolve => { loadResolve = resolve; });
const report = { startedAt: new Date().toISOString(), binarySha256: createHash('sha256').update(readFileSync(cli)).digest('hex'), cases: [], requests: [], externalRequests: [], exceptions: [], screenshots: [] };
function send(method, params = {}) {
  const id = ++nextId; ws.send(JSON.stringify({ id, method, params }));
  return new Promise((resolve, reject) => { const timer = setTimeout(() => { pending.delete(id); reject(new Error(`CDP timeout: ${method}`)); }, 120000).unref(); pending.set(id, { resolve, reject, timer }); });
}
async function evaluate(expression) {
  const value = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (value.exceptionDetails) throw new Error(value.exceptionDetails.exception?.description || JSON.stringify(value.exceptionDetails));
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
      if (server.exitCode !== null || vite.exitCode !== null) throw new Error('An owned smoke server exited.');
      try { const response = await fetch(new URL('/api/health', env.BETA_MARKER_API_URL), { signal: AbortSignal.timeout(1000) }); if (response.ok) { ready = true; break; } } catch {}
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
    if (message.id && pending.has(message.id)) { const p = pending.get(message.id); pending.delete(message.id); clearTimeout(p.timer); message.error ? p.reject(new Error(message.error.message)) : p.resolve(message.result); return; }
    if (message.method === 'Page.loadEventFired') { loadResolve?.(); loadResolve = undefined; }
    if (message.method === 'Network.responseReceived') {
      const response = message.params.response;
      if (response.url.startsWith(url.origin)) report.requests.push({ path: new URL(response.url).pathname, status: response.status });
      else if (!/^(data|blob|about):/.test(response.url)) report.externalRequests.push(response.url);
    }
    if (message.method === 'Runtime.exceptionThrown') report.exceptions.push(message.params.exceptionDetails.text);
  };
  await send('Page.enable'); await send('Network.enable'); await send('Runtime.enable');
  await send('Emulation.setFocusEmulationEnabled', { enabled: true });
  await send('Emulation.setDeviceMetricsOverride', { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false });
  await send('Browser.setDownloadBehavior', { behavior: 'allow', downloadPath: out });
  // This origin belongs only to this runner. Use a fresh document for each independent workflow.
  async function fresh(query = '') {
    let load = loaded();
    await send('Page.navigate', { url: url.href + query });
    await load;
    await until('document.querySelector("#wasm-status")?.textContent.includes("Rust WASM is active")', 'real WASM ready');
    await evaluate('localStorage.clear(); sessionStorage.clear()');
    load = loaded();
    await send('Page.reload');
    await load;
    await until('document.querySelector("#wasm-status")?.textContent.includes("Rust WASM is active")', 'clean workflow ready');
  }
  const run = async (module, fn) => evaluate(`(async()=>{const m=await import(${JSON.stringify('/src/' + module)});return await m[${JSON.stringify(fn)}]()})()`);
  for (const [name, module, functions, query] of [
    ['editor', 'editor-smoke.mjs', ['runEditorSmoke'], ''],
    ['floor', 'floor-smoke.mjs', ['runFloorSmoke','checkFloorLanguagesAndManual'], ''],
    ['pattern', 'pattern-smoke.mjs', ['runPatternSwitchChecks','runPatternSmoke','runPatternChecks','runPatternTraceChecks'], ''],
    ['production', 'pattern-production-smoke.mjs', ['runProductionSeams','runProductionLibraryPreview','applyProductionLibrary','runProductionMarksAndFold','runProductionHandoff'], ''],
    ['quick', 'quick/quick-smoke.mjs', ['runQuickSmoke'], '?quick-mock=1#/quick'],
    ['roll-audit', 'audit-smoke.mjs', ['runRollAuditChecks'], ''],
    ['incomplete-plan', 'audit-smoke.mjs', ['runIncompletePlanCheck'], ''],
    ['compare', 'audit-smoke.mjs', ['runCompareChecks'], ''],
  ]) {
    await fresh(query);
    const results = {};
    for (const fn of functions) results[fn] = await run(module, fn);
    report.cases.push({ name, results });
    await shot(`${name}.png`);
    console.log(`PASS ${name}`);
  }
  await fresh();
  const feedback = await evaluate(`(async () => {
    const frame = document.createElement('iframe'); document.body.append(frame);
    frame.contentDocument.body.innerHTML = '<div class="topbar-tools"><button id="about-button">About</button></div><section id="screen-3"></section><dialog id="about-dialog"><div class="dialog-body"></div></dialog>';
    try { return await frame.contentWindow.eval("import('/src/feedback/opener-smoke.mjs').then(m => m.runDesktopFeedbackChecks())"); }
    finally { frame.remove(); }
  })()`);
  report.cases.push({ name: 'feedback-opener', results: feedback });
  console.log('PASS feedback-opener');
  if (report.externalRequests.length || report.exceptions.length) throw new Error('External request or uncaught browser exception.');
  report.passed = true;
} finally {
  try { ws?.close(); } catch {}
  proc.kill(); // Only this script's headless browser.
  vite.kill();
  server?.kill(); // Only the CLI child created by this script, if any.
  await delay(500);
  // Resolved target is a unique child of the explicitly supplied output directory.
  if (resolve(profile).startsWith(`${out}\\`) || resolve(profile).startsWith(`${out}/`)) {
    try { rmSync(profile, { recursive: true, force: true }); } catch {}
  }
  writeFileSync(join(out, 'report.json'), JSON.stringify(report, null, 2));
}
console.log(`PASS: ${report.cases.length} browser checks; ${report.externalRequests.length} external requests; ${report.exceptions.length} uncaught exceptions.`);
