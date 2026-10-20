import type { Pt } from '@beta/marker-wasm';
/** SVG view in drawing coordinates (y down). */
export interface View { x: number; y: number; width: number; height: number }
/** Fit the whole strip without distorting millimetres. */
export function fitView(length: number, width: number, aspect: number): View {
  const padding = Math.max(width, length, 100) * 0.07;
  const w = Math.max(length + padding * 2, (width + padding * 2) * aspect);
  const h = w / aspect;
  return { x: (length - w) / 2, y: (width - h) / 2, width: w, height: h };
}
/** Zoom around a point; the anchor stays at the same screen position. */
export function zoomAt(view: View, anchor: Pt, factor: number): View {
  const scale = Math.max(20 / view.width, Math.min(200000 / view.width, factor));
  return { x: anchor[0] - (anchor[0] - view.x) * scale, y: anchor[1] - (anchor[1] - view.y) * scale,
    width: view.width * scale, height: view.height * scale };
}
/** Map an SVG drawing point back to marker axes (y up). */
export function markerPoint(drawing: Pt, fabricWidth: number): Pt { return [drawing[0], fabricWidth - drawing[1]]; }
/** Pixel position to view coordinates; useful for pointer interaction and tests. */
export function screenPoint(view: View, pixel: Pt, width: number, height: number): Pt {
  return [view.x + pixel[0] * view.width / width, view.y + pixel[1] * view.height / height];
}
/** Readable metre ruler ticks, about eight divisions across the current view. */
export function rulerTicks(view: View): number[] {
  const raw = view.width / 8;
  const power = 10 ** Math.floor(Math.log10(raw));
  const step = [1, 2, 2.5, 5, 10].find(n => n * power >= raw)! * power;
  const start = Math.max(0, Math.ceil(view.x / step) * step);
  const ticks: number[] = [];
  for (let x = start; x <= view.x + view.width + 1e-6 && ticks.length < 30; x += step) ticks.push(x);
  return ticks;
}
