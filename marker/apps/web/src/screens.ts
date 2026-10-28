/** Only the active screen is in the page.
 *
 * Every screen is still built once and keeps its elements, listeners and state, but an inactive screen is "parked":
 * taken out of the document and kept in memory, with a comment node holding its place. Its text and controls never
 * reach the accessibility tree, the page stays small (fewer elements to style and lay out on every change), and
 * page-wide searches cannot pick up another screen's fields by mistake.
 *
 * Code that looks up elements must use `find`, `findAll` or `byId` instead of `document.querySelector…`: they look in
 * the page first and then in the parked screens, so a screen can still update its own elements while it is parked. */

const parked = new Set<Element>();
const marks = new WeakMap<Element, Comment>();

/** Take an element out of the page; `unpark` puts it back in the same place. Safe to call many times. */
export function park(element: Element): void {
  if (parked.has(element) || !element.isConnected) return;
  let mark = marks.get(element);
  if (!mark) { mark = document.createComment(` ${element.id || element.className} `); marks.set(element, mark); }
  element.replaceWith(mark);
  parked.add(element);
}

export function unpark(element: Element): void {
  if (!parked.has(element)) return;
  const mark = marks.get(element);
  if (mark?.isConnected) mark.replaceWith(element);
  parked.delete(element);
}

export function isParked(element: Element | null | undefined): boolean { return !!element && parked.has(element); }

/** Parked elements, for code that must reach every screen (for example the language switch). */
export function parkedRoots(): Element[] { return [...parked]; }

/** First element that matches: in the page, else in a parked screen. */
export function find<T extends Element = HTMLElement>(selector: string): T | null {
  const hit = document.querySelector<T>(selector);
  if (hit) return hit;
  for (const root of parked) {
    if (root.matches(selector)) return root as unknown as T;
    const inner = root.querySelector<T>(selector);
    if (inner) return inner;
  }
  return null;
}

/** All matches: the page first, then the parked screens. */
export function findAll<T extends Element = HTMLElement>(selector: string): T[] {
  const out = Array.from(document.querySelectorAll<T>(selector));
  for (const root of parked) {
    if (root.matches(selector)) out.push(root as unknown as T);
    out.push(...Array.from(root.querySelectorAll<T>(selector)));
  }
  return out;
}

export function byId<T extends HTMLElement = HTMLElement>(id: string): T | null {
  return (document.getElementById(id) as T | null) ?? (parked.size ? find<T>(`#${CSS.escape(id)}`) : null);
}

type Connection = { saveData?: boolean; effectiveType?: string };
/** May the browser download engines that no open screen needs yet? Not with data saver or a slow connection, and not
 * on a narrow screen when the connection is unknown (most likely a phone on mobile data). The desktop app always may. */
export function backgroundLoadAllowed(env: { desktop: boolean; connection?: Connection; wide: boolean }): boolean {
  if (env.desktop) return true;
  const c = env.connection;
  if (c?.saveData) return false;
  if (c?.effectiveType) return c.effectiveType === '4g';
  return env.wide;
}
/** Run work that the first screen does not need: an engine download a later screen will use.
 * Desktop app: once the browser is idle. Browser: only when `backgroundLoadAllowed`, and not before the page has been
 * open for 8 s, so the first screen and the first tap get the network. Screens still load what they need when they open. */
let holds = 0;
const held: Array<() => void> = [];
/** While something the visitor is waiting for downloads (the sample tour's engines), background downloads wait.
 * Returns the release function; background work queued meanwhile starts after the last release. */
export function holdBackground(): () => void {
  holds++;
  let released = false;
  return () => { if (released) return; released = true; holds--; if (!holds) held.splice(0).forEach(fn => fn()); };
}
export function whenIdle(work: () => void): void {
  const idle = (globalThis as { requestIdleCallback?: (fn: () => void, options?: { timeout: number }) => number }).requestIdleCallback;
  const run = () => { if (holds) { held.push(run); return; } if (idle) idle(work, { timeout: 2000 }); else setTimeout(work, 1200); };
  const desktop = !!(globalThis as { __TAURI__?: unknown }).__TAURI__;
  if (desktop) { run(); return; }
  const connection = (navigator as Navigator & { connection?: Connection }).connection;
  if (!backgroundLoadAllowed({ desktop, connection, wide: window.matchMedia('(min-width: 900px)').matches })) return;
  const wait = Math.max(0, 8000 - performance.now());
  setTimeout(run, wait);
}

/** A screen may give its own top-bar title (the Style flow shows the current step there). */
export interface ScreenTitle { eyebrow: string; title: string; heading?: boolean }
const titles = new Map<number, () => ScreenTitle | null>();
export function setScreenTitle(index: number, provider: () => ScreenTitle | null): void { titles.set(index, provider); }
export function screenTitle(index: number): ScreenTitle | null { return titles.get(index)?.() ?? null; }
/** Ask the shell to redraw the top-bar title (after a step change or a language change). */
export function refreshTitle(): void { window.dispatchEvent(new Event('beta-title')); }
