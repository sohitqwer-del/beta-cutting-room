import { html, raw, type Part, type Raw } from './html.ts';
import { site } from './config.ts';

export interface PageMeta {
  file: string;
  title: string;
  navLabel: string;
  description: string;
}

export const pages: PageMeta[] = [
  { file: 'index.html', navLabel: 'Home', title: 'BETA Cutting Room: open-source cutting-room software for garment factories', description: 'Open-source, self-hosted cutting-room software for garment factories: pattern, marker, cut planning, rolls, spreading and bundles. Runs offline on your own computers.' },
  { file: 'benchmark.html', navLabel: 'Benchmark', title: 'Benchmark: honest, repeatable marker results | BETA Cutting Room', description: 'How BETA markers compare with Lectra’s recorded results and published research, with method, assumptions and commands to repeat every number.' },
  { file: 'compare.html', navLabel: 'Compare', title: 'Compare with commercial cutting-room software | BETA Cutting Room', description: 'Feature comparison with Tukatech, Gerber AccuMark, Lectra, Optitex, Audaces and Richpeace/BOKE, based on vendor documents.' },
  { file: 'install.html', navLabel: 'Install', title: 'Install and self-host | BETA Cutting Room', description: 'Install BETA Cutting Room on a Windows PC or run it on your own server. System requirements, privacy and offline use.' },
  { file: 'docs.html', navLabel: 'Help', title: 'Help and documentation | BETA Cutting Room', description: 'Manuals, technical documents and answers to common questions.' },
  { file: 'open-source.html', navLabel: 'Open source', title: 'Open source, licences and credits | BETA Cutting Room', description: 'Licence, credits for the open-source projects BETA builds on, how to contribute and how to contact us.' },
];

// Runs before paint so a saved theme choice does not flash.
const themeBoot = `(function(){try{var t=localStorage.getItem('beta-site-theme');if(t==='light'||t==='dark')document.documentElement.setAttribute('data-theme',t);}catch(e){}})();`;

// Cycles Auto -> Light -> Dark. Works from file:// (no modules, no fetch).
const themeToggle = `(function(){var b=document.getElementById('theme-toggle');if(!b)return;var root=document.documentElement;var order=['auto','light','dark'];var names={auto:'Automatic',light:'Light',dark:'Dark'};
function current(){return root.getAttribute('data-theme')||'auto';}
function show(){var c=current();b.querySelector('[data-theme-label]').textContent=names[c];b.setAttribute('aria-label','Colour theme: '+names[c]+'. Change theme');}
b.addEventListener('click',function(){var n=order[(order.indexOf(current())+1)%3];if(n==='auto'){root.removeAttribute('data-theme');try{localStorage.removeItem('beta-site-theme');}catch(e){}}else{root.setAttribute('data-theme',n);try{localStorage.setItem('beta-site-theme',n);}catch(e){}}show();});
b.hidden=false;show();})();`;

export const logo = raw(`<svg class="logo-mark" viewBox="0 0 32 32" width="32" height="32" aria-hidden="true" focusable="false"><rect width="32" height="32" rx="8" fill="currentColor"/><path d="M10 8h7.2c3.3 0 5.4 1.7 5.4 4.3 0 1.8-1 3-2.5 3.6 2 .5 3.3 1.9 3.3 4 0 2.9-2.3 4.6-5.9 4.6H10z M13.3 11v4h3.4c1.5 0 2.4-.8 2.4-2s-.9-2-2.4-2z M13.3 17.9v4.2h3.9c1.6 0 2.6-.8 2.6-2.1s-1-2.1-2.6-2.1z" fill="var(--logo-ink)" fill-rule="evenodd"/></svg>`);

function navLinks(current: string, cls: string): Raw {
  return html`<ul class="${cls}">${pages.map((p) => html`<li><a href="${p.file}"${p.file === current ? raw(' aria-current="page"') : ''}>${p.navLabel}</a></li>`)}</ul>`;
}

/** Wraps page content in the shared header, navigation and footer. */
export function layout(meta: PageMeta, main: Part): string {
  const doc = html`<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${meta.title}</title>
<meta name="description" content="${meta.description}">
<meta name="color-scheme" content="light dark">
<meta name="referrer" content="no-referrer">
<link rel="icon" href="assets/favicon.svg" type="image/svg+xml">
<link rel="stylesheet" href="assets/site.css">
<script>${raw(themeBoot)}</script>
</head>
<body>
<a class="skip-link" href="#main">Skip to main content</a>
<header class="site-header">
  <div class="wrap header-inner">
    <a class="brand" href="index.html">${logo}<span>${site.name}</span></a>
    <nav class="nav-desktop" aria-label="Main">${navLinks(meta.file, 'nav-list')}</nav>
    <div class="header-tools">
      <button type="button" id="theme-toggle" class="theme-toggle" hidden>
        <svg viewBox="0 0 20 20" width="18" height="18" aria-hidden="true" focusable="false"><circle cx="10" cy="10" r="7.25" fill="none" stroke="currentColor" stroke-width="1.5"/><path d="M10 2.75a7.25 7.25 0 0 1 0 14.5z" fill="currentColor"/></svg>
        <span data-theme-label>Automatic</span>
      </button>
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
    <div>
      <p class="footer-brand">${logo}<span>${site.name} ${site.version}</span></p>
      <p>Open source under the Apache License 2.0. Self-hosted. Your patterns stay in your factory.</p>
      <p class="muted">This website has no trackers, no cookies and loads nothing from other servers.</p>
    </div>
    <nav aria-label="Footer">${navLinks(meta.file, 'footer-list')}</nav>
  </div>
</footer>
<script>${raw(themeToggle)}</script>
</body>
</html>
`;
  return doc.value;
}

/** A labelled section with an optional lead paragraph. */
export function section(id: string, title: string, body: Part, opts: { lead?: string; cls?: string; eyebrow?: string } = {}): Raw {
  return html`<section class="section ${opts.cls ?? ''}" id="${id}" aria-labelledby="${id}-title">
  <div class="wrap">
    ${opts.eyebrow ? html`<p class="eyebrow">${opts.eyebrow}</p>` : ''}
    <h2 id="${id}-title">${title}</h2>
    ${opts.lead ? html`<p class="lead">${opts.lead}</p>` : ''}
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
</div></div>`;
}

/** External link that opens normally and leaks no referrer. */
export function ext(href: string, text: Part): Raw {
  return html`<a href="${href}" rel="noopener noreferrer external">${text}<span class="sr-only"> (external site)</span></a>`;
}
