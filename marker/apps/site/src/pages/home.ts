import { html, type Raw } from '../html.ts';
import { section } from '../layout.ts';
import { heroMarker, icon, screenPlaceholder } from '../illustrations.ts';
import { site } from '../config.ts';
import type { BuildContext } from './context.ts';

const steps = [
  { key: 'pattern', title: 'Pattern', text: 'Draft, grade and digitize patterns. Import DXF-AAMA or a supported Seamly2D file.', module: 'BETA Pattern' },
  { key: 'marker', title: 'Marker', text: 'Nest all pieces on the fabric width with grain, nap, pairs and size ratio. Edit by hand.', module: 'BETA Marker' },
  { key: 'plan', title: 'Cut plan', text: 'Turn an order into size ratios, ply heights and lay lengths.', module: 'BETA Marker' },
  { key: 'rolls', title: 'Rolls', text: 'Inspect rolls (4-point), group shade and shrinkage, allocate rolls to lays.', module: 'BETA Marker' },
  { key: 'spread', title: 'Spread', text: 'Print spreading sheets with roll order, plies, splice zones and end allowances.', module: 'Floor' },
  { key: 'cut', title: 'Cut', text: 'Plot the marker 1:1 (PLT/HPGL or PDF) or send a cutter file. Track each lay.', module: 'Floor' },
  { key: 'bundles', title: 'Bundles', text: 'Number plies, make bundles and print QR tickets and stickers. Plan re-cuts.', module: 'Floor' },
  { key: 'sewing', title: 'Sewing', text: 'Scan bundles out to sewing lines. Export work-in-progress and fabric use.', module: 'Floor' },
];

const screens: { slot: string; label: string; kind: Parameters<typeof screenPlaceholder>[0]; tab: number; caption: string }[] = [
  { slot: 'marker-editor', label: 'Marker editor', kind: 'marker', tab: 2, caption: 'Marker editor: nest, bump, compact and check every piece.' },
  { slot: 'pattern', label: 'Pattern', kind: 'pattern', tab: 0, caption: 'Pattern: draft, grade and check all sizes.' },
  { slot: 'cut-plan', label: 'Cut plan', kind: 'table', tab: 3, caption: 'Cut plan: ratios, plies and lay lengths for an order.' },
  { slot: 'rolls', label: 'Rolls', kind: 'rolls', tab: 4, caption: 'Rolls: inspection, shade groups and allocation.' },
  { slot: 'spreading', label: 'Floor spreading', kind: 'floor', tab: 6, caption: 'Floor: spreading sheets and lay tracking.' },
  { slot: 'bundles', label: 'Bundles', kind: 'tickets', tab: 5, caption: 'Bundles: QR tickets, stickers and re-cuts.' },
];

const features = [
  { title: 'Pattern making', items: ['Drafting with measurements and formulas', 'Grading and size checks', 'Digitize a paper pattern from a photo', 'DXF-AAMA export; Seamly2D file subset import'] },
  { title: 'Marker making', items: ['Automatic nesting with garment rules: grain, one-way nap, left/right pairs, size ratio, gaps', 'Folded and tubular fabric, shade bands, fabric defects', 'Manual editing: bump, compact, swap, sections, marker library', 'Every marker is checked by an independent validator before export'] },
  { title: 'Cut planning and costing', items: ['Size ratios, ply heights and lay lengths from an order', 'Fabric consumption and cost estimates', 'Compare fabric widths (command line)'] },
  { title: 'Rolls and spreading', items: ['4-point roll inspection and defect maps', 'Shade and shrinkage groups', 'Roll allocation, splice zones and end bits'] },
  { title: 'Bundles and the floor', items: ['Ply numbering, bundles, QR tickets and stickers', 'Re-cut planning for damaged parts', 'Bundle scan-out to sewing lines with a USB scanner'] },
  { title: 'Files and data', items: ['Plot: PLT/HPGL and 1:1 PDF; SVG, DXF, generic ISO cutter file', 'Import orders and rolls from CSV or Excel', 'Export consumption and work-in-progress as CSV'] },
];

const notYet = [
  'Plotter and cutter profiles are generic. Test them on your machines before production.',
  'No direct Tally or SAP connector yet. Exchange data with CSV or Excel files.',
  'The server mode serves one computer in version 0.4. Shared use over the factory network is planned.',
  'The Windows installer is not code-signed yet. Windows may show a warning.',
  'English only. Hindi and Tamil are planned.',
];

function screenFigure(ctx: BuildContext, s: (typeof screens)[number]): Raw {
  const file = `${s.slot}.png`;
  const media = ctx.screens.has(file)
    ? html`<img src="assets/screens/${file}" alt="${`Screenshot of the ${s.label} screen in BETA Cutting Room`}" loading="lazy" decoding="async">`
    : screenPlaceholder(s.kind, s.label, s.tab);
  return html`<figure class="screen">${media}<figcaption>${s.caption}</figcaption></figure>`;
}

export function downloadButton(url: string | null, label: string, cls = 'btn btn-primary'): Raw {
  return url
    ? html`<a class="${cls}" href="${url}">${label}</a>`
    : html`<span class="${cls} btn-disabled" aria-disabled="true"><span class="nowrap">${label}</span><span class="badge">Coming soon</span></span>`;
}

export function contactLine(): Raw {
  return site.contactEmail
    ? html`<a href="${`mailto:${site.contactEmail}`}">${site.contactEmail}</a>`
    : html`<span class="placeholder-text">Contact address coming soon</span>`;
}

