// Records the demo video scenes in a headless browser. It drives the real app (beta-marker serve) and the title cards.
// For the 0.8 static browser app and tour, use launch8.mjs record; this driver retains the historical workflow.
// Usage: node record.mjs --app http://127.0.0.1:7891/ --work <work dir> [--playwright <path to playwright-core>] [--chrome <exe>]
// Needs <work>/tts/durations.json (from make-video.mjs tts). Writes <work>/frames/<scene>/*.jpg and <work>/timeline.json.
//
// Frames come from the DevTools screencast (JPEG, full resolution, with timestamps), not Playwright's recordVideo,
// whose low bitrate blurs small UI text. The cursor, tap marks, highlight rings and zooms are injected into the
// recording page only; nothing is added to the app.
// Only built-in demo data, synthetic rolls and the Quick Marker practice photos are used. Never factory data.
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { allBeats, GAP, scenes } from './scenes.mjs';
import { CARDS } from './cards.mjs';

const args = Object.fromEntries(process.argv.slice(2).reduce((a, v, i, all) => (v.startsWith('--') ? [...a, [v.slice(2), all[i + 1]?.startsWith('--') || all[i + 1] === undefined ? '1' : all[i + 1]]] : a), []));
const appUrl = new URL(args.app ?? 'http://127.0.0.1:7878/');
if (!['127.0.0.1', 'localhost'].includes(appUrl.hostname)) throw new Error('Record only from a local server.');
const work = resolve(args.work ?? 'video-work');
const pw = await import(pathToFileURL(resolve(args.playwright ?? process.env.PLAYWRIGHT_CORE ?? 'node_modules/playwright-core')).href + '/index.mjs');
const chrome = args.chrome ?? process.env.CHROME_PATH ?? 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const durations = JSON.parse(readFileSync(join(work, 'tts', 'durations.json'), 'utf8'));
const only = args.only ? new Set(args.only.split(',')) : null;

const LEAD_IN = 0.35, TAIL = 0.5;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const now = () => Date.now() / 1000;
const beatsById = Object.fromEntries(allBeats().map((b) => [b.id, b]));

