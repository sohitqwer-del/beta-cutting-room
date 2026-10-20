import { loadMarkerCore, type Job, type MarkerCore, type MarkerResult, type Pt } from '@beta/marker-wasm';
import { icon } from '../icons';
import { apiFetch, downloadFile, isDesktop } from '../transport';
import type { DocumentState } from '../state';
import type { ProjectorLayout, QuickPiece, QuickWarning } from '@beta/marker-wasm';
import { exportFile, isPractice, loadQuickEngine, mockRequested, nestAvailable, serverAvailable, nestInRounds, nestPractice, scanInWorker, type QuickEngine } from './adapter';
import { ART, qicon } from './art';
import { bounds, mmToOverlay, pathData } from './geometry';
import { currentLanguage, pieceName, sizeName, t, warningText, type QuickKey } from './i18n';
import { samplePhoto } from './mock';
import { correct, loadCalibration, measureProblem, saveCalibration, sections, squarePx, toScreen, type Calibration, type Section } from './projector';
import { installPwa, type InstallState } from './pwa';
import { t as feedbackText } from '../feedback/i18n';
import * as S from './state';
import { formatWidth, metres, metresPerGarment, parseNumber, percent, savingVsUsual, toMm, WIDTH_PRESETS, widthProblem } from './units';

/** Quick Marker: photo of paper pattern pieces → checked pieces → fabric and order → nest → print, chalk,
 * projector or share. Phone first (360-430 px, one hand, 48 px targets), also works on desktop.
 * All shape reading, assist, job building and PDFs call Rust through adapter.ts; nesting uses the Rust nester. */
export interface QuickHost { message(text: string, error?: boolean): void }
export const QUICK_SCREEN = 11;

const esc = (v: unknown): string => String(v ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]!));
const tx = (key: QuickKey, values?: Record<string, string | number>) => esc(t(key, values));
const errorText = (e: unknown) => e instanceof Error ? e.message : String(e);
const BUDGETS: Array<[QuickKey, number]> = [['time_fast', 20], ['time_better', 60], ['time_best', 180]];
const FLOW: S.Step[] = ['capture', 'review', 'order', 'nest', 'use'];
const STEP_LABEL: Record<S.Step, QuickKey> = { start: 'title', capture: 'step_capture', review: 'step_review', order: 'step_order', nest: 'step_nest', use: 'step_use' };
const MAX_PHOTO = 20 * 1024 * 1024;

