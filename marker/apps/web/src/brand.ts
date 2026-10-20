import { version } from '../package.json';

export const PRODUCT = 'BETA Cutting Room';
export const VERSION = version;

/**
 * Original BETA mark: a "B" made of three nested pattern pieces (a notched stem and two bowls)
 * on a marker strip. Same geometry as marker/apps/site/src/brand.ts.
 */
export function logo(size = 32, label = ''): string {
  const a11y = label ? `role="img" aria-label="${label}"` : 'aria-hidden="true" focusable="false"';
  const detail = size >= 48
    ? '<path d="M8 5.5H56M8 58.5H56" fill="none" stroke="#fff" stroke-opacity=".38" stroke-width="1.3" stroke-dasharray="3 2.2"/>'
    : '';
  const marks = size >= 48
    ? '<path d="M21.5 17V47M19.1 20.2L21.5 17L23.9 20.2M19.1 43.8L21.5 47L23.9 43.8" fill="none" stroke="#176b52" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/><circle cx="41" cy="43.5" r="1.7" fill="#176b52"/>'
    : '';
  return `<svg class="logo" width="${size}" height="${size}" viewBox="0 0 64 64" ${a11y}>
<rect width="64" height="64" rx="15" fill="#176b52"/>${detail}
<path d="M16 11H26V53H16V36L19.5 32L16 28Z" fill="#fff"/>
<path d="M29 11H38.5A10 10 0 0 1 38.5 31H29Z" fill="#f4b740"/>
<path d="M29 34H40.5A9.5 9.5 0 0 1 40.5 53H29Z" fill="#c9ecd9"/>${marks}
</svg>`;
}

export const CREDITS: Array<[string, string, string]> = [
  ['sparrow', 'MIT', 'Nesting search heuristic'],
  ['jagua-rs', 'MPL-2.0', 'Collision detection engine, used unmodified'],
  ['FreeSewing', 'MIT', 'Open sewing patterns used as test and benchmark jobs'],
  ['Lectra nesting dataset (Zenodo 7030786)', 'CC BY 4.0', 'Public benchmark of real nesting tasks'],
];