// ---------- Overlay injected into recording pages ----------
const OVERLAY = `(() => {
  if (window.__rec) return;
  const css = \`
  #rec-layer { position: fixed; inset: 0; pointer-events: none; z-index: 2147483647; }
  #rec-cursor { position: absolute; left: 0; top: 0; width: 34px; height: 34px; transform: translate(-200px, -200px); transition: transform .7s cubic-bezier(.3,.7,.2,1), opacity .3s; filter: drop-shadow(0 3px 4px rgba(0,0,0,.35)); }
  .rec-ripple { position: absolute; width: 22px; height: 22px; margin: -11px 0 0 -11px; border-radius: 50%; border: 4px solid #f4b740; animation: rec-rip .6s ease-out forwards; }
  .rec-tap { position: absolute; width: 56px; height: 56px; margin: -28px 0 0 -28px; border-radius: 50%; background: rgba(244,183,64,.55); border: 4px solid #f4b740; animation: rec-tap .7s ease-out forwards; }
  @keyframes rec-rip { from { transform: scale(.4); opacity: 1; } to { transform: scale(2.6); opacity: 0; } }
  @keyframes rec-tap { 0% { transform: scale(.5); opacity: 1; } 100% { transform: scale(1.6); opacity: 0; } }
  .rec-ring { position: absolute; border: 5px solid #f4b740; border-radius: 14px; box-shadow: 0 0 0 6px rgba(244,183,64,.28), 0 0 26px rgba(244,183,64,.6); opacity: 0; transition: opacity .35s, left .5s, top .5s, width .5s, height .5s; }
  .rec-ring.on { opacity: 1; animation: rec-pulse 1.4s ease-in-out infinite; }
  @keyframes rec-pulse { 50% { box-shadow: 0 0 0 12px rgba(244,183,64,.16), 0 0 34px rgba(244,183,64,.7); } }
  #rec-alive { position: absolute; left: 0; bottom: 0; width: 2px; height: 2px; background: #000; animation: rec-alive 1s steps(2) infinite; }
  @keyframes rec-alive { 50% { opacity: .02; } 0%, 100% { opacity: .01; } }
  body.rec-zoom-host { transition: transform 1s cubic-bezier(.3,.7,.2,1) !important; transform-origin: 0 0 !important; }
  \`;
  function boot() {
    const st = document.createElement('style'); st.textContent = css; document.documentElement.append(st);
    const layer = document.createElement('div'); layer.id = 'rec-layer';
    layer.innerHTML = '<svg id="rec-cursor" viewBox="0 0 24 24"><path d="M4 2 L4 20 L9 15.5 L12.5 22.5 L15.5 21 L12 14.2 L18.5 14 Z" fill="#fff" stroke="#10231c" stroke-width="1.6" stroke-linejoin="round"/></svg><div class="rec-ring" id="rec-ring"></div><div class="rec-ring" id="rec-ring2"></div><i id="rec-alive"></i>';
    document.documentElement.append(layer);
  }
  if (document.body) boot(); else document.addEventListener('DOMContentLoaded', boot);
  const $ = (id) => document.getElementById(id);
  window.__rec = {
    cursor(x, y, ms = 700) { const c = $('rec-cursor'); if (!c) return; c.style.transitionDuration = ms + 'ms'; c.style.transform = 'translate(' + (x - 4) + 'px,' + (y - 2) + 'px)'; },
    cursorShow(on) { const c = $('rec-cursor'); if (c) c.style.opacity = on ? '1' : '0'; },
    ripple(x, y, tap) { const r = document.createElement('div'); r.className = tap ? 'rec-tap' : 'rec-ripple'; r.style.left = x + 'px'; r.style.top = y + 'px'; $('rec-layer').append(r); setTimeout(() => r.remove(), 900); },
    ring(r, which = 'rec-ring', pad = 8) { const e = $(which); if (!e) return; if (!r) { e.classList.remove('on'); return; } Object.assign(e.style, { left: (r.x - pad) + 'px', top: (r.y - pad) + 'px', width: (r.width + 2 * pad) + 'px', height: (r.height + 2 * pad) + 'px' }); e.classList.add('on'); },
    zoom(r, s) {
      const b = document.body; b.classList.add('rec-zoom-host');
      if (!r) { b.style.transform = ''; this.cursorShow(true); return; }
      const W = innerWidth, H = innerHeight, cx = r.x + r.width / 2, cy = r.y + r.height / 2;
      const tx = Math.min(0, Math.max(W - W * s, W / 2 - cx * s)), ty = Math.min(0, Math.max(H - H * s, H / 2 - cy * s));
      this.cursorShow(false); this.ring(null); this.ring(null, 'rec-ring2');
      b.style.transform = 'translate(' + tx + 'px,' + ty + 'px) scale(' + s + ')';
    },
  };
})();`;

// ---------- Screencast capture ----------
class Take {
  constructor(name, page, size) { this.name = name; this.page = page; this.size = size; this.cdp = null; this.frames = []; this.dir = null; this.n = 0; this.cursor = { x: size.cssW * 0.6, y: size.cssH * 0.55 }; }
  async start(dir) {
    this.dir = dir; this.frames = []; this.n = 0;
    rmSync(dir, { recursive: true, force: true }); mkdirSync(dir, { recursive: true });
    this.cdp ??= await this.page.context().newCDPSession(this.page);
    this.cdp.removeAllListeners('Page.screencastFrame');
    this.cdp.on('Page.screencastFrame', (e) => {
      const file = `${String(this.n++).padStart(6, '0')}.jpg`;
      writeFileSync(join(this.dir, file), Buffer.from(e.data, 'base64'));
      this.frames.push({ file, t: e.metadata.timestamp ?? now() });
      this.cdp.send('Page.screencastFrameAck', { sessionId: e.sessionId }).catch(() => {});
    });
    await this.cdp.send('Page.startScreencast', { format: 'jpeg', quality: 92, maxWidth: this.size.w, maxHeight: this.size.h, everyNthFrame: 1 });
  }
  async stop() { await this.cdp.send('Page.stopScreencast'); await sleep(150); return this.frames; }
}

