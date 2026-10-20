/** Original pictures for the Quick Marker guide (Apache-2.0, drawn for this app). Fixed colours: they show a real
 * scene (dark cloth, white paper), so they do not follow the theme. All are decorative: the text says the same. */
const piece = '#e9dcc0', cloth = '#2b2f3a', edge = '#8a7a5a';
const frame = (body: string) => `<svg class="q-art" viewBox="0 0 120 80" aria-hidden="true" focusable="false"><rect x="1" y="1" width="118" height="78" rx="8" fill="#f2f5f3"/>${body}</svg>`;
const pieces = `<path d="M22 22h22v34l-7 6H22z" fill="${piece}" stroke="${edge}"/><path d="M50 20h24v30l-6 10H54l-4-8z" fill="${piece}" stroke="${edge}"/><path d="M28 64h18v8H28z" fill="${piece}" stroke="${edge}"/>`;

export const ART = {
  cloth: frame(`<rect x="10" y="10" width="100" height="60" rx="4" fill="${cloth}"/>${pieces}<path d="M80 22h18v20H80z" fill="${piece}" stroke="${edge}"/>`),
  a4: frame(`<rect x="10" y="10" width="100" height="60" rx="4" fill="${cloth}"/>${pieces}<rect x="82" y="18" width="20" height="28" fill="#fff" stroke="#c9ced6"/><text x="92" y="35" text-anchor="middle" font-size="8" font-weight="700" fill="#4a5560" font-family="sans-serif">A4</text>`),
  above: frame(`<path d="M12 68h96" stroke="#56655e" stroke-width="2"/><rect x="18" y="62" width="84" height="5" rx="1" fill="${cloth}"/><rect x="34" y="60" width="22" height="3" fill="${piece}"/><rect x="62" y="60" width="16" height="3" fill="#fff"/>
    <rect x="40" y="12" width="40" height="8" rx="3" fill="#1d2028"/><circle cx="60" cy="20" r="2" fill="#7db3ff"/>
    <path d="M44 26l-8 28M60 26v28M76 26l8 28" stroke="#3865ac" stroke-width="1.6" stroke-dasharray="3 3"/>
    <circle cx="101" cy="22" r="9" fill="#1c6a3e"/><path d="M96.5 22.5l3 3 5.5-6" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>`),
  target: frame(`<rect x="22" y="12" width="76" height="56" fill="#fff" stroke="#c9ced6"/>${Array.from({ length: 6 }, (_, i) => `<rect x="${26 + i * 12}" y="16" width="6" height="6" fill="#14201b"/><rect x="${26 + i * 12}" y="58" width="6" height="6" fill="#14201b"/>`).join('')}<path d="M40 28h22v24H40z" fill="${piece}" stroke="${edge}"/>`),
  sheet: frame(`<rect x="10" y="10" width="100" height="60" rx="4" fill="${cloth}"/><rect x="66" y="18" width="24" height="34" fill="#fff" stroke="#c9ced6"/><text x="78" y="38" text-anchor="middle" font-size="9" font-weight="700" fill="#4a5560" font-family="sans-serif">A4</text><path d="M24 24h28v30l-6 8H24z" fill="${piece}" stroke="${edge}"/>`),
  open: `<svg class="q-art small" viewBox="0 0 80 48" aria-hidden="true" focusable="false"><rect x="6" y="16" width="68" height="16" rx="2" fill="#c4e4d2" stroke="#1f5f4a" stroke-width="1.5"/></svg>`,
  folded: `<svg class="q-art small" viewBox="0 0 80 48" aria-hidden="true" focusable="false"><rect x="6" y="22" width="68" height="10" rx="2" fill="#a9d2bf" stroke="#1f5f4a" stroke-width="1.5"/><path d="M6 22c0-8 68-8 68 0" fill="#c4e4d2" stroke="#1f5f4a" stroke-width="1.5"/><path d="M6 32h68" stroke="#bd651e" stroke-width="2" stroke-dasharray="3 2"/></svg>`,
  tubular: `<svg class="q-art small" viewBox="0 0 80 48" aria-hidden="true" focusable="false"><rect x="6" y="14" width="68" height="20" rx="10" fill="#c4e4d2" stroke="#1f5f4a" stroke-width="1.5"/><path d="M12 24h56" stroke="#1f5f4a" stroke-width="1" stroke-dasharray="2 2"/></svg>`,
};

/** Extra stroke icons for this screen, in the same style as src/icons.ts. */
export const QUICK_ICONS: Record<string, string> = {
  camera: '<path d="M4 8h3l2-3h6l2 3h3a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V9a1 1 0 0 1 1-1z"/><circle cx="12" cy="13.5" r="3.8"/>',
  gallery: '<rect x="3" y="4" width="18" height="16" rx="2"/><circle cx="9" cy="10" r="2"/><path d="M21 16l-5-5-9 9"/>',
  share: '<circle cx="18" cy="5" r="2.5"/><circle cx="6" cy="12" r="2.5"/><circle cx="18" cy="19" r="2.5"/><path d="M8.2 10.8l7.6-4.4M8.2 13.2l7.6 4.4"/>',
  projector: '<rect x="2" y="8" width="20" height="9" rx="2"/><circle cx="16" cy="12.5" r="2.5"/><path d="M6 12h4M5 17v2M19 17v2"/>',
  chalk: '<path d="M4 20l4-1L19 8l-3-3L5 16z"/><path d="M14 7l3 3"/><path d="M3 22h8" stroke-dasharray="2 2"/>',
  pair: '<path d="M4 4h6v13l-3 3H4z"/><path d="M20 4h-6v13l3 3h3z"/>',
  fold: '<path d="M12 3v18" stroke-dasharray="2.5 2"/><path d="M12 5H5v14h7"/>',
  grain: '<path d="M12 3v18M8 7l4-4 4 4M8 17l4 4 4-4"/>',
  sparkle: '<path d="M12 3l1.8 5.2L19 10l-5.2 1.8L12 17l-1.8-5.2L5 10l5.2-1.8z"/><path d="M19 16l.8 2.2L22 19l-2.2.8L19 22l-.8-2.2L16 19l2.2-.8z"/>',
  ruler: '<path d="M3 17L17 3l4 4L7 21z"/><path d="M7 13l2 2M10 10l2 2M13 7l2 2"/>',
  square: '<rect x="5" y="5" width="14" height="14"/><path d="M5 2v2M19 2v2M2 5h2M2 19h2"/>',
  contrast: '<circle cx="12" cy="12" r="9"/><path d="M12 3a9 9 0 0 1 0 18z" fill="currentColor"/>',
  install: '<rect x="7" y="2" width="10" height="20" rx="2"/><path d="M12 7v7M9 11l3 3 3-3M10 18h4"/>',
};
export function qicon(name: string): string {
  return `<svg class="icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">${QUICK_ICONS[name] ?? ''}</svg>`;
}
