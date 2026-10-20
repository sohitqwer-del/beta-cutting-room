/** QA-004: Seamly2D import warnings live in `meta.seamly_import_warnings` (a list of strings written by the Rust importer).
 * The UI reads them from the document on every render, so they survive edits, saves, reloads and the hand-off to Marker. */
type Meta = Record<string, unknown> | null | undefined;
export const SEAMLY_WARNINGS = 'seamly_import_warnings';

export function seamlyWarnings(meta: Meta): string[] {
  const value = meta?.[SEAMLY_WARNINGS];
  return Array.isArray(value) ? value.filter((w): w is string => typeof w === 'string' && w.trim() !== '') : [];
}

/** Copy the pattern's open import warnings onto a marker job made from it. */
export function carryWarnings<T extends { meta?: Record<string, unknown> | null }>(job: T, patternMeta: Meta, patternName: string): T {
  const warnings = seamlyWarnings(patternMeta);
  if (!warnings.length) return job;
  return { ...job, meta: { ...job.meta, [SEAMLY_WARNINGS]: warnings, seamly_source_pattern: patternName } };
}

export function warningsHtml(warnings: string[], intro: string, headingId: string, esc: (v: unknown) => string): string {
  if (!warnings.length) return '';
  return `<h2 id="${headingId}">Seamly2D import warnings (${warnings.length})</h2><p>${esc(intro)}</p><ul>${warnings.map(w => `<li>${esc(w)}</li>`).join('')}</ul>`;
}