// ---------- Scene clock with hidden (cut) intervals ----------
class Clock {
  constructor() { this.start = now(); this.pauses = []; this.pausedTotal = 0; }
  elapsed() { return now() - this.start - this.pausedTotal; }
  async until(t) { while (this.elapsed() < t) await sleep(20); }
  async hidden(fn) { const a = now(); try { return await fn(); } finally { const b = now(); this.pauses.push([a, b]); this.pausedTotal += b - a; } }
}

// ---------- Helpers for actions ----------
function helpers(take, clock) {
  const page = take.page;
  const rec = (fn, ...a) => page.evaluate(([f, a]) => window.__rec?.[f]?.(...a), [fn, a]);
  const box = async (sel) => { const l = typeof sel === 'string' ? page.locator(sel).first() : sel; await l.scrollIntoViewIfNeeded().catch(() => {}); const b = await l.boundingBox(); if (!b) throw new Error(`No box for ${sel}`); return b; };
  const phone = take.name === 'phone';
  const h = {
    page, clock, sleep, rec,
    hidden: (fn) => clock.hidden(fn),
    async move(sel, ms = 750) { const b = await box(sel); const x = b.x + b.width / 2, y = b.y + b.height / 2; if (!phone) { await rec('cursor', x, y, ms); await sleep(ms + 120); } take.cursor = { x, y }; return { x, y, b }; },
    async click(sel, opts = {}) {
      const { x, y } = await h.move(sel, opts.ms ?? 750);
      await rec('ripple', x, y, phone);
      if (phone) await page.locator(typeof sel === 'string' ? sel : 'body').first().tap().catch(async () => (typeof sel === 'string' ? page.locator(sel).first() : sel).click());
      else await (typeof sel === 'string' ? page.locator(sel).first() : sel).click();
      await sleep(opts.after ?? 350);
    },
    async type(sel, text, opts = {}) {
      await h.click(sel, { after: 120 });
      const l = typeof sel === 'string' ? page.locator(sel).first() : sel;
      await l.press('Control+A'); await l.pressSequentially(String(text), { delay: opts.delay ?? 120 });
      await l.dispatchEvent('change'); await sleep(250);
    },
    async ring(sel, which = 'rec-ring', pad = 8) { if (!sel) return rec('ring', null, which); const b = await box(sel); await rec('ring', { x: b.x, y: b.y, width: b.width, height: b.height }, which, pad); },
    async zoom(sel, s = 1.6) { if (!sel) { await rec('zoom', null); await sleep(1050); return; } const b = await box(sel); await rec('zoom', { x: b.x, y: b.y, width: b.width, height: b.height }, s); await sleep(1050); },
    // Zoom on the smallest elements inside root that contain each text.
    async zoomText(root, texts, s) {
      const r = await page.evaluate(([root, texts]) => {
        const rs = texts.map((t) => [...document.querySelector(root).querySelectorAll('*')].filter((e) => e.textContent.includes(t) && e.getBoundingClientRect().width > 0).pop().parentElement.getBoundingClientRect());
        const x = Math.min(...rs.map((r) => r.x)), y = Math.min(...rs.map((r) => r.y));
        return { x, y, width: Math.max(...rs.map((r) => r.right)) - x, height: Math.max(...rs.map((r) => r.bottom)) - y };
      }, [root, texts]);
      await rec('zoom', r, s); await sleep(1050);
    },
    async scrollTo(sel, block = 'center') { await page.locator(sel).first().evaluate((e, block) => e.scrollIntoView({ behavior: 'smooth', block }), block); await sleep(800); },
    async waitFor(fn, ms = 90000, arg) { await page.waitForFunction(fn, arg, { timeout: ms, polling: 100 }); },
    eval: (fn, arg) => page.evaluate(fn, arg),
  };
  return h;
}

