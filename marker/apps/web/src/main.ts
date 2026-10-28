import { apiFetch, downloadFile, isDesktop } from './transport';
import { installDesktop } from '../../desktop/src/native';
import { nest as nestInBrowser, loadMarkerCore, type ImportReport, type Issue, type Job, type MarkerCore, type MarkerDrawing, type MarkerResult, type Nap, type Pt, type RotationRule, type ValidationReport } from '@beta/marker-wasm';
import { bbox, placed } from '@beta/marker-wasm/geometry';
import { demoJob, demoResult } from './demo';
import { changePlacement, emptyResult, History, readDocument, type DocumentState } from './state';
import { fitReadable, fitView, markerPoint, rulerTicks, screenPoint, zoomAt, type View } from './view';
import { installFeatures } from './features';
import { dxfImportPath, issueExplanation, nestPayload } from './workflows';
import { FOLD_EDGE_PREFIX, FOLD_EDGE_TEXT, FOOTPRINT_TIP, markerYield, YIELD_TIP } from './yield';
import { splitNotes, technicalDetailsHtml } from './engine-notes';
import { loadEditorCore, type EditorCore } from './editor-core';
import { cleanSelection } from './editor-state';
import { installEditorTools } from './editor-tools';
import { seamlyWarnings, warningsHtml } from './import-warnings';
import { installShell, SCREENS } from './shell';
import { icon } from './icons';
import { staticMode, installStaticNotices } from './static-mode';
import { label } from './floor-i18n';
import type { FloorLanguage } from './floor-types';
import './style.css';
// Screen styles after the design system so they can refine it.
import './quick/quick.css';
import './style/style.css';
import './tour/tour.css';
import './launch.css';
import { find, findAll, whenIdle } from './screens';
import { appUnit, longLength, onUnit, shortLength, shortUnit } from './units-pref';
import { labelsSvg, placedLabels, prepareLabels } from './marker-labels';
import { bySize } from './sizes';
import { bootStep } from './boot';

bootStep(70, 'Starting the app…');
const app = find<HTMLDivElement>('#app')!;
const tool = (id: string, name: string, label: string, tip: string, keys = '', extra = ''): string =>
  `<button type="button" id="${id}" class="icon-btn" aria-label="${label}"${keys ? ` aria-keyshortcuts="${keys}"` : ''} data-tip="${tip}" ${extra}>${icon(name)}<span class="btn-label">${label}</span></button>`;
// Layout: command bar (files) → Setup panel (left, Nest docked at the bottom) → toolbar, canvas and status bar → Tools panel (right).
app.innerHTML = `
<header class="topbar"><span id="server-status" class="connection" role="status">Checking local server…</span></header>
<div class="filebar commandbar" id="editor-filebar" role="group" aria-label="Marker files">
  <button id="open-button">${icon('open')}Open file</button><input id="open-file" type="file" accept=".dxf,.json" hidden>
  <button id="demo">${icon('demo')}Load demo</button><span class="file-hint">DXF pieces, marker job JSON or marker result JSON</span>
  <div class="push"></div>
  <details class="menu" id="save-menu"><summary>${icon('save')}Save</summary><div class="menu-panel" role="group" aria-label="Save marker files">
    <button id="save-job">Save job JSON<small>Pieces and rules</small></button><button id="save-result">Save result JSON<small>Placements</small></button></div></details>
  <details class="menu" id="export-menu"><summary>${icon('download')}Export</summary><div class="menu-panel export-panel" role="group" aria-label="Export marker">
    <p class="hint">SVG, plotter (PLT), PDF and cutter files are made by the local Rust engine.</p>
    <span class="menu-label">Plotter and cutter</span><button data-export="plt">PLT plotter file</button><button data-export="iso">ISO cutter file</button><button data-export="dxf">DXF marker</button>
    <span class="menu-label">Print and share</span><button data-export="pdf">PDF, full size</button><button data-export="mini">Mini-marker PDF</button><button data-export="labels">Piece labels PDF</button><button data-export="svg">SVG picture</button>
    <span class="menu-label">Reports</span><button data-export="report">Marker report (text)</button><button data-export="xlsx">Excel report (XLSX)</button></div></details>
</div>
<div id="message" class="message" role="status" aria-live="polite" hidden></div>
<dialog id="dxf-dialog" aria-labelledby="dxf-title"><form method="dialog" class="dialog-body">
  <h2 id="dxf-title">Open DXF pieces</h2><p id="dxf-file" class="hint"></p>
  <fieldset><legend>A piece says "Cut 2". What does it mean?</legend>
    <label class="check"><input type="radio" name="dxf-pairs" value="pair" checked>A left and a right piece (a pair) · usual</label>
    <label class="check"><input type="radio" name="dxf-pairs" value="same">Two pieces the same way up</label><p class="hint">Pieces that are their own mirror (for example cuffs) are never split.</p></fieldset>
  <label>Units in the file<select id="dxf-units"><option value="auto">Find out from the file</option><option value="mm">Millimetres</option><option value="cm">Centimetres</option><option value="inch">Inches</option></select></label>
  <div class="dialog-actions"><button type="submit" value="cancel">Cancel</button><button type="submit" value="open" class="primary" id="dxf-open">Open pieces</button></div></form></dialog>
<div class="workspace">
<aside aria-label="Job settings" class="setup-panel" id="setup-panel">
  <div class="panel-heading"><span class="eyebrow">Marker setup</span><h1 id="job-title"></h1></div>
  <form id="job-form"></form>
  <details class="import-settings advanced"><summary>DXF import options</summary><label>Source units<select id="units"><option value="auto">Detect automatically</option><option value="mm">Millimetres</option><option value="cm">Centimetres</option><option value="inch">Inches</option></select></label><label class="check"><input id="pairs" type="checkbox" checked>Even quantities are left/right pairs</label><p class="hint">Splits an even quantity into left and right pieces: Quantity 2 becomes 1 normal + 1 mirrored copy.</p></details>
  <form id="nest-form" class="nest-dock"><div class="nest-row"><label class="nest-time">Time limit (seconds)<input id="time" type="number" value="30" min="0.1" step="0.1" required></label><button class="primary large" id="nest" type="submit">${icon('play')}<span class="btn-label">Nest marker</span></button></div>
    <details class="advanced"><summary>More nesting options</summary><div class="fields"><label>Seed<input id="seed" type="number" value="42" min="0" max="9007199254740991" step="1" required></label><label>Portfolio runs<input id="runs" type="number" value="1" min="1" max="100" step="1" required></label></div><label class="check"><input id="warm-start" type="checkbox">Start from current marker</label><button id="renest" type="submit" class="wide">${icon('lock')}Re-nest unlocked pieces</button><p class="hint">Rust nesting runs in this browser when no server is connected. Lock pieces with L, then re-nest around them.</p></details></form>
</aside>
<div class="editor-main">
  <h2 id="marker-title" class="sr-only">Marker layout</h2>
  <section id="import-warnings" class="notice warn" aria-labelledby="import-warnings-heading" hidden></section>
  <div class="canvas-card">
    <div class="toolbar" role="group" aria-label="Marker tools">
      ${tool('toggle-setup', 'panel-left', 'Setup panel', 'Show or hide the setup panel', '', 'aria-controls="setup-panel" aria-pressed="true" data-tip-align="start"')}
      <div class="toolgroup" role="group" aria-label="History">${tool('undo', 'undo', 'Undo', 'Undo · Ctrl+Z', 'Control+Z')}${tool('redo', 'redo', 'Redo', 'Redo · Ctrl+Y', 'Control+Y')}</div>
      <div class="toolgroup segmented" id="tool-modes" role="group" aria-label="Pointer mode"></div>
      <div class="toolgroup" id="tool-actions" role="group" aria-label="Piece actions">${tool('rotate', 'rotate', 'Rotate', 'Next allowed rotation · R', 'R')}${tool('lock', 'lock', 'Lock', 'Lock or unlock · L', 'L')}</div>
      <div class="toolgroup" id="tool-tidy" role="group" aria-label="Tidy the marker"></div>
      <div class="push"></div>
      <label class="tool-select colour-by">${icon('swatches')}<span class="label-text">Colour by</span><select id="colour-mode" aria-label="Colour pieces by"><option value="size">Size</option><option value="bundle">Bundle</option></select></label>
      <div class="toolgroup" role="group" aria-label="Zoom">${tool('zoom-out', 'zoom-out', 'Zoom out', 'Zoom out · −', '-')}${tool('fit', 'fit', 'Fit', 'Fit marker · 0', '0')}${tool('zoom-in', 'zoom-in', 'Zoom in', 'Zoom in · +', '+')}</div>
      ${tool('toggle-inspector', 'panel-right', 'Tools panel', 'Show or hide the tools panel', '', 'aria-controls="inspector" aria-pressed="false" data-tip-align="end"')}
    </div>
    <div class="canvas-stage">
      <svg id="canvas" role="group" aria-label="Marker. Tab to select pieces. Arrows nudge the selected group. Alt plus arrows bump. R rotates, L locks. Question mark opens keyboard help." tabindex="0"></svg>
      <div id="canvas-empty" class="canvas-empty" hidden>${icon('marker')}<strong>No pieces on the marker yet</strong><span>Check the fabric width and size ratio in Setup, then press <b>Nest marker</b>. You can also place pieces by hand from the piece bin.</span></div>
      <div id="nest-progress" class="nest-progress" aria-label="Nesting progress" role="status" aria-live="polite" hidden><div class="nest-progress-row"><span id="nest-progress-time">Starting…</span><button type="button" id="nest-cancel" class="small" hidden>${icon('stop')}Cancel search</button></div><progress id="nest-progress-bar" max="1" value="0" aria-label="Search time used"></progress><div class="nest-progress-row"><span>Best so far</span><strong id="nest-progress-best">No valid marker yet</strong></div><span id="nest-cancel-note" class="hint" hidden>The search stops at the time limit.</span></div>
    </div>
    <div class="statusbar">
      <section id="stats" class="stats" aria-label="Marker statistics"></section>
      <div class="status-chips"><a class="check-link" href="#validation-heading"><span id="check-status" class="tag"></span></a><a class="check-link" href="#ed-supply" id="ed-unplaced-chip" hidden></a><span id="engine" class="tag engine-tag"></span></div>
    </div>
    <div class="canvas-footer"><span id="selection">Select a piece to edit</span><div id="legend" class="legend" aria-label="Piece sizes"></div><span id="fabric-label"></span><span class="ruler-label"><span id="ruler-name">Ruler in metres</span> · <span id="zoom-label"></span></span></div>
  </div>
  <div class="under-canvas"><p>Wheel to zoom · drag to move or pan · Alt+arrows: bump · <kbd>?</kbd> all shortcuts</p></div>
  <section class="validation-panel" aria-labelledby="validation-heading"><div class="validation-heading"><h2 id="validation-heading">Live checks</h2></div><p id="wasm-status" role="status">Loading local Rust WASM…</p><ul id="issues" aria-live="polite"></ul></section>
  <p class="help">Marker efficiency = piece area / (width × length). Gaps and buffers do not count as piece area. Lengths are shown in the unit chosen in the top bar (cm or inch); saved files always use millimetres.</p>
</div>
<aside id="inspector" class="inspector" aria-labelledby="inspector-title">
  <div class="inspector-head"><h2 id="inspector-title">Tools</h2>${tool('inspector-close', 'close', 'Hide tools panel', 'Hide tools panel', '', 'data-tip-align="end"')}</div>
  <section class="inspector-section" aria-labelledby="inspector-selected"><h3 id="inspector-selected">Selected</h3><div id="inspector-summary"></div>
    <div class="inspector-row">${tool('pack', 'compact', 'Pack left', 'Bump the selection left to contact')}</div></section>
  <div id="inspector-tools"></div>
</aside></div>`;

