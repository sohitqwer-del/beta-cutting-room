// BETA Cutting Room brand mark: a "B" made of three nested pattern pieces (a stem with a
// notch and two bowls) on a marker strip. One geometry for the site, the app and the icons.

/** Brand colours. Text colours pass WCAG AA on white. */
export const BRAND = {
  green: '#176b52',
  greenDark: '#0f4d3b',
  ink: '#10231c',
  amber: '#f4b740',
  mint: '#c9ecd9',
  paper: '#fbfaf6',
  white: '#ffffff',
} as const;

/** Mark geometry on a 64 x 64 grid. */
export const MARK = {
  /** Stem piece with a V notch on its left edge, like a pattern notch. */
  stem: 'M16 11H26V53H16V36L19.5 32L16 28Z',
  /** Upper bowl piece. */
  upper: 'M29 11H38.5A10 10 0 0 1 38.5 31H29Z',
  /** Lower bowl piece, a little wider, as in a real B. */
  lower: 'M29 34H40.5A9.5 9.5 0 0 1 40.5 53H29Z',
  /** Grain line with arrow heads on the stem (large sizes only). */
  grain: 'M21.5 17V47M19.1 20.2L21.5 17L23.9 20.2M19.1 43.8L21.5 47L23.9 43.8',
  /** Drill hole on the lower bowl, as marked on a pattern piece (large sizes only). */
  drill: { cx: 41, cy: 43.5, r: 1.7 },
  /** Selvedge lines of the marker strip (large sizes only). */
  selvedge: 'M8 5.5H56M8 58.5H56',
} as const;

export type MarkVariant = 'tile' | 'colour' | 'mono' | 'mono-white';

export interface MarkOptions {
  variant?: MarkVariant;
  size?: number;
  /** Adds grain line and selvedge dashes. Use for 48 px and larger. */
  detail?: boolean;
  /** Accessible name. Without it the SVG is decorative (aria-hidden). */
  label?: string;
  /** Adds xmlns so the SVG works as a standalone file. */
  standalone?: boolean;
  className?: string;
}

/** The brand mark as an SVG string. */
export function markSvg(opts: MarkOptions = {}): string {
  const v = opts.variant ?? 'tile';
  const size = opts.size ?? 32;
  const a11y = opts.label ? `role="img" aria-label="${opts.label}"` : 'aria-hidden="true" focusable="false"';
  const ns = opts.standalone ? ' xmlns="http://www.w3.org/2000/svg"' : '';
  const cls = opts.className ? ` class="${opts.className}"` : '';
  const fills: Record<MarkVariant, [string, string, string]> = {
    tile: [BRAND.white, BRAND.amber, BRAND.mint],
    colour: [BRAND.green, BRAND.amber, '#2f9e78'],
    mono: [BRAND.ink, BRAND.ink, BRAND.ink],
    'mono-white': [BRAND.white, BRAND.white, BRAND.white],
  };
  const [stem, upper, lower] = fills[v];
  const parts: string[] = [];
  if (v === 'tile') parts.push(`<rect width="64" height="64" rx="15" fill="${BRAND.green}"/>`);
  if (v === 'tile' && opts.detail) parts.push(`<path d="${MARK.selvedge}" fill="none" stroke="${BRAND.white}" stroke-opacity=".38" stroke-width="1.3" stroke-dasharray="3 2.2"/>`);
  parts.push(`<path d="${MARK.stem}" fill="${stem}"/>`, `<path d="${MARK.upper}" fill="${upper}"/>`, `<path d="${MARK.lower}" fill="${lower}"/>`);
  if (opts.detail && v !== 'mono' && v !== 'mono-white') {
    const mark = v === 'tile' ? BRAND.green : BRAND.white;
    parts.push(`<path d="${MARK.grain}" fill="none" stroke="${mark}" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>`);
    parts.push(`<circle cx="${MARK.drill.cx}" cy="${MARK.drill.cy}" r="${MARK.drill.r}" fill="${mark}"/>`);
  }
  return `<svg${ns}${cls} viewBox="0 0 64 64" width="${size}" height="${size}" ${a11y}>${parts.join('')}</svg>`;
}