// ---------- Scene preparation, run before capture starts (not recorded) ----------
const PREP = {
  async pattern(h) {
    const p = h.page;
    await p.evaluate(() => document.querySelector('button[data-screen="4"]').click());
    await h.waitFor(() => document.querySelectorAll('[data-mode]').length > 3);
    await p.evaluate(() => document.querySelector('#pattern-starter').click()); await h.sleep(300);
    await p.evaluate(() => document.querySelector('[data-mode="measurements"]').click()); await h.sleep(200);
    await p.evaluate(() => { const e = document.querySelector('[data-pd-measure="chest"][data-size="M"]'); if (e) { e.value = '940'; e.dispatchEvent(new Event('change', { bubbles: true })); } });
    await h.sleep(400); await p.evaluate(() => window.scrollTo(0, 0)); await h.sleep(300);
  },
};

// ---------- App actions (desktop, 1440 x 810 CSS px rendered at 1920 x 1080) ----------
const setValue = (page, sel, v) => page.evaluate(([s, v]) => { const e = document.querySelector(s); e.value = String(v); e.dispatchEvent(new Event('input', { bubbles: true })); e.dispatchEvent(new Event('change', { bubbles: true })); }, [sel, v]);
const dismissToast = (page) => page.evaluate(() => document.querySelector('#message button, .toast-close')?.click());

