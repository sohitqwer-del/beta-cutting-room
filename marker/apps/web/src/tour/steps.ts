/** "See it in 60 seconds": a guided sample on the real engine (Pattern → Grade → Lay → live Marker → Cut plan).
 * Pure step model and caption text. The screen part is in ui.ts; it drives the Style screen through StyleControl. */
import type { StyleFacts } from '../style/ui';
import { longLength, shortLength, type AppUnit } from '../units-pref';
import { t, type TourKey } from './i18n';

export type TourStep = 'loading' | 'pattern' | 'grade' | 'lay' | 'marker' | 'cut' | 'end';
/** Steps with a number in the card ("Step 2 of 5"). Loading and the end card have none. */
export const NUMBERED: TourStep[] = ['pattern', 'grade', 'lay', 'marker', 'cut'];
export const ORDER: TourStep[] = ['loading', ...NUMBERED, 'end'];

/** The sample: the engine's T-shirt block in four sizes on an open 60-inch lay, one garment of each size per marker. */
export const SAMPLE = {
  block: 't_shirt' as const,
  sizes: ['S', 'M', 'L', 'XL'],
  lay: { layout: 'open' as const, face: 'up' as const, nap: 'none' as const, widthMm: 1524, plies: 40, endAllowanceMm: 20 },
  /** Search time for the live marker. Short enough for a one-minute tour, long enough to see it improve. */
  searchS: 15,
};

export function nextStep(step: TourStep): TourStep | null { return ORDER[ORDER.indexOf(step) + 1] ?? null; }
export function stepNumber(step: TourStep): number { return NUMBERED.indexOf(step) + 1; }

export interface Card { title: string; text: string; next: string; number: number; total: number }
/** What the tour card says on each step, filled with the real numbers of the sample. */
export function card(step: TourStep, facts: StyleFacts | null, unit: AppUnit): Card {
  const n = stepNumber(step), total = NUMBERED.length;
  const L = (mm: number) => { const v = longLength(mm, unit, 2); return `${v.value} ${v.unit}`; };
  const W = (mm: number) => shortLength(mm, unit);
  const pct = (x: number | null) => (x == null ? '–' : `${(x * 100).toFixed(1)}%`);
  const f = facts;
  const text = (key: TourKey, values: Record<string, string | number> = {}) => t(key, values);
  switch (step) {
    case 'loading': return { title: text('loading_title'), text: text('loading_text'), next: text('skip'), number: 0, total };
    case 'pattern': return { title: text('pattern_title'), text: text('pattern_text', { n: f?.pieces ?? 0, names: listOf((f?.pieceNames ?? []).map(name => name.replace(/\s*\(.*\)\s*$/, '')) ) }), next: text('next'), number: n, total };
    case 'grade': return { title: text('grade_title'), text: text(f && f.seamChecks ? 'grade_text' : 'grade_text_plain', { sizes: listOf(f?.sizes ?? []), base: f?.baseSize ?? '', ok: f?.seamChecksOk ?? 0, n: f?.seamChecks ?? 0 }), next: text('next'), number: n, total };
    case 'lay': return { title: text('lay_title'), text: text('lay_text', { width: f ? W(f.widthMm) : '', plies: f?.plies ?? 0, g: f?.garmentsPerMarker ?? 0 }), next: text('make_marker'), number: n, total };
    case 'marker': return f?.lengthMm != null
      ? { title: text('marker_done_title'), text: text(f.checksOk === false ? 'marker_done_text_check' : 'marker_done_text', { g: f.garmentsPerMarker, length: L(f.lengthMm), e: pct(f.efficiency) }), next: text('next'), number: n, total }
      : { title: text('marker_title'), text: text('marker_text'), next: text('use_best'), number: n, total };
    case 'cut': return { title: text('cut_title'), text: text(f && f.passes !== f.plies ? 'cut_text' : 'cut_text_open', { plies: f?.plies ?? 0, passes: f?.passes ?? 0, fabric: f?.fabricMm != null ? L(f.fabricMm) : '–', garments: f?.garmentsInLay ?? 0 }), next: text('finish'), number: n, total };
    case 'end': return { title: text('end_title'), text: text('end_text'), next: text('close'), number: 0, total };
  }
}

/** Live line while the marker search runs: best length so far, efficiency and time. */
export function liveLine(p: { elapsedS: number; budgetS: number; bestLengthMm: number | null; efficiency: number | null }, unit: AppUnit): string {
  const time = t('live_time', { s: Math.min(p.budgetS, Math.floor(p.elapsedS)), total: p.budgetS });
  if (p.bestLengthMm == null) return `${t('live_first')} · ${time}`;
  const v = longLength(p.bestLengthMm, unit, 2);
  return `${t('live_best', { length: `${v.value} ${v.unit}`, e: p.efficiency == null ? '–' : (p.efficiency * 100).toFixed(1) })} · ${time}`;
}

/** "S, M, L and XL". */
export function listOf(items: string[]): string {
  if (items.length <= 1) return items.join('');
  return `${items.slice(0, -1).join(', ')} ${t('and')} ${items[items.length - 1]}`;
}

/** Interrupted tours (tab closed mid-way) must not leave the sample in place of the visitor's own style. */
export const BACKUP_KEY = 'beta-tour-backup';
export const STYLE_KEY = 'beta-style-v1';
export function recoverInterrupted(store: Pick<Storage, 'getItem' | 'setItem' | 'removeItem'> | null): boolean {
  if (!store) return false;
  try {
    const raw = store.getItem(BACKUP_KEY); if (raw === null) return false;
    const saved = JSON.parse(raw) as { snapshot?: string | null };
    if (typeof saved?.snapshot === 'string') store.setItem(STYLE_KEY, saved.snapshot); else store.removeItem(STYLE_KEY);
    store.removeItem(BACKUP_KEY);
    return true;
  } catch { try { store.removeItem(BACKUP_KEY); } catch { /* blocked */ } return false; }
}
