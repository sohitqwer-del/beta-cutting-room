import { html, raw, signed, type Raw } from '../html.ts';
import { ext, section } from '../layout.ts';
import { factoryDiagram, heroMarker, icon, screenPlaceholder } from '../illustrations.ts';
import { feedbackLinks, site } from '../config.ts';
import { headlineGroup, type Group, type Run } from '../benchmark.ts';
import { pick } from './benchmark.ts';
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
  { slot: 'style-pattern', label: 'Style pattern canvas', kind: 'pattern', tab: 0, caption: 'Style: start with a block or blank sheet, then draft, grade and prepare the cut.' },
  { slot: 'pattern', label: 'Pattern', kind: 'pattern', tab: 0, caption: 'Pattern: draft, grade and check all sizes.' },
  { slot: 'cut-plan', label: 'Cut plan', kind: 'table', tab: 3, caption: 'Cut plan: ratios, plies and lay lengths for an order.' },
  { slot: 'rolls', label: 'Rolls', kind: 'rolls', tab: 4, caption: 'Rolls: inspection, shade groups and allocation.' },
  { slot: 'spreading', label: 'Spreading', kind: 'floor', tab: 6, caption: 'Floor: spreading sheets and lay tracking.' },
  { slot: 'bundles', label: 'Bundles', kind: 'tickets', tab: 5, caption: 'Bundles: QR tickets, stickers and re-cuts.' },
];

const features = [
  { icon: 'pattern', title: 'Pattern making', items: ['Seven editable basic blocks or a blank sheet, with a drafting canvas', 'Size-chart grading, stacked sizes and seam checks', 'Digitize a paper pattern from a photo', 'DXF-AAMA export; Seamly2D file subset import'] },
  { icon: 'marker', title: 'Marker making', items: ['Automatic nesting with garment rules: grain, one-way nap, left/right pairs, size ratio, gaps', 'Folded and tubular fabric, shade bands, fabric defects', 'Manual editing: bump, compact, swap, sections, marker library', 'Every marker is checked by an independent validator before export'] },
  { icon: 'plan', title: 'Cut planning and costing', items: ['Size ratios, ply heights and lay lengths from an order', 'Fabric consumption and cost estimates', 'Compare fabric widths (command line)'] },
  { icon: 'rolls', title: 'Rolls and spreading', items: ['4-point roll inspection and defect maps', 'Shade and shrinkage groups', 'Roll allocation, splice zones and end bits'] },
  { icon: 'bundles', title: 'Bundles and the floor', items: ['Ply numbering, bundles, QR tickets and stickers', 'Re-cut planning for damaged parts', 'Bundle scan-out to sewing lines with a USB scanner'] },
  { icon: 'files', title: 'Files and data', items: ['Plot: PLT/HPGL and 1:1 PDF; SVG, DXF, generic ISO cutter file', 'Import orders and rolls from CSV or Excel', 'Export consumption and work-in-progress as CSV'] },
];

const notYet = [
  'Basic blocks need a fit review by a pattern master. Extend and trim work between straight lines.',
  'In the browser, marker PLT, full-size PDF and DXF need the desktop app or a server. Tiled PDF works locally.',
  'Quick Marker real-photo accuracy is untested. Check dimensions against the paper before cutting.',
  'Plotter and cutter profiles are generic. Test them on your machines before production.',
  'Folded and tubular fabric work. Folded results improved a lot in 0.5 (T-shirt test 43% to 63%) but are still below open fabric.',
  'No direct Tally or SAP connector yet. Exchange data with CSV or Excel files.',
  'Server access is local by default. Factory-network use needs an access token and an HTTPS proxy.',
  'The Windows installer is not code-signed yet. Windows may show a warning.',
  'Hindi and Tamil are drafts and need native review.',
];

const audiences = [
  { icon: 'people', title: 'Garment factories', text: 'Cutting rooms that want control of their own data and no per-seat licence fees. First users are in India.' },
  { icon: 'marker', title: 'CAD and marker operators', text: 'People who make markers every day and need grain, nap, pairs, ratios, folds and shade rules to be right.' },
  { icon: 'plan', title: 'Merchandisers and planners', text: 'Fabric consumption, width comparison and cut plans from the same marker the cutting room uses.' },
  { icon: 'server', title: 'Factory IT', text: 'A Windows installer, a command-line server and plain files. No cloud service to approve.' },
  { icon: 'code', title: 'Researchers and developers', text: 'Open code, open benchmark and open data formats. Improve the nesting engine and prove it.' },
];