const ACTIONS = {
  async markerDemo(h) { await h.click('#demo', { after: 900 }); },
  async markerWidth(h) {
    await dismissToast(h.page);
    const width = h.page.locator('#job-form input').first();
    await h.ring(width); await h.type(width, '1500', { delay: 160 }); await h.sleep(900); await h.ring(null);
  },
  async markerRatio(h) {
    const L = h.page.getByLabel('L', { exact: true }), M = h.page.getByLabel('M', { exact: true });
    await L.evaluate((e) => e.scrollIntoView({ behavior: 'smooth', block: 'center' })); await h.sleep(700);
    await h.ring(L.locator('xpath=ancestor::*[count(.//input)>=2][1]'));
    await h.type(L, '2'); await h.type(M, '2'); await h.sleep(400); await h.ring(null);
  },
  async markerNest(h) {
    await h.ring('#time'); await h.type('#time', '6'); await h.ring(null);
    await h.ring('#nest'); await h.click('#nest', { after: 300 }); await h.ring(null);
  },
  async markerWait(h) {
    await h.sleep(300);
    if (await h.page.locator('#nest-progress:not([hidden])').count()) { await h.zoom('#nest-progress', 1.55); }
    await h.waitFor(() => document.querySelector('#nest-progress')?.hidden && !document.querySelector('#app')?.hasAttribute('aria-busy'));
    await h.sleep(500); await h.zoom(null); await dismissToast(h.page);
    await h.page.locator('#fit').click(); await h.sleep(400);
  },
  async markerResult(h) { await h.zoomText('#stats', ['Marker length', 'Efficiency'], 1.9); },
  async markerChecks(h) { await h.zoom('#check-status', 2.1); await h.sleep(2600); await h.zoom(null); },
  async markerExport(h) {
    await h.click('#export-menu > summary', { after: 500 });
    await h.ring(h.page.getByRole('button', { name: 'PDF, full size' })); await h.sleep(1300);
    await h.ring(h.page.getByRole('button', { name: 'PLT plotter file' })); await h.sleep(1500); await h.ring(null);
  },
  async planOpen(h) { await h.page.locator('#export-menu').evaluate((d) => { d.open = false; }); await h.click('button[data-screen="1"]', { after: 600 }); },
  async planFill(h) {
    await h.ring('#plan-quantities'); await h.type('#quantity-0', '40'); await h.type('#quantity-1', '40'); await h.ring(null);
    await h.ring('#max-plies'); await h.type('#max-plies', '20'); await h.sleep(300); await h.ring(null);
  },
  async planRun(h) {
    await h.click('#make-plan', { after: 200 });
    await h.waitFor(() => document.querySelector('#plan-error')?.textContent.includes('Plan ready'));
    await h.sleep(300); await h.scrollTo('#plan-result', 'start'); await h.zoom('#plan-result', 1.25); await h.sleep(1200); await h.zoom(null);
  },
  async rolls(h) {
    await h.hidden(async () => {
      await h.page.evaluate(() => document.querySelector('button[data-screen="6"]').click()); await h.sleep(300);
      for (const [id, len] of [['DEMO-001', 100]]) await addRoll(h.page, id, len);
      await fillRoll(h.page, 'DEMO-002', 85);
      await h.page.locator('#roll-form').evaluate((e) => e.scrollIntoView({ block: 'start' })); await h.sleep(300);
    });
    await h.ring('#roll-form'); await h.click(h.page.locator('#roll-form button[type="submit"]'), { after: 200 });
    await h.waitFor(() => document.querySelector('#roll-status')?.textContent.includes('DEMO-002'));
    await h.ring(null); await h.scrollTo('#inspect-rolls');
    await h.click('#inspect-rolls', { after: 200 });
    await h.waitFor(() => document.querySelector('#roll-status')?.textContent.includes('Inspection ready'));
    await h.scrollTo('#inspection-result', 'start'); await h.ring('#inspection-result'); await h.sleep(1500); await h.ring(null);
  },
  async spreading(h) {
    await h.hidden(async () => {
      const p = h.page;
      const jsClick = (sel) => p.evaluate((sel) => document.querySelector(sel).click(), sel);
      await jsClick('button[data-screen="8"]');
      await h.waitFor(() => !document.querySelector('#floor-status')?.textContent.includes('Loading') && !document.querySelector('#screen-8')?.hasAttribute('aria-busy'));
      await jsClick('#floor-use-rolls'); await h.sleep(300);
      await jsClick('[data-floor-page="spreading"]');
      await h.waitFor(() => document.querySelectorAll('[data-floor-roll]').length >= 2);
      for (const [id, v] of [['floor-lay-id', 'DEMO-LAY-01'], ['floor-order', 'SAMPLE-ORDER'], ['floor-plies', 20], ['floor-date', '2026-10-20']]) await setValue(p, `#${id}`, v);
      await p.evaluate(() => { const c = document.querySelector('[data-floor-roll]'); c.checked = true; c.dispatchEvent(new Event('change', { bubbles: true })); });
      await p.locator('#floor-prepare-form').evaluate((e) => e.scrollIntoView({ block: 'center' })); await h.sleep(300);
    });
    await h.click(h.page.locator('#floor-prepare-form button[type="submit"]').last(), { after: 200 });
    await h.waitFor(() => document.querySelector('#floor-sheet-summary')?.textContent.includes('DEMO-LAY-01'));
    await h.scrollTo('#floor-sheet-summary', 'start'); await h.ring('#floor-sheet-summary'); await h.sleep(1600); await h.ring(null);
  },
  async bundles(h) {
    await h.click('button[data-screen="7"]', { after: 500 });
    await h.ring('#make-bundles'); await h.click('#make-bundles', { after: 200 }); await h.ring(null);
    await h.waitFor(() => document.querySelector('#bundle-status')?.textContent.includes('bundles ready'));
    await h.sleep(400); await h.scrollTo('#bundle-status', 'start'); await h.sleep(1800);
  },
  async patternGrade(h) {
    await h.click('[data-mode="grading"]', { after: 300 }); await h.click('#pd-fit', { after: 300 });
    await h.waitFor(() => document.querySelectorAll('#pd-canvas polygon').length >= 4);
    await h.scrollTo('#pd-canvas', 'end'); await h.ring('#pd-canvas'); await h.sleep(1800); await h.ring(null);
  },
  async patternSeams(h) {
    await h.page.evaluate(() => window.scrollTo({ top: 0, behavior: 'smooth' })); await h.sleep(500);
    await h.click('[data-mode="checks"]', { after: 600 });
    const check = h.page.getByRole('button', { name: 'Check all sizes' });
    await h.ring(check); await h.click(check, { after: 900 }); await h.ring(null);
    await h.scrollTo('#pd-controls', 'end'); await h.ring('#pd-controls'); await h.sleep(2200); await h.ring(null);
  },
  async patternOutput(h) {
    await h.page.evaluate(() => window.scrollTo({ top: 0, behavior: 'smooth' })); await h.sleep(400);
    await h.click('[data-mode="output"]', { after: 600 });
    await h.ring('#pd-controls'); await h.sleep(2200); await h.ring(null);
  },
  async feedback(h) {
    await h.ring('#feedback-button'); await h.sleep(700); await h.click('#feedback-button', { after: 400 }); await h.ring(null);
    await h.sleep(1600); await h.page.keyboard.press('Escape'); await h.sleep(300);
  },
  // ---------- Phone (390 x 844 CSS px at 2x) ----------
  async quickStart(h) { await h.sleep(600); await h.ring('#q-camera + *, label[for="q-camera"], .q-start .primary, #q-take', 'rec-ring', 6).catch(() => {}); await h.sleep(2500); await h.ring(null); },
  async quickPhoto(h) {
    await h.scrollTo('#q-sample'); await h.ring('#q-sample'); await h.sleep(500); await h.click('#q-sample', { after: 200 }); await h.ring(null);
    await h.waitFor(() => document.querySelector('#screen-11')?.dataset.step === 'capture' && document.querySelectorAll('.q-photo-card').length === 2 && !document.querySelector('.q-status.q-reading'), 60000);
    await h.sleep(1500);
  },
  async quickReview(h) {
    await h.click('#q-to-review', { after: 400 });
    await h.waitFor(() => document.querySelector('#screen-11')?.dataset.step === 'review');
    await h.sleep(1200);
    const piece = h.page.locator('.q-piece').filter({ has: h.page.locator('strong', { hasText: /^Piece 1$/ }) }).first();
    await h.click(piece, { after: 300 }); await h.waitFor(() => document.querySelector('#q-sheet')?.open);
    await h.sleep(800);
    if (await h.page.locator('#q-chip-name').count()) await h.click('#q-chip-name', { after: 500 });
    await h.click('#q-accept', { after: 500 });
    await h.page.evaluate(() => document.querySelector('#q-sheet')?.close()); await h.sleep(300);
    await h.click('#q-accept-all', { after: 600 });
  },
  async quickOrder(h) {
    await h.click('#q-to-order', { after: 400 });
    await h.waitFor(() => document.querySelector('#screen-11')?.dataset.step === 'order');
    await h.click('[data-q="preset"][data-value="58"]', { after: 400 });
    await h.click('[data-q="garments"][data-delta="1"]', { after: 300 });
    await h.click('#q-budget-20', { after: 300 });
  },
  async quickNest(h) {
    await h.click('#q-make', { after: 300 });
    await h.waitFor(() => document.querySelector('#screen-11')?.dataset.step === 'use' || (document.querySelector('#q-best-len')?.textContent ?? '–') !== '–', 60000);
    await h.sleep(700);
    if (await h.page.locator('#q-stop').count()) await h.click('#q-stop', { after: 300 });
    await h.waitFor(() => document.querySelector('#screen-11')?.dataset.step === 'use', 90000);
    await h.sleep(400); await h.zoom('.q-result', 1.5); await h.sleep(1600); await h.zoom(null);
  },
  async quickUse(h) {
    await h.scrollTo('#q-tiles', 'center'); await h.ring('#q-tiles'); await h.sleep(900);
    await h.ring('#q-chalk'); await h.sleep(900); await h.ring('#q-projector-btn'); await h.sleep(700); await h.ring(null);
    await h.click('#q-projector-btn', { after: 300 });
    await h.waitFor(() => !document.querySelector('#q-projector')?.hidden);
    await h.sleep(2200);
  },
};
async function fillRoll(page, id, len) {
  for (const [f, v] of [['roll-id', id], ['roll-lot', 'SAMPLE-LOT'], ['roll-dye', 'DEMO-NAVY'], ['roll-ticket', len], ['roll-length', len]]) await setValue(page, `#${f}`, v);
}
async function addRoll(page, id, len) {
  await fillRoll(page, id, len);
  await page.locator('#roll-form').evaluate((f) => f.requestSubmit());
  await page.waitForFunction((id) => document.querySelector('#roll-status')?.textContent.includes(`Roll ${id} saved`), id, { timeout: 30000 });
}

