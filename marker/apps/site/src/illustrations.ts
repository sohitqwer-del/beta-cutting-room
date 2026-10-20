// Original illustrative SVGs (no screenshots, no vendor material). Colours come from CSS classes.
// Motion is CSS only: each moving part reads its start offset from CSS custom properties,
// and the final (resting) position is the plain SVG layout, so no-motion shows the end state.
import { esc, raw, type Raw } from './html.ts';

const BODY = 'M0 22 Q8 2 30 0 L42 0 Q50 16 58 16 Q66 16 74 0 L86 0 Q104 2 100 22 L96 52 Q90 60 92 72 L94 120 L6 120 L8 72 Q10 60 4 52 Z';
const SLEEVE = 'M0 66 Q6 14 50 0 Q94 14 100 66 L92 80 L8 80 Z';
const POCKET = 'M0 0 H30 V26 L15 36 L0 26 Z';

/** Small deterministic pseudo-random sequence, so every build draws the same picture. */
function seq(seed: number): () => number {
  let s = seed;
  return () => { s = (s * 1103515245 + 12345) % 2147483648; return s / 2147483648; };
}

function piece(path: string, size: number, tx: number, ty: number, opts: { rotate?: boolean; sy?: number; grain?: boolean; w?: number; h?: number } = {}): string {
  const w = opts.w ?? 100; const h = opts.h ?? 120;
  const rot = opts.rotate ? ` rotate(180 ${w / 2} ${(h * (opts.sy ?? 1)) / 2})` : '';
  const scale = opts.sy ? ` scale(1 ${opts.sy})` : '';
  const grain = opts.grain ? `<path class="grain" d="M${w / 2} ${h * 0.3} V${h * 0.82} M${w / 2 - 4} ${h * 0.36} L${w / 2} ${h * 0.3} L${w / 2 + 4} ${h * 0.36}"/>` : '';
  return `<g class="pc pc-${size}" transform="translate(${tx} ${ty})${rot}${scale}"><path d="${path}"/>${grain}</g>`;
}

/** Wraps a piece so it can fly in from an offset (CSS custom properties) to where it rests. */
function fly(inner: string, i: number, rnd: () => number): string {
  const fx = Math.round(260 + rnd() * 260); const fy = Math.round((rnd() - 0.5) * 420); const fr = Math.round((rnd() - 0.5) * 240);
  return `<g class="fly" style="--fx:${fx}px;--fy:${fy}px;--fr:${fr}deg;--i:${i}">${inner}</g>`;
}

/** Hero picture: shirt pieces in three sizes fly in and nest on a fabric strip. */
export function heroMarker(): Raw {
  const p: string[] = [];
  p.push(piece(BODY, 1, 24, 34, { grain: true }));
  p.push(piece(SLEEVE, 1, 24, 158, { sy: 0.85, h: 80 }));
  p.push(piece(SLEEVE, 1, 128, 34, { sy: 0.85, h: 80, rotate: true }));
  p.push(piece(BODY, 1, 128, 106, { rotate: true, grain: true }));
  p.push(piece(BODY, 2, 232, 34, { grain: true }));
  p.push(piece(SLEEVE, 2, 232, 158, { sy: 0.85, h: 80 }));
  p.push(piece(SLEEVE, 2, 336, 34, { sy: 0.85, h: 80, rotate: true }));
  p.push(piece(BODY, 2, 336, 106, { rotate: true, grain: true }));
  p.push(piece(BODY, 3, 440, 34, { grain: true }));
  p.push(piece(SLEEVE, 3, 440, 158, { sy: 0.85, h: 80 }));
  for (const [i, x] of [548, 566, 584, 602].entries()) p.push(`<rect class="pc pc-${(i % 3) + 1}" x="${x}" y="34" width="13" height="92" rx="5"/>`);
  for (const [i, [x, y]] of [[548, 134], [584, 134], [548, 180], [584, 180]].entries()) p.push(piece(POCKET, (i % 3) + 1, x!, y!, { w: 30, h: 36 }));
  const rnd = seq(7030786);
  return raw(`<svg class="illus hero-marker" viewBox="-10 0 646 280" role="img" aria-labelledby="hero-illus-t">
<title id="hero-illus-t">Illustration of a marker: shirt fronts, backs, sleeves, collars and pockets in three sizes nested on a strip of fabric.</title>
<rect class="fabric" x="20" y="30" width="604" height="200" rx="2"/>
<path class="selvedge" d="M20 30 H624 M20 230 H624"/>
<g class="pieces">${p.map((s, i) => fly(s, i, rnd)).join('\n')}</g>
<rect class="scan" x="18" y="26" width="5" height="208" rx="2.5"/>
<g class="dim dim-width"><path d="M10 34 V226 M6 38 L10 30 L14 38 M6 222 L10 230 L14 222"/><text x="0" y="0" transform="translate(6 130) rotate(-90)" text-anchor="middle">fabric width</text></g>
<g class="dim dim-length"><path class="dim-line" d="M20 250 H624"/><path d="M28 246 L20 250 L28 254 M616 246 L624 250 L616 254"/><text x="322" y="272" text-anchor="middle">marker length: shorter is better</text></g>
<g class="legend-sizes" transform="translate(20 8)"><rect class="pc pc-1" width="12" height="12" rx="2"/><text x="18" y="10">S</text><rect class="pc pc-2" x="40" width="12" height="12" rx="2"/><text x="58" y="10">M</text><rect class="pc pc-3" x="80" width="12" height="12" rx="2"/><text x="98" y="10">L</text></g>
</svg>`);
}

