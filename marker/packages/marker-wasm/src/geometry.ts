import type { Placement, Pt } from './types';
/** Drawing only: mirror, rotate CCW, then translate. Rust performs all checks. */
export function transform(points: readonly Pt[], mirrored: boolean, rotation: number, translate: Pt): Pt[] {
  const angle = rotation * Math.PI / 180;
  const c = Math.cos(angle), s = Math.sin(angle);
  return points.map(([px, y]) => {
    const x = mirrored ? -px : px;
    return [c * x - s * y + translate[0], s * x + c * y + translate[1]];
  });
}
/** Apply a placement for SVG drawing. */
export function placed(points: readonly Pt[], placement: Placement): Pt[] {
  return transform(points, placement.mirrored, placement.rotation_deg, placement.translate);
}
/** View bounds; null for an empty outline. */
export function bbox(points: readonly Pt[]): [number, number, number, number] | null {
  if (!points.length) return null;
  return points.reduce<[number, number, number, number]>((b, [x, y]) =>
    [Math.min(b[0], x), Math.min(b[1], y), Math.max(b[2], x), Math.max(b[3], y)],
  [Infinity, Infinity, -Infinity, -Infinity]);
}
