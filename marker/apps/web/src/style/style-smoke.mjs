// Page-side helpers for the Style flow smoke. A headless driver (Chrome DevTools protocol) imports this module
// in the Vite dev app, asks it where things are, sends real mouse and key input, and checks the results here.
// Usage from the driver: const s = await import('/src/style/style-smoke.mjs'); s.where('front.hem_r');

const $ = selector => document.querySelector(selector);
const canvas = () => $('#sc-canvas');

/** Client (CSS pixel) position of a world point in mm, y up, on the drafting canvas. */
export function toClient(x, y) {
  const svg = canvas(), p = svg.createSVGPoint(); p.x = x; p.y = -y;
  const q = p.matrixTransform(svg.getScreenCTM());
  return [Math.round(q.x * 10) / 10, Math.round(q.y * 10) / 10];
}
/** World position (mm) of a drawn point or curve handle, found by its id. */
export function pointAt(id) {
  for (const el of document.querySelectorAll('#sc-static circle, #sc-static rect')) {
    if (el.querySelector('title')?.textContent !== id) continue;
    if (el.tagName === 'circle') return [Number(el.getAttribute('cx')), -Number(el.getAttribute('cy'))];
    const w = Number(el.getAttribute('width'));
    return [Number(el.getAttribute('x')) + w / 2, -(Number(el.getAttribute('y')) + w / 2)];
  }
  return null;
}
export function where(id) { const p = pointAt(id); return p ? toClient(p[0], p[1]) : null; }
/** Client position of a world point between two named points (t = 0..1), for clicking on an edge. */
export function between(a, b, t = 0.5) {
  const p = pointAt(a), q = pointAt(b); if (!p || !q) return null;
  return toClient(p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t);
}
/** An empty spot on the canvas, to the right of everything drawn. */
export function emptySpot(dx = 0, dy = 0) {
  const r = canvas().getBoundingClientRect();
  return [Math.round(r.left + r.width * 0.55 + dx), Math.round(r.top + r.height * 0.5 + dy)];
}
export function client(selector) { const r = $(selector)?.getBoundingClientRect(); return r ? [r.left + r.width / 2, r.top + r.height / 2] : null; }
/** The saved session (written 0.3 s after each change). */
export function session() { try { return JSON.parse(localStorage.getItem('beta-style-v1') ?? 'null'); } catch { return null; } }
export function text(selector) { return $(selector)?.textContent?.replace(/\s+/g, ' ').trim() ?? ''; }

/** Keep exported files in memory instead of downloading them. */
const files = [];
export function captureDownloads() {
  if (window.__styleCapture) return true;
  window.__styleCapture = true;
  const create = URL.createObjectURL.bind(URL), blobs = new Map();
  URL.createObjectURL = blob => { const url = create(blob); blobs.set(url, blob); return url; };
  HTMLAnchorElement.prototype.click = function () { const blob = blobs.get(this.href); if (blob && this.download) files.push({ name: this.download, blob }); };
  window.confirm = () => true;
  return true;
}
/** Exported files: name, size and the first bytes as text, to check the format. */
export async function exported() {
  return Promise.all(files.map(async f => ({ name: f.name, bytes: f.blob.size, head: (await f.blob.slice(0, 64).text()).replace(/[^\x20-\x7e]/g, '.') })));
}
/** Accessibility spot check: controls without a name, and targets smaller than 24 px, inside the Style screen. */
export function audit() {
  const root = $('#screen-12'), out = { unnamed: [], small: [] };
  for (const el of root.querySelectorAll('button, input:not([type=hidden]), select, textarea, [role=button]')) {
    const r = el.getBoundingClientRect(); if (!r.width || !r.height || el.closest('[hidden]')) continue;
    const name = el.getAttribute('aria-label') || el.getAttribute('aria-labelledby') || el.labels?.length || el.textContent.trim() || el.title || el.querySelector('title')?.textContent;
    if (!name) out.unnamed.push(el.outerHTML.slice(0, 90));
    const target = el.matches('input[type=radio], input[type=checkbox]') ? (el.closest('label')?.getBoundingClientRect() ?? r) : r;
    if (Math.min(target.width, target.height) < 24 && el.tagName !== 'circle') out.small.push(`${el.id || el.className?.baseVal || el.className || el.tagName} ${Math.round(r.width)}x${Math.round(r.height)}`);
  }
  return out;
}