function slide(ctx: BuildContext, s: (typeof screens)[number], i: number): Raw {
  const file = `${s.slot}.png`;
  const media = ctx.screens.has(file)
    ? html`<img src="assets/screens/${file}" alt="${`Screenshot of the ${s.label} screen in BETA Cutting Room`}" width="1920" height="1200" loading="lazy" decoding="async">`
    : screenPlaceholder(s.kind, s.label, s.tab);
  return html`<figure class="slide${i === 0 ? ' is-active' : ''}" id="slide-${s.slot}" data-slide>${media}<figcaption>${s.caption}</figcaption></figure>`;
}

export function downloadButton(url: string | null, label: string, cls = 'btn btn-primary', sub?: string): Raw {
  return url
    ? html`<a class="${cls}" href="${url}"><span class="btn-label">${label}</span>${sub ? html`<span class="btn-sub">${sub}</span>` : ''}</a>`
    : html`<span class="${cls} btn-disabled" aria-disabled="true"><span class="nowrap">${label}</span><span class="badge">Coming soon</span></span>`;
}

export function contactLine(): Raw {
  return site.contactEmail
    ? html`<a href="${`mailto:${site.contactEmail}`}">${site.contactEmail}</a>`
    : html`<span class="placeholder-text">Contact address coming soon</span>`;
}

/** A number that counts up when it scrolls into view. The final text is in the HTML. */
function counter(value: number, decimals: number, text: string, opts: { signed?: boolean } = {}): Raw {
  return html`<span class="count" data-count="${value}" data-decimals="${decimals}"${opts.signed ? raw(' data-signed="1"') : ''}>${text}</span>`;
}

/** Win / tie / loss as a stacked bar with the numbers in text beside it. */
function wtlBar(g: Group): Raw {
  if (g.win === null || g.tie === null || g.loss === null) return html``;
  const loss = g.loss + g.too_long_losses; const total = g.win + g.tie + loss;
  const w = (n: number) => `${((100 * n) / Math.max(1, total)).toFixed(2)}%`;
  return html`<div class="wtl-block">
  <div class="wtl-bar" aria-hidden="true"><span class="wtl-win" style="${`width:${w(g.win)}`}"></span><span class="wtl-tie" style="${`width:${w(g.tie)}`}"></span><span class="wtl-loss" style="${`width:${w(loss)}`}"></span></div>
  <p class="wtl-legend"><span><i class="dot dot-win"></i>${g.win} wins</span><span><i class="dot dot-tie"></i>${g.tie} ties</span><span><i class="dot dot-loss"></i>${loss} losses</span></p>
</div>`;
}

function benchCard(run: Run, title: string, sub: string): Raw {
  const g = headlineGroup(run);
  const mean = g.mean_diff_pp;
  return html`<article class="bench-card" data-reveal>
  <header><h3>${title}</h3><p class="muted small">${sub}</p></header>
  <div class="bench-numbers">
    <div class="bench-stat"><span class="bench-value${mean !== null && mean < 0 ? ' is-neg' : ''}">${mean === null ? '–' : html`${counter(mean, 2, signed(mean), { signed: true })}<small> pp</small>`}</span><span class="bench-label">Mean difference</span></div>
    <div class="bench-stat"><span class="bench-value${g.median_diff_pp !== null && g.median_diff_pp < 0 ? ' is-neg' : ''}">${g.median_diff_pp === null ? '–' : html`${counter(g.median_diff_pp, 2, signed(g.median_diff_pp), { signed: true })}<small> pp</small>`}</span><span class="bench-label">Median task</span></div>
  </div>
  ${wtlBar(g)}
  <p class="small muted">${g.valid} of ${g.attempts} BETA layouts passed the independent validator. A tie is within 0.1 points.</p>
</article>`;
}

