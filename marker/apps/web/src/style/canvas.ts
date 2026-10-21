import type { Annotation, Hit, PatternDocument, PatternEdge, PatternInspection, PatternPiece, Pt, StyleEditOperation as StyleOperation } from '@beta/marker-wasm';
import { icon } from '../icons';
import type { StyleEngine } from './adapter';
import { has, t, type StyleKey } from './i18n';
import { chordWidth, edgeDistance, isProductionLabel, planLabel, shorten } from './labels';
import { chainLoop, inBox, snapPoint, type EdgeHit, type Guide, type SnapResult, type SnapTarget } from './snap';
import { boundsOf, fitView, pan, rulerTicks, toScreen, toWorld, viewBox, zoomAt, type View } from './view';
import { formatAngle, formatLength, fromMm, normaliseAngle, parseLength, step, type LengthUnit } from './units';

/** Pattern drafting canvas: tools with icons and names, direct manipulation, snapping with guides,
 * exact values in a small popover, multi-select, zoom and pan, rulers in cm or inch, an inspector and a piece list.
 * Every coordinate shown comes from the Rust engine (inspect); the canvas only turns pointer input into STYLE-API operations. */
export interface CanvasHost {
  engine(): StyleEngine | null;
  doc(): PatternDocument | null;
  unit(): LengthUnit;
  /** One undoable edit. Resolves false if the engine refused it (the reason is already shown). */
  commit(ops: StyleOperation[], done: string): Promise<boolean>;
  undo(): void; redo(): void; canUndo(): boolean; canRedo(): boolean;
  say(text: string): void;
  message(text: string, error?: boolean): void;
}
export type ToolId = 'select' | 'point' | 'line' | 'curve' | 'arc' | 'offset' | 'extend' | 'mirror' | 'split' | 'join' | 'dart' | 'pleat'
  | 'notch' | 'drill' | 'buttons' | 'text' | 'seam' | 'grain' | 'fold' | 'measure';
interface ToolSpec { id: ToolId; icon: string; key: string; group: 'draw' | 'shape' | 'mark' }
export const TOOLS: ToolSpec[] = [
  { id: 'select', icon: 'pointer', key: 'V', group: 'draw' }, { id: 'point', icon: 'point', key: 'P', group: 'draw' },
  { id: 'line', icon: 'line', key: 'L', group: 'draw' }, { id: 'curve', icon: 'curve', key: 'C', group: 'draw' }, { id: 'arc', icon: 'arc', key: 'A', group: 'draw' }, { id: 'measure', icon: 'measure', key: 'R', group: 'draw' },
  { id: 'offset', icon: 'offset', key: 'O', group: 'shape' }, { id: 'extend', icon: 'line', key: 'E', group: 'shape' }, { id: 'mirror', icon: 'mirror', key: 'M', group: 'shape' }, { id: 'split', icon: 'split', key: 'X', group: 'shape' },
  { id: 'join', icon: 'join', key: 'J', group: 'shape' }, { id: 'dart', icon: 'dart', key: 'D', group: 'shape' }, { id: 'pleat', icon: 'pleat', key: 'K', group: 'shape' },
  { id: 'notch', icon: 'notch', key: 'N', group: 'mark' }, { id: 'drill', icon: 'drill', key: 'H', group: 'mark' }, { id: 'buttons', icon: 'buttons', key: 'B', group: 'mark' }, { id: 'text', icon: 'text', key: 'T', group: 'mark' },
  { id: 'seam', icon: 'seam', key: 'S', group: 'mark' }, { id: 'grain', icon: 'grain', key: 'G', group: 'mark' }, { id: 'fold', icon: 'fold-line', key: 'F', group: 'mark' },
];
/** Lay block pieces side by side on the sheet when they overlap. Each piece's construction points share its id
 * prefix ("front.A"); its fixed points are moved by one offset (formulas kept, so every size moves together) and
 * the points built from them follow. Returns no operations when nothing overlaps. */
export function arrangeOps(doc: PatternDocument, ins: PatternInspection, gap = 80): StyleOperation[] {
  const boxes = doc.pieces.map(p => ({ p, b: boundsOf(ins.pieces.find(q => q.id === p.id)?.graded?.piece.outline ?? []) })).filter((x): x is { p: PatternPiece; b: [number, number, number, number] } => !!x.b);
  const overlap = boxes.some((a, i) => boxes.some((c, j) => j > i && a.b[0] < c.b[2] && c.b[0] < a.b[2] && a.b[1] < c.b[3] && c.b[1] < a.b[3]));
  if (!overlap) return [];
  const ops: StyleOperation[] = [];
  let cursor = Math.min(...boxes.map(x => x.b[0]));
  for (const { p, b } of boxes) {
    const w = b[2] - b[0], dx = cursor - b[0], dy = -b[1];
    cursor += w * 1.15 + gap;
    if (Math.abs(dx) < 0.5 && Math.abs(dy) < 0.5) continue;
    for (const [id, o] of Object.entries(doc.drawing)) {
      if (!id.startsWith(`${p.id}.`) || o.kind !== 'point' || o.operation.op !== 'absolute') continue;
      const at = ins.objects[id]?.points[0]; if (at) ops.push({ op: 'move_point', id, size: doc.base_size, target: [Math.round((at[0] + dx) * 100) / 100, Math.round((at[1] + dy) * 100) / 100] });
    }
  }
  return ops;
}
const esc = (v: unknown): string => String(v ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]!));
const tx = (key: StyleKey, values?: Record<string, string | number>) => esc(t(key, values));
const errorText = (e: unknown) => e instanceof Error ? e.message : String(e);
const round = (v: number, d = 1) => Math.round(v * 10 ** d) / 10 ** d;
const MATERIALS = ['shell', 'lining', 'fusing', 'rib'];

interface Step { at: Pt; id?: string; piece?: string; edge?: EdgeHit }
type Drag =
  | { kind: 'points'; ids: string[]; start: Pt; at: Pt; pointer: number; moved: boolean; anchor: Pt | null }
  | { kind: 'pan'; last: Pt; pointer: number }
  | { kind: 'box'; start: Pt; at: Pt; pointer: number; additive: boolean };
/** Selection keys: `pt:<id>`, `path:<id>` or `piece:<id>`. */
type Sel = string;

