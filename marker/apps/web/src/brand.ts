import { version } from '../package.json';

export const PRODUCT = 'BETA Cutting Room';
export const VERSION = version;

/** Original BETA mark: a "B" cut like a pattern piece, with a notch and a dashed cutting line. */
export function logo(size = 32, label = ''): string {
  const a11y = label ? `role="img" aria-label="${label}"` : 'aria-hidden="true" focusable="false"';
  return `<svg class="logo" width="${size}" height="${size}" viewBox="0 0 32 32" ${a11y}>
<rect width="32" height="32" rx="7" class="logo-tile"/>
<path class="logo-letter" fill-rule="evenodd" d="M10 6.5h7.4c3.4 0 5.7 1.9 5.7 4.8 0 1.7-.9 3-2.3 3.7 2 .6 3.4 2.2 3.4 4.4 0 3.2-2.6 5.3-6.3 5.3H10zm3.6 3.3v4.3h3.3c1.4 0 2.3-.8 2.3-2.15s-.9-2.15-2.3-2.15zm0 7.4v4.6h3.8c1.6 0 2.6-.85 2.6-2.3s-1-2.3-2.6-2.3z"/>
<path class="logo-notch" d="M10 13.6l2.4 1.8-2.4 1.8z"/>
<path class="logo-cut" d="M6.5 27.5h19" stroke-dasharray="2.4 1.6"/>
</svg>`;
}

export const CREDITS: Array<[string, string, string]> = [
  ['sparrow', 'MIT', 'Nesting search heuristic'],
  ['jagua-rs', 'MPL-2.0', 'Collision detection engine, used unmodified'],
  ['FreeSewing', 'MIT', 'Open sewing patterns used as test and benchmark jobs'],
  ['Lectra nesting dataset (Zenodo 7030786)', 'CC BY 4.0', 'Public benchmark of real nesting tasks'],
];
