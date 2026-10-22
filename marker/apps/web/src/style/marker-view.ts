import type { Job, MarkerResult, Pt } from '@beta/marker-wasm';

/** Marker drawing on screen: zoom, pan, fit and piece labels. Display only; all geometry comes from Rust.
 * SVG user units are millimetres. The fabric runs along x; y is drawn downwards from the far selvedge
 * (svg y = fabric width - marker y), so the near selvedge (y = 0) is at the bottom, as on the table. */
export interface Box { x: number; y: number; w: number; h: number }

/** The whole marker with a margin of `marginPx` screen pixels, keeping the screen's aspect ratio.
 * `bottomPx` stays free under the marker for the controls drawn over the bottom of the drawing. */
export function fitMarker(lengthMm: number, widthMm: number, screenW: number, screenH: number, marginPx = 12, bottomPx = 0): Box {
  const sw = Math.max(50, screenW), sh = Math.max(50, screenH), free = Math.max(1, sh - 2 * marginPx - bottomPx);
  const u = Math.max((lengthMm || 1000) / Math.max(1, sw - 2 * marginPx), (widthMm || 1000) / free);
  const w = sw * u, h = sh * u;
  // Centre the marker in the area above the reserved strip.
  return { x: (lengthMm - w) / 2, y: widthMm / 2 - (marginPx + free / 2) * u, w, h };
}
/** Fit for reading: like fitMarker, but a long marker fills at least `minShare` of the height and starts at its
 * beginning, so pieces stay readable (move along the length for the rest). `whole` is true when nothing is cut off. */
export function readableMarker(lengthMm: number, widthMm: number, screenW: number, screenH: number, marginPx = 12, bottomPx = 0, minShare = 0.45): { box: Box; whole: boolean } {
  const fit = fitMarker(lengthMm, widthMm, screenW, screenH, marginPx, bottomPx), sh = Math.max(50, screenH);
  if (widthMm / mmPerPx(fit, Math.max(50, screenW)) >= minShare * sh) return { box: fit, whole: true };
  const u = widthMm / (minShare * sh), w = Math.max(50, screenW) * u, h = sh * u;
  return { box: { x: -marginPx * u, y: widthMm / 2 - (sh - bottomPx) / 2 * u, w, h }, whole: false };
}
export const mmPerPx = (box: Box, screenW: number): number => box.w / Math.max(1, screenW);
/** Zoom by `factor` (>1 zooms in) around the point `at` in user units, within 1:20 000 and 20 px per mm. */
export function zoomBox(box: Box, factor: number, at: Pt, screenW: number): Box {
  const now = mmPerPx(box, screenW), next = Math.min(200, Math.max(0.05, now / factor)), k = next / now;
  return { x: at[0] - (at[0] - box.x) * k, y: at[1] - (at[1] - box.y) * k, w: box.w * k, h: box.h * k };
}
export function panBox(box: Box, dxPx: number, dyPx: number, screenW: number): Box {
  const u = mmPerPx(box, screenW);
  return { ...box, x: box.x - dxPx * u, y: box.y - dyPx * u };
}
export const boxText = (b: Box): string => `${b.x.toFixed(1)} ${b.y.toFixed(1)} ${b.w.toFixed(1)} ${b.h.toFixed(1)}`;

export interface PieceFacts { name: string; size: string; copy: number | null; bundle: number | null; rotation: number | null; mirrored: boolean; material: string }
/** What a cutter asks about one piece on the marker. `placement` is the index the engine gives with the outline. */
export function pieceFacts(job: Job, result: MarkerResult, pieceId: string, placement?: number): PieceFacts {
  const piece = job.pieces.find(p => p.id === pieceId);
  const p = placement != null ? result.placements[placement] : result.placements.find(q => q.piece_id === pieceId);
  const material = (piece as { material?: string } | undefined)?.material ?? '';
  return {
    name: piece?.name ?? pieceId, size: piece?.size ?? '', copy: p ? p.copy + 1 : null, bundle: p?.bundle != null ? p.bundle + 1 : null,
    rotation: p ? Math.round(p.rotation_deg * 10) / 10 : null, mirrored: !!p?.mirrored, material,
  };
}
