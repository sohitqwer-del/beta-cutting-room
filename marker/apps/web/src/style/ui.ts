import type { BlockInfo, BlockKind, GradedPiece, Job, MarkerResult, MatchRule, PatternDocument, PatternInspection, Pt, RepeatBand, SeamCheck, SeamReport, SizeChart, StyleOperation, ValidationReport } from '@beta/marker-wasm';
import { icon } from '../icons';
import { downloadFile, isDesktop } from '../transport';
import { staticMode } from '../static-mode';
import { PatternHistory } from '../pattern-state';
import type { DocumentState } from '../state';
import { exportMarker, isPracticeResult, loadStyleEngine, nestMarker, practiceRequested, type StyleEngine } from './adapter';
import { BLOCK_ART, FACE_ART, LAYOUT_ART, NAP_ART, REPEAT_ART } from './art';
import { placed, ruleReports, stripeStep, styleRuleOf, type MarkerMatch, type RuleReport } from './match-report';
import { arrangeOps, installCanvas } from './canvas';
import { currentLanguage, t, type StyleKey } from './i18n';
import * as S from './state';
import { boundsOf } from './view';
import { formatLength, fromMm, metres, parseLength, percent, step as stepCount, toMm, WIDTH_PRESETS, widthProblem, type LengthUnit } from './units';

/** Style: from scratch to cut. One guided flow in 8 steps (Style, Pattern, Grade, Lay, Gaps, Stripes, Marker, Cut).
 * Each step has a clear Next and can be revisited. All garment geometry and checks call Rust through adapter.ts. */
export const STYLE_SCREEN = 12;
export interface StyleHost { message(text: string, error?: boolean): void; editorState(): DocumentState }

