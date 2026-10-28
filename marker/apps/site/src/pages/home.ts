import { html, raw, signed, type Raw } from '../html.ts';
import { ext, section } from '../layout.ts';
import { factoryDiagram, heroMarker, icon } from '../illustrations.ts';
import { feedbackLinks, repoLink, site } from '../config.ts';
import { headlineGroup, type Group, type Run } from '../benchmark.ts';
import { heroLegend, heroLive, metres } from '../hero-live.ts';
import { faqList, topFaq } from '../faq.ts';
import { pick } from './benchmark.ts';
import type { BuildContext } from './context.ts';

/** Three ways in, by visitor. Each has a real screenshot of the app (sample data) and three plain points. */
const paths = [
  {
    id: 'path-paper', who: 'Small unit with paper patterns', title: 'Photo of your paper patterns', img: 'path-paper', phone: true,
    alt: 'Photo to marker on a phone: the pieces BETA found in a photo of paper patterns, outlined, with an A4 sheet for scale',
    points: ['Photograph the paper pieces on a dark cloth, with an A4 sheet beside them for scale.', 'Check one tape measurement, name each part and set its fabric.', 'Get a marker, full-size tiles to print, or a chalk guide. No CAD needed.'],
    cta: 'Digitize paper patterns', href: 'app/#/quick',
  },
  {
    id: 'path-pattern', who: 'Pattern master', title: 'Make a style from a block', img: 'path-pattern', phone: false,
    alt: 'Make a style: the T-shirt block on the drafting canvas with front and back pieces, notches and grain lines',
    points: ['Start from seven basic blocks (T-shirt, shirt, trouser, skirt, kurta, polo, track pant) or a blank sheet.', 'Grade every size from one size chart, with seam checks and a spec sheet.', 'Export DXF-AAMA, plotter PLT and a 1:1 PDF.'],
    cta: 'Make a style', href: 'app/#/style',
  },
  {
    id: 'path-room', who: 'Cutting room', title: 'From DXF to the cutting floor', img: 'path-marker', phone: false,
    alt: 'A T-shirt marker in four sizes, every piece labelled with its name and size, all checks passed',
    points: ['Open DXF-AAMA pieces from your CAD and nest a marker on your fabric width.', 'Plan the cut: lays, plies and fabric for the whole order, and compare widths by cost.', 'Rolls, spreading sheets, bundles, tickets and the sewing hand-off.'],
    cta: 'Open DXF files', href: 'app/#/marker',
  },
];

/** Everything that exists, grouped. "New" marks what 0.8 adds. */
const groups: { icon: string; title: string; items: [string, boolean?][] }[] = [
  { icon: 'pattern', title: 'Pattern', items: [
    ['Seven editable basic blocks or a blank sheet, on a drafting canvas'],
    ['Size-chart grading, all sizes stacked, seam checks'],
    ['Spec sheet: points of measure per size (PDF, Excel)'],
    ['DXF-AAMA, plotter PLT and 1:1 PDF out; DXF-AAMA and a Seamly2D subset in'],
  ] },
  { icon: 'marker', title: 'Marker', items: [
    ['Automatic nesting with grain, one-way nap, left/right pairs, ratios and gaps'],
    ['The marker improves live on screen; stop and keep the best'],
    ['Improve one area by hand: preview, apply, undo', true],
    ['Keeps your previous marker when it is shorter and still valid', true],
    ['Every marker checked by an independent validator; plotter, DXF, PDF and cutter files'],
  ] },
  { icon: 'plan', title: 'Cutting room', items: [
    ['Cut plans: ratios, plies, passes and fabric for an order'],
    ['Compare fabric widths, ratios and the cost of the whole order', true],
    ['Rolls: 4-point inspection, shade and shrinkage groups'],
    ['Spreading sheets, bundles, QR tickets, re-cuts and the sewing hand-off'],
  ] },
  { icon: 'people', title: 'Small factories', items: [
    ['Photo to marker on a phone, with an A4 sheet as the scale'],
    ['Reusable paper pattern sets with a tape check', true],
    ['Fabric groups (main, lining, rib, fusing) with GSM, one marker each', true],
    ['Full-size tiles, chalk guide and projector mode; works offline as an app'],
  ] },
];

const notYet = [
  'The 0.8 additions (area improvement, order-cost comparison, paper pattern sets) are new and tested only in parts.',
  'Basic blocks pass 10,806 automatic checks, but they need sewn samples and a fit review by a pattern master.',
  'Folded and tube lays use more fabric than open lays on many orders. The app shows the real yield so you can see it.',
  'Real-photo accuracy of Photo to marker is not yet measured in factories. Check sizes against the paper before cutting.',
  'Plotter and cutter profiles are generic. Test them on your machines before production.',
  'The browser plans on one thread (about 2 seconds per marker). Big jobs are much better in the desktop app.',
  'The Windows installer is not code-signed yet, so Windows may show a warning. Hindi and Tamil are drafts.',
];

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
  <p class="small muted">${g.valid} of ${g.attempts} BETA layouts passed the independent validator. A tie is within 0.1 points. Measured ${run.date}.</p>
