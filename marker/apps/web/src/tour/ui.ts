import { icon } from '../icons';
import { isDesktop } from '../transport';
import { byId, find, holdBackground } from '../screens';
import { appUnit } from '../units-pref';
import type { StyleControl } from '../style/ui';
import type { Step as StyleStep } from '../style/state';
import { BACKUP_KEY, card, liveLine, NUMBERED, SAMPLE, type TourStep } from './steps';
import { t, type TourKey } from './i18n';

/** "See it in 60 seconds" (Home button or the address #/tour). A card leads through the real Style screen with the
 * sample T-shirt: Pattern, Grade, Lay, live Marker and the cut plan, then offers "Try your own".
 * The tour keeps the visitor's own style (if any) and puts it back when it ends, however it ends. */
export interface TourHost {
  control: StyleControl;
  /** Open a screen through the shell (same as a menu click). */
  show(index: number): void;
  message(text: string, error?: boolean): void;
  styleScreen: number; quickScreen: number;
}
type Choice = 'close' | 'skip' | 'style' | 'photo' | 'dxf' | 'keep' | 'left';

const esc = (v: unknown): string => String(v ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]!));
const tx = (key: TourKey, values?: Record<string, string | number>) => esc(t(key, values));
const STYLE_STEP: Partial<Record<TourStep, StyleStep>> = { pattern: 'pattern', grade: 'grade', lay: 'lay', marker: 'marker', cut: 'cut' };

