// Headless full-page screenshots of dist/ at desktop and mobile widths (light theme), plus the
// home page with prefers-reduced-motion. Each page is scrolled first so scroll reveals run.
// Also reports console errors and any request that is not a local file (there should be none).
// Usage: node scripts/screenshots.mjs [--out <dir>] [--browser <chrome-or-edge.exe>]
// Uses Chrome DevTools Protocol over Node's built-in WebSocket; no npm dependencies.
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';

const args = process.argv.slice(2);
const opt = (name, fallback) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : fallback; };
const siteRoot = resolve(fileURLToPath(new URL('..', import.meta.url)));
const dist = join(siteRoot, 'dist');
const out = resolve(opt('--out', join(siteRoot, '.screens')));
const candidates = [
  opt('--browser'), process.env.CHROME_PATH,
  'C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
  '/usr/bin/google-chrome', '/usr/bin/chromium', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
].filter(Boolean);
const browser = candidates.find((p) => existsSync(p));
if (!browser) { console.error('No Chrome or Edge found. Pass --browser <path>.'); process.exit(1); }
if (!existsSync(join(dist, 'index.html'))) { console.error('Build the site first: npm run build -w @beta/marker-site'); process.exit(1); }
mkdirSync(out, { recursive: true });

const pages = ['index', 'benchmark', 'compare', 'install', 'docs', 'open-source'];
const views = [
  { name: 'desktop', width: 1440, height: 900, mobile: false },
  { name: 'mobile', width: 390, height: 844, mobile: true },
];
const shots = [];
for (const p of pages) for (const v of views) shots.push({ page: p, view: v, motion: 'motion' });
for (const v of views) shots.push({ page: 'index', view: v, motion: 'reduced' });

const profile = mkdtempSync(join(tmpdir(), 'beta-site-shots-'));
const proc = spawn(browser, ['--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--hide-scrollbars', '--force-color-profile=srgb', 'about:blank'], { stdio: 'ignore' });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function devtoolsPort() {
  const file = join(profile, 'DevToolsActivePort');
  for (let i = 0; i < 100; i++) { if (existsSync(file)) { const [port] = readFileSync(file, 'utf8').split('\n'); if (port) return port; } await sleep(100); }
  throw new Error('Browser did not start');
}

let ws; let nextId = 0; const pending = new Map(); const listeners = [];
function send(method, params = {}) {
  const id = ++nextId;
  ws.send(JSON.stringify({ id, method, params }));
  return new Promise((res, rej) => pending.set(id, { res, rej }));
}
function once(method) { return new Promise((res) => listeners.push({ method, res, once: true })); }

const report = { browser, shots: [], externalRequests: [], consoleErrors: [] };
try {
  const port = await devtoolsPort();
  const target = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: 'PUT' })).json();
  ws = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((res, rej) => { ws.onopen = res; ws.onerror = rej; });
  ws.onmessage = (e) => {
    const msg = JSON.parse(e.data);
    if (msg.id && pending.has(msg.id)) { const p = pending.get(msg.id); pending.delete(msg.id); msg.error ? p.rej(new Error(msg.error.message)) : p.res(msg.result); return; }
    if (msg.method === 'Network.requestWillBeSent') { const url = msg.params.request.url; if (!url.startsWith('file:') && !url.startsWith('data:') && !url.startsWith('about:')) report.externalRequests.push(url); }
    if (msg.method === 'Runtime.consoleAPICalled' && msg.params.type === 'error') report.consoleErrors.push(msg.params.args.map((a) => a.value ?? a.description).join(' '));
    if (msg.method === 'Runtime.exceptionThrown') report.consoleErrors.push(msg.params.exceptionDetails.text);
    for (let i = listeners.length - 1; i >= 0; i--) if (listeners[i].method === msg.method) { listeners[i].res(msg.params); listeners.splice(i, 1); }
  };
  await send('Page.enable'); await send('Network.enable'); await send('Runtime.enable');
  for (const s of shots) {
    await send('Emulation.setDeviceMetricsOverride', { width: s.view.width, height: s.view.height, deviceScaleFactor: 1, mobile: s.view.mobile });
    await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-color-scheme', value: 'light' }, { name: 'prefers-reduced-motion', value: s.motion === 'reduced' ? 'reduce' : 'no-preference' }] });
    const loaded = once('Page.loadEventFired');
    await send('Page.navigate', { url: pathToFileURL(join(dist, `${s.page}.html`)).href });
    await loaded; await sleep(300);
    // Scroll down in steps so every scroll reveal and counter runs, then wait for them to finish.
    const total = (await send('Runtime.evaluate', { expression: 'document.documentElement.scrollHeight', returnByValue: true })).result.value;
    for (let y = 0; y < total; y += Math.round(s.view.height * 0.7)) { await send('Runtime.evaluate', { expression: `window.scrollTo({ top: ${y}, behavior: 'instant' })` }); await sleep(180); }
    await sleep(1800);
    await send('Runtime.evaluate', { expression: 'window.scrollTo({ top: 0, behavior: "instant" })' });
    await sleep(2200);
    const hidden = await send('Runtime.evaluate', { expression: `Array.from(document.querySelectorAll('[data-reveal]')).filter((e) => getComputedStyle(e).opacity !== '1').length`, returnByValue: true });
    const { cssContentSize } = await send('Page.getLayoutMetrics');
    const height = Math.ceil(cssContentSize.height);
    const overflow = await send('Runtime.evaluate', { expression: 'document.documentElement.scrollWidth > window.innerWidth', returnByValue: true });
    const shot = await send('Page.captureScreenshot', { format: 'png', captureBeyondViewport: true, clip: { x: 0, y: 0, width: s.view.width, height, scale: 1 } });
    const file = join(out, `${s.page}-${s.view.width}${s.motion === 'reduced' ? '-reduced-motion' : ''}.png`);
    writeFileSync(file, Buffer.from(shot.data, 'base64'));
    report.shots.push({ file, width: s.view.width, height, horizontalOverflow: overflow.result.value, hiddenReveals: hidden.result.value });
    console.log(`${file}  ${s.view.width}x${height}${overflow.result.value ? '  (page scrolls sideways!)' : ''}${hidden.result.value ? `  (${hidden.result.value} sections still hidden!)` : ''}`);
  }
} finally {
  try { ws?.close(); } catch {}
  proc.kill();
  await sleep(500);
  try { rmSync(profile, { recursive: true, force: true }); } catch {}
}
writeFileSync(join(out, 'report.json'), JSON.stringify(report, null, 2));
console.log(`External requests: ${report.externalRequests.length}. Console errors: ${report.consoleErrors.length}.`);
if (report.externalRequests.length || report.consoleErrors.length) { console.log(report); process.exit(1); }