</article>`;
}

/** Benchmark highlight: mean, median and win/tie/loss always together, from the data; folded fabric said plainly. */
function benchmarkHighlight(runs: Run[]): Raw {
  const lm = pick(runs, 'lectra-matched'); const l60 = pick(runs, 'lectra-60s-08') ?? pick(runs, 'lectra-60s');
  const shown = [lm, l60].filter((r): r is Run => r !== undefined);
  if (!shown.length) return html``;
  const losesMore = shown.some((r) => { const g = headlineGroup(r); return g.win !== null && g.loss !== null && g.loss + g.too_long_losses > g.win; });
  const summary = losesMore
    ? 'BETA is close to Lectra’s recorded results, but it still loses more tasks than it wins. We publish every number so you can check.'
    : 'BETA is close to Lectra’s recorded results on this sample. This is not a general win. We publish every number so you can check.';
  const cards: Raw[] = [];
  if (lm) cards.push(benchCard(lm, 'Historical 20 October: with Lectra’s own time per task', `About 5 minutes per task · ${headlineGroup(lm).attempts} tasks`));
  if (l60) cards.push(benchCard(l60, '0.8, 28 October: 60 seconds per task', `${headlineGroup(l60).attempts} tasks`));
  return section('benchmark', 'Honest numbers: mixed, not a win', html`
<div class="bench-grid">${cards}</div>
<div class="bench-foot" data-reveal>
  <p><strong>No general win over Lectra.</strong> ${summary} Points (pp) are BETA’s marker efficiency minus Lectra’s; below zero means BETA’s marker was longer.</p>
  <a class="link-arrow" href="benchmark.html">Method, assumptions and all results<span aria-hidden="true"> →</span></a>
</div>
<div class="callout callout-plain fold-note" data-reveal>
  <h3>Folded fabric is still behind open fabric</h3>
  <p>In our 0.7 tests, folded T-shirt orders with odd counts per size needed 23% to 74% more fabric than an open lay, and tubes about 11% to 12% more. 0.8 tries mixed fold choices per piece: in a small pilot on 12 public jobs the change ranged from 1.9% more fabric (folded hoodie) to 5.7% less (shirt with a locked piece); the folded T-shirt did not change. The app shows the real yield and the pieces cut but not used, so you can decide.</p>
</div>`, { eyebrow: 'Public benchmark', lead: 'We test BETA on the public Lectra dataset: real nesting tasks from fashion companies, with the efficiency Lectra’s own engine reached on each one.', cls: 'section-alt' });
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

/** Hero picture: the recorded live marker from the real app, or the illustration when no recording exists. */
function heroPicture(ctx: BuildContext): Raw {
  const h = ctx.hero;
  if (!h) {
    return html`<figure class="hero-figure"><div class="hero-card"><div class="window-bar" aria-hidden="true"><i></i><i></i><i></i><span>Marker · shirt · sizes S, M, L</span><span class="badge">Illustration</span></div><div class="hero-stage is-playing" id="hero-stage">${heroMarker()}</div></div></figure>`;
  }
  const first = h.frames[0]!, last = h.frames[h.frames.length - 1]!;
  return html`<figure class="hero-figure hero-live" data-hero-live data-final="${metres(last.length_mm)}" aria-labelledby="hero-live-cap">
  <div class="hero-card">
    <div class="window-bar" aria-hidden="true"><i></i><i></i><i></i><span>Make a style · Sample T-shirt · Marker</span><span class="badge badge-final">Real app</span></div>
    <div class="hz-stage">${heroLive(h)}</div>
    <div class="hz-foot">
      <p class="hz-status" aria-hidden="true"><span class="hz-run"><span class="spinner"></span>Best so far <b data-hz-length>${metres(first.length_mm)}</b></span><span class="hz-done"><span class="tick">✓</span><b>${metres(last.length_mm)}</b>${h.efficiency ? html` · ${h.efficiency}% of the fabric in pieces` : ''} · checked</span></p>
      ${heroLegend(h)}
      <button type="button" class="btn-icon hz-toggle" data-hz-toggle hidden aria-pressed="false"><svg class="i-pause" viewBox="0 0 20 20" width="16" height="16" aria-hidden="true" focusable="false"><path d="M7 5v10M13 5v10" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"/></svg><svg class="i-play" viewBox="0 0 20 20" width="16" height="16" aria-hidden="true" focusable="false"><path d="M7 4.5v11l9-5.5z" fill="currentColor"/></svg><span class="sr-only" data-hz-label>Pause the animation</span></button>
    </div>
  </div>
  <figcaption id="hero-live-cap">Recorded in the browser app: a T-shirt in sizes ${h.sizes.join(', ')}, 15 seconds of search. The marker got shorter from ${metres(first.length_mm)} to ${metres(last.length_mm)}. Shorter is better.</figcaption>