/** "Your data stays in your factory": a factory with a PC, a server and a plotter; no cloud. */
export function factoryDiagram(): Raw {
  return raw(`<svg class="illus factory" viewBox="0 0 560 320" role="img" aria-labelledby="factory-t">
<title id="factory-t">Diagram: inside your factory, a PC, a server and a plotter share data. There is no link to a cloud service.</title>
<path class="f-wall" d="M30 296 V132 L95 92 V132 L160 92 V132 L225 92 V132 L290 92 V132 L355 92 V132 L420 92 V296 Z"/>
<path class="f-floor" d="M14 296 H436"/>
<text class="f-label" x="48" y="160">Your factory</text>
<g class="f-node" style="--i:0"><g transform="translate(64 196)"><rect class="f-box" x="0" y="0" width="76" height="52" rx="6"/><rect class="f-screen" x="7" y="7" width="62" height="36" rx="3"/><path class="f-stroke" d="M28 52 V64 M18 66 H58"/><text class="f-cap" x="38" y="86" text-anchor="middle">PC</text></g></g>
<g class="f-node" style="--i:1"><g transform="translate(186 180)"><rect class="f-box" x="0" y="0" width="64" height="80" rx="6"/><path class="f-stroke" d="M10 18 H54 M10 36 H54 M10 54 H54"/><circle class="f-led" cx="48" cy="11" r="3"/><circle class="f-led" cx="48" cy="29" r="3"/><circle class="f-led" cx="48" cy="47" r="3"/><text class="f-cap" x="32" y="102" text-anchor="middle">Server</text></g></g>
<g class="f-node" style="--i:2"><g transform="translate(296 204)"><rect class="f-box" x="0" y="0" width="100" height="40" rx="6"/><rect class="f-paper" x="12" y="-14" width="76" height="16" rx="2"/><path class="f-stroke" d="M10 28 H90"/><path class="f-stroke" d="M8 40 V58 M92 40 V58"/><text class="f-cap" x="50" y="78" text-anchor="middle">Plotter</text></g></g>
<path class="f-link" d="M140 222 H186"/><path class="f-link" d="M250 222 H296"/>
<circle class="f-packet" style="--i:0" cx="143" cy="222" r="5"/><circle class="f-packet" style="--i:1" cx="253" cy="222" r="5"/>
<g class="f-cloud"><path class="f-cloud-shape" d="M456 118 a24 24 0 0 1 2 -47 a32 32 0 0 1 60 -8 a24 24 0 0 1 10 55 Z"/><text class="f-cap" x="492" y="146" text-anchor="middle">Cloud</text>
<g class="f-no"><circle cx="526" cy="112" r="16"/><path d="M519 105 L533 119 M533 105 L519 119"/></g></g>
<path class="f-cut" d="M420 140 L448 124"/>
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
  return raw(`<svg class="illus screen-ph" viewBox="0 0 400 250" role="img" aria-label="${esc(`Placeholder for a screenshot of ${label}`)}">
<rect class="ph-window" x="1" y="1" width="398" height="248" rx="10"/>
<path class="ph-bar" d="M1 11 Q1 1 11 1 H389 Q399 1 399 11 V24 H1 Z"/>
<circle class="ph-dot" cx="14" cy="12.5" r="3"/><circle class="ph-dot" cx="24" cy="12.5" r="3"/><circle class="ph-dot" cx="34" cy="12.5" r="3"/>
<rect class="ph-side" x="1" y="24" width="100" height="225"/>
${tabs.map((t, i) => `<rect class="${i === activeTab ? 'ph-tab-on' : 'ph-tab'}" data-tab="${t}" x="10" y="${34 + i * 22}" width="82" height="14" rx="3"/>`).join('')}
${content[kind]}
<text class="ph-text" x="245" y="225" text-anchor="middle">Screenshot coming soon</text>
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
  files: '<path d="M7 3 H16 L21 8 V23 H7 Z M16 3 V8 H21"/><path d="M10 13 H18 M10 17 H18"/>',
  check: '<path d="M13 3 L22 6 V12 Q22 19 13 23 Q4 19 4 12 V6 Z"/><path d="M9 13 L12 16 L17 10"/>',
  bug: '<rect x="8" y="8" width="10" height="13" rx="5"/><path d="M10 8 Q10 5 13 5 Q16 5 16 8 M8 12 H4 M18 12 H22 M8 17 H4 M18 17 H22 M13 11 V21 M9 4 L10.5 6 M17 4 L15.5 6"/>',
  missing: '<rect x="3" y="3" width="8" height="8" rx="1.5"/><rect x="15" y="3" width="8" height="8" rx="1.5"/><rect x="3" y="15" width="8" height="8" rx="1.5"/><path d="M19 15 V23 M15 19 H23" />',
  idea: '<path d="M9 17 Q5 14 5 10 Q5 3 13 3 Q21 3 21 10 Q21 14 17 17 V19 H9 Z M10 22 H16"/>',
  mail: '<rect x="3" y="6" width="20" height="15" rx="2"/><path d="M3 8 L13 15 L23 8"/>',
  heart: '<path d="M13 22 Q3 15 3 9 Q3 4 8 4 Q11 4 13 7 Q15 4 18 4 Q23 4 23 9 Q23 15 13 22 Z"/>',
  download: '<path d="M13 3 V16 M8 11 L13 16 L18 11 M4 20 V23 H22 V20"/>',
  server: '<rect x="4" y="3" width="18" height="8" rx="1.5"/><rect x="4" y="15" width="18" height="8" rx="1.5"/><path d="M8 7 H12 M8 19 H12"/><circle cx="18" cy="7" r="0.8"/><circle cx="18" cy="19" r="0.8"/>',
  zip: '<path d="M6 3 H20 V23 H6 Z"/><path d="M13 3 V5 M11 6 H13 M13 7 H15 M11 8 H13 M13 9 H15 M11 10 H13"/><rect x="10.5" y="12" width="5" height="5" rx="1"/>',
  code: '<path d="M9 7 L3 13 L9 19 M17 7 L23 13 L17 19 M15 4 L11 22"/>',
  spark: '<path d="M13 3 L15 11 L23 13 L15 15 L13 23 L11 15 L3 13 L11 11 Z"/>',
  offline: '<path d="M4 9 Q13 2 22 9 M7 13 Q13 8 19 13 M10 17 Q13 15 16 17"/><circle cx="13" cy="21" r="1.2"/><path d="M4 3 L22 23"/>',
  people: '<circle cx="9" cy="8" r="3.5"/><circle cx="18" cy="9" r="2.8"/><path d="M2.5 22 Q3 14 9 14 Q15 14 15.5 22 M15 15 Q22 13.5 23.5 21"/>',
};

/** Small line icon. */
export function icon(name: string): Raw {
  return raw(`<svg class="icon" viewBox="0 0 26 26" width="26" height="26" aria-hidden="true" focusable="false">${ICONS[name] ?? ''}</svg>`);
}
