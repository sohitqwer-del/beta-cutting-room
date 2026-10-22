import type { Pt } from '@beta/marker-wasm';
import { inside } from './snap';
import { boundsOf } from './view';

/** Piece labels on the drafting canvas. Display only: where the name fits inside its own piece at the current zoom.
 * A label never leaves its piece, so it cannot run over another piece. Small pieces show the name only, or nothing
 * (the piece still has a tooltip and is listed in the inspector). */
export interface LabelPlan {
  /** Anchor in world mm (y up), centre of the text block. */
  at: Pt;
  /** Font sizes in screen pixels. */
  name: number; detail: number;
  showDetail: boolean;
  /** Name shortened with "…" when it is too long for the piece. */
  text: string;
}
/** Rough width of text in ems (bold UI font, Latin). Indic scripts are wider; the factor keeps a margin. */
export const textEms = (text: string): number => [...text].reduce((w, c) => w + (/[ऀ-෿]/.test(c) ? 0.75 : /[A-Z0-9MW]/.test(c) ? 0.66 : c === ' ' ? 0.3 : 0.56), 0);
export const LABEL_MAX = 17, LABEL_MIN = 10;

/** Point well inside the outline (largest clearance from the edges on a coarse grid, refined once). */
const anchors = new WeakMap<Pt[], { at: Pt; clearance: number } | null>();
export function labelAnchor(outline: Pt[]): { at: Pt; clearance: number } | null {
  if (anchors.has(outline)) return anchors.get(outline)!;
  const found = findAnchor(outline); anchors.set(outline, found); return found;
}
function findAnchor(outline: Pt[]): { at: Pt; clearance: number } | null {
  const b = boundsOf(outline); if (!b || outline.length < 3) return null;
  const cx = (b[0] + b[2]) / 2, cy = (b[1] + b[3]) / 2, half = Math.max(b[2] - b[0], b[3] - b[1]) / 2 || 1;
  let at: Pt | null = null, score = -Infinity;
  const search = (x0: number, y0: number, x1: number, y1: number, n: number) => {
    for (let i = 0; i <= n; i++) for (let j = 0; j <= n; j++) {
      const p: Pt = [x0 + (x1 - x0) * i / n, y0 + (y1 - y0) * j / n];
      if (!inside(outline, p)) continue;
      // Clearance from the edges, with a small pull to the middle of the piece, where people look first.
      const s = edgeDistance(outline, p) * (1 - 0.12 * Math.hypot(p[0] - cx, p[1] - cy) / half);
      if (s > score) { score = s; at = p; }
    }
  };
  search(b[0], b[1], b[2], b[3], 14);
  if (!at) return null;
  const first: Pt = at, dx = (b[2] - b[0]) / 14, dy = (b[3] - b[1]) / 14;
  search(first[0] - dx, first[1] - dy, first[0] + dx, first[1] + dy, 6);
  return { at: at as Pt, clearance: edgeDistance(outline, at as Pt) };
}
export function edgeDistance(outline: Pt[], at: Pt): number {
  let d = Infinity;
  for (let i = 0, j = outline.length - 1; i < outline.length; j = i++) {
    const [ax, ay] = outline[j]!, [bx, by] = outline[i]!, vx = bx - ax, vy = by - ay, len = vx * vx + vy * vy;
    const k = len ? Math.max(0, Math.min(1, ((at[0] - ax) * vx + (at[1] - ay) * vy) / len)) : 0;
    d = Math.min(d, Math.hypot(at[0] - ax - k * vx, at[1] - ay - k * vy));
  }
  return d;
}
/** Free width of the piece along the horizontal line through `at` (world mm). */
export function chordWidth(outline: Pt[], at: Pt): number {
  const xs: number[] = [];
  for (let i = 0, j = outline.length - 1; i < outline.length; j = i++) {
    const [xi, yi] = outline[i]!, [xj, yj] = outline[j]!;
    if ((yi > at[1]) !== (yj > at[1])) xs.push(xi + (at[1] - yi) * (xj - xi) / (yj - yi));
  }
  xs.sort((a, b) => a - b);
  for (let k = 0; k + 1 < xs.length; k += 2) if (xs[k]! <= at[0] && at[0] <= xs[k + 1]!) return xs[k + 1]! - xs[k]!;
  return 0;
}
/** Plan one label. `scale` is screen pixels per mm. Returns null when not even a short name fits. */
export function planLabel(outline: Pt[], name: string, detail: string, scale: number): LabelPlan | null {
  const anchor = labelAnchor(outline); if (!anchor) return null;
  // Room in screen pixels: the chord through the anchor (less a margin) and twice the clearance for height.
  return fitLabel(anchor.at, name, detail, chordWidth(outline, anchor.at) * scale * 0.86, anchor.clearance * 2 * scale * 0.9);
}
/** Fit a name (and a detail line) into `room` x `tall` screen pixels around an anchor placed by the engine or above.
 * Too small: the name alone, then shortened with "…", then null (nothing drawn). */
export function fitLabel(at: Pt, name: string, detail: string, room: number, tall: number): LabelPlan | null {
  if (room < 28 || tall < LABEL_MIN * 1.2) return null;
  let size = Math.min(LABEL_MAX, room / Math.max(textEms(name), 1), tall / 1.25);
  let text = name;
  if (size < LABEL_MIN) {
    size = Math.min(LABEL_MAX, tall / 1.25);
    if (size < LABEL_MIN) return null;
    text = shorten(name, room / LABEL_MIN);
    size = LABEL_MIN;
    if (!text) return null;
  }
  const detailSize = Math.max(9.5, size * 0.78);
  const showDetail = !!detail && size >= 11.5 && textEms(detail) * detailSize <= room && tall >= size * 1.25 + detailSize * 1.4;
  return { at, name: Math.round(size * 10) / 10, detail: Math.round(detailSize * 10) / 10, showDetail, text };
}
/** Cut text to fit `ems` with an ellipsis; empty when fewer than three letters would remain. */
export function shorten(text: string, ems: number): string {
  if (textEms(text) <= ems) return text;
  const chars = [...text];
  while (chars.length > 3 && textEms(chars.join('') + '…') > ems) chars.pop();
  const out = chars.join('').trimEnd();
  return out.length >= 3 && textEms(out + '…') <= ems ? `${out}…` : '';
}
/** The engine's own production label ("Front | size M | cut 1 + 0 mirrored | fabric shell") repeats the canvas label. */
export const isProductionLabel = (text: string, pieceName: string): boolean => text.startsWith(`${pieceName} | size `);
