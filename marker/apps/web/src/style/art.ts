/** Original pictures for the Style screens (Apache-2.0). Decorative: every picture sits next to a text label,
 * so they are aria-hidden. Colours come from the design tokens and work in light and dark themes. */
const svg = (body: string, view = '0 0 120 80', cls = 's-art') => `<svg class="${cls}" viewBox="${view}" aria-hidden="true" focusable="false">${body}</svg>`;
const PIECE = 'fill="var(--draw-fill)" stroke="var(--piece-stroke)" stroke-width="2" stroke-linejoin="round"';
const FABRIC = 'fill="var(--surface-2)" stroke="var(--line-strong)" stroke-width="1.5"';
const FOLD = 'stroke="var(--fold)" stroke-width="4" stroke-linecap="round"';

export const BLOCK_ART: Record<string, string> = {
  blank: svg('<rect x="22" y="8" width="76" height="64" rx="3" fill="var(--paper)" stroke="var(--line-strong)" stroke-width="2" stroke-dasharray="5 4"/><path d="M60 30v20M50 40h20" stroke="var(--accent)" stroke-width="3" stroke-linecap="round"/>'),
  t_shirt: svg(`<path d="M44 10l-22 12 8 14 9-5v39h42V31l9 5 8-14-22-12c-3 6-9 9-16 9s-13-3-16-9z" ${PIECE}/>`),
  polo: svg(`<path d="M44 10l-22 12 8 14 9-5v39h42V31l9 5 8-14-22-12c-3 6-9 9-16 9s-13-3-16-9z" ${PIECE}/><path d="M50 12l10 10 10-10M60 22v14" fill="none" stroke="var(--piece-stroke)" stroke-width="2"/><circle cx="60" cy="27" r="1.6" fill="var(--piece-stroke)"/><circle cx="60" cy="32" r="1.6" fill="var(--piece-stroke)"/>`),
  shirt: svg(`<path d="M46 8l-22 10 4 50 12-2V30v40h40V30v36l12 2 4-50-22-10-14 12z" ${PIECE}/><path d="M60 20v50" stroke="var(--piece-stroke)" stroke-width="2"/><path d="M46 8l14 12 14-12" fill="none" stroke="var(--piece-stroke)" stroke-width="2"/><circle cx="64" cy="32" r="1.6" fill="var(--piece-stroke)"/><circle cx="64" cy="44" r="1.6" fill="var(--piece-stroke)"/><circle cx="64" cy="56" r="1.6" fill="var(--piece-stroke)"/>`),
  kurta: svg(`<path d="M47 6l-21 10 3 30 11-3v34h40V43l11 3 3-30-21-10c-2 5-7 8-13 8s-11-3-13-8z" ${PIECE}/><path d="M60 14v18" stroke="var(--piece-stroke)" stroke-width="2"/><path d="M40 66v11M80 66v11" stroke="var(--surface)" stroke-width="2"/>`),
  trouser: svg(`<path d="M38 6h44l6 68H66L60 30l-6 44H32z" ${PIECE}/><path d="M38 14h44" stroke="var(--piece-stroke)" stroke-width="2"/>`),
  track_pant: svg(`<path d="M38 6h44l4 62-4 6H66L60 30l-6 44H38l-4-6z" ${PIECE}/><path d="M38 14h44" stroke="var(--piece-stroke)" stroke-width="2" stroke-dasharray="3 2"/><path d="M84 18l-2 50M36 18l2 50" stroke="var(--draw-grain)" stroke-width="2.5"/>`),
  a_line_skirt: svg(`<path d="M44 8h32l18 64H26z" ${PIECE}/><path d="M44 15h32" stroke="var(--piece-stroke)" stroke-width="2"/>`),
};