export function installQuick(host: QuickHost, content: HTMLElement): { root: HTMLElement } {
  let state = S.initialState();
  let engine: QuickEngine | null = null, engineProblem = '', practice = false, canNest: boolean | null = null;
  let markerCore: MarkerCore | undefined, photoCounter = 0, nestAbort: AbortController | null = null, nestTimer = 0, nestStart = 0, milestone = false;
  let install: InstallState = { canInstall: false, offline: 'unsupported' };
  let paper: 'A4' | 'A3' = 'A4';
  const photoUrls = new Map<string, string>(), overlayUrls = new Map<string, string>();
  let viewPhoto = '';

  const root = document.createElement('section');
  root.id = `screen-${QUICK_SCREEN}`; root.className = 'screen quick'; root.hidden = true;
  root.setAttribute('aria-labelledby', 'q-title');
  // The feedback module's click handler opens its dialog for [data-fb-open]; this link stays when steps re-render.
  root.innerHTML = `<div class="q-head" id="q-head"></div><div id="q-body" class="q-body"></div>
    <p class="q-foot no-print"><button type="button" class="fb-screen-link ghost small" data-fb-open="bug"><svg class="icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false"><path d="M4 5h16v11H10l-5 4v-4H4z"/><path d="M8 9.5h8M8 12.5h5"/></svg><span data-fb="screen_link">${esc(feedbackText('screen_link'))}</span></button></p>
    <input id="q-camera" type="file" accept="image/*" capture="environment" multiple hidden>
    <input id="q-gallery" type="file" accept="image/jpeg,image/png,image/*" multiple hidden>
    <p id="q-live" class="sr-only" role="status" aria-live="polite"></p>
    <svg width="0" height="0" class="sr-only" aria-hidden="true"><defs><marker id="q-arrow" viewBox="0 0 10 10" refX="7" refY="5" markerWidth="5" markerHeight="5" orient="auto"><path d="M0 0L10 5L0 10z" fill="var(--draw-grain)"/></marker></defs></svg>`;
  content.append(root);
  document.body.insertAdjacentHTML('beforeend', `<dialog id="q-sheet" class="q-sheet" aria-labelledby="q-sheet-title"><div class="q-sheet-body" id="q-sheet-body"></div></dialog>
    <div id="q-projector" class="q-projector" role="dialog" aria-modal="true" aria-labelledby="q-proj-title" hidden></div>`);
  const $ = <T extends HTMLElement = HTMLElement>(id: string) => document.getElementById(id) as T;
  const body = $('q-body'), head = $('q-head'), sheet = $<HTMLDialogElement>('q-sheet'), proj = $('q-projector');
  const say = (text: string) => { const live = $('q-live'); live.textContent = ''; window.setTimeout(() => { live.textContent = text; }, 40); };
  const pwa = installPwa(next => { install = next; if (state.step === 'start') render(); });

  // ---------- Engine ----------
  async function ready(forceMock = false): Promise<QuickEngine | null> {
    if (engine && (!forceMock || engine.kind === 'mock')) return engine;
    const loaded = await loadQuickEngine({ mock: forceMock || mockRequested(location.search) });
    if (!loaded.ok) { engineProblem = loaded.reason === 'missing' ? t('engine_missing') : t('engine_failed', { error: loaded.error }); return null; }
    engine = loaded.engine; practice = engine.kind === 'mock'; engineProblem = '';
    if (engine.kind === 'wasm' && typeof Worker !== 'undefined') {
      const moduleUrl = new URL('wasm/beta_marker_wasm.js', document.baseURI).href;
      engine = scanInWorker(engine, () => new Worker(new URL('./scan-worker.ts', import.meta.url), { type: 'module' }), moduleUrl);
    }
    return engine;
  }
  const nester = async () => { if (canNest == null) canNest = await nestAvailable(apiFetch); return canNest; };
  void loadMarkerCore().then(core => { markerCore = core; }).catch(() => { /* previews fall back to a simple transform */ });

  // ---------- Rendering ----------
  function render(focusTitle = false): void {
    const active = document.activeElement as HTMLElement | null;
    const keep = active && root.contains(active) ? active.id : '';
    head.innerHTML = headHtml();
    body.innerHTML = ({ start: startHtml, capture: captureHtml, review: reviewHtml, order: orderHtml, nest: nestHtml, use: useHtml } as Record<S.Step, () => string>)[state.step]();
    root.dataset.step = state.step;
    if (focusTitle) { const h = root.querySelector<HTMLElement>('h1'); h?.focus({ preventScroll: true }); window.scrollTo(0, 0); }
    else if (keep) document.getElementById(keep)?.focus({ preventScroll: true });
    if (sheet.open) renderSheet();
  }
  function goStep(step: S.Step): void {
    const why = S.blocker(state, step);
    if (why) { flashWhy(t(`blocked_${why}` as QuickKey)); return; }
    if (step === 'order') state = S.ensureGarments(state);
    state = S.go(state, step); render(true);
  }

  /** A blocked step: say why next to the main button (and to screen readers) instead of a toast over the dock. */
  function flashWhy(text: string): void {
    say(text);
    const why = root.querySelector<HTMLElement>('.q-dock .q-why');
    if (why) { why.textContent = text; why.classList.remove('q-flash'); void why.offsetWidth; why.classList.add('q-flash'); }
    else host.message(text);
  }
  function headHtml(): string {
    const at = FLOW.indexOf(state.step);
    const crumbs = FLOW.map((s, i) => {
      const can = !S.blocker(state, s) && state.nest.status !== 'running';
      return `<li><button type="button" data-q="goto" data-step="${s}" ${i === at ? 'aria-current="step"' : ''} ${can ? '' : 'disabled'}><span class="q-dot" aria-hidden="true">${i < at ? icon('check') : i + 1}</span><span class="q-crumb">${tx(STEP_LABEL[s])}</span></button></li>`;
    }).join('');
    return `${state.step !== 'start' ? `<button type="button" class="q-back ghost" data-q="back" id="q-back" ${state.nest.status === 'running' ? 'disabled' : ''}>${icon('arrow-left')}<span>${tx('back')}</span></button>` : ''}
      ${state.step !== 'start' ? `<ol class="q-steps" aria-label="${tx('step_of', { n: at + 1, total: FLOW.length })}">${crumbs}</ol>` : ''}
      ${practice ? `<span class="chip warn q-practice-chip">${icon('alert')}${tx('practice')}</span>` : ''}`;
  }

  // ----- Start -----
  function startHtml(): string {
    const lang = currentLanguage();
    const langs: Array<[string, string]> = [['en', 'English'], ['hi', 'हिन्दी'], ['ta', 'தமிழ்']];
    return `<div class="q-hero"><h1 id="q-title" tabindex="-1">${tx('title')}</h1><p class="q-lead">${tx('lead')}</p>
      <div class="q-langs" role="group" aria-label="${tx('language')}">${langs.map(([v, name]) => `<button type="button" data-q="lang" data-lang="${v}" lang="${v}" aria-pressed="${lang === v}">${name}</button>`).join('')}</div></div>
      ${engineProblem ? `<div class="notice warn q-notice" role="status">${icon('alert')}<span>${esc(engineProblem)}</span></div>` : ''}
      <button type="button" class="primary q-big" data-q="camera" id="q-take">${qicon('camera')}<span>${tx('take_photo')}</span></button>
      <button type="button" class="q-wide" data-q="gallery" id="q-gallery-btn">${qicon('gallery')}<span>${tx('gallery')}</span></button>
      <section class="q-card" aria-labelledby="q-guide-title"><h2 id="q-guide-title">${tx('guide_title')}</h2>
        <ol class="q-guide">${([['cloth', 'guide_1'], ['a4', 'guide_2'], ['above', 'guide_3']] as const).map(([art, key], i) => `<li><figure>${ART[art]}<figcaption><span class="q-num" aria-hidden="true">${i + 1}</span>${tx(key)}</figcaption></figure></li>`).join('')}</ol></section>
      <fieldset class="q-card q-modes"><legend>${tx('mode_title')}</legend>
        ${modeCard('a4-sheet', ART.sheet, 'mode_quick', 'mode_quick_hint')}${modeCard('beta-target', ART.target, 'mode_accurate', 'mode_accurate_hint')}
        ${state.reference === 'beta-target' ? `<button type="button" class="small" data-q="print-target" id="q-print-target">${icon('print')}${tx('mode_accurate_print')}</button>` : ''}</fieldset>
      <div class="q-row q-start-foot">
        <button type="button" class="ghost" data-q="sample" id="q-sample">${icon('demo')}${tx('try_sample')}</button>
        ${install.canInstall ? `<button type="button" data-q="install" id="q-install">${qicon('install')}${tx('install')}</button>` : ''}
        ${install.offline === 'ready' ? `<span class="chip ok">${icon('check')}${tx('offline_ready')}</span>` : ''}
      </div>
      ${install.canInstall ? `<p class="hint">${tx('install_hint')}</p>` : ''}`;
  }
  function modeCard(value: S.Reference, art: string, title: QuickKey, hint: QuickKey): string {
    return `<label class="q-mode"><input type="radio" name="q-mode" value="${value}" data-q-change="mode" ${state.reference === value ? 'checked' : ''}>${art}<span><strong>${tx(title)}</strong><small>${tx(hint)}</small></span></label>`;
  }

  // ----- Capture -----
  function captureHtml(): string {
    const reading = state.photos.some(p => p.status === 'reading');
    const found = state.pieces.length;
    return `<h1 id="q-title" tabindex="-1">${tx('photos_title')}</h1>${practiceNote()}
      <div class="q-photos">${state.photos.length ? state.photos.map(photoCard).join('') : `<p class="q-empty">${tx('photos_empty')}</p>`}</div>
      <div class="q-actions">
        <button type="button" class="q-wide" data-q="camera" id="q-take-more">${qicon('camera')}<span>${tx(state.photos.length ? 'take_another' : 'take_photo')}</span></button>
        <button type="button" class="q-wide" data-q="gallery" id="q-gallery-more">${qicon('gallery')}<span>${tx('gallery')}</span></button>
      </div>
      <p class="hint">${tx('photo_tip')}</p>
      <div class="q-dock"><button type="button" class="primary q-big" data-q="goto" data-step="review" id="q-to-review" ${found && !reading ? '' : 'disabled'}>${tx('to_review')}${found ? ` (${found})` : ''}${icon('arrow-right')}</button></div>`;
  }
  function photoCard(photo: S.Photo, index: number): string {
    const scan = photo.scan, url = overlayUrls.get(photo.id) ?? photoUrls.get(photo.id) ?? '';
    const warnings = state.warnings.filter(w => w.photoId === photo.id || (!w.photoId && index === 0));
    const status = photo.status === 'reading' ? `<span class="q-spinner" aria-hidden="true"></span>${tx('photo_reading')}`
      : photo.status === 'failed' ? `${icon('alert')}${tx('photo_failed')}` : `${icon('check-circle')}${tx('photo_found', { n: state.pieces.filter(p => p.piece.photoId === photo.id).length })}`;
    const scale = scan ? `<span class="chip ${scan.scale.confidence === 'high' ? 'ok' : scan.scale.confidence === 'low' ? 'bad' : 'warn'}">${tx(`scale_${scan.scale.confidence}` as QuickKey)} · ${tx('scale_error', { mm: scan.scale.expectedErrorMm.toFixed(1) })}</span>` : '';
    return `<article class="q-photo-card" aria-labelledby="q-ph-${photo.id}">
      ${url ? `<img src="${esc(url)}" alt="" class="q-thumb">` : '<div class="q-thumb"></div>'}
      <div class="q-photo-text"><h2 id="q-ph-${photo.id}">${tx('photo_label', { n: index + 1 })}</h2>
        <p class="q-status q-${photo.status}">${status}</p>${scale}
        ${photo.error ? `<p class="error-text">${esc(photo.error)}</p>` : ''}
        ${photo.status !== 'failed' ? `<label class="q-inline">${tx('photo_size')}<select id="q-size-${photo.id}" data-q-change="photo-size" data-id="${photo.id}">${sizeOptions(photo.size)}</select></label>` : ''}
      </div>
      <button type="button" class="ghost icon-btn q-remove" data-q="remove-photo" data-id="${photo.id}" aria-label="${tx('photo_remove')}: ${tx('photo_label', { n: index + 1 })}" data-tip="${tx('photo_remove')}" id="q-rm-${photo.id}">${icon('trash')}</button>
      ${warnings.length ? `<ul class="q-warnings">${groupWarnings(warnings).map(([w, n]) => { const x = warningText(w.code, w.message); return `<li>${icon('alert')}<span><strong>${esc(x.title)}${n > 1 ? ` <span class="q-times">× ${n}</span>` : ''}</strong><span>${esc(x.fix)}</span></span></li>`; }).join('')}</ul>` : ''}
    </article>`;
  }
  /** One line per warning code (per-piece warnings repeat), with how many times it was found. Unknown codes keep their own text. */
  function groupWarnings(list: QuickWarning[]): Array<[QuickWarning, number]> {
    const groups = new Map<string, [QuickWarning, number]>();
    for (const w of list) { const key = warningText(w.code, '').title ? w.code : `${w.code}:${w.message}`; const g = groups.get(key); if (g) g[1]++; else groups.set(key, [w, 1]); }
    return [...groups.values()];
  }
  function sizeOptions(selected: string): string {
    const all = [...new Set([...S.SIZE_PRESETS, selected])];
    return all.map(s => `<option value="${esc(s)}" ${s === selected ? 'selected' : ''}>${esc(sizeName(s))}</option>`).join('');
  }
  const practiceNote = () => practice ? `<p class="notice warn q-notice">${icon('alert')}<span>${tx('practice_note')}</span></p>` : '';

  // ----- Review -----
  function reviewHtml(): string {
    const done = state.photos.filter(p => p.status === 'done');
    if (!done.some(p => p.id === viewPhoto)) viewPhoto = done[0]?.id ?? '';
    const acc = S.accepted(state).length, wait = S.pending(state).length;
    return `<h1 id="q-title" tabindex="-1">${tx('review_title')}</h1><p class="q-lead">${tx('review_hint')}</p>${practiceNote()}
      <div class="q-review">
        <div class="q-review-photo">
          ${done.length > 1 ? `<div class="segmented q-photo-tabs" role="group" aria-label="${tx('photos_title')}">${done.map(p => `<button type="button" data-q="view-photo" data-id="${p.id}" aria-pressed="${p.id === viewPhoto}">${tx('photo_label', { n: state.photos.indexOf(p) + 1 })}</button>`).join('')}</div>` : ''}
          ${photoOverlay(viewPhoto)}
          <p class="hint">${tx('outline_note')}</p>
        </div>
        <div class="q-review-list">
          <p class="q-count" aria-live="polite"><strong>${tx('review_count', { accepted: acc, total: state.pieces.length })}</strong>${wait ? ` · ${tx('review_waiting', { n: wait })}` : ''}</p>
          ${wait ? `<button type="button" class="q-wide" data-q="accept-all" id="q-accept-all" aria-describedby="q-accept-all-hint">${icon('check')}${tx('accept_all')}</button><p class="hint" id="q-accept-all-hint">${tx('accept_all_hint')}</p>` : ''}
          <ul class="q-pieces">${state.pieces.map(pieceRow).join('')}</ul>
        </div>
      </div>
      <div class="q-dock"><button type="button" class="primary q-big" data-q="goto" data-step="order" id="q-to-order" ${S.blocker(state, 'order') ? 'aria-disabled="true"' : ''}>${tx('to_order')}${icon('arrow-right')}</button>
        ${S.blocker(state, 'order') ? `<p class="hint q-why">${tx(`blocked_${S.blocker(state, 'order')}` as QuickKey)}</p>` : ''}</div>`;
  }
  function photoOverlay(photoId: string): string {
    const photo = state.photos.find(p => p.id === photoId), scan = photo?.scan;
    const url = overlayUrls.get(photoId) ?? photoUrls.get(photoId);
    if (!photo || !scan || !url) return '';
    const [w, h] = scan.processedSize;
    const map = scan.scale.method === 'a4-sheet' ? mmToOverlay(scan.scale.imageToMm, scan.sourceSize, scan.processedSize) : null;
    const shapes = map ? state.pieces.filter(p => p.piece.photoId === photoId).map(p => {
      const d = p.piece.outline.map(map).map(([x, y], k) => `${k ? 'L' : 'M'}${x.toFixed(1)} ${y.toFixed(1)}`).join('') + 'Z';
      return `<path d="${d}" class="q-shape q-${p.status}${state.selected === p.piece.id ? ' q-selected' : ''}" data-q="open-piece" data-id="${esc(p.piece.id)}"/>`;
    }).join('') : '';
    return `<div class="q-photo" style="aspect-ratio:${w} / ${h}"><img src="${esc(url)}" alt="${tx('photo_label', { n: state.photos.indexOf(photo) + 1 })}"><svg viewBox="0 0 ${w} ${h}" aria-hidden="true" preserveAspectRatio="none">${shapes}</svg></div>`;
  }
  /** Draw pieces the way they lie in the photo (A4 mode), so a thumbnail matches what the user sees.
   * Without a photo map (BETA target mode) pieces are drawn in mm with y up. */
  const photoMaps = new Map<string, ((p: Pt) => Pt) | null>();
  function toView(photoId: string): (p: Pt) => Pt {
    if (!photoMaps.has(photoId)) {
      const scan = state.photos.find(ph => ph.id === photoId)?.scan;
      photoMaps.set(photoId, scan && scan.scale.method === 'a4-sheet' ? mmToOverlay(scan.scale.imageToMm, scan.sourceSize, scan.processedSize) : null);
    }
    return photoMaps.get(photoId) ?? (([x, y]) => [x, -y]);
  }
  function thumb(p: QuickPiece, size = 64): string {
    const view = toView(p.photoId), line = (pts: Pt[]) => pathData(pts.map(view), false).replace(/Z$/, '');
    const outline = p.outline.map(view), b = bounds(outline), m = Math.max(b.width, b.height) * .06;
    return `<svg class="q-piece-svg" width="${size}" height="${size}" viewBox="${(b.minX - m).toFixed(1)} ${(b.minY - m).toFixed(1)} ${(b.width + 2 * m).toFixed(1)} ${(b.height + 2 * m).toFixed(1)}" aria-hidden="true">
      <path d="${pathData(outline, false)}" class="q-piece-shape"/>
      ${p.fold_line ? `<path d="${line(p.fold_line)}" class="q-fold-line"/>` : ''}
      <path d="${line(S.shownGrain(p))}" class="q-grain-line${p.grain ? '' : ' q-suggested'}" marker-end="url(#q-arrow)"/>
    </svg>`;
  }
  function pieceRow(r: S.ReviewPiece): string {
    const p = r.piece, n = S.copies(p), chipsN = S.chips(state, p.id).length;
    const copiesText = S.isPair(p) ? `${p.quantity ?? 0} + ${p.mirror_quantity ?? 0} ↔` : `× ${n}`;
    const removed = r.removedAsPairOf ? tx('removed_pair', { name: label(r.removedAsPairOf) }) : r.removedAsDuplicateOf ? tx('removed_duplicate', { name: label(r.removedAsDuplicateOf) }) : '';
    return `<li><button type="button" class="q-piece q-${r.status}" data-q="open-piece" data-id="${esc(p.id)}" id="q-pc-${cssId(p.id)}" aria-haspopup="dialog">
      ${thumb(p)}<span class="q-piece-text"><strong>${esc(pieceName(p.name))}</strong>
      <span class="q-piece-meta">${state.photos.length > 1 ? `<span>${tx('photo_label', { n: state.photos.findIndex(ph => ph.id === p.photoId) + 1 })}</span>` : ''}<span class="chip">${esc(sizeName(p.size ?? state.defaultSize))}</span><span>${copiesText}</span>${p.fold_line ? `<span class="q-mini">${qicon('fold')}</span>` : ''}${chipsN ? `<span class="q-mini q-sugg">${qicon('sparkle')}${chipsN}</span>` : ''}</span>
      ${removed ? `<small>${removed}</small>` : ''}</span>
      <span class="q-badge q-${r.status}">${r.status === 'accepted' ? icon('check') : r.status === 'deleted' ? icon('close') : icon('alert')}${tx(`status_${r.status}` as QuickKey)}</span></button></li>`;
  }
  const label = (id: string) => pieceName(state.pieces.find(p => p.piece.id === id)?.piece.name ?? '');
  const cssId = (id: string) => id.replace(/[^a-zA-Z0-9_-]/g, '_');

  // ----- Piece sheet (dialog) -----
  function openPiece(id: string): void {
    state = S.select(state, id); renderSheet();
    if (!sheet.open) sheet.showModal();
    sheet.querySelector<HTMLElement>('#q-sheet-title')?.focus();
    render();
  }
  function renderSheet(): void {
    const r = state.pieces.find(p => p.piece.id === state.selected); if (!r) { sheet.close(); return; }
    const p = r.piece, chips = S.chips(state, p.id), pair = S.isPair(p);
    const keep = document.activeElement && sheet.contains(document.activeElement) ? (document.activeElement as HTMLElement).id : '';
    const chipText = (c: S.Chip) => {
      switch (c.kind) {
        case 'name': return tx('chip_name', { value: pieceName(c.value) });
        case 'pair': return tx('chip_pair');
        case 'quantity': { const [a, b] = c.value.split('+'); return b === '0' ? tx('chip_quantity_plain', { normal: a! }) : tx('chip_quantity', { normal: a!, mirrored: b! }); }
        case 'fold': return tx('chip_fold');
        case 'grain': return tx('chip_grain');
        case 'duplicate': return tx('chip_duplicate');
      }
    };
    const chipIcon: Record<S.SuggestionKind, string> = { name: qicon('sparkle'), pair: qicon('pair'), quantity: icon('plus'), fold: qicon('fold'), grain: qicon('grain'), duplicate: icon('trash') };
    const copies = pair ? (p.quantity ?? 1) : S.copies(p);
    $('q-sheet-body').innerHTML = `<div class="q-sheet-head"><h2 id="q-sheet-title" tabindex="-1">${esc(pieceName(p.name))}</h2><span class="q-badge q-${r.status}">${tx(`status_${r.status}` as QuickKey)}</span>
        <button type="button" class="ghost icon-btn" data-q="close-sheet" id="q-sheet-close" aria-label="${tx('close')}">${icon('close')}</button></div>
      <div class="q-sheet-piece">${thumb(p, 160)}</div>
      <section aria-labelledby="q-sugg-title" class="q-sugg"><h3 id="q-sugg-title">${qicon('sparkle')}${tx('suggestions')}</h3>
        ${chips.length ? `<ul class="q-chips">${chips.map(c => `<li class="q-chip"><button type="button" class="q-chip-yes" data-q="chip" data-kind="${c.kind}" id="q-chip-${c.kind}">${chipIcon[c.kind]}<span>${chipText(c)}</span><small class="q-conf q-${c.confidence}">${tx(`conf_${c.confidence}` as QuickKey)}</small></button><button type="button" class="q-chip-no ghost icon-btn" data-q="chip-no" data-kind="${c.kind}" aria-label="${tx('chip_dismiss')}: ${chipText(c)}" id="q-chip-no-${c.kind}">${icon('close')}</button></li>`).join('')}</ul>` : `<p class="hint">${tx('suggest_none')}</p>`}
      </section>
      <div class="q-fields">
        <label>${tx('name')}<input id="q-name" value="${esc(pieceName(p.name))}" data-q-change="name" autocomplete="off" maxlength="40"></label>
        <fieldset class="q-size-pick"><legend>${tx('size')}</legend><div class="q-pills">${[...new Set([...S.SIZE_PRESETS, p.size ?? state.defaultSize])].map(s => `<label class="q-pill"><input type="radio" name="q-psize" value="${esc(s)}" data-q-change="size" ${s === (p.size ?? state.defaultSize) ? 'checked' : ''}><span>${esc(sizeName(s))}</span></label>`).join('')}</div>
          <label class="q-other">${tx('size_other')}<input id="q-size-other" data-q-change="size-other" autocomplete="off" maxlength="12" placeholder="38, 40, 6-7Y…"></label></fieldset>
        ${switchRow('q-pair', 'pair', pair, 'pair', 'pair_hint')}
        ${stepper('q-copies', pair ? 'copies_pair' : 'copies', copies, 'copies', 1, 20)}
        ${S.canFold(p) ? switchRow('q-fold', 'fold', !!p.fold_line, 'fold', 'fold_hint') : `<p class="hint">${qicon('fold')} ${tx('fold_none')}</p>`}
        <div class="q-row"><button type="button" data-q="grain-turn" id="q-grain">${qicon('grain')}${tx('grain_turn')}</button><span class="hint">${tx('grain_hint')} ${tx(p.grain ? 'grain_confirmed' : 'grain_unconfirmed')}</span></div>
      </div>
      <div class="q-sheet-actions">
        ${r.status === 'deleted' ? `<button type="button" data-q="restore" id="q-restore">${icon('undo')}${tx('restore')}</button>` : `<button type="button" class="danger" data-q="delete" id="q-delete">${icon('trash')}${tx('delete')}</button>`}
        ${r.status === 'accepted' ? `<button type="button" class="primary" data-q="next-piece" id="q-next">${tx(nextPending() ? 'next_piece' : 'done')}</button>` : r.status === 'pending' ? `<button type="button" class="primary" data-q="accept" id="q-accept">${icon('check')}${tx(p.grain ? 'use_piece' : 'use_piece_grain')}</button>` : `<button type="button" data-q="close-sheet" id="q-done">${tx('close')}</button>`}
      </div>`;
    if (keep) document.getElementById(keep)?.focus();
  }
  const nextPending = () => S.pending(state).find(p => p.piece.id !== state.selected)?.piece.id;
  function switchRow(id: string, action: string, on: boolean, title: QuickKey, hint: QuickKey): string {
    return `<label class="q-switch" for="${id}"><span><strong>${tx(title)}</strong><small>${tx(hint)}</small></span><input type="checkbox" role="switch" id="${id}" data-q-change="${action}" ${on ? 'checked' : ''}></label>`;
  }
  function stepper(id: string, title: QuickKey, value: number, action: string, min: number, max: number, arg = ''): string {
    const name = esc(t(title)) + (arg ? ` ${esc(sizeName(arg))}` : '');
    return `<div class="q-stepper" role="group" aria-labelledby="${id}-label"><span id="${id}-label" class="q-stepper-label">${arg ? `<span class="chip">${esc(sizeName(arg))}</span>` : tx(title)}</span>
      <div class="q-stepper-row"><button type="button" class="q-step-btn" data-q="${action}" data-delta="-1" data-arg="${esc(arg)}" id="${id}-minus" aria-label="${tx('minus')}: ${name}" ${value <= min ? 'disabled' : ''}>${icon('minus')}</button>
      <input type="number" inputmode="numeric" id="${id}" value="${value}" min="${min}" max="${max}" step="1" data-q-change="${action}-set" data-arg="${esc(arg)}" aria-label="${name}">
      <button type="button" class="q-step-btn" data-q="${action}" data-delta="1" data-arg="${esc(arg)}" id="${id}-plus" aria-label="${tx('plus')}: ${name}" ${value >= max ? 'disabled' : ''}>${icon('plus')}</button></div></div>`;
  }
  function setCopies(value: number): void {
    const id = state.selected, p = state.pieces.find(r => r.piece.id === id)?.piece; if (!id || !p) return;
    state = S.isPair(p) ? S.setQuantity(state, id, value, value) : S.setQuantity(state, id, value, 0);
    changed();
  }
  /** Any piece or order edit makes an old marker out of date. */
  function changed(): void { if (state.nest.result || state.nest.status !== 'idle') state = S.clearNest(state); render(); }

  // ----- Fabric and order -----
  let widthText = '';
  function orderHtml(): string {
    const f = state.fabric, unit = f.unit;
    if (!widthText && f.widthMm) widthText = formatWidth(f.widthMm, unit);
    const problem = widthProblem(f.widthMm);
    const sizes = S.orderSizes(state);
    return `<h1 id="q-title" tabindex="-1">${tx('order_title')}</h1>${practiceNote()}
      <section class="q-card" aria-labelledby="q-width-label"><h2 id="q-width-label">${tx('width')}</h2>
        <div class="q-width-row"><input id="q-width" type="text" inputmode="decimal" value="${esc(widthText)}" data-q-input="width" aria-labelledby="q-width-label" aria-describedby="q-width-error" aria-invalid="${!!problem}">
          <div class="segmented q-unit" role="group" aria-label="${tx('width_unit')}">${(['inch', 'cm'] as const).map(u => `<button type="button" data-q="unit" data-unit="${u}" aria-pressed="${unit === u}" id="q-unit-${u}">${u === 'inch' ? 'inch' : 'cm'}</button>`).join('')}</div></div>
        <p id="q-width-error" class="q-why ${problem ? 'error-text' : ''}" aria-live="polite">${problem ? tx(`width_${problem}` as QuickKey) : `= ${(f.widthMm! / 10).toFixed(1)} cm · ${(f.widthMm! / 25.4).toFixed(1)} inch`}</p>
        <div class="q-pills" role="group" aria-label="${tx('presets')}">${WIDTH_PRESETS[unit].map(v => `<button type="button" class="q-preset" data-q="preset" data-value="${v}" aria-pressed="${!!f.widthMm && Math.abs(toMm(v, unit) - f.widthMm) < 0.5}">${v} ${unit === 'inch' ? '"' : 'cm'}</button>`).join('')}</div></section>
      <fieldset class="q-card q-layouts"><legend>${tx('layout')}</legend>
        ${(['open', 'folded', 'tubular'] as const).map(l => `<label class="q-layout"><input type="radio" name="q-layout" value="${l}" data-q-change="layout" ${f.layout === l ? 'checked' : ''}>${ART[l]}<span><strong>${tx(`layout_${l}` as QuickKey)}</strong><small>${tx(`layout_${l}_hint` as QuickKey)}</small></span></label>`).join('')}</fieldset>
      <div class="q-card">${switchRow('q-nap', 'nap', f.nap, 'nap', 'nap_hint')}</div>
      <section class="q-card" aria-labelledby="q-garments-title"><h2 id="q-garments-title">${tx('garments')}</h2><p class="hint">${tx('garments_hint')}</p>
        <div class="q-garments">${sizes.map(s => stepper(`q-g-${cssId(s)}`, 'garments', state.garments[s] ?? 0, 'garments', 0, 99, s)).join('')}</div>
        ${S.totalGarments(state) ? '' : `<p class="error-text">${tx('garments_none')}</p>`}</section>
      <fieldset class="q-card"><legend>${tx('time')}</legend><div class="segmented q-time" role="group">${BUDGETS.map(([k, s]) => `<button type="button" data-q="budget" data-s="${s}" aria-pressed="${state.nest.budgetS === s}" id="q-budget-${s}">${tx(k)} <small>${s < 60 ? `${s} s` : `${s / 60} min`}</small></button>`).join('')}</div></fieldset>
      <div class="q-dock"><button type="button" class="primary q-big" data-q="nest" id="q-make" ${S.blocker(state, 'nest') ? 'aria-disabled="true"' : ''}>${icon('play')}${tx('make_marker')}</button>
        ${S.blocker(state, 'nest') ? `<p class="hint q-why">${tx(`blocked_${S.blocker(state, 'nest')}` as QuickKey)}</p>` : ''}</div>`;
  }

  // ----- Nest -----
  function nestHtml(): string {
    const n = state.nest, running = n.status === 'running';
    const elapsed = running ? (performance.now() - nestStart) / 1000 : n.elapsedS;
    return `<h1 id="q-title" tabindex="-1">${tx('nest_title')}</h1>${practiceNote()}
      ${canNest === false && !practice ? `<div class="notice warn q-notice">${icon('alert')}<span>${tx('nest_no_engine')}</span></div>` : ''}
      ${n.status === 'failed' ? `<div class="notice error q-notice" role="alert">${icon('alert')}<span>${tx('nest_failed', { error: n.error ?? '' })}</span></div><button type="button" class="primary q-big" data-q="nest" id="q-retry">${icon('rotate')}${tx('nest_retry')}</button>` : ''}
      ${n.status === 'idle' ? `<button type="button" class="primary q-big" data-q="nest" id="q-make-again">${icon('play')}${tx('make_marker')}</button>` : ''}
      ${n.status !== 'failed' && n.status !== 'idle' ? `<div class="q-card q-nest" aria-busy="${running}">
        <p class="q-nest-state" id="q-nest-state">${running ? `<span class="q-spinner" aria-hidden="true"></span>${tx(nestAbort?.signal.aborted ? 'nest_stopping' : 'nest_running')}` : ''}</p>
        <progress id="q-nest-bar" max="${n.budgetS}" value="${Math.min(n.budgetS, elapsed).toFixed(1)}" aria-label="${tx('time')}"></progress>
        <p class="q-nest-time" id="q-nest-time">${tx('nest_elapsed', { s: Math.round(Math.min(n.budgetS, elapsed)), total: n.budgetS })}</p>
        <div class="q-tiles"><div><span>${tx('nest_best')}</span><strong id="q-best-len">${n.bestLengthMm ? `${metres(n.bestLengthMm)} m` : '–'}</strong></div><div><span>${tx('efficiency')}</span><strong id="q-best-eff">${n.bestEfficiency != null ? `${percent(n.bestEfficiency)}%` : '–'}</strong></div></div>
        <div class="q-preview" id="q-preview">${n.result && n.job ? markerSvg(n.job, n.result) : `<p class="hint">${tx('nest_none_yet')}</p>`}</div>
        ${running ? `<button type="button" class="q-big" data-q="stop" id="q-stop" ${nestAbort?.signal.aborted ? 'disabled' : ''}>${icon('stop')}${tx('nest_stop')}</button>` : ''}
      </div>` : ''}`;
  }
  function placedOutlines(job: Job, result: MarkerResult): Pt[][] {
    if (markerCore) try { return markerCore.drawing(job, result.placements).outlines; } catch { /* fall back */ }
    const byId = new Map(job.pieces.map(p => [p.id, p]));
    return result.placements.map(pl => {
      const c = Math.cos(pl.rotation_deg * Math.PI / 180), s = Math.sin(pl.rotation_deg * Math.PI / 180);
      return (byId.get(pl.piece_id)?.outline ?? []).map(([x0, y]) => { const x = pl.mirrored ? -x0 : x0; return [c * x - s * y + pl.translate[0], s * x + c * y + pl.translate[1]] as Pt; });
    });
  }
  function markerSvg(job: Job, result: MarkerResult): string {
    const W = job.fabric.width, L = Math.max(result.length, 1), outlines = placedOutlines(job, result);
    const stroke = Math.max(L, W) / 400;
    const colours = (id: string) => `var(--fill-${Math.abs([...id].reduce((a, c) => a * 31 + c.charCodeAt(0), 7)) % 6})`;
    return `<svg class="q-marker" viewBox="${-stroke * 4} ${-W - stroke * 4} ${L + stroke * 8} ${W + stroke * 8}" role="img" aria-label="${tx('length')} ${metres(result.length)} m, ${tx('efficiency')} ${percent(result.efficiency)}%">
      <rect x="0" y="${-W}" width="${L}" height="${W}" class="q-fabric" style="stroke-width:${stroke}"/>
      ${outlines.map((o, k) => `<path d="${pathData(o)}" style="fill:${colours(result.placements[k]?.piece_id ?? '')};stroke-width:${stroke}" class="q-placed"/>`).join('')}</svg>`;
  }

  // ----- Use -----
  function useHtml(): string {
    const n = state.nest, job = n.job!, result = n.result!;
    const garments = S.totalGarments(state), per = metresPerGarment(result.length, garments), saving = savingVsUsual(per, state.usualM);
    const fake = isPractice(result);
    return `<h1 id="q-title" tabindex="-1">${tx('use_title')}</h1>${practiceNote()}${fake ? `<p class="notice warn q-notice">${icon('alert')}<span>${tx('practice_result')}</span></p>` : ''}
      <div class="q-tiles q-result">
        <div><span>${tx('length')}</span><strong>${metres(result.length)} m</strong></div>
        <div><span>${tx('efficiency')}</span><strong>${percent(result.efficiency)}%</strong></div>
        <div><span>${tx('per_garment')}</span><strong>${per == null ? '–' : `${per.toFixed(2)} m`}</strong><small>${tx('garments_count', { n: garments })}</small></div>
      </div>
      <div class="q-preview">${markerSvg(job, result)}</div>
      <div class="q-card q-usual"><label>${tx('usual')} (m)<input id="q-usual" type="text" inputmode="decimal" value="${state.usualM ?? ''}" data-q-input="usual" placeholder="1.60" aria-describedby="q-usual-hint q-saving"></label>
        <p class="hint" id="q-usual-hint">${tx('usual_hint')}</p>
        <p id="q-saving" class="q-saving ${saving ? (saving.better ? 'ok' : 'warn') : ''}" aria-live="polite">${saving ? (saving.better ? `${icon('check-circle')}${tx('saved', { m: saving.savedM.toFixed(2), pct: percent(saving.savedPct) })}` : tx('saved_none')) : ''}</p></div>
      <h2>${tx('use_it')}</h2>
      <div class="q-use-grid">
        <div class="q-use-card"><button type="button" class="q-use" data-q="tiles" id="q-tiles">${icon('print')}<span><strong>${tx('print_tiles')}</strong><small>${tx('print_tiles_hint')}</small></span></button>
          <div class="segmented q-paper" role="group" aria-label="${tx('paper')}">${(['A4', 'A3'] as const).map(p => `<button type="button" data-q="paper" data-paper="${p}" aria-pressed="${paper === p}" id="q-paper-${p}">${p}</button>`).join('')}</div></div>
        <button type="button" class="q-use" data-q="chalk" id="q-chalk">${qicon('chalk')}<span><strong>${tx('chalk')}</strong><small>${tx('chalk_hint')}</small></span></button>
        <button type="button" class="q-use" data-q="projector" id="q-projector-btn">${qicon('projector')}<span><strong>${tx('projector')}</strong><small>${tx('projector_hint')}</small></span></button>
        <button type="button" class="q-use" data-q="share-pdf" id="q-share-pdf">${qicon('share')}<span><strong>${tx('share_pdf')}</strong><small>${tx('share_hint')}</small></span></button>
        <button type="button" class="q-use" data-q="share-plt" id="q-share-plt">${qicon('share')}<span><strong>${tx('share_plt')}</strong><small>${tx('plt_needs_server')}</small></span></button>
        <button type="button" class="q-use" data-q="editor" id="q-editor">${icon('marker')}<span><strong>${tx('open_editor')}</strong></span></button>
      </div>
      <button type="button" class="ghost q-wide" data-q="restart" id="q-restart">${icon('new')}${tx('start_again')}</button>`;
  }

  // ---------- Photos ----------
  async function addFiles(files: File[], forceMock = false): Promise<void> {
    if (!files.length) return;
    const e = await ready(forceMock);
    if (!e) { render(); host.message(engineProblem, true); return; }
    if (state.step === 'start') { state = { ...state, step: 'capture' }; render(true); }
    for (const file of files) {
      const id = `photo-${++photoCounter}`;
      photoUrls.set(id, URL.createObjectURL(file));
      state = S.addPhoto(state, id, file.name); render(); say(t('photo_reading'));
      try {
        if (file.size > MAX_PHOTO) throw new Error('Photo is larger than 20 MB. Use the normal camera quality.');
        const scan = await e.scan(new Uint8Array(await file.arrayBuffer()), { reference: state.reference, photoId: id });
        if (scan.overlayPng.length) overlayUrls.set(id, URL.createObjectURL(new Blob([scan.overlayPng as BlobPart], { type: 'image/png' })));
        state = S.photoScanned(state, id, scan);
        const done = state.photos.filter(p => p.status === 'done' && p.scan).map(p => p.scan!);
        state = S.applySession(state, await e.merge(done));
        say(t('photo_found', { n: scan.pieces.length }));
      } catch (error) {
        state = S.photoFailed(state, id, errorText(error)); say(`${t('photo_failed')}: ${errorText(error)}`);
      }
      photoMaps.delete(id); state = S.clearNest(state); render();
    }
  }
  async function trySample(): Promise<void> {
    const blobs = await Promise.all([samplePhoto(0), samplePhoto(1)]);
    await addFiles(blobs.map((b, i) => new File([b], `sample-${i + 1}.jpg`, { type: 'image/jpeg' })), true);
  }

  // ---------- Nest ----------
  async function startNest(): Promise<void> {
    const why = S.blocker(state, 'nest');
    if (why) { flashWhy(t(`blocked_${why}` as QuickKey)); return; }
    if (!engine) return;
    let job: Job;
    try {
      job = await engine.toJob(S.jobPieces(state), { width_mm: state.fabric.widthMm!, nap: state.fabric.nap ? 'one_way' : 'none', layout: state.fabric.layout }, S.garmentsFor(state));
    } catch (error) { host.message(t('nest_failed', { error: errorText(error) }), true); return; }
    const real = await nester();
    state = S.nestStarted(state, state.nest.budgetS); state = { ...state, nest: { ...state.nest, job } };
    nestAbort = new AbortController(); nestStart = performance.now(); render(true);
    if (!real && !practice) { state = S.nestFailed(state, t('nest_no_engine')); nestAbort = null; render(); return; }
    window.clearInterval(nestTimer);
    nestTimer = window.setInterval(() => { if (state.step === 'nest' && state.nest.status === 'running') updateNestLive(); }, 500);
    const onProgress = (p: { elapsedS: number; best: MarkerResult | null; statusLengthMm?: number | null; statusEfficiency?: number | null }) => {
      const len = p.best ? Math.min(p.best.length, p.statusLengthMm ?? Infinity) : p.statusLengthMm ?? null;
      state = S.nestProgress(state, p.elapsedS, len, p.best?.efficiency ?? p.statusEfficiency ?? null, job, p.best ?? undefined);
      if (p.best && document.getElementById('q-preview')) $('q-preview').innerHTML = markerSvg(job, p.best);
      updateNestLive();
    };
    try {
      const result = real ? await nestInRounds(apiFetch, job, { budgetS: state.nest.budgetS, signal: nestAbort.signal, onProgress }) : await nestPractice(job, { budgetS: 1, onProgress });
      if (result) state = S.nestProgress(state, (performance.now() - nestStart) / 1000, result.length, result.efficiency, job, result);
      state = S.nestFinished(state, nestAbort.signal.aborted);
      const best = state.nest.result;
      if (best) {
        state = { ...state, step: 'use' };
        say(`${t('length')} ${metres(best.length)} m`);
        if (!milestone && !isPractice(best)) { milestone = true; window.dispatchEvent(new Event('beta-feedback-milestone')); }
      }
    } catch (error) {
      state = S.nestFailed(state, errorText(error));
      if (state.nest.result) state = { ...state, step: 'use' };
      host.message(t('nest_failed', { error: errorText(error) }), true);
    } finally { window.clearInterval(nestTimer); nestAbort = null; render(true); }
  }
  function updateNestLive(): void {
    const n = state.nest, elapsed = Math.min(n.budgetS, (performance.now() - nestStart) / 1000);
    const bar = document.getElementById('q-nest-bar') as HTMLProgressElement | null; if (!bar) return;
    bar.value = elapsed;
    $('q-nest-time').textContent = t('nest_elapsed', { s: Math.round(elapsed), total: n.budgetS });
    $('q-best-len').textContent = n.bestLengthMm ? `${metres(n.bestLengthMm)} m` : '–';
    $('q-best-eff').textContent = n.bestEfficiency != null ? `${percent(n.bestEfficiency)}%` : '–';
  }

  // ---------- Files ----------
  async function share(blob: Blob, name: string): Promise<void> {
    const file = new File([blob], name, { type: blob.type || 'application/octet-stream' });
    const nav = navigator as Navigator & { canShare?: (d: ShareData) => boolean };
    if (!isDesktop() && nav.canShare?.({ files: [file] })) {
      try { await navigator.share({ files: [file], title: name }); host.message(t('shared')); return; }
      catch (error) { if ((error as Error).name === 'AbortError') return; }
    }
    const saved = await downloadFile(blob, name);
    if (saved) host.message(t('file_ready', { name: saved }));
  }
  async function makeFile(kind: 'tiles' | 'chalk' | 'pdf' | 'plt'): Promise<void> {
    const n = state.nest; if (!engine || !n.job || !n.result) return;
    const button = root.querySelector<HTMLButtonElement>(`[data-q="${kind === 'pdf' ? 'share-pdf' : kind === 'plt' ? 'share-plt' : kind}"]`);
    if (button) button.setAttribute('aria-busy', 'true');
    try {
      if (kind === 'tiles') { const bytes = await engine.tiledPdf(n.job, n.result, { paper, overlap_mm: 10 }); await downloadOrShare(bytes, `marker-tiles-${paper}.pdf`, false); }
      if (kind === 'chalk') { const bytes = await engine.chalkPdf(n.job, n.result, { language: currentLanguage() }); await downloadOrShare(bytes, `chalk-guide-${currentLanguage()}.pdf`, false); }
      if (kind === 'pdf') {
        const blob = new Blob([await engine.tiledPdf(n.job, n.result, { paper, overlap_mm: 10 }) as BlobPart], { type: 'application/pdf' });
        await share(blob, 'marker.pdf');
      }
      if (kind === 'plt') {
        if (!(await serverAvailable(apiFetch))) { host.message(t('plt_needs_server'), true); return; }
        await share(await exportFile(apiFetch, n.job, n.result, 'plt'), 'marker.plt');
      }
    } catch (error) { host.message(t('file_failed', { error: errorText(error) }), true); }
    finally { button?.removeAttribute('aria-busy'); }
  }
  async function downloadOrShare(bytes: Uint8Array, name: string, shareIt: boolean): Promise<void> {
    const blob = new Blob([bytes as BlobPart], { type: 'application/pdf' });
    if (shareIt) { await share(blob, name); return; }
    const saved = await downloadFile(blob, name); if (saved) host.message(t('file_ready', { name: saved }));
  }

  // ---------- Projector ----------
  let cal: Calibration = loadCalibration(storage()), layout: ProjectorLayout | null = null, part = 0, calOpen = false, inverse = false;
  let wake: { release(): Promise<void> } | null = null, hideTimer = 0, opener: HTMLElement | null = null;
  function storage(): Storage | null { try { return window.localStorage; } catch { return null; } }
  async function openProjector(): Promise<void> {
    const n = state.nest; if (!engine || !n.job || !n.result) return;
    try { layout = await engine.projector(n.job, n.result); } catch (error) { host.message(t('file_failed', { error: errorText(error) }), true); return; }
    opener = document.activeElement as HTMLElement; part = 0; calOpen = !cal.measured;
    proj.hidden = false; document.body.classList.add('q-projecting');
    try { await proj.requestFullscreen?.(); } catch { /* the overlay still covers the window */ }
    await keepAwake();
    drawProjector(); proj.querySelector<HTMLElement>('#q-proj-title')?.focus();
  }
  async function keepAwake(): Promise<void> {
    const nav = navigator as Navigator & { wakeLock?: { request(type: 'screen'): Promise<{ release(): Promise<void> }> } };
    try { wake = (await nav.wakeLock?.request('screen')) ?? null; } catch { wake = null; }
  }
  document.addEventListener('visibilitychange', () => { if (!proj.hidden && document.visibilityState === 'visible') void keepAwake().then(drawProjector); });
  async function closeProjector(): Promise<void> {
    proj.hidden = true; document.body.classList.remove('q-projecting');
    try { if (document.fullscreenElement) await document.exitFullscreen(); } catch { /* ignore */ }
    try { await wake?.release(); } catch { /* ignore */ } wake = null;
    (opener?.isConnected ? opener : $('q-projector-btn'))?.focus();
  }
  document.addEventListener('fullscreenchange', () => { if (!proj.hidden) drawProjector(); });
  window.addEventListener('resize', () => { if (!proj.hidden) drawProjector(); });
  function drawProjector(): void {
    if (!layout) return;
    const vw = window.innerWidth, vh = window.innerHeight;
    const parts = sections(layout.lengthMm, layout.widthMm, { width: vw, height: vh }, cal);
    part = Math.max(0, Math.min(part, parts.length - 1));
    const sec = parts[part]!;
    const map = toScreen(sec, cal, vh);
    const ticks: string[] = [];
    for (let x = Math.ceil(sec.x0 / 100) * 100; x <= Math.min(sec.x1, layout.lengthMm); x += 100) {
      const [sx] = map([x, 0]); const big = x % 500 === 0;
      ticks.push(`<path d="M${sx.toFixed(1)} ${vh}v${big ? -26 : -12}" class="q-tick"/>${big ? `<text x="${(sx + 4).toFixed(1)}" y="${vh - 30}" class="q-tick-label">${(x / 1000).toFixed(1)} m</text>` : ''}`);
    }
    const shapes = layout.pieces.map(p => {
      const d = p.outline.map(map).map(([x, y], k) => `${k ? 'L' : 'M'}${x.toFixed(1)} ${y.toFixed(1)}`).join('') + 'Z';
      const b = bounds(p.outline.map(map)), name = `${pieceName(p.name)}${p.size ? ` · ${sizeName(p.size)}` : ''}${p.mirrored ? ' ↔' : ''}`;
      const [rx, ry] = map(p.referencePoint);
      // Label inside the visible part of the piece, so it can be read at 1:1 when the piece is larger than the screen.
      const lx0 = Math.max(b.minX, 0), lx1 = Math.min(b.maxX, vw), ly0 = Math.max(b.minY, 70), ly1 = Math.min(b.maxY, vh - 40);
      const text = lx1 - lx0 > 40 && ly1 - ly0 > 20 ? `<text x="${((lx0 + lx1) / 2).toFixed(1)}" y="${((ly0 + ly1) / 2).toFixed(1)}" class="q-proj-label" text-anchor="middle">${esc(name)}</text>` : '';
      return `<path d="${d}" class="q-proj-piece"/><circle cx="${rx.toFixed(1)}" cy="${ry.toFixed(1)}" r="5" class="q-proj-ref"/>${text}`;
    }).join('');
    const [ex] = map([layout.lengthMm, 0]), [sx0] = map([0, 0]), [, top] = map([0, layout.widthMm]), [, sel] = map([0, 0]);
    const sq = squarePx(cal, layout.calibrationSquareMm || 100);
    proj.classList.toggle('q-inverse', inverse);
    proj.innerHTML = `<svg class="q-proj-svg" width="${vw}" height="${vh}" viewBox="0 0 ${vw} ${vh}" aria-hidden="true">
        <path d="M${sx0.toFixed(1)} ${sel.toFixed(1)}H${ex.toFixed(1)}" class="q-proj-edge"/><path d="M${sx0.toFixed(1)} ${top.toFixed(1)}H${ex.toFixed(1)}" class="q-proj-edge"/>
        <path d="M${sx0.toFixed(1)} ${sel.toFixed(1)}V${top.toFixed(1)}" class="q-proj-edge"/><path d="M${ex.toFixed(1)} ${sel.toFixed(1)}V${top.toFixed(1)}" class="q-proj-edge"/>
        ${shapes}${ticks.join('')}
        ${sel > 40 && sel < vh + 2 ? `<text x="${Math.max(sx0, 0) + 8}" y="${(Math.min(sel, vh) - 40).toFixed(1)}" class="q-proj-note">${tx('proj_selvedge')} (Y = 0)</text>` : ''}
        ${sx0 >= 0 && sx0 < vw - 120 ? `<text x="${(sx0 + 8).toFixed(1)}" y="${Math.max(top, 0) + 96}" class="q-proj-note">${tx('proj_start')} (X = 0)</text>` : ''}
        ${calOpen ? `<rect x="${((vw - sq.width) / 2).toFixed(1)}" y="${((vh - sq.height) / 2).toFixed(1)}" width="${sq.width.toFixed(1)}" height="${sq.height.toFixed(1)}" class="q-proj-square"/>` : ''}
      </svg>
      <div class="q-proj-bar" id="q-proj-bar">
        <h2 id="q-proj-title" tabindex="-1" class="sr-only">${tx('projector')}</h2>
        <button type="button" data-q="proj-exit" id="q-proj-exit">${icon('close')}<span>${tx('proj_exit')}</span></button>
        <button type="button" class="icon-btn" data-q="proj-prev" id="q-proj-prev" aria-label="${tx('proj_prev')}" ${part === 0 ? 'disabled' : ''}>${icon('arrow-left')}</button>
        <span class="q-proj-part" aria-live="polite"><strong>${tx('proj_section', { n: part + 1, total: parts.length })}</strong><small>${tx('proj_range', { from: (sec.x0 / 1000).toFixed(2), to: (Math.min(sec.x1, layout.lengthMm) / 1000).toFixed(2) })}${parts.some(p => p.row > 0) ? ` · ${(sec.y0 / 1000).toFixed(2)}–${(Math.min(sec.y1, layout.widthMm) / 1000).toFixed(2)} m` : ''}</small></span>
        <button type="button" class="icon-btn" data-q="proj-next" id="q-proj-next" aria-label="${tx('proj_next')}" ${part >= parts.length - 1 ? 'disabled' : ''}>${icon('arrow-right')}</button>
        <button type="button" data-q="proj-cal" id="q-proj-cal" aria-expanded="${calOpen}">${qicon('square')}<span>${tx('proj_calibrate')}</span></button>
        <button type="button" data-q="proj-contrast" id="q-proj-contrast" aria-pressed="${inverse}">${qicon('contrast')}<span>${tx('proj_contrast')}</span></button>
        <span class="q-proj-status">${cal.measured ? `${icon('check')}${tx('proj_calibrated')}` : `${icon('alert')}${tx('proj_not_calibrated')}`}${wake ? ` · ${tx('proj_awake')}` : ''}</span>
      </div>
      ${calOpen ? `<form class="q-proj-cal" id="q-proj-form" aria-labelledby="q-cal-title"><h3 id="q-cal-title">${tx('proj_cal_title')}</h3><p>${tx('proj_square')}. ${tx('proj_cal_text')}</p>
        <div class="q-row"><label>${tx('proj_cal_width')}<input id="q-cal-w" type="text" inputmode="decimal" placeholder="100"></label><label>${tx('proj_cal_height')}<input id="q-cal-h" type="text" inputmode="decimal" placeholder="100"></label></div>
        <p id="q-cal-msg" role="status"></p>
        <div class="q-row"><button type="submit" class="primary">${tx('proj_cal_apply')}</button><button type="button" data-q="proj-reset">${tx('proj_cal_reset')}</button></div></form>` : ''}`;
    proj.querySelector('#q-proj-form')?.addEventListener('submit', event => {
      event.preventDefault();
      const w = parseNumber($<HTMLInputElement>('q-cal-w').value), h = parseNumber($<HTMLInputElement>('q-cal-h').value);
      if (measureProblem(w) || measureProblem(h)) { $('q-cal-msg').textContent = t('proj_cal_bad'); return; }
      cal = correct(cal, w!, h!); saveCalibration(storage(), cal); drawProjector();
      $('q-cal-msg').textContent = t('proj_cal_done'); $<HTMLInputElement>('q-cal-w').focus();
    });
    showBar();
  }
  function showBar(): void {
    proj.classList.remove('q-bar-hidden'); window.clearTimeout(hideTimer);
    hideTimer = window.setTimeout(() => { if (!calOpen && !proj.contains(document.activeElement) || document.activeElement === proj) proj.classList.add('q-bar-hidden'); }, 5000);
  }
  proj.addEventListener('pointermove', showBar);
  proj.addEventListener('keydown', event => {
    showBar();
    if (event.key === 'Escape') { event.preventDefault(); void closeProjector(); }
    if ((event.target as Element).closest('input')) return;
    if (event.key === 'ArrowRight' || event.key === 'PageDown') { event.preventDefault(); part++; drawProjector(); $('q-proj-next')?.focus(); }
    if (event.key === 'ArrowLeft' || event.key === 'PageUp') { event.preventDefault(); part = Math.max(0, part - 1); drawProjector(); $('q-proj-prev')?.focus(); }
  });

  // ---------- Events ----------
  const camera = $<HTMLInputElement>('q-camera'), gallery = $<HTMLInputElement>('q-gallery');
  for (const input of [camera, gallery]) input.addEventListener('change', () => { const files = Array.from(input.files ?? []); input.value = ''; void addFiles(files); });
  document.addEventListener('click', event => {
    const el = (event.target as Element).closest<HTMLElement>('[data-q]');
    if (!el || !(root.contains(el) || sheet.contains(el) || proj.contains(el))) return;
    if (el.getAttribute('aria-disabled') === 'true' && el.dataset.q !== 'goto' && el.dataset.q !== 'nest' && el.dataset.q !== 'share-plt') return;
    const id = el.dataset.id ?? state.selected ?? '';
    switch (el.dataset.q) {
      case 'camera': void ready().then(e => { if (e) camera.click(); else { render(); host.message(engineProblem, true); } }); break;
      case 'gallery': void ready().then(e => { if (e) gallery.click(); else { render(); host.message(engineProblem, true); } }); break;
      case 'sample': void trySample(); break;
      case 'install': void pwa.install(); break;
      case 'lang': { const select = document.querySelector<HTMLSelectElement>('#app-language'); if (select && !select.disabled) { select.value = el.dataset.lang!; select.dispatchEvent(new Event('change')); } else document.documentElement.dataset.language = el.dataset.lang!; break; }
      case 'print-target': void apiFetch('/api/digitize/target', { method: 'POST', body: '{}', headers: { 'Content-Type': 'application/json' } }).then(async r => { if (!r.ok) throw new Error(r.status === 404 ? t('plt_needs_server') : `Server ${r.status}`); await downloadFile(await r.blob(), 'beta-a3-target.pdf'); }).catch(error => host.message(errorText(error).includes('fetch') ? t('plt_needs_server') : errorText(error), true)); break;
      case 'back': { const prev = state.step === 'capture' ? 'start' : S.previousStep(state.step); if (prev) { state = { ...state, step: prev === 'nest' ? 'order' : prev }; render(true); } break; }
      case 'goto': goStep(el.dataset.step as S.Step); break;
      case 'remove-photo': state = S.removePhoto(state, id); revoke(id); state = S.clearNest(state); render(); break;
      case 'view-photo': viewPhoto = id; render(); break;
      case 'open-piece': openPiece(id); break;
      case 'close-sheet': sheet.close(); break;
      case 'accept-all': state = S.acceptAllPending(state); changed(); say(t('review_count', { accepted: S.accepted(state).length, total: state.pieces.length })); break;
      case 'accept': state = S.accept(state, id); changed(); { const next = nextPending(); if (next) openPiece(next); else sheet.close(); } break;
      case 'next-piece': { const next = nextPending(); if (next) openPiece(next); else sheet.close(); break; }
      case 'delete': state = S.remove(state, id); changed(); { const next = nextPending(); if (next) openPiece(next); else sheet.close(); } break;
      case 'restore': state = S.restore(state, id); changed(); break;
      case 'chip': { const chip = S.chips(state, id).find(c => c.kind === el.dataset.kind); if (chip) { state = S.applyChip(state, id, chip); changed(); say(t('done')); sheet.querySelector<HTMLElement>('.q-chip-yes, #q-name')?.focus(); } break; }
      case 'chip-no': state = S.dismissChip(state, id, el.dataset.kind as S.SuggestionKind); render(); sheet.querySelector<HTMLElement>('.q-chip-yes, #q-name')?.focus(); break;
      case 'copies': { const p = state.pieces.find(r => r.piece.id === id)?.piece; if (p) setCopies((S.isPair(p) ? (p.quantity ?? 1) : S.copies(p)) + Number(el.dataset.delta)); break; }
      case 'grain-turn': state = S.turnGrain(state, id); changed(); break;
      case 'unit': { const unit = el.dataset.unit as 'inch' | 'cm'; state = S.setUnit(state, unit); widthText = state.fabric.widthMm ? formatWidth(state.fabric.widthMm, unit) : ''; render(); break; }
      case 'preset': { const v = Number(el.dataset.value); state = S.setWidth(state, toMm(v, state.fabric.unit)); widthText = String(v); changed(); break; }
      case 'garments': { const size = el.dataset.arg!; state = S.setGarments(state, size, (state.garments[size] ?? 0) + Number(el.dataset.delta)); changed(); break; }
      case 'budget': state = { ...state, nest: { ...state.nest, budgetS: Number(el.dataset.s) } }; render(); break;
      case 'nest': void startNest(); break;
      case 'stop': nestAbort?.abort(); render(); say(t('nest_stopping')); break;
      case 'paper': paper = el.dataset.paper as 'A4' | 'A3'; render(); break;
      case 'tiles': void makeFile('tiles'); break;
      case 'chalk': void makeFile('chalk'); break;
      case 'share-pdf': void makeFile('pdf'); break;
      case 'share-plt': void makeFile('plt'); break;
      case 'projector': void openProjector(); break;
      case 'editor': { const n = state.nest; if (n.job && n.result) window.dispatchEvent(new CustomEvent<DocumentState>('beta-open-document', { detail: { job: n.job, result: n.result } })); break; }
      case 'restart': void restart(); break;
      case 'proj-exit': void closeProjector(); break;
      case 'proj-prev': part = Math.max(0, part - 1); drawProjector(); $('q-proj-prev')?.focus(); break;
      case 'proj-next': part++; drawProjector(); $('q-proj-next')?.focus(); break;
      case 'proj-cal': calOpen = !calOpen; drawProjector(); (calOpen ? $('q-cal-w') : $('q-proj-cal'))?.focus(); break;
      case 'proj-contrast': inverse = !inverse; drawProjector(); $('q-proj-contrast')?.focus(); break;
      case 'proj-reset': cal = { pxPerMmX: 96 / 25.4, pxPerMmY: 96 / 25.4, measured: false }; saveCalibration(storage(), cal); drawProjector(); break;
    }
  });
  document.addEventListener('change', event => {
    const el = event.target as HTMLInputElement;
    if (!el.dataset.qChange || !(root.contains(el) || sheet.contains(el))) return;
    const id = el.dataset.id ?? state.selected ?? '';
    switch (el.dataset.qChange) {
      case 'mode': state = { ...state, reference: el.value as S.Reference }; render(); break;
      case 'photo-size': state = S.setPhotoSize(state, id, el.value); changed(); break;
      case 'name': state = S.rename(state, id, el.value); changed(); break;
      case 'size': state = S.setSize(state, id, el.value); changed(); break;
      case 'size-other': if (el.value.trim()) { state = S.setSize(state, id, el.value); changed(); } break;
      case 'pair': state = S.setPair(state, id, el.checked); changed(); break;
      case 'fold': state = S.setFold(state, id, el.checked); changed(); break;
      case 'copies-set': setCopies(Number(el.value)); break;
      case 'layout': state = S.setLayout(state, el.value as 'open' | 'folded' | 'tubular'); changed(); break;
      case 'nap': state = S.setNap(state, el.checked); changed(); break;
      case 'garments-set': state = S.setGarments(state, el.dataset.arg!, Number(el.value)); changed(); break;
    }
  });
  // Text fields update as you type without re-rendering (keeps the keyboard open on phones).
  document.addEventListener('input', event => {
    const el = event.target as HTMLInputElement;
    if (!el.dataset.qInput || !root.contains(el)) return;
    if (el.dataset.qInput === 'width') {
      widthText = el.value; const value = parseNumber(el.value);
      state = S.clearNest(S.setWidth(state, value == null ? null : toMm(value, state.fabric.unit)));
      const problem = widthProblem(state.fabric.widthMm), msg = $('q-width-error');
      msg.textContent = problem ? t(`width_${problem}` as QuickKey) : `= ${(state.fabric.widthMm! / 10).toFixed(1)} cm · ${(state.fabric.widthMm! / 25.4).toFixed(1)} inch`;
      msg.classList.toggle('error-text', !!problem); el.setAttribute('aria-invalid', String(!!problem));
      root.querySelectorAll<HTMLButtonElement>('.q-preset').forEach(b => b.setAttribute('aria-pressed', String(!!state.fabric.widthMm && Math.abs(toMm(Number(b.dataset.value), state.fabric.unit) - state.fabric.widthMm) < 0.5)));
      const make = document.getElementById('q-make'); if (make) { if (S.blocker(state, 'nest')) make.setAttribute('aria-disabled', 'true'); else make.removeAttribute('aria-disabled'); }
    }
    if (el.dataset.qInput === 'usual') {
      state = { ...state, usualM: parseNumber(el.value) };
      const per = metresPerGarment(state.nest.result?.length ?? 0, S.totalGarments(state)), saving = savingVsUsual(per, state.usualM), out = $('q-saving');
      out.className = `q-saving ${saving ? (saving.better ? 'ok' : 'warn') : ''}`;
      out.innerHTML = saving ? (saving.better ? `${icon('check-circle')}${tx('saved', { m: saving.savedM.toFixed(2), pct: percent(saving.savedPct) })}` : tx('saved_none')) : '';
    }
  });
  sheet.addEventListener('close', () => { const id = state.selected; state = S.select(state, null); render(); if (id) document.getElementById(`q-pc-${cssId(id)}`)?.focus(); });
  sheet.addEventListener('click', event => { if (event.target === sheet) sheet.close(); });
  function revoke(id: string): void { for (const m of [photoUrls, overlayUrls]) { const u = m.get(id); if (u) URL.revokeObjectURL(u); m.delete(id); } }
  async function restart(): Promise<void> {
    if (state.photos.length && !window.confirm(t('start_again_confirm'))) return;
    for (const p of state.photos) revoke(p.id);
    const keep = { reference: state.reference, fabric: state.fabric, defaultSize: state.defaultSize };
    state = { ...S.initialState(), ...keep }; widthText = '';
    if (practice) { engine = null; practice = false; }
    render(true);
  }

  // Language: re-render on the app switch (html[data-language]) and keep the sidebar label translated.
  const navLabel = () => { const b = document.querySelector<HTMLElement>(`[data-screen="${QUICK_SCREEN}"] .nav-label`); if (b) { b.textContent = t('nav'); b.lang = currentLanguage(); } };
  new MutationObserver(() => { navLabel(); if (!root.hidden) render(); if (!proj.hidden) drawProjector(); }).observe(document.documentElement, { attributes: true, attributeFilter: ['data-language'] });
  window.addEventListener('beta-screen', event => {
    if ((event as CustomEvent<number>).detail !== QUICK_SCREEN) return;
    navLabel();
    void ready().then(() => render()).catch(() => render());
    void nester().then(() => { if (!root.hidden) render(); });
  });
  render();
  return { root };
}
