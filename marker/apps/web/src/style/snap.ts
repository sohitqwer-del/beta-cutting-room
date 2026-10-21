import type { Pt } from '@beta/marker-wasm';

/** Pointer snapping for the drafting canvas. Pure functions: positions in world mm, y up.
 * Only decides where the pointer lands; every pattern coordinate is still computed by the Rust engine. */
export interface SnapTarget { id: string; at: Pt }
export interface SnapOptions {
  /** Search radius in world mm (the canvas passes about 10 screen pixels). */
  radius: number;
  points: boolean;
  /** Grid spacing in mm; 0 = off. */
  grid: number;
  /** Angle step in degrees from the anchor; 0 = off. */
  angleStep: number;
  /** Line through other points' x or y (alignment guides). */
  align: boolean;
  /** Start of the current line, for angle snapping. */
  anchor?: Pt | null;
  /** Ids to ignore, for example the point being dragged. */
  ignore?: string[];
}
export type Guide = { kind: 'line'; from: Pt; to: Pt } | { kind: 'ring'; at: Pt };
export interface SnapResult { at: Pt; kind: 'point' | 'angle' | 'align' | 'grid' | 'free'; target?: string; guides: Guide[]; angle?: number }

const dist = (a: Pt, b: Pt): number => Math.hypot(a[0] - b[0], a[1] - b[1]);
const roundTo = (v: number, step: number): number => Math.round(v / step) * step;

export function snapPoint(at: Pt, targets: SnapTarget[], options: SnapOptions): SnapResult {
  const ignore = new Set(options.ignore ?? []);
  const live = targets.filter(t => !ignore.has(t.id));
  // 1. An existing point always wins inside the radius.
  if (options.points) {
    let best: SnapTarget | undefined, bestD = options.radius;
    for (const t of live) { const d = dist(at, t.at); if (d <= bestD) { best = t; bestD = d; } }
    if (best) return { at: [...best.at], kind: 'point', target: best.id, guides: [{ kind: 'ring', at: [...best.at] }] };
  }
  // 2. Angle from the anchor (0°, 15°, 30° … by default). Keeps the distance from the anchor.
  const anchor = options.anchor;
  if (anchor && options.angleStep > 0) {
    const r = dist(anchor, at);
    if (r > options.radius) {
      const angle = Math.atan2(at[1] - anchor[1], at[0] - anchor[0]) * 180 / Math.PI;
      const snapped = roundTo(angle, options.angleStep);
      const off = Math.abs(Math.sin((angle - snapped) * Math.PI / 180) * r);
      if (off <= options.radius) {
        const rad = snapped * Math.PI / 180;
        const p: Pt = [anchor[0] + r * Math.cos(rad), anchor[1] + r * Math.sin(rad)];
        // A grid step along the snapped direction keeps lengths tidy (whole cm on a 1 cm grid).
        const length = options.grid > 0 ? Math.max(options.grid, roundTo(r, options.grid)) : r;
        const q: Pt = [anchor[0] + length * Math.cos(rad), anchor[1] + length * Math.sin(rad)];
        return { at: options.grid > 0 ? q : p, kind: 'angle', angle: ((snapped % 360) + 360) % 360, guides: [{ kind: 'line', from: [...anchor], to: options.grid > 0 ? q : p }] };
      }
    }
  }
  // 3. Alignment with other points in x and/or y, then the grid for any free axis.
  let x = at[0], y = at[1]; const guides: Guide[] = []; let aligned = false;
  if (options.align) {
    let bx: SnapTarget | undefined, by: SnapTarget | undefined, dx = options.radius, dy = options.radius;
    for (const t of live) {
      const ex = Math.abs(t.at[0] - at[0]), ey = Math.abs(t.at[1] - at[1]);
      if (ex <= dx) { dx = ex; bx = t; }
      if (ey <= dy) { dy = ey; by = t; }
    }
    if (bx) { x = bx.at[0]; aligned = true; }
    if (by) { y = by.at[1]; aligned = true; }
    if (bx) guides.push({ kind: 'line', from: [...bx.at], to: [x, y] });
    if (by) guides.push({ kind: 'line', from: [...by.at], to: [x, y] });
    if (options.grid > 0) { if (!bx) x = roundTo(x, options.grid); if (!by) y = roundTo(y, options.grid); }
    if (aligned) return { at: [x, y], kind: 'align', guides };
  }
  if (options.grid > 0) return { at: [roundTo(at[0], options.grid), roundTo(at[1], options.grid)], kind: 'grid', guides: [] };
  return { at: [...at], kind: 'free', guides: [] };
}

/** A spot on a piece edge as the Rust hit test reports it (`hit_test`): zero-based edge, distance along it from its start. */
export interface EdgeHit { segment: number; point: Pt; distance: number; along: number; edge: number; length: number }
/** Point inside a closed outline (even-odd rule). */
export function inside(outline: Pt[], at: Pt): boolean {
  let hit = false;
  for (let i = 0, j = outline.length - 1; i < outline.length; j = i++) {
    const [xi, yi] = outline[i]!, [xj, yj] = outline[j]!;
    if ((yi > at[1]) !== (yj > at[1]) && at[0] < (xj - xi) * (at[1] - yi) / (yj - yi) + xi) hit = !hit;
  }
  return hit;
}
/** Axis-aligned box test for box selection (world mm). */
export function inBox(p: Pt, a: Pt, b: Pt): boolean {
  return p[0] >= Math.min(a[0], b[0]) && p[0] <= Math.max(a[0], b[0]) && p[1] >= Math.min(a[1], b[1]) && p[1] <= Math.max(a[1], b[1]);
}

export interface ChainPath { id: string; start: Pt; end: Pt }
/** Order selected paths into one closed loop by their end points, for "Make a piece".
 * Returns edges with direction, or null when the paths do not close into one loop. */
export function chainLoop(paths: ChainPath[], tolerance = 0.05): { object: string; reversed: boolean }[] | null {
  if (paths.length < 2) return null;
  const same = (a: Pt, b: Pt) => dist(a, b) <= tolerance;
  const rest = paths.slice(1);
  const out = [{ object: paths[0]!.id, reversed: false }];
  const first = paths[0]!.start;
  let tail = paths[0]!.end;
  while (rest.length) {
    const i = rest.findIndex(p => same(p.start, tail) || same(p.end, tail));
    if (i < 0) return null;
    const p = rest.splice(i, 1)[0]!;
    const reversed = !same(p.start, tail);
    out.push({ object: p.id, reversed });
    tail = reversed ? p.start : p.end;
  }
  return same(tail, first) ? out : null;
}
