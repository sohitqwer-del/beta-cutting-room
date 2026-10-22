/** Engine notes in two groups. Rust adds both kinds to `warnings`:
 * - notes a cutting room must read (fold edge, pairs, shortfall, defects), shown as they are;
 * - search log lines (passes, seeds, timings, retained incumbents), kept under a closed "Technical details". */

/** Search and timing log lines from the nester, matcher, cluster and planner. Anything else stays visible. */
const TECHNICAL = [
  /^Bounded obstacle packing:/, /^Portfolio:/, /^Search schedule:/, /^Shade assignment search:/,
  /^\d+ portfolio runs failed/, /^Rejected \d+ invalid search candidates/, /^Search cancelled; best/,
  /^Browser improvement failed/, /^Matching round stopped/, /^validation_repair:/,
  /^Two-layer surplus for /, /^Fusing block: /,
  /^Expanded \d+ compact pairs/, /^Cluster construction used/, /^Cluster strategy used plain/, /^Exact copy aggregation:/,
  /^Large pieces first, then/, /^Small pieces removed and reinserted/, /candidate failed; plain fallback:/,
  /^Retained valid incumbent after/, /^Raced plain, cluster/, /^Race candidates:/,
  /^Calibration for .* kept the short marker/, /^Final nest for .* kept its validated incumbent/, /^Final ratio kept its validated incumbent/,
];

export function isTechnicalNote(note: string): boolean {
  return TECHNICAL.some(re => re.test(note));
}

/** Split engine notes; each group keeps its order and drops repeats. */
export function splitNotes(notes: readonly string[] | undefined): { shown: string[]; technical: string[] } {
  const shown: string[] = [], technical: string[] = [];
  for (const note of new Set(notes ?? [])) (isTechnicalNote(note) ? technical : shown).push(note);
  return { shown, technical };
}

export const TECHNICAL_DETAILS = 'Technical details';

/** A closed disclosure with the search log lines, or '' when there are none. */
export function technicalDetailsHtml(technical: readonly string[], esc: (v: unknown) => string, summary = TECHNICAL_DETAILS): string {
  if (!technical.length) return '';
  return `<details class="advanced tech-notes"><summary>${esc(summary)} (${technical.length})</summary><ul>${technical.map(n => `<li>${esc(n)}</li>`).join('')}</ul></details>`;
}