export function homePage(ctx: BuildContext): Raw {
  const win = site.downloads.windows;
  return html`
<div class="hero">
  <div class="wrap hero-inner">
    <div class="hero-copy">
      <p class="eyebrow">Version ${site.version} · Open source · Self-hosted</p>
      <h1>Cutting-room software for garment factories</h1>
      <p class="hero-lead">Pattern, marker, cut planning, rolls and bundles in one program. Open source (Apache-2.0). Self-hosted: it runs on your own Windows PC or factory server. Your patterns never leave your factory. It works offline.</p>
      <div class="cta-row">
        ${downloadButton(win.url, 'Download for Windows')}
        <a class="btn btn-secondary" href="benchmark.html">See the benchmark</a>
      </div>
      <ul class="trust-list">
        <li>No account</li><li>No cloud</li><li>No licence fee</li><li>Works offline</li>
      </ul>
    </div>
    <figure class="hero-figure">
      ${heroMarker()}
      <figcaption>Illustration. A marker places every pattern piece on the fabric so the lay uses as little length as possible.</figcaption>
    </figure>
  </div>
</div>

${section('workflow', 'From pattern to sewing line', html`
<ol class="flow-strip" aria-hidden="true">${steps.map((s) => html`<li><span>${s.title}</span></li>`)}</ol>
<ol class="workflow">
  ${steps.map((s, i) => html`<li class="step"><span class="step-icon">${icon(s.key)}</span><span class="step-num">Step ${i + 1}</span><h3>${s.title}</h3><p>${s.text}</p><span class="step-module">${s.module}</span></li>`)}
</ol>`, { lead: 'One record follows the style through the cutting room. Each step uses the result of the step before, so numbers are not typed twice.' })}

${section('screens', 'See the screens', html`<div class="screen-grid">${screens.map((s) => screenFigure(ctx, s))}</div>
<p class="note">The app is being redesigned for version ${site.version}. Real screenshots replace these drawings before release.</p>`, { cls: 'section-alt' })}

${section('features', 'What it does today', html`
<div class="card-grid">
  ${features.map((f) => html`<article class="card"><h3>${f.title}</h3><ul class="check-list">${f.items.map((i) => html`<li>${i}</li>`)}</ul></article>`)}
</div>
<div class="callout callout-plain">
  <h3>Not yet</h3>
  <p>We prefer to tell you before you find out.</p>
  <ul>${notYet.map((n) => html`<li>${n}</li>`)}</ul>
</div>`, { lead: 'Each of these runs on your own computer. The heavy work (nesting, checks, file output) is written in Rust for speed.' })}

${section('who', 'Who it is for', html`
<div class="card-grid card-grid-5">
  <article class="card"><h3>Garment factories</h3><p>Cutting rooms that want control of their own data and no per-seat licence fees. First users are in India.</p></article>
  <article class="card"><h3>CAD and marker operators</h3><p>People who make markers every day and need grain, nap, pairs, ratios, folds and shade rules to be right.</p></article>
  <article class="card"><h3>Merchandisers and planners</h3><p>Fabric consumption, width comparison and cut plans from the same marker the cutting room uses.</p></article>
  <article class="card"><h3>Factory IT</h3><p>A Windows installer, a command-line server and plain files. No cloud service to approve.</p></article>
  <article class="card"><h3>Researchers and developers</h3><p>Open code, open benchmark and open data formats. Improve the nesting engine and prove it.</p></article>
</div>`, { cls: 'section-alt' })}

${section('download', 'Download', html`
<div class="card-grid card-grid-3">
  <article class="card download-card">
    <h3>Windows desktop app</h3>
    <p>For one PC. Windows 10 or 11, 64-bit. Installer ${win.size}. No administrator rights needed.</p>
    ${downloadButton(win.url, 'Download installer')}
    <p class="small muted"><a href="install.html#windows">Installation steps</a></p>
  </article>
  <article class="card download-card">
    <h3>Server build</h3>
    <p>The <code>beta-marker serve</code> command runs the same app in a web browser.</p>
    ${downloadButton(site.downloads.server.url, 'Download server build')}
    <p class="small muted"><a href="install.html#server">Server steps</a> · build it from source today</p>
  </article>
  <article class="card download-card">
    <h3>Source code</h3>
    <p>Everything is open source under Apache-2.0. Build it yourself and check every line.</p>
    ${downloadButton(site.repoUrl, 'View source code', 'btn btn-secondary')}
    <p class="small muted"><a href="open-source.html">Licence and credits</a></p>
  </article>
</div>`)}

${section('samples', 'Send us your markers', html`
<div class="split">
  <div>
    <p>We want to compare BETA with your current CAD on your own styles: same pieces, same fabric width, same rules. You get the results, and they stay private.</p>
    <h3>What to send</h3>
    <ol class="plain-steps">
      <li>3 to 5 real styles exported as <strong>DXF-AAMA or ASTM</strong> from your CAD (for example a shirt, trouser, T-shirt or jeans).</li>
      <li>For each style, the marker your CAD made: fabric width, size ratio, rules (nap, 0/180, gaps), <strong>marker length and efficiency</strong>, and how long the auto-nest ran.</li>
      <li>If you can, one plotter file (PLT) from your cutting room, so we can check the plot scale on your plotter.</li>
    </ol>
  </div>
  <aside class="callout callout-accent" aria-label="Privacy promise">
    <h3>Your files stay private</h3>
    <p>We never commit, publish or share your patterns or markers without your written permission. Published results name no style, buyer or factory unless you agree.</p>
    <p><strong>Contact:</strong> ${contactLine()}</p>
  </aside>
</div>`, { cls: 'section-alt' })}
`;
}
