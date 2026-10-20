// Original illustrative SVGs (no screenshots, no vendor material). Colours come from CSS classes,
// so light and dark themes both work.
import { esc, raw, type Raw } from './html.ts';

const BODY = 'M0 22 Q8 2 30 0 L42 0 Q50 16 58 16 Q66 16 74 0 L86 0 Q104 2 100 22 L96 52 Q90 60 92 72 L94 120 L6 120 L8 72 Q10 60 4 52 Z';
const SLEEVE = 'M0 66 Q6 14 50 0 Q94 14 100 66 L92 80 L8 80 Z';
const POCKET = 'M0 0 H30 V26 L15 36 L0 26 Z';

function piece(path: string, size: number, tx: number, ty: number, opts: { rotate?: boolean; sy?: number; grain?: boolean; w?: number; h?: number } = {}): string {
  const w = opts.w ?? 100; const h = opts.h ?? 120;
  const rot = opts.rotate ? ` rotate(180 ${w / 2} ${(h * (opts.sy ?? 1)) / 2})` : '';
  const scale = opts.sy ? ` scale(1 ${opts.sy})` : '';
  const grain = opts.grain ? `<path class="grain" d="M${w / 2} ${h * 0.3} V${h * 0.82} M${w / 2 - 4} ${h * 0.36} L${w / 2} ${h * 0.3} L${w / 2 + 4} ${h * 0.36}"/>` : '';
  return `<g class="pc pc-${size}" transform="translate(${tx} ${ty})${rot}${scale}"><path d="${path}"/>${grain}</g>`;
}

/** Hero picture: a marker with three sizes nested on a fabric strip. */
export function heroMarker(): Raw {
  const p: string[] = [];
  // Size 1 (S)
  p.push(piece(BODY, 1, 24, 34, { grain: true }));
  p.push(piece(SLEEVE, 1, 24, 158, { sy: 0.85, h: 80 }));
  p.push(piece(BODY, 1, 128, 106, { rotate: true, grain: true }));
  p.push(piece(SLEEVE, 1, 128, 34, { sy: 0.85, h: 80, rotate: true }));
  // Size 2 (M)
  p.push(piece(BODY, 2, 232, 34, { grain: true }));
  p.push(piece(SLEEVE, 2, 232, 158, { sy: 0.85, h: 80 }));
  p.push(piece(BODY, 2, 336, 106, { rotate: true, grain: true }));
  p.push(piece(SLEEVE, 2, 336, 34, { sy: 0.85, h: 80, rotate: true }));
  // Size 3 (L)
  p.push(piece(BODY, 3, 440, 34, { grain: true }));
  p.push(piece(SLEEVE, 3, 440, 158, { sy: 0.85, h: 80 }));
  // Small parts
  for (const [i, x] of [548, 566, 584, 602].entries()) p.push(`<rect class="pc pc-${(i % 3) + 1}" x="${x}" y="34" width="13" height="92" rx="5"/>`);
  for (const [i, [x, y]] of [[548, 134], [584, 134], [548, 180], [584, 180]].entries()) p.push(piece(POCKET, (i % 3) + 1, x!, y!, { w: 30, h: 36 }));
  return raw(`<svg class="illus hero-marker" viewBox="0 0 640 270" role="img" aria-labelledby="hero-illus-t">
<title id="hero-illus-t">Illustration of a marker: shirt fronts, backs, sleeves, collars and pockets in three sizes nested on a strip of fabric.</title>
<rect class="fabric" x="20" y="30" width="604" height="200" rx="2"/>
<path class="selvedge" d="M20 30 H624 M20 230 H624"/>
${p.join('\n')}
<g class="dim"><path d="M10 34 V226 M6 38 L10 30 L14 38 M6 222 L10 230 L14 222"/><text x="0" y="0" transform="translate(6 130) rotate(-90)" text-anchor="middle">fabric width</text></g>
<g class="dim"><path d="M20 250 H624 M28 246 L20 250 L28 254 M616 246 L624 250 L616 254"/><text x="322" y="266" text-anchor="middle">marker length: shorter is better</text></g>
<g class="legend-sizes" transform="translate(20 8)"><rect class="pc pc-1" width="12" height="12" rx="2"/><text x="18" y="10">S</text><rect class="pc pc-2" x="40" width="12" height="12" rx="2"/><text x="58" y="10">M</text><rect class="pc pc-3" x="80" width="12" height="12" rx="2"/><text x="98" y="10">L</text></g>
</svg>`);
}

