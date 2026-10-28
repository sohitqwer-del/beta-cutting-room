// Questions people ask first. The top eight follow the shared FAQ wording agreed with the docs (marker FAQ.md);
// keep the meaning when you edit them. The Help page adds the practical ones after these.
import { fmt, html, type Raw } from './html.ts';
import { repoLink, site } from './config.ts';
import { headlineGroup, type Run } from './benchmark.ts';

const link = (path: string, text: string): Raw => { const href = repoLink(path); return href ? html`<a href="${href}" rel="noopener noreferrer external">${text}</a>` : html`${text}`; };

/** The top eight. The nesting answer is written from the benchmark data, so it always matches the numbers shown. */
export function topFaq(runs: Run[]): [string, Raw][] {
  return [
  ['Is it free?', html`Yes. The code is Apache-2.0 and there is no licence fee. Your own computers, hosting, printing and any training or support you choose to buy still cost money.`],
  ['Is it open source?', html`Yes, all of it, under Apache-2.0. Third-party code and data keep their own licences; see ${link('marker/THIRD-PARTY-LICENSES.md', 'the notices')}.`],
  ['Who built it?', html`Sohit and contributors, with the help of modern AI coding tools. It is a young project: it needs review by real pattern masters, cutting masters and factory owners.`],
  ['Can I trust the results?', html`An independent validator and many tests check every marker, but they do not replace your own checks. Before production, check the scale, the fit, the ply and layer counts and the files on your plotter or cutter.`],
  ['Can it replace my commercial CAD?', html`Not proven. We make no general claim that BETA is better. On the public Lectra benchmark the results are mixed: see <a href="#benchmark">the numbers</a>.`],
  ['What happens to my data?', html`It stays on your device: in this browser, in the desktop app, or on a factory server you choose. There is no account and no analytics. If you attach files to a public GitHub issue, those files become public.`],
  ['Does it work offline?', html`Yes. The desktop app is fully local. The browser app works offline after its first complete visit (it keeps an offline copy). Save backup files of your work either way.`],
  ['How good is the nesting?', nestingAnswer(runs)],
  ];
}

/** "0.73 points less efficient" / "0.42 points more efficient". */
const points = (pp: number) => `${fmt(Math.abs(pp))} points ${pp < 0 ? 'less' : 'more'} efficient`;
function sentence(run: Run | undefined, what: string): string {
  if (!run) return '';
  const g = headlineGroup(run);
  if (g.mean_diff_pp === null || g.median_diff_pp === null) return '';
  const losses = (g.loss ?? 0) + g.too_long_losses;
  return `${what} (${g.attempts} tasks, measured ${run.date}): ${points(g.mean_diff_pp)} on average, median ${points(g.median_diff_pp)}; ${g.win} wins, ${g.tie} ties, ${losses} losses.`;
}
export function nestingAnswer(runs: Run[]): Raw {
  const by = (id: string) => runs.filter((r) => r.id === id && r.status === 'final')[0];
  const sixty = by('lectra-60s-08') ?? by('lectra-60s'), matched = by('lectra-matched');
  return html`Close to a commercial engine on some jobs and behind on others. On public Lectra tasks, against Lectra's recorded results: ${sentence(sixty, 'with 60 seconds per task')} ${sentence(matched, "Historical 20 October, with Lectra's own time per task")} See <a href="benchmark.html">the benchmark</a> for the method.`;
}

export function faqList(items: [string, Raw][]): Raw {
  return html`<div class="faq">${items.map(([q, a]) => html`<details class="disclosure"><summary>${q}</summary><p>${a}</p></details>`)}</div>`;
}

export const contactFaq = (): Raw => html`Write to ${site.contactEmail ? html`<a href="${`mailto:${site.contactEmail}`}">${site.contactEmail}</a>` : 'the team'} or open an issue on GitHub.`;
