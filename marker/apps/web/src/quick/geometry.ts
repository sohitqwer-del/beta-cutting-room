import type { Pt } from '@beta/marker-wasm';

/** Drawing helpers only. Shapes, scale and nesting are computed in Rust; these place them on screen. */
export type Homography = [number, number, number, number, number, number, number, number, number];

/** Apply a row-major 3 x 3 projective map. */
export function applyHomography(h: ArrayLike<number>, [x, y]: Pt): Pt {
  const w = h[6]! * x + h[7]! * y + h[8]!;
  return [(h[0]! * x + h[1]! * y + h[2]!) / w, (h[3]! * x + h[4]! * y + h[5]!) / w];
}

/** Inverse of a 3 x 3 matrix (adjugate / determinant). Null when the map cannot be inverted. */
export function invertHomography(h: ArrayLike<number>): Homography | null {
  if (h.length !== 9) return null;
  const [a, b, c, d, e, f, g, i, j] = Array.from(h) as Homography;
  const A = e * j - f * i, B = -(d * j - f * g), C = d * i - e * g;
  const det = a * A + b * B + c * C;
  if (!Number.isFinite(det) || Math.abs(det) < 1e-12) return null;
  const inv: Homography = [
    A, -(b * j - c * i), b * f - c * e,
    B, a * j - c * g, -(a * f - c * d),
    C, -(a * i - b * g), a * e - b * d,
  ];
  return inv.map(v => v / det) as Homography;
}

/** mm (y up) → overlay pixels. imageToMm maps original pixels to mm; the overlay is the photo scaled to processedSize. */
export function mmToOverlay(imageToMm: ArrayLike<number>, sourceSize: [number, number], processedSize: [number, number]): ((p: Pt) => Pt) | null {
  const inv = invertHomography(imageToMm);
  if (!inv || !sourceSize[0] || !sourceSize[1]) return null;
  const sx = processedSize[0] / sourceSize[0], sy = processedSize[1] / sourceSize[1];
  return p => { const [x, y] = applyHomography(inv, p); return [x * sx, y * sy]; };
}

export interface Box { minX: number; minY: number; maxX: number; maxY: number; width: number; height: number }
export function bounds(points: Pt[]): Box {
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  for (const [x, y] of points) { minX = Math.min(minX, x); minY = Math.min(minY, y); maxX = Math.max(maxX, x); maxY = Math.max(maxY, y); }
  if (!points.length) minX = minY = maxX = maxY = 0;
  return { minX, minY, maxX, maxY, width: maxX - minX, height: maxY - minY };
}

/** Shoelace area in mm². */
export function area(points: Pt[]): number {
  let sum = 0;
  for (let k = 0; k < points.length; k++) { const [x1, y1] = points[k]!, [x2, y2] = points[(k + 1) % points.length]!; sum += x1 * y2 - x2 * y1; }
  return Math.abs(sum) / 2;
}

/** SVG path for a closed outline. `flipY` turns mm (y up) into screen (y down) inside a viewBox. */
export function pathData(points: Pt[], flipY = true): string {
  return points.map(([x, y], k) => `${k ? 'L' : 'M'}${x.toFixed(1)} ${(flipY ? -y : y).toFixed(1)}`).join('') + (points.length ? 'Z' : '');
}

/** A viewBox around an outline (y flipped), with a margin in mm. */
export function viewBoxFor(points: Pt[], margin = 10): string {
  const b = bounds(points);
  return `${(b.minX - margin).toFixed(1)} ${(-b.maxY - margin).toFixed(1)} ${(b.width + 2 * margin).toFixed(1)} ${(b.height + 2 * margin).toFixed(1)}`;
}

/** Turn a grain line by 90° about its middle (user correction when the grain guess is wrong). */
export function turnLine([[x1, y1], [x2, y2]]: [Pt, Pt]): [Pt, Pt] {
  const cx = (x1 + x2) / 2, cy = (y1 + y2) / 2, dx = (x2 - x1) / 2, dy = (y2 - y1) / 2;
  return [[cx + dy, cy - dx], [cx - dy, cy + dx]];
}

/** Default grain when nothing was found: along the longest side of the bounding box. */
export function longAxis(points: Pt[]): [Pt, Pt] {
  const b = bounds(points), cx = (b.minX + b.maxX) / 2, cy = (b.minY + b.maxY) / 2;
  return b.width >= b.height ? [[b.minX + b.width * .15, cy], [b.maxX - b.width * .15, cy]] : [[cx, b.minY + b.height * .15], [cx, b.maxY - b.height * .15]];
}
