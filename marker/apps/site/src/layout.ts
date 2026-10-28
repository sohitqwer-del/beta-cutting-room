import { html, raw, type Part, type Raw } from './html.ts';
import { feedbackLinks, site } from './config.ts';
import { markSvg } from './brand.ts';

export interface PageMeta {
  file: string;
  title: string;
  navLabel: string;
  description: string;
}

export const pages: PageMeta[] = [
  { file: 'index.html', navLabel: 'Home', title: 'BETA Cutting Room: free, open-source cutting-room software for garment factories', description: 'Free, open-source cutting-room software for garment factories: patterns, markers and cut plans, in your browser. See a real T-shirt marker in 60 seconds, with no install and no account.' },
  { file: 'benchmark.html', navLabel: 'Benchmark', title: 'Benchmark: honest, repeatable marker results | BETA Cutting Room', description: 'How BETA markers compare with Lectra’s recorded results and published research, with method, assumptions and commands to repeat every number.' },
  { file: 'compare.html', navLabel: 'Compare', title: 'Compare with commercial cutting-room software | BETA Cutting Room', description: 'Feature comparison with Tukatech, Gerber AccuMark, Lectra, Optitex, Audaces and Richpeace/BOKE, based on vendor documents.' },
  { file: 'install.html', navLabel: 'Install', title: 'Install and self-host | BETA Cutting Room', description: 'Install BETA Cutting Room on a Windows PC or run it on your own server. System requirements, privacy and offline use.' },
  { file: 'docs.html', navLabel: 'Help', title: 'Help and documentation | BETA Cutting Room', description: 'Manuals, technical documents and answers to common questions.' },
  { file: 'open-source.html', navLabel: 'Open source', title: 'Open source, licences and credits | BETA Cutting Room', description: 'Licence, credits for the open-source projects BETA builds on, how to contribute and how to contact us.' },
];

// Runs before first paint. Turns on motion styles only when the visitor allows motion.
// If assets/site.js does not run within 2.5 s, motion is turned off again, so content is never hidden.
const motionBoot = `(function(){var r=document.documentElement;try{if(window.matchMedia&&!matchMedia('(prefers-reduced-motion: reduce)').matches&&'IntersectionObserver' in window){r.classList.add('motion');setTimeout(function(){if(!r.classList.contains('motion-ready'))r.classList.remove('motion');},2500);}}catch(e){}})();`;

/** The brand mark for headers and footers (decorative; the name is next to it). */
export const logo = raw(markSvg({ size: 34, className: 'logo-mark' }));

/** "BETA Cutting Room" as styled text beside the mark. */
const brandName = html`<span class="brand-name"><b>BETA</b> Cutting Room</span>`;

function navLinks(current: string, cls: string): Raw {
  return html`<ul class="${cls}">${pages.map((p) => html`<li><a href="${p.file}"${p.file === current ? raw(' aria-current="page"') : ''}>${p.navLabel}</a></li>`)}</ul>`;
}

function absolute(path: string): string | null {
  return site.siteUrl ? `${site.siteUrl.replace(/\/?$/, '/')}${path}` : null;
}

/** Social card (1200 x 630, real app screen, logo and one line): made by the launch tools, see assets/brand/README.md. */
export const SOCIAL_IMAGE = { path: 'assets/brand/social-card.png', width: 1200, height: 630, alt: 'BETA Cutting Room: a real T-shirt marker made in the browser, with the line "Free, open-source cutting-room software for garment factories"' };

