import { loadMarkerExportCore, type Job, type MarkerExportCore, type MarkerLabel, type MarkerResult, type Pt } from '@beta/marker-wasm';
import { fitLabel, planLabel } from './style/labels';

/** Piece labels on a marker (Style marker step and Marker editor). The one place that asks the engine where they go.
 * Rust (`MarkerExportCore.labels`) gives an anchor inside each placed piece, a readable angle and the clearance to the
 * nearest cut edge; the screen only decides the text size for the zoom and hides what does not fit. Until the
 * engine has loaded (or with an older engine build without labels), a display-only fit inside the outline is used. */
export interface PlacedLabel {
  /** Placement index in the marker result. */
  placement: number;
  /** Anchor in marker mm, y up. */
  at: Pt;
  rotation: number;
  name: string; detail: string;
  /** Font sizes in screen pixels. */
  nameSize: number; detailSize: number; showDetail: boolean;
}

let core: MarkerExportCore | null = null, loading: Promise<void> | null = null, unavailable = false;
const waiting = new Set<() => void>();
const cache = new WeakMap<MarkerResult, MarkerLabel[] | null>();

/** Start loading the engine labels; `ready` runs once they can be used (to redraw). */
export function prepareLabels(ready?: () => void): void {
  if (core || unavailable) return;
  if (ready) waiting.add(ready);
  loading ??= loadMarkerExportCore()
    .then(c => { core = typeof c.labels === 'function' ? c : null; unavailable = !core; })
    .catch(() => { unavailable = true; })
    .finally(() => { const list = [...waiting]; waiting.clear(); if (core) list.forEach(fn => fn()); });
}
/** Use this engine (tests, or a host with its own loader); null falls back to the display-only fit. */
export function useLabelEngine(engine: MarkerExportCore | null): void { core = engine; unavailable = !engine; }
function engineLabels(job: Job, result: MarkerResult): MarkerLabel[] | null {
  if (!core) return null;
  if (cache.has(result)) return cache.get(result)!;
  let out: MarkerLabel[] | null = null;
  try { out = core.labels(job, result.placements); } catch { out = null; }
  cache.set(result, out);
  return out;
}

/** Labels that fit at `pxPerMm` screen pixels per millimetre. `outlines` (placement order) are used only for the
 * fallback; `detailOf` gives the second line (size, and more in the editor). */
export function placedLabels(job: Job, result: MarkerResult, pxPerMm: number, outlines: (Pt[] | undefined)[], detailOf: (placement: number) => string): PlacedLabel[] {
  const out: PlacedLabel[] = [];
  const names = (placement: number) => { const p = result.placements[placement]; return job.pieces.find(q => q.id === p?.piece_id)?.name ?? p?.piece_id ?? ''; };
  const fromEngine = engineLabels(job, result);
  if (fromEngine) {
    for (const l of fromEngine) {
      // A circle of the clearance fits inside the piece; text may run a little wider along the baseline.
      const plan = fitLabel(l.anchor, l.name || names(l.placement), detailOf(l.placement), l.clearance_mm * 2.2 * pxPerMm, l.clearance_mm * 2 * pxPerMm * 0.9);
      if (plan) out.push({ placement: l.placement, at: plan.at, rotation: l.rotation_deg, name: plan.text, detail: detailOf(l.placement), nameSize: plan.name, detailSize: plan.detail, showDetail: plan.showDetail });
    }
    return out;
  }
  outlines.forEach((points, placement) => {
    if (!points || points.length < 3) return;
    const plan = planLabel(points, names(placement), detailOf(placement), pxPerMm);
    if (plan) out.push({ placement, at: plan.at, rotation: 0, name: plan.text, detail: detailOf(placement), nameSize: plan.name, detailSize: plan.detail, showDetail: plan.showDetail });
  });
  return out;
}

/** SVG text for labels on a marker drawn with svg y = fabric width - marker y; sizes become user units. */
export function labelsSvg(labels: PlacedLabel[], widthMm: number, mmPerPx: number, esc: (v: string) => string): string {
  return labels.map(l => {
    const x = l.at[0], y = widthMm - l.at[1], n = l.nameSize * mmPerPx, d = l.detailSize * mmPerPx;
    // Engine angles are counter-clockwise with y up; the drawing has y down.
    const turn = Math.abs(l.rotation) > 0.5 ? ` transform="rotate(${(-l.rotation).toFixed(1)} ${x.toFixed(1)} ${y.toFixed(1)})"` : '';
    return `<text x="${x.toFixed(1)}" y="${y.toFixed(1)}" text-anchor="middle"${turn}><tspan class="n" x="${x.toFixed(1)}" font-size="${n.toFixed(2)}" dy="${(l.showDetail ? -0.15 * n : 0.35 * n).toFixed(2)}">${esc(l.name)}</tspan>${l.showDetail ? `<tspan class="d" x="${x.toFixed(1)}" font-size="${d.toFixed(2)}" dy="${(1.15 * d).toFixed(2)}">${esc(l.detail)}</tspan>` : ''}</text>`;
  }).join('');
}