</figure>`;
}

function pathCard(p: (typeof paths)[number], i: number): Raw {
  const pic = p.phone
    ? html`<div class="path-phone"><img src="${`assets/screens/${p.img}-390.webp`}" srcset="${`assets/screens/${p.img}-390.webp 390w, assets/screens/${p.img}-600.webp 600w`}" sizes="(max-width: 700px) 60vw, 240px" alt="${p.alt}" width="390" height="700" loading="lazy" decoding="async"></div>`
    : html`<div class="path-shot"><img src="${`assets/screens/${p.img}-800.webp`}" srcset="${`assets/screens/${p.img}-800.webp 800w, assets/screens/${p.img}-1200.webp 1200w`}" sizes="(max-width: 700px) 92vw, (max-width: 1100px) 46vw, 400px" alt="${p.alt}" width="800" height="500" loading="lazy" decoding="async"></div>`;
  return html`<article class="path-card${p.phone ? ' path-card-phone' : ''}" id="${p.id}" data-reveal>
  <div class="path-pic">${pic}</div>
  <div class="path-copy">
    <p class="path-who"><span class="path-num">${i + 1}</span>${p.who}</p>
    <h3>${p.title}</h3>
    <ul class="check-list">${p.points.map((x) => html`<li>${x}</li>`)}</ul>
    <a class="btn btn-secondary btn-small" href="${p.href}">${p.cta}<span aria-hidden="true"> →</span></a>
  </div>
</article>`;
}

export function homePage(ctx: BuildContext): Raw {
  const d = site.downloads;
  return html`
<div class="hero hero-launch">
  <div class="hero-bg" aria-hidden="true"><span class="blob blob-1"></span><span class="blob blob-2"></span><span class="blob blob-3"></span></div>
  <div class="wrap hero-inner">
    <div class="hero-copy">
      <p class="hero-pills"><span class="pill">Version ${site.version}</span><span class="pill">Free and open source</span><a class="pill pill-fun" href="#help"><span aria-hidden="true">✦</span> A fun project, built with modern AI tools</a></p>
      <h1 class="hero-title">Free, open-source <span class="hl"><span class="nowrap">cutting-room</span> software</span> for garment factories</h1>
      <p class="hero-lead">Patterns, markers and cut plans, in your browser. Nothing to install, no account, and your files stay on your device.</p>
      <div class="cta-row">
        ${site.tourUrl ? downloadButton(site.tourUrl, 'Try it now, no install', 'btn btn-primary btn-large btn-try', 'A 60-second tour on a real T-shirt') : ''}
        <a class="btn btn-secondary btn-large" href="#download"><span class="btn-label">Download</span><span class="btn-sub">Windows app · Linux server</span></a>
      </div>
      <p class="small muted hero-note">${site.webAppUrl ? html`Or <a href="${site.webAppUrl}">open the app in your browser</a> without the tour. ` : ''}Best for trying and small jobs; for big markers use the desktop app.</p>
      <ul class="trust-list">
        <li>Free · Apache-2.0</li><li>Runs on your device</li><li>No account</li><li>No tracking</li>
      </ul>
    </div>
    ${heroPicture(ctx)}
  </div>
</div>

${demoVideo(ctx)}

${section('paths', 'Start where you are', html`
<div class="paths">${paths.map((p, i) => pathCard(p, i))}</div>
<p class="note">Real screens of the app with sample data. Every path has a sample, so you can try it before you use your own files.</p>`, { eyebrow: 'Three ways in', lead: 'Paper patterns on a phone, a pattern master’s drafting table, or a cutting room with CAD files: each one works in the browser.' })}

${section('features', 'What BETA does today', html`
<div class="card-grid card-grid-4 feature-groups" data-reveal data-stagger>
  ${groups.map((g) => html`<article class="card feature-card"><span class="card-icon">${icon(g.icon)}</span><h3>${g.title}</h3><ul class="check-list">${g.items.map(([text, isNew]) => html`<li>${text}${isNew ? html` <span class="new-tag">New in 0.8</span>` : ''}</li>`)}</ul></article>`)}
</div>
<div class="callout callout-plain not-yet" data-reveal>
  <h3>Not yet, or not proven</h3>
  <p>We prefer to tell you before you find out.</p>
  <ul>${notYet.map((n) => html`<li>${n}</li>`)}</ul>
</div>`, { eyebrow: 'Features', lead: 'Everything runs on your own computer. The heavy work (geometry, nesting, checks, files) is written in Rust for speed.', cls: 'section-alt' })}