/** Benchmark highlight: mean, median and win/tie/loss always together. Wording comes from the data. */
function benchmarkHighlight(runs: Run[]): Raw {
  const lm = pick(runs, 'lectra-matched'); const l60 = pick(runs, 'lectra-60s');
  const shown = [lm, l60].filter((r): r is Run => r !== undefined);
  if (!shown.length) return html``;
  const losesMore = shown.some((r) => { const g = headlineGroup(r); return g.win !== null && g.loss !== null && g.loss + g.too_long_losses > g.win; });
  const summary = losesMore
    ? 'BETA is close to Lectra’s recorded results, but it still loses more tasks than it wins. We publish every number so you can check.'
    : 'BETA is close to Lectra’s recorded results on this sample. This is not a general win. We publish every number so you can check.';
  const cards: Raw[] = [];
  if (lm) cards.push(benchCard(lm, 'With Lectra’s own time per task', `About 5 minutes per task · ${headlineGroup(lm).attempts} tasks`));
  if (l60) cards.push(benchCard(l60, 'With 60 seconds per task', `${headlineGroup(l60).attempts} tasks`));
  return section('benchmark', 'Honest numbers, not big claims', html`
<div class="bench-grid">${cards}</div>
<div class="bench-foot" data-reveal>
  <p><strong>No general win over Lectra.</strong> ${summary} Points (pp) are BETA’s marker efficiency minus Lectra’s; below zero means BETA’s marker was longer.</p>
  <a class="link-arrow" href="benchmark.html">Method, assumptions and all results<span aria-hidden="true"> →</span></a>
</div>`, { eyebrow: 'Public benchmark', lead: 'We test BETA on the public Lectra dataset: real nesting tasks from fashion companies, with the efficiency that Lectra’s own engine reached on each one.', cls: 'section-alt' });
}

/** "Watch the demo": a self-hosted video with captions, or a poster linking to the video file. */
export function demoVideo(ctx: BuildContext): Raw {
  const v = site.demoVideo;
  const meta = html`<p class="video-meta"><span>${v.length}</span><span>Captions in English</span><span>Real app, sample data</span></p>`;
  let media: Raw;
  if (ctx.video) {
    media = html`<video class="demo-video" controls preload="none" playsinline poster="${v.poster}" width="1920" height="1080" aria-describedby="demo-title-desc">
  <source src="${v.file}" type="video/mp4">
  <track kind="captions" srclang="en" label="English" src="${v.captions}" default>
  <a href="${v.file}">Download the demo video (MP4)</a>
</video>`;
  } else if (v.demoVideoUrl) {
    media = html`<a class="demo-poster-link" href="${v.demoVideoUrl}" rel="noopener noreferrer external"><img src="${v.poster}" alt="Watch the BETA Cutting Room demo video" width="1920" height="1080" loading="lazy" decoding="async"><span class="demo-play" aria-hidden="true"></span><span class="sr-only"> (MP4 file${v.size !== '0 MB' ? `, ${v.size}` : ''}, opens on GitHub)</span></a>`;
  } else return html``;
  return html`<section class="section section-video" id="demo" aria-labelledby="demo-title">
  <div class="wrap video-grid">
    <div class="video-copy" data-reveal>
      <p class="eyebrow">Demo video</p>
      <h2 id="demo-title">Watch the demo</h2>
      <p class="lead" id="demo-title-desc">What a pattern and a marker are, then the real app: nest a marker, plan the cut, rolls and bundles, photo to marker on a phone, and our honest benchmark.</p>
      ${meta}
      <p class="small muted">Sound is optional. Every word is also on screen.</p>
    </div>
    <div class="video-frame" data-reveal>${media}</div>
  </div>
</section>`;
}