export function installCanvas(root: HTMLElement, host: CanvasHost) {
  let tool: ToolId = 'select', steps: Step[] = [], chain: string[] = [];
  let size = '', ins: PatternInspection | null = null, preview: PatternInspection | null = null, allSizes: Record<string, Pt[][]> = {};
  let view: View = { left: -100, top: 900, scale: 0.6 }, fitted = false;
  let selection = new Set<Sel>(), hover: Pt | null = null, snap: SnapResult | null = null, drag: Drag | undefined;
  let snapPoints = true, snapGrid = true, snapAngle = true, showAll = false, showNames = true, panKey = false;
  let measure: { text: string; points: Pt[] } | null = null, inspectorTab: 'selected' | 'pieces' = 'selected';
  let lastDoc = '', busy = false, previewTicket = 0;
  const touches = new Map<number, Pt>(); let pinch: { d: number; mid: Pt; view: View } | null = null;

  root.innerHTML = `<div class="sc">
    <aside class="sc-palette" aria-label="${tx('tools_label')}">${(['draw', 'shape', 'mark'] as const).map(g => `<div class="sc-group" role="group" aria-labelledby="sc-g-${g}"><span class="sc-group-title" id="sc-g-${g}">${tx(`group_${g}`)}</span>
      ${TOOLS.filter(x => x.group === g).map(x => `<button type="button" class="sc-tool" data-tool="${x.id}" aria-pressed="${x.id === tool}" aria-keyshortcuts="${x.key}" title="${tx(`tool_${x.id}_help`)}">${icon(x.icon)}<span>${tx(`tool_${x.id}`)}</span><kbd>${x.key}</kbd></button>`).join('')}</div>`).join('')}
      <div class="sc-group" role="group" aria-labelledby="sc-g-quick"><span class="sc-group-title" id="sc-g-quick">${tx('group_quick')}</span><button type="button" class="sc-tool" id="sc-rect">${icon('shape')}<span>${tx('rect_piece')}</span></button></div>
    </aside>
    <div class="sc-main">
      <div class="sc-bar" role="toolbar" aria-label="${tx('canvas_bar')}">
        <button type="button" class="icon-btn" id="sc-undo" aria-label="${tx('undo')}" data-tip="${tx('undo')} · Ctrl+Z" aria-keyshortcuts="Control+Z">${icon('undo')}</button>
        <button type="button" class="icon-btn" id="sc-redo" aria-label="${tx('redo')}" data-tip="${tx('redo')} · Ctrl+Y" aria-keyshortcuts="Control+Y">${icon('redo')}</button>
        <label class="sc-size"><span>${tx('size_shown')}</span><select id="sc-size"></select></label>
        <span class="sc-sep" aria-hidden="true"></span>
        <div class="segmented sc-snaps" role="group" aria-label="${tx('snap_label')}"><span class="sc-snap-word" aria-hidden="true">${tx('snap_word')}</span>
          <button type="button" id="sc-snap-points" aria-pressed="true">${tx('snap_points')}</button><button type="button" id="sc-snap-grid" aria-pressed="true">${tx('snap_grid')}</button><button type="button" id="sc-snap-angle" aria-pressed="true">${tx('snap_angle')}</button></div>
        <button type="button" id="sc-all" class="small sc-lbl-btn" aria-pressed="false" aria-label="${tx('show_all_sizes')}" data-tip="${tx('show_all_sizes')}">${icon('layers')}<span class="sc-lbl">${tx('show_all_sizes')}</span></button>
        <button type="button" id="sc-names" class="small sc-lbl-btn" aria-pressed="true" aria-label="${tx('show_names')}" data-tip="${tx('show_names')}">${icon('text')}<span class="sc-lbl">${tx('show_names')}</span></button>
        <button type="button" id="sc-arrange" class="small sc-lbl-btn" aria-label="${tx('arrange')}" data-tip="${tx('arrange_hint')}">${icon('swatches')}<span class="sc-lbl">${tx('arrange')}</span></button>
        <div class="push"></div>
        <button type="button" class="icon-btn" id="sc-zoom-out" aria-label="${tx('zoom_out')}" data-tip="${tx('zoom_out')} · −">${icon('zoom-out')}</button>
        <button type="button" class="icon-btn" id="sc-fit" aria-label="${tx('fit')}" data-tip="${tx('fit')} · 0">${icon('fit')}</button>
        <button type="button" class="icon-btn" id="sc-zoom-in" aria-label="${tx('zoom_in')}" data-tip="${tx('zoom_in')} · +" data-tip-align="end">${icon('zoom-in')}</button>
      </div>
      <div class="sc-stage" id="sc-stage">
        <span class="sc-ruler-unit" id="sc-ruler-unit" aria-hidden="true"></span><svg class="sc-ruler-x" id="sc-ruler-x" aria-hidden="true"></svg><svg class="sc-ruler-y" id="sc-ruler-y" aria-hidden="true"></svg>
        <div class="sc-cell"><svg class="sc-canvas" id="sc-canvas" tabindex="0" role="application" aria-roledescription="${tx('canvas_role')}" aria-label="${tx('canvas_label')}" aria-describedby="sc-hint"><g id="sc-grid"></g><g id="sc-static"></g><g id="sc-live"></g></svg>
        <div class="sc-empty" id="sc-empty" hidden><strong>${tx('blank_title')}</strong><span>${tx('blank_text')}</span><div class="toolgroup"><button type="button" data-tool="line">${icon('line')}${tx('tool_line')}</button><button type="button" id="sc-rect-empty" class="primary">${icon('shape')}${tx('rect_piece')}</button></div></div>
        <form class="sc-pop" id="sc-pop" hidden></form></div>
      </div>
      <div class="sc-foot"><p class="sc-hint" id="sc-hint" aria-live="polite"></p><span class="sc-cursor" id="sc-cursor" aria-hidden="true"></span></div>
    </div>
    <aside class="sc-inspector" aria-label="${tx('inspector')}">
      <div class="segmented sc-tabs" role="group" aria-label="${tx('inspector')}"><button type="button" data-tab="selected" aria-pressed="true">${tx('tab_selected')}</button><button type="button" data-tab="pieces" aria-pressed="false">${tx('tab_pieces')}</button></div>
      <div id="sc-inspect" class="sc-inspect"></div>
    </aside></div>`;
  const $ = <T extends Element = HTMLElement>(s: string): T => root.querySelector<T>(s)!;
  const svg = $<SVGSVGElement>('#sc-canvas'), pop = $<HTMLFormElement>('#sc-pop');
  const doc = () => host.doc();
  const unit = () => host.unit();
  const L = (mm: number, withUnit = true) => formatLength(mm, unit(), withUnit);
  const px = (n: number) => n / view.scale;
  const stageSize = (): [number, number] => { const r = svg.getBoundingClientRect(); return [Math.max(1, r.width), Math.max(1, r.height)]; };
  const geometry = () => preview ?? ins;

  // ---------- Engine data ----------
  async function reload(): Promise<void> {
    const d = doc(), e = host.engine();
    if (!d || !e) { ins = null; render(); return; }
    if (!d.sizes.includes(size)) size = d.base_size;
    const key = JSON.stringify(d) + size;
    if (key === lastDoc && ins) { render(); return; }
    lastDoc = key;
    try {
      ins = await e.inspect(d, size); preview = null;
      if (showAll) { const g = await e.grade(d); allSizes = Object.fromEntries(Object.entries(g.sizes).map(([s, pieces]) => [s, pieces.map(p => p.piece.outline)])); }
    } catch (error) { host.message(errorText(error), true); }
    // Drop selections that no longer exist.
    selection = new Set([...selection].filter(s => s.startsWith('piece:') ? d.pieces.some(p => `piece:${p.id}` === s) : !!ins?.objects[s.slice(s.indexOf(':') + 1)]));
    if (!fitted && stageSize()[0] > 80) { fit(); fitted = true; }
    render();
  }

  // ---------- Picking (on Rust geometry) ----------
  const controlPoints = (): Set<string> => {
    const d = doc(), out = new Set<string>(); if (!d) return out;
    for (const o of Object.values(d.drawing)) if (o.kind === 'cubic') { out.add(o.points[1]); out.add(o.points[2]); }
    return out;
  };
  const visibleHandles = (): Set<string> => {
    const d = doc(), out = new Set<string>(); if (!d) return out;
    for (const [id, o] of Object.entries(d.drawing)) if (o.kind === 'cubic' && (selection.has(`path:${id}`) || selection.has(`pt:${o.points[1]}`) || selection.has(`pt:${o.points[2]}`))) { out.add(o.points[1]); out.add(o.points[2]); }
    return out;
  };
  function pointTargets(ignore: string[] = []): SnapTarget[] {
    const g = geometry(); if (!g) return [];
    const hidden = controlPoints(), shown = visibleHandles();
    return Object.entries(g.objects).filter(([id, o]) => o.kind === 'point' && !ignore.includes(id) && (!hidden.has(id) || shown.has(id))).map(([id, o]) => ({ id, at: o.points[0]! }));
  }
  /** Rust hit test (STYLE-API `hit_test`) for the size shown; one cached answer per position and radius. */
  let lastHits: { key: string; hits: Hit[] } | null = null, cursorFrame = 0;
  function hits(at: Pt, radius: number): Hit[] {
    const e = host.engine(), d = doc(); if (!e || !d) return [];
    const key = `${lastDoc}|${size}|${at[0].toFixed(2)},${at[1].toFixed(2)}|${radius.toFixed(3)}`;
    if (lastHits?.key === key) return lastHits.hits;
    let list: Hit[] = [];
    try { list = e.hitTestNow(d, size, at, radius); } catch { /* a document the engine cannot read has nothing to pick */ }
    lastHits = { key, hits: list }; return list;
  }
  function pick(at: Pt): Sel | null {
    const list = hits(at, px(9)), g = geometry(); if (!g) return null;
    const shown = new Set(pointTargets().map(p => p.id));
    const point = list.find(h => h.kind === 'point' && shown.has(h.id)); if (point) return `pt:${point.id}`;
    const path = list.find(h => h.kind === 'path' && g.objects[h.id]?.kind === 'path'); if (path) return `path:${path.id}`;
    return pieceAt(at) ? `piece:${pieceAt(at)}` : null;
  }
  function pieceAt(at: Pt): string | null { return hits(at, px(9)).filter(h => h.kind === 'piece').at(-1)?.id ?? null; }
  /** The piece edge under the pointer for notches, darts, seam allowance and fold: piece, zero-based edge and distance along it, from Rust. */
  function edgeAt(at: Pt): { piece: string; edge: EdgeHit } | null {
    const h = hits(at, px(12)).find(x => x.kind === 'path' && x.piece_id != null && x.edge != null && x.along_mm != null); if (!h) return null;
    const length = geometry()?.pieces.find(p => p.id === h.piece_id)?.edges?.[h.edge!]?.length_mm ?? 0;
    return { piece: h.piece_id!, edge: { edge: h.edge!, along: h.along_mm!, point: h.point, distance: h.distance_mm, segment: 0, length } };
  }
  function snapAt(at: Pt, ignore: string[] = [], anchor: Pt | null = null): SnapResult {
    const grid = unit() === 'inch' ? 25.4 / 8 : 5;
    return snapPoint(at, pointTargets(ignore), { radius: px(10), points: snapPoints, grid: snapGrid ? (view.scale > 1.5 ? grid / (unit() === 'inch' ? 2 : 5) : grid) : 0, angleStep: snapAngle ? 15 : 0, align: snapPoints, anchor, ignore });
  }

  // ---------- Ids and operations ----------
  function nextId(prefix: string, taken: Set<string>): string {
    const d = doc(); let n = 1;
    while (taken.has(`${prefix}${n}`) || d?.drawing[`${prefix}${n}`] || d?.pieces.some(p => p.id === `${prefix}${n}`)) n++;
    const id = `${prefix}${n}`; taken.add(id); return id;
  }
  /** A step's point: the existing point it snapped to, or a new fixed point (rounded to 0.1 mm). */
  function pointFor(s: Step, ops: StyleOperation[], taken: Set<string>): string {
    if (s.id) return s.id;
    const id = nextId('pt', taken);
    ops.push({ op: 'put_object', id, object: { kind: 'point', operation: { op: 'absolute', x: round(s.at[0]), y: round(s.at[1]) } } });
    s.id = id; return id;
  }
  const piece = (id: string): PatternPiece | undefined => doc()?.pieces.find(p => p.id === id);
  /** A piece cut on the fold is symmetric: edge i and edge n-1-i are mirror twins and change together (STYLE-API). */
  /** Allowances may be numbers or formulas such as `v.seam_allowance`; show the value when it is a plain variable. */
  const allowanceMm = (v: PatternEdge['allowance']): number | null => {
    if (typeof v === 'number') return v;
    const name = /^v\.([A-Za-z_][A-Za-z0-9_]*)$/.exec(String(v ?? '').trim())?.[1], value = name ? doc()?.variables[name] : undefined;
    return typeof value === 'number' ? value : v == null ? 0 : null;
  };
  /** Seam allowance on a piece cut on the fold must stay symmetric (the engine rejects one-sided widths): edge i and n-1-i change together. */
  const withMirror = (p: PatternPiece, edge: number): number[] => p.fold_line && p.edges.length > 2 ? [...new Set([edge, p.edges.length - 1 - edge])].sort((a, b) => a - b) : [edge];
  async function commit(ops: StyleOperation[], done: string): Promise<boolean> {
    if (busy || !ops.length) return false;
    busy = true;
    try { return await host.commit(ops, done); } finally { busy = false; }
  }

  // ---------- Tools ----------
  function setTool(next: ToolId): void {
    tool = next; steps = []; chain = []; measure = null; closePop();
    root.querySelectorAll<HTMLButtonElement>('.sc-tool[data-tool]').forEach(b => b.setAttribute('aria-pressed', String(b.dataset.tool === tool)));
    svg.dataset.tool = tool; hint(); renderLive();
    host.say(t('tool_now', { tool: t(`tool_${tool}`) }));
  }
  function hint(): void {
    const n = steps.length;
    const key = `hint_${tool}_${Math.min(n, 3)}` as StyleKey;
    let text = t(has(key) ? key : `hint_${tool}_0` as StyleKey);
    if (tool === 'line' && chain.length) text += ' ' + t('hint_line_chain');
    $('#sc-hint').innerHTML = `${icon(TOOLS.find(x => x.id === tool)!.icon)}<span>${esc(text)}</span>`;
  }
  async function toolClick(at: Pt, raw: Pt, shift: boolean): Promise<void> {
    const d = doc(); if (!d) return;
    const sp = snapAt(raw, [], steps.at(-1)?.at ?? null);
    const s: Step = { at: sp.at, id: sp.kind === 'point' ? sp.target : undefined };
    const taken = new Set<string>();
    switch (tool) {
      case 'select': {
        const hit = pick(raw);
        if (!hit) { if (!shift) selection.clear(); }
        else if (shift) { if (selection.has(hit)) selection.delete(hit); else selection.add(hit); }
        else selection = new Set([hit]);
        render(); return;
      }
      case 'point': {
        if (s.id) { selection = new Set([`pt:${s.id}`]); render(); return; }
        const ops: StyleOperation[] = []; const id = pointFor(s, ops, taken);
        if (await commit(ops, t('done_point'))) { selection = new Set([`pt:${id}`]); render(); }
        return;
      }
      case 'line': {
        if (!steps.length) { steps = [s]; hint(); openPop('line'); renderLive(); return; }
        await addLineTo(s); return;
      }
      case 'curve': {
        if (!steps.length) { steps = [s]; hint(); renderLive(); return; }
        const a = steps[0]!, ops: StyleOperation[] = [];
        const ida = pointFor(a, ops, taken), idb = pointFor(s, ops, taken);
        if (ida === idb) { host.message(t('err_same_point'), true); return; }
        const [ax, ay] = a.at, [bx, by] = s.at, nx = -(by - ay) * 0.2, ny = (bx - ax) * 0.2;
        const c1 = nextId('pt', taken), c2 = nextId('pt', taken), id = nextId('cv', taken);
        ops.push({ op: 'put_object', id: c1, object: { kind: 'point', operation: { op: 'absolute', x: round(ax + (bx - ax) / 3 + nx), y: round(ay + (by - ay) / 3 + ny) } } },
          { op: 'put_object', id: c2, object: { kind: 'point', operation: { op: 'absolute', x: round(ax + 2 * (bx - ax) / 3 + nx), y: round(ay + 2 * (by - ay) / 3 + ny) } } },
          { op: 'put_object', id, object: { kind: 'cubic', points: [ida, c1, c2, idb] } });
        steps = [];
        if (await commit(ops, t('done_curve'))) { selection = new Set([`path:${id}`]); render(); }
        hint(); return;
      }
      case 'arc': {
        if (steps.length < 2) { steps.push(s); hint(); renderLive(); return; }
        const [c, a] = steps, ops: StyleOperation[] = [];
        const centre = pointFor(c!, ops, taken), radius = Math.hypot(a!.at[0] - c!.at[0], a!.at[1] - c!.at[1]);
        const start = Math.atan2(a!.at[1] - c!.at[1], a!.at[0] - c!.at[0]) * 180 / Math.PI, end = Math.atan2(s.at[1] - c!.at[1], s.at[0] - c!.at[0]) * 180 / Math.PI;
        let sweep = ((end - start) % 360 + 360) % 360; if (shift) sweep -= 360; if (Math.abs(sweep) < 0.5) sweep = 360;
        const id = nextId('arc', taken);
        ops.push({ op: 'put_object', id, object: { kind: 'arc', center: centre, radius: round(radius), start: round(start), sweep: round(sweep) } });
        steps = [];
        if (radius < 1) { host.message(t('err_small'), true); hint(); return; }
        if (await commit(ops, t('done_arc'))) { selection = new Set([`path:${id}`]); render(); }
        hint(); return;
      }
      case 'extend': {
        const hit = pick(raw), id = hit?.startsWith('path:') ? hit.slice(5) : '', o = id ? d.drawing[id] : undefined;
        if (o?.kind !== 'line') { host.message(t('err_pick_line'), true); return; }
        if (!steps.length) { steps = [{ at: s.at, id }]; selection = new Set([`path:${id}`]); render(); hint(); return; }
        const source = steps[0]!.id!, src = d.drawing[source], g = geometry();
        if (id === source || src?.kind !== 'line') { host.message(t('err_pick_other_line'), true); return; }
        const a = g?.objects[src.from]?.points[0], b = g?.objects[src.to]?.points[0], c0 = steps[0]!.at;
        const end = a && b && Math.hypot(b[0] - c0[0], b[1] - c0[1]) < Math.hypot(a[0] - c0[0], a[1] - c0[1]) ? 1 : 0;
        const nid = nextId(`${source.replace(/[^\w]/g, '_')}_ext`, taken); steps = [];
        if (await commit([{ op: 'extend_trim', id: nid, source, target: id, end }], t('done_extend'))) { selection = new Set([`path:${nid}`]); render(); }
        hint(); return;
      }
      case 'offset': case 'join': {
        const near = tool === 'offset' ? pick(raw) : null;
        if (near?.startsWith('path:')) { steps = [{ at: s.at, id: near.slice(5) }]; selection = new Set([near]); render(); openPop('offset'); return; }
        const p = pieceAt(raw); if (!p) { host.message(t(tool === 'offset' ? 'err_pick_offset' : 'err_pick_piece'), true); return; }
        if (tool === 'offset') { steps = [{ at: s.at, piece: p }]; openPop('offset'); return; }
        if (!steps.length) { steps = [{ at: s.at, piece: p }]; hint(); return; }
        if (steps[0]!.piece === p) { host.message(t('err_two_pieces'), true); return; }
        const id = nextId(`${steps[0]!.piece}_join`, taken), a = steps[0]!.piece!; steps = [];
        await commit([{ op: 'join_pieces', pieces: [a, p], id, size }], t('done_join')); hint(); return;
      }
      case 'mirror': case 'split': case 'pleat': case 'grain': {
        if (!steps.length) { const p = pieceAt(raw); if (!p) { host.message(t('err_pick_piece'), true); return; } steps = [{ at: s.at, piece: p }]; hint(); return; }
        steps.push(s); hint(); renderLive();
        if (steps.length < 3) return;
        if (tool === 'pleat') { openPop('pleat'); return; }
        await finishTwoPoint(); return;
      }
      case 'dart': {
        if (!steps.length) { const e = edgeAt(raw); if (!e) { host.message(t('err_pick_edge'), true); return; } steps = [{ at: e.edge.point, piece: e.piece, edge: e.edge }]; hint(); renderLive(); return; }
        steps.push(s); openPop('dart'); return;
      }
      case 'notch': case 'seam': case 'fold': {
        const e = edgeAt(raw); if (!e) { host.message(t('err_pick_edge'), true); return; }
        const p = piece(e.piece)!;
        if (tool === 'notch') {
          // On a piece cut on the fold the engine adds the mirrored notch itself when the lay is folded or a tube.
          await commit([{ op: 'add_annotation', piece: p.id, annotation: { kind: 'shaped_notch', edge: e.edge.edge, distance: round(e.edge.along), depth: 5, width: 3, shape: 'slit' } }], t('done_notch', { piece: p.name }));
          return;
        }
        if (tool === 'seam') { steps = [{ at: e.edge.point, piece: p.id, edge: e.edge }]; openPop('seam'); return; }
        const edge = p.edges[e.edge.edge]!, o = d.drawing[edge.object];
        if (o?.kind !== 'line') { host.message(t('err_fold_straight'), true); return; }
        const ends: [string, string] = edge.reversed ? [o.to, o.from] : [o.from, o.to];
        await commit([{ op: 'put_piece', replace: true, piece: { ...p, fold_line: ends } }], t('done_fold', { piece: p.name }));
        return;
      }
      case 'drill': case 'text': case 'buttons': {
        const p = pieceAt(raw) ?? steps[0]?.piece; if (!p) { host.message(t('err_pick_piece'), true); return; }
        if (tool === 'drill') {
          // The engine mirrors drill holes across the fold itself on a folded or tube hand-off.
          const ops: StyleOperation[] = [], id = pointFor(s, ops, taken);
          ops.push({ op: 'add_annotation', piece: p, annotation: { kind: 'points', mark: 'drill', points: [id], text: null } });
          await commit(ops, t('done_drill')); return;
        }
        if (tool === 'text') { steps = [{ ...s, piece: p }]; openPop('text'); return; }
        steps.push({ ...s, piece: p }); hint(); renderLive();
        if (steps.length === 2) openPop('buttons');
        return;
      }
      case 'measure': {
        const hit = pick(raw);
        if (!steps.length && hit?.startsWith('path:')) {
          const id = hit.slice(5), e = host.engine()!;
          try { const m = await e.measure(d, size, { kind: 'length', object: id }); measure = { text: t('measure_length', { value: L(m.value) }), points: m.points }; } catch (error) { host.message(errorText(error), true); }
          renderLive(); announceMeasure(); return;
        }
        steps.push(s);
        if (steps.length === 2) {
          const [a, b] = steps as [Step, Step];
          const angle = Math.atan2(b.at[1] - a.at[1], b.at[0] - a.at[0]) * 180 / Math.PI;
          measure = { text: t('measure_distance', { value: L(Math.hypot(b.at[0] - a.at[0], b.at[1] - a.at[1])), angle: formatAngle(angle), dx: L(Math.abs(b.at[0] - a.at[0])), dy: L(Math.abs(b.at[1] - a.at[1])) }), points: [a.at, b.at] };
          steps = []; announceMeasure();
        }
        hint(); renderLive(); return;
      }
    }
  }
  function announceMeasure(): void { if (measure) { host.say(measure.text); $('#sc-hint').innerHTML = `${icon('measure')}<span><strong>${esc(measure.text)}</strong> · ${tx('hint_measure_0')}</span>`; } }
  /** Line tool: add a segment to `s`. Clicking the first point of the chain closes the shape into a piece. */
  async function addLineTo(s: Step): Promise<void> {
    const a = steps.at(-1)!, ops: StyleOperation[] = [], taken = new Set<string>();
    const ida = pointFor(a, ops, taken), idb = pointFor(s, ops, taken);
    if (ida === idb) { host.message(t('err_same_point'), true); return; }
    const id = nextId('ln', taken);
    ops.push({ op: 'put_object', id, object: { kind: 'line', from: ida, to: idb } });
    const closing = chain.length >= 2 && steps[0]!.id === idb;
    let pieceId = '';
    if (closing) {
      pieceId = nextId('piece', taken);
      const n = (doc()?.pieces.length ?? 0) + 1;
      ops.push({ op: 'put_piece', piece: { id: pieceId, name: t('new_piece_name', { n }), quantity: 1, mirror_quantity: 0, material: 'shell', annotations: [], grade_points: {}, anchors: {},
        edges: [...chain, id].map(object => ({ object, reversed: false, allowance: 10, corner: 'miter' as const })) } });
    }
    if (!(await commit(ops, closing ? t('done_piece') : t('done_line')))) return;
    if (closing) { steps = []; chain = []; closePop(); selection = new Set([`piece:${pieceId}`]); inspectorTab = 'selected'; render(); host.say(t('done_piece')); hint(); return; }
    chain.push(id); steps.push(s); openPop('line'); hint(); renderLive();
  }
  async function finishTwoPoint(extra: Record<string, number | string> = {}): Promise<void> {
    const [base, a, b] = steps as [Step, Step, Step], p = piece(base.piece!)!, ops: StyleOperation[] = [], taken = new Set<string>();
    const ida = pointFor(a, ops, taken), idb = pointFor(b, ops, taken);
    if (ida === idb) { host.message(t('err_same_point'), true); return; }
    let done = '';
    if (tool === 'grain') { ops.push({ op: 'put_piece', replace: true, piece: { ...p, grain: [ida, idb] } }); done = t('done_grain', { piece: p.name }); }
    else {
      const id = nextId(`${p.id}_${tool}`, taken);
      ops.push({ op: 'piece_action', request: { action: tool, piece: p.id, id, size, a: ida, b: idb, ...extra } });
      done = t(`done_${tool as 'mirror' | 'split' | 'pleat'}`, { piece: p.name });
    }
    steps = []; closePop();
    await commit(ops, done); hint(); renderLive();
  }

  // ---------- Popover: exact values ----------
  function openPop(kind: 'line' | 'point' | 'offset' | 'pleat' | 'dart' | 'seam' | 'text' | 'buttons' | 'rect'): void {
    const u = unit() === 'inch' ? 'in' : 'cm';
    const num = (id: string, label: string, value: string, extra = '') => `<label class="sc-field">${label}<span class="sc-num"><input id="${id}" inputmode="decimal" autocomplete="off" value="${esc(value)}" ${extra}><span>${extra.includes('data-angle') ? '°' : u}</span></span></label>`;
    const p = steps[0]?.piece ? piece(steps[0].piece) : undefined;
    const body: Record<typeof kind, string> = {
      line: `${num('pop-length', tx('length'), '')}${num('pop-angle', tx('angle'), '', 'data-angle')}`,
      point: `${num('pop-x', 'X', '')}${num('pop-y', 'Y', '')}`,
      offset: `${num('pop-distance', tx('distance'), formatLength(10, unit()))}${steps[0]?.piece ? '' : `<label class="sc-field">${tx('offset_side')}<select id="pop-side"><option value="left">${tx('side_left')}</option><option value="right">${tx('side_right')}</option></select></label>`}`,
      pleat: `${num('pop-depth', tx('pleat_depth'), formatLength(20, unit()))}<label class="sc-field">${tx('pleat_kind')}<select id="pop-kind"><option value="knife">${tx('pleat_knife')}</option><option value="box">${tx('pleat_box')}</option></select></label>`,
      dart: num('pop-width', tx('dart_width'), formatLength(20, unit())),
      seam: `${num('pop-width', tx('seam_width'), formatLength(allowanceMm(p?.edges[steps[0]?.edge?.edge ?? 0]?.allowance) ?? 10, unit()))}<label class="check"><input type="checkbox" id="pop-all">${tx('seam_all_edges')}</label><label class="sc-field">${tx('corner')}<select id="pop-corner"><option value="miter">${tx('corner_miter')}</option><option value="square">${tx('corner_square')}</option><option value="bevel">${tx('corner_bevel')}</option><option value="mirrored">${tx('corner_mirrored')}</option></select></label>`,
      text: `<label class="sc-field">${tx('label_text')}<input id="pop-text" maxlength="60" value="${esc(p?.name ?? '')}"></label>`,
      buttons: `<label class="sc-field">${tx('mark_kind')}<select id="pop-kind"><option value="button">${tx('mark_button')}</option><option value="buttonhole">${tx('mark_buttonhole')}</option><option value="drill">${tx('mark_drill')}</option></select></label><label class="sc-field">${tx('count')}<input id="pop-count" type="number" min="1" max="60" step="1" value="5"></label>${num('pop-length', tx('button_size'), formatLength(12, unit()))}`,
      rect: `${num('pop-w', tx('width'), unit() === 'inch' ? '6' : '15')}${num('pop-h', tx('height'), unit() === 'inch' ? '7' : '18')}<label class="sc-field">${tx('piece_name')}<input id="pop-name" maxlength="60" value="${esc(t('rect_default_name'))}"></label>`,
    };
    pop.innerHTML = `<p class="sc-pop-title">${tx(`pop_${kind}`)}</p>${body[kind]}<div class="sc-pop-actions"><button type="button" class="small ghost" id="pop-cancel">${tx('cancel')}</button><button type="submit" class="small primary">${tx(kind === 'line' ? 'pop_line_ok' : 'ok')}</button></div>`;
    pop.dataset.kind = kind; pop.hidden = false; placePop();
    if (kind !== 'line') pop.querySelector<HTMLInputElement>('input, select')?.focus();
    pop.querySelector<HTMLButtonElement>('#pop-cancel')!.onclick = () => { if (kind === 'line') { steps = []; chain = []; } else if (['dart', 'buttons', 'pleat'].includes(kind)) steps = steps.slice(0, -1); else steps = []; closePop(); hint(); renderLive(); svg.focus(); };
  }
  /** The popover stays in the top-right corner of the drawing so it never covers the next click. */
  function placePop(): void { /* Docked by CSS. */ }
  function closePop(): void { pop.hidden = true; pop.innerHTML = ''; }
  const popNum = (id: string, negative = false): number | null => { const i = pop.querySelector<HTMLInputElement>(`#${id}`); return i ? parseLength(i.value, unit(), negative) : null; };
  pop.addEventListener('submit', event => { event.preventDefault(); void submitPop(); });
  async function submitPop(): Promise<void> {
    const kind = pop.dataset.kind, d = doc(); if (!d) return;
    const taken = new Set<string>();
    const bad = (key: StyleKey) => { host.message(t(key), true); };
    if (kind === 'line') {
      const a = steps.at(-1); if (!a) return;
      const len = popNum('pop-length'), angText = pop.querySelector<HTMLInputElement>('#pop-angle')!.value.trim();
      if (len == null || len <= 0) return bad('err_length');
      const live = hover ? Math.atan2(hover[1] - a.at[1], hover[0] - a.at[0]) * 180 / Math.PI : 0;
      const angle = angText ? Number(angText.replace(',', '.')) : live;
      if (!Number.isFinite(angle)) return bad('err_angle');
      const rad = angle * Math.PI / 180;
      await addLineTo({ at: [a.at[0] + len * Math.cos(rad), a.at[1] + len * Math.sin(rad)] });
      pop.querySelector<HTMLInputElement>('#pop-length')?.focus();
      return;
    }
    if (kind === 'rect') {
      const w = popNum('pop-w'), h = popNum('pop-h'), name = pop.querySelector<HTMLInputElement>('#pop-name')!.value.trim() || t('rect_default_name');
      if (!w || !h) return bad('err_length');
      const g = geometry(), b = boundsOf(Object.values(g?.objects ?? {}).flatMap(o => o.points));
      const x0 = b ? round(b[2] + 100, 0) : 0, y0 = b ? round(b[1], 0) : 0, ops: StyleOperation[] = [];
      const ids = [[0, 0], [w, 0], [w, h], [0, h]].map(([x, y]) => { const id = nextId('pt', taken); ops.push({ op: 'put_object', id, object: { kind: 'point', operation: { op: 'absolute', x: round(x0 + x!), y: round(y0 + y!) } } }); return id; });
      const lines = ids.map((from, i) => { const id = nextId('ln', taken); ops.push({ op: 'put_object', id, object: { kind: 'line', from, to: ids[(i + 1) % 4]! } }); return id; });
      const g0 = nextId('pt', taken), g1 = nextId('pt', taken), pid = nextId('piece', taken);
      ops.push({ op: 'put_object', id: g0, object: { kind: 'point', operation: { op: 'absolute', x: round(x0 + w / 2), y: round(y0 + h * 0.2) } } },
        { op: 'put_object', id: g1, object: { kind: 'point', operation: { op: 'absolute', x: round(x0 + w / 2), y: round(y0 + h * 0.8) } } },
        { op: 'put_piece', piece: { id: pid, name, quantity: 1, mirror_quantity: 0, material: 'shell', grain: [g0, g1], annotations: [], grade_points: {}, anchors: {}, edges: lines.map(object => ({ object, reversed: false, allowance: 10, corner: 'miter' as const })) } });
      closePop();
      if (await commit(ops, t('done_piece'))) { selection = new Set([`piece:${pid}`]); fitted = false; lastDoc = ''; await reload(); }
      return;
    }
    if (kind === 'point') {
      const x = popNum('pop-x', true), y = popNum('pop-y', true); if (x == null || y == null) return bad('err_length');
      const ops: StyleOperation[] = [], id = pointFor({ at: [x, y] }, ops, taken); closePop();
      if (await commit(ops, t('done_point'))) { selection = new Set([`pt:${id}`]); render(); }
      return;
    }
    if (kind === 'offset' && steps[0]?.id && !steps[0].piece) {
      const dist = popNum('pop-distance'); if (dist == null) return bad('err_length');
      const source = steps[0].id, side = pop.querySelector<HTMLSelectElement>('#pop-side')!.value as 'left' | 'right', id = nextId(`${source.replace(/[^\w]/g, '_')}_par`, taken);
      closePop(); steps = [];
      if (await commit([{ op: 'offset_path', id, source, distance: round(dist, 2), side }], t('done_offset_path'))) { selection = new Set([`path:${id}`]); render(); }
      hint(); return;
    }
    const base = steps[0]; if (!base?.piece) { closePop(); return; }
    const p = piece(base.piece)!;
    if (kind === 'offset') {
      const dist = popNum('pop-distance', true); if (!dist) return bad('err_length');
      closePop(); steps = [];
      await commit([{ op: 'offset_piece', piece: p.id, id: nextId(`${p.id}_offset`, taken), size, distance: dist }], t('done_offset', { piece: p.name })); return;
    }
    if (kind === 'pleat') {
      const depth = popNum('pop-depth'); if (!depth) return bad('err_length');
      await finishTwoPoint({ depth, pleat_kind: pop.querySelector<HTMLSelectElement>('#pop-kind')!.value }); return;
    }
    if (kind === 'dart') {
      const width = popNum('pop-width'); if (!width) return bad('err_length');
      const ops: StyleOperation[] = [], apex = pointFor(steps[1]!, ops, taken), id = nextId(`${p.id}_dart`, taken);
      ops.push({ op: 'piece_action', request: { action: 'dart', piece: p.id, id, size, seam_edge: base.edge!.edge, center: round(base.edge!.along), width, apex } });
      closePop(); steps = []; await commit(ops, t('done_dart', { piece: p.name })); hint(); renderLive(); return;
    }
    if (kind === 'seam') {
      const width = popNum('pop-width'); if (width == null) return bad('err_length');
      const all = pop.querySelector<HTMLInputElement>('#pop-all')!.checked, corner = pop.querySelector<HTMLSelectElement>('#pop-corner')!.value as 'miter';
      closePop(); steps = [];
      await commit([{ op: 'set_allowance', piece: p.id, edges: all ? p.edges.map((_, i) => i) : withMirror(p, base.edge!.edge), width: round(width, 2), corner }], t('done_seam', { piece: p.name, width: L(width) })); return;
    }
    if (kind === 'text') {
      const text = pop.querySelector<HTMLInputElement>('#pop-text')!.value.trim(); if (!text) return bad('err_text');
      const ops: StyleOperation[] = [], id = pointFor(base, ops, taken);
      ops.push({ op: 'add_annotation', piece: p.id, annotation: { kind: 'points', mark: 'text', points: [id], text } });
      closePop(); steps = []; await commit(ops, t('done_text')); return;
    }
    if (kind === 'buttons') {
      const count = Number(pop.querySelector<HTMLInputElement>('#pop-count')!.value), length = popNum('pop-length');
      if (!Number.isInteger(count) || count < 1 || count > 60) return bad('err_count');
      if (!length) return bad('err_length');
      const [a, b] = steps as [Step, Step], ops: StyleOperation[] = [], origin = pointFor(a, ops, taken);
      const dist = Math.hypot(b.at[0] - a.at[0], b.at[1] - a.at[1]), direction = Math.atan2(b.at[1] - a.at[1], b.at[0] - a.at[0]) * 180 / Math.PI;
      ops.push({ op: 'mark_row', row: { piece: p.id, id: nextId(`${p.id}_btn`, taken), origin, count, spacing: count > 1 ? round(dist / (count - 1), 2) : 0, direction: round(direction, 2), kind: pop.querySelector<HTMLSelectElement>('#pop-kind')!.value as 'button', length: round(length, 2), angle: round(direction, 2) } });
      closePop(); steps = []; await commit(ops, t('done_buttons', { count })); hint(); renderLive();
    }
  }

  // ---------- Rendering ----------
  function render(): void {
    const d = doc(), g = geometry();
    $<HTMLSelectElement>('#sc-size').innerHTML = (d?.sizes ?? []).map(s => `<option value="${esc(s)}" ${s === size ? 'selected' : ''}>${esc(s)}${s === d?.base_size ? ` · ${tx('base')}` : ''}</option>`).join('');
    $<HTMLButtonElement>('#sc-undo').disabled = !host.canUndo(); $<HTMLButtonElement>('#sc-redo').disabled = !host.canRedo();
    $('#sc-empty').hidden = !d || Object.keys(d.drawing).length > 0 || tool !== 'select' || steps.length > 0;
    renderView();
    const r = 4 / view.scale, hidden = controlPoints(), handles = visibleHandles();
    let out = '';
    if (showAll && Object.keys(allSizes).length) {
      d?.sizes.forEach((s, i) => { if (s === size) return; for (const o of allSizes[s] ?? []) out += `<polygon class="sc-size-ghost" points="${pts(o)}" style="stroke:var(--size-${i % 6})"/>`; });
    }
    (g?.pieces ?? []).forEach((p, i) => {
      const gp = p.graded; if (!gp) return;
      const sel = selection.has(`piece:${p.id}`), info = d?.pieces.find(q => q.id === p.id);
      out += `<g class="sc-piece${sel ? ' selected' : ''}${p.ok ? '' : ' bad'}"><polygon class="sc-cut" points="${pts(gp.piece.outline)}" style="fill:var(--fill-${i % 6})"/><polygon class="sc-stitch" points="${pts(gp.seam)}"/>`;
      if (gp.piece.grain) out += arrow(gp.piece.grain[0], gp.piece.grain[1], 'sc-grain');
      if (gp.piece.fold_line) out += `<line class="sc-fold" x1="${gp.piece.fold_line[0][0]}" y1="${-gp.piece.fold_line[0][1]}" x2="${gp.piece.fold_line[1][0]}" y2="${-gp.piece.fold_line[1][1]}"/>`;
      const name = info?.name ?? p.id;
      for (const m of gp.piece.marks ?? []) {
        if (m.kind === 'text' && m.points[0]) { if (showNames && m.text && !isProductionLabel(m.text, name)) out += markText(m.text, m.points[0], gp.piece.outline); }
        else if (m.points.length === 1) out += `<circle class="sc-drill" cx="${m.points[0]![0]}" cy="${-m.points[0]![1]}" r="${px(3.5)}"/>`;
        else out += `<polyline class="sc-mark" points="${pts(m.points)}"/>`;
      }
      if (showNames) out += pieceLabel(gp.piece.outline, name, info);
      out += `<title>${esc(name)}</title></g>`;
    });
    const objects = Object.entries(g?.objects ?? {});
    for (const [id, o] of objects) if (o.kind === 'path') {
      const sel = selection.has(`path:${id}`);
      out += `<polyline class="sc-path${sel ? ' selected' : ''}" points="${pts(o.points)}"><title>${esc(id)}</title></polyline>`;
    }
    // Curve handles for selected curves.
    for (const [id, o] of Object.entries(d?.drawing ?? {})) if (o.kind === 'cubic' && handles.has(o.points[1])) {
      const at = (k: string) => g?.objects[k]?.points[0];
      const [a, c1, c2, b] = o.points.map(at);
      if (a && c1) out += `<line class="sc-handle-line" x1="${a[0]}" y1="${-a[1]}" x2="${c1[0]}" y2="${-c1[1]}"/>`;
      if (b && c2) out += `<line class="sc-handle-line" x1="${b[0]}" y1="${-b[1]}" x2="${c2[0]}" y2="${-c2[1]}"/>`;
      void id;
    }
    for (const [id, o] of objects) if (o.kind === 'point' && (!hidden.has(id) || handles.has(id))) {
      const [x, y] = o.points[0]!, sel = selection.has(`pt:${id}`), handle = hidden.has(id);
      out += handle ? `<rect class="sc-handle${sel ? ' selected' : ''}" x="${x - r * 1.2}" y="${-y - r * 1.2}" width="${r * 2.4}" height="${r * 2.4}"><title>${esc(id)}</title></rect>`
        : `<circle class="sc-point${sel ? ' selected' : ''}" cx="${x}" cy="${-y}" r="${sel ? r * 1.5 : r}"><title>${esc(id)}</title></circle>`;
    }
    $('#sc-static').innerHTML = out;
    renderLive(); renderInspector();
  }
  const pts = (list: Pt[]) => list.map(([x, y]) => `${x},${-y}`).join(' ');
  /** Name and "size · cut" inside the piece, sized for the zoom; hidden when the piece is too small on screen. */
  function pieceLabel(outline: Pt[], name: string, info: PatternPiece | undefined): string {
    const material = info?.material && info.material !== 'shell' ? ` · ${has(`material_${info.material}`) ? t(`material_${info.material}` as StyleKey) : info.material}` : '';
    const detail = `${size} · ${t('cut_n', { n: info?.quantity ?? 1 })}${info?.mirror_quantity ? ` + ${t('mirror_n', { n: info.mirror_quantity })}` : ''}${material}`;
    const plan = planLabel(outline, name, detail, view.scale); if (!plan) return '';
    const [x, y] = plan.at, lift = plan.showDetail ? px(plan.detail * 0.62) : 0;
    return `<text class="sc-piece-name" x="${x}" y="${-y - lift}" font-size="${px(plan.name)}" text-anchor="middle" dominant-baseline="middle">${esc(plan.text)}${plan.showDetail ? `<tspan class="sc-piece-detail" x="${x}" dy="${px(plan.name * 0.62 + plan.detail * 0.72)}" font-size="${px(plan.detail)}">${esc(detail)}</tspan>` : ''}</text>`;
  }
  /** A text mark placed by the user: small, shortened to the piece width, hidden when the piece is small on screen. */
  function markText(text: string, at: Pt, outline: Pt[]): string {
    const room = chordWidth(outline, at) * view.scale * 0.9, fit = shorten(text, room / 11);
    return fit && view.scale * edgeDistance(outline, at) > 6 ? `<text class="sc-mark-text" x="${at[0]}" y="${-at[1]}" font-size="${px(11)}" text-anchor="middle" dominant-baseline="middle">${esc(fit)}</text>` : '';
  }
  function arrow(a: Pt, b: Pt, cls: string): string {
    const ang = Math.atan2(b[1] - a[1], b[0] - a[0]), h = px(9), w = px(5);
    const head = (p: Pt, dir: number) => `M${p[0]},${-p[1]} L${p[0] - h * Math.cos(dir) + w * Math.sin(dir)},${-(p[1] - h * Math.sin(dir) - w * Math.cos(dir))} M${p[0]},${-p[1]} L${p[0] - h * Math.cos(dir) - w * Math.sin(dir)},${-(p[1] - h * Math.sin(dir) + w * Math.cos(dir))}`;
    return `<path class="${cls}" d="M${a[0]},${-a[1]} L${b[0]},${-b[1]} ${head(b, ang)} ${head(a, ang + Math.PI)}"/>`;
  }
  function renderView(): void {
    const [w, h] = stageSize();
    svg.setAttribute('viewBox', viewBox(view, w, h));
    // Grid: 1 cm (or 1 inch) lines, 10 cm (or 6 inch) bold lines, only when dense enough to help.
    const fine = unit() === 'inch' ? 25.4 : 10, bold = unit() === 'inch' ? 152.4 : 100;
    const [l, top] = [view.left, view.top], right = l + w / view.scale, bottom = top - h / view.scale;
    let grid = `<rect class="sc-paper" x="${l}" y="${-top}" width="${w / view.scale}" height="${h / view.scale}"/>`;
    for (const step of [fine, bold]) {
      if (step * view.scale < 8) continue;
      let d = '';
      for (let x = Math.ceil(l / step) * step; x <= right; x += step) d += `M${x},${-top}V${-bottom}`;
      for (let y = Math.ceil(bottom / step) * step; y <= top; y += step) d += `M${l},${-y}H${right}`;
      grid += `<path class="${step === bold ? 'sc-grid-bold' : 'sc-grid'}" d="${d}"/>`;
    }
    grid += `<path class="sc-axis" d="M${l},0H${right}M0,${-top}V${-bottom}"/>`;
    $('#sc-grid').innerHTML = grid;
    // Rulers in screen pixels.
    const tx_ = rulerTicks(l, right, view.scale, unit()), ty = rulerTicks(bottom, top, view.scale, unit());
    $('#sc-ruler-x').innerHTML = tx_.map(k => { const x = (k.mm - l) * view.scale; return `<line x1="${x}" x2="${x}" y1="${k.major ? 6 : 15}" y2="22"/>${k.label ? `<text x="${x + 3}" y="12">${k.label}</text>` : ''}`; }).join('');
    $('#sc-ruler-y').innerHTML = ty.map(k => { const y = (top - k.mm) * view.scale; return `<line y1="${y}" y2="${y}" x1="${k.major ? 6 : 15}" x2="22"/>${k.label ? `<text x="12" y="${y - 3}" transform="rotate(-90 12 ${y - 3})">${k.label}</text>` : ''}`; }).join('');
    $('#sc-ruler-unit').textContent = unit() === 'inch' ? 'in' : 'cm';
  }
  function renderLive(): void {
    let out = '';
    const guides: Guide[] = snap?.guides ?? [];
    for (const gd of guides) out += gd.kind === 'ring' ? `<circle class="sc-guide-ring" cx="${gd.at[0]}" cy="${-gd.at[1]}" r="${px(9)}"/>` : `<line class="sc-guide" x1="${gd.from[0]}" y1="${-gd.from[1]}" x2="${gd.to[0]}" y2="${-gd.to[1]}"/>`;
    const at = snap?.at ?? hover;
    const last = steps.at(-1);
    if (at && last && ['line', 'curve', 'measure', 'mirror', 'split', 'pleat', 'grain', 'buttons', 'dart'].includes(tool)) {
      out += `<line class="sc-rubber" x1="${last.at[0]}" y1="${-last.at[1]}" x2="${at[0]}" y2="${-at[1]}"/>`;
      const len = Math.hypot(at[0] - last.at[0], at[1] - last.at[1]), ang = Math.atan2(at[1] - last.at[1], at[0] - last.at[0]) * 180 / Math.PI;
      const mx = (at[0] + last.at[0]) / 2, my = (at[1] + last.at[1]) / 2;
      out += `<text class="sc-live-label" x="${mx + px(8)}" y="${-my - px(8)}" font-size="${px(12)}">${esc(L(len))} · ${esc(formatAngle(ang))}</text>`;
      if (tool === 'line' && !pop.hidden && document.activeElement?.closest('#sc-pop') == null) {
        const li = pop.querySelector<HTMLInputElement>('#pop-length'), ai = pop.querySelector<HTMLInputElement>('#pop-angle');
        if (li) li.placeholder = formatLength(len, unit()); if (ai) ai.placeholder = String(round(normaliseAngle(ang)));
      }
    }
    if (at && tool === 'arc' && steps.length) {
      const c = steps[0]!.at, rr = Math.hypot((steps[1]?.at ?? at)[0] - c[0], (steps[1]?.at ?? at)[1] - c[1]);
      out += `<circle class="sc-rubber" cx="${c[0]}" cy="${-c[1]}" r="${rr}"/>`;
    }
    for (const s of steps) out += `<circle class="sc-step" cx="${s.at[0]}" cy="${-s.at[1]}" r="${px(5)}"/>`;
    if (steps[0]?.edge) out += `<polyline class="sc-edge-pick" points="${pts([steps[0].edge.point])}"/>`;
    if (['notch', 'seam', 'fold', 'dart'].includes(tool) && hover && !steps.length) {
      const e = edgeAt(hover);
      if (e) { const line = geometry()?.pieces.find(p => p.id === e.piece)?.graded?.seam_edges?.[e.edge.edge]; if (line) out += `<polyline class="sc-edge-hover" points="${pts(line)}"/><circle class="sc-step" cx="${e.edge.point[0]}" cy="${-e.edge.point[1]}" r="${px(4)}"/>`; }
    }
    if (measure) {
      out += `<polyline class="sc-measure" points="${pts(measure.points)}"/>`;
      const m = measure.points[Math.floor(measure.points.length / 2)]!;
      out += `<text class="sc-measure-label" x="${m[0] + px(8)}" y="${-m[1] + px(18)}" font-size="${px(13)}">${esc(measure.text)}</text>`;
    }
    if (drag?.kind === 'box') { const [a, b] = [drag.start, drag.at]; out += `<rect class="sc-box" x="${Math.min(a[0], b[0])}" y="${-Math.max(a[1], b[1])}" width="${Math.abs(b[0] - a[0])}" height="${Math.abs(b[1] - a[1])}"/>`; }
    if (at && tool !== 'select') out += `<g class="sc-cross"><line x1="${at[0] - px(10)}" x2="${at[0] + px(10)}" y1="${-at[1]}" y2="${-at[1]}"/><line x1="${at[0]}" x2="${at[0]}" y1="${-at[1] - px(10)}" y2="${-at[1] + px(10)}"/></g>`;
    $('#sc-live').innerHTML = out;
    $('#sc-cursor').textContent = at ? `X ${L(at[0])} · Y ${L(at[1])}${snap && snap.kind !== 'free' ? ` · ${t(`snapped_${snap.kind}`)}` : ''}` : '';
    placePop();
  }

  // ---------- Inspector ----------
  function renderInspector(): void {
    root.querySelectorAll<HTMLButtonElement>('[data-tab]').forEach(b => b.setAttribute('aria-pressed', String(b.dataset.tab === inspectorTab)));
    const box = $('#sc-inspect'), d = doc(), g = geometry();
    const active = document.activeElement as HTMLElement | null, keep = active && box.contains(active) ? active.id : '';
    if (!d) { box.innerHTML = ''; return; }
    if (inspectorTab === 'pieces') {
      box.innerHTML = `<p class="hint">${tx('pieces_hint')}</p><ul class="sc-piece-list">${d.pieces.map((p, i) => {
        const ok = g?.pieces.find(q => q.id === p.id)?.ok;
        return `<li class="${selection.has(`piece:${p.id}`) ? 'selected' : ''}"><button type="button" class="sc-piece-pick" data-pick-piece="${esc(p.id)}"><i class="swatch" style="background:var(--fill-${i % 6})"></i><strong>${esc(p.name)}</strong><span class="chip ${ok ? 'ok' : 'bad'}">${ok ? tx('closed') : tx('not_closed')}</span></button>
          <div class="sc-piece-row"><span class="sc-stepper"><button type="button" class="icon-btn small" data-qty="${esc(p.id)}" data-delta="-1" aria-label="${tx('fewer', { name: p.name })}">${icon('minus')}</button><output aria-label="${tx('quantity')}">${p.quantity ?? 1}</output><button type="button" class="icon-btn small" data-qty="${esc(p.id)}" data-delta="1" aria-label="${tx('more', { name: p.name })}">${icon('plus')}</button></span>
          <label class="check sc-pair"><input type="checkbox" data-pair="${esc(p.id)}" ${p.mirror_quantity ? 'checked' : ''}>${tx('pair')}</label>
          <select data-material="${esc(p.id)}" aria-label="${tx('fabric_of', { name: p.name })}">${MATERIALS.map(m => `<option value="${m}" ${(p.material ?? 'shell') === m ? 'selected' : ''}>${tx(`material_${m}` as StyleKey)}</option>`).join('')}</select></div></li>`;
      }).join('') || `<li class="hint">${tx('no_pieces')}</li>`}</ul>`;
    } else box.innerHTML = selectedHtml();
    if (keep) document.getElementById(keep)?.focus();
  }
  function linkedNote(d: PatternDocument, p: PatternPiece): string {
    const o = d.drawing[p.edges[0]?.object ?? ''] as unknown as { kind?: string; recipe?: { source?: unknown; operation?: unknown } } | undefined;
    if (o?.kind !== 'derived_path') return '';
    const src = ([] as unknown[]).concat(o.recipe?.source ?? []).map(x => typeof x === 'string' ? (d.pieces.find(q => q.id === x)?.name ?? x) : '').filter(Boolean).join(' + ');
    return `<p class="notice sc-linked">${icon('layers')}<span>${tx('linked_piece', { source: src || '—' })}</span></p>`;
  }
  function selectedHtml(): string {
    const d = doc()!, g = geometry(), list = [...selection];
    if (!list.length) return `<h3>${tx('nothing_selected')}</h3><ul class="sc-tips"><li>${tx('tip_select')}</li><li>${tx('tip_draw')}</li><li>${tx('tip_type')}</li><li>${tx('tip_keys')}</li></ul>
      <p class="hint">${tx('doc_summary', { pieces: d.pieces.length, objects: Object.keys(d.drawing).length, sizes: d.sizes.length })}</p>`;
    if (list.length > 1) {
      const paths = list.filter(s => s.startsWith('path:'));
      return `<h3>${tx('n_selected', { n: list.length })}</h3><p class="hint">${tx('multi_hint')}</p><div class="sc-actions">
        ${paths.length >= 2 ? `<button type="button" class="primary" id="sc-make-piece">${icon('shape')}${tx('make_piece')}</button>` : ''}
        <button type="button" class="danger" id="sc-delete">${icon('trash')}${tx('delete')}</button></div>`;
    }
    const sel = list[0]!, id = sel.slice(sel.indexOf(':') + 1);
    if (sel.startsWith('pt:')) {
      const o = d.drawing[id], at = g?.objects[id]?.points[0], fixed = o?.kind === 'point' && o.operation.op === 'absolute';
      return `<h3>${icon('point')}${tx('point')} <code>${esc(id)}</code></h3>
        <div class="sc-fields"><label class="sc-field">${tx('name')}<input id="sc-rename" value="${esc(id)}" autocomplete="off" pattern="[A-Za-z_][A-Za-z0-9_.]*"></label>
        <label class="sc-field">X<span class="sc-num"><input id="sc-x" inputmode="decimal" value="${at ? formatLength(at[0], unit()) : ''}"><span>${unit() === 'inch' ? 'in' : 'cm'}</span></span></label>
        <label class="sc-field">Y<span class="sc-num"><input id="sc-y" inputmode="decimal" value="${at ? formatLength(at[1], unit()) : ''}"><span>${unit() === 'inch' ? 'in' : 'cm'}</span></span></label></div>
        <p class="hint">${fixed ? tx('point_fixed', { size }) : tx('point_built', { how: o?.kind === 'point' ? o.operation.op.replaceAll('_', ' ') : '' })}</p>
        ${!fixed && at ? `<button type="button" id="sc-free">${tx('free_point')}</button>` : ''}
        <details class="advanced"><summary>${tx('formula')}</summary><pre class="sc-formula">${esc(JSON.stringify(o?.kind === 'point' ? o.operation : o, null, 1))}</pre></details>
        <div class="sc-actions"><button type="button" class="danger" id="sc-delete">${icon('trash')}${tx('delete')}</button></div>`;
    }
    if (sel.startsWith('path:')) {
      const o = d.drawing[id], pts_ = g?.objects[id]?.points ?? [];
      let len = 0; for (let i = 1; i < pts_.length; i++) len += Math.hypot(pts_[i]![0] - pts_[i - 1]![0], pts_[i]![1] - pts_[i - 1]![1]);
      const a = pts_[0], b = pts_.at(-1);
      const linked = (o as unknown as { kind?: string; source?: string } | undefined);
      if (linked?.kind === 'offset_path') return `<h3>${icon('offset')}${tx('parallel_line')} <code>${esc(id)}</code></h3><dl class="sc-facts"><div><dt>${tx('length')}</dt><dd>${esc(L(len))}</dd></div></dl>
        <p class="hint">${tx('parallel_hint', { source: linked.source ?? '' })}</p><div class="sc-actions"><button type="button" class="danger" id="sc-delete">${icon('trash')}${tx('delete')}</button></div>`;
      const kind = o?.kind === 'line' ? 'line' : o?.kind === 'arc' ? 'arc' : 'curve';
      return `<h3>${icon(kind)}${tx(kind)} <code>${esc(id)}</code></h3><dl class="sc-facts"><div><dt>${tx('length')}</dt><dd>${esc(L(len))}</dd></div>${kind === 'line' && a && b ? `<div><dt>${tx('angle')}</dt><dd>${esc(formatAngle(Math.atan2(b[1] - a[1], b[0] - a[0]) * 180 / Math.PI))}</dd></div>` : ''}</dl>
        ${kind === 'curve' ? `<p class="hint">${tx('curve_hint')}</p>` : ''}
        <div class="sc-actions"><button type="button" class="danger" id="sc-delete">${icon('trash')}${tx('delete')}</button></div>`;
    }
    const p = d.pieces.find(q => q.id === id); if (!p) return '';
    const gp = g?.pieces.find(q => q.id === id);
    return `<h3>${icon('shape')}${tx('piece')}</h3><div class="sc-fields">
        <label class="sc-field">${tx('piece_name')}<input id="sc-piece-name" value="${esc(p.name)}" maxlength="60"></label>
        <label class="sc-field">${tx('quantity')}<input id="sc-piece-qty" type="number" min="0" max="99" step="1" value="${p.quantity ?? 1}"></label>
        <label class="sc-field">${tx('fabric')}<select id="sc-piece-material">${MATERIALS.map(m => `<option value="${m}" ${(p.material ?? 'shell') === m ? 'selected' : ''}>${tx(`material_${m}` as StyleKey)}</option>`).join('')}</select></label></div>
      <label class="check"><input type="checkbox" id="sc-piece-pair" ${p.mirror_quantity ? 'checked' : ''}>${tx('pair_long')}</label>
      <p class="chip ${gp?.ok ? 'ok' : 'bad'}">${gp?.ok ? tx('closed') : esc(gp?.error ?? tx('not_closed'))}</p>
      <p class="hint">${p.grain ? tx('has_grain') : tx('no_grain')} · ${p.fold_line ? tx('has_fold') : tx('no_fold')}</p>${linkedNote(d, p)}
      <h4>${tx('seam_allowance')}</h4><table class="sc-edges"><thead><tr><th scope="col">${tx('edge')}</th><th scope="col">${tx('length')}</th><th scope="col">${tx('allowance')}</th></tr></thead><tbody>
      ${p.edges.map((e, i) => `<tr><th scope="row">${i + 1}</th><td>${gp?.edges?.[i] ? esc(L(gp.edges[i]!.length_mm, false)) : '—'}</td><td><input class="sc-allow" data-allow="${i}" inputmode="decimal" aria-label="${tx('allowance_edge', { n: i + 1 })}" value="${allowanceMm(e.allowance) != null ? formatLength(allowanceMm(e.allowance)!, unit()) : esc(e.allowance)}" ${typeof e.allowance === 'string' ? `title="${esc(e.allowance)}"` : ''}></td></tr>`).join('')}</tbody></table>
      ${(p.annotations ?? []).length ? `<h4>${tx('marks')}</h4><ul class="sc-marks">${(p.annotations ?? []).map((a, i) => `<li>${esc(markName(a))}<button type="button" class="small ghost" data-remove-mark="${i}" aria-label="${tx('remove_mark', { n: i + 1 })}">${icon('close')}</button></li>`).join('')}</ul>` : ''}
      <div class="sc-actions"><button type="button" id="sc-copy-piece">${icon('copy')}${tx('copy_piece')}</button><button type="button" class="danger" id="sc-delete">${icon('trash')}${tx('delete_piece')}</button></div>`;
  }
  const markName = (a: Annotation): string => a.kind === 'shaped_notch' || a.kind === 'notch' ? t('mark_notch_on', { n: a.edge + 1 }) : a.kind === 'button' ? t('mark_button') : a.kind === 'seam_intersections' ? t('mark_notch') : a.mark === 'text' ? `${t('mark_text')}: ${a.text ?? ''}` : a.mark === 'drill' ? t('mark_drill') : t('mark_line');

  async function inspectorChange(target: HTMLInputElement | HTMLSelectElement): Promise<void> {
    const d = doc(); if (!d) return;
    const [sel] = [...selection], id = sel ? sel.slice(sel.indexOf(':') + 1) : '';
    const p = target.dataset.qty ?? target.dataset.pair ?? target.dataset.material ? piece(target.dataset.pair ?? target.dataset.material ?? '') : piece(id);
    if (target.dataset.pair || target.dataset.material) {
      const q = p!; await commit([{ op: 'put_piece', replace: true, piece: target.dataset.pair ? { ...q, mirror_quantity: (target as HTMLInputElement).checked ? Math.max(1, q.quantity ?? 1) : 0 } : { ...q, material: target.value } }], t('done_piece_saved', { piece: q.name })); return;
    }
    if (target.id === 'sc-rename') {
      const next = target.value.trim(); if (!next || next === id) return;
      if (!/^[A-Za-z_][A-Za-z0-9_.]*$/.test(next)) { host.message(t('err_name'), true); return; }
      if (await commit([{ op: 'rename_object', id, new_id: next }], t('done_rename'))) { selection = new Set([`pt:${next}`]); render(); }
      return;
    }
    if (target.id === 'sc-x' || target.id === 'sc-y') {
      const x = parseLength($<HTMLInputElement>('#sc-x').value, unit(), true), y = parseLength($<HTMLInputElement>('#sc-y').value, unit(), true);
      if (x == null || y == null) { host.message(t('err_length'), true); return; }
      await commit([{ op: 'move_point', id, size, target: [x, y] }], t('done_move')); return;
    }
    if (!p) return;
    if (target.id === 'sc-piece-name') { const name = target.value.trim(); if (name) await commit([{ op: 'put_piece', replace: true, piece: { ...p, name } }], t('done_piece_saved', { piece: name })); return; }
    if (target.id === 'sc-piece-qty') { const q = Number(target.value); if (Number.isInteger(q) && q >= 0) await commit([{ op: 'put_piece', replace: true, piece: { ...p, quantity: q, mirror_quantity: p.mirror_quantity ? q : 0 } }], t('done_piece_saved', { piece: p.name })); return; }
    if (target.id === 'sc-piece-material') { await commit([{ op: 'put_piece', replace: true, piece: { ...p, material: target.value } }], t('done_piece_saved', { piece: p.name })); return; }
    if (target.id === 'sc-piece-pair') { await commit([{ op: 'put_piece', replace: true, piece: { ...p, mirror_quantity: (target as HTMLInputElement).checked ? Math.max(1, p.quantity ?? 1) : 0 } }], t('done_piece_saved', { piece: p.name })); return; }
    if (target.dataset.allow) {
      const width = parseLength(target.value, unit()); if (width == null) { host.message(t('err_length'), true); return; }
      await commit([{ op: 'set_allowance', piece: p.id, edges: withMirror(p, Number(target.dataset.allow)), width: round(width, 2), corner: p.edges[Number(target.dataset.allow)]?.corner ?? 'miter' }], t('done_seam', { piece: p.name, width: L(width) }));
    }
  }
  async function inspectorClick(b: HTMLButtonElement): Promise<void> {
    const d = doc(); if (!d) return;
    if (b.dataset.tab) { inspectorTab = b.dataset.tab as 'selected'; renderInspector(); return; }
    if (b.dataset.pickPiece) { selection = new Set([`piece:${b.dataset.pickPiece}`]); render(); return; }
    if (b.dataset.qty) { const p = piece(b.dataset.qty)!, q = step(p.quantity ?? 1, Number(b.dataset.delta), 0, 99); await commit([{ op: 'put_piece', replace: true, piece: { ...p, quantity: q, mirror_quantity: p.mirror_quantity ? q : 0 } }], t('done_piece_saved', { piece: p.name })); return; }
    const [sel] = [...selection], id = sel ? sel.slice(sel.indexOf(':') + 1) : '';
    if (b.id === 'sc-delete') { await deleteSelection(); return; }
    if (b.id === 'sc-free') {
      const at = geometry()?.objects[id]?.points[0]; if (!at) return;
      await commit([{ op: 'put_object', id, replace: true, object: { kind: 'point', operation: { op: 'absolute', x: round(at[0], 2), y: round(at[1], 2) } } }], t('done_free')); return;
    }
    if (b.id === 'sc-copy-piece') { const p = piece(id)!; const nid = nextId(`${p.id}_copy`, new Set()); if (await commit([{ op: 'put_piece', piece: { ...structuredClone(p), id: nid, name: t('copy_of', { name: p.name }) } }], t('done_copy'))) { selection = new Set([`piece:${nid}`]); render(); } return; }
    if (b.dataset.removeMark) { const p = piece(id)!; await commit([{ op: 'put_piece', replace: true, piece: { ...p, annotations: (p.annotations ?? []).filter((_, i) => i !== Number(b.dataset.removeMark)) } }], t('done_mark_removed')); return; }
    if (b.id === 'sc-make-piece') await makePiece();
  }
  async function makePiece(): Promise<void> {
    const g = geometry(); if (!g) return;
    const paths = [...selection].filter(s => s.startsWith('path:')).map(s => s.slice(5));
    const loop = chainLoop(paths.map(id => ({ id, start: g.objects[id]!.points[0]!, end: g.objects[id]!.points.at(-1)! })), 0.05);
    if (!loop) { host.message(t('err_not_loop'), true); return; }
    const pid = nextId('piece', new Set()), n = (doc()?.pieces.length ?? 0) + 1;
    if (await commit([{ op: 'put_piece', piece: { id: pid, name: t('new_piece_name', { n }), quantity: 1, mirror_quantity: 0, material: 'shell', annotations: [], grade_points: {}, anchors: {}, edges: loop.map(e => ({ ...e, allowance: 10, corner: 'miter' as const })) } }], t('done_piece'))) { selection = new Set([`piece:${pid}`]); render(); }
  }
  async function deleteSelection(): Promise<void> {
    const list = [...selection]; if (!list.length) return;
    const ops: StyleOperation[] = [
      ...list.filter(s => s.startsWith('piece:')).map(s => ({ op: 'delete_piece' as const, id: s.slice(6) })),
      ...list.filter(s => s.startsWith('path:')).map(s => ({ op: 'delete_object' as const, id: s.slice(5) })),
      ...list.filter(s => s.startsWith('pt:')).map(s => ({ op: 'delete_object' as const, id: s.slice(3) })),
    ];
    if (await commit(ops, t('done_delete', { n: list.length }))) { selection.clear(); render(); }
  }
  async function nudge(dx: number, dy: number): Promise<void> {
    const g = geometry(); const ids = [...selection].filter(s => s.startsWith('pt:')).map(s => s.slice(3));
    if (!ids.length || !g) { view = pan(view, -dx * 40, dy * 40); renderView(); render(); return; }
    await commit(ids.map(id => { const at = g.objects[id]!.points[0]!; return { op: 'move_point' as const, id, size, target: [at[0] + dx, at[1] + dy] as Pt }; }), t('done_move'));
  }

  // ---------- Pointer input ----------
  /** Pointer capture keeps drags alive outside the canvas; a pen or test pointer may not support it. */
  const capture = (id: number) => { try { svg.setPointerCapture(id); } catch { /* drag still works inside the canvas */ } };
  const local = (e: PointerEvent | WheelEvent): Pt => { const r = svg.getBoundingClientRect(); return [e.clientX - r.left, e.clientY - r.top]; };
  svg.addEventListener('wheel', e => { e.preventDefault(); view = zoomAt(view, local(e), Math.exp(-Math.max(-0.4, Math.min(0.4, e.deltaY * 0.0015)))); render(); }, { passive: false });
  svg.addEventListener('pointerdown', e => {
    svg.focus({ preventScroll: true });
    const sp = local(e), at = toWorld(view, sp);
    if (e.pointerType === 'touch') {
      touches.set(e.pointerId, sp);
      if (touches.size === 2) { const [a, b] = [...touches.values()] as [Pt, Pt]; pinch = { d: Math.hypot(a[0] - b[0], a[1] - b[1]), mid: [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2], view: { ...view } }; drag = undefined; return; }
    }
    if (e.button === 1 || panKey || (e.button === 0 && tool === 'select' && e.pointerType === 'touch' && !pick(at))) {
      drag = { kind: 'pan', last: sp, pointer: e.pointerId }; capture(e.pointerId); e.preventDefault(); return;
    }
    if (e.button !== 0) return;
    if (tool === 'select') {
      const hit = pick(at);
      if (hit?.startsWith('pt:')) {
        if (!selection.has(hit)) selection = e.shiftKey ? new Set([...selection, hit]) : new Set([hit]);
        const ids = [...selection].filter(s => s.startsWith('pt:')).map(s => s.slice(3));
        const d = doc(), handle = Object.values(d?.drawing ?? {}).find(o => o.kind === 'cubic' && (o.points[1] === hit.slice(3) || o.points[2] === hit.slice(3)));
        const anchorId = handle?.kind === 'cubic' ? (handle.points[1] === hit.slice(3) ? handle.points[0] : handle.points[3]) : null;
        drag = { kind: 'points', ids, start: geometry()!.objects[hit.slice(3)]!.points[0]!, at, pointer: e.pointerId, moved: false, anchor: anchorId ? geometry()?.objects[anchorId]?.points[0] ?? null : null };
        capture(e.pointerId); render(); e.preventDefault(); return;
      }
      if (!hit && e.pointerType !== 'touch') { drag = { kind: 'box', start: at, at, pointer: e.pointerId, additive: e.shiftKey }; capture(e.pointerId); e.preventDefault(); return; }
    }
    e.preventDefault();
    void toolClick(snapAt(at, [], steps.at(-1)?.at ?? null).at, at, e.shiftKey);
  });
  svg.addEventListener('pointermove', e => {
    const sp = local(e), at = toWorld(view, sp);
    if (e.pointerType === 'touch' && touches.has(e.pointerId)) {
      touches.set(e.pointerId, sp);
      if (pinch && touches.size === 2) {
        const [a, b] = [...touches.values()] as [Pt, Pt], dist = Math.hypot(a[0] - b[0], a[1] - b[1]), mid: Pt = [(a[0] + b[0]) / 2, (a[1] + b[1]) / 2];
        view = pan(zoomAt(pinch.view, pinch.mid, dist / Math.max(1, pinch.d)), mid[0] - pinch.mid[0], mid[1] - pinch.mid[1]); render(); return;
      }
    }
    hover = at;
    if (drag?.kind === 'pan') { view = pan(view, sp[0] - drag.last[0], sp[1] - drag.last[1]); drag.last = sp; render(); return; }
    if (drag?.kind === 'box') { drag.at = at; renderLive(); return; }
    if (drag?.kind === 'points') {
      const moved = Math.hypot(at[0] - drag.at[0], at[1] - drag.at[1]) * view.scale;
      if (!drag.moved && moved < 3) return;
      drag.moved = true;
      snap = snapAt([drag.start[0] + at[0] - drag.at[0], drag.start[1] + at[1] - drag.at[1]], drag.ids, drag.anchor);
      void dragPreview(drag);
      return;
    }
    snap = tool === 'select' ? null : snapAt(at, [], steps.at(-1)?.at ?? null);
    renderLive();
    // The pointer shape asks the Rust hit test at most once per frame.
    if (tool === 'select' && !cursorFrame) cursorFrame = window.requestAnimationFrame(() => { cursorFrame = 0; svg.style.cursor = pick(at) ? 'pointer' : 'default'; });
  });
  /** Live preview while dragging: the engine moves the points (formulas kept); failures show the last good shape. */
  async function dragPreview(dr: Extract<Drag, { kind: 'points' }>): Promise<void> {
    const ticket = ++previewTicket, e = host.engine(), d = doc(), base = ins; if (!e || !d || !base || !snap) return;
    const dx = snap.at[0] - dr.start[0], dy = snap.at[1] - dr.start[1];
    try {
      const next = await e.edit(d, dr.ids.map(id => { const p = base.objects[id]!.points[0]!; return { op: 'move_point' as const, id, size, target: [p[0] + dx, p[1] + dy] as Pt }; }));
      const look = await e.inspect(next, size);
      if (ticket === previewTicket && drag === dr) { preview = look; render(); }
    } catch { if (ticket === previewTicket) renderLive(); }
  }
  async function endDrag(e: PointerEvent, cancel: boolean): Promise<void> {
    touches.delete(e.pointerId); if (touches.size < 2) pinch = null;
    const dr = drag; drag = undefined; if (!dr) return;
    if (dr.kind === 'box' && !cancel) {
      const g = geometry(), hits = new Set<Sel>(dr.additive ? selection : []);
      for (const p of pointTargets()) if (inBox(p.at, dr.start, dr.at)) hits.add(`pt:${p.id}`);
      if (!hits.size || (hits.size === selection.size && dr.additive)) for (const p of g?.pieces ?? []) if (p.graded && p.graded.piece.outline.every(q => inBox(q, dr.start, dr.at))) hits.add(`piece:${p.id}`);
      if (Math.hypot(dr.at[0] - dr.start[0], dr.at[1] - dr.start[1]) * view.scale < 4) { const hit = pick(dr.start); selection = hit ? new Set([hit]) : dr.additive ? selection : new Set(); }
      else selection = hits;
      render(); if (selection.size) host.say(t('n_selected', { n: selection.size }));
      return;
    }
    if (dr.kind === 'points' && dr.moved && !cancel && snap && ins) {
      const dx = snap.at[0] - dr.start[0], dy = snap.at[1] - dr.start[1], base = ins;
      previewTicket++; snap = null;
      const ok = await commit(dr.ids.map(id => { const p = base.objects[id]!.points[0]!; return { op: 'move_point' as const, id, size, target: [p[0] + dx, p[1] + dy] as Pt }; }), t('done_move'));
      if (!ok) { preview = null; render(); }
      return;
    }
    if (dr.kind === 'points') { preview = null; render(); }
  }
  svg.addEventListener('pointerup', e => void endDrag(e, false));
  svg.addEventListener('pointercancel', e => void endDrag(e, true));
  svg.addEventListener('pointerleave', () => { if (!drag) { hover = null; snap = null; renderLive(); } });

  // ---------- Keyboard ----------
  // Keys work anywhere on the Pattern step (not only when the drawing has focus), but not inside other dialogs or menus.
  document.addEventListener('keydown', e => {
    const target = e.target as HTMLElement;
    if (!root.offsetParent || (target !== document.body && !root.contains(target)) || e.defaultPrevented) return;
    const typing = !!target.closest('input, select, textarea');
    if ((e.ctrlKey || e.metaKey) && !typing && ['z', 'y'].includes(e.key.toLowerCase())) { e.preventDefault(); if (e.key.toLowerCase() === 'y' || e.shiftKey) host.redo(); else host.undo(); return; }
    if (e.key === 'Escape') {
      if (!pop.hidden) { pop.querySelector<HTMLButtonElement>('#pop-cancel')?.click(); e.preventDefault(); return; }
      if (steps.length || chain.length || measure) { steps = []; chain = []; measure = null; hint(); renderLive(); e.preventDefault(); return; }
      if (tool !== 'select') { setTool('select'); e.preventDefault(); return; }
      if (selection.size) { selection.clear(); render(); e.preventDefault(); }
      return;
    }
    if (typing) return;
    if (e.key === ' ') { panKey = true; svg.style.cursor = 'grab'; e.preventDefault(); return; }
    if (e.ctrlKey || e.metaKey || e.altKey) return;
    const spec = TOOLS.find(x => x.key.toLowerCase() === e.key.toLowerCase());
    if (spec) { setTool(spec.id); e.preventDefault(); return; }
    if (e.key === 'Enter' && tool === 'line') { steps = []; chain = []; closePop(); hint(); renderLive(); e.preventDefault(); return; }
    if (/^[0-9.]$/.test(e.key) && tool === 'line' && steps.length && !pop.hidden) { const li = pop.querySelector<HTMLInputElement>('#pop-length')!; li.focus(); li.value = ''; return; }
    if (e.key === 'Delete' || e.key === 'Backspace') { e.preventDefault(); void deleteSelection(); return; }
    if (e.key === '+' || e.key === '=') { e.preventDefault(); const [w, h] = stageSize(); view = zoomAt(view, [w / 2, h / 2], 1.25); render(); return; }
    if (e.key === '-') { e.preventDefault(); const [w, h] = stageSize(); view = zoomAt(view, [w / 2, h / 2], 0.8); render(); return; }
    if (e.key === '0') { e.preventDefault(); fit(); return; }
    const arrows: Record<string, Pt> = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, 1], ArrowDown: [0, -1] };
    const a = arrows[e.key];
    if (a && (target as Element) === svg) { e.preventDefault(); const k = e.shiftKey ? 10 : 1; void nudge(a[0] * k, a[1] * k); }
  });
  document.addEventListener('keyup', e => { if (e.key === ' ') { panKey = false; svg.style.cursor = ''; } });
  root.addEventListener('click', e => {
    const b = (e.target as Element).closest<HTMLButtonElement>('button'); if (!b) return;
    if (b.dataset.tool) { setTool(b.dataset.tool as ToolId); svg.focus({ preventScroll: true }); return; }
    if (b.closest('#sc-inspect') || b.dataset.tab) { void inspectorClick(b); return; }
    switch (b.id) {
      case 'sc-undo': host.undo(); break; case 'sc-redo': host.redo(); break;
      case 'sc-zoom-in': { const [w, h] = stageSize(); view = zoomAt(view, [w / 2, h / 2], 1.25); render(); break; }
      case 'sc-zoom-out': { const [w, h] = stageSize(); view = zoomAt(view, [w / 2, h / 2], 0.8); render(); break; }
      case 'sc-fit': fit(); break;
      case 'sc-rect': case 'sc-rect-empty': setTool('select'); steps = []; openPop('rect'); break;
      case 'sc-snap-points': snapPoints = !snapPoints; b.setAttribute('aria-pressed', String(snapPoints)); break;
      case 'sc-snap-grid': snapGrid = !snapGrid; b.setAttribute('aria-pressed', String(snapGrid)); break;
      case 'sc-snap-angle': snapAngle = !snapAngle; b.setAttribute('aria-pressed', String(snapAngle)); break;
      case 'sc-all': showAll = !showAll; b.setAttribute('aria-pressed', String(showAll)); lastDoc = ''; void reload(); break;
      case 'sc-names': showNames = !showNames; b.setAttribute('aria-pressed', String(showNames)); render(); break;
      case 'sc-arrange': void arrange(); break;
    }
  });
  root.addEventListener('change', e => {
    const target = e.target as HTMLInputElement | HTMLSelectElement;
    if (target.id === 'sc-size') { size = target.value; lastDoc = ''; void reload(); return; }
    if (target.closest('#sc-inspect')) void inspectorChange(target);
  });
  new ResizeObserver(() => { if (!fitted && ins && stageSize()[0] > 80) { fit(); fitted = true; } render(); }).observe($('#sc-stage'));

  async function arrange(): Promise<void> {
    const d = doc(), g = ins; if (!d || !g) return;
    const ops = arrangeOps(d, g);
    if (!ops.length) { host.say(t('arrange_none')); return; }
    if (await commit(ops, t('arranged'))) { fitted = false; lastDoc = ''; await reload(); }
  }
  function fit(): void {
    const g = geometry(), [w, h] = stageSize();
    const points = [...Object.values(g?.objects ?? {}).flatMap(o => o.points), ...(g?.pieces ?? []).flatMap(p => p.graded?.piece.outline ?? [])];
    view = fitView(boundsOf(points), w, h, 48); render();
  }
  setTool('select');
  return {
    refresh: () => { void reload(); },
    /** The document changed outside the canvas (block chosen, file opened, undo). */
    reset: () => { fitted = false; lastDoc = ''; selection.clear(); setTool('select'); void reload(); },
    size: () => size,
    arrange,
    focus: () => svg.focus({ preventScroll: true }),
    setTool,
  };
}