const esc = (v: unknown): string => String(v ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]!));
const tx = (key: StyleKey, values?: Record<string, string | number>) => esc(t(key, values));
/** Engine messages in plain words. A list of check issues (JSON from Rust) becomes a count and the first two reasons. */
export function errorText(e: unknown): string {
  const text = e instanceof Error ? e.message : String(e);
  if (/unknown variant `[a-z_]+`/.test(text)) return t('err_engine_old');
  const list = /^(.*?):\s*(\[\s*\{[\s\S]*\])\s*$/.exec(text);
  if (!list) return text;
  try {
    const issues = JSON.parse(list[2]!) as Array<{ message?: string }>;
    const first = issues.slice(0, 2).map(i => i.message ?? '').filter(Boolean).join(' ');
    return `${list[1]!.trim().replace(/[.:]$/, '')}. ${t('issues_count', { n: issues.length })} ${first}${issues.length > 2 ? ' …' : ''}`;
  } catch { return text; }
}
const STEP_ICON: Record<S.Step, string> = { style: 'style', pattern: 'pattern', grade: 'grade', lay: 'spread', gaps: 'gap', stripes: 'stripes', marker: 'marker', cut: 'cut' };
const BUDGETS: Array<[StyleKey, number]> = [['time_fast', 20], ['time_better', 60], ['time_best', 180]];
const pts = (list: Pt[]) => list.map(([x, y]) => `${x},${-y}`).join(' ');
const human = (name: string) => name.replaceAll('_', ' ').replace(/^./, c => c.toUpperCase());

export function installStyle(host: StyleHost, content: HTMLElement): { root: HTMLElement } {
  let state: S.Session = S.restore() ?? S.initialSession();
  let engine: StyleEngine | null = null, engineError = '', loading: Promise<void> | null = null;
  let history = new PatternHistory(state.doc ?? emptyDoc());
  let blocks: BlockInfo[] = [], chart: SizeChart | null = null, chartPreset: 'indian_adult' | 'iso_example' = 'indian_adult';
  let base: PatternInspection | null = null, seams: SeamReport[] = [], extraChecks: SeamCheck[] = [], extraReports: SeamReport[] = [];
  let gradePiece = '', gradePoint = '', nest: Record<string, GradedPiece[]> = {}, hiddenSizes = new Set<string>();
  let matchPick: { a?: { piece: string; point: string }; pieceA: string; pieceB: string } = { pieceA: '', pieceB: '' };
  let budget = 20, abort: AbortController | null = null, progress = { elapsed: 0, best: null as MarkerResult | null };
  let layers = { stripes: true, matches: true, previous: false, sizes: true };
  /** Style match rule picked with "Show on marker"; its points are highlighted and the others dimmed. */
  let focusRule: number | null = null;
  let graded: Record<string, GradedPiece[]> = {}, gradedKey = '';
  let handedOff = false, saveTimer = 0;
  /** Focus to restore once a drawing that is filled in later (the size nest) has its points again. */
  let pendingFocus = '';

  const root = document.createElement('section');
  root.id = `screen-${STYLE_SCREEN}`; root.className = 'screen style'; root.hidden = true;
  root.setAttribute('aria-labelledby', 'st-title');
  root.innerHTML = `<nav class="workflow st-steps" aria-label="${tx('steps_label')}"><ol id="st-steps"></ol><button type="button" class="workflow-next" id="st-next-top"><span class="next-word">${tx('next')}</span><span class="next-name"></span>${icon('arrow-right')}</button></nav>
    <div class="st-head" id="st-head"></div>
    <div class="st-how" id="st-how" hidden></div>
    <div class="st-body" id="st-body"></div>
    <div class="st-canvas" id="st-canvas" hidden></div>
    <div class="st-dock" id="st-dock"></div>
    <p id="st-live" class="sr-only" role="status" aria-live="polite"></p>
    <input type="file" id="st-open-file" accept=".json,.betastyle,application/json" hidden>`;
  content.append(root);
  const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
  const body = $('st-body');
  const say = (text: string) => { const live = $('st-live'); live.textContent = ''; window.setTimeout(() => { live.textContent = text; }, 30); };
  const unit = (): LengthUnit => state.unit;
  const L = (mm: number, withUnit = true) => formatLength(mm, unit(), withUnit);
  const uName = () => (unit() === 'inch' ? 'in' : 'cm');
  /** Match offsets are a few millimetres, so they are always shown in mm with one decimal. */
  const MM = (mm: number) => `${(Math.round(mm * 10) / 10).toFixed(1)} mm`;

  function emptyDoc(): PatternDocument { return { format: 'beta-pattern.doc/v1', name: '', sizes: ['M'], base_size: 'M', measurements: { M: {} }, variables: {}, drawing: {}, pieces: [] }; }
  function persist(): void { window.clearTimeout(saveTimer); saveTimer = window.setTimeout(() => { if (!S.save(state)) host.message(t('storage_full'), true); }, 300); }
  function update(next: S.Session, redraw = true): void { state = next; persist(); if (redraw) render(); }

  // ---------- Engine ----------
  function ensureEngine(): Promise<void> {
    loading ??= (async () => {
      const r = await loadStyleEngine();
      if (!r.ok) { engineError = r.error; render(); return; }
      engine = r.engine;
      try { blocks = await engine.blocks(); chart = await engine.sizeChart(chartPreset); } catch (error) { engineError = errorText(error); }
      await refreshDoc(); render(); canvas.reset();
    })();
    return loading;
  }
  /** Re-read Rust geometry for the base size and the seam checks after every change. */
  async function refreshDoc(): Promise<void> {
    if (!engine || !state.doc) { base = null; seams = []; return; }
    try {
      base = await engine.inspect(state.doc, state.doc.base_size);
      seams = await engine.blockSeams(state.doc);
      extraReports = (await Promise.all(extraChecks.map(c => engine!.seamReport(state.doc!, c).catch(() => null)))).filter((r): r is SeamReport => !!r);
    } catch (error) { host.message(errorText(error), true); }
  }
  /** One undoable edit through the engine. A refused edit changes nothing. */
  async function commit(ops: StyleOperation[], done: string): Promise<boolean> {
    if (!engine || !state.doc) return false;
    try {
      const next = await engine.edit(state.doc, ops);
      history.apply(next); state = S.setDoc(state, next); persist();
      await refreshDoc(); canvas.refresh(); if (state.step !== 'pattern') render(); else renderChrome();
      say(done); return true;
    } catch (error) { host.message(errorText(error), true); return false; }
  }
  async function useDoc(doc: PatternDocument, start: S.Session['start']): Promise<void> {
    history = new PatternHistory(doc); state = S.setDoc(state, doc, start);
    state = { ...state, marker: { status: 'idle', stale: false } }; persist();
    gradePiece = ''; gradePoint = ''; extraChecks = []; await refreshDoc(); canvas.reset(); render();
  }
  async function undoRedo(redo: boolean): Promise<void> {
    if (!(redo ? history.canRedo : history.canUndo)) return;
    const doc = redo ? history.redo() : history.undo();
    state = S.setDoc(state, doc); persist(); await refreshDoc(); canvas.refresh(); render(); say(t(redo ? 'redone' : 'undone'));
  }
  const canvas = installCanvas($('st-canvas'), {
    engine: () => engine, doc: () => state.doc, unit, commit,
    undo: () => void undoRedo(false), redo: () => void undoRedo(true), canUndo: () => history.canUndo, canRedo: () => history.canRedo,
    say, message: host.message,
  });

  // ---------- Frame ----------
  function goStep(step: S.Step): void {
    const why = S.blocker(state, step);
    if (why) { flashWhy(t(`why_${why}` as StyleKey)); return; }
    if (step !== state.step) { markHowSeen(state.step); delete howToggled[state.step]; }
    state = S.go(state, step); persist(); render(true);
  }
  function flashWhy(text: string): void {
    say(text); const why = root.querySelector<HTMLElement>('.st-why');
    if (why) { why.textContent = text; why.classList.remove('st-flash'); void why.offsetWidth; why.classList.add('st-flash'); } else host.message(text);
  }
  function renderChrome(): void {
    const at = S.STEPS.indexOf(state.step);
    $('st-steps').innerHTML = S.STEPS.map((s, i) => {
      const open = !S.blocker(state, s), done = i < at && open;
      return `<li><button type="button" data-st-step="${s}" ${s === state.step ? 'aria-current="step"' : ''} aria-disabled="${!open}" aria-description="${tx(`step_${s}_hint`)}" title="${tx(`step_${s}`)}"><span class="step-no" aria-hidden="true">${done ? icon('check') : i + 1}</span><span class="step-name">${tx(`step_${s}`)}</span></button></li>`;
    }).join('');
    // Keep the current step in view when the bar scrolls sideways (narrow screens).
    const list = $('st-steps'); list.parentElement!.classList.remove('compact');
    if (list.scrollWidth > list.clientWidth + 2) list.parentElement!.classList.add('compact');
    const cur = list.querySelector<HTMLElement>('[aria-current=step]')?.parentElement;
    if (cur && (cur.offsetLeft < list.scrollLeft || cur.offsetLeft + cur.offsetWidth > list.scrollLeft + list.clientWidth)) list.scrollLeft = cur.offsetLeft - 24;
    const next = S.nextStep(state.step);
    const top = $<HTMLButtonElement>('st-next-top'); top.hidden = !next;
    top.querySelector('.next-word')!.textContent = t('next');
    if (next) { top.querySelector('.next-name')!.textContent = t(`step_${next}`); top.setAttribute('aria-label', t('next_aria', { step: t(`step_${next}`) })); }
    const prev = S.prevStep(state.step);
    $('st-dock').innerHTML = `${prev ? `<button type="button" class="ghost" data-st="back">${icon('arrow-left')}${tx('back_to', { step: t(`step_${prev}`) })}</button>` : '<span></span>'}
      <p class="st-why hint" aria-hidden="true"></p>
      ${next ? `<button type="button" class="primary large" data-st="next" id="st-next">${tx('next_to', { step: t(`step_${next}`) })}${icon('arrow-right')}</button>` : `<button type="button" class="primary large" data-st="home">${icon('check')}${tx('finish')}</button>`}`;
    // One status for the whole app sits in the top bar; here only loading or a failure is shown.
    const engineChip = engine ? '' : engineError ? `<span class="chip bad">${icon('alert')}${tx('engine_failed')}</span>` : `<span class="chip">${tx('engine_loading')}</span>`;
    const how = howOpen();
    $('st-head').innerHTML = `<div class="st-title"><span class="icon-tile">${icon(STEP_ICON[state.step])}</span><div><span class="eyebrow">${tx('step_of', { n: at + 1, total: S.STEPS.length })}${state.info.name ? ` · <span class="st-style-name">${esc(state.info.name)}</span>` : ''}</span><h1 id="st-title" tabindex="-1">${tx(`step_${state.step}_title`)}</h1></div></div>
      <div class="st-head-tools">${engineChip}<button type="button" class="small ghost st-how-btn" data-st="how" aria-expanded="${how}" aria-controls="st-how">${icon('help')}<span>${tx('how_title')}</span></button>
        <div class="segmented" role="group" aria-label="${tx('units')}"><button type="button" data-unit="cm" aria-pressed="${unit() === 'cm'}">cm</button><button type="button" data-unit="inch" aria-pressed="${unit() === 'inch'}">inch</button></div>
        <details class="menu"><summary>${icon('save')}${tx('file_menu')}</summary><div class="menu-panel" role="group" aria-label="${tx('file_menu')}">
          <button type="button" data-st="save-file" ${state.doc ? '' : 'disabled'}>${tx('save_style')}<small>.json</small></button><button type="button" data-st="open-file">${tx('open_style')}</button>
          <hr><button type="button" data-st="restart">${tx('start_again')}</button></div></details></div>`;
    root.dataset.step = state.step;
    renderHow(how);
  }
  // The step bar may start or stop overflowing when the window or the language changes.
  new ResizeObserver(() => { const list = document.getElementById('st-steps'); if (!list || root.hidden) return; list.parentElement!.classList.remove('compact'); if (list.scrollWidth > list.clientWidth + 2) list.parentElement!.classList.add('compact'); }).observe(root.querySelector('.st-steps')!);
  // ---------- How this works (one short strip per step; open the first time a step is shown) ----------
  const HOW_KEY = 'beta-style-how-seen';
  let howSeen = new Set<string>(), howToggled: Partial<Record<S.Step, boolean>> = {};
  try { howSeen = new Set(JSON.parse(localStorage.getItem(HOW_KEY) ?? '[]') as string[]); } catch { /* first visit */ }
  function howOpen(): boolean { return howToggled[state.step] ?? !howSeen.has(state.step); }
  function markHowSeen(step: S.Step): void {
    if (howSeen.has(step)) return;
    howSeen.add(step); try { localStorage.setItem(HOW_KEY, JSON.stringify([...howSeen])); } catch { /* session only */ }
  }
  function renderHow(open: boolean): void {
    const box = $('st-how'); box.hidden = !open; if (!open) { box.innerHTML = ''; return; }
    const s = state.step;
    box.innerHTML = `<h2 class="sr-only">${tx('how_title')}</h2><p class="st-how-lead">${tx(`step_${s}_lead`)}</p><ol>${([1, 2, 3] as const).map(n => `<li>${tx(`how_${s}_${n}` as StyleKey)}</li>`).join('')}</ol><button type="button" class="small ghost" data-st="how-close">${tx('how_got_it')}</button>`;
  }
  function render(focusTitle = false): void {
    const active = document.activeElement as HTMLElement | null, keep = active && root.contains(active) && active.id ? active.id : '';
    renderChrome();
    const pattern = state.step === 'pattern';
    $('st-canvas').hidden = !pattern; body.hidden = pattern;
    if (!pattern) body.innerHTML = engineError && state.step !== 'style' ? engineNotice() : ({ style: styleHtml, pattern: () => '', grade: gradeHtml, lay: layHtml, gaps: gapsHtml, stripes: stripesHtml, marker: markerHtml, cut: cutHtml } as Record<S.Step, () => string>)[state.step]();
    if (pattern) canvas.refresh();
    markerSize = ''; if (state.step === 'marker') window.requestAnimationFrame(redrawMarker);
    if (focusTitle) { root.querySelector<HTMLElement>('#st-title')?.focus({ preventScroll: true }); window.scrollTo(0, 0); document.getElementById('content')?.scrollTo(0, 0); }
    else if (keep) { const el = document.getElementById(keep); if (el) el.focus({ preventScroll: true }); else pendingFocus = keep; }
    if (state.step === 'grade') void loadNest();
    if (state.step === 'stripes' && layers.sizes) void loadGraded();
  }
  const engineNotice = () => `<div class="notice error st-engine-error" role="alert">${icon('alert')}<div><strong>${tx('engine_failed')}</strong><p>${tx('engine_failed_hint')}</p><details><summary>${tx('details')}</summary><p><code>${esc(engineError)}</code></p><p>${tx('engine_failed_dev')}</p></details></div></div>`;
  const card = (title: string, inner: string, cls = '') => `<section class="card st-card ${cls}"><h2>${title}</h2>${inner}</section>`;
  const lengthField = (id: string, label: string, mm: number, extra = '') => `<label class="st-field" for="${id}">${label}<span class="sc-num"><input id="${id}" inputmode="decimal" autocomplete="off" value="${esc(formatLength(mm, unit()))}" ${extra}><span>${uName()}</span></span></label>`;
  const radioCards = (name: string, options: Array<[string, string, string]>, current: string, cls = '') => `<div class="st-choices ${cls}" role="radiogroup" aria-label="${tx(`${name}_label` as StyleKey)}">${options.map(([value, art, label]) => `<label class="st-choice"><input type="radio" name="st-${name}" value="${value}" ${value === current ? 'checked' : ''}>${art}<span>${label}</span></label>`).join('')}</div>`;

  // ---------- 1 Style ----------
  function styleHtml(): string {
    const sizes = chart?.sizes ?? [], chosen = state.doc?.sizes ?? sizes;
    const info = state.info, field = (key: keyof S.StyleInfo, label: StyleKey, extra = '') => `<label class="st-field">${tx(label)}<input id="st-info-${key}" data-info="${key}" value="${esc(info[key])}" ${extra}></label>`;
    const gallery = [...blocks.map(b => [b.id, b.name, b.pieces.length] as const), ['blank', t('blank_sheet'), 0] as const];
    const current = state.start?.kind === 'block' ? state.start.block : state.start?.kind === 'blank' ? 'blank' : '';
    return `${engineError ? engineNotice() : ''}<div class="st-grid two">
      ${card(tx('about_style'), `<div class="st-fields">${field('name', 'style_name', `required maxlength="80" aria-required="true" placeholder="${tx('style_name_placeholder')}"`)}${field('number', 'style_number', 'maxlength="40"')}${field('garment', 'garment_type', 'maxlength="60"')}${field('season', 'season', 'maxlength="40"')}${field('fabric', 'main_fabric', 'maxlength="60"')}</div>
        <label class="st-field">${tx('notes')}<textarea id="st-info-notes" data-info="notes" rows="2" maxlength="500">${esc(info.notes)}</textarea></label><p class="hint">${tx('buyer_free_hint')}</p>`)}
      ${card(tx('size_range'), `<label class="st-field">${tx('size_chart')}<select id="st-preset"><option value="indian_adult" ${chartPreset === 'indian_adult' ? 'selected' : ''}>${tx('preset_indian')}</option><option value="iso_example" ${chartPreset === 'iso_example' ? 'selected' : ''}>${tx('preset_iso')}</option></select></label>
        <fieldset class="st-sizes"><legend>${tx('sizes_in_style')}</legend>${sizes.map(s => `<label class="st-size-chip"><input type="checkbox" data-range="${esc(s)}" ${!state.doc || chosen.includes(s) ? 'checked' : ''}><span>${esc(s)}</span></label>`).join('') || `<p class="hint">${tx('engine_loading')}</p>`}</fieldset>
        <p class="hint">${tx('size_range_hint')}</p>`)}
    </div>
    ${card(tx('start_from'), `<p class="hint">${tx('start_from_hint')}</p>${!engine && !engineError ? `<p class="st-loading" role="status"><span class="st-spinner" aria-hidden="true"></span>${tx('blocks_loading')}</p>` : ''}<div class="st-gallery" role="group" aria-label="${tx('start_from')}">${gallery.map(([id, name, n]) => `<button type="button" class="st-block" data-block="${id}" aria-pressed="${id === current}">${BLOCK_ART[id] ?? ''}<strong>${esc(name)}</strong><span>${id === 'blank' ? tx('blank_sheet_hint') : tx('n_pieces', { n })}</span></button>`).join('')}
      <button type="button" class="st-block" data-st="open-file">${icon('open')}<strong>${tx('open_style')}</strong><span>${tx('open_style_hint')}</span></button></div>
      ${state.doc ? `<p class="notice"><strong>${tx('current_pattern')}</strong> ${tx('current_pattern_text', { pieces: state.doc.pieces.length, sizes: state.doc.sizes.join(' · ') })}</p>` : ''}`, 'st-start')}`;
  }
  async function startFrom(id: string): Promise<void> {
    if (!engine) { await ensureEngine(); if (!engine) return; }
    if (state.doc && (state.doc.pieces.length || Object.keys(state.doc.drawing).length) && !window.confirm(t('confirm_replace'))) return;
    const block = id === 'blank' ? undefined : id as BlockKind, info = blocks.find(b => b.id === id);
    if (!state.info.name.trim()) state = S.setInfo(state, { name: info ? t('new_style_of', { block: info.name }) : t('new_style') });
    if (info && !state.info.garment) state = S.setInfo(state, { garment: info.name });
    const range = Array.from(root.querySelectorAll<HTMLInputElement>('[data-range]:checked'), i => i.dataset.range!);
    const c = chart!, sizes = range.length ? c.sizes.filter(s => range.includes(s)) : c.sizes;
    const useChart: SizeChart = { ...c, sizes, base_size: sizes.includes(c.base_size) ? c.base_size : sizes[Math.floor((sizes.length - 1) / 2)]!, measurements: Object.fromEntries(sizes.map(s => [s, c.measurements[s]!])) };
    try {
      const doc = await engine!.create({ name: state.info.name, block, chart: useChart });
      await useDoc(doc, block ? { kind: 'block', block } : { kind: 'blank' });
      // Pieces of a new block start apart on the sheet, so each one can be seen and clicked.
      if (block && base) { const ops = arrangeOps(doc, base); if (ops.length) { try { const spread = await engine!.edit(doc, ops); await useDoc(spread, { kind: 'block', block }); } catch { /* keep the engine's layout */ } } }
      say(block ? t('block_ready', { block: info?.name ?? '', n: doc.pieces.length }) : t('blank_ready'));
    } catch (error) { host.message(errorText(error), true); }
  }

  // ---------- 3 Grade ----------
  function gradeHtml(): string {
    const d = state.doc!, keys = [...new Set(d.sizes.flatMap(s => Object.keys(d.measurements[s] ?? {})))];
    const piece = d.pieces.find(p => p.id === gradePiece) ?? d.pieces[0]!; gradePiece = piece.id;
    const table = `<div class="table-scroll st-chart"><table><caption>${tx('size_chart_caption', { unit: uName() })}</caption><thead><tr><th scope="col">${tx('measurement')}</th>${d.sizes.map(s => `<th scope="col"><span class="st-size-head">${esc(s)}<label class="st-base" title="${tx('base_size')}"><input type="radio" name="st-base" value="${esc(s)}" ${s === d.base_size ? 'checked' : ''} aria-label="${tx('base_size_is', { size: s })}">${s === d.base_size ? tx('base') : ''}</label>${d.sizes.length > 1 ? `<button type="button" class="icon-btn small ghost" data-remove-size="${esc(s)}" aria-label="${tx('remove_size', { size: s })}">${icon('close')}</button>` : ''}</span></th>`).join('')}</tr></thead>
      <tbody>${keys.map(k => `<tr><th scope="row">${esc(human(k))}</th>${d.sizes.map(s => `<td><input class="st-cell" inputmode="decimal" data-measure="${esc(k)}" data-size="${esc(s)}" value="${d.measurements[s]?.[k] != null ? esc(formatLength(d.measurements[s]![k]!, unit())) : ''}" aria-label="${esc(human(k))} ${esc(s)} (${uName()})"></td>`).join('')}</tr>`).join('') || `<tr><td colspan="${d.sizes.length + 1}" class="hint">${tx('chart_empty')}</td></tr>`}</tbody></table></div>
      <p class="hint">${tx('size_chart_hint')}</p><div class="st-row"><label class="st-field">${tx('new_size')}<input id="st-new-size" maxlength="12" placeholder="XXL"></label><button type="button" data-st="add-size">${icon('plus')}${tx('add_size')}</button>
      <label class="st-field">${tx('preset')}<select id="st-chart-preset"><option value="">${tx('choose')}</option><option value="indian_adult">${tx('preset_indian')}</option><option value="iso_example">${tx('preset_iso')}</option></select></label></div>`;
    const sizesKey = d.sizes.map((s, i) => `<label class="st-size-key"><input type="checkbox" data-size-toggle="${esc(s)}" ${hiddenSizes.has(s) ? '' : 'checked'}><i style="background:var(--size-${i % 6})"></i>${esc(s)}</label>`).join('');
    const ruleId = `rule_${gradePiece}_${gradePoint}`, rule = d.rules?.[piece.grade_points?.[gradePoint] ?? ruleId];
    const ruleBox = gradePoint ? `<div class="st-rule"><h3>${tx('rule_for', { point: gradePoint })}</h3><p class="hint">${tx('rule_hint')}</p>
      <table class="st-rule-table"><thead><tr><th scope="col">${tx('size')}</th><th scope="col">${tx('move_x')} (${uName()})</th><th scope="col">${tx('move_y')} (${uName()})</th></tr></thead><tbody>${d.sizes.map(s => `<tr><th scope="row">${esc(s)}${s === d.base_size ? ` · ${tx('base')}` : ''}</th><td><input inputmode="decimal" data-rule-x="${esc(s)}" value="${esc(formatLength(rule?.deltas[s]?.[0] ?? 0, unit()))}" ${s === d.base_size ? 'disabled' : ''} aria-label="${tx('move_x')} ${esc(s)}"></td><td><input inputmode="decimal" data-rule-y="${esc(s)}" value="${esc(formatLength(rule?.deltas[s]?.[1] ?? 0, unit()))}" ${s === d.base_size ? 'disabled' : ''} aria-label="${tx('move_y')} ${esc(s)}"></td></tr>`).join('')}</tbody></table>
      <div class="st-row"><button type="button" class="primary" data-st="save-rule">${tx('use_rule')}</button>${piece.grade_points?.[gradePoint] ? `<button type="button" class="danger" data-st="remove-rule">${tx('remove_rule')}</button>` : ''}<button type="button" class="ghost" data-st="close-rule">${tx('close')}</button></div></div>` : `<p class="hint">${tx('pick_point_for_rule')}</p>`;
    const reports = [...seams, ...extraReports];
    const seamTable = reports.length ? `<div class="table-scroll"><table class="st-seams"><caption>${tx('seam_caption')}</caption><thead><tr><th scope="col">${tx('seam')}</th>${d.sizes.map(s => `<th scope="col">${esc(s)}</th>`).join('')}</tr></thead><tbody>${reports.map(r => `<tr><th scope="row">${esc(r.check.name)}<small>${esc(pieceName(r.check.a.piece))} ↔ ${esc(pieceName(r.check.b.piece))}</small></th>${r.rows.map(row => `<td><span class="chip ${row.pass ? 'ok' : 'bad'}">${row.pass ? icon('check') : icon('alert')}${esc(signed(row.difference_mm))}</span></td>`).join('')}</tr>`).join('')}</tbody></table></div>` : `<p class="hint">${tx('no_seam_checks')}</p>`;
    const opts = d.pieces.map(p => `<option value="${esc(p.id)}">${esc(p.name)}</option>`).join('');
    return `<div class="st-grid grade">
      ${card(tx('size_chart'), table, 'st-chart-card')}
      ${card(tx('nest_view'), `<div class="st-row st-piece-tabs" role="group" aria-label="${tx('piece')}">${d.pieces.map(p => `<button type="button" class="small" data-grade-piece="${esc(p.id)}" aria-pressed="${p.id === gradePiece}">${esc(p.name)}</button>`).join('')}</div>
        <div class="st-row st-size-keys">${sizesKey}</div><svg class="st-nest" id="st-nest" role="img" aria-label="${tx('nest_aria', { piece: piece.name })}"></svg>${ruleBox}`, 'st-nest-card')}
    </div>
    ${card(tx('seam_check'), `${seamTable}<details class="advanced"><summary>${tx('add_seam_check')}</summary><div class="st-row"><label class="st-field">${tx('piece_a')}<select id="st-check-a">${opts}</select></label><label class="st-field">${tx('edges_a')}<input id="st-check-ea" placeholder="2" inputmode="numeric"></label>
      <label class="st-field">${tx('piece_b')}<select id="st-check-b">${opts}</select></label><label class="st-field">${tx('edges_b')}<input id="st-check-eb" placeholder="2" inputmode="numeric"></label>${lengthField('st-check-ease', tx('ease'), 0)}<button type="button" data-st="add-check">${icon('plus')}${tx('add_check')}</button></div><p class="hint">${tx('edges_hint')}</p></details>`)}`;
  }
  const signed = (mm: number) => `${mm > 0.05 ? '+' : ''}${formatLength(mm, unit())} ${uName()}`;
  const pieceName = (id: string) => state.doc?.pieces.find(p => p.id === id)?.name ?? id;
  function startPoint(doc: PatternDocument, pieceId: string): string {
    const p = doc.pieces.find(q => q.id === pieceId), e = p?.edges[0], o = e ? doc.drawing[e.object] : undefined;
    if (o?.kind === 'line') return e!.reversed ? o.to : o.from;
    if (o?.kind === 'cubic') return e!.reversed ? o.points[3] : o.points[0];
    return '';
  }
  /** Grade rules move outline corners: the end points of the piece's edges. */
  function outlineIds(doc: PatternDocument, pieceId: string): Set<string> {
    const out = new Set<string>();
    for (const e of doc.pieces.find(p => p.id === pieceId)?.edges ?? []) {
      const o = doc.drawing[e.object];
      if (o?.kind === 'line') { out.add(o.from); out.add(o.to); }
      else if (o?.kind === 'cubic') { out.add(o.points[0]); out.add(o.points[3]); }
      else if (o?.kind === 'spline') { out.add(o.points[0]!); out.add(o.points.at(-1)!); }
    }
    return out;
  }
  /** All sizes for the stripes overlay (cached per document). */
  async function loadGraded(): Promise<void> {
    if (!engine || !state.doc) return;
    const key = JSON.stringify(state.doc); if (key === gradedKey) return;
    gradedKey = key;
    try { graded = (await engine.grade(state.doc)).sizes; } catch { graded = {}; }
    if (state.step === 'stripes') { const svgEl = document.getElementById('st-stripe-preview'); if (svgEl) svgEl.innerHTML = stripePreview(); }
  }
  async function loadNest(): Promise<void> {
    if (!engine || !state.doc || !gradePiece) return;
    const ref = startPoint(state.doc, gradePiece);
    try { nest = ref ? await engine.nestSizes(state.doc, ref) : {}; } catch { nest = {}; }
    if (!nest || !Object.keys(nest).length) { try { const g = await engine.grade(state.doc); nest = g.sizes; } catch { nest = {}; } }
    drawNest();
  }
  function drawNest(): void {
    const svgEl = document.getElementById('st-nest'); if (!svgEl || !state.doc) return;
    const d = state.doc, list = d.sizes.map((s, i) => ({ s, i, p: nest[s]?.find(g => g.piece.id === gradePiece || g.piece.id === `${gradePiece}-${s}` || g.piece.name === d.pieces.find(q => q.id === gradePiece)?.name) }));
    const all = list.flatMap(x => x.p?.seam ?? []), b = boundsOf(all);
    if (!b) { svgEl.innerHTML = `<text x="50%" y="50%" text-anchor="middle" class="st-svg-empty">${tx('nest_empty')}</text>`; svgEl.removeAttribute('viewBox'); return; }
    const pad = Math.max(b[2] - b[0], b[3] - b[1]) * 0.06, r = Math.max(b[2] - b[0], b[3] - b[1]) / 110;
    svgEl.setAttribute('viewBox', `${b[0] - pad} ${-b[3] - pad} ${b[2] - b[0] + 2 * pad} ${b[3] - b[1] + 2 * pad}`);
    const baseP = list.find(x => x.s === d.base_size)?.p, outlinePoints = outlineIds(d, gradePiece);
    const restore = pendingFocus; pendingFocus = '';
    window.setTimeout(() => { if (restore) document.getElementById(restore)?.focus({ preventScroll: true }); }, 0);
    svgEl.innerHTML = list.filter(x => x.p && !hiddenSizes.has(x.s)).map(x => `<polygon class="st-nest-size${x.s === d.base_size ? ' base' : ''}" points="${pts(x.p!.seam)}" style="stroke:var(--size-${x.i % 6})"><title>${esc(x.s)}</title></polygon>`).join('')
      + Object.entries(baseP?.points ?? {}).filter(([id]) => outlinePoints.has(id)).map(([id, p]) => `<circle id="st-gp-${esc(id.replace(/[^\w-]/g, '_'))}" class="st-nest-point${id === gradePoint ? ' selected' : ''}${d.pieces.find(q => q.id === gradePiece)?.grade_points?.[id] ? ' ruled' : ''}" data-grade-point="${esc(id)}" cx="${p[0]}" cy="${-p[1]}" r="${id === gradePoint ? r * 1.6 : r}" tabindex="0" role="button" aria-label="${tx('point_rule_aria', { point: id })}"><title>${esc(id)}</title></circle>`).join('');
  }

  // ---------- 4 Lay ----------
  function layHtml(): string {
    const l = state.lay, groups = S.materials(state.doc), presets = WIDTH_PRESETS[l.widthUnit], even = S.pliesMustBeEven(l);
    const wv = formatLength(l.widthMm, l.widthUnit === 'inch' ? 'inch' : 'cm');
    const widthBad = widthProblem(l.widthMm);
    const gm = S.garmentsPerMarker(l), spreads = l.layout === 'open' ? l.plies : Math.ceil(l.plies / 2);
    return `<div class="st-grid two">
      ${card(tx('fabric_width'), `<div class="st-row"><label class="st-field st-big">${tx('usable_width')}<span class="sc-num big"><input id="st-width" inputmode="decimal" value="${esc(wv)}" aria-describedby="st-width-other" ${widthBad ? 'aria-invalid="true"' : ''}><span>${l.widthUnit === 'inch' ? 'in' : 'cm'}</span></span></label>
        <div class="segmented" role="group" aria-label="${tx('width_unit')}"><button type="button" data-width-unit="inch" aria-pressed="${l.widthUnit === 'inch'}">inch</button><button type="button" data-width-unit="cm" aria-pressed="${l.widthUnit === 'cm'}">cm</button></div></div>
        <p class="hint" id="st-width-other">${widthBad ? `<span class="error-text">${tx(`width_${widthBad}` as StyleKey)}</span>` : tx('width_other', { value: formatLength(l.widthMm, l.widthUnit === 'inch' ? 'cm' : 'inch', true) })}</p>
        <div class="st-row st-presets" role="group" aria-label="${tx('common_widths')}">${presets.map(p => `<button type="button" class="small" data-width-preset="${p}" aria-pressed="${Math.abs(toMm(p, l.widthUnit) - l.widthMm) < 0.5}">${p} ${l.widthUnit === 'inch' ? 'in' : 'cm'}</button>`).join('')}</div>
        ${l.layout !== 'open' ? `<p class="hint">${tx('folded_width_hint')}</p>` : ''}
        ${groups.length > 1 ? `<label class="st-field">${tx('fabric_for_marker')}<select id="st-material">${groups.map(g => `<option value="${esc(g)}" ${g === l.material ? 'selected' : ''}>${tx(`material_${g}` as StyleKey) || esc(g)} · ${state.doc!.pieces.filter(p => (p.material || 'shell') === g).length} ${tx('pieces_word')}</option>`).join('')}</select></label><p class="hint">${tx('fabric_for_marker_hint')}</p>` : ''}`)}
      ${card(tx('spread_mode'), radioCards('layout', [['open', LAYOUT_ART.open, t('layout_open')], ['folded', LAYOUT_ART.folded, t('layout_folded')], ['tubular', LAYOUT_ART.tubular, t('layout_tubular')]], l.layout))}
      ${card(tx('face'), radioCards('face', [['up', FACE_ART.up, t('face_up')], ['down', FACE_ART.down, t('face_down')], ['face_to_face', FACE_ART.face_to_face, t('face_to_face')]], l.face) + `<p class="hint">${tx('face_hint')}</p>`)}
      ${card(tx('nap'), radioCards('nap', [['none', NAP_ART.none, t('nap_none')], ['one_way', NAP_ART.one_way, t('nap_one_way')], ['one_way_per_bundle', NAP_ART.one_way_per_bundle, t('nap_per_garment')]], l.nap) + `<label class="check"><input type="checkbox" id="st-flip" ${l.allowFlip ? 'checked' : ''}>${tx('allow_flip')}</label>`)}
    </div>
    ${card(tx('plies_ratio'), `<div class="st-row"><div class="st-field"><span id="st-plies-label">${tx('plies')}</span><span class="st-stepper" role="group" aria-labelledby="st-plies-label"><button type="button" class="icon-btn" data-plies="-${even ? 2 : 1}" aria-label="${tx('fewer_plies')}">${icon('minus')}</button><input id="st-plies" type="number" min="1" max="500" step="${even ? 2 : 1}" value="${l.plies}" aria-labelledby="st-plies-label" ${S.layProblem(l) === 'lay_plies' ? 'aria-invalid="true"' : ''}><button type="button" class="icon-btn" data-plies="${even ? 2 : 1}" aria-label="${tx('more_plies')}">${icon('plus')}</button></span></div>
        ${lengthField('st-end', tx('end_allowance'), l.endAllowanceMm)}<p class="hint">${even ? tx('plies_even') : tx('plies_hint')}</p></div>
      <div class="table-scroll"><table class="st-ratio"><caption>${tx('ratio_caption')}</caption><thead><tr><th scope="col">${tx('size')}</th><th scope="col">${tx('in_marker')}</th><th scope="col">${tx('order_qty')}</th><th scope="col">${tx('lays_needed')}</th></tr></thead><tbody>
      ${state.doc!.sizes.map(s => { const r = l.ratio[s] ?? 0, o = l.order[s] ?? 0, per = r * spreads; return `<tr><th scope="row">${esc(s)}</th><td><span class="st-stepper"><button type="button" class="icon-btn small" data-ratio="${esc(s)}" data-delta="-1" aria-label="${tx('fewer_size', { size: s })}">${icon('minus')}</button><output aria-label="${tx('in_marker')} ${esc(s)}">${r}</output><button type="button" class="icon-btn small" data-ratio="${esc(s)}" data-delta="1" aria-label="${tx('more_size', { size: s })}">${icon('plus')}</button></span></td>
        <td><input class="st-order" type="number" min="0" max="1000000" step="1" data-order="${esc(s)}" value="${o}" aria-label="${tx('order_qty')} ${esc(s)}"></td><td>${o > 0 && per > 0 ? (o / per).toFixed(1) : '—'}</td></tr>`; }).join('')}</tbody></table></div>
      <p class="st-summary">${tx('lay_summary', { garments: gm, plies: l.plies, total: gm * spreads })}</p>`)}`;
  }

  // ---------- 5 Gaps ----------
  function gapsHtml(): string {
    const g = state.gaps, d = state.doc!;
    return `<div class="st-grid gaps">
      ${card(tx('gaps_all'), `<div class="st-fields">${lengthField('st-piece-gap', tx('piece_gap'), g.pieceGap)}${lengthField('st-edge-gap', tx('edge_gap'), g.edgeGap)}${lengthField('st-end2', tx('end_allowance'), state.lay.endAllowanceMm)}</div>
        <p class="hint">${tx('gaps_hint')}</p>
        <div class="table-scroll"><table class="st-piece-gaps"><caption>${tx('gaps_per_piece')}</caption><thead><tr><th scope="col">${tx('piece')}</th><th scope="col">${tx('buffer')} (${uName()})</th><th scope="col">${tx('fusing_block')}</th><th scope="col">${tx('block_margin')} (${uName()})</th></tr></thead><tbody>
        ${d.pieces.map(p => `<tr><th scope="row">${esc(p.name)}</th><td><input inputmode="decimal" data-buffer="${esc(p.id)}" value="${esc(formatLength(g.buffers[p.id] ?? 0, unit()))}" aria-label="${tx('buffer')} ${esc(p.name)}"></td><td><label class="check st-cell-check"><input type="checkbox" data-block-on="${esc(p.id)}" ${g.blocks[p.id] != null ? 'checked' : ''} aria-label="${tx('fusing_block')} ${esc(p.name)}"></label></td><td><input inputmode="decimal" data-block="${esc(p.id)}" value="${esc(formatLength(g.blocks[p.id] ?? 10, unit()))}" ${g.blocks[p.id] == null ? 'disabled' : ''} aria-label="${tx('block_margin')} ${esc(p.name)}"></td></tr>`).join('')}</tbody></table></div>`)}
      ${card(tx('gaps_preview'), `${gapPreview() ? `<svg class="st-gap-preview" id="st-gap-preview" role="img" aria-label="${tx('gaps_preview_aria', { gap: L(g.pieceGap), edge: L(g.edgeGap), end: L(state.lay.endAllowanceMm) })}">${gapPreview()}</svg>` : `<p class="st-empty-note">${tx('preview_empty')}</p>`}<p class="hint">${tx('gaps_preview_hint')}</p>`)}
    </div>`;
  }
  /** Illustration only: two real piece outlines (base size) side by side with the gaps drawn to scale. */
  function gapPreview(): string {
    const pieces = (base?.pieces ?? []).filter(p => p.graded).map(p => ({ id: p.id, outline: p.graded!.piece.outline })).slice(0, 2);
    if (!pieces.length) return '';
    const g = state.gaps, end = state.lay.endAllowanceMm, edge = g.edgeGap;
    let x = end, out = '', height = 0;
    const placed = pieces.map(p => {
      const b = boundsOf(p.outline)!, buf = g.buffers[p.id] ?? 0, blk = g.blocks[p.id];
      const pad = Math.max(buf, blk ?? 0), w = b[2] - b[0], h = b[3] - b[1];
      const ox = x + pad - b[0], oy = edge + pad - b[1];
      const item = { ...p, b, ox, oy, buf, blk, w, h, pad };
      x += w + 2 * pad + g.pieceGap; height = Math.max(height, h + 2 * pad); return item;
    });
    const length = x - g.pieceGap + end, width = height + 2 * edge + 40;
    const f = (n: number) => n.toFixed(1), unitPx = Math.max(length, width) / 400;
    out += `<rect class="st-fabric" x="0" y="${-width}" width="${f(length)}" height="${f(width)}"/>`;
    out += `<rect class="st-end" x="0" y="${-width}" width="${f(end)}" height="${f(width)}"/><rect class="st-end" x="${f(length - end)}" y="${-width}" width="${f(end)}" height="${f(width)}"/>`;
    out += `<line class="st-edge-line" x1="0" x2="${f(length)}" y1="${-edge}" y2="${-edge}"/>`;
    for (const p of placed) {
      out += `<polygon class="st-gap-piece" points="${p.outline.map(([a, b]) => `${f(a + p.ox)},${f(-(b + p.oy))}`).join(' ')}"/>`;
      if (p.buf > 0) out += `<rect class="st-buffer" x="${f(p.b[0] + p.ox - p.buf)}" y="${f(-(p.b[3] + p.oy + p.buf))}" width="${f(p.w + 2 * p.buf)}" height="${f(p.h + 2 * p.buf)}" rx="${f(p.buf)}"/>`;
      if (p.blk != null) out += `<rect class="st-block-rect" x="${f(p.b[0] + p.ox - p.blk)}" y="${f(-(p.b[3] + p.oy + p.blk))}" width="${f(p.w + 2 * p.blk)}" height="${f(p.h + 2 * p.blk)}"/>`;
    }
    const dim = (x1: number, x2: number, y: number, label: string) => `<g class="st-dim"><line x1="${f(x1)}" x2="${f(x2)}" y1="${f(y)}" y2="${f(y)}" marker-start="url(#st-arr)" marker-end="url(#st-arr)"/><text x="${f((x1 + x2) / 2)}" y="${f(y - unitPx * 4)}" font-size="${f(unitPx * 11)}" text-anchor="middle">${esc(label)}</text></g>`;
    const a = placed[0]!, b = placed[1];
    const yTop = -width + unitPx * 16;
    out += dim(0, end, yTop, t('dim_end', { v: L(end) }));
    if (b) out += dim(a.ox + a.b[2] + a.pad, b.ox + b.b[0] - b.pad, -(a.oy + a.h / 2), t('dim_gap', { v: L(g.pieceGap) }));
    out += `<g class="st-dim"><line x1="${f(length - end - unitPx * 20)}" x2="${f(length - end - unitPx * 20)}" y1="0" y2="${f(-edge)}"/><text x="${f(length - end - unitPx * 20)}" y="${f(unitPx * 13)}" font-size="${f(unitPx * 11)}" text-anchor="end">${esc(t('dim_edge', { v: L(edge) }))}</text></g>`;
    const view = `${f(-unitPx * 10)} ${f(-width - unitPx * 10)} ${f(length + unitPx * 20)} ${f(width + unitPx * 28)}`;
    return `<defs><marker id="st-arr" viewBox="0 0 10 10" refX="5" refY="5" markerWidth="5" markerHeight="5" orient="auto-start-reverse"><path d="M0 0L10 5L0 10z" fill="currentColor"/></marker></defs><g data-view="${view}">${out}</g>`;
  }

  // ---------- 6 Stripes and checks ----------
  function stripesHtml(): string {
    const r = state.repeat, d = state.doc!, problem = S.repeatProblem(r);
    const needX = r.kind === 'checks' || r.direction === 'across', needY = r.kind === 'checks' || r.direction === 'along';
    const settings = r.kind === 'none' ? `<p class="hint">${tx('repeat_none_hint')}</p>` : `
      ${r.kind === 'stripes' ? `<div class="segmented" role="group" aria-label="${tx('stripe_direction')}"><button type="button" data-direction="along" aria-pressed="${r.direction === 'along'}">${tx('stripes_along')}</button><button type="button" data-direction="across" aria-pressed="${r.direction === 'across'}">${tx('stripes_across')}</button></div>` : ''}
      <div class="st-fields">${needY ? lengthField('st-period-y', tx('repeat_across'), r.periodY) : ''}${needX ? lengthField('st-period-x', tx('repeat_along'), r.periodX) : ''}
        ${needY ? lengthField('st-offset-y', tx('offset_across'), r.offsetY) : ''}${needX ? lengthField('st-offset-x', tx('offset_along'), r.offsetX) : ''}${lengthField('st-tolerance', tx('match_tolerance'), r.tolerance)}</div>
      ${problem ? `<p class="error-text">${tx(`repeat_${problem}` as StyleKey)}</p>` : ''}
      <h3>${tx('colours')}</h3><ul class="st-bands">${r.bands.map((b, i) => `<li><input type="color" value="${esc(b.colour)}" data-band-colour="${i}" aria-label="${tx('band_colour', { n: i + 1 })}">${lengthField(`st-band-w-${i}`, tx('band_width'), b.width_mm, `data-band-width="${i}"`)}${lengthField(`st-band-o-${i}`, tx('band_offset'), b.offset_mm, `data-band-offset="${i}"`)}${r.kind === 'checks' ? `<label class="st-field">${tx('band_axis')}<select data-band-axis="${i}"><option value="y" ${b.axis === 'y' ? 'selected' : ''}>${tx('stripes_along')}</option><option value="x" ${b.axis === 'x' ? 'selected' : ''}>${tx('stripes_across')}</option></select></label>` : ''}<button type="button" class="icon-btn ghost" data-remove-band="${i}" aria-label="${tx('remove_band', { n: i + 1 })}">${icon('trash')}</button></li>`).join('')}</ul>
      <button type="button" class="small" data-st="add-band">${icon('plus')}${tx('add_colour')}</button>`;
    const opts = (cur: string) => d.pieces.map(p => `<option value="${esc(p.id)}" ${p.id === cur ? 'selected' : ''}>${esc(p.name)}</option>`).join('');
    matchPick.pieceA ||= d.pieces[0]?.id ?? ''; matchPick.pieceB ||= d.pieces[1]?.id ?? d.pieces[0]?.id ?? '';
    const matches = r.kind === 'none' ? '' : card(tx('match_points'), `<p class="hint">${tx('match_hint')}</p>
      <div class="st-match-pick"><div><label class="st-field">${tx('piece_a')}<select id="st-match-a">${opts(matchPick.pieceA)}</select></label><svg class="st-match-svg" id="st-match-svg-a" role="group" aria-label="${tx('match_svg', { side: 'A' })}">${matchSvg(matchPick.pieceA, 'a')}</svg></div>
        <div><label class="st-field">${tx('piece_b')}<select id="st-match-b">${opts(matchPick.pieceB)}</select></label><svg class="st-match-svg" id="st-match-svg-b" role="group" aria-label="${tx('match_svg', { side: 'B' })}">${matchSvg(matchPick.pieceB, 'b')}</svg></div></div>
      <p class="st-match-step" role="status">${matchPick.a ? tx('match_pick_b', { point: matchPick.a.point }) : tx('match_pick_a')}</p>
      <label class="st-field st-inline">${tx('match_axes')}<select id="st-match-axes">${(r.kind === 'checks' ? ['both', 'y', 'x'] : r.direction === 'along' ? ['y'] : ['x']).map(a => `<option value="${a}">${tx(`axes_${a}` as StyleKey)}</option>`).join('')}</select></label>
      ${matchListHtml()}`, 'st-matches');
    return `<div class="st-grid two">
      ${card(tx('repeat_kind'), radioCards('repeat', [['none', REPEAT_ART.none, t('repeat_none')], ['stripes', REPEAT_ART.stripes, t('repeat_stripes')], ['checks', REPEAT_ART.checks, t('repeat_checks')]], r.kind) + settings)}
      ${card(tx('overlay_pieces'), `<label class="check"><input type="checkbox" id="st-overlay-sizes" ${layers.sizes ? 'checked' : ''}>${tx('overlay_sizes')}</label>${base?.pieces.some(p => p.graded) ? `<svg class="st-stripe-preview" id="st-stripe-preview" role="img" aria-label="${tx('overlay_pieces_aria')}">${stripePreview()}</svg>` : `<p class="st-empty-note">${tx('preview_empty')}</p>`}<p class="hint">${tx('overlay_pieces_hint')}</p>`)}
    </div>${matches}`;
  }
  function bandsFor(axis: 'x' | 'y'): RepeatBand[] {
    const own = state.repeat.bands.filter(b => b.axis === axis);
    return own.length ? own : [{ axis, offset_mm: 0, width_mm: 1.2, colour: '#3865ac' }];
  }
  /** Piece preview with the repeat drawn over it. Grain runs up the page, so "along the length" stripes are vertical. */
  function stripePreview(): string {
    const r = state.repeat; if (!base) return '';
    const pieces = base.pieces.filter(p => p.graded).slice(0, 4);
    let x = 0, out = '', defs = '';
    const b0 = pieces.map(p => boundsOf(p.graded!.piece.outline)!);
    const top = Math.max(...b0.map(b => b[3] - b[1]));
    pieces.forEach((p, i) => {
      const b = b0[i]!, dx = x - b[0], dy = -b[1];
      const path = p.graded!.piece.outline.map(([a, c]) => `${(a + dx).toFixed(1)},${(-(c + dy)).toFixed(1)}`).join(' ');
      defs += `<clipPath id="st-clip-${i}"><polygon points="${path}"/></clipPath>`;
      out += `<polygon class="st-stripe-piece" points="${path}"/>`;
      if (r.kind !== 'none') {
        let lines = '';
        const w = b[2] - b[0], h = b[3] - b[1];
        if (r.kind === 'checks' || r.direction === 'along') for (const band of bandsFor('y')) for (let k = Math.floor(-w / r.periodY) - 1; k * r.periodY < 2 * w && r.periodY > 0.5; k++) { const lx = x + r.offsetY + band.offset_mm + k * r.periodY; lines += `<line x1="${lx.toFixed(1)}" x2="${lx.toFixed(1)}" y1="${-h - 10}" y2="10" style="stroke:${esc(band.colour)}" stroke-width="${band.width_mm}"/>`; }
        if (r.kind === 'checks' || r.direction === 'across') for (const band of bandsFor('x')) for (let k = -1; k * r.periodX < h + r.periodX && r.periodX > 0.5; k++) { const ly = -(r.offsetX + band.offset_mm + k * r.periodX); lines += `<line x1="${x - 10}" x2="${x + w + 10}" y1="${ly.toFixed(1)}" y2="${ly.toFixed(1)}" style="stroke:${esc(band.colour)}" stroke-width="${band.width_mm}"/>`; }
        out += `<g clip-path="url(#st-clip-${i})">${lines}</g>`;
      }
      if (layers.sizes) out += (state.doc?.sizes ?? []).filter(s => s !== state.doc!.base_size).map(s => {
        const g = graded[s]?.find(q => q.piece.id === p.id || q.piece.id === `${p.id}-${s}`); if (!g) return '';
        const gb = boundsOf(g.piece.outline)!, sx = x + (b[2] - b[0]) / 2 - (gb[0] + gb[2]) / 2, sy = -gb[1];
        return `<polygon class="st-stripe-size" style="stroke:var(--size-${state.doc!.sizes.indexOf(s) % 6})" points="${g.piece.outline.map(([a, c]) => `${(a + sx).toFixed(1)},${(-(c + sy)).toFixed(1)}`).join(' ')}"><title>${esc(s)}</title></polygon>`;
      }).join('');
      out += `<polygon class="st-stripe-outline" points="${path}"/><text x="${(x + (b[2] - b[0]) / 2).toFixed(1)}" y="${(-(b[3] - b[1]) - 14).toFixed(1)}" text-anchor="middle" class="st-stripe-name">${esc(state.doc?.pieces.find(q => q.id === p.id)?.name ?? p.id)}</text>`;
      x += b[2] - b[0] + 60;
    });
    return `<defs>${defs}</defs><g data-view="${-20} ${(-top - 40).toFixed(1)} ${(x + 20).toFixed(1)} ${(top + 60).toFixed(1)}">${out}</g>`;
  }
  function matchSvg(pieceId: string, side: 'a' | 'b'): string {
    const p = base?.pieces.find(q => q.id === pieceId)?.graded; if (!p) return '';
    const b = boundsOf(p.piece.outline)!, pad = Math.max(b[2] - b[0], b[3] - b[1]) * 0.08, r = Math.max(b[2] - b[0], b[3] - b[1]) / 45;
    const chosen = side === 'a' ? matchPick.a?.point : '';
    const anchors = new Set(Object.values(state.doc?.pieces.find(q => q.id === pieceId)?.anchors ?? {}));
    return `<g data-view="${b[0] - pad} ${-b[3] - pad} ${b[2] - b[0] + 2 * pad} ${b[3] - b[1] + 2 * pad}"><polygon class="st-stripe-piece" points="${pts(p.piece.outline)}"/><polygon class="st-match-seam" points="${pts(p.seam)}"/>
      ${[...outlineIds(state.doc!, pieceId)].flatMap(id => { const at = base?.objects[id]?.points[0]; return at ? [[id, at] as const] : []; }).map(([id, at]) => `<circle id="st-mp-${side}-${esc(id.replace(/[^\w-]/g, '_'))}" class="st-match-point${id === chosen ? ' selected' : ''}${anchors.has(id) ? ' anchored' : ''}" data-match-side="${side}" data-match-point="${esc(id)}" cx="${at[0]}" cy="${-at[1]}" r="${r}" tabindex="0" role="button" aria-label="${tx('match_point_aria', { point: id })}"><title>${esc(id)}</title></circle>`).join('')}</g>`;
  }
  /** Rule reports for the current marker, or null when there is no checked marker. */
  function reports(): RuleReport[] | null {
    const m = state.marker; if (m.status !== 'done' || !m.overlay) return null;
    return ruleReports(state.repeat.matches.length, m.job, (m.overlay as { matches?: MarkerMatch[] }).matches ?? []);
  }
  const pointName = (pieceId: string, anchor: string) => `${pieceName(pieceId)} · ${human(anchor.replace(/^match_\d+([ab])$/, (_, side: string) => `${t('match_point_word')} ${side.toUpperCase()}`))}`;
  function ruleText(m: MatchRule): string { return `${esc(pointName(m.a.piece_id, m.a.point_id))} ↔ ${m.b ? esc(pointName(m.b.piece_id, m.b.point_id)) : tx('fabric_repeat')}`; }
  function ruleChip(r: RuleReport | undefined): string {
    if (!r) return `<span class="chip">${tx('match_unchecked')}</span>`;
    if (state.marker.stale) return `<span class="chip warn">${icon('alert')}${tx('match_stale')}</span>`;
    if (r.state === 'not_placed') return `<span class="chip warn">${icon('alert')}${tx('match_not_placed')}</span>`;
    if (r.state === 'matched') return `<span class="chip ok">${icon('check')}${r.maxMm != null ? tx('match_ok_mm', { total: r.checked, mm: MM(r.maxMm) }) : tx('match_ok', { total: r.checked })}</span>`;
    return `<span class="chip bad">${icon('alert')}${r.worstMm != null ? tx('match_off_mm', { mm: MM(r.worstMm), n: r.off, total: r.checked }) : tx('match_bad_plain', { n: r.off, total: r.checked })}</span>`;
  }
  function matchListHtml(): string {
    const list = state.repeat.matches, rep = reports();
    if (!list.length) return `<div class="st-empty-note">${icon('stripes')}<div><strong>${tx('no_matches')}</strong><span>${tx('no_matches_hint')}</span></div></div>`;
    const counts = rep ? { ok: rep.filter(r => r.state === 'matched').length, off: rep.filter(r => r.state === 'off').length, none: rep.filter(r => r.state === 'not_placed').length } : null;
    const summary = !rep ? tx('match_summary_unchecked') : state.marker.stale ? tx('match_summary_stale') : tx('match_summary', { ok: counts!.ok, off: counts!.off, none: counts!.none, total: list.length });
    return `<div class="st-match-head"><h3>${tx('match_rules_title')}</h3><p class="hint">${summary}</p>
      <ul class="st-match-key" aria-label="${tx('legend')}"><li><i class="ok"></i>${tx('key_matched')}</li><li><i class="bad"></i>${tx('key_off')}</li><li><i class="warn"></i>${tx('key_not_placed')}</li></ul></div>
      <ol class="st-match-list">${list.map((m, i) => { const r = rep?.[i], state_ = !r ? 'unchecked' : r.state;
        return `<li class="st-rule ${state_}"><span class="st-rule-no" aria-hidden="true">${i + 1}</span><span class="st-rule-what">${ruleText(m)}<small>${tx(`axes_${m.axes}` as StyleKey)}</small></span>${ruleChip(r)}
          <button type="button" class="small" data-show-rule="${i}" ${r && r.state !== 'not_placed' ? '' : 'disabled'} aria-label="${tx('show_on_marker_n', { n: i + 1 })}">${icon('marker')}${tx('show_on_marker')}</button>
          <button type="button" class="icon-btn ghost" data-remove-match="${i}" aria-label="${tx('remove_match', { n: i + 1 })}">${icon('trash')}</button></li>`; }).join('')}</ol>
      ${!rep ? `<p class="hint">${tx('match_check_hint')}</p>` : ''}`;
  }
  async function pickMatch(side: 'a' | 'b', point: string): Promise<void> {
    if (side === 'a') { matchPick.a = { piece: matchPick.pieceA, point }; render(); say(t('match_pick_b', { point })); return; }
    if (!matchPick.a) { say(t('match_pick_a')); return; }
    const a = matchPick.a, b = { piece: matchPick.pieceB, point }, d = state.doc!;
    if (a.piece === b.piece && a.point === b.point) { host.message(t('err_same_point'), true); return; }
    const n = state.repeat.matches.length + 1, name = `match_${n}`;
    const pa = d.pieces.find(p => p.id === a.piece)!, pb = d.pieces.find(p => p.id === b.piece)!;
    const nameA = Object.entries(pa.anchors ?? {}).find(([, v]) => v === a.point)?.[0] ?? `${name}a`;
    const nameB = Object.entries(pb.anchors ?? {}).find(([, v]) => v === b.point)?.[0] ?? `${name}b`;
    const ops: StyleOperation[] = a.piece === b.piece ? [{ op: 'put_piece', replace: true, piece: { ...pa, anchors: { ...pa.anchors, [nameA]: a.point, [nameB]: b.point } } }]
      : [{ op: 'put_piece', replace: true, piece: { ...pa, anchors: { ...pa.anchors, [nameA]: a.point } } }, { op: 'put_piece', replace: true, piece: { ...pb, anchors: { ...pb.anchors, [nameB]: b.point } } }];
    if (!(await commit(ops, t('match_added')))) return;
    const rule: MatchRule = { a: { piece_id: a.piece, point_id: nameA }, b: { piece_id: b.piece, point_id: nameB }, axes: ((document.getElementById('st-match-axes') as HTMLSelectElement | null)?.value ?? 'y') as MatchRule['axes'] };
    matchPick.a = undefined; update(S.setRepeat(state, { matches: [...state.repeat.matches, rule] }));
    say(t('match_added'));
  }

  // ---------- 7 Marker ----------
  function markerHtml(): string {
    const m = state.marker, l = state.lay, running = m.status === 'running';
    const chips = [
      [t('chip_width', { v: formatLength(l.widthMm, l.widthUnit === 'inch' ? 'inch' : 'cm', true) }), 'lay'], [t(`layout_${l.layout}`), 'lay'], [t('chip_plies', { n: l.plies }), 'lay'],
      [t('chip_ratio', { r: Object.entries(l.ratio).filter(([, n]) => n > 0).map(([s, n]) => `${s}:${n}`).join(' ') }), 'lay'],
      [t('chip_gap', { v: L(state.gaps.pieceGap) }), 'gaps'], [state.repeat.kind === 'none' ? t('repeat_none') : t('chip_repeat', { kind: t(`repeat_${state.repeat.kind}`), n: state.repeat.matches.length }), 'stripes'],
      ...(l.material ? [[t('chip_fabric', { v: t(`material_${l.material}` as StyleKey) || l.material }), 'lay'] as [string, string]] : []),
    ] as Array<[string, S.Step]>;
    const r = m.result, rep = m.report, ov = m.overlay as (typeof m.overlay & { matches?: MarkerMatch[] }) | undefined;
    const matched = ov?.matches ?? [], bad = matched.filter(x => !x.pass).length;
    const practice = isPracticeResult(r);
    const someNotMatched = !!r && state.repeat.matches.length > 0 && bad > 0;
    const stats = r ? `<div class="st-stats" aria-label="${tx('marker_stats')}">
        <div><span>${tx('marker_length')}</span><strong>${metres(r.length, 3)}<small>m</small></strong></div>
        <div><span>${tx('efficiency')}</span><strong>${percent(r.efficiency)}<small>%</small></strong></div>
        <div><span>${tx('per_garment')}</span><strong>${S.garmentsPerMarker(l) ? metres(r.length / S.garmentsPerMarker(l), 3) : '—'}<small>m</small></strong></div>
        <div><span>${tx('spread_length')}</span><strong>${metres(ov?.spread_length_mm ?? r.length + 2 * l.endAllowanceMm, 3)}<small>m</small></strong></div>
        <div><span>${tx('checks')}</span><strong class="${rep && !rep.ok ? 'error-text' : ''}">${rep ? rep.ok ? tx('checks_ok') : tx('checks_bad', { n: rep.errors.length }) : '—'}</strong></div>
        ${matched.length ? `<div><span>${tx('stripe_match')}</span><strong class="${bad ? 'error-text' : ''}">${bad ? tx('matched_some', { ok: matched.length - bad, total: matched.length }) : tx('matched_all', { total: matched.length })}</strong></div>` : ''}</div>` : '';
    return `<div class="st-marker">
      <div class="st-marker-top"><ul class="st-chips">${chips.map(([text, step]) => `<li><button type="button" class="chip" data-goto="${step}" aria-label="${tx('change_setting', { setting: text })}">${esc(text)}</button></li>`).join('')}</ul>
        <div class="st-row"><div class="segmented" role="group" aria-label="${tx('search_time')}">${BUDGETS.map(([k, s]) => `<button type="button" data-budget="${s}" aria-pressed="${s === budget}" ${running ? 'disabled' : ''}>${tx(k)}</button>`).join('')}</div>
        ${running ? `<button type="button" class="danger" data-st="stop">${icon('stop')}${tx('stop_use_best')}</button>` : `<button type="button" class="primary large" data-st="nest" id="st-nest-run">${icon('play')}${r ? tx('nest_again') : tx('nest')}</button>`}</div></div>
      ${running ? `<div class="st-progress" role="status" aria-live="polite"><progress max="1" value="${Math.min(1, progress.elapsed / budget)}" aria-label="${tx('search_time')}"></progress><span>${tx('searching', { s: progress.elapsed.toFixed(0), total: budget })}${progress.best ? ` · ${tx('best_so_far', { m: metres(progress.best.length, 3), e: percent(progress.best.efficiency) })}` : ''}</span></div>` : ''}
      ${(m.status === 'failed' || m.error) && !running ? `<div class="notice error st-note" role="alert">${icon('alert')}<div><strong>${tx('nest_failed')}</strong> ${esc(m.error)}<br><span>${tx('nest_failed_hint')}</span></div></div>` : ''}
      ${m.stale && r ? `<p class="notice warn st-note">${icon('alert')}${tx('marker_stale')}</p>` : ''}
      ${practice ? `<p class="notice warn st-note">${icon('alert')}${tx('practice_marker')}</p>` : ''}
      ${someNotMatched ? `<p class="notice warn st-note">${icon('alert')}${tx('browser_no_match')}</p>` : ''}
      ${r ? focusBanner() : ''}
      <div class="st-marker-canvas">${r ? `<div class="st-layers"><div class="st-layer-checks" role="group" aria-label="${tx('overlays')}">${layerSwitches()}</div>
          <ul class="st-legend" aria-label="${tx('legend')}">${markerLegend()}</ul>
          <button type="button" class="small ghost st-edit-hand" data-st="open-editor" title="${tx('open_editor_hint')}" aria-label="${tx('open_editor')}">${icon('marker')}<span>${tx('open_editor')}</span></button></div>` : ''}${r ? `<svg id="st-marker-svg" role="img" data-match-source="${matchSource}" aria-label="${tx('marker_aria', { m: metres(r.length, 3), e: percent(r.efficiency) })}">${markerSvg()}</svg>` : running ? `<div class="canvas-empty">${icon('marker')}<strong>${tx('marker_working')}</strong><span>${tx('marker_working_text')}</span></div>` : `<div class="canvas-empty">${icon('marker')}<strong>${tx('no_marker')}</strong><span>${tx('no_marker_text')}</span></div>`}</div>
      ${stats}
      ${rep && !rep.ok ? `<details class="advanced"><summary>${tx('check_details')}</summary><ul>${rep.errors.map(e => `<li><strong>${esc(e.code)}</strong> ${esc(e.message)}</li>`).join('')}</ul></details>` : ''}</div>`;
  }
  /** Size of the marker drawing on screen. The height keeps the stats in view at 1366 x 768. */
  function markerBox(): [number, number] {
    const el = document.getElementById('st-marker-svg');
    if (el && el.clientWidth > 50 && el.clientHeight > 50) return [el.clientWidth, el.clientHeight];
    return [Math.max(320, (body.clientWidth || window.innerWidth - 120) - 48), Math.max(200, window.innerHeight - 460)];
  }
  /** The drawing fills the free height, so its true size is known only after layout: redraw when it changes. */
  let markerSize = '';
  function redrawMarker(): void {
    const el = document.getElementById('st-marker-svg'); if (!el || state.step !== 'marker' || !state.marker.result) return;
    const size = markerBox().join('x'); if (size === markerSize && el.childElementCount) return;
    markerSize = size; el.innerHTML = markerSvg();
    const key = document.getElementById('st-stripe-key'); if (key) key.innerHTML = stripeKey();
  }
  new ResizeObserver(() => window.requestAnimationFrame(redrawMarker)).observe(body);
  /** Millimetres per screen pixel in the marker drawing. */
  function markerScale(): number {
    const m = state.marker, r = m.result!, W = m.job!.fabric.width, [w, h] = markerBox();
    return Math.max((r.length * 1.02) / w, (W * 1.04) / h);
  }
  /** Overlay switches, each with its key swatch (stripes, match rings, previous marker). */
  function layerSwitches(): string {
    const m = state.marker, ov = m.overlay as (typeof m.overlay & { matches?: MarkerMatch[] }) | undefined;
    const sw = (k: 'stripes' | 'matches' | 'previous', key: string, text: string, disabled = false) => `<label class="check st-key"><input type="checkbox" data-layer="${k}" ${layers[k] ? 'checked' : ''} ${disabled ? 'disabled' : ''}>${key}<span${k === 'stripes' ? ' id="st-stripe-key"' : ''}>${text}</span></label>`;
    let out = '';
    if (state.repeat.kind !== 'none') out += sw('stripes', `<i class="st-key-line" style="background:${esc(ov?.stripes[0]?.colour ?? 'var(--repeat)')}"></i>`, stripeKey());
    if (state.repeat.matches.length) {
      out += sw('matches', '<i class="st-key-ring ok"></i>', tx('layer_matches'));
      if (ov?.matches?.some(x => placed(x) && !x.pass)) out += `<span class="st-key-off"><i class="st-key-ring bad"></i>${tx('key_off')}</span>`;
    }
    return out + sw('previous', '<i class="st-key-dash"></i>', tx('layer_previous'), !m.previous);
  }
  const stripeKey = () => { const n = stripeEvery(); return n > 1 ? tx('legend_stripes_every', { n }) : tx('legend_stripes'); };
  /** Sizes by fill colour, in size-chart order, and the fold edge. */
  function markerLegend(): string {
    const m = state.marker, job = m.job!, present = new Set(job.pieces.flatMap(p => p.size ? [p.size] : [])), all = state.doc?.sizes ?? [...present];
    const sizes = [...all.filter(s => present.has(s)), ...[...present].filter(s => !all.includes(s))];
    const item = (sw: string, text: string) => `<li>${sw}<span>${text}</span></li>`;
    let out = sizes.map(s => item(`<i class="st-key-fill" style="background:var(--fill-${Math.max(0, all.indexOf(s)) % 6})"></i>`, esc(s))).join('');
    if (job.fabric.layout === 'folded' || job.fabric.layout === 'tubular') out += item('<i class="st-key-fold"></i>', tx('legend_fold'));
    return out;
  }
  /** Draw every n-th stripe when the real ones would be closer than a few pixels (they would merge into grey). */
  function stripeEvery(): number {
    const r = state.repeat, period = r.kind === 'checks' ? Math.min(r.periodX, r.periodY) : r.direction === 'along' ? r.periodY : r.periodX;
    return stripeStep(period, markerScale(), 6);
  }
  function focusBanner(): string {
    if (focusRule == null || !state.repeat.matches[focusRule]) return '';
    const r = reports()?.[focusRule];
    return `<div class="notice st-note st-focus" role="status"><span class="st-rule-no">${focusRule + 1}</span><span>${tx('showing_match', { n: focusRule + 1 })} ${ruleText(state.repeat.matches[focusRule]!)}</span>${ruleChip(r)}<button type="button" class="small ghost" data-st="focus-clear">${tx('show_all_matches')}</button></div>`;
  }
  function markerSvg(): string {
    const m = state.marker, r = m.result!, job = m.job!, W = job.fabric.width, ov = m.overlay as (typeof m.overlay & { matches?: MarkerMatch[] }) | undefined;
    const Y = (y: number) => (W - y).toFixed(1), P = (list: Pt[]) => list.map(([x, y]) => `${x.toFixed(1)},${Y(y)}`).join(' ');
    const sizes = state.doc?.sizes ?? [], sizeOf = (id: string) => job.pieces.find(p => p.id === id)?.size ?? '';
    const u = markerScale(), striped = layers.stripes && !!ov?.stripes.length;
    let out = `<rect class="st-m-fabric" x="0" y="0" width="${r.length.toFixed(1)}" height="${W}"/>`;
    if (layers.previous && ov?.previous.length) out += ov.previous.filter(p => p.before.length).map(p => `<polygon class="st-m-prev ${p.kind}" points="${P(p.before)}"/>`).join('');
    out += (ov?.outlines ?? []).map(o => `<polygon class="st-m-piece${striped ? ' striped' : ''}" points="${P(o.points)}" style="fill:var(--fill-${Math.max(0, sizes.indexOf(sizeOf(o.piece_id))) % 6})"><title>${esc(o.piece_id)}</title></polygon>`).join('');
    if (striped) {
      // Stripe lines from the engine (clipped to the pieces). Dense ones are thinned so each drawn line stays visible.
      const n = stripeEvery(), rp = state.repeat, minW = u * 1.1;
      const keep = (line: { points: [Pt, Pt] }) => {
        if (n === 1) return true;
        const horizontal = Math.abs(line.points[1][1] - line.points[0][1]) < Math.abs(line.points[1][0] - line.points[0][0]);
        const at = horizontal ? line.points[0][1] : line.points[0][0], period = horizontal ? rp.periodY : rp.periodX, offset = horizontal ? rp.offsetY : rp.offsetX;
        return period > 0 ? ((Math.round((at - offset) / period) % n) + n) % n === 0 : true;
      };
      out += `<g class="st-m-stripes">${ov!.stripes.filter(keep).map(line => `<line class="st-m-stripe" x1="${line.points[0][0].toFixed(1)}" y1="${Y(line.points[0][1])}" x2="${line.points[1][0].toFixed(1)}" y2="${Y(line.points[1][1])}" style="stroke:${esc(line.colour)}" stroke-width="${Math.max(line.width_mm, minW).toFixed(2)}"/>`).join('')}</g>`;
    }
    out += (ov?.outlines ?? []).map(o => `<polygon class="st-m-edge" points="${P(o.points)}"/>`).join('');
    if (job.fabric.layout === 'folded' || job.fabric.layout === 'tubular') out += `<line class="st-m-fold" x1="0" x2="${r.length}" y1="${Y(0)}" y2="${Y(0)}"/>${job.fabric.layout === 'tubular' ? `<line class="st-m-fold" x1="0" x2="${r.length}" y1="0" y2="0"/>` : ''}`;
    if (layers.matches) {
      const styleRules = state.repeat.matches.length, rr = u * 6.5;
      for (const x of (ov?.matches ?? []).filter(placed)) {
        const rule = styleRuleOf(job, x.rule, styleRules), focus = focusRule == null ? '' : rule === focusRule ? ' focus' : ' dim';
        const label = x.pass ? t('match_point_ok') : x.error_mm != null && Number.isFinite(x.error_mm) ? t('match_point_bad', { mm: MM(x.error_mm) }) : t('match_point_bad_plain');
        const ring = (p: Pt) => `<circle class="halo" cx="${p[0].toFixed(1)}" cy="${Y(p[1])}" r="${rr.toFixed(1)}"/><circle class="ring" cx="${p[0].toFixed(1)}" cy="${Y(p[1])}" r="${rr.toFixed(1)}"/><circle class="dot" cx="${p[0].toFixed(1)}" cy="${Y(p[1])}" r="${(u * 1.8).toFixed(1)}"/>`;
        const onFabric = (p: Pt) => p[0] >= -1 && p[0] <= r.length + 1 && p[1] >= -1 && p[1] <= W + 1;
        const link = x.b && focus === ' focus' && onFabric(x.a) && onFabric(x.b) ? `<line class="link" x1="${x.a[0].toFixed(1)}" y1="${Y(x.a[1])}" x2="${x.b[0].toFixed(1)}" y2="${Y(x.b[1])}"/>` : '';
        out += `<g class="st-m-match ${x.pass ? 'ok' : 'bad'}${focus}">${link}${ring(x.a)}${x.b ? ring(x.b) : ''}<title>${esc(`${t('match_n', { n: rule + 1 })}: ${label}`)}</title></g>`;
      }
    }
    return `<g data-view="${(-u * 10).toFixed(1)} ${(-u * 10).toFixed(1)} ${(r.length + u * 20).toFixed(1)} ${(W + u * 20).toFixed(1)}">${out}</g>`;
  }
  /** Per-garment match results: the engine's match report (overlay or validator), else pass/fail from validator issues. */
  let matchSource: 'engine' | 'validator' | '' = '';
  async function attachMatches(overlay: unknown, report: ValidationReport | undefined, job: Job, result: MarkerResult): Promise<void> {
    const ov = overlay as { matches?: MarkerMatch[] } | undefined; if (!ov) return;
    const fromReport = (report as { matches?: MarkerMatch[] } | undefined)?.matches;
    matchSource = ov.matches || fromReport ? 'engine' : 'validator';
    ov.matches ??= fromReport ?? (report && engine ? await engine.matchStatus(job, result, report).catch(() => []) : []);
  }
  async function runNest(): Promise<void> {
    if (!engine || !state.doc || state.marker.status === 'running') return;
    const old = state.marker.status === 'done' && state.marker.job && state.marker.result ? { job: state.marker.job, result: state.marker.result } : state.marker.previous;
    let job: Job;
    try { job = await engine.toJob(state.doc, S.jobSettings(state)); job.meta = { ...job.meta, beta_style_name: state.info.name, beta_style_number: state.info.number }; }
    catch (error) { update({ ...state, marker: { ...state.marker, status: 'failed', error: errorText(error) } }); return; }
    abort = new AbortController(); progress = { elapsed: 0, best: null };
    update({ ...state, marker: { ...state.marker, status: 'running', error: undefined } });
    say(t('nest_started'));
    const started = performance.now(), timer = window.setInterval(() => { progress.elapsed = (performance.now() - started) / 1000; if (state.step === 'marker') renderProgress(); }, 500);
    try {
      const result = await nestMarker(job, { budgetS: budget, signal: abort.signal, practice: practiceRequested(location.search), onProgress: p => { progress.best = p.best; } });
      if (!result) throw new Error(t('nest_none'));
      const report = await engine.validate(job, result).catch(() => undefined);
      const overlay = await engine.overlay(job, result, { ...(old ? { previous: old } : {}) }).catch(error => { host.message(errorText(error), true); return undefined; });
      await attachMatches(overlay, report, job, result);
      update({ ...state, marker: { status: 'done', job, result, report, overlay, previous: old, stale: false } });
      say(t('nest_done', { m: metres(result.length, 3), e: percent(result.efficiency) }));
    } catch (error) {
      update({ ...state, marker: { ...state.marker, status: state.marker.result ? 'done' : 'failed', error: errorText(error), stale: state.marker.stale } });
      host.message(errorText(error), true);
    } finally { window.clearInterval(timer); abort = null; render(); }
  }
  function renderProgress(): void {
    const box = root.querySelector('.st-progress'); if (!box) return;
    box.querySelector('progress')!.value = Math.min(1, progress.elapsed / budget);
    box.querySelector('span')!.textContent = `${t('searching', { s: progress.elapsed.toFixed(0), total: budget })}${progress.best ? ` · ${t('best_so_far', { m: metres(progress.best.length, 3), e: percent(progress.best.efficiency) })}` : ''}`;
  }

  // ---------- 8 Cut ----------
  function cutHtml(): string {
    const m = state.marker, r = m.result!, l = state.lay, f = S.layFabric(r.length, l), gm = S.garmentsPerMarker(l);
    const tile = (label: string, value: string) => `<div><span>${label}</span><strong>${value}</strong></div>`;
    return `<div class="st-stats st-cut-tiles">${tile(tx('marker_length'), `${metres(r.length, 3)}<small>m</small>`)}${tile(tx('spread_length'), `${metres(f.perPlyMm, 3)}<small>m</small>`)}${tile(tx('plies'), String(l.plies))}${tile(tx('fabric_needed'), `${metres(f.totalMm, 2)}<small>m</small>`)}${tile(tx('garments_in_lay'), String(gm * f.spreads))}</div>
      <div class="st-grid two">
      ${card(tx('files_pattern'), `<p class="hint">${tx('files_pattern_hint')}</p><div class="st-files"><button type="button" data-export="pattern-pdf-a4">${icon('print')}${tx('pattern_pdf_a4')}</button><button type="button" data-export="pattern-pdf-a3">${icon('print')}${tx('pattern_pdf_a3')}</button><button type="button" data-export="pattern-dxf">${icon('download')}${tx('pattern_dxf')}</button><button type="button" data-export="pattern-plt">${icon('download')}${tx('pattern_plt')}</button><button type="button" data-st="save-file">${icon('save')}${tx('save_style')}</button></div>`)}
      ${card(tx('files_marker'), `${serverless() ? '' : `<p class="hint">${tx('files_marker_hint')}</p>`}<label class="check"><input type="checkbox" id="st-guides" ${state.repeat.kind === 'none' ? 'disabled' : ''}>${tx('stripe_guides')}</label><div class="st-files"><button type="button" data-export="marker-tiled">${icon('print')}${tx('marker_tiled')}</button>${(['plt', 'pdf', 'dxf'] as const).map(f => `<button type="button" data-export="marker-${f}" ${serverless() ? `disabled aria-describedby="st-needs-server"` : ''}>${icon(f === 'pdf' ? 'print' : 'download')}${tx(`marker_${f}`)}</button>`).join('')}</div>
        ${serverless() ? `<p class="hint" id="st-needs-server">${icon('info')} ${tx('needs_server')}</p>` : ''}
        `)}
      </div>
      ${card(tx('next_in_cutting_room'), `<div class="st-files"><button type="button" class="primary" data-st="cut-plan">${icon('plan')}${tx('make_cut_plan')}</button><button type="button" data-st="spreading">${icon('spread')}${tx('print_spreading')}</button><button type="button" data-st="open-editor">${icon('marker')}${tx('open_editor')}</button><button type="button" data-st="bundles">${icon('bundles')}${tx('make_bundles')}</button></div><p class="hint">${tx('next_in_cutting_room_hint')}</p>`)}`;
  }
  /** No desktop app and no shop server: only the browser's Rust WASM is available (status set by main.ts). */
  const serverless = () => !isDesktop() && (staticMode() || document.getElementById('server-status')?.dataset.floorLabel === 'status_browser');
  async function exportFile(kind: string, button: HTMLButtonElement): Promise<void> {
    if (!engine || !state.doc) return;
    const old = button.innerHTML; button.disabled = true;
    const name = (state.info.number || state.info.name || 'style').replace(/[^\w.-]+/g, '-').slice(0, 60) || 'style';
    try {
      let blob: Blob, file: string;
      if (kind.startsWith('pattern-pdf')) { blob = new Blob([new Uint8Array(await engine.exportPatternPdf(state.doc, kind.endsWith('a3') ? 'A3' : 'A4'))], { type: 'application/pdf' }); file = `${name}-pattern-1to1.pdf`; }
      else if (kind === 'pattern-dxf') { blob = new Blob([await engine.exportPatternDxf(state.doc)], { type: 'application/dxf' }); file = `${name}-pattern.dxf`; }
      else if (kind === 'pattern-plt') { blob = new Blob([await engine.exportPatternPlt(state.doc)], { type: 'application/vnd.hp-hpgl' }); file = `${name}-pattern.plt`; }
      else {
        const m = state.marker; if (!m.job || !m.result) throw new Error(t('why_no_marker'));
        const guides = (document.getElementById('st-guides') as HTMLInputElement | null)?.checked;
        const job: Job = guides ? { ...structuredClone(m.job), meta: { ...m.job.meta, beta_stripe_guides: true } } : m.job;
        const format = kind === 'marker-tiled' ? 'tiled-a4' : kind.slice(7) as 'plt' | 'pdf' | 'dxf';
        blob = await exportMarker(job, m.result, format); file = `${name}-marker.${format === 'tiled-a4' ? 'tiles-a4.pdf' : format}`;
      }
      const path = await downloadFile(blob, file);
      host.message(path ? t('saved_file', { file: path }) : t('save_cancelled'));
    } catch (error) { host.message(errorText(error), true); }
    finally { button.disabled = false; button.innerHTML = old; }
  }
  function openEditor(): void {
    const m = state.marker; if (!m.job || !m.result) return;
    handedOff = true;
    window.dispatchEvent(new CustomEvent<DocumentState>('beta-open-document', { detail: { job: { ...m.job, meta: { ...m.job.meta, beta_style_handoff: state.info.name } }, result: m.result } }));
  }
  function goScreen(index: number, page?: string): void {
    const button = document.querySelector<HTMLButtonElement>(`.screen-tabs [data-screen="${index}"]`); button?.click();
    if (page) document.querySelector<HTMLButtonElement>(`[data-floor-page="${page}"]`)?.click();
  }
  function cutPlan(): void {
    openEditor();
    window.setTimeout(() => {
      goScreen(1);
      const names = [...new Set(state.marker.job!.pieces.flatMap(p => p.size ? [p.size] : []))].sort();
      names.forEach((s, i) => { const input = document.getElementById(`quantity-${i}`) as HTMLInputElement | null; if (input) input.value = String(state.lay.order[s] ?? 0); });
      const plies = document.getElementById('max-plies') as HTMLInputElement | null; if (plies) plies.value = String(state.lay.plies);
      const end = document.getElementById('end-allowance') as HTMLInputElement | null; if (end) end.value = String(state.lay.endAllowanceMm);
    }, 50);
  }
  /** Back from the marker editor: a banner offers to return, optionally with the hand-edited marker. */
  function editorBanner(): void {
    const main = document.querySelector('#editor-screen .editor-main'); if (!main) return;
    let banner = document.getElementById('st-back-banner');
    const show = handedOff && host.editorState().job.meta?.beta_style_handoff === state.info.name;
    if (!show) { banner?.remove(); return; }
    if (!banner) { banner = document.createElement('div'); banner.id = 'st-back-banner'; banner.className = 'notice st-back-banner'; main.prepend(banner); }
    banner.innerHTML = `<span>${icon('style')}${tx('from_style', { name: state.info.name })}</span><button type="button" data-back="keep">${icon('arrow-left')}${tx('back_to_style')}</button><button type="button" class="primary" data-back="use">${icon('check')}${tx('use_edited')}</button>`;
  }
  document.addEventListener('click', event => {
    const b = (event.target as Element).closest<HTMLButtonElement>('#st-back-banner [data-back]'); if (!b) return;
    if (b.dataset.back === 'use') void useEdited(); else goScreen(STYLE_SCREEN);
  });
  async function useEdited(): Promise<void> {
    const ed = host.editorState(); if (!engine) return;
    try {
      const report = await engine.validate(ed.job, ed.result), overlay = await engine.overlay(ed.job, ed.result, state.marker.previous ? { previous: state.marker.previous } : {});
      await attachMatches(overlay, report, ed.job, ed.result);
      state = { ...state, marker: { ...state.marker, status: 'done', job: ed.job, result: ed.result, report, overlay, stale: false } };
      goScreen(STYLE_SCREEN); goStep('cut'); host.message(t('edited_used'));
    } catch (error) { host.message(errorText(error), true); }
  }

  // ---------- Events ----------
  root.addEventListener('click', event => {
    const el = (event.target as Element).closest<HTMLElement>('button, [data-grade-point], [data-match-point]'); if (!el || !root.contains(el)) return;
    if (el.dataset.stStep) { goStep(el.dataset.stStep as S.Step); return; }
    if (el.id === 'st-next-top') { const n = S.nextStep(state.step); if (n) goStep(n); return; }
    if (el.dataset.unit) { update({ ...state, unit: el.dataset.unit as LengthUnit }); canvas.refresh(); return; }
    if (el.dataset.block) { void startFrom(el.dataset.block); return; }
    if (el.dataset.gotoStep || el.dataset.goto) { goStep((el.dataset.goto ?? el.dataset.gotoStep) as S.Step); return; }
    if (el.dataset.gradePiece) { gradePiece = el.dataset.gradePiece; gradePoint = ''; render(); return; }
    if (el.dataset.gradePoint) { gradePoint = el.dataset.gradePoint; render(); return; }
    if (el.dataset.removeSize) { void removeSize(el.dataset.removeSize); return; }
    if (el.dataset.widthUnit) { update(S.setLay(state, { widthUnit: el.dataset.widthUnit as LengthUnit })); return; }
    if (el.dataset.widthPreset) { update(S.setLay(state, { widthMm: toMm(Number(el.dataset.widthPreset), state.lay.widthUnit) })); return; }
    if (el.dataset.plies) { update(S.setLay(state, { plies: stepCount(state.lay.plies, Number(el.dataset.plies), S.pliesMustBeEven(state.lay) ? 2 : 1, 500) })); return; }
    if (el.dataset.ratio) { update(S.setLay(state, { ratio: { ...state.lay.ratio, [el.dataset.ratio]: stepCount(state.lay.ratio[el.dataset.ratio] ?? 0, Number(el.dataset.delta), 0, 50) } })); return; }
    if (el.dataset.direction) { update(S.setRepeat(state, { direction: el.dataset.direction as 'along' })); return; }
    if (el.dataset.removeBand) { update(S.setRepeat(state, { bands: state.repeat.bands.filter((_, i) => i !== Number(el.dataset.removeBand)) })); return; }
    if (el.dataset.removeMatch) { update(S.setRepeat(state, { matches: state.repeat.matches.filter((_, i) => i !== Number(el.dataset.removeMatch)) })); return; }
    if (el.dataset.showRule) { focusRule = Number(el.dataset.showRule); layers = { ...layers, matches: true }; goStep('marker'); return; }
    if (el.dataset.matchPoint) { void pickMatch(el.dataset.matchSide as 'a' | 'b', el.dataset.matchPoint); return; }
    if (el.dataset.budget) { budget = Number(el.dataset.budget); render(); return; }
    if (el.dataset.export) { void exportFile(el.dataset.export, el as HTMLButtonElement); return; }
    switch (el.dataset.st) {
      case 'back': { const p = S.prevStep(state.step); if (p) goStep(p); break; }
      case 'next': { const n = S.nextStep(state.step); if (n) goStep(n); break; }
      case 'home': goScreen(9); break;
      case 'how': howToggled[state.step] = !howOpen(); if (!howToggled[state.step]) markHowSeen(state.step); renderChrome(); if (state.step === 'pattern') canvas.refresh(); break;
      case 'how-close': howToggled[state.step] = false; markHowSeen(state.step); renderChrome(); root.querySelector<HTMLElement>('.st-how-btn')?.focus(); if (state.step === 'pattern') canvas.refresh(); break;
      case 'save-file': void saveFile(); break;
      case 'open-file': $<HTMLInputElement>('st-open-file').click(); break;
      case 'restart': if (window.confirm(t('confirm_restart'))) { S.clearSaved(); state = S.initialSession(); history = new PatternHistory(emptyDoc()); base = null; seams = []; canvas.reset(); render(true); } break;
      case 'add-size': void addSize(); break;
      case 'save-rule': void saveRule(); break;
      case 'remove-rule': void commit([{ op: 'attach_rule', piece: gradePiece, point: gradePoint, rule: null }], t('rule_removed')); break;
      case 'close-rule': gradePoint = ''; render(); break;
      case 'add-check': void addCheck(); break;
      case 'add-band': { const axis = state.repeat.kind === 'stripes' && state.repeat.direction === 'across' ? 'x' : 'y'; update(S.setRepeat(state, { bands: [...state.repeat.bands, { axis, offset_mm: 0, width_mm: 3, colour: ['#1d4f8c', '#b54537', '#176e60', '#94600e'][state.repeat.bands.length % 4]! }] })); break; }
      case 'nest': void runNest(); break;
      case 'stop': abort?.abort(); say(t('stopping')); break;
      case 'focus-clear': focusRule = null; render(); break;
      case 'open-editor': openEditor(); break;
      case 'cut-plan': cutPlan(); break;
      case 'spreading': openEditor(); window.setTimeout(() => goScreen(8, 'spreading'), 50); break;
      case 'bundles': openEditor(); window.setTimeout(() => goScreen(7), 50); break;
    }
  });
  root.addEventListener('keydown', event => {
    const el = (event.target as Element).closest<HTMLElement>('[data-grade-point], [data-match-point]');
    if (el && (event.key === 'Enter' || event.key === ' ')) { event.preventDefault(); el.dispatchEvent(new MouseEvent('click', { bubbles: true })); }
  });
  root.addEventListener('input', event => {
    const el = event.target as HTMLInputElement;
    if (el.dataset.info) { state = S.setInfo(state, { [el.dataset.info]: el.value }); persist(); if (el.dataset.info === 'name') renderChrome(); }
  });
  root.addEventListener('change', event => {
    const el = event.target as HTMLInputElement & HTMLSelectElement;
    const mm = (negative = false) => parseLength(el.value, unit(), negative);
    const need = (v: number | null): v is number => { if (v == null) { host.message(t('err_length'), true); el.setAttribute('aria-invalid', 'true'); return false; } el.removeAttribute('aria-invalid'); return true; };
    if (el.id === 'st-preset') { chartPreset = el.value as 'indian_adult'; void engine?.sizeChart(chartPreset).then(c => { chart = c; render(); }); return; }
    if (el.dataset.range) { if (!root.querySelectorAll('[data-range]:checked').length) { el.checked = true; host.message(t('keep_one_size'), true); } return; }
    if (el.name === 'st-layout') { update(S.setLay(state, { layout: el.value as S.LaySetup['layout'], plies: S.pliesMustBeEven({ layout: el.value as 'open', face: state.lay.face }) && state.lay.plies % 2 ? state.lay.plies + 1 : state.lay.plies })); return; }
    if (el.name === 'st-face') { update(S.setLay(state, { face: el.value as S.Face, plies: S.pliesMustBeEven({ layout: state.lay.layout, face: el.value as S.Face }) && state.lay.plies % 2 ? state.lay.plies + 1 : state.lay.plies })); return; }
    if (el.name === 'st-nap') { update(S.setLay(state, { nap: el.value as S.LaySetup['nap'] })); return; }
    if (el.name === 'st-repeat') { update(S.setRepeat(state, { kind: el.value as S.RepeatKind })); return; }
    switch (el.id) {
      case 'st-width': { const v = parseLength(el.value, state.lay.widthUnit === 'inch' ? 'inch' : 'cm'); if (need(v)) update(S.setLay(state, { widthMm: v })); return; }
      case 'st-material': update(S.setLay(state, { material: el.value })); return;
      case 'st-flip': update(S.setLay(state, { allowFlip: el.checked })); return;
      case 'st-plies': { const v = Number(el.value); if (Number.isInteger(v) && v > 0) update(S.setLay(state, { plies: v })); else host.message(t('why_lay_plies'), true); return; }
      case 'st-end': case 'st-end2': { const v = mm(); if (need(v)) update(S.setLay(state, { endAllowanceMm: v })); return; }
      case 'st-piece-gap': { const v = mm(); if (need(v)) update(S.setGaps(state, { pieceGap: v })); return; }
      case 'st-edge-gap': { const v = mm(); if (need(v)) update(S.setGaps(state, { edgeGap: v })); return; }
      case 'st-period-x': { const v = mm(); if (need(v)) update(S.setRepeat(state, { periodX: v })); return; }
      case 'st-period-y': { const v = mm(); if (need(v)) update(S.setRepeat(state, { periodY: v })); return; }
      case 'st-offset-x': { const v = mm(true); if (need(v)) update(S.setRepeat(state, { offsetX: v })); return; }
      case 'st-offset-y': { const v = mm(true); if (need(v)) update(S.setRepeat(state, { offsetY: v })); return; }
      case 'st-tolerance': { const v = mm(); if (need(v)) update(S.setRepeat(state, { tolerance: v })); return; }
      case 'st-overlay-sizes': layers.sizes = el.checked; render(); return;
      case 'st-match-a': matchPick = { ...matchPick, pieceA: el.value, a: undefined }; render(); return;
      case 'st-match-b': matchPick = { ...matchPick, pieceB: el.value }; render(); return;
      case 'st-chart-preset': if (el.value) void applyPreset(el.value as 'indian_adult'); return;
    }
    if (el.name === 'st-base') { void setChart({ base_size: el.value }); return; }
    if (el.dataset.measure) { const v = mm(); if (!need(v)) return; const d = state.doc!; const measurements = structuredClone(d.measurements); (measurements[el.dataset.size!] ??= {})[el.dataset.measure] = Math.round(v * 10) / 10; void setChart({ measurements }); return; }
    if (el.dataset.order) { const v = Math.max(0, Math.round(Number(el.value) || 0)); update(S.setLay(state, { order: { ...state.lay.order, [el.dataset.order]: v } })); return; }
    if (el.dataset.buffer) { const v = mm(); if (need(v)) update(S.setGaps(state, { buffers: { ...state.gaps.buffers, [el.dataset.buffer]: v } })); return; }
    if (el.dataset.blockOn) { update(S.setGaps(state, { blocks: { ...state.gaps.blocks, [el.dataset.blockOn]: el.checked ? 10 : null } })); return; }
    if (el.dataset.block) { const v = mm(); if (need(v)) update(S.setGaps(state, { blocks: { ...state.gaps.blocks, [el.dataset.block]: v } })); return; }
    if (el.dataset.sizeToggle) { if (el.checked) hiddenSizes.delete(el.dataset.sizeToggle); else hiddenSizes.add(el.dataset.sizeToggle); drawNest(); return; }
    if (el.dataset.layer) { layers = { ...layers, [el.dataset.layer]: el.checked }; render(); return; }
    const band = (key: 'bandColour' | 'bandWidth' | 'bandOffset' | 'bandAxis') => el.dataset[key] != null ? Number(el.dataset[key]) : -1;
    for (const key of ['bandColour', 'bandWidth', 'bandOffset', 'bandAxis'] as const) {
      const i = band(key); if (i < 0) continue;
      const bands = structuredClone(state.repeat.bands), b = bands[i]!;
      if (key === 'bandColour') b.colour = el.value;
      if (key === 'bandAxis') b.axis = el.value as 'x';
      if (key === 'bandWidth' || key === 'bandOffset') { const v = mm(key === 'bandOffset'); if (!need(v)) return; if (key === 'bandWidth') b.width_mm = v; else b.offset_mm = v; }
      update(S.setRepeat(state, { bands })); return;
    }
  });
  async function setChart(patch: Partial<SizeChart>): Promise<void> {
    const d = state.doc!; const c: SizeChart = { name: 'chart', sizes: d.sizes, base_size: d.base_size, measurements: d.measurements, ...patch };
    await commit([{ op: 'set_chart', chart: c }], t('chart_updated'));
  }
  async function applyPreset(preset: 'indian_adult' | 'iso_example'): Promise<void> {
    if (!engine) return;
    const c = await engine.sizeChart(preset);
    if (!window.confirm(t('confirm_preset', { sizes: c.sizes.join(' · ') }))) { render(); return; }
    await commit([{ op: 'set_chart', chart: c }], t('chart_updated'));
  }
  async function addSize(): Promise<void> {
    const d = state.doc!, input = document.getElementById('st-new-size') as HTMLInputElement, name = input.value.trim();
    if (!name || name.length > 12 || d.sizes.includes(name)) { host.message(t('err_size_name'), true); return; }
    // Start the new size one grade step beyond the last size (the step between the last two sizes).
    const last = d.sizes.at(-1)!, prev = d.sizes.at(-2), m = structuredClone(d.measurements);
    m[name] = Object.fromEntries(Object.entries(m[last] ?? {}).map(([k, v]) => [k, Math.round((v + (prev ? v - (m[prev]?.[k] ?? v) : 0)) * 10) / 10]));
    await setChart({ sizes: [...d.sizes, name], measurements: m });
    say(t('size_added', { size: name }));
  }
  async function removeSize(size: string): Promise<void> {
    const d = state.doc!; if (d.sizes.length < 2) return;
    if (!window.confirm(t('confirm_remove_size', { size }))) return;
    const sizes = d.sizes.filter(s => s !== size), m = structuredClone(d.measurements); delete m[size];
    await setChart({ sizes, base_size: sizes.includes(d.base_size) ? d.base_size : sizes[0]!, measurements: m });
  }
  async function saveRule(): Promise<void> {
    const d = state.doc!, p = d.pieces.find(q => q.id === gradePiece)!, id = p.grade_points?.[gradePoint] ?? `rule_${gradePiece}_${gradePoint}`.replace(/[^A-Za-z0-9_.-]/g, '_');
    const deltas: Record<string, Pt> = {};
    for (const s of d.sizes) {
      if (s === d.base_size) { deltas[s] = [0, 0]; continue; }
      const x = parseLength((root.querySelector<HTMLInputElement>(`[data-rule-x="${CSS.escape(s)}"]`)!).value, unit(), true), y = parseLength((root.querySelector<HTMLInputElement>(`[data-rule-y="${CSS.escape(s)}"]`)!).value, unit(), true);
      if (x == null || y == null) { host.message(t('err_length'), true); return; }
      deltas[s] = [x, y];
    }
    if (await commit([{ op: 'set_rule', id, rule: { mode: 'relative', deltas } }, { op: 'attach_rule', piece: gradePiece, point: gradePoint, rule: id }], t('rule_saved'))) void loadNest();
  }
  async function addCheck(): Promise<void> {
    const get = (id: string) => (document.getElementById(id) as HTMLInputElement).value;
    const edges = (v: string) => v.split(/[ ,]+/).filter(Boolean).map(n => Number(n) - 1);
    const ea = edges(get('st-check-ea')), eb = edges(get('st-check-eb')), ease = parseLength(get('st-check-ease'), unit(), true) ?? 0;
    if (!ea.length || !eb.length || [...ea, ...eb].some(n => !Number.isInteger(n) || n < 0)) { host.message(t('err_edges'), true); return; }
    const check: SeamCheck = { name: t('custom_check', { n: extraChecks.length + 1 }), a: { piece: get('st-check-a'), edges: ea }, b: { piece: get('st-check-b'), edges: eb }, ease_mm: ease, tolerance_mm: 1 };
    try { await engine!.seamReport(state.doc!, check); extraChecks.push(check); await refreshDoc(); render(); say(t('check_added')); } catch (error) { host.message(errorText(error), true); }
  }
  async function saveFile(): Promise<void> {
    if (!engine || !state.doc) return;
    try {
      const doc = { ...state.doc, name: state.info.name || state.doc.name, meta: { ...state.doc.meta, beta_style_info: { ...state.info } as never, beta_style_settings: { lay: state.lay, gaps: state.gaps, repeat: state.repeat, unit: state.unit } as never } };
      const text = await engine.save(doc), name = (state.info.number || state.info.name || 'style').replace(/[^\w.-]+/g, '-').slice(0, 60);
      const path = await downloadFile(new Blob([text], { type: 'application/json' }), `${name}.style.json`);
      host.message(path ? t('saved_file', { file: path }) : t('save_cancelled'));
    } catch (error) { host.message(errorText(error), true); }
  }
  $<HTMLInputElement>('st-open-file').addEventListener('change', async event => {
    const input = event.target as HTMLInputElement, file = input.files?.[0]; input.value = '';
    if (!file) return;
    try {
      if (file.size > 20 * 1024 * 1024) throw new Error(t('err_file_size'));
      await ensureEngine(); if (!engine) return;
      const doc = await engine.load(await file.text());
      const meta = doc.meta as { beta_style_info?: Partial<S.StyleInfo>; beta_style_settings?: Partial<Pick<S.Session, 'lay' | 'gaps' | 'repeat' | 'unit'>> } | undefined;
      state = S.setInfo(state, { ...meta?.beta_style_info, name: meta?.beta_style_info?.name || doc.name });
      if (meta?.beta_style_settings) state = { ...state, ...meta.beta_style_settings, lay: { ...state.lay, ...meta.beta_style_settings.lay }, gaps: { ...state.gaps, ...meta.beta_style_settings.gaps }, repeat: { ...state.repeat, ...meta.beta_style_settings.repeat } };
      await useDoc(doc, { kind: 'open' }); host.message(t('opened_file', { name: file.name }));
    } catch (error) { host.message(errorText(error), true); }
  });
  // Fit SVG previews to their drawings (viewBox from the inner g[data-view]).
  new MutationObserver(() => root.querySelectorAll<SVGSVGElement>('svg').forEach(s => { const v = s.querySelector('g[data-view]')?.getAttribute('data-view'); if (v && s.getAttribute('viewBox') !== v) s.setAttribute('viewBox', v); })).observe(body, { childList: true, subtree: true });
  window.addEventListener('beta-screen', event => {
    const index = (event as CustomEvent<number>).detail;
    if (index === STYLE_SCREEN) { void ensureEngine(); render(); }
    if (index === 0) editorBanner(); else document.getElementById('st-back-banner')?.remove();
  });
  // Language changes re-render the flow (html[data-language] is set by the app switch).
  let language = currentLanguage();
  const navLabel = () => {
    const b = document.querySelector<HTMLElement>(`[data-screen="${STYLE_SCREEN}"] .nav-label`); if (b) { b.textContent = t('nav'); b.lang = currentLanguage(); }
    // The top bar title was set before this label changed.
    if (!root.hidden) { const title = document.getElementById('topbar-title'); if (title) title.textContent = t('nav'); }
  };
  new MutationObserver(() => { if (currentLanguage() !== language) { language = currentLanguage(); navLabel(); if (!root.hidden) render(); } }).observe(document.documentElement, { attributes: true, attributeFilter: ['data-language'] });
  window.setTimeout(navLabel, 0);
  render();
  void fromMm;
  return { root };
}
