// Tiny headless Chrome/Edge driver over the DevTools protocol (Node's built-in WebSocket).
// No npm dependencies. Used by the brand script to render SVG and HTML to PNG.
import { spawn, type ChildProcess } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

export function findBrowser(explicit?: string): string {
  const candidates = [
    explicit, process.env.CHROME_PATH,
    'C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe',
    '/usr/bin/google-chrome', '/usr/bin/chromium', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  ].filter((p): p is string => Boolean(p));
  const found = candidates.find((p) => existsSync(p));
  if (!found) throw new Error('No Chrome or Edge found. Set CHROME_PATH.');
  return found;
}

export class Headless {
  private proc!: ChildProcess;
  private profile!: string;
  private ws!: WebSocket;
  private id = 0;
  private pending = new Map<number, { res: (v: any) => void; rej: (e: Error) => void }>();
  private waiters: { method: string; res: (p: any) => void }[] = [];

  async start(browser = findBrowser()): Promise<this> {
    this.profile = mkdtempSync(join(tmpdir(), 'beta-headless-'));
    this.proc = spawn(browser, ['--headless=new', '--remote-debugging-port=0', `--user-data-dir=${this.profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--hide-scrollbars', '--force-color-profile=srgb', 'about:blank'], { stdio: 'ignore' });
    const file = join(this.profile, 'DevToolsActivePort');
    let port = '';
    for (let i = 0; i < 100 && !port; i++) { if (existsSync(file)) port = readFileSync(file, 'utf8').split('\n')[0] ?? ''; if (!port) await sleep(100); }
    if (!port) throw new Error('Browser did not start');
    const target = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: 'PUT' })).json() as { webSocketDebuggerUrl: string };
    this.ws = new WebSocket(target.webSocketDebuggerUrl);
    await new Promise((res, rej) => { this.ws.onopen = res; this.ws.onerror = rej; });
    this.ws.onmessage = (e) => {
      const msg = JSON.parse(String(e.data));
      if (msg.id && this.pending.has(msg.id)) { const p = this.pending.get(msg.id)!; this.pending.delete(msg.id); if (msg.error) p.rej(new Error(msg.error.message)); else p.res(msg.result); return; }
      for (let i = this.waiters.length - 1; i >= 0; i--) if (this.waiters[i]!.method === msg.method) { this.waiters[i]!.res(msg.params); this.waiters.splice(i, 1); }
    };
    await this.send('Page.enable');
    return this;
  }

  send(method: string, params: object = {}): Promise<any> {
    const id = ++this.id;
    this.ws.send(JSON.stringify({ id, method, params }));
    return new Promise((res, rej) => this.pending.set(id, { res, rej }));
  }

  private once(method: string): Promise<any> { return new Promise((res) => this.waiters.push({ method, res })); }

  /** Loads a URL at a viewport size and returns a PNG of the viewport. */
  async png(url: string, width: number, height: number, opts: { transparent?: boolean; wait?: number } = {}): Promise<Buffer> {
    await this.send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: false });
    await this.send('Emulation.setDefaultBackgroundColorOverride', opts.transparent ? { color: { r: 0, g: 0, b: 0, a: 0 } } : {});
    const loaded = this.once('Page.loadEventFired');
    await this.send('Page.navigate', { url });
    await loaded;
    await this.send('Runtime.evaluate', { expression: 'document.fonts ? document.fonts.ready.then(() => true) : true', awaitPromise: true });
    await sleep(opts.wait ?? 150);
    const shot = await this.send('Page.captureScreenshot', { format: 'png', clip: { x: 0, y: 0, width, height, scale: 1 } });
    return Buffer.from(shot.data, 'base64');
  }

  async stop(): Promise<void> {
    try { this.ws?.close(); } catch { /* already closed */ }
    this.proc?.kill();
    await sleep(500);
    try { rmSync(this.profile, { recursive: true, force: true }); } catch { /* browser may still hold files */ }
  }
}
