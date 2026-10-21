import type { Pt } from '@beta/marker-wasm';
import { MM_PER_INCH, type LengthUnit } from './units';

/** Canvas view: world millimetres with y up; screen pixels with y down.
 * `left` and `top` are the world coordinates of the screen's top-left corner; `scale` is pixels per mm. */
export interface View { left: number; top: number; scale: number }
export const MIN_SCALE = 0.02, MAX_SCALE = 60;

export function toWorld(view: View, screen: Pt): Pt { return [view.left + screen[0] / view.scale, view.top - screen[1] / view.scale]; }
export function toScreen(view: View, world: Pt): Pt { return [(world[0] - view.left) * view.scale, (view.top - world[1]) * view.scale]; }
/** Zoom by `factor` (>1 zooms in) keeping the world point under `screen` still. */
export function zoomAt(view: View, screen: Pt, factor: number): View {
  const scale = Math.min(MAX_SCALE, Math.max(MIN_SCALE, view.scale * factor));
  const at = toWorld(view, screen);
  return { scale, left: at[0] - screen[0] / scale, top: at[1] + screen[1] / scale };
}
export function pan(view: View, dxPx: number, dyPx: number): View { return { ...view, left: view.left - dxPx / view.scale, top: view.top + dyPx / view.scale }; }
export type Bounds = [number, number, number, number];
export function boundsOf(points: Pt[]): Bounds | null {
  if (!points.length) return null;
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  for (const [x, y] of points) { if (x < minX) minX = x; if (y < minY) minY = y; if (x > maxX) maxX = x; if (y > maxY) maxY = y; }
  return [minX, minY, maxX, maxY];
}
/** Fit world bounds into a screen of width x height pixels with a margin in pixels. Empty bounds show 1 m x 0.8 m. */
export function fitView(bounds: Bounds | null, width: number, height: number, margin = 40): View {
  const [minX, minY, maxX, maxY] = bounds ?? [0, 0, 1000, 800];
  const w = Math.max(maxX - minX, 50), h = Math.max(maxY - minY, 50);
  const scale = Math.min(MAX_SCALE, Math.max(MIN_SCALE, Math.min((width - 2 * margin) / w, (height - 2 * margin) / h)));
  const cx = (minX + maxX) / 2, cy = (minY + maxY) / 2;
  return { scale, left: cx - width / 2 / scale, top: cy + height / 2 / scale };
}
/** SVG viewBox in world mm (y flipped: world y maps to -y). */
export function viewBox(view: View, width: number, height: number): string {
  return `${view.left} ${-view.top} ${width / view.scale} ${height / view.scale}`;
}

export interface Tick { mm: number; label: string; major: boolean }
/** Ruler ticks in the chosen unit between world `from` and `to` (mm). Major ticks are at least `minPx` apart. */
export function rulerTicks(from: number, to: number, scale: number, unit: LengthUnit, minPx = 64): Tick[] {
  const unitMm = unit === 'inch' ? MM_PER_INCH : 10;
  const steps = unit === 'inch' ? [0.125, 0.25, 0.5, 1, 2, 6, 12, 24, 48, 120] : [0.1, 0.5, 1, 2, 5, 10, 20, 50, 100, 200, 500];
  const major = steps.find(s => s * unitMm * scale >= minPx) ?? steps.at(-1)!;
  const minorIndex = steps.indexOf(major) - 1;
  const minor = minorIndex >= 0 && steps[minorIndex]! * unitMm * scale >= 10 ? steps[minorIndex]! : major;
  const out: Tick[] = [];
  const first = Math.ceil(Math.min(from, to) / (minor * unitMm)), last = Math.floor(Math.max(from, to) / (minor * unitMm));
  if (last - first > 2000) return out;
  for (let i = first; i <= last; i++) {
    const value = i * minor; const mm = value * unitMm;
    const isMajor = Math.abs(value / major - Math.round(value / major)) < 1e-9;
    out.push({ mm, major: isMajor, label: isMajor ? String(Number(value.toFixed(3))) : '' });
  }
  return out;
}