const $ = <T extends Element = HTMLElement>(selector: string): T => find<T>(selector)!;
$('#warm-start').closest('label')!.insertAdjacentHTML('afterend', `<label class="check"><input id="adaptive-search" type="checkbox" ${staticMode() ? 'disabled' : ''}>Compare search methods (experimental)</label><p class="hint">Strategy comparison needs the desktop app or local server and at least 12 seconds. Ordinary search remains the default.</p>`);
const canvas = $<SVGSVGElement>('#canvas');
let state: DocumentState = structuredClone({ job: demoJob, result: demoResult });
let history = new History(state);
let selected = -1;
let selection: number[] = [];
let editorCore: EditorCore | undefined;
let editorTools: ReturnType<typeof installEditorTools> | undefined;
let core: MarkerCore | undefined;
let drawing: MarkerDrawing | undefined;
let report: ValidationReport | undefined;
let jobIssues: Issue[] = [];
let wasmProblem = '';
let dirtyWithoutChecks = false;
let nesting = false;
let serverOnline = false;
let browserStop: AbortController | undefined;
let exporting = false;
let features: ReturnType<typeof installFeatures> | undefined;
let view: View = { x: 0, y: 0, width: 2000, height: 1500 };
const colours = [0, 1, 2, 3, 4, 5].map(i => `var(--fill-${i})`);
const esc = (value: unknown): string => String(value ?? '').replace(/[&<>"']/g, char => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[char]!));
const errorText = (error: unknown): string => error instanceof Error ? error.message : String(error);
function message(text: string, error = false): void {
  const element = $('#message'); element.hidden = !text; element.textContent = text; element.classList.toggle('error', error);
}
function sizes(): string[] { return [...new Set([...Object.keys(state.job.order?.sizes ?? {}), ...state.job.pieces.flatMap(p => p.size ? [p.size] : [])])].sort(bySize); }
function colour(size: string | null | undefined): string { return colours[Math.max(0, sizes().indexOf(size ?? '')) % colours.length]!; }
function outline(p: MarkerResult['placements'][number], index: number): Pt[] {
  if (drawing?.outlines[index]) return drawing.outlines[index]!;
  return placed(state.job.pieces.find(piece => piece.id === p.piece_id)?.outline ?? [], p);
}
function displayLength(): number {
  return Math.max(100, editorTools?.inspection()?.table_length ?? 0, ...state.result.placements.map((p,i) => bbox(outline(p,i))?.[2] ?? 0));
}
let fitPending = false, wholeNext = false;
/** Fit the marker. A long marker first fits readable; the Fit button (or 0) pressed again shows the whole length. */
function fit(toggle = false): void {
  const rect = canvas.getBoundingClientRect();
  // A hidden editor has no size yet; fit when it is first shown.
  if (!rect.width || !rect.height) { fitPending = true; return; }
  fitPending = false;
  const aspect = Math.max(0.3, rect.width / Math.max(1, rect.height));
  const whole = fitView(displayLength(), state.job.fabric.width, aspect), readable = fitReadable(displayLength(), state.job.fabric.width, aspect);
  const showWhole = toggle && wholeNext && readable.width < whole.width;
  view = showWhole ? whole : readable; wholeNext = !showWhole && readable.width < whole.width;
  renderView();
}
/** Long lengths (marker, per garment) in the unit chosen in the top bar: metres for cm, yards for inch. */
function longText(mm: number): string { const v = longLength(mm); return `${v.value}<small>${v.unit}</small>`; }
/** A position or distance on the marker in cm or inch. */
const at = (mm: number) => shortLength(mm, appUnit(), false);
/** Common fabric widths (sold by the inch). On a folded or tube lay the marker uses the width on the table (half). */
function widthPresets(): string {
  const folded = state.job.fabric.layout === 'folded' || state.job.fabric.layout === 'tubular';
  const chips = [36, 44, 58, 60].map(inch => { const mm = inch * 25.4 / (folded ? 2 : 1); return `<button type="button" class="small" data-width-mm="${mm}" aria-pressed="${Math.abs(mm - state.job.fabric.width) < 0.5}">${inch}″${appUnit() === 'cm' ? ` <small>${shortLength(inch * 25.4)}</small>` : ''}</button>`; }).join('');
  const table = folded ? `<p class="hint">Folded lay: the width on the table is ${shortLength(state.job.fabric.width)}${'' } (open width ${shortLength(state.job.fabric.width * 2)}). Type the folded width, or choose the open width below.</p>` : '';
  return `<div class="width-presets" role="group" aria-label="Common fabric widths${folded ? ' (folded to half on the table)' : ''}">${chips}</div>${table}`;
}
function numberField(label: string, field: string, value: number, min = 0, step = 'any'): string {
  return `<label>${label}<input data-field="${field}" type="number" value="${esc(value)}" min="${min}" max="${step === '1' ? 100000 : 100000000}" step="${step}" required></label>`;
}
function renderForm(): void {
  const openPieces = Array.from($('#job-form').querySelectorAll('details'), element => element.open);
  const active = document.activeElement as HTMLInputElement | HTMLSelectElement | null;
  const activeField = active?.dataset.field;
  const activePiece = active?.dataset.piece;
  const activeKey = active?.dataset.key;
  const { job } = state;
  $('#job-title').textContent = job.name;
  $('#marker-title').textContent = job.name;
  $('#job-form').innerHTML = `
    <section class="setup-step"><h2 class="section-title"><span class="step-badge" aria-hidden="true">1</span>Fabric and gaps</h2>${widthPresets()}<div class="fields two">${numberField('Fabric width (mm)', 'width', job.fabric.width, 1)}
    <label>Nap<select data-field="nap"><option value="none" ${(job.fabric.nap ?? 'none') === 'none' ? 'selected' : ''}>None</option><option value="one_way_per_bundle" ${job.fabric.nap === 'one_way_per_bundle' ? 'selected' : ''}>One way per garment</option><option value="one_way" ${job.fabric.nap === 'one_way' ? 'selected' : ''}>One-way</option></select></label>
    ${numberField('Piece gap (mm)', 'piece_gap', job.spacing?.piece_gap ?? 0)}${numberField('Edge gap (mm)', 'edge_gap', job.spacing?.edge_gap ?? 0)}</div>
    <details class="advanced"><summary>More fabric options</summary><div class="fields two">${numberField('Tilt (degrees)', 'tilt', job.fabric.tilt_deg ?? 0)}</div><p class="hint">Fold, shrinkage, shade bands, stripes and defects are in Fabric rules.</p></details></section>
    <section class="setup-step"><h2 class="section-title"><span class="step-badge" aria-hidden="true">2</span>Size ratio <span class="small">garments per marker</span></h2><div class="ratio">${sizes().map(size => numberField(esc(size), `ratio:${esc(size)}`, job.order ? job.order.sizes[size] ?? 0 : 1, 0, '1')).join('') || '<p class="hint">No sizes in this job. Each piece uses its own quantity.</p>'}</div></section>
    <section class="setup-step"><h2 class="section-title"><span class="step-badge" aria-hidden="true">3</span>Pieces <span class="small">${job.pieces.length} shapes</span></h2><div class="piece-list">${job.pieces.map((piece, i) => `
      <details><summary><span class="swatch" style="background:${colour(piece.size)}"></span><span>${esc(piece.name)}</span><span class="piece-count">${esc(piece.size || 'No size')} · ${piece.quantity ?? 1}+${piece.mirror_quantity ?? 0}</span></summary>
      <div class="fields piece-fields"><label>Name<input data-piece="${i}" data-key="name" value="${esc(piece.name)}" required></label><label>Size<input data-piece="${i}" data-key="size" value="${esc(piece.size)}"></label>
      <label>Quantity<input data-piece="${i}" data-key="quantity" type="number" value="${piece.quantity ?? 1}" min="0" max="100000" step="1" required></label><label>Mirror quantity<input data-piece="${i}" data-key="mirror_quantity" type="number" value="${piece.mirror_quantity ?? 0}" min="0" max="100000" step="1" required></label>
      <label>Rotation<select data-piece="${i}" data-key="rotation">${(['inherit', 'fixed', 'flip180', 'quarter', 'free'] as const).map(rule => `<option value="${rule}" ${(piece.rotation ?? 'inherit') === rule ? 'selected' : ''}>${({ inherit: 'Fabric rule', fixed: 'Fixed', flip180: '0° / 180°', quarter: 'Quarter turn', free: 'Any angle' })[rule]}</option>`).join('')}</select></label>
      <label>Buffer (mm)<input data-piece="${i}" data-key="buffer" type="number" value="${piece.buffer ?? 0}" min="0" step="any" required></label></div></details>`).join('')}</div></section>`;
  $('#job-form').querySelectorAll('details').forEach((element, i) => element.open = openPieces[i] ?? false);
  const selector = activeField ? `[data-field="${CSS.escape(activeField)}"]` : activePiece !== undefined && activeKey ? `[data-piece="${Number(activePiece)}"][data-key="${CSS.escape(activeKey)}"]` : '';
  if (selector) $('#job-form').querySelector<HTMLElement>(selector)?.focus({ preventScroll: true });
}
const notNestedYet = () => state.result.placements.length === 0;
function refreshChecks(): void {
  report = undefined; jobIssues = []; drawing = undefined;
  if (!core) return;
  try {
    jobIssues = core.check_job(state.job);
    drawing = core.drawing(state.job, state.result.placements);
    const metrics = core.metrics(state.job, state.result.placements);
    state.result = { ...state.result, ...metrics, width: state.job.fabric.width };
    report = core.validate(state.job, state.result);
    dirtyWithoutChecks = false; wasmProblem = '';
  } catch (error) { dirtyWithoutChecks = true; wasmProblem = `Live checks failed: ${errorText(error)}`; }
}
/** Footprint efficiency, useful yield and surplus copies, read from Rust (validator report first). */
const yieldNow = () => markerYield(state.job.fabric.layout, state.result, report);
function renderStats(): void {
  let required: string | number = '—';
  let cut: string | number = '—';
  if (drawing) cut = drawing.cut_copies;
  try { if (core) required = core.demands(state.job).reduce((count, demand) => count + demand.count, 0); } catch { /* shown in live checks */ }
  const garments = Object.values(state.job.order?.sizes ?? {}).reduce((sum, count) => sum + count, 0);
  const usable = !dirtyWithoutChecks, y = yieldNow();
  const values: string[][] = [
    ['Marker length', usable ? longText(state.result.length) : '—'],
    // Folded and tube lays: useful yield (fabric that ends up in garments, both layers) is the headline; the footprint
    // (table area the pieces cover, unused copies included) stays as a smaller second figure (fold7).
    ...(usable && y.headline === 'yield'
      ? [[`<abbr title="${YIELD_TIP}">Yield</abbr>`, `${(y.usefulYield! * 100).toFixed(1)}<small>%</small>`], [`<abbr title="${FOOTPRINT_TIP}">Footprint</abbr>`, `${(y.efficiency * 100).toFixed(1)}<small>%</small>`, 'secondary']]
      : [['Efficiency', usable ? `${(state.result.efficiency * 100).toFixed(1)}<small>%</small>` : '—']]),
    ['Copies cut / required', `${cut}<small>/ ${required}</small>`],
    ['Per garment', usable && garments > 0 ? longText(state.result.length / garments) : '—'],
  ];
  $('#stats').innerHTML = values.map(([label, value, cls]) => `<div${cls ? ` class="${cls}"` : ''}><span>${label}</span><strong>${value}</strong></div>`).join('');
  $('#engine').textContent = state.result.engine;
  const imported = seamlyWarnings(state.job.meta);
  $('#import-warnings').hidden = !imported.length;
  $('#import-warnings').innerHTML = warningsHtml(imported, `This marker was made from the Seamly2D pattern "${String(state.job.meta?.seamly_source_pattern ?? 'unknown')}". These import warnings are still open. Check each piece before cutting. Fix them in Pattern and send the pattern again to clear this list.`, 'import-warnings-heading', esc);
  const editInspection = editorTools?.inspection();
  const editorIssues: Issue[] = editInspection && !editInspection.geometry_ok ? [{code:'editor_geometry',message:'Resolve the clearance, cut-line or table-end checks below before cutting.',piece_ids:[]}] : [];
  const errors = [...jobIssues, ...report?.errors ?? [], ...editorIssues].filter(issue => !(notNestedYet() && issue.code === 'missing_piece')).filter((issue, i, all) =>
    all.findIndex(other => other.code === issue.code && other.message === issue.message && JSON.stringify(other.piece_ids) === JSON.stringify(issue.piece_ids)) === i);
  const warnings = [...report?.warnings ?? []];
  // Before nesting, "missing piece" is expected (nothing is placed yet). Codes stay in a tooltip, not in the text.
  const shown = notNestedYet() ? [...errors, ...warnings].filter(issue => issue.code !== 'missing_piece') : [...errors, ...warnings];
  // Search log lines (passes, timings) go under a closed "Technical details"; cutting-room notes stay visible.
  const engineNotes = splitNotes(state.result.warnings);
  $('#issues').innerHTML = shown.map(issue => `<li class="${errors.includes(issue) ? 'issue-error' : ''}" title="${esc(issue.code)}">${esc(issueExplanation(issue.code))} ${esc(issue.message)}</li>`).join('') + engineNotes.shown.map(warning => warning.startsWith(FOLD_EDGE_PREFIX) ? `<li class="fold-edge"><strong>${esc(FOLD_EDGE_TEXT)}</strong> <span class="hint">${esc(warning)}</span></li>` : `<li>${esc(warning)}</li>`).join('')
    + (engineNotes.technical.length ? `<li class="tech-notes-item">${technicalDetailsHtml(engineNotes.technical, esc)}</li>` : '');
  const unavailable = !core || Boolean(wasmProblem);
  $('#wasm-status').textContent = unavailable ? (wasmProblem || 'WASM is unavailable. The viewer and manual moves still work. Build @beta/marker-wasm and reload to enable live checks, rotations and packing.') : 'Rust WASM is active. Every edit recomputes metrics and runs the independent validator.';
  const surplus = y.surplus;
  const notNested = state.result.placements.length === 0;
  $('#check-status').textContent = unavailable ? 'Checks unavailable' : notNested ? `${required === '—' ? '' : `${required} `}piece${required === 1 ? '' : 's'} ready to nest` : errors.length ? `${errors.length} issue${errors.length === 1 ? '' : 's'}` : surplus ? `${surplus} piece${surplus === 1 ? '' : 's'} cut but not used` : 'Checks passed';
  $('#check-status').classList.toggle('bad', !notNested && (unavailable || errors.length > 0));
  $('#check-status').classList.toggle('warn', !notNested && !errors.length && surplus > 0);
  $('#check-status').classList.toggle('neutral', notNested && !unavailable);
}
function renderSelection(): void {
  selection = cleanSelection(selection, state.result.placements.length);
  if (!selection.includes(selected)) selected = selection[0] ?? -1;
  const p = state.result.placements[selected];
  canvas.querySelectorAll<SVGGElement>('[data-index]').forEach(element => {
    const active = selection.includes(Number(element.dataset.index));
    element.classList.toggle('selected', active); element.setAttribute('aria-pressed', String(active));
  });
  $('#selection').textContent = p ? `${state.job.pieces.find(piece => piece.id === p.piece_id)?.name ?? p.piece_id} · ${p.translate.map(at).join(', ')} ${shortUnit()} · ${p.rotation_deg.toFixed(1)}°${p.mirrored ? ' · mirrored' : ''}${p.locked ? ' · locked' : ''}` : 'Select a piece to edit';
  $<HTMLButtonElement>('#rotate').disabled = !p || !!p.locked || !core || nesting;
  $<HTMLButtonElement>('#pack').disabled = !p || !!p.locked || !core || nesting;
  $<HTMLButtonElement>('#lock').disabled = !p || nesting;
  const lockLabel = p?.locked ? 'Unlock' : 'Lock';
  $('#lock').innerHTML = `${icon(p?.locked ? 'unlock' : 'lock')}<span class="btn-label">${lockLabel}</span>`; $('#lock').setAttribute('aria-label', lockLabel);
  renderInspector(p);
  $<HTMLButtonElement>('#undo').disabled = !history.canUndo || nesting;
  $<HTMLButtonElement>('#redo').disabled = !history.canRedo || nesting;
  editorTools?.selection();
}
/** Tools panel summary of the selected placements. Read-only; editing stays in the tools and keyboard shortcuts. */
function renderInspector(p: MarkerResult['placements'][number] | undefined): void {
  const box = $('#inspector-summary');
  if (!p) { box.innerHTML = '<p class="hint">Click a piece on the marker, or press Tab on the marker to reach one. Shift+click adds more pieces.</p>'; return; }
  const piece = state.job.pieces.find(piece => piece.id === p.piece_id);
  const more = selection.length > 1 ? `<p class="inspector-more">${selection.length} pieces selected. Tools act on the whole group.</p>` : '';
  const row = (label: string, value: string) => `<div><dt>${label}</dt><dd>${value}</dd></div>`;
  box.innerHTML = `<p class="inspector-piece"><span class="swatch" style="background:${colour(piece?.size)}"></span><strong>${esc(piece?.name ?? p.piece_id)}</strong><span class="chip">${esc(piece?.size || 'No size')}</span></p>${more}
    <dl class="inspector-facts">${row('Position', `${p.translate.map(at).join(', ')} ${shortUnit()}`)}${row('Rotation', `${p.rotation_deg.toFixed(1)}°`)}${row('Copy', String(p.copy + 1))}${row('Bundle', p.bundle == null ? 'Not assigned' : String(p.bundle + 1))}${row('Mirrored', p.mirrored ? 'Yes' : 'No')}${row('Locked', p.locked ? 'Yes' : 'No')}</dl>`;
}
/** Setup and Tools panels can be hidden to give the canvas more room. The choice is remembered on this computer. */
const PANELS_KEY = 'beta-editor-panels';
function setPanel(name: 'setup' | 'inspector', open: boolean, remember = true): void {
  $('.workspace').classList.toggle(`hide-${name}`, !open);
  $(`#toggle-${name}`).setAttribute('aria-pressed', String(open));
  if (remember) try { const saved = JSON.parse(localStorage.getItem(PANELS_KEY) ?? '{}') as Record<string, boolean>; localStorage.setItem(PANELS_KEY, JSON.stringify({ ...saved, [name]: open })); } catch { /* session only */ }
  requestAnimationFrame(() => fit());
}
function renderView(): void {
  const focused = document.activeElement?.getAttribute('data-index');
  canvas.setAttribute('viewBox', `${view.x} ${view.y} ${view.width} ${view.height}`);
  const width = state.job.fabric.width;
  const length = displayLength();
  const unit = view.width / Math.max(1, canvas.clientWidth);
  const badIds = new Set([...jobIssues, ...report?.errors ?? []].flatMap(issue => issue.piece_ids ?? []));
  const rulerMm = appUnit() === 'inch' ? 914.4 : 1000;
  const ticks = rulerTicks({ ...view, x: view.x / rulerMm, width: view.width / rulerMm }).map(t => t * rulerMm);
  canvas.innerHTML = `<defs><pattern id="grid" width="100" height="100" patternUnits="userSpaceOnUse"><path class="fabric-grid" d="M 100 0 L 0 0 0 100" stroke-width="${unit * 0.6}"/></pattern></defs>
  <rect class="fabric" x="0" y="0" width="${length}" height="${width}" stroke-width="${unit}"/><rect x="0" y="0" width="${length}" height="${width}" fill="url(#grid)"/>
  ${features?.overlay(length, width, unit) ?? ''}
  ${ticks.map(x => `<line x1="${x}" x2="${x}" y1="${view.y}" y2="${view.y + 11 * unit}" class="ruler-tick" stroke-width="${unit}"/><text class="ruler-text" x="${x + 4 * unit}" y="${view.y + 22 * unit}" font-size="${11 * unit}">${(x / rulerMm).toFixed(view.width < 500 ? 3 : 2)}</text>`).join('')}
  ${state.result.placements.map((p, index) => {
    const piece = state.job.pieces.find(piece => piece.id === p.piece_id);
    if (!piece) return '';
    const points = outline(p,index);
    if (!bbox(points)) return '';
    return `<g class="piece ${badIds.has(piece.id) ? 'invalid' : ''}" data-index="${index}" tabindex="0" role="button" aria-pressed="false" aria-label="${esc(piece.name)} ${esc(piece.size)} copy ${p.copy + 1}${p.mirrored ? ', mirrored' : ''}${p.locked ? ', locked' : ''}${p.bundle != null ? `, bundle ${p.bundle + 1}` : ', bundle unassigned'}"><title>${esc(piece.name)} · ${esc(piece.size)}${p.mirrored ? ' · mirrored' : ''}${p.locked ? ' · locked' : ''}${p.bundle != null ? ` · bundle ${p.bundle + 1}` : ''}</title><polygon points="${points.map(([px, py]) => `${px},${width - py}`).join(' ')}" style="fill:${features?.colourByBundle() ? colours[(p.bundle ?? 0) % colours.length] : colour(piece.size)}"/></g>`;
  }).join('')}
  ${drag && drag.index >= 0 && drag.moved ? '' : `<g class="piece-labels" aria-hidden="true">${pieceLabels(width, unit)}</g>`}
  ${editorTools?.overlay(width,unit) ?? ''}`;
  $('#fabric-label').textContent = `${shortLength(width)} fabric width · grain →`;
  $('#ruler-name').textContent = appUnit() === 'inch' ? 'Ruler in yards' : 'Ruler in metres';
  $('#canvas-empty').hidden = state.result.placements.length > 0;
  $('#zoom-label').textContent = `${(1 / unit).toFixed(2)} px/mm`;
  $('#legend').innerHTML = features?.colourByBundle() ? [...new Set(state.result.placements.map(p => p.bundle))].map(bundle => `<span><i class="swatch" style="background:${colours[(bundle ?? 0) % colours.length]}"></i>${bundle == null ? 'Unassigned bundle' : `Bundle ${bundle + 1}`}</span>`).join('') : sizes().map(size => `<span><i class="swatch" style="background:${colour(size)}"></i>${esc(size)}</span>`).join('') + '<span class="mirror-key">M = mirrored copy</span>';
  renderSelection();
  if (focused !== null && focused !== undefined) canvas.querySelector<SVGGElement>(`[data-index="${Number(focused)}"]`)?.focus({ preventScroll: true });
}
/** Name and size (· M mirrored, · locked) on each piece where they fit at this zoom; hidden while a piece is dragged. */
function pieceLabels(width: number, unit: number): string {
  prepareLabels(() => renderView());
  const detail = (i: number) => { const p = state.result.placements[i], piece = state.job.pieces.find(q => q.id === p?.piece_id); return `${piece?.size ?? ''}${p?.mirrored ? ' · M' : ''}${p?.locked ? ' · locked' : ''}`; };
  return labelsSvg(placedLabels(state.job, state.result, 1 / unit, state.result.placements.map((p, i) => outline(p, i)), detail), width, unit, esc);
}
function render(form = false): void { if (form) { renderForm(); features?.refresh(); } refreshChecks(); editorTools?.refresh(); renderStats(); renderView(); }
function commit(next: DocumentState, form = false): void {
  if (nesting || features?.busy()) return;
  state = next; dirtyWithoutChecks = !core; render(form); history.commit(state); renderSelection();
}
function select(index: number, additive = false): void {
  selection = index < 0 ? [] : additive ? selection.includes(index) ? selection.filter(i => i !== index) : [...selection,index] : [index];
  selected = selection.includes(index) ? index : selection[0] ?? -1; renderSelection();
}
function updateSelected(patch: Parameters<typeof changePlacement>[2]): void { commit(changePlacement(state, selected, patch)); }
function rotate(): void {
  editorTools?.run({ op:'next_rotation' });
}
function toggleLock(): void {
  const placements = selection.map(i => state.result.placements[i]).filter(p => !!p);
  if (!placements.length) return;
  const locked = !placements.every(p => p.locked);
  let next = state; for (const i of selection) next = changePlacement(next,i,{locked}); commit(next);
}
function packLeft(): void {
  editorTools?.run({ op:'bump',direction:'left' });
}
function historyAction(redo: boolean): void { if (nesting || features?.busy()) return; state = redo ? history.redo() : history.undo(); dirtyWithoutChecks = !core; render(true); }
async function save(name: string, data: BlobPart, type: string): Promise<void> {
  const path = await downloadFile(new Blob([data], { type }), name);
  message(path ? `Saved: ${path}` : 'Save cancelled.');
}

/** While the browser search runs, draw its best marker so far (not kept in undo history; editing waits for the end). */
let liveFrame = 0, liveBest: MarkerResult | undefined, liveFitted = false;
function showLive(best: MarkerResult | undefined): void {
  if (!best?.placements?.length || !core) return;
  liveBest = best;
  if (liveFrame) return;
  liveFrame = requestAnimationFrame(() => {
    liveFrame = 0; if (!nesting || !liveBest) return;
    const kept = { result: state.result, drawing };
    try {
      state = { ...state, result: { ...state.result, ...liveBest } };
      try { drawing = core!.drawing(state.job, liveBest.placements); } catch { drawing = undefined; }
      if (!liveFitted) { liveFitted = true; fit(); } else renderView();
    } finally { state = { ...state, result: kept.result }; drawing = kept.drawing; }
  });
}
async function api(path: string, body: BodyInit): Promise<Response> {
  let response: Response;
  try { response = await apiFetch(`/api/${path}`, { method: 'POST', body, headers: { 'Content-Type': path.startsWith('import') || path === 'digitize' ? 'application/octet-stream' : 'application/json' } }); }
  catch (error) { throw new Error(isDesktop() ? errorText(error) : 'Local server is unavailable. Start beta-marker serve. JSON files, editing and browser nesting work without it.'); }
  if (response.status === 401 && !isDesktop()) {
    shell.askToken();
    throw new Error('This BETA server needs an access token. Enter the token from your administrator, then try again.');
  }
  if (!response.ok) {
    const payload = await response.json().catch(() => ({ error: `Server returned ${response.status}. Use the desktop app or server for this operation.` })) as { error?: string };
    throw new Error(payload.error || `Server error ${response.status}`);
  }
  return response;
}

/** Search progress: elapsed time against the budget, plus the best valid marker reported so far. */
function showProgress(elapsed: number, budget: number, bestLength: number | null, bestEfficiency: number | null): void {
  $('#nest-progress').hidden = false;
  $<HTMLProgressElement>('#nest-progress-bar').value = budget > 0 ? Math.min(1, elapsed / budget) : 0;
  $('#nest-progress-time').textContent = `Searching: ${elapsed.toFixed(1)} s of about ${budget.toFixed(budget < 10 ? 1 : 0)} s`;
  $('#nest-progress-best').textContent = bestLength == null ? 'No valid marker yet' : `${longLength(bestLength).value} ${longLength(bestLength).unit}${bestEfficiency == null ? '' : ` · ${(bestEfficiency * 100).toFixed(1)}% efficiency`}`;
  $('#nest-cancel').hidden = !isDesktop() && !browserStop; $('#nest-cancel-note').hidden = isDesktop() || !!browserStop;
}
$('#nest-cancel').addEventListener('click', () => { if (browserStop) { browserStop.abort(); message('Stopping after this round; keeping the best valid marker.'); return; } const native = find<HTMLButtonElement>('#native-cancel'); if (native) { native.disabled = false; native.click(); } });
$('#job-form').addEventListener('submit', event => event.preventDefault());
$('#job-form').addEventListener('click', event => {
  const b = (event.target as Element).closest<HTMLButtonElement>('[data-width-mm]'); if (!b || nesting) return;
  const next = structuredClone(state); next.job.fabric.width = Number(b.dataset.widthMm); commit(next, true);
});
$('#job-form').addEventListener('change', event => {
  if (nesting) return;
  const input = event.target as HTMLInputElement | HTMLSelectElement;
  if (!input.checkValidity()) { input.reportValidity(); return; }
  const next = structuredClone(state);
  const field = input.dataset.field;
  const value = Number(input.value);
  if (field === 'width') next.job.fabric.width = value;
  if (field === 'nap') next.job.fabric.nap = input.value as Nap;
  if (field === 'tilt') next.job.fabric.tilt_deg = value;
  if (field === 'piece_gap' || field === 'edge_gap') next.job.spacing = { ...next.job.spacing, [field]: value };
  if (field?.startsWith('ratio:')) {
    next.job.order ??= { sizes: Object.fromEntries(sizes().map(size => [size, 1])) };
    next.job.order.sizes[field.slice(6)] = value;
  }
  if (input.dataset.piece !== undefined) {
    const piece = next.job.pieces[Number(input.dataset.piece)]!;
    switch (input.dataset.key) {
      case 'name': piece.name = input.value; break;
      case 'size': piece.size = input.value || null; break;
      case 'quantity': piece.quantity = value; break;
      case 'mirror_quantity': piece.mirror_quantity = value; break;
      case 'buffer': piece.buffer = value; break;
      case 'rotation': piece.rotation = input.value as RotationRule; break;
    }
  }
  // Refresh summaries and newly edited sizes while preserving the active field.
  commit(next, true);
});
$('#open-button').addEventListener('click', () => $<HTMLInputElement>('#open-file').click());
/** Before a DXF import: "Cut 2" as a left/right pair (usual) or two same pieces, and the file units. The choices
 * fill the DXF import options (kept in Setup for later imports). */
function askDxf(name: string): Promise<boolean> {
  const dialog = $<HTMLDialogElement>('#dxf-dialog');
  $('#dxf-file').textContent = name;
  dialog.querySelector<HTMLInputElement>(`[name=dxf-pairs][value=${$<HTMLInputElement>('#pairs').checked ? 'pair' : 'same'}]`)!.checked = true;
  $<HTMLSelectElement>('#dxf-units').value = $<HTMLSelectElement>('#units').value;
  return new Promise(resolve => {
    dialog.addEventListener('close', () => {
      const open = dialog.returnValue === 'open';
      if (open) { $<HTMLInputElement>('#pairs').checked = dialog.querySelector<HTMLInputElement>('[name=dxf-pairs]:checked')?.value !== 'same'; $<HTMLSelectElement>('#units').value = $<HTMLSelectElement>('#dxf-units').value; }
      resolve(open);
    }, { once: true });
    dialog.returnValue = ''; dialog.showModal(); $<HTMLButtonElement>('#dxf-open').focus();
  });
}
$('#open-file').addEventListener('change', async event => {
  const input = event.target as HTMLInputElement, file = input.files?.[0]; if (!file || nesting || features?.busy()) return;
  try {
    if (file.size > 50 * 1024 * 1024) throw new Error('Choose a file smaller than 50 MB.');
    if (file.name.toLowerCase().endsWith('.dxf')) {
      if (!(await askDxf(file.name))) { message('Import cancelled.'); return; }
      message('Importing DXF…');
      const { report } = await (await api(dxfImportPath(state.job.fabric.width, $<HTMLSelectElement>('#units').value, $<HTMLInputElement>('#pairs').checked), file)).json() as { report: ImportReport };
      const job: Job = { format: 'beta-marker.job/v1', name: file.name.replace(/\.dxf$/i, ''), fabric: { ...state.job.fabric }, spacing: { ...state.job.spacing }, pieces: report.pieces, order: report.sizes.length ? { sizes: Object.fromEntries(report.sizes.map(size => [size, 1])) } : null };
      readDocument(JSON.stringify(job)); state = { job, result: emptyResult(job) };
      message([`Imported ${report.pieces.length} pieces.`, ...report.warnings].join(' '));
    } else {
      const document = readDocument(await file.text());
      if ('pieces' in document) { state = { job: document, result: emptyResult(document) }; message('Job loaded. Open its result JSON or nest the job.'); }
      else {
        if (document.job_name !== state.job.name) throw new Error('Open the matching job JSON before this result. Job names must match.');
        state = { ...state, result: document }; message('Result loaded.');
      }
    }
    selected = -1; selection = []; dirtyWithoutChecks = false; render(true); history = new History(state); fit(); renderSelection();
  } catch (error) { message(errorText(error), true); }
  finally { input.value = ''; }
});
$('#demo').addEventListener('click', () => {
  if (nesting || features?.busy()) return;
  state = structuredClone({ job: demoJob, result: demoResult }); selected = -1; selection = []; dirtyWithoutChecks = false;
  render(true); history = new History(state); fit(); renderSelection(); message('Original synthetic demo loaded.');
});
$('#save-job').addEventListener('click', () => { void save('marker.job.json', JSON.stringify(state.job, null, 2), 'application/json').catch(error => message(errorText(error), true)); });
$('#save-result').addEventListener('click', () => { void save('marker.result.json', JSON.stringify(state.result, null, 2), 'application/json').catch(error => message(errorText(error), true)); });
$('#nest-form').addEventListener('submit', async event => {
  event.preventDefault(); if (nesting || features?.busy()) return;
  const unlocked = (event as SubmitEvent).submitter?.id === 'renest';
  nesting = true; const button = $<HTMLButtonElement>('#nest'); button.disabled = true; $('#nest .btn-label').textContent = 'Nesting…';
  app.setAttribute('aria-busy', 'true'); renderSelection();
  $('#job-form').querySelectorAll<HTMLInputElement | HTMLSelectElement>('input, select').forEach(input => input.disabled = true);
  for (const id of ['demo', 'open-file', 'open-button']) $<HTMLButtonElement | HTMLInputElement>(`#${id}`).disabled = true;
  const local = !isDesktop() && (staticMode() || !serverOnline);
  browserStop = local ? new AbortController() : undefined; liveBest = undefined; liveFitted = false;
  message(local ? 'Rust is nesting in this browser. Stop keeps the best valid marker.' : 'Rust is nesting the marker.');
  const limit = Number($<HTMLInputElement>('#time').value), runs = Number($<HTMLInputElement>('#runs').value);
  showProgress(0, local ? limit : limit * Math.max(1, runs), null, null);
  let lastLength: number | null = null, lastEfficiency: number | null = null;
  const started = performance.now();
  const progressTimer = window.setInterval(() => {
    if (local) { showProgress((performance.now() - started) / 1000, limit, lastLength, lastEfficiency); return; }
    void apiFetch('/api/nest/status').then(r => r.json()).then((p: {elapsed_s:number;best_length_mm:number|null;best_efficiency:number|null;result?:MarkerResult|null}) => {
      if (nesting) { showProgress(p.elapsed_s, limit * Math.max(1, runs), p.best_length_mm, p.best_efficiency); showLive(p.result ?? undefined); }
    }).catch(() => {});
  }, 500);
  try {
    const payload = { ...nestPayload(state, { time_limit_s: limit, seed: Number($<HTMLInputElement>('#seed').value), workers: 2, runs }, unlocked, $<HTMLInputElement>('#warm-start').checked), adaptive: $<HTMLInputElement>('#adaptive-search').checked };
    if (payload.adaptive && (limit < 12 || (!isDesktop() && !serverOnline))) throw new Error('Strategy comparison needs the desktop app or local server and at least 12 seconds. Use ordinary search here.');
    let result: MarkerResult;
    if (local) {
      if (runs !== 1) throw new Error('Browser nesting uses one run. Set Portfolio runs to 1, or connect the desktop app/server.');
      result = await nestInBrowser(payload.job, {
        time_limit_s: limit, seed: payload.options.seed, fixed: payload.options.fixed, warm_start: payload.options.warm_start,
      }, progress => {
        lastLength = progress.result.length; lastEfficiency = progress.result.efficiency;
        showProgress(progress.seconds, limit, lastLength, lastEfficiency);
        showLive(progress.result);
      }, { signal: browserStop!.signal });
    } else {
      const response = await api('nest', JSON.stringify(payload));
      const decoded = readDocument(await response.text());
      if (!('placements' in decoded)) throw new Error('The server did not return a marker result.');
      result = decoded;
    }
    const choice = editorCore?.chooseSearch?.(state, result, payload.options.fixed ?? []);
    const found = result;
    if (choice) result = choice.result;
    const editCheck = editorCore?.inspect({job:state.job,result});
    if (editCheck && (editCheck.section_violations.length || editCheck.end_violations.length)) throw new Error('The new nest crosses a saved cut line or marker end. The current marker is kept. Adjust the cut lines or table end before nesting again.');
    nesting = false; commit({ ...state, result }); fit(); message(choice?.retained ? `Kept the previous marker. It is ${lengthWords(result.length)} long and still passes every check; the new search only found ${lengthWords(found.length)}.` : 'Nesting finished. Review the validation checks before cutting.');
  } catch (error) { message(errorText(error), true); }
  finally {
    window.clearInterval(progressTimer); browserStop = undefined; $('#nest-progress').hidden = true; nesting = false; button.disabled = false; $('#nest .btn-label').textContent = 'Nest marker'; app.removeAttribute('aria-busy');
    $('#job-form').querySelectorAll<HTMLInputElement | HTMLSelectElement>('input, select').forEach(input => input.disabled = false);
    for (const id of ['demo', 'open-file', 'open-button']) $<HTMLButtonElement | HTMLInputElement>(`#${id}`).disabled = false;
    renderSelection();
  }
});
findAll<HTMLButtonElement>('[data-export]').forEach(button => button.addEventListener('click', async () => {
  if (exporting) return; exporting = true;
  const format = button.dataset.export!; const previous = button.textContent; button.textContent = 'Exporting…';
  try {
    const inspection = editorTools?.inspection();
    if (state.job.meta?.editor_v1 && !inspection) throw new Error('Editor checks must load and pass before exporting this saved marker.');
    if (inspection && (!inspection.validation.ok || !inspection.geometry_ok)) throw new Error('Resolve validation, clearance, section and table-end issues before exporting.');
    const response = await api(`export?format=${format}`, JSON.stringify({ job: state.job, result: state.result }));
    const blob = await response.blob(); const extension = response.headers.get('X-Beta-Extension') || format; await save(`marker-${format}.${extension}`, blob, blob.type);
    const open = seamlyWarnings(state.job.meta).length;
    if (open) message(`${$('#message').textContent} ${open} Seamly2D import warning${open === 1 ? ' is' : 's are'} still open. See the warning panel before cutting.`);
  } catch (error) { message(errorText(error), true); }
  finally { exporting = false; button.textContent = previous; }
}));
$('#undo').addEventListener('click', () => historyAction(false)); $('#redo').addEventListener('click', () => historyAction(true));
$('#rotate').addEventListener('click', rotate); $('#lock').addEventListener('click', toggleLock); $('#pack').addEventListener('click', packLeft); $('#fit').addEventListener('click', () => fit(true));
function zoom(factor: number, point: Pt = [view.x + view.width / 2, view.y + view.height / 2]): void { view = zoomAt(view, point, factor); renderView(); }
$('#zoom-in').addEventListener('click', () => zoom(1 / 1.25)); $('#zoom-out').addEventListener('click', () => zoom(1.25));
function pointer(event: PointerEvent | WheelEvent): Pt {
  const rect = canvas.getBoundingClientRect();
  return screenPoint(view, [event.clientX - rect.left, event.clientY - rect.top], rect.width, rect.height);
}
canvas.addEventListener('wheel', event => { event.preventDefault(); zoom(Math.exp(Math.max(-0.3, Math.min(0.3, event.deltaY * 0.001))), pointer(event)); }, { passive: false });
type Drag = { pointer: number; index: number; start: Pt; end: Pt; original: DocumentState; view: View; moved: boolean };
let drag: Drag | undefined;
canvas.addEventListener('pointerdown', event => {
  if (event.button !== 0 || nesting || features?.busy()) return;
  const at = markerPoint(pointer(event), state.job.fabric.width);
  if (editorTools?.point(at)) { event.preventDefault(); return; }
  if (features?.canvasPoint(markerPoint(pointer(event), state.job.fabric.width), displayLength(), state.job.fabric.width)) { event.preventDefault(); renderView(); return; }
  const group = (event.target as Element).closest<SVGGElement>('[data-index]');
  const index = (event.target as Element).hasAttribute('data-length-handle') ? -3 : editorTools?.mode() === 'select' && !group ? -2 : group ? Number(group.dataset.index) : -1;
  if (index >= 0) { if (event.shiftKey) {select(index,true); group?.focus(); event.preventDefault(); return;} if (!selection.includes(index)) select(index); }
  else if (index === -1) select(-1);
  if (group) group.focus(); else canvas.focus();
  if (index >= 0 && state.result.placements[index]?.locked) return;
  drag = { pointer: event.pointerId, index, start: pointer(event), end: pointer(event), original: structuredClone(state), view: { ...view }, moved: false };
  canvas.setPointerCapture(event.pointerId); event.preventDefault();
});
canvas.addEventListener('pointermove', event => {
  if (!drag || drag.pointer !== event.pointerId) return;
  const rect = canvas.getBoundingClientRect();
  const current = screenPoint(drag.view, [event.clientX - rect.left, event.clientY - rect.top], rect.width, rect.height);
  const dx = current[0] - drag.start[0], dy = current[1] - drag.start[1];
  if (Math.hypot(dx, dy) < 0.5 && !drag.moved) return;
  drag.moved = true;
  drag.end = current;
  if (drag.index === -2) {
    renderView();
    const box = document.createElementNS('http://www.w3.org/2000/svg','rect');
    for (const [key,value] of Object.entries({x:Math.min(current[0],drag.start[0]),y:Math.min(current[1],drag.start[1]),width:Math.abs(dx),height:Math.abs(dy),fill:'#226fbd22',stroke:'#226fbd','stroke-width':view.width/Math.max(1,canvas.clientWidth)*2,'pointer-events':'none'})) box.setAttribute(key,String(value));
    canvas.append(box);
  }
  else if (drag.index === -1) { view = { ...drag.view, x: drag.view.x - dx, y: drag.view.y - dy }; renderView(); }
  else {
    const [x, y] = markerPoint([dx, dy], 0);
    try {
      if (editorTools && editorCore) {
        state = editorTools.preview(drag.original, drag.index === -3 ? {op:'length',length:current[0]} : {op:'move',delta:[x,y]});
        dirtyWithoutChecks = !core; render();
      }
    } catch (e) { message(errorText(e),true); }
  }
});
function finishDrag(cancel: boolean): void {
  if (!drag) return;
  if (drag.index === -2 && drag.moved && !cancel) {
    editorTools?.run({op:'rectangle',from:markerPoint(drag.start,state.job.fabric.width),to:markerPoint(drag.end,state.job.fabric.width)});
  }
  if ((drag.index >= 0 || drag.index === -3) && drag.moved) {
    if (cancel) { state = drag.original; render(); } else { history.commit(state); renderSelection(); }
  }
  drag = undefined;
  renderView();
}
canvas.addEventListener('pointerup', () => finishDrag(false)); canvas.addEventListener('pointercancel', () => finishDrag(true));
canvas.addEventListener('focusin', event => { const index = (event.target as Element).getAttribute('data-index'); if (index !== null && !selection.includes(Number(index))) select(Number(index)); });
document.addEventListener('keydown', event => {
  if ((event.target as Element).closest('input, select, textarea')) return;
  if (!$('.workspace').hasAttribute('hidden') && (event.ctrlKey || event.metaKey) && ['z', 'y'].includes(event.key.toLowerCase())) {
    event.preventDefault(); historyAction(event.key.toLowerCase() === 'y' || event.shiftKey); return;
  }
  if (!canvas.contains(event.target as Node) && event.target !== canvas) return;
  if (editorTools?.key(event)) {event.preventDefault();return;}
  const key = event.key.toLowerCase();
  if (key === 'r') { event.preventDefault(); rotate(); }
  if (key === 'l') { event.preventDefault(); toggleLock(); }
  if (key === '0' || key === 'home') { event.preventDefault(); fit(true); }
  if (key === '+' || key === '=') { event.preventDefault(); zoom(1 / 1.25); }
  if (key === '-') { event.preventDefault(); zoom(1.25); }
  if (key === 'escape') { features?.cancel(); renderView(); finishDrag(true); select(-1); canvas.focus(); }
  const delta: Record<string, Pt> = { ArrowLeft: [-1, 0], ArrowRight: [1, 0], ArrowUp: [0, 1], ArrowDown: [0, -1] };
  const direction = delta[event.key];
  if (direction) {
    event.preventDefault(); const p = state.result.placements[selected]; const step = event.shiftKey ? 10 : 1;
    if (p) updateSelected({ translate: [p.translate[0] + direction[0] * step, p.translate[1] + direction[1] * step] });
    else { view = { ...view, x: view.x + direction[0] * view.width * 0.05, y: view.y - direction[1] * view.height * 0.05 }; renderView(); }
  }
});
new ResizeObserver(() => { if (!canvas.clientWidth || !canvas.clientHeight) return; const aspect = canvas.clientWidth / Math.max(1, canvas.clientHeight); view.height = view.width / aspect; renderView(); }).observe(canvas);
features = installFeatures({ get: () => state, core: () => core, busy: () => nesting, api, message, redraw: renderView,
  commit: next => commit(next, true), open: next => { if (nesting) return; state = structuredClone(next); selected = -1; selection = []; dirtyWithoutChecks = !core; render(true); history = new History(state); fit(); } });
editorTools = installEditorTools({get:()=>state,core:()=>editorCore,busy:()=>nesting||!!features?.busy(),selection:()=>selection,
  select: indices=>{selection=cleanSelection(indices,state.result.placements.length);selected=selection[0]??-1;renderSelection();},commit,redraw:renderView,message,save});
const shell = installShell({ show: index => features!.show(index), message, recheck: checkHealth, state: () => state });
window.addEventListener('beta-screen', event => { if ((event as CustomEvent<number>).detail === 0 && fitPending) requestAnimationFrame(() => fit()); });
// Other screens (Quick Marker) hand a finished job and marker to the editor.
window.addEventListener('beta-open-document', event => {
  if (nesting || features?.busy()) { message('Wait for the current operation to finish.', true); return; }
  state = structuredClone((event as CustomEvent<DocumentState>).detail); selected = -1; selection = []; dirtyWithoutChecks = !core;
  render(true); history = new History(state); features?.show(0); fit(); message('Marker opened in the editor. Check it before cutting.');
});
$('#colour-mode').addEventListener('change', event => { features?.setColourByBundle((event.target as HTMLSelectElement).value === 'bundle'); renderView(); });
{
  let saved: Record<string, boolean> = {};
  try { saved = JSON.parse(localStorage.getItem(PANELS_KEY) ?? '{}') as Record<string, boolean>; } catch { /* defaults */ }
  setPanel('setup', saved.setup ?? true, false);
  setPanel('inspector', saved.inspector ?? window.matchMedia('(min-width: 1600px)').matches, false);
  $('#toggle-setup').addEventListener('click', () => setPanel('setup', $('.workspace').classList.contains('hide-setup')));
  $('#toggle-inspector').addEventListener('click', () => setPanel('inspector', $('.workspace').classList.contains('hide-inspector')));
  $('#inspector-close').addEventListener('click', () => { setPanel('inspector', false); $('#toggle-inspector').focus(); });
  window.addEventListener('beta-inspector', () => { if ($('.workspace').classList.contains('hide-inspector')) setPanel('inspector', true); });
}
render(true); fit();
onUnit(() => render(true));
// The marker engine (1.4 MB compressed) loads when a screen that uses it opens, not on Home: a phone visitor who only
// looks at Home or takes the sample tour does not download it twice. Desktop and a direct #/marker link load it at once.
let coreLoading: Promise<void> | undefined;
function ensureCore(): Promise<void> {
  return coreLoading ??= loadMarkerCore().then(async loaded => { core = loaded; editorCore = await loadEditorCore(); render(); history = new History(state); }).catch(error => { wasmProblem = `WASM could not load: ${errorText(error)} Build @beta/marker-wasm and reload to enable editing and checks.`; renderStats(); renderSelection(); });
}
const LIGHT_SCREENS = new Set([9, 11, 12]);
if (isDesktop() || !LIGHT_SCREENS.has(SCREENS.find(s => `#/${s.hash}` === location.hash)?.index ?? 9)) void ensureCore();
else whenIdle(() => void ensureCore());
window.addEventListener('beta-screen', event => { if (!LIGHT_SCREENS.has((event as CustomEvent<number>).detail)) void ensureCore(); });
/** A marker length in the app unit, for messages: "3.059 m" or "3.345 yd". */
function lengthWords(mm: number): string { const v = longLength(mm, appUnit()); return `${v.value} ${v.unit}`; }
function checkHealth(): void {
  if (staticMode()) { serverOnline = false; browserStatus(); return; }
  void apiFetch('/api/health').then(async response => {
    if (response.status === 401 && !isDesktop()) { $('#server-status').textContent = 'Server needs an access token'; $('#server-status').classList.remove('connected'); shell.askToken(); return; }
    if (!response.ok) throw new Error();
    const health = await response.json() as { ok: boolean; version: string }; if (!health.ok) throw new Error();
    serverOnline = true;
    delete $('#server-status').dataset.floorLabel; $('#server-status').removeAttribute('title');
    $('#server-status').textContent = `${isDesktop() ? 'Desktop · Rust engine' : 'Local server'} · v${health.version}`; $('#server-status').classList.add('connected');
    whereText(isDesktop() ? 'status_desktop' : 'status_server');
  }).catch(() => { serverOnline = false; browserStatus(); if (!staticMode()) whereText('status_offline_server'); });
}
/** One line in the status popover: where BETA runs and what that means. The static build adds its own note there. */
function whereText(key: 'status_desktop' | 'status_server' | 'status_offline_server'): void {
  const where = find<HTMLElement>('#status-where'); if (!where) return;
  const language = (document.documentElement.dataset.language ?? 'en') as FloorLanguage;
  where.dataset.floorLabel = key; where.textContent = label(['en', 'hi', 'ta'].includes(language) ? language : 'en', key);
}
/** No server: the Rust engine runs in this browser (drawing, grading, checks and nesting). One plain status, translated. */
function browserStatus(): void {
  const status = $('#server-status'), language = (document.documentElement.dataset.language ?? 'en') as FloorLanguage;
  status.dataset.floorLabel = 'status_browser'; status.textContent = label(['en', 'hi', 'ta'].includes(language) ? language : 'en', 'status_browser');
  status.title = label(['en', 'hi', 'ta'].includes(language) ? language : 'en', staticMode() ? 'status_browser_tip' : 'status_offline_tip'); status.classList.add('connected');
}
checkHealth();
installStaticNotices(message);

void installDesktop({ get: () => state, open: next => { if (nesting || features?.busy()) throw new Error('Wait for the current operation.'); state = structuredClone(next); selected = -1; dirtyWithoutChecks = !core; render(true); history = new History(state); fit(); }, busy: () => nesting || !!features?.busy(), message, planning: () => features?.desktopSnapshot(), saved: () => features?.desktopSaved(), restorePlanning: value => features?.desktopRestore(value) });
