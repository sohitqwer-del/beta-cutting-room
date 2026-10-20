import { html, type Raw } from '../html.ts';
import { ext, pageHead, section } from '../layout.ts';

// Facts from marker/docs/V04-RESEARCH.md (checked 19 October 2026) and marker/docs/PLAN.md.
// "doc" = the vendor's public material documents it. "nv" = our research did not verify it
// (this does not mean the feature is missing). Keep wording short; sources are listed below.

type Kind = 'yes' | 'partial' | 'doc' | 'no' | 'nv' | 'info';
interface Cell { k: Kind; t?: string; src?: string[] }
const yes = (t?: string): Cell => ({ k: 'yes', ...(t ? { t } : {}) });
const partial = (t: string): Cell => ({ k: 'partial', t });
const doc = (t: string | undefined, ...src: string[]): Cell => ({ k: 'doc', ...(t ? { t } : {}), src });
const nv = (t?: string): Cell => ({ k: 'nv', ...(t ? { t } : {}) });
const info = (t: string, ...src: string[]): Cell => ({ k: 'info', t, src });
const no = (t: string): Cell => ({ k: 'no', t });

const columns = [
  { key: 'beta', name: 'BETA Cutting Room', products: 'version 0.4' },
  { key: 'tuka', name: 'Tukatech', products: 'TUKAcad, TUKAmark, SMARTmark' },
  { key: 'gerber', name: 'Gerber', products: 'AccuMark, AccuNest, AccuPlan' },
  { key: 'lectra', name: 'Lectra', products: 'Diamino, Quick Nest, Flex Offer, Valia' },
  { key: 'optitex', name: 'Optitex', products: 'Marker, Nest++ Pro, Match++' },
  { key: 'audaces', name: 'Audaces', products: 'Marker, Supera' },
  { key: 'rp', name: 'Richpeace / BOKE', products: 'Super Nest V10 / V22 AI Super Nesting' },
] as const;

type Row = { feature: string; group: string; cells: Record<(typeof columns)[number]['key'], Cell> };