export const LAYOUT_ART: Record<'open' | 'folded' | 'tubular', string> = {
  open: svg(`<rect x="10" y="30" width="100" height="22" ${FABRIC}/><path d="M10 30h100M10 52h100" stroke="var(--line-strong)" stroke-width="1.5"/><text x="60" y="72" text-anchor="middle" font-size="9" fill="var(--text-3)">1 layer</text>`),
  folded: svg(`<rect x="10" y="26" width="100" height="26" ${FABRIC}/><rect x="10" y="32" width="100" height="20" fill="var(--surface-3)" stroke="var(--line-strong)" stroke-width="1"/><path d="M10 26h100" ${FOLD}/><text x="60" y="72" text-anchor="middle" font-size="9" fill="var(--text-3)">2 layers · 1 fold</text>`),
  tubular: svg(`<rect x="10" y="26" width="100" height="26" rx="2" ${FABRIC}/><path d="M10 26h100M10 52h100" ${FOLD}/><text x="60" y="72" text-anchor="middle" font-size="9" fill="var(--text-3)">2 layers · 2 folds</text>`),
};
const shirtMark = (x: number, y: number, flip = false) => `<g transform="translate(${x} ${y})${flip ? ' scale(1 -1)' : ''}"><path d="M-7 -6l-6 4 3 4 3-2v10h14V0l3 2 3-4-6-4c-1 2-3 3-7 3s-6-1-7-3z" fill="var(--draw-fill)" stroke="var(--piece-stroke)" stroke-width="1.5"/></g>`;
export const FACE_ART: Record<'up' | 'down' | 'face_to_face', string> = {
  up: svg('<rect x="14" y="34" width="92" height="14" rx="2" fill="var(--accent-soft)" stroke="var(--accent)" stroke-width="2"/><path d="M60 30V12M53 19l7-7 7 7" fill="none" stroke="var(--accent)" stroke-width="2.5" stroke-linecap="round"/><text x="60" y="66" text-anchor="middle" font-size="9" fill="var(--text-3)">face up</text>'),
  down: svg('<rect x="14" y="34" width="92" height="14" rx="2" fill="var(--surface-3)" stroke="var(--line-strong)" stroke-width="2"/><path d="M60 52v18M53 63l7 7 7-7" fill="none" stroke="var(--accent)" stroke-width="2.5" stroke-linecap="round"/><text x="60" y="22" text-anchor="middle" font-size="9" fill="var(--text-3)">face down</text>'),
  face_to_face: svg('<rect x="14" y="26" width="92" height="12" rx="2" fill="var(--surface-3)" stroke="var(--line-strong)" stroke-width="2"/><rect x="14" y="42" width="92" height="12" rx="2" fill="var(--accent-soft)" stroke="var(--accent)" stroke-width="2"/><path d="M60 38v4" stroke="var(--accent)" stroke-width="2.5"/><text x="60" y="70" text-anchor="middle" font-size="9" fill="var(--text-3)">pairs, faces inside</text>'),
};
export const NAP_ART: Record<'none' | 'one_way' | 'one_way_per_bundle', string> = {
  none: svg(`<rect x="8" y="16" width="104" height="46" ${FABRIC}/>${shirtMark(34, 40)}${shirtMark(86, 40, true)}<path d="M50 40h20" stroke="var(--text-3)" stroke-width="1.5" stroke-dasharray="3 3"/>`),
  one_way: svg(`<rect x="8" y="16" width="104" height="46" ${FABRIC}/>${shirtMark(34, 40)}${shirtMark(86, 40)}<path d="M16 70h88M96 65l8 5-8 5" fill="none" stroke="var(--accent)" stroke-width="2.5"/>`),
  one_way_per_bundle: svg(`<rect x="8" y="16" width="104" height="46" ${FABRIC}/><rect x="14" y="20" width="44" height="38" rx="3" fill="none" stroke="var(--accent)" stroke-dasharray="4 3"/>${shirtMark(36, 40)}${shirtMark(86, 40, true)}<rect x="64" y="20" width="44" height="38" rx="3" fill="none" stroke="var(--size-1)" stroke-dasharray="4 3"/>`),
};
export const REPEAT_ART: Record<'none' | 'stripes' | 'checks', string> = {
  none: svg(`<rect x="14" y="12" width="92" height="56" rx="3" ${FABRIC}/>`),
  stripes: svg(`<rect x="14" y="12" width="92" height="56" rx="3" ${FABRIC}/><path d="M14 22h92M14 34h92M14 46h92M14 58h92" stroke="var(--repeat)" stroke-width="3"/>`),
  checks: svg(`<rect x="14" y="12" width="92" height="56" rx="3" ${FABRIC}/><path d="M14 26h92M14 40h92M14 54h92M32 12v56M52 12v56M72 12v56M92 12v56" stroke="var(--repeat)" stroke-width="2.5"/>`),
};