export function homePage(ctx: BuildContext): Raw {
  const d = site.downloads;
  const l60 = pick(ctx.runs, 'lectra-60s');
  const benchTasks = l60 ? headlineGroup(l60).attempts : null;
  return html`
<div class="hero">
  <div class="hero-bg" aria-hidden="true"><span class="blob blob-1"></span><span class="blob blob-2"></span><span class="blob blob-3"></span></div>
  <div class="wrap hero-inner">
    <div class="hero-copy">
      <p class="hero-pills"><span class="pill">Version ${site.version}</span><span class="pill">Free and open source</span><a class="pill pill-fun" href="#fun"><span aria-hidden="true">✦</span> A fun project, built with modern AI tools</a></p>
      <h1 class="hero-title"><span class="nowrap">Open-source,</span> <span class="nowrap">self-hosted</span> <span class="hl"><span class="nowrap">cutting-room</span> software</span> for garment factories</h1>
      <p class="hero-lead">Pattern, marker, cut planning, rolls, spreading and bundles in one program. It runs on your own Windows PC or factory server and works offline. Your patterns never leave your factory.</p>
      <div class="cta-row">
        ${site.webAppUrl ? downloadButton(site.webAppUrl, 'Open the app in your browser', 'btn btn-secondary btn-large') : ''}
        ${downloadButton(d.windows.url, 'Download for Windows', 'btn btn-primary btn-large', `Installer · ${d.windows.size}`)}
        ${downloadButton(d.portable.url, 'Portable ZIP', 'btn btn-secondary btn-large', `No install · ${d.portable.size}`)}
        ${downloadButton(d.server.url, 'Linux server', 'btn btn-secondary btn-large', `tar.gz · ${d.server.size}`)}
      </div>
      <p class="small muted">Best for trying and small jobs; for big markers use the desktop app</p>
      <ul class="trust-list">
        <li>No account</li><li>No cloud</li><li>No licence fee</li><li>Works offline</li>
      </ul>
      <p class="humble"><span class="humble-icon" aria-hidden="true">${icon('heart')}</span><span><strong>BETA is young and not perfect yet.</strong> We need your help to improve it. <a href="#help">Tell us what to fix</a>.</span></p>
    </div>
    <figure class="hero-figure" data-tilt>
      <div class="hero-card">
        <div class="window-bar" aria-hidden="true"><i></i><i></i><i></i><span>Marker · shirt · sizes S, M, L</span><span class="badge">Illustration</span></div>
        <div class="hero-stage is-playing" id="hero-stage">
          ${heroMarker()}
          <p class="hero-status" aria-hidden="true"><span class="st st-1"><span class="spinner"></span>Nesting 18 pieces…</span><span class="st st-2"><span class="tick">✓</span>All pieces placed. Checked by the validator.</span></p>
        </div>
        <button type="button" class="replay" data-replay="hero-stage" hidden><svg viewBox="0 0 20 20" width="16" height="16" aria-hidden="true" focusable="false"><path d="M4 10a6 6 0 1 0 2-4.5M4 3v3.5h3.5" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>Replay</button>
      </div>
      <figcaption>Illustration. A marker places every pattern piece on the fabric so the lay uses as little length as possible.</figcaption>
    </figure>
  </div>
</div>

${demoVideo(ctx)}

<section class="stats" aria-label="BETA in numbers">
  <div class="wrap"><ul class="stat-band" data-reveal data-stagger>
    <li><span class="stat-big">${counter(8, 0, '8')}</span><span class="stat-text">steps from pattern to sewing line, in one program</span></li>
    ${benchTasks ? html`<li><span class="stat-big">${counter(benchTasks, 0, String(benchTasks))}</span><span class="stat-text">real Lectra nesting tasks in our public benchmark</span></li>` : ''}
    <li><span class="stat-big">${counter(100, 0, '100')}<small>%</small></span><span class="stat-text">of markers checked by an independent validator before export</span></li>
    <li><span class="stat-big">0</span><span class="stat-text">accounts, cloud services or licence fees</span></li>
  </ul></div>
</section>

${section('workflow', 'From pattern to sewing line', html`
<div class="flow" data-flow>
  <div class="flow-track" aria-hidden="true"><span class="flow-fill"></span><span class="flow-runner"></span></div>
  <ol class="flow-steps">
    ${steps.map((s, i) => html`<li class="flow-step" data-flow-step><span class="flow-node"><span class="flow-icon">${icon(s.key)}</span></span><span class="step-num">Step ${i + 1}</span><h3>${s.title}</h3><p>${s.text}</p><span class="step-module">${s.module}</span></li>`)}
  </ol>
  <button type="button" class="replay replay-inline" data-flow-replay hidden><svg viewBox="0 0 20 20" width="16" height="16" aria-hidden="true" focusable="false"><path d="M4 10a6 6 0 1 0 2-4.5M4 3v3.5h3.5" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round"/></svg>Play the steps again</button>
</div>`, { eyebrow: 'One program', lead: 'One record follows the style through the cutting room. Each step uses the result of the step before, so numbers are not typed twice.' })}

${section('screens', 'See the real app', html`
<div class="showcase" data-carousel data-reveal>
  <div class="showcase-frame">
    <div class="window-bar" aria-hidden="true"><i></i><i></i><i></i><span>BETA Cutting Room ${site.version}</span></div>
    <div class="slides">${screens.map((s, i) => slide(ctx, s, i))}</div>
  </div>
  <div class="showcase-controls">
    <div class="tabs" data-tablist aria-label="App screens" hidden>${screens.map((s, i) => html`<button type="button" class="tab" id="tab-${s.slot}" data-tab aria-selected="${i === 0 ? 'true' : 'false'}"><span>${s.label}</span><i class="tab-progress" aria-hidden="true"></i></button>`)}</div>
    <button type="button" class="btn-icon" data-toggle hidden aria-pressed="false"><svg class="i-pause" viewBox="0 0 20 20" width="18" height="18" aria-hidden="true" focusable="false"><path d="M7 5v10M13 5v10" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"/></svg><svg class="i-play" viewBox="0 0 20 20" width="18" height="18" aria-hidden="true" focusable="false"><path d="M7 4.5v11l9-5.5z" fill="currentColor"/></svg><span class="sr-only" data-toggle-label>Pause slideshow</span></button>
  </div>
</div>
<p class="note">Real screens from version ${site.version}, with original sample data. No factory data.</p>`, { eyebrow: 'Screenshots', cls: 'section-alt' })}

${benchmarkHighlight(ctx.runs)}

${section('features', 'What it does today', html`
<div class="card-grid" data-reveal data-stagger>
  ${features.map((f) => html`<article class="card feature-card"><span class="card-icon">${icon(f.icon)}</span><h3>${f.title}</h3><ul class="check-list">${f.items.map((i) => html`<li>${i}</li>`)}</ul></article>`)}
</div>
<div class="callout callout-plain not-yet" data-reveal>
  <h3>Not yet</h3>
  <p>We prefer to tell you before you find out.</p>
  <ul>${notYet.map((n) => html`<li>${n}</li>`)}</ul>
</div>`, { eyebrow: 'Features', lead: 'Each of these runs on your own computer. The heavy work (nesting, checks, file output) is written in Rust for speed.' })}

<section class="section section-data" id="privacy" aria-labelledby="privacy-title">
  <div class="wrap split split-even">
    <div data-reveal>
      <p class="eyebrow">Privacy</p>
      <h2 id="privacy-title">Your data stays in your factory</h2>
      <p class="lead">BETA runs on computers you control. There is no BETA cloud to send your patterns to.</p>
      <ul class="check-list check-list-big">
        <li>Patterns, markers, orders and roll data stay on your PCs and your server.</li>
        <li>No account, no tracking, no usage data and no crash reports sent anywhere.</li>
        <li>Works with no internet. Install and update from a USB drive.</li>
        <li>The server listens only on its own computer unless you turn on factory network access with a token.</li>
      </ul>
      <p><a class="link-arrow" href="install.html#privacy">Read about privacy and self-hosting<span aria-hidden="true"> →</span></a></p>
    </div>
    <div class="factory-wrap" data-reveal>${factoryDiagram()}</div>
  </div>
</section>

${section('who', 'Who it is for', html`
<div class="card-grid card-grid-5" data-reveal data-stagger>
  ${audiences.map((a) => html`<article class="card audience-card"><span class="card-icon card-icon-small">${icon(a.icon)}</span><h3>${a.title}</h3><p>${a.text}</p></article>`)}
</div>`, { cls: 'section-alt' })}

${section('download', 'Download', html`
<div class="card-grid card-grid-4" data-reveal data-stagger>
  <article class="card download-card download-main">
    <span class="card-icon">${icon('download')}</span>
    <h3>Windows installer</h3>
    <p>For one PC. Windows 10 or 11, 64-bit. ${d.windows.size}. No administrator rights needed.</p>
    ${downloadButton(d.windows.url, 'Download installer')}
    <p class="small muted"><a href="install.html#windows">Installation steps</a></p>
  </article>
  <article class="card download-card">
    <span class="card-icon">${icon('zip')}</span>
    <h3>Portable ZIP</h3>
    <p>No installation. Unzip, run <code>start-server.cmd</code> and open the app in your browser. ${d.portable.size}.</p>
    ${downloadButton(d.portable.url, 'Download ZIP', 'btn btn-secondary')}
    <p class="small muted"><a href="install.html#portable">Portable steps</a></p>
  </article>
  <article class="card download-card">
    <span class="card-icon">${icon('server')}</span>
    <h3>Linux server</h3>
    <p>The <code>beta-marker serve</code> command runs the same app in a web browser. ${d.server.size}. Needs glibc 2.43 or newer.</p>
    ${downloadButton(d.server.url, 'Download server', 'btn btn-secondary')}
    <p class="small muted"><a href="install.html#server">Server steps</a></p>
  </article>
  <article class="card download-card">
    <span class="card-icon">${icon('code')}</span>
    <h3>Source code</h3>
    <p>Everything is open source under Apache-2.0. Build it yourself and check every line.</p>
    ${downloadButton(site.repoUrl, 'View on GitHub', 'btn btn-secondary')}
    <p class="small muted"><a href="open-source.html">Licence and credits</a></p>
  </article>
</div>`)}

<section class="section section-alt" id="fun" aria-labelledby="fun-title">
  <div class="wrap fun-grid">
    <div class="fun-card" data-reveal>
      <span class="fun-badge"><span aria-hidden="true">✦</span> A fun project</span>
      <h2 id="fun-title">Built for fun, with the help of modern AI tools</h2>
      <p>BETA Cutting Room is a fun project by a small team. We build it with the help of modern AI coding assistants, and we test it like serious software: an independent validator checks every marker before you can export it.</p>
      <p>The code is open. You are welcome to read it, try it and help us make it better.</p>
      ${site.repoUrl ? ext(site.repoUrl, html`See the code on GitHub<span aria-hidden="true"> →</span>`) : ''}
    </div>
    <div class="help-block" id="help" data-reveal>
      <h2 id="help-title" class="h-help">Help us improve</h2>
      <p>BETA is young and not perfect yet. We need your help to improve it. A short note is enough, and every report helps.</p>
      ${helpLinks()}
    </div>
  </div>
</section>

${section('samples', 'Send us your markers', html`
<div class="split">
  <div data-reveal>
    <p>We want to compare BETA with your current CAD on your own styles: same pieces, same fabric width, same rules. You get the results, and they stay private.</p>
    <h3>What to send</h3>
    <ol class="plain-steps">
      <li>3 to 5 real styles exported as <strong>DXF-AAMA or ASTM</strong> from your CAD (for example a shirt, trouser, T-shirt or jeans).</li>
      <li>For each style, the marker your CAD made: fabric width, size ratio, rules (nap, 0/180, gaps), <strong>marker length and efficiency</strong>, and how long the auto-nest ran.</li>
      <li>If you can, one plotter file (PLT) from your cutting room, so we can check the plot scale on your plotter.</li>
    </ol>
  </div>
  <aside class="callout callout-accent" aria-label="Privacy promise" data-reveal>
    <h3>Your files stay private</h3>
    <p>We never commit, publish or share your patterns or markers without your written permission. Published results name no style, buyer or factory unless you agree.</p>
    <p><strong>Contact:</strong> ${contactLine()}</p>
  </aside>
</div>`)}
`;
}

/** Feedback links (GitHub issues and email) as a list of cards. */
function helpLinks(cls = ''): Raw {
  const iconFor = { bug: 'bug', missing: 'missing', idea: 'idea' } as const;
  const email = site.feedback.email;
  return html`<ul class="help-links ${cls}" data-stagger>
  ${feedbackLinks().map((f) => html`<li><a class="help-link" href="${f.url}" rel="noopener noreferrer external"><span class="card-icon card-icon-small">${icon(iconFor[f.key])}</span><span><strong>${f.label}</strong><span class="help-text">${f.text}</span></span><span class="sr-only"> (GitHub, external site)</span></a></li>`)}
  ${email ? html`<li><a class="help-link" href="${email}"><span class="card-icon card-icon-small">${icon('mail')}</span><span><strong>Email us</strong><span class="help-text">No GitHub account? Write to ${email.replace(/^mailto:/, '')}.</span></span></a></li>` : ''}
</ul>`;
}

/** Shared "Help us improve" section for inner pages. */
export function helpSection(cls = ''): Raw {
  return section('help', 'Help us improve', html`<div data-reveal>${helpLinks('help-links-row')}</div>`, { lead: 'BETA is young and not perfect yet. We need your help to improve it. A short note is enough.', cls });
}