// ---------- Runner ----------
const browser = await pw.chromium.launch({ executablePath: chrome, headless: true, args: ['--hide-scrollbars', '--font-render-hinting=none'] });
const report = { startedAt: new Date().toISOString(), app: appUrl.href, errors: [], external: [], scenes: [] };
async function newTake(name, opts, size) {
  const ctx = await browser.newContext({ ...opts, acceptDownloads: true, colorScheme: 'light', locale: 'en-IN', reducedMotion: 'no-preference' });
  await ctx.addInitScript(OVERLAY);
  const page = await ctx.newPage();
  page.on('pageerror', (e) => report.errors.push(`${name}: ${e.message}`));
  page.on('request', (r) => { const u = r.url(); if (/^https?:/.test(u) && !u.startsWith(appUrl.origin)) report.external.push(u); });
  return new Take(name, page, size);
}
const takes = {
  card: await newTake('card', { viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1 }, { w: 1920, h: 1080, cssW: 1920, cssH: 1080 }),
  vcard: await newTake('vcard', { viewport: { width: 1080, height: 1920 }, deviceScaleFactor: 1 }, { w: 1080, h: 1920, cssW: 1080, cssH: 1920 }),
  app: await newTake('app', { viewport: { width: 1440, height: 810 }, deviceScaleFactor: 4 / 3 }, { w: 1920, h: 1080, cssW: 1440, cssH: 810 }),
  phone: await newTake('phone', { viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true }, { w: 780, h: 1688, cssW: 390, cssH: 844 }),
};