export function installTour(host: TourHost): { start(): void; running(): boolean } {
  const { control } = host;
  let step: TourStep | null = null, snapshot: string | null = null, hadStyle = false;
  let pending: Promise<unknown> | null = null, liveTimer = 0, loadingText = '', failure = '', leading = false;
  /** Background engine downloads wait while the tour runs, so the tour's own engines get the network. */
  let releaseBackground = () => {};
  /** Seconds from "See it in 60 seconds" to each moment (read by the headless tour smoke; nothing is sent anywhere). */
  let started = 0;
  const times: Record<string, number> = {};
  (window as { __tourTimes?: Record<string, number> }).__tourTimes = times;
  const mark = (name: string) => { times[name] = Math.round(performance.now() - started) / 1000; };

  const box = document.createElement('aside');
  box.id = 'tour'; box.className = 'tour'; box.hidden = true;
  box.setAttribute('aria-labelledby', 'tour-title');
  (find('#app') ?? document.body).append(box);

  function start(): void {
    if (step) return;
    snapshot = control.snapshot();
    try { hadStyle = !!snapshot && !!(JSON.parse(snapshot) as { doc?: unknown }).doc; } catch { hadStyle = false; }
    try { localStorage.setItem(BACKUP_KEY, JSON.stringify({ snapshot })); } catch { /* the tour still runs; a closed tab may then keep the sample */ }
    started = performance.now(); for (const k of Object.keys(times)) delete times[k];
    releaseBackground = holdBackground();
    failure = ''; step = 'loading'; loadingText = t('loading_engine');
    lead(() => host.show(host.styleScreen));
    control.touring(true);
    render(true);
    pending = (async () => {
      const ok = await control.load();
      if (step !== 'loading') return;
      if (!ok) { failure = t('loading_failed', { error: control.engineError() }); render(); return; }
      mark('engine');
      loadingText = t('loading_block'); render();
      const made = await control.sample({ block: SAMPLE.block, name: t('sample_name'), sizes: SAMPLE.sizes, lay: SAMPLE.lay });
      if (step !== 'loading') return;
      if (!made) { failure = t('loading_failed', { error: control.engineError() || '' }); render(); return; }
      mark('pattern');
      go('pattern');
    })();
  }

  /** Screen changes made by the tour itself are not "the visitor left the tour". */
  function lead(fn: () => void): void { leading = true; try { fn(); } finally { leading = false; } }

  function go(next: TourStep): void {
    step = next;
    const s = STYLE_STEP[next];
    if (s) lead(() => control.go(s));
    render(true);
    if (next === 'marker') void runMarker();
    if (next === 'end') mark('end');
  }

  async function runMarker(): Promise<void> {
    mark('marker_start');
    window.clearInterval(liveTimer);
    liveTimer = window.setInterval(renderLive, 400);
    const run = control.nest(SAMPLE.searchS);
    pending = run;
    await run;
    window.clearInterval(liveTimer);
    if (step !== 'marker') return;
    mark('marker_done');
    render(true);
  }

  function next(): void {
    if (!step) return;
    if (step === 'loading') { void finish('skip'); return; }
    if (step === 'marker' && control.progress().running) { control.stop(); return; }
    if (step === 'end') { void finish('close'); return; }
    const order: TourStep[] = [...NUMBERED, 'end'];
    go(order[order.indexOf(step) + 1] ?? 'end');
  }

  async function finish(choice: Choice): Promise<void> {
    if (!step) return;
    step = null; window.clearInterval(liveTimer); releaseBackground();
    box.hidden = true; box.innerHTML = '';
    control.stop();
    // A file chooser needs the click that asked for it, so it opens before anything waits.
    if (choice === 'dxf') { lead(() => host.show(0)); find<HTMLButtonElement>('#open-button')?.click(); }
    await pending; pending = null;
    control.touring(false);
    if (choice !== 'keep') await control.restore(snapshot);
    try { localStorage.removeItem(BACKUP_KEY); } catch { /* nothing kept */ }
    if (choice === 'style') { lead(() => host.show(host.styleScreen)); control.go('style'); }
    else if (choice === 'photo') lead(() => host.show(host.quickScreen));
    else if (choice === 'keep') { lead(() => host.show(host.styleScreen)); control.go('pattern'); }
    else if (choice === 'close' || choice === 'skip') lead(() => host.show(9));
    if (hadStyle && choice !== 'keep') host.message(t('restored'));
    if (choice !== 'left' && choice !== 'dxf') window.requestAnimationFrame(() => (byId('topbar-title') ?? byId('home-title'))?.focus({ preventScroll: true }));
  }

  // ---------- Card ----------
  function render(focusTitle = false): void {
    if (!step) return;
    box.hidden = false;
    const s = step, facts = control.facts(), c = card(s, s === 'loading' ? null : facts, appUnit());
    box.classList.toggle('tour-end', s === 'end');
    box.dataset.step = s;
    const dots = `<ol class="tour-dots" aria-hidden="true">${NUMBERED.map((n, i) => `<li class="${i + 1 < c.number ? 'done' : i + 1 === c.number ? 'now' : ''}"></li>`).join('')}</ol>`;
    const eyebrow = c.number ? `${tx('tour_label')} · ${tx('step_of', { n: c.number, total: c.total })}` : tx('tour_label');
    const head = `<div class="tour-head"><span class="tour-eyebrow">${eyebrow}</span>${s === 'end' ? '' : `<button type="button" class="ghost small tour-skip" data-tour="skip">${tx('skip')}</button>`}</div>${s === 'end' ? '' : dots}`;
    if (s === 'end') { box.innerHTML = head + endHtml(facts); }
    else {
      // From Make the marker until the search ends (the engine starts a moment after the click).
      const running = s === 'marker' && times.marker_done === undefined;
      const live = s === 'marker' ? `<p class="tour-live" id="tour-live">${running ? '<span class="tour-spinner" aria-hidden="true"></span>' : icon('check')}<span>${esc(running ? liveLine(control.progress(), appUnit()) : '')}</span></p>` : '';
      const loading = s === 'loading' ? (failure
        ? `<p class="tour-fail" role="alert">${icon('alert')}<span>${esc(failure)}</span></p>`
        : `<p class="tour-live">${'<span class="tour-spinner" aria-hidden="true"></span>'}<span>${esc(loadingText)}</span></p><div class="tour-bar" aria-hidden="true"><span></span></div>`) : '';
      const nextLabel = s === 'loading' ? '' : `<button type="button" class="primary tour-next" data-tour="next" id="tour-next">${esc(c.next)}${icon('arrow-right')}</button>`;
      box.innerHTML = `${head}<h2 id="tour-title" tabindex="-1">${esc(c.title)}</h2><p class="tour-text" id="tour-text">${esc(running ? t('marker_text') : c.text)}</p>${live}${loading}
        <div class="tour-actions">${s === 'loading' && failure ? `<button type="button" class="primary" data-tour="close">${tx('close')}</button>` : nextLabel}</div>
        <p class="sr-only" role="status" aria-live="polite" id="tour-say">${esc(`${c.number ? `${t('step_of', { n: c.number, total: c.total })}. ` : ''}${c.title}. ${running ? t('marker_text') : c.text}`)}</p>`;
    }
    if (focusTitle) window.requestAnimationFrame(() => {
      // Keep keyboard focus on Next between steps (the button is re-created); otherwise move it to the card title.
      const nextButton = byId('tour-next'), active = document.activeElement;
      if (active === document.body || !active || box.contains(active) || !active.isConnected) (nextButton ?? byId('tour-title'))?.focus({ preventScroll: true });
    });
  }
  function renderLive(): void {
    if (step !== 'marker') return;
    const p = control.progress(), el = byId('tour-live');
    if (p.bestLengthMm != null && times.first_live === undefined && p.running) mark('first_live');
    if (!p.running) return;
    const span = el?.querySelector('span:last-child'); if (span) span.textContent = liveLine(p, appUnit());
    const nextButton = byId('tour-next'); if (nextButton && p.bestLengthMm == null) nextButton.setAttribute('aria-disabled', 'true'); else nextButton?.removeAttribute('aria-disabled');
  }
  function endHtml(facts: ReturnType<StyleControl['facts']>): string {
    const unit = appUnit(), len = facts?.lengthMm != null ? (unit === 'inch' ? `${(facts.lengthMm / 914.4).toFixed(2)} yd` : `${(facts.lengthMm / 1000).toFixed(2)} m`) : '';
    const chips = facts ? [t('summary_sizes', { n: facts.sizes.length }), len && t('summary_marker', { length: len }), facts.efficiency != null && t('summary_used', { e: `${(facts.efficiency * 100).toFixed(1)}%` }), facts.garmentsInLay != null && t('summary_garments', { n: facts.garmentsInLay })].filter(Boolean) as string[] : [];
    const choice = (key: Choice, iconName: string, title: TourKey, text: TourKey) => `<li><button type="button" class="tour-choice" data-tour-choice="${key}"><span class="icon-tile">${icon(iconName)}</span><span><strong>${tx(title)}</strong><small>${tx(text)}</small></span>${icon('arrow-right', 'action-arrow')}</button></li>`;
    const seconds = times.end ?? Math.round(performance.now() - started) / 1000;
    return `<h2 id="tour-title" tabindex="-1">${icon('check', 'tour-end-icon')}${tx('end_title')}</h2>
      <p class="tour-text">${tx('end_text')} <span class="tour-time">${tx('end_time', { s: Math.round(seconds) })}</span></p>
      ${chips.length ? `<ul class="tour-chips">${chips.map(c => `<li>${esc(c)}</li>`).join('')}</ul>` : ''}
      <h3 class="tour-try">${tx('try_title')}</h3>
      <ul class="tour-choices">
        ${choice('style', 'style', 'try_style', 'try_style_text')}
        ${choice('photo', 'photo-marker', 'try_photo', 'try_photo_text')}
        ${choice('dxf', 'open', 'try_dxf', 'try_dxf_text')}
        ${hadStyle ? '' : choice('keep', 'pattern', 'try_keep', 'try_keep_text')}
      </ul>
      <div class="tour-foot"><button type="button" class="ghost small" data-tour="feedback">${icon('chat')}${tx('tell_us')}</button>${isDesktop() ? '' : `<a class="ghost small tour-link" href="../#download">${icon('download')}${tx('desktop')}</a>`}<button type="button" class="small" data-tour="close" id="tour-close">${tx('close')}</button></div>
      <p class="sr-only" role="status" aria-live="polite">${tx('end_title')}. ${tx('end_text')}</p>`;
  }

  box.addEventListener('click', event => {
    const el = (event.target as Element).closest<HTMLElement>('[data-tour], [data-tour-choice]'); if (!el) return;
    const choice = el.dataset.tourChoice as Choice | undefined;
    if (choice) { void finish(choice); return; }
    switch (el.dataset.tour) {
      case 'next': if (el.getAttribute('aria-disabled') !== 'true') next(); break;
      case 'skip': void finish('skip'); break;
      case 'close': void finish('close'); break;
      case 'feedback': void finish('close').then(() => find<HTMLButtonElement>('#feedback-button')?.click()); break;
    }
  });
  box.addEventListener('keydown', event => { if (event.key === 'Escape') { event.preventDefault(); void finish(step === 'end' ? 'close' : 'skip'); } });
  // The visitor left the tour (menu, Back button): end it quietly and give back their own style.
  window.addEventListener('beta-screen', event => {
    if (!step || leading) return;
    if ((event as CustomEvent<number>).detail !== host.styleScreen) void finish('left');
  });
  // The marker drawing changes size with the card; the Style screen follows the window, so nothing else to do here.
  return { start, running: () => step !== null };
}