const rows: Row[] = [
  { group: 'Marker making', feature: 'Automatic nesting', cells: {
    beta: yes('sparrow engine with garment rules'), tuka: doc('SMARTmark (extra module)', 'T1'), gerber: doc('AccuNest Draft and Full modes', 'G3'),
    lectra: doc('Diamino; cloud Quick Nest / Flex Offer; Valia', 'L1', 'L4'), optitex: doc('Nest++ Pro, multicore', 'O3'), audaces: doc('Supera processing server', 'A1'), rp: doc('Super Nest V10; BOKE V22 AI', 'R1', 'B1') } },
  { group: 'Marker making', feature: 'Manual editing (bump, compact, swap)', cells: {
    beta: yes('bump in four directions, group compact, swap, sections'), tuka: doc('size sections, bump lines', 'T4'), gerber: doc('sliding, bump lines, compact marker, split piece', 'W1', 'W2', 'W4'),
    lectra: nv(), optitex: doc('batch bump lines, compaction (O/26)', 'O2'), audaces: nv(), rp: doc('Richpeace manual rescue, overlap fine-adjust; BOKE not verified', 'R4') } },
  { group: 'Marker making', feature: 'Grain, one-way nap, pairs, size ratio, gaps', cells: {
    beta: yes(), tuka: doc(undefined, 'PLAN'), gerber: doc(undefined, 'PLAN'), lectra: doc(undefined, 'PLAN'), optitex: doc(undefined, 'PLAN'), audaces: doc('grain and rotation restrictions', 'A2'), rp: nv() } },
  { group: 'Marker making', feature: 'Stripe and check matching', cells: {
    beta: partial('matching rules; no fabric-photo overlay yet'), tuka: doc('matching and grouping', 'T1'), gerber: doc('matching (2026.1 fixes)', 'G2'),
    lectra: doc('motif and panel management (Valia)', 'L4'), optitex: doc('Match++ module', 'O4'), audaces: nv(), rp: nv() } },
  { group: 'Marker making', feature: 'Fabric defects and shade zones', cells: {
    beta: yes('defects, shade bands'), tuka: doc(undefined, 'PLAN'), gerber: doc(undefined, 'PLAN'), lectra: doc('sectional and shade-zone nesting', 'L1'),
    optitex: doc('damaged zones, shading controls', 'O2'), audaces: nv(), rp: doc('Richpeace defect avoidance, shade sections', 'R1') } },
  { group: 'Marker making', feature: 'Folded or tubular fabric', cells: {
    beta: partial('supported; folded T-shirt test improved from 43% to 63% fabric use in 0.5, still below open fabric (~78%)'), tuka: nv(), gerber: doc(undefined, 'PLAN'), lectra: nv(), optitex: doc('fold work', 'O3'), audaces: nv(), rp: nv() } },
  { group: 'Planning and floor', feature: 'Cut order planning', cells: {
    beta: yes('ratios, plies, lay lengths'), tuka: doc('TUKAcutplan', 'PLAN'), gerber: doc('AccuPlan', 'G5'), lectra: doc('Valia planning', 'L6'),
    optitex: doc('CutPlan', 'PLAN'), audaces: nv(), rp: doc('BOKE cut-plan interface', 'B1') } },
  { group: 'Planning and floor', feature: 'Rolls, spreading, bundles and tickets', cells: {
    beta: yes('roll inspection, allocation, spreading sheets, QR bundle tickets'), tuka: nv(), gerber: doc('batch cut tickets', 'G1'), lectra: doc('actual-versus-plan KPIs, equipment hand-off (Valia)', 'L6'),
    optitex: nv(), audaces: doc('consumption monitoring in the wider ICF suite', 'A2'), rp: nv() } },
  { group: 'Planning and floor', feature: 'Pattern making and grading', cells: {
    beta: partial('drafting, grading, photo digitizing; not full commercial parity'), tuka: doc('TUKAcad', 'T2'), gerber: doc('AccuMark', 'G4'), lectra: nv(),
    optitex: doc('PDS', 'O4'), audaces: doc('Audaces360 bundles', 'A4'), rp: doc('Richpeace Garment CAD V10', 'R2') } },
  { group: 'Files', feature: 'DXF-AAMA / ASTM pattern import', cells: {
    beta: yes(), tuka: doc('import and export listed; full format list not verified', 'T2'), gerber: nv('full DXF converter list not verified'), lectra: doc('Flex Pattern Converter takes other CAD data', 'L1'),
    optitex: nv(), audaces: doc('DXF pattern intake', 'A2'), rp: doc('DXF, AAMA/ASTM (Richpeace); DXF conversion (BOKE)', 'R2', 'B1') } },
  { group: 'Files', feature: 'Plot and cutter output', cells: {
    beta: partial('PLT/HPGL, 1:1 PDF, SVG, DXF, generic ISO; no certified cutter profiles yet'), tuka: doc('PDF plot', 'T1'), gerber: doc('marker plot PDF', 'G1'), lectra: doc('equipment connections (Valia)', 'L6'),
    optitex: doc('PDF/DXF batch output', 'O2'), audaces: nv('plotting workflow; export list not published'), rp: doc('plot and cut output; PLT, cutting files (BOKE)', 'R2', 'B1') } },
  { group: 'Deployment and cost', feature: 'Where it runs', cells: {
    beta: yes('your Windows PC, or your own server'), tuka: doc('Windows workstation; Net-Q server queue', 'T3'), gerber: doc('Windows CAD and nesting server; optional cloud', 'G1'), lectra: doc('local Diamino; Quick/Flex/Valia in the cloud', 'L2', 'L6'),
    optitex: doc('installed desktop', 'O3'), audaces: doc('desktop plus processing server', 'A1'), rp: doc('installed Windows software', 'R1') } },
  { group: 'Deployment and cost', feature: 'Works with no internet connection', cells: {
    beta: yes(), tuka: nv(), gerber: nv(), lectra: nv(), optitex: nv(), audaces: nv(), rp: nv() } },
  { group: 'Deployment and cost', feature: 'Open source', cells: {
    beta: yes('Apache-2.0'), tuka: no('No (proprietary)'), gerber: no('No (proprietary)'), lectra: no('No (proprietary)'), optitex: no('No (proprietary)'), audaces: no('No (proprietary)'), rp: no('No (proprietary)') } },
  { group: 'Deployment and cost', feature: 'Published price', cells: {
    beta: yes('Free'), tuka: info('TUKAcad Professional $199/month incl. TUKAmark; SMARTmark: quote', 'T2'), gerber: info('Quote; 1–3 year subscriptions', 'G4'), lectra: info('Quote', 'L6'),
    optitex: info('Quote'), audaces: info('Quote', 'A4'), rp: info('Richpeace: quote. BOKE advertises $7,500 per set (V22 AI), undated', 'B1') } },
  { group: 'Deployment and cost', feature: 'Public, repeatable benchmark', cells: {
    beta: yes('this website'), tuka: no('None found'), gerber: no('None found'), lectra: info('Lectra engine results are in the open dataset BETA tests against; no tool-against-tool test found'), optitex: no('None found'), audaces: no('None found'), rp: no('None found') } },
];

