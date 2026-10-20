// Plain SVG charts, rendered at build time. Each row is HTML text (label + value)
// beside an aria-hidden SVG track, so the numbers stay readable without colour or SVG.
import { esc, fmt, html, raw, signed, type Raw } from './html.ts';

let chartCounter = 0;
const pct = (n: number) => `${Math.max(0, Math.min(100, n)).toFixed(3)}%`;

/** Smallest "nice" bound (1, 2, 2.5, 5 x 10^k) that is at least `value`. */
export function niceCeil(value: number): number {
  if (value <= 0) return 1;
  const power = 10 ** Math.floor(Math.log10(value));
  for (const step of [1, 2, 2.5, 5, 10]) if (step * power >= value - 1e-9) return step * power;
  return 10 * power;
}

function axis(ticks: number[], toX: (v: number) => number, format: (v: number) => string): Raw {
  return html`<div class="chart-axis" aria-hidden="true"><span></span><svg class="chart-ticks" width="100%" height="18" focusable="false">${ticks.map((t, i) => {
    const anchor = i === 0 ? 'start' : i === ticks.length - 1 ? 'end' : 'middle';
    return raw(`<text x="${pct(toX(t))}" y="13" text-anchor="${anchor}">${format(t)}</text>`);
  })}</svg><span></span></div>`;
}

function grid(ticks: number[], toX: (v: number) => number, zero?: number): string {
  return ticks.map((t) => `<line class="${zero !== undefined && t === zero ? 'chart-zero' : 'chart-grid'}" x1="${pct(toX(t))}" x2="${pct(toX(t))}" y1="0" y2="100%"/>`).join('');
}

export interface DivergingRow { label: string; value: number | null; note?: string }

/** Horizontal bars around zero. Negative = BETA behind the reference. */
export function divergingChart(opts: { title: string; desc: string; rows: DivergingRow[]; unit?: string; negLabel?: string; posLabel?: string; domain?: number }): Raw {
  const id = `chart-${++chartCounter}`;
  const unit = opts.unit ?? 'pp';
  const values = opts.rows.map((r) => r.value).filter((v): v is number => v !== null);
  const d = opts.domain ?? niceCeil(Math.max(1, ...values.map(Math.abs)));
  const toX = (v: number) => 50 + (v / d) * 50;
  const ticks = [-d, -d / 2, 0, d / 2, d];
  return html`<figure class="chart" aria-labelledby="${id}-t" aria-describedby="${id}-d">
  <figcaption><span class="chart-title" id="${id}-t">${opts.title}</span><span class="chart-desc" id="${id}-d">${opts.desc}</span></figcaption>
  <div class="chart-legend chart-legend-split" aria-hidden="true"><span><i class="swatch swatch-neg"></i>${opts.negLabel ?? 'BETA behind (longer marker)'}</span><span>${opts.posLabel ?? 'BETA ahead (shorter marker)'}<i class="swatch swatch-pos"></i></span></div>
  <ul class="chart-rows">${opts.rows.map((r) => {
    const v = r.value;
    const bar = v === null ? '' : `<rect class="${v < 0 ? 'bar-neg' : 'bar-pos'}" x="${pct(Math.min(toX(0), toX(v)))}" width="${pct(Math.abs(toX(v) - toX(0)))}" y="5" height="14" rx="3"><title>${esc(r.label)}: ${signed(v)} ${unit}</title></rect>`;
    return html`<li class="chart-row"><span class="chart-label">${r.label}${r.note ? html`<small>${r.note}</small>` : ''}</span><svg class="chart-track" width="100%" height="24" aria-hidden="true" focusable="false">${raw(grid(ticks, toX, 0) + bar)}</svg><span class="chart-value">${v === null ? 'n/a' : `${signed(v)} ${unit}`}</span></li>`;
  })}</ul>
  ${axis(ticks, toX, (t) => (t === 0 ? '0' : signed(t, Number.isInteger(t) ? 0 : 1)))}
</figure>`;
}

export interface DotRow { label: string; beta: number; reference: number }