/** Wraps page content in the shared header, navigation and footer. */
export function layout(meta: PageMeta, main: Part): string {
  const ogImage = absolute(SOCIAL_IMAGE.path);
  const pageUrl = absolute(meta.file === 'index.html' ? '' : meta.file);
  const feedback = feedbackLinks();
  const doc = html`<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${meta.title}</title>
<meta name="description" content="${meta.description}">
<meta name="color-scheme" content="light">
<meta name="theme-color" content="#fbfaf6">
<meta name="referrer" content="no-referrer">
<meta property="og:type" content="website">
<meta property="og:site_name" content="${site.name}">
<meta property="og:title" content="${meta.title}">
<meta property="og:description" content="${meta.description}">
${pageUrl ? html`<meta property="og:url" content="${pageUrl}">` : ''}
<meta property="og:locale" content="en_GB">
${ogImage ? html`<meta property="og:image" content="${ogImage}">
<meta property="og:image:type" content="image/png">
<meta property="og:image:width" content="${SOCIAL_IMAGE.width}">
<meta property="og:image:height" content="${SOCIAL_IMAGE.height}">
<meta property="og:image:alt" content="${SOCIAL_IMAGE.alt}">` : ''}
<meta name="twitter:card" content="summary_large_image">
<meta name="twitter:title" content="${meta.title}">
<meta name="twitter:description" content="${meta.description}">
${ogImage ? html`<meta name="twitter:image" content="${ogImage}">
<meta name="twitter:image:alt" content="${SOCIAL_IMAGE.alt}">` : ''}
<meta name="application-name" content="${site.name}">
<link rel="icon" href="assets/favicon.svg" type="image/svg+xml">
<link rel="icon" href="assets/brand/favicon-32.png" type="image/png" sizes="32x32">
<link rel="apple-touch-icon" href="assets/brand/apple-touch-icon.png">
<link rel="manifest" href="site.webmanifest">
<link rel="preload" href="assets/fonts/bricolage-grotesque-latin-wght-normal.woff2" as="font" type="font/woff2" crossorigin>
<link rel="stylesheet" href="assets/site.css">
<script>${raw(motionBoot)}</script>
<script src="assets/site.js" defer></script>
</head>
<body>
<a class="skip-link" href="#main">Skip to main content</a>
<header class="site-header">
  <div class="wrap header-inner">
    <a class="brand" href="index.html">${logo}${brandName}</a>
    <nav class="nav-desktop" aria-label="Main">${navLinks(meta.file, 'nav-list')}</nav>
    <div class="header-tools">
      ${site.tourUrl ? html`<a class="btn btn-primary btn-small header-cta" href="${site.tourUrl}">Try it now</a>` : html`<a class="btn btn-primary btn-small header-cta" href="index.html#download">Download</a>`}
      <details class="nav-mobile">
        <summary><span class="sr-only">Menu</span><svg viewBox="0 0 20 20" width="20" height="20" aria-hidden="true" focusable="false"><path d="M3 5.5h14M3 10h14M3 14.5h14" stroke="currentColor" stroke-width="1.6" stroke-linecap="round"/></svg></summary>
        <nav aria-label="Main (small screens)">${navLinks(meta.file, 'nav-list-mobile')}</nav>
      </details>
    </div>
  </div>
</header>
<main id="main" tabindex="-1">
${main}
</main>
<footer class="site-footer">
  <div class="wrap footer-inner">
    <div class="footer-about">
      <p class="footer-brand">${logo}${brandName}</p>
      <p>${site.tagline} Version ${site.version}. Your files stay on your device.</p>
      <p class="footer-fun"><span aria-hidden="true">✦</span> A fun project, built with the help of modern AI tools.</p>
      <p class="muted small">This website has no trackers and no cookies, and it loads nothing from other servers.</p>
    </div>
    <nav class="footer-col" aria-label="Pages"><h2 class="footer-h">Pages</h2>${navLinks(meta.file, 'footer-list')}</nav>
    <div class="footer-col"><h2 class="footer-h">Help us improve</h2><ul class="footer-list">
      ${feedback.map((f) => html`<li><a href="${f.url}" rel="noopener noreferrer external">${f.label}<span class="sr-only"> (GitHub, external site)</span></a></li>`)}
      ${site.feedback.email ? html`<li><a href="${site.feedback.email}">Email us</a></li>` : ''}
    </ul></div>
    <div class="footer-col"><h2 class="footer-h">Project</h2><ul class="footer-list">
      ${site.repoUrl ? html`<li><a href="${site.repoUrl}" rel="noopener noreferrer external">Source code on GitHub<span class="sr-only"> (external site)</span></a></li>` : ''}
      <li><a href="open-source.html#licence">Licence: Apache-2.0</a></li>
      ${site.contactEmail ? html`<li><a href="${`mailto:${site.contactEmail}`}">${site.contactEmail}</a></li>` : ''}
    </ul></div>
  </div>
</footer>
</body>
</html>
`;
  return doc.value;
}

/** A labelled section with an optional lead paragraph. */
export function section(id: string, title: string, body: Part, opts: { lead?: string; cls?: string; eyebrow?: string } = {}): Raw {
  return html`<section class="section ${opts.cls ?? ''}" id="${id}" aria-labelledby="${id}-title">
  <div class="wrap">
    <div class="section-head" data-reveal>
      ${opts.eyebrow ? html`<p class="eyebrow">${opts.eyebrow}</p>` : ''}
      <h2 id="${id}-title">${title}</h2>
      ${opts.lead ? html`<p class="lead">${opts.lead}</p>` : ''}
    </div>
    ${body}
  </div>
</section>`;
}

/** Page title block used at the top of inner pages. */
export function pageHead(title: string, lead: Part, eyebrow?: string): Raw {
  return html`<div class="page-head"><div class="wrap">
  ${eyebrow ? html`<p class="eyebrow">${eyebrow}</p>` : ''}
  <h1>${title}</h1>
  <p class="lead">${lead}</p>
</div><div class="page-head-art" aria-hidden="true"><span></span><span></span><span></span></div></div>`;
}

/** External link that opens normally and leaks no referrer. */
export function ext(href: string, text: Part): Raw {
  return html`<a href="${href}" rel="noopener noreferrer external">${text}<span class="sr-only"> (external site)</span></a>`;
}