// Prepare the app pages (not recorded).
{
  const p = takes.app.page;
  await p.goto(appUrl.href);
  await p.waitForFunction(() => document.querySelector('#server-status')?.classList.contains('connected') && document.querySelector('#wasm-status')?.textContent.includes('Rust WASM is active'), null, { timeout: 60000 });
  await p.selectOption('#app-theme', 'light'); await p.selectOption('#app-language', 'en');
  await p.locator('button[data-screen="0"]').first().click(); await sleep(500);
  await p.evaluate(() => { localStorage.setItem('beta-plan-defaults-video', '1'); });
  // Advanced cut-plan limits are set before recording; the scene types only what the narration names.
  for (const [s, v] of [['#max-garments', 2], ['#plan-time', 0.3], ['#end-allowance', 10]]) await setValue(p, s, v);
  const q = takes.phone.page;
  await q.goto(new URL('#/quick', appUrl).href);
  await q.waitForFunction(() => document.querySelector('#screen-11')?.dataset.step === 'start' && !document.querySelector('#screen-11').hidden, null, { timeout: 60000 });
  await q.evaluate(() => window.scrollTo(0, 0));
}

const timeline = { generated: new Date().toISOString(), gap: GAP, leadIn: LEAD_IN, tail: TAIL, scenes: [] };
// Vertical replays of some cards for the 9:16 teaser use the same narration timing.
const runList = [...scenes.map((s) => ({ ...s, key: s.id })), ...scenes.filter((s) => s.vertical).map((s) => ({ ...s, key: `v-${s.id}`, take: 'vcard' }))];
for (const scene of runList) {
  if (only && !only.has(scene.key)) continue;
  const take = takes[scene.take];
  if (scene.take === 'card' || scene.take === 'vcard') {
    await take.page.setContent(CARDS[scene.card]({ vertical: scene.take === 'vcard' })); await take.page.evaluate(() => document.fonts.ready); await sleep(300);
  }
  if (PREP[scene.key]) await PREP[scene.key](helpers(take, new Clock()));
  await take.start(join(work, 'frames', scene.key));
  const clock = new Clock();
  const h = helpers(take, clock);
  if (scene.take === 'app') await h.rec('cursor', take.cursor.x, take.cursor.y, 0);
  const beats = [];
  let next = LEAD_IN;
  for (const [i, beat] of scene.beats.entries()) {
    const id = `${scene.id}-${i + 1}`;
    const dur = durations[id]; if (dur === undefined) throw new Error(`No narration for ${id}; run the tts step.`);
    await clock.until(next);
    const at = clock.elapsed();
    beats.push({ id, at, dur });
    const len = Math.max(dur + GAP, beat.min ?? 0);
    try {
      if (typeof beat.act === 'number') await take.page.evaluate((n) => window.beat(n), beat.act);
      else await ACTIONS[beat.act](h);
    } catch (e) {
      report.errors.push(`${id}: ${e.message.split('\n')[0]}`);
      console.error(`Action failed in ${id}: ${e.message}`);
    }
    next = Math.max(at + len, clock.elapsed());
  }
  await clock.until(next + TAIL);
  const end = now();
  const frames = await take.stop();
  // Map frame times to scene time, dropping frames inside hidden intervals.
  const pauses = clock.pauses;
  const mapped = [];
  const hiddenBefore = (t) => pauses.filter(([, b]) => b <= t).reduce((s, [a, b]) => s + (b - a), 0);
  for (const [i, f] of frames.entries()) {
    if (f.t < clock.start - 1 || f.t > end + 1) continue;
    const pause = pauses.find(([a, b]) => f.t >= a && f.t < b);
    if (pause) {
      // The screen changed off camera: show the newest hidden frame at the cut, unless a later hidden frame exists.
      const nextT = frames[i + 1]?.t ?? Infinity;
      if (nextT < pause[1]) continue;
      mapped.push({ file: f.file, t: Math.max(0, pause[1] - clock.start - hiddenBefore(pause[1])) });
      continue;
    }
    mapped.push({ file: f.file, t: Math.max(0, f.t - clock.start - hiddenBefore(f.t)) });
  }
  mapped.sort((a, b) => a.t - b.t);
  const duration = end - clock.start - clock.pausedTotal;
  timeline.scenes.push({ id: scene.key, scene: scene.id, take: scene.take, size: { w: take.size.w, h: take.size.h }, duration, beats, frames: mapped });
  report.scenes.push({ id: scene.key, duration: +duration.toFixed(2), frames: mapped.length, hidden: +clock.pausedTotal.toFixed(2) });
  console.log(`${scene.key}: ${duration.toFixed(1)} s, ${mapped.length} frames${clock.pausedTotal ? `, ${clock.pausedTotal.toFixed(1)} s hidden` : ''}`);
}
await browser.close();
const tlFile = join(work, only ? 'timeline-partial.json' : 'timeline.json');
if (only && (() => { try { readFileSync(join(work, 'timeline.json')); return true; } catch { return false; } })()) {
  // Merge re-recorded scenes into the full timeline.
  const full = JSON.parse(readFileSync(join(work, 'timeline.json'), 'utf8'));
  full.scenes = full.scenes.map((s) => timeline.scenes.find((n) => n.id === s.id) ?? s);
  writeFileSync(join(work, 'timeline.json'), JSON.stringify(full));
} else writeFileSync(tlFile, JSON.stringify(timeline));
writeFileSync(join(work, 'record-report.json'), JSON.stringify(report, null, 2));
if (report.external.length) console.error(`External requests: ${report.external.join(', ')}`);
if (report.errors.length) { console.error(`Problems:\n  ${report.errors.join('\n  ')}`); process.exitCode = 1; }
void beatsById;