const kindLabel: Record<Kind, string> = { yes: 'Yes', partial: 'Partly', doc: 'Vendor documents', no: '', nv: 'Not verified', info: '' };

function cellHtml(c: Cell): Raw {
  const label = kindLabel[c.k];
  // A documented cell with detail shows the ring mark and the detail; the words stay for screen readers.
  const labelHtml = !label ? '' : c.k === 'doc' && c.t ? html`<span class="sr-only">${label}: </span>` : html`<strong>${label}</strong>${c.t ? ': ' : ''}`;
  return html`<td class="cell cell-${c.k}"><span class="cell-mark" aria-hidden="true"></span><span>${labelHtml}${c.t ? html`<span class="${c.k === 'doc' ? 'cell-doc-detail' : ''}">${c.t}</span>` : ''}${c.src?.length ? html` <span class="src">${c.src.map((s) => html`<a href="${`#src-${s}`}">[${s}]</a>`)}</span>` : ''}</span></td>`;
}

const sources: [string, string, string][] = [
  ['PLAN', 'BETA plan comparison table (vendor documents, checked 18 October 2026)', ''],
  ['T1', 'Tukatech SMARTmark product page', 'https://tukatech.com/products/smartmark'],
  ['T2', 'Tukatech TUKAcad packages and prices', 'https://tukatech.com/products/tukacad'],
  ['T3', 'Tukatech SMARTmark queue guidance, 25 March 2025', 'https://tukatech.com/news/best-roi-smartmark-queue-and-netq'],
  ['T4', 'Tukatech sectional markers and bump lines (legacy tutorial)', 'https://tukatech.com/news/easy-cad-marker-making-p3'],
  ['G1', 'Gerber AccuMark public release history', 'https://help.gerbertechnology.com/Whats_New/AccuMark/SupportedVersions.htm'],
  ['G2', 'Gerber AccuMark Professional 2026.1 release notes', 'https://help.gerbertechnology.com/Resources/v2026.1.relnot_PE.pdf'],
  ['G3', 'Gerber AccuNest modes and editions', 'https://gerber-help.lectra.com/AccuNest/Getting_Started_with_AccuNest.htm'],
  ['G4', 'Lectra: Gerber AccuMark product and subscription', 'https://www.lectra.com/en/fashion/products/gerber-accumark-fashion'],
  ['G5', 'Gerber AccuPlan workflow', 'https://gerber-help.lectra.com/AccuPlan/Lesson_3.htm'],
  ['W1', 'Gerber help: bump lines and sliding', 'https://help.gerbertechnology.com/AccuMark/Easy_Marking/Group_Bumpline_Tab_Fabric.htm'],
  ['W2', 'Gerber help: Compact Marker', 'https://help.gerbertechnology.com/AccuMark/Easy_Marking/Tab_Marker.htm'],
  ['W4', 'Gerber help: Split Piece', 'https://gerber-help.lectra.com/AccuMark/Easy_Marking/Tab_Advanced.htm'],
  ['L1', 'Lectra Flex Offer specification, 2021', 'https://www.lectra.com/sites/default/files/2021-02/pr-flex-offer-by-lectra-en_VF%20%281%29.pdf'],
  ['L2', 'Lectra Flex Offer customer case, October 2024', 'https://www.lectra.com/en/library/united-creations-hits-new-production-targets-with-cloud-based-automated-nesting'],
  ['L4', 'Lectra Valia Fashion features, April 2026', 'https://www.lectra.com/en/library/valia-fashion-the-5-essential-features-transforming-fashion-manufacturing'],
  ['L6', 'Lectra Valia ERP, planning and equipment workflow, April 2026', 'https://www.lectra.com/en/library/discover-main-features-of-valia-fashion-fashion-manufacturing-solution-transforming-cutting'],
  ['O2', 'Optitex O/26 release', 'https://optitex.com/optitex-o26-release/'],
  ['O3', 'Optitex Marker product page', 'https://optitex.com/products/marker/'],
  ['O4', 'Optitex module brochure (Nest++ Pro, Match++)', 'https://exhibitorsearch.messefrankfurt.com/images/original/document_downloads/10000009202601/506383/1771345033642_2638381955.pdf'],
  ['A1', 'Audaces Supera', 'https://audaces.com/en/solutions/supera'],
  ['A2', 'Audaces FAQ', 'https://audaces.com/en/faq'],
  ['A4', 'Audaces company plans (price on request)', 'https://audaces.com/it/piani/aziende'],
  ['R1', 'Richpeace New Super Nest V10', 'https://www.richpeace.com/garment_cad-show-1147.html'],
  ['R2', 'Richpeace Garment CAD V10', 'https://www.richpeace.com/garment_cad-show-437.html'],
  ['R4', 'Richpeace CAD V9 brochure', 'https://download.richpeace.com/EN/2024/202401/2022.4.19%20CAD%20SOFTWARE%20V.9%281%29.pdf'],
  ['B1', 'BOKE V22 AI Super Nesting catalogue', 'https://en.bokecad.com/super-nesting/'],
];

export function comparePage(): Raw {
  const groups = [...new Set(rows.map((r) => r.group))];
  return html`
${pageHead('Compare', 'How BETA Cutting Room relates to the commercial systems many factories use today. We list only what public vendor material shows. Where our research could not confirm a fact, we say “not verified”.', 'Feature comparison')}

<div class="wrap">
  <div class="callout callout-plain">
    <h2 class="h-small">How to read this table</h2>
    <ul class="legend-list">
      <li><span class="cell-mark mark-yes" aria-hidden="true"></span><strong>Yes</strong>: BETA does this today.</li>
      <li><span class="cell-mark mark-partial" aria-hidden="true"></span><strong>Partly</strong>: BETA does part of it; the limit is stated.</li>
      <li><span class="cell-mark mark-doc" aria-hidden="true"></span><strong>Vendor documents</strong> (ring): the vendor’s public material describes it. We have not tested it. The cell text says what the vendor documents.</li>
      <li><span class="cell-mark mark-nv" aria-hidden="true"></span><strong>Not verified</strong>: the public material we checked did not confirm it. The feature may still exist.</li>
    </ul>
    <p class="small">Research date: 19 October 2026. Products and prices change. This table compares features, not marker quality. No fair, public test between these systems exists. See the <a href="benchmark.html">benchmark</a> for what we have measured.</p>
  </div>
</div>

${section('table', 'Features', html`
<div class="table-wrap compare-wrap" tabindex="0" role="region" aria-label="Feature comparison table, scrolls sideways">
<table class="compare-table">
  <caption class="sr-only">Features of BETA Cutting Room and six commercial cutting-room systems</caption>
  <thead><tr><th scope="col" class="feature-col">Feature</th>${columns.map((c) => html`<th scope="col"${c.key === 'beta' ? html` class="beta-col"` : ''}><span class="col-name">${c.name}</span><span class="col-products">${c.products}</span></th>`)}</tr></thead>
  ${groups.map((g) => html`<tbody><tr class="group-row"><th scope="rowgroup" colspan="${columns.length + 1}">${g}</th></tr>
  ${rows.filter((r) => r.group === g).map((r) => html`<tr><th scope="row" class="feature-col">${r.feature}</th>${columns.map((c) => cellHtml(r.cells[c.key]))}</tr>`)}</tbody>`)}
</table>
</div>
<p class="small muted">On a small screen, scroll the table sideways. The first column stays in place.</p>`)}

${section('notes', 'Notes', html`
<div class="prose">
  <ul>
    <li><strong>Vendor claims are not benchmarks.</strong> Tukatech testimonials claim 1% to 3% savings, Audaces “up to 3%”, BOKE “1–3%” and Lectra “up to 30% waste reduction” for motif-heavy lines. These are the vendors’ own tests on their own data.</li>
    <li><strong>“AI” is the vendors’ word.</strong> Their public pages do not describe the method, so we cannot compare it.</li>
    <li><strong>Price.</strong> Most vendors quote per factory. Quotes usually exclude tax, hardware, training and support. BETA is free; support and hardware still cost money.</li>
    <li><strong>Other systems.</strong> Gemini, Style3D/ASSYST, CLO and ETSYSTEM are covered in our research notes but not in this table.</li>
    <li>Found an error? Tell us and we will correct it. Vendors are welcome to send documentation.</li>
  </ul>
</div>`, { cls: 'section-alt' })}

${section('sources', 'Sources', html`
<ol class="source-list">
  ${sources.map(([id, label, url]) => html`<li id="${`src-${id}`}"><span class="src-id">[${id}]</span> ${url ? ext(url, label) : html`${label} (<code>marker/docs/PLAN.md</code>)`}</li>`)}
</ol>
<p class="small muted">Full research notes with every source: <code>marker/docs/V04-RESEARCH.md</code>. Links were checked on 19 October 2026.</p>`)}
`;
}
