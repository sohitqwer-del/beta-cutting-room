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

/** Run work that the first screen does not need once the browser is idle (or after 2 s at the latest). */
export function whenIdle(work: () => void): void {
  const idle = (globalThis as { requestIdleCallback?: (fn: () => void, options?: { timeout: number }) => number }).requestIdleCallback;
  if (idle) idle(work, { timeout: 2000 }); else setTimeout(work, 1200);
}

/** A screen may give its own top-bar title (the Style flow shows the current step there). */
export interface ScreenTitle { eyebrow: string; title: string; heading?: boolean }
const titles = new Map<number, () => ScreenTitle | null>();
export function setScreenTitle(index: number, provider: () => ScreenTitle | null): void { titles.set(index, provider); }
export function screenTitle(index: number): ScreenTitle | null { return titles.get(index)?.() ?? null; }
/** Ask the shell to redraw the top-bar title (after a step change or a language change). */
export function refreshTitle(): void { window.dispatchEvent(new Event('beta-title')); }