/** Slot for a real screenshot. Shows a neutral mock-up until the PNG exists. */
export function screenPlaceholder(kind: 'pattern' | 'marker' | 'table' | 'tickets' | 'rolls' | 'floor', label: string, activeTab: number): Raw {
  const content: Record<typeof kind, string> = {
    pattern: `<path class="ph-line" d="M140 60 L190 54 Q200 70 214 70 L226 120 L232 190 L142 190 Z M260 60 Q300 40 340 60 L336 190 L262 190 Z"/><path class="ph-dash" d="M150 70 L186 66 M268 70 L332 70"/><circle class="ph-dot" cx="190" cy="54" r="3"/><circle class="ph-dot" cx="226" cy="120" r="3"/>`,
    marker: `<rect class="ph-fabric" x="120" y="50" width="250" height="140"/>${[0, 1, 2, 3].map((i) => `<rect class="ph-piece ph-p${(i % 3) + 1}" x="${126 + i * 60}" y="56" width="54" height="74" rx="8"/><rect class="ph-piece ph-p${((i + 1) % 3) + 1}" x="${126 + i * 60}" y="136" width="54" height="48" rx="14"/>`).join('')}`,
    table: [0, 1, 2, 3, 4, 5].map((i) => `<rect class="${i === 0 ? 'ph-head' : 'ph-row'}" x="120" y="${50 + i * 24}" width="250" height="18" rx="3"/>`).join(''),
    tickets: [0, 1, 2, 3].map((i) => `<g transform="translate(${124 + (i % 2) * 124} ${52 + Math.floor(i / 2) * 70})"><rect class="ph-row" width="116" height="62" rx="4"/><rect class="ph-qr" x="8" y="8" width="40" height="40"/><rect class="ph-head" x="56" y="10" width="50" height="8" rx="2"/><rect class="ph-head" x="56" y="26" width="36" height="8" rx="2"/></g>`).join(''),
    rolls: [0, 1, 2, 3].map((i) => `<g transform="translate(${130 + i * 60} 70)"><rect class="ph-row" width="44" height="110" rx="6"/><ellipse class="ph-head" cx="22" cy="8" rx="22" ry="8"/></g>`).join(''),
    floor: [0, 1, 2, 3, 4].map((i) => `<rect class="ph-piece ph-p${(i % 3) + 1}" x="${120 + i * 8}" y="${60 + i * 22}" width="${240 - i * 16}" height="16" rx="2"/>`).join(''),
  };
  const tabs = ['Pattern', 'Digitize', 'Editor', 'Cut plan', 'Rolls', 'Bundles', 'Floor'];
  return raw(`<svg class="illus screen-ph" viewBox="0 0 400 230" role="img" aria-label="${esc(`Placeholder for a screenshot of ${label}`)}">
<rect class="ph-window" x="1" y="1" width="398" height="228" rx="10"/>
<path class="ph-bar" d="M1 11 Q1 1 11 1 H389 Q399 1 399 11 V24 H1 Z"/>
<circle class="ph-dot" cx="14" cy="12.5" r="3"/><circle class="ph-dot" cx="24" cy="12.5" r="3"/><circle class="ph-dot" cx="34" cy="12.5" r="3"/>
<rect class="ph-side" x="1" y="24" width="100" height="205"/>
${tabs.map((t, i) => `<rect class="${i === activeTab ? 'ph-tab-on' : 'ph-tab'}" data-tab="${t}" x="10" y="${34 + i * 22}" width="82" height="14" rx="3"/>`).join('')}
${content[kind]}
<text class="ph-text" x="245" y="215" text-anchor="middle">Screenshot coming soon</text>
</svg>`);
}

const ICONS: Record<string, string> = {
  pattern: '<path d="M6 4 L10 3 Q12 6 14 6 Q16 6 18 3 L22 4 L21 9 L19 9 L19 21 L9 21 L9 9 L7 9 Z"/>',
  marker: '<rect x="2" y="5" width="22" height="16" rx="1.5"/><path d="M6 8 V18 M11 8 V18 M16 8 L20 12 L16 18"/>',
  plan: '<rect x="3" y="4" width="20" height="18" rx="2"/><path d="M3 9 H23 M3 14 H23 M10 4 V22 M16 4 V22"/>',
  rolls: '<ellipse cx="9" cy="13" rx="5" ry="8"/><path d="M9 5 H19 Q24 5 24 13 Q24 21 19 21 H9"/><ellipse cx="9" cy="13" rx="1.5" ry="2.5"/>',
  spread: '<path d="M3 18 L13 22 L23 18 M3 14 L13 18 L23 14 M3 10 L13 14 L23 10 L13 6 Z"/>',
  cut: '<circle cx="7" cy="18" r="3"/><circle cx="7" cy="8" r="3"/><path d="M9.5 16.5 L23 6 M9.5 9.5 L23 20"/>',
  bundles: '<rect x="4" y="8" width="18" height="13" rx="1.5"/><path d="M4 12 H22 M13 8 V21 M8 8 L10 3 H16 L18 8"/>',
  sewing: '<path d="M4 20 H22 M7 20 V9 Q7 5 11 5 H20 V12 H11 M17 12 V17"/><path d="M17 17 L17 19" />',
};

/** Small line icon used in the workflow steps. */
export function icon(name: keyof typeof ICONS | string): Raw {
  return raw(`<svg class="icon" viewBox="0 0 26 26" width="26" height="26" aria-hidden="true" focusable="false">${ICONS[name] ?? ''}</svg>`);
}
