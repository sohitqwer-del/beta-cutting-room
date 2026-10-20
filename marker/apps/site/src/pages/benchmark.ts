import { fmt, html, signed, type Raw } from '../html.ts';
import { ext, pageHead, section } from '../layout.ts';
import { barChart, divergingChart, dotChart } from '../charts.ts';
import { describeStratum, headlineGroup, type Group, type Run } from '../benchmark.ts';
import { repoLink } from '../config.ts';
import type { BuildContext } from './context.ts';

/** Link to a repository document, or the path as code when no public repository is set. */
function docRef(path: string, text?: string): Raw {
  if (/^https?:\/\//.test(path)) return ext(path, text ?? path);
  const href = repoLink(path);
  return href ? ext(href, text ?? path) : html`${text ? html`${text} ` : ''}<code>${path}</code>`;
}

function statusBadge(run: Run): Raw {
  if (run.status === 'final') return html`<span class="badge badge-final">Final</span>`;
  if (run.status === 'provisional') return html`<span class="badge badge-provisional">Provisional${run.progress ? ` · ${run.progress.completed} of ${run.progress.planned} tasks` : ''}</span>`;
  return html`<span class="badge badge-sample">Sample data</span>`;
}

function diffText(v: number | null): string { return v === null ? 'not available' : `${signed(v)} pp`; }

/** Plain words for a mean difference against a reference. */
function diffWords(v: number | null, refShort: string): string {
  if (v === null) return '';
  if (Math.abs(v) < 0.1) return `about the same as ${refShort}`;
  return v < 0 ? `${fmt(-v)} points behind ${refShort}` : `${fmt(v)} points ahead of ${refShort}`;
}

function wtl(g: Group): Raw {
  if (g.win === null || g.tie === null || g.loss === null) return html`<span class="muted">Win/tie/loss not available yet</span>`;
  return html`Wins ${g.win} · ties ${g.tie} · losses ${g.loss}${g.too_long_losses ? html` <span class="muted">(plus ${g.too_long_losses} too-long losses)</span>` : ''}`;
}

function groupTable(caption: string, rows: Group[], labeller: (l: string) => string = (l) => l): Raw {
  const hasEff = rows.some((r) => r.mean_beta_eff_pct !== null);
  const hasRef = rows.some((r) => r.mean_diff_pp !== null);
  return html`<div class="table-wrap" tabindex="0" role="region" aria-label="${caption}"><table class="data-table">
<caption>${caption}</caption>
<thead><tr><th scope="col">Group</th><th scope="col">Valid / attempts</th>${hasEff ? html`<th scope="col">BETA mean</th><th scope="col">Reference mean</th>` : ''}${hasRef ? html`<th scope="col">Mean diff.</th><th scope="col">Median diff.</th><th scope="col">Win / tie / loss</th><th scope="col">Mean length saving</th>` : ''}</tr></thead>
<tbody>${rows.map((g) => html`<tr><th scope="row">${labeller(g.label)}</th><td>${g.valid} / ${g.attempts}${g.other_failures ? html` <span class="muted">(${g.other_failures} failed)</span>` : ''}</td>${hasEff ? html`<td>${g.mean_beta_eff_pct === null ? '–' : `${fmt(g.mean_beta_eff_pct)}%`}</td><td>${g.mean_reference_eff_pct === null ? '–' : `${fmt(g.mean_reference_eff_pct)}%`}</td>` : ''}${hasRef ? html`<td>${g.mean_diff_pp === null ? '–' : `${signed(g.mean_diff_pp)} pp`}</td><td>${g.median_diff_pp === null ? '–' : `${signed(g.median_diff_pp)} pp`}</td><td>${g.win === null ? '–' : `${g.win} / ${g.tie} / ${g.loss}${g.too_long_losses ? ` (+${g.too_long_losses} too long)` : ''}`}</td><td>${g.mean_length_saving_pct === null ? '–' : `${signed(g.mean_length_saving_pct)}%`}</td>` : ''}</tr>`)}</tbody>
</table></div>`;
}

function conditions(run: Run): Raw {
  const c = run.conditions;
  const rows: [string, string | null][] = [
    ['Time budget', c.budget_label],
    ['Machine', c.machine],
    ['Parallel work', c.jobs && c.workers_per_task ? `${c.jobs} tasks at a time, ${c.workers_per_task} search threads each` : null],
    ['Seeds', c.seeds.length ? c.seeds.join(', ') : null],
    ['Background load', c.background_load],
    ['BETA version', run.software.beta_version + (run.software.engine_revision ? ` (revision ${run.software.engine_revision})` : '')],
    ['Engine', run.software.engine],
    ['Date', run.date],
    ['Notes', c.notes],
  ];
  return html`<dl class="facts">${rows.filter((r) => r[1]).map(([k, v]) => html`<div><dt>${k}</dt><dd>${v}</dd></div>`)}</dl>`;
}

function reproduceBlock(run: Run): Raw {
  return html`<details class="disclosure"><summary>How to repeat this run</summary>
<p>Run from the <code>marker/</code> folder of the source code. Full steps: ${docRef(run.reproduce.doc)}.</p>
<pre><code>${run.reproduce.commands.join('\n')}</code></pre>
${run.sources.length ? html`<p>Reports: ${run.sources.map((s, i) => html`${i ? ' · ' : ''}${docRef(s.url, s.label)}`)}</p>` : ''}
</details>`;
}

function runCard(run: Run): Raw {
  const g = headlineGroup(run);
  const hasRef = run.reference.type !== 'none';
  const refShort = run.kind === 'lectra' ? 'Lectra' : run.kind === 'academic' ? 'the paper' : 'the reference';
  let chart: Raw | string = '';
  if (run.kind === 'academic') {
    const rows = run.groups.filter((x) => x.mean_beta_eff_pct !== null && x.mean_reference_eff_pct !== null)
      .map((x) => ({ label: x.label.charAt(0).toUpperCase() + x.label.slice(1), beta: x.mean_beta_eff_pct!, reference: x.mean_reference_eff_pct! }));
    if (rows.length) chart = dotChart({ title: 'Efficiency by instance', desc: 'Mean of three 60-second BETA runs against the paper’s mean of 100 runs at 1200 seconds. Further right is better.', rows, betaLabel: 'BETA, 60 s', referenceLabel: 'Paper, 1200 s' });
  } else if (run.items.length && run.items.every((i) => i.beta_eff_pct !== null)) {
    chart = barChart({ title: 'Marker efficiency by garment', desc: 'Share of the marker area covered by pieces. Higher is better.', rows: run.items.map((i) => ({ label: i.id.charAt(0).toUpperCase() + i.id.slice(1), value: i.beta_eff_pct! })) });
  } else if (run.strata.length && hasRef) {
    chart = divergingChart({ title: 'Mean difference by marker size and Lectra efficiency', desc: `BETA minus Lectra, in percentage points. ${run.strata[0]!.attempts} tasks in each row. Bars to the left mean BETA’s marker was longer.`, rows: run.strata.map((s) => ({ label: describeStratum(s.label), value: s.mean_diff_pp })) });
  }
  return html`<article class="run-card${run.status !== 'final' ? ` run-${run.status}` : ''}" id="run-${run.id}-${run.status}">
  <header class="run-head">
    <h3>${run.title}</h3>
    <div class="run-badges">${statusBadge(run)}</div>
  </header>
  ${run.status_note ? html`<p class="status-note">${run.status === 'provisional' ? html`<strong>Provisional:</strong> ` : ''}${run.status_note}</p>` : ''}
  ${run.progress && run.status === 'provisional' ? html`<div class="progress"><progress max="${run.progress.planned}" value="${run.progress.completed}">${run.progress.completed} of ${run.progress.planned}</progress><span>${run.progress.completed} of ${run.progress.planned} tasks finished</span></div>` : ''}
  ${run.summary_text ? html`<p>${run.summary_text}</p>` : ''}
  <div class="stat-row">
    ${hasRef ? html`<div class="stat"><span class="stat-value${(g.mean_diff_pp ?? 0) < 0 ? ' stat-neg' : ''}">${diffText(g.mean_diff_pp)}</span><span class="stat-label">Mean difference. ${g.label}. On average BETA is ${diffWords(g.mean_diff_pp, refShort)}.${g.median_diff_pp !== null ? ` Median task: ${signed(g.median_diff_pp)} pp.` : ''}</span></div>` : ''}
    ${g.mean_beta_eff_pct !== null && !hasRef ? html`<div class="stat"><span class="stat-value">${fmt(g.mean_beta_eff_pct)}%</span><span class="stat-label">Mean BETA efficiency. ${g.label}.</span></div>` : ''}
    ${hasRef && g.mean_length_saving_pct !== null ? html`<div class="stat"><span class="stat-value">${signed(g.mean_length_saving_pct)}%</span><span class="stat-label">Mean marker length saving. Negative means BETA’s marker was longer.</span></div>` : ''}
    <div class="stat"><span class="stat-value">${g.valid} / ${g.attempts}</span><span class="stat-label">BETA layouts that passed the independent validator</span></div>
  </div>
  ${hasRef ? html`<p class="wtl">${wtl(g)}. A tie is within 0.1 points.</p>` : ''}
  ${chart}
  ${run.correlation || run.quantiles_pp ? html`<p class="small">${run.correlation ? html`Correlation with the reference over ${run.correlation.pairs} pairs: Pearson ${fmt(run.correlation.pearson, 3)}, Spearman ${fmt(run.correlation.spearman, 3)}. ` : ''}${run.quantiles_pp ? html`Per-task differences range from ${signed(run.quantiles_pp.min)} to ${signed(run.quantiles_pp.max)} pp; 90% of tasks fall between ${signed(run.quantiles_pp.p05)} and ${signed(run.quantiles_pp.p95)} pp.` : ''}</p>` : ''}
  <details class="disclosure"><summary>Conditions and assumptions</summary>
    ${conditions(run)}
    <p><strong>Compared with:</strong> ${run.reference.name}${run.reference.licence ? ` (${run.reference.licence})` : ''}.</p>
    ${run.assumptions.length ? html`<ul>${run.assumptions.map((a) => html`<li>${a}</li>`)}</ul>` : ''}
  </details>
  <details class="disclosure"><summary>All numbers (table)</summary>
    ${groupTable(`${run.title}: groups`, run.groups)}
    ${run.strata.length ? groupTable(`${run.title}: by stratum`, run.strata, describeStratum) : ''}
    ${run.items.length ? html`<div class="table-wrap" tabindex="0" role="region" aria-label="${`${run.title}: per task`}"><table class="data-table"><caption>Per task</caption><thead><tr><th scope="col">Task</th><th scope="col">Valid</th><th scope="col">BETA efficiency</th><th scope="col">Run time</th></tr></thead><tbody>${run.items.map((i) => html`<tr><th scope="row">${i.id}</th><td>${i.valid ? 'Yes' : `No${i.failure ? ` (${i.failure})` : ''}`}</td><td>${i.beta_eff_pct === null ? '–' : `${fmt(i.beta_eff_pct)}%`}</td><td>${i.runtime_s === null ? '–' : `${fmt(i.runtime_s, 1)} s`}</td></tr>`)}</tbody></table></div>` : ''}
    ${run.task_ids.length ? html`<p class="small">Task IDs (${run.task_ids.length}): <span class="mono">${run.task_ids.join(', ')}</span></p>` : ''}
    <p class="small muted">Source file: <code>data/benchmark/${run.file}</code></p>
  </details>
  ${reproduceBlock(run)}
</article>`;
}

/** Median and win/tie/loss for a summary tile, so a mean is never read alone. */
function tileDetail(g: Group): string {
  const parts: string[] = [];
  if (g.median_diff_pp !== null) parts.push(`median ${signed(g.median_diff_pp)} pp`);
  if (g.win !== null && g.tie !== null && g.loss !== null) parts.push(`${g.win} wins, ${g.tie} ties, ${g.loss} losses`);
  return parts.join(' · ');
}

/** "won 25, tied 3 and lost 32 of 60 tasks", or an empty string. */
function wtlWords(g: Group): string {
  if (g.win === null || g.tie === null || g.loss === null) return '';
  return `won ${g.win}, tied ${g.tie} and lost ${g.loss + g.too_long_losses} of ${g.attempts} tasks`;
}

/**
 * Plain-English summary written from the data, whatever its sign. A positive mean is always
 * shown with its median and win/tie/loss, and never as a general win.
 */
function lectraSummary(l60: Run | undefined, lm: Run | undefined): Raw {
  const sentences: Raw[] = [];
  const m = lm ? headlineGroup(lm) : undefined;
  if (m && m.mean_diff_pp !== null) {
    const median = m.median_diff_pp !== null ? `the median task was ${signed(m.median_diff_pp)} pp` : '';
    const wtl = wtlWords(m);
    const detail = [median, wtl ? `BETA ${wtl}` : ''].filter(Boolean).join(', and ');
    if (m.mean_diff_pp > 0.1) {
      sentences.push(html`<strong>No general win over Lectra.</strong> With Lectra’s recorded time per task, BETA’s mean was ${signed(m.mean_diff_pp)} pp${detail ? `, but ${detail}` : ''}.`);
    } else if (m.mean_diff_pp < -0.1) {
      sentences.push(html`<strong>BETA does not beat Lectra on this data.</strong> With Lectra’s recorded time per task, BETA’s markers were ${fmt(-m.mean_diff_pp)} efficiency points lower on average${detail ? `; ${detail}` : ''}.`);
    } else {
      sentences.push(html`<strong>About level with Lectra on average, not better.</strong> With Lectra’s recorded time per task, BETA’s mean was ${signed(m.mean_diff_pp)} pp${detail ? `; ${detail}` : ''}.`);
    }
  }
  const s = l60 ? headlineGroup(l60) : undefined;
  if (s && s.mean_diff_pp !== null) {
    const wtl = wtlWords(s);
    sentences.push(html`With only 60 seconds per task, BETA’s mean was ${signed(s.mean_diff_pp)} pp${wtl ? ` (BETA ${wtl})` : ''}.`);
  }
  sentences.push(html`Equal seconds do not mean equal computing power: Lectra’s engine version and hardware are unknown, and these are sampled tasks, not all Lectra customers.`);
  const size = sizeSentence(l60);
  if (size) sentences.push(html`${size}`);
  return html`<p>${sentences.map((x, i) => html`${i ? ' ' : ''}${x}`)}</p>`;
}

/** Picks the best run with an id for the summary tiles: final, then sample, then provisional. */
export function pick(runs: Run[], id: string): Run | undefined {
  const rank = { final: 0, sample: 1, provisional: 2 } as const;
  return runs.filter((r) => r.id === id).sort((a, b) => rank[a.status] - rank[b.status])[0];
}

function sizeSentence(run: Run | undefined): string {
  if (!run || !run.strata.length) return '';
  const band = (prefix: string) => run.strata.filter((s) => s.label.startsWith(`pieces:${prefix}`)).map((s) => s.mean_diff_pp).filter((v): v is number => v !== null);
  const small = band('1-25'); const large = [...band('76-150'), ...band('151+')];
  if (!small.length || !large.length) return '';
  const mean = (xs: number[]) => xs.reduce((x, y) => x + y, 0) / xs.length;
  const worse = mean(large) < mean(small) ? ' Large markers are where BETA needs the most work.' : '';
  return `At 60 seconds, small markers (up to 25 pieces) range from ${signed(Math.min(...small))} to ${signed(Math.max(...small))} pp across the efficiency groups. Markers with more than 75 pieces range from ${signed(Math.min(...large))} to ${signed(Math.max(...large))} pp.${worse}`;
}

export function benchmarkPage(ctx: BuildContext): Raw {
  const runs = ctx.runs;
  const l60 = pick(runs, 'lectra-60s'); const lm = pick(runs, 'lectra-matched'); const ac = pick(runs, 'academic-60s');
  const nonFinal = runs.some((r) => r.status !== 'final');
  const tiles: Raw[] = [];
  if (lm) tiles.push(html`<div class="tile"><span class="tile-value">${diffText(headlineGroup(lm).mean_diff_pp)}</span><span class="tile-label">Mean vs Lectra with Lectra’s own time per task (${headlineGroup(lm).valid} tasks)</span><span class="tile-sub">${tileDetail(headlineGroup(lm))}</span>${lm.status !== 'final' ? statusBadge(lm) : ''}</div>`);
  if (l60) tiles.push(html`<div class="tile"><span class="tile-value">${diffText(headlineGroup(l60).mean_diff_pp)}</span><span class="tile-label">Mean vs Lectra at 60 seconds per task (${headlineGroup(l60).valid} tasks)</span><span class="tile-sub">${tileDetail(headlineGroup(l60))}</span>${l60.status !== 'final' ? statusBadge(l60) : ''}</div>`);
  if (ac) tiles.push(html`<div class="tile"><span class="tile-value">${diffText(headlineGroup(ac).mean_diff_pp)}</span><span class="tile-label">vs the sparrow paper’s 20-minute means, with 60 seconds</span>${ac.status !== 'final' ? statusBadge(ac) : ''}</div>`);
  const lectraRuns = [l60, lm].filter((r): r is Run => r !== undefined);
  const valid = lectraRuns.reduce((s, r) => s + headlineGroup(r).valid, 0); const attempts = lectraRuns.reduce((s, r) => s + headlineGroup(r).attempts, 0);
  if (attempts) tiles.push(html`<div class="tile"><span class="tile-value">${valid} / ${attempts}</span><span class="tile-label">BETA layouts on Lectra tasks that passed the independent validator (${lectraRuns.map((r) => `${headlineGroup(r).attempts} at ${r.conditions.budget_s === null ? 'Lectra’s time' : `${r.conditions.budget_s} s`}`).join(', ')})</span></div>`);

  const kindSection = (kind: Run['kind']) => runs.filter((r) => r.kind === kind).map(runCard);

  return html`
${pageHead('Benchmark', 'We publish how BETA markers compare with a commercial engine’s recorded results and with published research. Every number comes with its method, its assumptions and the commands to repeat it.', 'Honest, repeatable results')}

${nonFinal ? html`<div class="wrap"><div class="callout callout-warn" role="note"><h2 class="h-small">Data status</h2><p>Results marked <strong>Sample data</strong> are copied from our documented benchmark reports while the public export is prepared. Results marked <strong>Provisional</strong> come from a run that has not finished; they will change. Each result shows its label.</p></div></div>` : ''}

${section('summary', 'In short', html`
<div class="tile-row">${tiles}</div>
<div class="prose">
  ${lectraSummary(l60, lm)}
  <p>A <strong>percentage point (pp)</strong> is the difference between two efficiencies: 85% against 86% is 1 pp. At 85% efficiency, 1 pp is about 1.2% of marker length.</p>
  <p>We have <strong>no matched tests against Tukatech, Gerber or Optitex</strong>. Their markers are not in any public dataset. Send us your markers and we will compare on your styles.</p>
</div>`)}

${section('lectra', 'Lectra dataset: real client markers', html`
<div class="prose"><p>The Lectra nesting dataset has 100,000 real nesting tasks from about 100 fashion customers. For each task it records the efficiency that <strong>Lectra’s own nesting engine</strong> reached. It is the only public source of commercial-engine results on real garment markers. We take a fixed, balanced sample, run BETA on the same pieces and fabric width, and compare.</p></div>
${kindSection('lectra')}`, { cls: 'section-alt' })}

${section('academic', 'Academic instances', html`
<div class="prose"><p>Research papers use a small set of standard nesting instances, including shirts, trousers and swimwear. The best open-source result is the sparrow paper. BETA uses sparrow as its search engine, so this test checks that BETA’s adapter does not lose quality, at a much shorter time.</p></div>
${kindSection('academic')}`)}

${section('garments', 'Whole garments (FreeSewing)', html`
<div class="prose"><p>FreeSewing publishes open-source garment patterns. We use them to test complete garments with real garment rules (grain, pairs, size ratio). There is no commercial result for these exact jobs, so the numbers show BETA alone.</p></div>
${kindSection('freesewing')}
${kindSection('other')}
<div class="callout callout-plain">
  <h3>One factory style (provisional, private)</h3>
  <p>On one real knitwear style from an Indian factory, BETA’s marker was 4.6% to 6.7% shorter than the marker from the factory’s Gemini CAD. This is one style. The data is private, so you cannot repeat the test, and it does not show that BETA beats Gemini in general. We need more factory comparisons: <a href="index.html#samples">send us your markers</a>.</p>
</div>`, { cls: 'section-alt' })}

${section('method', 'Method', html`
<div class="prose">
  <h3>What we measure</h3>
  <ul>
    <li><strong>Marker efficiency</strong> = total piece area ÷ (fabric width × marker length). Higher is better.</li>
    <li><strong>Difference</strong> = BETA efficiency minus the reference efficiency, in percentage points. A tie is within 0.1 points.</li>
    <li><strong>Length saving</strong> = 100 × (1 − reference efficiency ÷ BETA efficiency). Positive means BETA’s marker is shorter for the same pieces.</li>
    <li>Means and medians use valid layouts only. A layout that is too long for a fixed sheet counts as a loss.</li>
  </ul>
  <h3>Every BETA marker is checked</h3>
  <p>An independent validator checks every BETA layout: all pieces present, no overlaps, grain and allowed rotations, fabric width and gaps. A layout that fails gets no score.</p>
  <h3>Fair rules: Tier A and Tier B</h3>
  <p>Each Lectra task says which rotations and mirror images each piece may use. <strong>Tier A</strong> tasks give BETA exactly the same choices. <strong>Tier B</strong> tasks give BETA the same choices or fewer, never more. The headline uses Tier B, which includes Tier A. Where BETA has fewer choices, that is a handicap for BETA, not for Lectra. Tasks with constraint types we cannot apply exactly are left out (3,253 of 100,000).</p>
  <h3>The sheet-type assumption</h3>
  <p>Each task has a <code>sheet_type</code> code. There is no public codebook. We assume <strong>code 0 means an ordinary open strip of fabric</strong> (one layer, the stated width). Code 0 covers 99,403 of 100,000 tasks. The strong correlation between BETA and Lectra results supports this, but does not prove it. Codes 2 and 3 are tested separately and never enter the headline.</p>
  <h3>Sample and time</h3>
  <p>The 300-task sample uses seed 7030786 and has 25 tasks in each of 12 groups (4 piece-count bands × 3 Lectra-efficiency bands). The 60-task subset for the matched-time run was chosen before any results were seen. Lectra’s engine version, hardware and number of workers are unknown, and its recorded times vary (244 to 608 seconds in our subset). Equal seconds do not mean equal computing power. Our machines were shared with other work.</p>
  <h3>What we do not claim</h3>
  <ul>
    <li>We do not claim BETA beats Lectra, Tukatech, Gerber, Optitex or any other commercial system.</li>
    <li>The balanced sample describes those tasks. It is not an estimate for all Lectra customers.</li>
    <li>Vendor fabric-saving claims (often 1–5%) are their own tests on their own data. Nobody has published a fair test between these tools.</li>
  </ul>
</div>`)}

${section('reproduce', 'Repeat the benchmark yourself', html`
<div class="prose">
  <ol>
    <li>Get the source code and install Rust. Python is needed once, to convert the Lectra dataset files.</li>
    <li>Choose a data folder outside the source code. Set <code>BETA_DATA_DIR</code> and <code>CARGO_TARGET_DIR</code> to folders on a disk with space.</li>
    <li>Convert the dataset. The converter checks the published checksums and reads the files with a restricted loader:
<pre><code>python -I bench/tools/lectra_convert.py download
python -I bench/tools/lectra_convert.py inspect
python -I bench/tools/lectra_convert.py self-test
python -I bench/tools/lectra_convert.py convert --sample-size 300 --seed 7030786</code></pre></li>
    <li>Run the tests, then the benchmark commands shown under each result (“How to repeat this run”). Use a new output folder for each run. Reduce <code>--jobs</code> on a smaller computer.</li>
    <li>Each run writes <code>run.json</code>, <code>results.csv</code> and <code>summary.md</code>. The website reads summary files in the format described in ${docRef('marker/apps/site/data/benchmark/SCHEMA.md')}.</li>
  </ol>
  <p>Full instructions: ${docRef('marker/docs/benchmarks.md')}. No pattern geometry is published on this site; the files contain totals and task IDs only.</p>
</div>`, { cls: 'section-alt' })}

${section('attribution', 'Data and attribution', html`
<ul class="ref-list">
  <li><strong>Lectra nesting dataset.</strong> Lallier, Vézard, Pinaud and Blin (2022). <em>Nesting Tasks Dataset for 2D-Nesting Efficiency Estimation</em>, version 1.1. ${ext('https://doi.org/10.5281/zenodo.7030786', 'Zenodo 7030786')}. Licence: ${ext('https://creativecommons.org/licenses/by/4.0/', 'CC BY 4.0')}. Associated paper: ${ext('https://doi.org/10.1007/s10845-023-02084-6', 'doi:10.1007/s10845-023-02084-6')}. We converted coordinates to millimetres and may restrict rotations (see Tier B). No dataset geometry is published here.</li>
  <li><strong>Academic reference values.</strong> Gardeyn, Vanden Berghe and Wauters, <em>An open-source heuristic to reboot 2D nesting research</em>, ${ext('https://arxiv.org/abs/2509.13329', 'arXiv:2509.13329')}, Tables 2–3. Instance files from sparrow v0.3.0 (MIT).</li>
  <li><strong>FreeSewing patterns.</strong> ${ext('https://freesewing.eu/', 'FreeSewing')} (MIT licence).</li>
</ul>`)}
`;
}