/** Two dots per row on a shared % scale: reference (ring) and BETA (filled). */
export function dotChart(opts: { title: string; desc: string; rows: DotRow[]; betaLabel: string; referenceLabel: string; min?: number; max?: number }): Raw {
  const id = `chart-${++chartCounter}`;
  const all = opts.rows.flatMap((r) => [r.beta, r.reference]);
  const lo = opts.min ?? Math.floor(Math.min(...all) / 5) * 5;
  const hi = opts.max ?? Math.ceil(Math.max(...all) / 5) * 5;
  const toX = (v: number) => ((v - lo) / (hi - lo)) * 100;
  const ticks: number[] = [];
  for (let t = lo; t <= hi + 1e-9; t += 5) ticks.push(t);
  return html`<figure class="chart" aria-labelledby="${id}-t" aria-describedby="${id}-d">
  <figcaption><span class="chart-title" id="${id}-t">${opts.title}</span><span class="chart-desc" id="${id}-d">${opts.desc}</span></figcaption>
  <div class="chart-legend" aria-hidden="true"><span><i class="swatch swatch-ref"></i>${opts.referenceLabel}</span><span><i class="swatch swatch-beta"></i>${opts.betaLabel}</span></div>
  <ul class="chart-rows">${opts.rows.map((r) => {
    const x1 = pct(toX(Math.min(r.beta, r.reference))); const x2 = pct(toX(Math.max(r.beta, r.reference)));
    const svg = `${grid(ticks, toX)}<line class="dot-link" x1="${x1}" x2="${x2}" y1="12" y2="12"/>`
      + `<circle class="dot-ref" cx="${pct(toX(r.reference))}" cy="12" r="6"><title>${esc(r.label)}: ${esc(opts.referenceLabel)} ${fmt(r.reference)}%</title></circle>`
      + `<circle class="dot-beta" cx="${pct(toX(r.beta))}" cy="12" r="5.5"><title>${esc(r.label)}: ${esc(opts.betaLabel)} ${fmt(r.beta)}%</title></circle>`;
    return html`<li class="chart-row"><span class="chart-label">${r.label}</span><svg class="chart-track" width="100%" height="24" aria-hidden="true" focusable="false">${raw(svg)}</svg><span class="chart-value chart-value-wide"><span class="sr-only">${opts.betaLabel} </span>${fmt(r.beta)}%<span class="muted"> vs ${fmt(r.reference)}%</span><span class="sr-only"> ${opts.referenceLabel}</span></span></li>`;
  })}</ul>
  ${axis(ticks, toX, (t) => `${t}%`)}
</figure>`;
}

export interface BarRow { label: string; value: number }

/** Single-series bars from zero on a 0-100 % scale. */
export function barChart(opts: { title: string; desc: string; rows: BarRow[]; unit?: string; min?: number; max?: number }): Raw {
  const id = `chart-${++chartCounter}`;
  const lo = opts.min ?? 0; const hi = opts.max ?? 100;
  const toX = (v: number) => ((v - lo) / (hi - lo)) * 100;
  const ticks = [0, 0.25, 0.5, 0.75, 1].map((f) => lo + f * (hi - lo));
  const unit = opts.unit ?? '%';
  return html`<figure class="chart" aria-labelledby="${id}-t" aria-describedby="${id}-d">
  <figcaption><span class="chart-title" id="${id}-t">${opts.title}</span><span class="chart-desc" id="${id}-d">${opts.desc}</span></figcaption>
  <ul class="chart-rows">${opts.rows.map((r) => html`<li class="chart-row"><span class="chart-label">${r.label}</span><svg class="chart-track" width="100%" height="24" aria-hidden="true" focusable="false">${raw(`${grid(ticks, toX, lo)}<rect class="bar-single" x="0" width="${pct(toX(r.value))}" y="5" height="14" rx="3"><title>${esc(r.label)}: ${fmt(r.value)}${unit}</title></rect>`)}</svg><span class="chart-value">${fmt(r.value)}${unit}</span></li>`)}</ul>
  ${axis(ticks, toX, (t) => `${t}${unit}`)}
</figure>`;
}
