import { html, type Raw } from '../html.ts';
import { ext, pageHead, section } from '../layout.ts';
import { repoLink, site } from '../config.ts';
import { contactLine } from './home.ts';

const credits: [string, string, string, string][] = [
  ['sparrow', 'MIT', 'https://github.com/JeroenGar/sparrow', 'The nesting search BETA uses (version 0.3.0). By Jeroen Gardeyn and co-authors.'],
  ['jagua-rs', 'MPL-2.0', 'https://github.com/JeroenGar/jagua-rs', 'The collision engine under sparrow. Used unmodified, as a separate library. If we ever change its files, we will publish those changes under MPL-2.0.'],
  ['FreeSewing', 'MIT', 'https://freesewing.eu/', 'Open garment patterns used for tests and benchmarks.'],
  ['GarmentCodeData', 'CC BY 4.0', 'https://doi.org/10.3929/ethz-b-000690432', 'Open garment pattern data by Korosteleva et al. (ETH Zürich), used for tests.'],
  ['Open+Pattern PT windbreaker', 'CC BY 4.0', 'https://openpattern.io/product/us-pt-windbreaker-jacket/', 'A real graded DXF-AAMA pattern by Open+Pattern, used to test pattern import.'],
  ['Lectra nesting dataset', 'CC BY 4.0', 'https://doi.org/10.5281/zenodo.7030786', 'Lallier, Vézard, Pinaud and Blin (2022), Zenodo 7030786. Used for the benchmark only; not stored in our code.'],
  ['Tauri', 'MIT or Apache-2.0', 'https://tauri.app', 'The desktop app shell for Windows.'],
];

export function openSourcePage(): Raw {
  const licenceHref = repoLink('LICENSE');
  return html`
${pageHead('Open source', 'BETA Cutting Room is free software. You can use it, study it, change it and share it, also in commercial work.', 'Licence and credits')}

${section('licence', 'Licence', html`
<div class="split">
  <div class="prose">
    <p>The code is licensed under the <strong>Apache License, Version 2.0</strong>. In short, you may:</p>
    <ul class="check-list">
      <li>use it in your factory, free of charge, on as many computers as you like;</li>
      <li>change it and share your changes;</li>
      <li>include it in your own products, open or closed.</li>
    </ul>
    <p>You must keep the copyright and licence notices. The licence includes a patent grant and has no warranty. Open data made by the project uses CC BY 4.0.</p>
    <p>${licenceHref ? ext(licenceHref, 'Read the full licence') : html`The full licence text is in the <code>LICENSE</code> file of the source code`} or at ${ext('https://www.apache.org/licenses/LICENSE-2.0', 'apache.org')}.</p>
  </div>
  <aside class="callout callout-plain">
    <h3>Only permissive licences</h3>
    <p>BETA only adds dependencies under permissive licences (MIT, Apache-2.0, BSD, ISC, zlib), plus jagua-rs under MPL-2.0, used unmodified. A licence check runs on every desktop build. The desktop app ships the full third-party notices.</p>
  </aside>
</div>`)}

${section('credits', 'Built on the work of others', html`
<div class="table-wrap" tabindex="0" role="region" aria-label="Credits"><table class="data-table">
<caption class="sr-only">Open-source projects and data that BETA uses</caption>
<thead><tr><th scope="col">Project</th><th scope="col">Licence</th><th scope="col">How BETA uses it</th></tr></thead>
<tbody>${credits.map(([name, lic, url, use]) => html`<tr><th scope="row">${ext(url, name)}</th><td>${lic}</td><td>${use}</td></tr>`)}</tbody>
</table></div>
<div class="callout callout-plain">
  <h3>Seamly2D and Valentina</h3>
  <p>Seamly2D and Valentina are good open-source pattern programs under the GPL licence. BETA contains none of their code. BETA Pattern reads a supported subset of their pattern files so that you can bring existing work across. Any fixes we offer to Seamly2D are sent to that project under its own licence.</p>
</div>`, { cls: 'section-alt' })}

${section('contribute', 'Contribute', html`
<div class="card-grid card-grid-3">
  <article class="card"><h3>Report a problem</h3><p>Tell us what you did, what you expected and what happened. Attach a small test file that you are allowed to share. Never attach a buyer’s pattern without permission.</p></article>
  <article class="card"><h3>Improve the code</h3><p>Open an issue, then a focused pull request with tests. Computing parts are in Rust; screens are in TypeScript. Read <code>CONTRIBUTING.md</code> first.</p></article>
  <article class="card"><h3>Improve the benchmark</h3><p>Run it on your hardware and send the summary files. Better nesting must win on held-out tasks, with every layout valid.</p></article>
</div>
<p>${site.repoUrl ? html`Source code: ${ext(site.repoUrl, site.repoUrl)}` : html`<span class="placeholder-text">The public repository link will be added at release.</span>`}</p>`)}

${section('contact', 'Contact', html`
<div class="prose">
  <p>Questions, factory trials and sample markers: ${contactLine()}.</p>
  <p>We read every message. We answer in English; Hindi and Tamil support is planned.</p>
</div>`, { cls: 'section-alt' })}
`;
}
