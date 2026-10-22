import './feedback.css';
import { logo, VERSION } from '../brand';
import { downloadFile, isDesktop } from '../transport';
import { staticMode } from '../static-mode';
import type { DocumentState } from '../state';
import { FEEDBACK_CONFIG } from './config';
import { openFeedbackLink } from './external';
import { currentLanguage, t, translateFeedback, type FeedbackKey } from './i18n';
import { isMilestone, rememberAsk, rememberWelcome, shouldAsk, shouldShowWelcome, type KeyStore } from './memory';
import { buildReport, detectOs, issueUrl, languageName, mailtoUrl, platformName, privacyHints, reportFileName,
  type FeedbackContext, type FeedbackKind, type MarkerNumbers, type Platform, type Report } from './report';
import { byId, find, findAll } from '../screens';

/** Feedback everywhere, a humble first-run welcome and one gentle "How did that go?" ask.
 * Self-contained: the shell calls installFeedback() once. Nothing is sent without the user's choice.
 * Desktop links use the scoped official Tauri opener. Copy-link remains the fallback. */
export interface FeedbackHost { state?: () => DocumentState; message?: (text: string, error?: boolean) => void }
export interface FeedbackApi { open(kind?: FeedbackKind): void; showWelcome(): void; askOnce(): void }

/** English screen names for reports. Indexes are fixed by the shell. */
const SCREEN_NAMES: Record<number, string> = { 0: 'Marker editor', 1: 'Cut plan', 2: 'Compare', 3: 'Help', 4: 'Pattern formulas', 5: 'Digitize', 6: 'Rolls', 7: 'Bundles', 8: 'Floor', 9: 'Home', 10: 'Reports', 11: 'Photo to marker', 12: 'Make a style' };
/** Screens where anonymous marker numbers make sense. */
const MARKER_SCREENS = new Set([0, 1, 2]);
const KINDS: FeedbackKind[] = ['bug', 'missing', 'idea'];
const KIND_ICON: Record<FeedbackKind, string> = {
  bug: '<path d="M9 7.5a3 3 0 0 1 6 0"/><rect x="7" y="8" width="10" height="11" rx="5"/><path d="M12 11v8M3 13h4M17 13h4M4 8l3 2M20 8l-3 2M4 19l3-2M20 19l-3-2"/>',
  missing: '<rect x="4" y="4" width="16" height="16" rx="2" stroke-dasharray="3 2.4"/><path d="M12 8v8M8 12h8"/>',
  idea: '<path d="M9 18h6M10 21h4"/><path d="M12 3a6 6 0 0 0-3.5 10.9c.6.5 1 1.2 1 2.1h5c0-.9.4-1.6 1-2.1A6 6 0 0 0 12 3z"/>',
};
const svg = (paths: string): string => `<svg class="icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">${paths}</svg>`;
const BUBBLE = '<path d="M4 5h16v11H10l-5 4v-4H4z"/><path d="M8 9.5h8M8 12.5h5"/>';
const esc = (v: unknown): string => String(v ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]!));
const label = (key: FeedbackKey, tag = 'span', extra = ''): string => `<${tag} data-fb="${key}"${extra}>${esc(t(key))}</${tag}>`;

function storage(): KeyStore | null { try { return window.localStorage; } catch { return null; } }
/** Test browsers skip the welcome and the ask, so other agents' screen checks are not covered by them.
 * Add ?welcome or ?feedback-ask to the address to see them in a test browser. */
const automated = (flag: string): boolean => navigator.webdriver === true && !new RegExp(`[?&]${flag}\\b`).test(location.search);

async function copyText(text: string): Promise<boolean> {
  try { await navigator.clipboard.writeText(text); return true; } catch { /* Fall back below. */ }
  const area = document.createElement('textarea'); area.value = text; area.setAttribute('readonly', ''); area.className = 'fb-offscreen';
  // Inside an open modal dialog the rest of the page is inert, so the helper must live in the dialog.
  (find('dialog[open]') ?? document.body).append(area); area.select();
  let ok = false; try { ok = document.execCommand('copy'); } catch { ok = false; }
  area.remove(); return ok;
}

