import { html, type Raw } from '../html.ts';
import { ext, pageHead, section } from '../layout.ts';
import { repoLink } from '../config.ts';
import { helpSection } from './home.ts';

const technical: [string, string, string][] = [
  ['marker/README.md', 'BETA Marker overview', 'What the marker engine does, how it is built and where each part lives.'],
  ['marker/SPEC.md', 'Specification', 'Data formats, garment rules, units and the validator.'],
  ['marker/docs/CUTTING-ROOM-STATUS.md', 'Cutting-room module status', 'All 14 cutting-room modules: what works, what is tested and what is still missing.'],
  ['marker/docs/benchmarks.md', 'Benchmark method', 'Every benchmark rule, assumption, result and command.'],
  ['marker/docs/QA-2026-10-19.md', 'Quality report', 'End-to-end test of version 0.4 with findings.'],
  ['marker/docs/V04-RESEARCH.md', 'Market and research notes', 'What other systems do, with sources, and the plan for version 0.4.'],
  ['marker/apps/desktop/DESKTOP_REPORT.md', 'Desktop app report', 'How the Windows app is built and tested.'],
  ['marker/apps/web/README.md', 'App and server build', 'How to build the app and run the server.'],
  ['CONTRIBUTING.md', 'Contributing', 'How to propose changes and what each change needs.'],
];

const faq: [string, Raw][] = [
  ['Do I need an internet connection?', html`No. The desktop app and the server work offline. Install and update from a USB drive if you need to.`],
  ['Which pattern files can I open?', html`DXF-AAMA and ASTM pattern files, BETA pattern files, and a supported subset of Seamly2D files. Test a few of your own styles first, because each CAD writes DXF a little differently.`],
  ['Can it drive my plotter or cutter?', html`It writes PLT/HPGL and 1:1 PDF for plotters, and a generic ISO file for cutters. Profiles for specific machines are not certified yet. Plot a 1,000 mm test line and check it before production.`],
  ['Is BETA better than my current CAD?', html`We do not know yet, and we do not claim it. On the public Lectra data, results depend on the time allowed and the size of the marker, and BETA loses on many tasks. See the <a href="benchmark.html">benchmark</a>, and <a href="index.html#samples">send us your markers</a> for a comparison on your styles.`],
  ['What does it cost?', html`The software is free and open source. You pay only for your own hardware, and for any support or training you choose to buy from someone.`],
  ['Which languages are supported?', html`English today. Hindi and Tamil are planned.`],
];

function docLink(path: string, title: string): Raw {
  const href = repoLink(path);
  return href ? ext(href, title) : html`${title}`;
}

export function docsPage(hasManual: boolean): Raw {
  return html`
${pageHead('Help and documentation', 'Start with the user manual. The technical documents explain how BETA works, how it is tested and what is still missing.', 'Help')}

${section('manual', 'User manual', html`
<div class="card-grid card-grid-2">
  <article class="card">
    <h3>Cutting-room manual</h3>
    <p>Step-by-step help for the floor screens: orders and rolls import, spreading, tracking, re-cuts and the sewing hand-off.</p>
    ${hasManual ? html`<a class="btn btn-secondary" href="docs/cutting-room-manual.html">Open the manual</a>` : html`<span class="btn btn-secondary btn-disabled" aria-disabled="true">Manual coming soon</span>`}
  </article>
  <article class="card">
    <h3>Help inside the app</h3>
    <p>The app has a Help tab with the same information, available offline. Every command-line tool also explains itself: run <code>beta-marker --help</code> or <code>beta-pattern --help</code>.</p>
  </article>
</div>`)}

${section('technical', 'Technical documents', html`
<p class="small muted">These documents are in the source code. ${repoLink('x') ? 'Links open the public repository.' : 'The public repository link will be added at release; until then each document is listed with its path.'}</p>
<ul class="doc-list">
  ${technical.map(([path, title, text]) => html`<li><span class="doc-title">${docLink(path, title)}</span><span>${text}</span><code>${path}</code></li>`)}
</ul>`, { cls: 'section-alt' })}

${section('faq', 'Common questions', html`
<div class="faq">
  ${faq.map(([q, a]) => html`<details class="disclosure"><summary>${q}</summary><p>${a}</p></details>`)}
</div>`)}

${helpSection('section-alt')}
`;
}