${benchmarkHighlight(ctx.runs)}

<section class="section section-data" id="privacy" aria-labelledby="privacy-title">
  <div class="wrap split split-even">
    <div data-reveal>
      <p class="eyebrow">Privacy</p>
      <h2 id="privacy-title">Your files stay on your device</h2>
      <p class="lead">BETA runs in your browser, in the desktop app, or on a factory server you choose. There is no BETA cloud.</p>
      <ul class="check-list check-list-big">
        <li>Patterns, markers, orders and roll data stay in this browser or on your own PCs and server.</li>
        <li>No account, no tracking, no usage data and no crash reports sent anywhere. This website has no cookies.</li>
        <li>Works without internet once installed or opened once. Keep backup files of your work.</li>
        <li>Feedback is your choice. GitHub issues are public, so do not attach factory files there.</li>
      </ul>
      <p><a class="link-arrow" href="install.html#privacy">Privacy and self-hosting<span aria-hidden="true"> →</span></a></p>
    </div>
    <div class="factory-wrap" data-reveal>${factoryDiagram()}</div>
  </div>
</section>

<section class="section section-alt" id="help" aria-labelledby="help-title">
  <div class="wrap fun-grid">
    <div class="fun-card" data-reveal>
      <span class="fun-badge"><span aria-hidden="true">✦</span> A fun project</span>
      <h2 id="help-title">A fun project, built with modern AI tools: we need your help</h2>
      <p>BETA Cutting Room is a fun project by a small team. We build it with the help of modern AI coding assistants, and we test it like serious software: an independent validator checks every marker before you can export it.</p>
      <p><strong>BETA is young and not perfect yet.</strong> Try it on a real job and tell us what is wrong, confusing or missing. Pattern masters, cutting masters and developers are all welcome.</p>
      <div class="contribute-row">
        ${site.repoUrl ? html`<a class="btn btn-secondary btn-small" href="${site.repoUrl}" rel="noopener noreferrer external">See the code on GitHub<span class="sr-only"> (external site)</span></a>` : ''}
        ${repoLink('CONTRIBUTING.md') ? html`<a class="btn btn-secondary btn-small" href="${repoLink('CONTRIBUTING.md')!}" rel="noopener noreferrer external">How to contribute<span class="sr-only"> (external site)</span></a>` : ''}
      </div>
    </div>
    <div class="help-block" data-reveal>
      <h3 class="h-help">Tell us what to fix</h3>
      <p>A short note is enough, and every report helps.</p>
      ${helpLinks()}
      <p class="small muted samples-note"><strong>Have a real job to compare?</strong> Send 3 to 5 styles as DXF-AAMA with the marker your CAD made (width, ratio, length, efficiency) to ${contactLine()}. Your files stay private; we publish nothing without your written permission.</p>
    </div>
  </div>
</section>

${section('faq', 'Questions people ask first', html`${faqList(topFaq(ctx.runs))}<p class="note"><a class="link-arrow" href="docs.html#faq">More questions and the manuals<span aria-hidden="true"> →</span></a></p>`, { eyebrow: 'FAQ' })}

${section('download', 'Download', html`
<div class="card-grid card-grid-4" data-reveal data-stagger>
  <article class="card download-card download-main">
    <span class="card-icon">${icon('spark')}</span>
    <h3>Try in the browser</h3>
    <p>Nothing to install. The 60-second tour shows a real T-shirt from pattern to cut plan, then you can try your own.</p>
    ${downloadButton(site.tourUrl, 'Start the tour')}
    <p class="small muted">${site.webAppUrl ? html`<a href="${site.webAppUrl}">Open the app without the tour</a>` : ''}</p>
  </article>
  <article class="card download-card">
    <span class="card-icon">${icon('download')}</span>
    <h3>Windows app</h3>
    <p>For one PC. Windows 10 or 11, 64-bit. ${d.windows.size}. No administrator rights needed. Or the portable ZIP (${d.portable.size}).</p>
    ${downloadButton(d.windows.url, 'Download installer', 'btn btn-secondary')}
    <p class="small muted">${downloadButton(d.portable.url, 'Portable ZIP', 'link-plain')} · <a href="install.html#windows">Installation steps</a></p>
  </article>
  <article class="card download-card">
    <span class="card-icon">${icon('server')}</span>
    <h3>Linux server</h3>
    <p>Runs the same app for a whole factory network with <code>beta-marker serve</code>. ${d.server.size}. Needs glibc 2.31 or newer.</p>
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