/** Keep Tab inside an open feedback dialog. Native <dialog> makes the page inert; this also stops Tab leaving to the browser bar. */
function trapFocus(dialog: HTMLDialogElement): void {
  dialog.addEventListener('keydown', event => {
    if (event.key !== 'Tab') return;
    const items = Array.from(dialog.querySelectorAll<HTMLElement>('a[href], button, input, textarea, select, [tabindex]:not([tabindex="-1"])'))
      .filter(e => !e.hasAttribute('disabled') && !e.closest('[hidden]') && e.getClientRects().length > 0);
    if (!items.length) return;
    const first = items[0]!, last = items[items.length - 1]!;
    if (event.shiftKey && document.activeElement === first) { event.preventDefault(); last.focus(); }
    else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first.focus(); }
  });
}

/** Browser navigation stays unchanged; desktop buttons use the scoped opener. */
function externalLink(url: string, key: FeedbackKey): string {
  return isDesktop()
    ? `<button type="button" class="small" data-fb-external="${esc(url)}">${label(key)}</button>`
    : `<a class="button small fb-link" href="${esc(url)}" target="_blank" rel="noopener noreferrer">${label(key)} <span class="fb-offscreen" data-fb="new_tab">${esc(t('new_tab'))}</span></a>`;
}

export function installFeedback(host: FeedbackHost = {}): FeedbackApi {
  let screen = 9, opener: HTMLElement | null = null, kind: FeedbackKind = 'bug';
  let report: Report | null = null;

  // ----- Top bar button (before About; the shell keeps it on every screen) -----
  const tools = find<HTMLElement>('.topbar-tools') ?? find<HTMLElement>('header');
  const topButton = document.createElement('button');
  topButton.type = 'button'; topButton.id = 'feedback-button'; topButton.className = 'small fb-topbar-button';
  topButton.setAttribute('aria-haspopup', 'dialog'); topButton.setAttribute('aria-keyshortcuts', 'Shift+?');
  topButton.dataset.fbAttr = 'title:button_title';
  topButton.innerHTML = `${svg(BUBBLE)}${label('button')}`;
  topButton.addEventListener('click', () => open());
  const about = tools?.querySelector('#topbar-extra') ?? tools?.querySelector('#about-button');
  if (about) about.before(topButton); else tools?.append(topButton);

  // ----- Dialogs and the ask card -----
  const kindCards = KINDS.map(k => `<label class="fb-kind"><input type="radio" name="fb-kind" value="${k}"><span class="fb-kind-icon">${svg(KIND_ICON[k])}</span><span class="fb-kind-text">${label(`kind_${k}` as FeedbackKey, 'strong')}${label(`kind_${k}_hint` as FeedbackKey, 'small')}</span></label>`).join('');
  const sendRow = (id: string, key: FeedbackKey, hint: FeedbackKey, primary = false) =>
    `<li><button type="button" id="${id}" class="${primary ? 'primary' : ''}" aria-describedby="${id}-hint">${label(key)}</button><span id="${id}-hint" class="hint" data-fb="${hint}">${esc(t(hint))}</span></li>`;
  const desktop = isDesktop();
  document.body.insertAdjacentHTML('beforeend', `
  <dialog id="fb-dialog" class="fb-dialog" aria-labelledby="fb-title" aria-describedby="fb-intro"><div class="dialog-body">
    <div class="fb-head"><h2 id="fb-title" data-fb="title">${esc(t('title'))}</h2><button type="button" class="ghost fb-x" data-fb-close data-fb-attr="aria-label:close" aria-label="${esc(t('close'))}">${svg('<path d="M6 6l12 12M18 6L6 18"/>')}</button></div>
    <p id="fb-intro" class="fb-intro" data-fb="intro">${esc(t('intro'))}</p>
    <form id="fb-form" novalidate>
      <fieldset class="fb-kinds"><legend data-fb="kind_legend">${esc(t('kind_legend'))}</legend>${kindCards}</fieldset>
      <label for="fb-title-input">${label('field_title')}</label><input id="fb-title-input" maxlength="100" autocomplete="off" spellcheck="true">
      <label for="fb-details"><span id="fb-details-label"></span></label><textarea id="fb-details" rows="4" aria-required="true" aria-describedby="fb-error" spellcheck="true"></textarea>
      <label for="fb-steps"><span id="fb-steps-label"></span></label><textarea id="fb-steps" rows="3" spellcheck="true"></textarea>
      <label for="fb-contact">${label('field_contact')}</label><input id="fb-contact" autocomplete="off" aria-describedby="fb-contact-hint"><span id="fb-contact-hint" class="hint" data-fb="contact_hint">${esc(t('contact_hint'))}</span>
      <section class="fb-auto" aria-labelledby="fb-auto-title"><h3 id="fb-auto-title" data-fb="auto_title">${esc(t('auto_title'))}</h3><dl id="fb-auto-list"></dl><p class="hint" data-fb="auto_privacy">${esc(t('auto_privacy'))}</p>
        <label class="check" id="fb-numbers-row" hidden><input type="checkbox" id="fb-numbers">${label('numbers')}</label></section>
      <p id="fb-error" class="fb-error" role="alert"></p>
      <div class="dialog-actions"><button type="button" data-fb-close>${label('cancel')}</button><button type="submit" class="primary">${label('preview_button')}</button></div>
    </form>
    <div id="fb-preview-step" hidden>
      <h3 id="fb-preview-title" tabindex="-1" data-fb="preview_title">${esc(t('preview_title'))}</h3>
      <p class="hint" data-fb="preview_intro">${esc(t('preview_intro'))}</p>
      <div id="fb-privacy" class="notice warn" hidden></div>
      <pre id="fb-preview" class="fb-preview" tabindex="0" aria-labelledby="fb-preview-title"></pre>
      <p id="fb-short-note" class="hint" data-fb="short_note" hidden>${esc(t('short_note'))}</p>
      <h3 id="fb-send-title" data-fb="send_title">${esc(t('send_title'))}</h3>
      <ul class="fb-send" aria-labelledby="fb-send-title">
        ${sendRow('fb-github', 'send_github', 'send_github_hint', true)}
        ${sendRow('fb-email', 'send_email', 'send_email_hint')}
        ${sendRow('fb-copy', 'send_copy', 'send_copy_hint')}
        ${sendRow('fb-save', 'send_save', 'send_save_hint')}
      </ul>
      ${desktop ? `<p class="hint" data-fb="desktop_note">${esc(t('desktop_note'))}</p><p class="hint">${esc(FEEDBACK_CONFIG.email)} · ${esc(FEEDBACK_CONFIG.repository)}</p>` : ''}
      <div class="dialog-actions"><button type="button" id="fb-back">${label('back')}</button><button type="button" data-fb-close>${label('close')}</button></div>
    </div>
    <p id="fb-status" class="fb-status" role="status" aria-live="polite"></p>
  </div></dialog>
  <dialog id="fb-welcome" class="fb-welcome" aria-labelledby="fb-welcome-title" aria-describedby="fb-welcome-text"><div class="dialog-body">
    <div class="fb-welcome-head">${logo(52)}<div>${label('welcome_eyebrow', 'span', ' class="eyebrow"')}<h2 id="fb-welcome-title" data-fb="welcome_title">${esc(t('welcome_title'))}</h2></div></div>
    <div id="fb-welcome-text" class="fb-welcome-text">${label('welcome_p1', 'p')}${label('welcome_p2', 'p', ' class="fb-strong"')}${label('welcome_p3', 'p')}</div>
    <ul class="fb-points"><li>${svg(BUBBLE)}${label('welcome_point1')}</li><li>${svg('<rect x="5" y="10" width="14" height="10" rx="2"/><path d="M8 10V7a4 4 0 0 1 8 0v3"/>')}${label('welcome_point2')}</li></ul>
    <div class="dialog-actions"><button type="button" data-fb-welcome="feedback">${label('welcome_feedback')}</button><button type="button" class="primary" data-fb-welcome="start">${label('welcome_start')}</button></div>
  </div></dialog>
  <section id="fb-ask" class="fb-ask" aria-labelledby="fb-ask-title" hidden>
    <h2 id="fb-ask-title" data-fb="ask_title">${esc(t('ask_title'))}</h2><p data-fb="ask_text">${esc(t('ask_text'))}</p>
    <div class="fb-ask-actions"><button type="button" class="primary" data-fb-ask="send">${label('ask_send')}</button><button type="button" data-fb-ask="later">${label('ask_later')}</button></div>
  </section>
  <p id="fb-announce" class="fb-offscreen" role="status" aria-live="polite"></p>`);

  const $ = <T extends HTMLElement = HTMLElement>(id: string): T => byId(id) as T;
  const dialog = $<HTMLDialogElement>('fb-dialog'), welcome = $<HTMLDialogElement>('fb-welcome'), ask = $('fb-ask');
  const form = $<HTMLFormElement>('fb-form'), previewStep = $('fb-preview-step'), status = $('fb-status');
  const announce = (text: string) => { const el = $('fb-announce'); el.textContent = ''; window.setTimeout(() => { el.textContent = text; }, 50); };
  trapFocus(dialog); trapFocus(welcome);

  // ----- Help screen and About: a humble line and the same three actions -----
  const helpActions = `<div class="toolgroup fb-actions"><button type="button" class="primary small" data-fb-open>${label('title')}</button>${externalLink(FEEDBACK_CONFIG.discussions, 'discussion')}<button type="button" class="small" data-fb-show-welcome>${label('show_welcome')}</button></div>`;
  const help = find<HTMLElement>('#screen-3');
  if (help) {
    const box = document.createElement('section'); box.className = 'fb-help'; box.setAttribute('aria-labelledby', 'fb-help-title');
    box.innerHTML = `<h2 id="fb-help-title" data-fb="help_title">${esc(t('help_title'))}</h2>${label('humble', 'p')}${helpActions}`;
    const intro = help.querySelector('h1 + p') ?? help.querySelector('h1');
    if (intro) intro.after(box); else help.prepend(box);
  }
  const aboutBody = find<HTMLElement>('#about-dialog .dialog-body');
  if (aboutBody) {
    const box = document.createElement('div'); box.className = 'fb-about';
    box.innerHTML = `${label('humble', 'p', ' class="fb-strong"')}${helpActions.replace('primary small', 'small')}`;
    const actions = aboutBody.querySelector('.dialog-actions');
    if (actions) actions.before(box); else aboutBody.append(box);
  }

  // ----- Context shown to the user and put in the report -----
  function screenName(): string {
    const base = SCREEN_NAMES[screen] ?? 'Unknown';
    const page = screen === 8 ? find<HTMLElement>('[data-floor-page][aria-pressed=true]')?.dataset.floorPage : screen === 12 ? find<HTMLElement>('#screen-12')?.dataset.step : '';
    return page && /^[a-z_-]{1,30}$/.test(page) ? `${base} › ${page}` : base;
  }
  function context(): FeedbackContext {
    // "connected" is also set when the Rust engine runs in this browser; only a real server answer means "server".
    const status = find<HTMLElement>('#server-status');
    const platform: Platform = isDesktop() ? 'desktop' : staticMode() || status?.dataset.floorLabel === 'status_browser' || !status?.classList.contains('connected') ? 'browser' : 'server';
    const ua = navigator as Navigator & { userAgentData?: { platform?: string } };
    return { screen: screenName(), version: VERSION, platform, os: detectOs(navigator.userAgent, ua.userAgentData?.platform ?? navigator.platform), language: currentLanguage() };
  }
  function numbers(): MarkerNumbers | null {
    const state = host.state?.(); if (!state) return null;
    const placed = state.result.placements.length;
    const time = Number(find<HTMLInputElement>('#time')?.value);
    return { pieces: placed || state.job.pieces.length, efficiencyPct: placed ? state.result.efficiency * 100 : null, timeLimitS: Number.isFinite(time) && time > 0 ? time : null };
  }
  function renderContext(): void {
    const c = context();
    const rows: Array<[FeedbackKey, string]> = [['auto_screen', c.screen], ['auto_version', c.version], ['auto_platform', platformName(c.platform)], ['auto_os', c.os], ['auto_language', languageName(c.language)]];
    $('fb-auto-list').innerHTML = rows.map(([k, v]) => `<div><dt data-fb="${k}">${esc(t(k))}</dt><dd>${esc(v)}</dd></div>`).join('');
    $('fb-numbers-row').hidden = !MARKER_SCREENS.has(screen) || !host.state;
  }
  function setKind(next: FeedbackKind): void {
    kind = next;
    form.querySelectorAll<HTMLInputElement>('[name=fb-kind]').forEach(r => { r.checked = r.value === next; });
    $('fb-details-label').dataset.fb = `details_${next}`; $('fb-steps-label').dataset.fb = `steps_${next}`;
    translateFeedback(form);
  }

  // ----- Open, preview, send -----
  function open(next: FeedbackKind = kind): void {
    if (dialog.open) return;
    if (welcome.open) welcome.close();
    find<HTMLDialogElement>('#about-dialog[open]')?.close();
    hideAsk(false);
    opener = document.activeElement instanceof HTMLElement && document.activeElement !== document.body ? document.activeElement : topButton;
    setKind(next); renderContext(); showForm(); status.textContent = ''; $('fb-error').textContent = '';
    translateFeedback(dialog);
    dialog.showModal();
    form.querySelector<HTMLInputElement>('[name=fb-kind]:checked')?.focus();
  }
  function showForm(): void { form.hidden = false; previewStep.hidden = true; }
  form.addEventListener('change', event => { const r = event.target as HTMLInputElement; if (r.name === 'fb-kind') setKind(r.value as FeedbackKind); });
  form.addEventListener('submit', event => {
    event.preventDefault();
    const details = $<HTMLTextAreaElement>('fb-details'), title = $<HTMLInputElement>('fb-title-input');
    if (!details.value.trim() && !title.value.trim()) {
      details.setAttribute('aria-invalid', 'true'); $('fb-error').textContent = t('error_details'); details.focus(); return;
    }
    details.removeAttribute('aria-invalid'); $('fb-error').textContent = '';
    const input = { kind, title: title.value, details: details.value, steps: $<HTMLTextAreaElement>('fb-steps').value, contact: $<HTMLInputElement>('fb-contact').value };
    const withNumbers = !$('fb-numbers-row').hidden && $<HTMLInputElement>('fb-numbers').checked;
    report = buildReport(input, context(), withNumbers ? numbers() : null);
    $('fb-preview').textContent = report.text;
    const hints = privacyHints(input.title, input.details, input.steps, input.contact);
    $('fb-privacy').hidden = !hints.length;
    $('fb-privacy').innerHTML = hints.map(h => `<p data-fb="privacy_${h}">${esc(t(`privacy_${h}` as FeedbackKey))}</p>`).join('');
    $('fb-short-note').hidden = !issueUrl(report, kind).truncated;
    form.hidden = true; previewStep.hidden = false; status.textContent = '';
    $('fb-preview-title').focus();
  });
  $('fb-back').addEventListener('click', () => { showForm(); status.textContent = ''; $<HTMLTextAreaElement>('fb-details').focus(); });
  const done = (key: FeedbackKey) => { status.textContent = t(key); rememberAsk(storage(), VERSION); };
  const fail = (key: FeedbackKey) => { status.textContent = t(key); };

  $('fb-github').addEventListener('click', async () => {
    if (!report) return;
    const link = issueUrl(report, kind);
    if (link.truncated) await copyText(report.text);
    if (desktop) {
      const outcome = await openFeedbackLink(link.url, copyText);
      if (outcome === 'opened') done(link.truncated ? 'done_github_long' : 'done_github');
      else if (outcome === 'copied') done('done_link'); else fail('copy_failed');
      return;
    }
    window.open(link.url, '_blank', 'noopener,noreferrer');
    done(link.truncated ? 'done_github_long' : 'done_github');
  });
  $('fb-email').addEventListener('click', async () => {
    if (!report) return;
    const link = mailtoUrl(report);
    if (link.truncated) await copyText(report.text);
    if (desktop) {
      const outcome = await openFeedbackLink(link.url, copyText);
      if (outcome === 'opened') done('done_email');
      else if (outcome === 'copied') done('done_link'); else fail('copy_failed');
      return;
    }
    const a = document.createElement('a'); a.href = link.url; a.rel = 'noopener'; a.click();
    done('done_email');
  });
  $('fb-copy').addEventListener('click', async () => { if (report) (await copyText(report.text)) ? done('done_copy') : fail('copy_failed'); });
  $('fb-save').addEventListener('click', async () => {
    if (!report) return;
    try { const saved = await downloadFile(new Blob([report.text], { type: 'text/markdown;charset=utf-8' }), reportFileName(kind)); if (saved) done('done_save'); }
    catch { fail('save_failed'); }
  });

  dialog.addEventListener('close', () => { (opener?.isConnected && opener.getClientRects().length ? opener : topButton).focus(); opener = null; });
  document.addEventListener('click', async event => {
    const target = event.target as Element;
    if (target.closest('[data-fb-close]')) { target.closest('dialog')?.close(); return; }
    const openButton = target.closest<HTMLElement>('[data-fb-open]');
    if (openButton) { open((openButton.dataset.fbOpen || undefined) as FeedbackKind | undefined); return; }
    if (target.closest('[data-fb-show-welcome]')) { find<HTMLDialogElement>('#about-dialog[open]')?.close(); showWelcome(); return; }
    const external = target.closest<HTMLElement>('[data-fb-external]');
    if (external) {
      const outcome = await openFeedbackLink(external.dataset.fbExternal!, copyText);
      const ok = outcome === 'opened' || outcome === 'copied';
      const text = t(outcome === 'opened' ? 'link_opened' : outcome === 'copied' ? 'link_copied' : 'copy_failed');
      announce(text); host.message?.(text, !ok);
    }
  });

  // ----- "Report a problem with this screen": one link under each screen's main heading -----
  function screenLink(): void {
    const section = screen === 0 ? find<HTMLElement>('.workspace') : byId(`screen-${screen}`);
    if (!section || section.querySelector('.fb-screen-link')) return;
    const heading = (screen === 0 ? section.querySelector('.main-heading h2') : null) ?? section.querySelector('h1') ?? section.querySelector('h2');
    const link = document.createElement('button');
    link.type = 'button'; link.className = 'fb-screen-link no-print'; link.dataset.fbOpen = 'bug';
    link.innerHTML = `${svg(BUBBLE)}${label('screen_link')}`;
    if (heading) heading.after(link); else section.prepend(link);
  }
  window.addEventListener('beta-screen', event => { screen = (event as CustomEvent<number>).detail; screenLink(); });

  // ----- Keyboard: Shift+? outside text fields and drawing canvases (the canvases use ? for their own help) -----
  document.addEventListener('keydown', event => {
    if (event.key !== '?' || event.ctrlKey || event.altKey || event.metaKey || event.defaultPrevented) return;
    const target = event.target as Element | null;
    if (target?.closest('input, textarea, select, [contenteditable=""], [contenteditable=true], svg, canvas, [role=application], dialog[open]')) return;
    event.preventDefault(); open();
  });

  // ----- Welcome on first start -----
  function showWelcome(): void {
    if (welcome.open || dialog.open) return;
    translateFeedback(welcome);
    welcome.showModal();
    welcome.querySelector<HTMLButtonElement>('[data-fb-welcome="start"]')?.focus();
  }
  welcome.addEventListener('close', () => rememberWelcome(storage()));
  welcome.addEventListener('click', event => {
    const choice = (event.target as Element).closest<HTMLElement>('[data-fb-welcome]')?.dataset.fbWelcome;
    if (!choice) return;
    welcome.close();
    if (choice === 'feedback') open('idea');
  });

  // ----- One gentle ask after the first successful nest or export, at most once per version -----
  function hideAsk(remember: boolean): void { if (ask.hidden) return; ask.hidden = true; if (remember) rememberAsk(storage(), VERSION); }
  function askOnce(): void {
    if (!shouldAsk(storage(), VERSION, automated('feedback-ask')) || dialog.open || welcome.open || !ask.hidden) return;
    translateFeedback(ask); ask.hidden = false; rememberAsk(storage(), VERSION);
    announce(`${t('ask_title')} ${t('ask_text')}`);
  }
  ask.addEventListener('click', event => {
    const choice = (event.target as Element).closest<HTMLElement>('[data-fb-ask]')?.dataset.fbAsk;
    if (choice === 'send') { hideAsk(true); open('idea'); } else if (choice === 'later') { hideAsk(true); topButton.focus(); }
  });
  ask.addEventListener('keydown', event => { if (event.key === 'Escape') { hideAsk(true); topButton.focus(); } });
  const messageEl = find<HTMLElement>('#message');
  if (messageEl) new MutationObserver(() => { if (isMilestone(messageEl.textContent ?? '')) askOnce(); }).observe(messageEl, { childList: true, characterData: true, subtree: true });
  window.addEventListener('beta-feedback-milestone', askOnce);

  // ----- Language: follow the app switch on <html data-language> -----
  new MutationObserver(() => {
    for (const root of [topButton, dialog, welcome, ask, ...Array.from(findAll<HTMLElement>('.fb-help, .fb-about, .fb-screen-link'))]) translateFeedback(root);
    if (dialog.open && !form.hidden) renderContext();
  }).observe(document.documentElement, { attributes: true, attributeFilter: ['data-language'] });
  translateFeedback(topButton);

  if (shouldShowWelcome(storage(), automated('welcome'))) requestAnimationFrame(showWelcome);
  return { open, showWelcome, askOnce };
}

