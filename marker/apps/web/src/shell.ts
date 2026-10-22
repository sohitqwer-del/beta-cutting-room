import { icon } from './icons';
import { installUx } from './ux';
import { CREDITS, logo, PRODUCT, VERSION } from './brand';
import { isDesktop, serverToken, setServerToken } from './transport';
import { staticMode } from './static-mode';
import type { DocumentState } from './state';
import { installFeedback } from './feedback';
import { installQuick, QUICK_SCREEN } from './quick/ui';
import { installStyle, STYLE_SCREEN } from './style/ui';
import { t as styleText } from './style/i18n';
import { find, findAll, screenTitle } from './screens';
import { appUnit, onUnit, setAppUnit, type AppUnit } from './units-pref';
import { label, type LabelKey } from './floor-i18n';
import type { FloorLanguage } from './floor-types';

/** App frame: sidebar navigation (full or icon rail), top bar, workflow steps with "Next", Home, Reports, About,
 * theme, language and server token. It moves the existing screens into one layout and never changes their behaviour. */
export interface ShellHost {
  show(index: number): void;
  message(text: string, error?: boolean): void;
  recheck(): void;
  state(): DocumentState;
}

const esc = (v: unknown): string => String(v ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]!));

interface Screen { index: number; key: string; title: string; group: '' | 'more'; icon: string; hash: string; titleKey?: string }
/** Screen indexes are fixed by existing modules: 0 editor, 1 plan, 2 compare, 3 help, 4 pattern, 5 digitize, 6 rolls, 7 bundles, 8 floor, 11 quick marker, 12 style flow.
 * The menu shows the five jobs people come for; the rest sits under "More tools". Addresses (#/marker, #/pattern …) never change. */
export const SCREENS: Screen[] = [
  { index: 9, key: 'home', title: 'Home', group: '', icon: 'home', hash: 'home' },
  { index: STYLE_SCREEN, key: 'make_style', title: 'Make a style', group: '', icon: 'style', hash: 'style' },
  { index: QUICK_SCREEN, key: 'quick', title: 'Photo to marker', group: '', icon: 'photo-marker', hash: 'quick' },
  { index: 0, key: 'marker_files', title: 'Marker from files', group: '', icon: 'marker', hash: 'marker', titleKey: 'editor' },
  { index: 8, key: 'cutting_floor', title: 'Cutting floor', group: '', icon: 'floor', hash: 'floor' },
  { index: 1, key: 'cut_plan', title: 'Cut plan', group: 'more', icon: 'plan', hash: 'cut-plan' },
  { index: 6, key: 'rolls', title: 'Rolls', group: 'more', icon: 'rolls', hash: 'rolls' },
  { index: 7, key: 'bundles', title: 'Bundles', group: 'more', icon: 'bundles', hash: 'bundles' },
  { index: 10, key: 'reports', title: 'Reports', group: 'more', icon: 'reports', hash: 'reports' },
  { index: 2, key: 'compare', title: 'Compare', group: 'more', icon: 'compare', hash: 'compare' },
  { index: 4, key: 'pattern_formulas', title: 'Pattern formulas', group: 'more', icon: 'pattern', hash: 'pattern' },
  { index: 5, key: 'digitize', title: 'Digitize', group: 'more', icon: 'digitize', hash: 'digitize' },
  { index: 3, key: 'help', title: 'Help', group: 'more', icon: 'help', hash: 'help' },
];

/** Pattern → marker → plan → rolls → spread → cut → bundles → sewing. Floor steps open a Floor page.
 * Step 1 opens the Style flow (draft to marker); the formula Pattern screen also counts as step 1. */
const STEPS: Array<{ key: string; label: string; screen: number; also?: number[]; floorPage?: string; hint: string; icon: string }> = [
  { key: 'pattern', label: 'Pattern', screen: STYLE_SCREEN, also: [4], hint: 'Draft or import pieces and grade sizes.', icon: 'pattern' },
  { key: 'marker', label: 'Marker', screen: 0, hint: 'Nest pieces across the fabric width.', icon: 'marker' },
  { key: 'cut_plan', label: 'Cut plan', screen: 1, hint: 'Split an order into lays and plies.', icon: 'plan' },
  { key: 'rolls', label: 'Rolls', screen: 6, hint: 'Inspect cloth and group shades.', icon: 'rolls' },
  { key: 'spread', label: 'Spread', screen: 8, floorPage: 'spreading', hint: 'Release a lay with its rolls.', icon: 'spread' },
  { key: 'cut', label: 'Cut', screen: 8, floorPage: 'tracking', hint: 'Record what was cut.', icon: 'cut' },
  { key: 'bundles', label: 'Bundles', screen: 7, hint: 'Number parts and print tickets.', icon: 'bundles' },
  { key: 'sewing_step', label: 'Sewing', screen: 8, floorPage: 'sewing', hint: 'Scan bundles out to lines.', icon: 'sewing' },
];

const RECENT_KEY = 'beta-recent-files', THEME_KEY = 'beta-theme', LAST_KEY = 'beta-last-screen', RAIL_KEY = 'beta-nav-rail';
type Recent = { name: string; kind: string; at: string };
function readRecent(): Recent[] { try { const v = JSON.parse(localStorage.getItem(RECENT_KEY) ?? '[]') as Recent[]; return Array.isArray(v) ? v.slice(0, 8) : []; } catch { return []; } }
function rememberFile(name: string, kind: string): void {
  try { localStorage.setItem(RECENT_KEY, JSON.stringify([{ name, kind, at: new Date().toISOString() }, ...readRecent().filter(r => r.name !== name)].slice(0, 8))); } catch { /* Storage blocked. */ }
}
/** Light is the default. "system" follows the operating system; the CSS only turns dark for data-theme=dark|system. */
function applyTheme(theme: string): void {
  document.documentElement.dataset.theme = theme === 'dark' || theme === 'system' ? theme : 'light';
}

export function installShell(host: ShellHost) {
  const $ = <T extends Element = HTMLElement>(s: string, root?: ParentNode): T => (root ? root.querySelector<T>(s) : find<T>(s))!;
  const app = $<HTMLDivElement>('#app');
  let theme = 'light';
  try { theme = localStorage.getItem(THEME_KEY) ?? 'light'; } catch { /* default */ }
  applyTheme(theme);
  let current = 0;
  /** Set once the first screen is shown; `fromAddress` while a screen opens because the address changed. */
  let started = false, fromAddress = false;

  // ----- Frame -----
  const header = $<HTMLElement>('header');
  const status = $<HTMLElement>('#server-status');
  const nav = $<HTMLElement>('.screen-tabs');
  const filebar = $<HTMLElement>('#editor-filebar');
  const messageEl = $<HTMLElement>('#message');
  const sections = [$<HTMLElement>('.workspace'), ...Array.from(findAll<HTMLElement>('.screen'))];

  app.classList.add('app-shell');
  const skip = document.createElement('a'); skip.href = '#content'; skip.className = 'skip-link'; skip.textContent = 'Skip to main content';
  const sidebar = document.createElement('div'); sidebar.className = 'sidebar'; sidebar.id = 'sidebar';
  sidebar.innerHTML = `<button type="button" class="brand" data-go="9" aria-label="${PRODUCT} home">${logo(34)}<span class="brand-text"><strong>${PRODUCT}</strong><small>Pattern to sewing line</small></span></button>`;
  const appMain = document.createElement('div'); appMain.className = 'app-main';
  const content = document.createElement('main'); content.id = 'content'; content.tabIndex = -1;
  content.innerHTML = `<div class="print-header" aria-hidden="true"><span>${PRODUCT} ${esc(VERSION)}</span><span id="print-title"></span><span id="print-date"></span></div>`;
  app.prepend(skip, sidebar, appMain);
  appMain.append(header);

  // Top bar, one slim row: title · status (with what works where) · language · units · feedback · "⋯" (theme, About).
  header.innerHTML = `<button type="button" class="menu-toggle icon-btn ghost" id="menu-toggle" aria-controls="sidebar" aria-expanded="false" aria-label="Show navigation">${icon('menu')}</button>
    <div class="topbar-title" id="topbar-heading"><span class="eyebrow" id="topbar-group"></span><strong id="topbar-title">Home</strong></div>
    <details class="menu status-menu" id="status-menu"><summary aria-label="Where BETA runs and what works here"></summary><div class="menu-panel status-panel" id="status-panel" role="group" aria-label="Where BETA runs">
      <div class="status-where" id="status-where"></div>
      ${isDesktop() ? '' : `<button type="button" id="token-button" class="small" aria-haspopup="dialog">${icon('key')}<span id="token-label">Server token</span></button>`}</div></details>
    <div class="topbar-tools" id="topbar-tools" role="group" aria-label="App settings"></div>
    <button type="button" class="topbar-more icon-btn ghost" id="topbar-more" aria-controls="topbar-tools" aria-expanded="false" aria-label="Theme, About and more settings" data-tip="Theme and About" data-tip-align="end">${icon('more')}</button>`;
  const tools = $<HTMLElement>('.topbar-tools', header);
  // The status text (set by main.ts) is the summary of a small popover; the old "Browser edition" banner lives inside it.
  $('#status-menu > summary', header).append(status);
  // The feedback module adds its button before #topbar-extra (theme and About open from the "⋯" button).
  tools.insertAdjacentHTML('beforeend', `<label class="tool-select">${icon('globe')}<span class="label-text" data-floor-label="language">Language</span><select id="app-language" aria-label="Language"><option value="en">English</option><option value="hi" lang="hi">हिन्दी · draft</option><option value="ta" lang="ta">தமிழ் · draft</option></select></label>
    <div class="segmented unit-switch" role="group" aria-labelledby="unit-switch-label"><span class="sr-only" id="unit-switch-label" data-floor-label="units">Units</span><button type="button" data-app-unit="cm" aria-pressed="false">cm</button><button type="button" data-app-unit="inch" aria-pressed="false">inch</button></div>
    <div class="topbar-extra" id="topbar-extra" role="group" aria-label="Theme and About">
      <label class="tool-select menu-select">${icon('sun')}<span class="menu-select-text" data-floor-label="theme">Theme</span><select id="app-theme" aria-label="Colour theme"><option value="light">Light</option><option value="dark">Dark</option><option value="system">Same as computer</option></select></label>
      <button type="button" id="about-button" aria-haspopup="dialog" aria-label="About ${PRODUCT}">${icon('info')}<span data-floor-label="about">About</span></button></div>`);

  // Cutting-room screens: one row with this screen's file commands (editor) and the workflow steps with "Next".
  const workbar = document.createElement('div'); workbar.className = 'workbar';
  const workflow = document.createElement('nav'); workflow.className = 'workflow'; workflow.setAttribute('aria-label', 'Cutting room workflow');
  workflow.innerHTML = `<span class="workflow-label" data-floor-label="workflow">Workflow</span><ol>${STEPS.map((s, i) => `<li><button type="button" data-step="${i}" aria-description="${esc(s.hint)}" title="${esc(s.label)}"><span class="step-no" aria-hidden="true">${i + 1}</span><span class="step-name" data-floor-label="${s.key}">${esc(s.label)}</span></button></li>`).join('')}</ol>
    <button type="button" class="workflow-next" id="workflow-next" hidden><span class="next-word" data-floor-label="next">Next</span><span class="next-name"></span>${icon('arrow-right')}</button>`;
  header.after(workbar);
  workbar.append(filebar, workflow);
  // Step names that do not fit show as numbers (the name stays in the tooltip and for screen readers).
  const fitSteps = () => { const list = workflow.querySelector('ol')!; if (workbar.hidden || !workbar.isConnected) return; workflow.classList.remove('compact'); if (list.scrollWidth > list.clientWidth + 2) workflow.classList.add('compact'); };
  new ResizeObserver(() => fitSteps()).observe(workbar);
  appMain.append(content);
  content.append(...sections);
  const toasts = document.createElement('div'); toasts.className = 'toast-region';
  toasts.append(messageEl);
  toasts.insertAdjacentHTML('beforeend', '<button type="button" class="toast-close" aria-label="Dismiss message">Dismiss</button>');
  app.append(toasts);

  // ----- Sidebar navigation (existing [data-screen] buttons, regrouped, with icons) -----
  // Five jobs people come for, then "More tools" (folded) for everything else. Every screen and address still works.
  const existing = new Map(Array.from(nav.querySelectorAll<HTMLButtonElement>('[data-screen]'), b => [Number(b.dataset.screen), b]));
  sidebar.append(nav); nav.textContent = '';
  nav.insertAdjacentHTML('beforeend', `<div class="nav-main" role="group" aria-label="Main jobs"></div>
    <button type="button" class="nav-more-toggle" id="nav-more-toggle" aria-expanded="false" aria-controls="nav-more">${icon('more')}<span class="nav-label" data-floor-label="more_tools">More tools</span>${icon('chevron-down', 'nav-chevron')}</button>
    <div class="nav-more" id="nav-more" role="group" aria-label="More tools" hidden></div>`);
  const navMain = $<HTMLElement>('.nav-main', nav), navMore = $<HTMLElement>('#nav-more', nav), moreToggle = $<HTMLButtonElement>('#nav-more-toggle', nav);
  for (const s of SCREENS) {
    let button = existing.get(s.index);
    if (!button) { button = document.createElement('button'); button.type = 'button'; button.dataset.screen = String(s.index); button.setAttribute('aria-pressed', 'false'); button.setAttribute('aria-controls', `screen-${s.index}`); button.onclick = () => host.show(s.index); }
    button.innerHTML = `${icon(s.icon)}<span class="nav-label">${esc(s.title)}</span>`;
    (s.group === 'more' ? navMore : navMain).append(button);
  }
  const MORE_KEY = 'beta-nav-more';
  function setMore(open: boolean, remember = true): void {
    navMore.hidden = !open; moreToggle.setAttribute('aria-expanded', String(open));
    if (remember) try { localStorage.setItem(MORE_KEY, open ? 'open' : 'closed'); } catch { /* session only */ }
  }
  try { setMore(localStorage.getItem(MORE_KEY) === 'open', false); } catch { setMore(false, false); }
  moreToggle.addEventListener('click', () => setMore(navMore.hidden));
  nav.addEventListener('keydown', event => {
    if (!['ArrowUp', 'ArrowDown'].includes(event.key)) return;
    const buttons = Array.from(nav.querySelectorAll<HTMLButtonElement>('[data-screen], #nav-more-toggle')).filter(b => !b.closest('[hidden]')), at = buttons.indexOf(event.target as HTMLButtonElement);
    if (at < 0) return; event.preventDefault();
    const next = buttons[(at + (event.key === 'ArrowDown' ? 1 : -1) + buttons.length) % buttons.length]!; next.focus(); if (next.dataset.screen) next.click();
  });
  sidebar.insertAdjacentHTML('beforeend', `<div class="sidebar-foot"><p class="sidebar-promise">Open source · runs on your own PC or server · your data stays in your factory.</p><p class="sidebar-version">Version ${esc(VERSION)} · Apache-2.0</p>
    <button type="button" id="nav-rail-toggle" class="rail-toggle" aria-controls="sidebar">${icon('chevrons-left')}<span class="nav-label">Collapse menu</span></button></div>`);

  // Rail: icons only, labels as tooltips. Default below 1400 px wide so the work area gets the space.
  const railToggle = $<HTMLButtonElement>('#nav-rail-toggle');
  function setRail(rail: boolean, remember: boolean): void {
    app.classList.toggle('nav-rail', rail);
    railToggle.setAttribute('aria-expanded', String(!rail));
    railToggle.setAttribute('aria-label', rail ? 'Expand menu' : 'Collapse menu');
    railToggle.innerHTML = `${icon(rail ? 'chevrons-right' : 'chevrons-left')}<span class="nav-label">${rail ? 'Expand menu' : 'Collapse menu'}</span>`;
    if (remember) try { localStorage.setItem(RAIL_KEY, rail ? 'rail' : 'full'); } catch { /* session only */ }
    window.dispatchEvent(new Event('resize'));
  }
  let savedRail: string | null = null;
  try { savedRail = localStorage.getItem(RAIL_KEY); } catch { /* default */ }
  setRail(savedRail ? savedRail === 'rail' : window.innerWidth < 1400, false);
  railToggle.addEventListener('click', () => setRail(!app.classList.contains('nav-rail'), true));

  // ----- Home and Reports screens -----
  content.insertAdjacentHTML('beforeend', homeHtml() + reportsHtml());
  const homeScreen = $<HTMLElement>('#screen-9');
  // Quick Marker: phone flow for small units (self-contained module in src/quick/).
  installQuick({ message: host.message }, content);
  // Style flow: from a new style to a cut-ready marker (self-contained module in src/style/).
  installStyle({ message: host.message, editorState: host.state }, content);

  // ----- Dialogs -----
  document.body.insertAdjacentHTML('beforeend', `
  <dialog id="about-dialog" class="about" aria-labelledby="about-title"><div class="dialog-body">
    <div class="about-head">${logo(56, `${PRODUCT} logo`)}<div><h2 id="about-title">${PRODUCT}</h2><p>Version ${esc(VERSION)} · ${isDesktop() ? 'Desktop app' : staticMode() ? 'Browser edition · runs on this device' : 'Browser app with local server'}</p></div></div>
    <p class="promise">Open source · runs on your own PC or server · your data stays in your factory.</p>
    <p>Pattern, digitizing, marker making, cut planning, roll inspection, spreading, cutting and bundle tracking in one offline app. No account, no cloud service and no tracking. All garment calculations run in Rust on this computer or your own server.</p>
    <dl><dt>Licence</dt><dd>Apache-2.0</dd>${CREDITS.map(([name, licence, use]) => `<dt>${esc(name)}</dt><dd>${esc(licence)} · ${esc(use)}</dd>`).join('')}</dl>
    <p class="hint">Hindi and Tamil labels are machine-translated drafts. Review them with operators before production use. Full third-party notices ship with the desktop installer.</p>
    <div class="dialog-actions"><button type="button" class="primary" data-close>Close</button></div></div></dialog>
  <dialog id="token-dialog" aria-labelledby="token-title"><div class="dialog-body">
    <h2 id="token-title">Server access token</h2>
    <p>This BETA server is protected with an access token. Ask your administrator for it. The token is kept only in this browser tab and is removed when you close the tab.</p>
    <label>Access token<input id="token-input" type="password" autocomplete="off" spellcheck="false"></label>
    <p id="token-state" class="hint"></p>
    <div class="dialog-actions"><button type="button" id="token-clear" class="danger">Forget token</button><button type="button" data-close>Cancel</button><button type="button" id="token-save" class="primary">Save and connect</button></div></div></dialog>
  <dialog id="confirm-dialog" aria-labelledby="confirm-title"><div class="dialog-body"><h2 id="confirm-title">Please confirm</h2><p id="confirm-text"></p>
    <div class="dialog-actions"><button type="button" id="confirm-no">Cancel</button><button type="button" id="confirm-yes" class="primary">Continue</button></div></div></dialog>`);
  findAll<HTMLButtonElement>('dialog [data-close]').forEach(b => b.onclick = () => b.closest('dialog')!.close());

  function confirmAction(text: string, yes: string): Promise<boolean> {
    const dialog = $<HTMLDialogElement>('#confirm-dialog');
    $('#confirm-text').textContent = text; $('#confirm-yes').textContent = yes;
    return new Promise(resolve => {
      const done = (value: boolean) => { dialog.close(); resolve(value); };
      $<HTMLButtonElement>('#confirm-yes').onclick = () => done(true); $<HTMLButtonElement>('#confirm-no').onclick = () => done(false);
      dialog.oncancel = () => resolve(false);
      dialog.showModal(); $<HTMLButtonElement>('#confirm-no').focus();
    });
  }

  // ----- Navigation state -----
  const byIndex = new Map(SCREENS.map(s => [s.index, s]));
  const nextButton = $<HTMLButtonElement>('#workflow-next');
  let nextStep = -1;
  function sync(index: number): void {
    current = index;
    const s = byIndex.get(index);
    renderTitle();
    $('#print-date').textContent = new Date().toLocaleDateString();
    workbar.hidden = index === 9 || index === QUICK_SCREEN || index === STYLE_SCREEN;
    app.dataset.activeScreen = String(index);
    // A screen under "More tools" keeps its group open, so the highlighted button is visible.
    if (s?.group === 'more' && navMore.hidden) setMore(true, false);
    if (index === 9) renderHome();
    const page = find<HTMLElement>('[data-floor-page][aria-pressed=true]')?.dataset.floorPage;
    let active = -1;
    workflow.querySelectorAll<HTMLButtonElement>('[data-step]').forEach(b => {
      const step = STEPS[Number(b.dataset.step)]!;
      const on = (step.screen === index || !!step.also?.includes(index)) && (!step.floorPage || step.floorPage === page);
      if (on) { b.setAttribute('aria-current', 'step'); active = Number(b.dataset.step); } else b.removeAttribute('aria-current');
    });
    // Keep the current step in view when the step list scrolls (narrow screens).
    const list = workflow.querySelector('ol')!, currentStep = workflow.querySelector<HTMLElement>('[aria-current=step]');
    if (currentStep && list.scrollWidth > list.clientWidth) {
      const a = currentStep.getBoundingClientRect(), b = list.getBoundingClientRect();
      list.scrollLeft += a.left + a.width / 2 - (b.left + b.width / 2);
    }
    // "Next" always offers the following step of the workflow.
    nextStep = active >= 0 && active < STEPS.length - 1 ? active + 1 : -1;
    nextButton.hidden = nextStep < 0;
    if (nextStep >= 0) {
      const next = STEPS[nextStep]!, name = workflow.querySelector<HTMLElement>(`[data-step="${nextStep}"] [data-floor-label]`)?.textContent ?? next.label;
      $('.next-name', nextButton).textContent = name;
      nextButton.setAttribute('aria-label', `Next step: ${nextStep + 1} ${name}. ${next.hint}`);
      nextButton.dataset.tip = next.hint; nextButton.dataset.tipAlign = 'end';
    }
    // Each screen change is a browser history entry, so Back and Forward (and the phone back gesture) move between screens.
    if (s && location.hash !== `#/${s.hash}`) { if (started && !fromAddress) history.pushState(null, '', `#/${s.hash}`); else history.replaceState(null, '', `#/${s.hash}`); }
    try { localStorage.setItem(LAST_KEY, String(index)); } catch { /* optional */ }
    app.classList.remove('nav-open'); $('#menu-toggle').setAttribute('aria-expanded', 'false');
    closeTools(false);
    fitSteps();
  }
  /** Top-bar title: the screen's own title when it gives one (the Style flow shows its step), else the menu name. */
  function renderTitle(): void {
    const s = byIndex.get(current), language = (document.documentElement.dataset.language ?? 'en') as FloorLanguage;
    const lang: FloorLanguage = language === 'hi' || language === 'ta' ? language : 'en';
    const navName = nav.querySelector<HTMLElement>(`[data-screen="${current}"] .nav-label`)?.textContent ?? s?.title ?? '';
    const own = screenTitle(current);
    const title = own?.title ?? (s?.titleKey ? label(lang, s.titleKey as LabelKey) : navName);
    const eyebrow = own?.eyebrow ?? (s?.group === 'more' ? label(lang, 'more_tools') : '');
    const el = $('#topbar-title');
    el.textContent = title; $('#topbar-group').textContent = eyebrow;
    // A screen that puts its main heading in the top bar (Style) gets a real heading there, focusable after a step change.
    if (own?.heading) { el.setAttribute('role', 'heading'); el.setAttribute('aria-level', '1'); el.tabIndex = -1; }
    else { el.removeAttribute('role'); el.removeAttribute('aria-level'); el.removeAttribute('tabindex'); }
    $('#print-title').textContent = title;
    document.title = `${title} · ${PRODUCT}`;
  }
  window.addEventListener('beta-title', renderTitle);
  window.addEventListener('beta-screen', event => sync((event as CustomEvent<number>).detail));
  document.addEventListener('click', event => { if ((event.target as Element).closest('[data-floor-page]')) queueMicrotask(() => sync(current)); });
  function go(index: number, floorPage?: string, focus?: string): void {
    host.show(index);
    if (floorPage) { find<HTMLButtonElement>(`[data-floor-page="${floorPage}"]`)?.click(); sync(index); }
    if (focus) { const target = find<HTMLElement>(focus); target?.scrollIntoView({ block: 'center' }); target?.focus(); }
  }
  workflow.addEventListener('click', event => {
    const b = (event.target as Element).closest<HTMLButtonElement>('[data-step]'); if (!b) return;
    const step = STEPS[Number(b.dataset.step)]!; go(step.screen, step.floorPage);
  });
  nextButton.addEventListener('click', () => { const step = STEPS[nextStep]; if (step) { go(step.screen, step.floorPage); content.focus({ preventScroll: true }); } });
  document.addEventListener('click', event => {
    const target = event.target as Element;
    const goButton = target.closest<HTMLElement>('[data-go]');
    if (goButton) { go(Number(goButton.dataset.go), goButton.dataset.page, goButton.dataset.focus); return; }
    const act = target.closest<HTMLElement>('[data-act]');
    if (act) void action(act.dataset.act!, act);
    if (target.closest('[data-print]')) window.print();
  });
  $('#menu-toggle').addEventListener('click', () => {
    const open = !app.classList.contains('nav-open'); app.classList.toggle('nav-open', open);
    $('#menu-toggle').setAttribute('aria-expanded', String(open)); if (open) nav.querySelector<HTMLElement>('[aria-pressed=true]')?.focus();
  });
  // Phones: language, theme, token, feedback and About sit in a "More" panel.
  const more = $<HTMLButtonElement>('#topbar-more');
  function closeTools(focus: boolean): void { if (!header.classList.contains('tools-open')) return; header.classList.remove('tools-open'); more.setAttribute('aria-expanded', 'false'); if (focus) more.focus(); }
  more.addEventListener('click', () => {
    const open = !header.classList.contains('tools-open'); header.classList.toggle('tools-open', open); more.setAttribute('aria-expanded', String(open));
    // Wide screens open only the Theme and About panel; phones open every setting.
    const extra = $<HTMLElement>('#topbar-extra', header), phone = getComputedStyle(extra).display === 'contents';
    if (open) (phone ? tools : extra).querySelector<HTMLElement>('select, button')?.focus();
  });
  document.addEventListener('click', event => { if (header.classList.contains('tools-open') && !header.contains(event.target as Node)) closeTools(false); });
  document.addEventListener('keydown', event => {
    if (event.key !== 'Escape') return;
    if (app.classList.contains('nav-open')) { app.classList.remove('nav-open'); $('#menu-toggle').focus(); }
    closeTools(true);
  });

  async function action(name: string, element: HTMLElement): Promise<void> {
    const click = (selector: string) => find<HTMLElement>(selector)?.click();
    switch (name) {
      case 'open-file': click('#open-button'); break;
      case 'new-marker': go(0); click('#open-button'); break;
      case 'open-project': click('#project-open'); break;
      case 'demo': click('#demo'); go(0); break;
      case 'new-pattern':
        if (await confirmAction('Start a blank pattern? The current pattern draft is replaced. Save it first with Save pattern JSON if you need it.', 'Start blank pattern')) { go(4); click('#pd-new'); }
        break;
      case 'export': {
        const button = find<HTMLButtonElement>(`[data-export="${element.dataset.format}"]`);
        if (button) { button.click(); host.message('Preparing the file from the current marker…'); }
        break;
      }
      case 'recent': {
        const recent = find<HTMLSelectElement>('#native-recent');
        if (recent) { recent.value = element.dataset.path ?? ''; recent.dispatchEvent(new Event('change')); }
        break;
      }
    }
  }

  // ----- Home: one clear "Start here", then other ways in, the cutting floor and what to continue -----
  function homeHtml(): string {
    const L = (key: string, text: string) => `<span data-floor-label="${key}">${esc(text)}</span>`;
    const card = (act: string, iconName: string, titleKey: string, title: string, textKey: string, text: string, extra = '') => `<button type="button" class="action-card${extra}" ${act}><span class="icon-tile">${icon(iconName)}</span><span class="action-text"><strong>${L(titleKey, title)}</strong><span>${L(textKey, text)}</span></span>${icon('arrow-right', 'action-arrow')}</button>`;
    const task = (go: string, iconName: string, key: string, title: string) => `<button type="button" class="task-chip" ${go}>${icon(iconName)}${L(key, title)}</button>`;
    const flow = ['style', 'pattern', 'grade', 'lay', 'gaps', 'stripes', 'marker', 'cut'];
    return `<section id="screen-9" class="screen home" hidden aria-labelledby="home-title">
    <div class="home-hero">${logo(48, `${PRODUCT} logo`)}<div class="home-hero-text"><h1 id="home-title">${PRODUCT}</h1><p>${L('home_lead', 'Make patterns and markers, plan the cut and track every lay, on your own computer.')}</p></div>
      <ul class="chips" aria-label="About this app"><li class="chip ok">${icon('check')}${L('home_chip_open', 'Open source · Apache-2.0')}</li><li class="chip">${L('home_chip_offline', 'Works offline')}</li><li class="chip">${L('home_chip_data', 'Your data stays in your factory')}</li></ul></div>
    <div class="home-columns">
      <section class="home-section home-start" aria-labelledby="home-start"><h2 id="home-start">${L('home_start_here', 'Start here')}</h2>
        <div class="start-card">
          <span class="icon-tile">${icon('style')}</span>
          <div class="start-text"><h3>${L('make_style', 'Make a style')}</h3><p>${L('home_style_text', 'From a basic block or a blank sheet to a cut-ready marker, in eight guided steps.')}</p>
            <ol class="start-flow" aria-hidden="true">${flow.map((k, i) => `<li><b>${i + 1}</b>${esc(styleText(`step_${k}` as Parameters<typeof styleText>[0]))}</li>`).join('')}</ol>
            <div class="start-actions"><button type="button" class="primary large" data-go="${STYLE_SCREEN}" id="home-style-start">${icon('play')}${L('home_style_new', 'Start a new style')}</button><span id="home-style-continue"></span></div></div>
        </div></section>
      <section class="home-section" aria-labelledby="home-continue"><h2 id="home-continue">${L('home_continue', 'Continue')}</h2><div id="home-recent"></div></section>
    </div>
    <section class="home-section" aria-labelledby="home-have"><h2 id="home-have">${L('home_have', 'Already have patterns?')}</h2><div class="start-grid three">
      ${card(`data-go="${QUICK_SCREEN}"`, 'photo-marker', 'home_photo_title', 'Photo of paper pieces', 'home_photo_text', 'No CAD? Photograph your paper pattern pieces with a phone. BETA finds the shapes and plans the marker.')}
      ${card('data-act="new-marker"', 'marker', 'home_files_title', 'DXF files from your CAD', 'home_files_text', 'Open DXF pattern pieces, set the fabric width and nest the marker.')}
      ${isDesktop() ? card('data-act="open-project"', 'open', 'home_project_title', 'Open a project', 'home_project_text', 'A saved .betaproj folder with all screens.') : card('data-act="demo"', 'demo', 'home_demo_title', 'Try the demo', 'home_demo_text', 'A small sample shirt in the marker editor. Safe to try every tool.')}
    </div></section>
    <section class="home-section" aria-labelledby="home-floor"><h2 id="home-floor">${L('cutting_floor', 'Cutting floor')}</h2><div class="task-row">
      ${task('data-go="1"', 'plan', 'home_plan', 'Plan a cut order')}
      ${task('data-go="6"', 'rolls', 'home_rolls', 'Inspect rolls')}
      ${task('data-go="8" data-page="spreading"', 'print', 'home_spread', 'Print a spreading sheet')}
      ${task('data-go="7"', 'bundles', 'home_bundles', 'Make bundles')}
      ${task('data-go="10"', 'reports', 'reports', 'Reports')}
    </div></section>
    <div class="home-foot"><span>${L('home_promise', 'Open source · runs on your own PC or server · your data stays in your factory.')}</span><button type="button" class="small ghost" data-open-about>${icon('info')}About ${PRODUCT} ${esc(VERSION)}</button></div>
    </section>`;
  }
  function renderHome(): void {
    const list: string[] = [];
    const job = host.state().job;
    list.push(`<li><button type="button" data-go="0"><span class="icon-tile">${icon('marker')}</span><span><strong>${esc(job.name)}</strong><small>Current marker · ${job.pieces.length} pieces · ${(job.fabric.width / 10).toFixed(1)} cm wide</small></span>${icon('chevron-right', 'action-arrow')}</button></li>`);
    try {
      const saved = JSON.parse(localStorage.getItem('beta-cutting-room-v1') ?? 'null') as { document?: { name?: string; sizes?: string[] } } | null;
      if (saved?.document?.name) list.push(`<li><button type="button" data-go="4"><span class="icon-tile">${icon('pattern')}</span><span><strong>${esc(saved.document.name)}</strong><small>Pattern draft on this computer · ${saved.document.sizes?.length ?? 0} sizes</small></span>${icon('chevron-right', 'action-arrow')}</button></li>`);
    } catch { /* No draft. */ }
    const recent = find<HTMLSelectElement>('#native-recent');
    if (recent) {
      for (const o of Array.from(recent.options).filter(o => o.value).slice(0, 6)) list.push(`<li><button type="button" data-act="recent" data-path="${esc(o.value)}"><span class="icon-tile">${icon('file')}</span><span><strong>${esc(o.value.split(/[\\/]/).pop())}</strong><small>${esc(o.value)}</small></span>${icon('chevron-right', 'action-arrow')}</button></li>`);
    }
    const files = isDesktop() ? [] : readRecent();
    // A style in progress (Style flow session on this computer) can be continued from Home.
    let style: { info?: { name?: string }; step?: string; doc?: unknown } | null = null;
    try { style = JSON.parse(localStorage.getItem('beta-style-v1') ?? 'null'); } catch { /* none */ }
    const lang = (document.documentElement.dataset.language ?? 'en') as FloorLanguage, l: FloorLanguage = lang === 'hi' || lang === 'ta' ? lang : 'en';
    $('#home-style-continue').innerHTML = style?.doc && style.step ? `<button type="button" data-go="${STYLE_SCREEN}">${icon('arrow-right')}${esc(label(l, 'home_continue'))}: ${esc(style.info?.name || styleText('step_style'))} · ${esc(styleText(`step_${style.step}` as Parameters<typeof styleText>[0]))}</button>` : '';
    $('#home-recent').innerHTML = `<ul class="recent-list">${list.join('')}</ul>${files.length ? `<details class="recent-files"><summary>Files opened recently in this browser (${files.length})</summary><p class="hint">For safety a browser cannot reopen a file by itself. Choose Open a saved marker and pick it again.</p><ul class="plain-list">${files.map(f => `<li>${esc(f.name)} <span class="chip">${esc(f.kind)}</span> <span class="hint">${esc(new Date(f.at).toLocaleString())}</span></li>`).join('')}</ul></details>` : ''}`;
  }
  for (const [selector, kind] of [['#open-file', 'Marker'], ['#pattern-file', 'Pattern'], ['#roll-file', 'Rolls'], ['#compare-file', 'Compare'], ['#digitize-photo', 'Photo']] as const) {
    find<HTMLInputElement>(selector)?.addEventListener('change', event => { const f = (event.target as HTMLInputElement).files?.[0]; if (f) rememberFile(f.name, kind); });
  }

  // ----- Reports -----
  function reportsHtml(): string {
    const card = (iconName: string, title: string, text: string, where: string, buttons: string) => `<article><div class="report-head"><span class="icon-tile">${icon(iconName)}</span><h2>${title}</h2></div><p>${text}</p><span class="where">${where}</span><div class="toolgroup">${buttons}</div></article>`;
    return `<section id="screen-10" class="screen" hidden aria-labelledby="reports-title"><div class="screen-head"><div><span class="eyebrow">Reports and help</span><h1 id="reports-title" data-floor-label="reports">Reports</h1><p>Every report is made on this computer from checked data. Choose a report to open the screen that makes it.</p></div></div>
    <div class="report-grid">
      ${card('marker', 'Marker report and mini-marker', 'Marker length, efficiency, piece counts and a small printable picture of the current marker. The marker must pass all checks.', 'From: Marker editor', '<button type="button" data-act="export" data-format="report">Report (TXT)</button><button type="button" data-act="export" data-format="mini">Mini-marker (PDF)</button><button type="button" data-go="0">Open marker</button>')}
      ${card('plan', 'Cut plan', 'Lays, plies, size ratios, marker lengths and total fabric in metres for an order. Print it for the cutting master.', 'From: Cut plan', '<button type="button" data-go="1">Open cut plan</button>')}
      ${card('spread', 'Lay sheet and spreader files', 'Printable lay sheet with rolls, plies and end allowances, plus generic spreader CSV/JSON.', 'From: Floor › Spreading', '<button type="button" data-go="8" data-page="spreading">Open spreading</button>')}
      ${card('cut', 'Cutting report and fabric cost', 'Issued fabric, end bits, cut pieces, re-cuts and an estimated fabric cost per garment.', 'From: Bundles', '<button type="button" data-go="7">Open bundles</button>')}
      ${card('floor', 'Daily floor dashboard', 'Fabric used, weighted efficiency, end bits, re-cuts and sewing issues for a date. Export consumption as CSV.', 'From: Floor › Dashboard', '<button type="button" data-go="8" data-page="dashboard">Open dashboard</button>')}
      ${card('sewing', 'Sewing work in progress', 'Bundles issued to each sewing line, as a CSV for BETA Time.', 'From: Floor › Issue to sewing', '<button type="button" data-go="8" data-page="sewing">Open sewing issue</button>')}
      ${card('rolls', 'Roll inspection', '4-point inspection scores, pass or fail against the buyer limit, and shade and shrinkage groups.', 'From: Rolls', '<button type="button" data-go="6">Open rolls</button>')}
      ${card('help', 'Day in the cutting room', 'A printable one-page walkthrough from pattern to sewing for training new staff.', 'From: Help', '<button type="button" data-go="3">Open help</button><button type="button" data-go="8" data-page="manual">User manual</button>')}
    </div></section>`;
  }

  // ----- Language: one switch for the whole app, kept in step with the Floor switch -----
  const language = $<HTMLSelectElement>('#app-language');
  const floorLanguage = find<HTMLSelectElement>('#floor-language');
  if (floorLanguage) {
    language.value = floorLanguage.value;
    language.addEventListener('change', () => { floorLanguage.value = language.value; floorLanguage.dispatchEvent(new Event('change')); sync(current); });
    floorLanguage.addEventListener('change', () => { language.value = floorLanguage.value; });
    floorLanguage.dispatchEvent(new Event('change'));
  } else language.disabled = true;

  // ----- Units: one choice (cm or inch) for every screen -----
  const showUnit = (unit: AppUnit) => tools.querySelectorAll<HTMLButtonElement>('[data-app-unit]').forEach(b => b.setAttribute('aria-pressed', String(b.dataset.appUnit === unit)));
  showUnit(appUnit()); document.documentElement.dataset.unit = appUnit();
  tools.addEventListener('click', event => { const b = (event.target as Element).closest<HTMLButtonElement>('[data-app-unit]'); if (b) setAppUnit(b.dataset.appUnit === 'inch' ? 'inch' : 'cm'); });
  onUnit(showUnit);

  // ----- Theme -----
  const themeSelect = $<HTMLSelectElement>('#app-theme');
  themeSelect.value = ['light', 'dark', 'system'].includes(theme) ? theme : 'light';
  themeSelect.addEventListener('change', () => { theme = themeSelect.value; applyTheme(theme); try { localStorage.setItem(THEME_KEY, theme); } catch { /* session only */ } });

  // ----- About -----
  const about = $<HTMLDialogElement>('#about-dialog');
  $('#about-button').addEventListener('click', () => about.showModal());
  document.addEventListener('click', event => { if ((event.target as Element).closest('[data-open-about]')) about.showModal(); });

  // ----- Server token (browser only; session storage; never in a URL) -----
  const tokenDialog = $<HTMLDialogElement>('#token-dialog');
  function tokenState(): void {
    const set = !!serverToken();
    if (!isDesktop()) $('#token-label').textContent = set ? 'Token set' : 'Server token';
    $('#token-state').textContent = set ? 'A token is saved for this tab.' : 'No token is saved.';
  }
  function askToken(): void {
    if (isDesktop() || tokenDialog.open) return;
    $<HTMLInputElement>('#token-input').value = ''; tokenState(); tokenDialog.showModal(); $<HTMLInputElement>('#token-input').focus();
  }
  if (!isDesktop()) {
    tokenState();
    $('#token-button').addEventListener('click', askToken);
    const save = () => { const value = $<HTMLInputElement>('#token-input').value; if (!value.trim()) { $('#token-state').textContent = 'Enter the token, or choose Cancel.'; return; } setServerToken(value); tokenState(); tokenDialog.close(); host.message('Token saved for this tab. Checking the server…'); host.recheck(); };
    $('#token-save').addEventListener('click', save);
    $('#token-input').addEventListener('keydown', event => { if (event.key === 'Enter') { event.preventDefault(); save(); } });
    $('#token-clear').addEventListener('click', () => { setServerToken(''); tokenState(); tokenDialog.close(); host.message('Token removed from this tab.'); host.recheck(); });
  }

  // ----- Toasts: info hides after a while; errors stay until dismissed -----
  let timer = 0;
  const showToast = () => {
    window.clearTimeout(timer);
    const visible = !messageEl.hidden && !!messageEl.textContent?.trim();
    toasts.hidden = !visible; toasts.classList.remove('fading');
    if (visible && !messageEl.classList.contains('error')) timer = window.setTimeout(() => { toasts.classList.add('fading'); timer = window.setTimeout(() => { toasts.hidden = true; }, 400); }, 9000);
  };
  new MutationObserver(showToast).observe(messageEl, { childList: true, characterData: true, subtree: true, attributes: true, attributeFilter: ['hidden', 'class'] });
  $('.toast-close', toasts).addEventListener('click', () => { window.clearTimeout(timer); toasts.hidden = true; });
  showToast();

  // ----- Large tables: add a row filter above any table with more than 8 rows -----
  let pending = 0;
  const enhance = () => {
    pending = 0;
    content.querySelectorAll<HTMLTableElement>('.table-scroll table, .floor-scroll table').forEach(table => {
      const rows = table.tBodies[0]?.rows.length ?? 0;
      const box = table.closest<HTMLElement>('.table-scroll, .floor-scroll')!;
      const existingFilter = box.previousElementSibling?.classList.contains('table-filter');
      if (rows <= 8 || existingFilter || table.dataset.filterReady) return;
      table.dataset.filterReady = 'true';
      const name = table.caption?.textContent?.trim() || 'table';
      const filter = document.createElement('div'); filter.className = 'table-filter no-print';
      filter.innerHTML = `<input type="search" placeholder="Filter ${esc(name.toLowerCase())}…" aria-label="Filter rows in ${esc(name)}"><output aria-live="polite">${rows} rows</output>`;
      box.before(filter);
      const input = filter.querySelector('input')!, out = filter.querySelector('output')!;
      input.addEventListener('input', () => {
        const q = input.value.trim().toLowerCase(); let shown = 0;
        for (const row of Array.from(table.tBodies[0]!.rows)) { const hide = !!q && !row.textContent!.toLowerCase().includes(q); row.classList.toggle('filtered-out', hide); if (!hide) shown++; }
        out.textContent = q ? `${shown} of ${rows} rows` : `${rows} rows`;
      });
    });
  };
  new MutationObserver(() => { if (!pending) pending = requestAnimationFrame(enhance); }).observe(content, { childList: true, subtree: true });

  // ----- Empty states for result areas filled later by each screen -----
  const empty: Record<string, string> = {
    '#digitize-pieces': 'No scan yet. Choose a photo of pieces on the A3 target.',
    '#bundle-table': 'No bundles yet. Prepare a lay and choose Make bundles.',
    '#inspection-result': 'No inspection yet. Add rolls and defects, then inspect.',
    '#spread-result': 'No allocation yet. Inspect rolls, then allocate a group to the cut plan.',
  };
  for (const [selector, text] of Object.entries(empty)) find(selector)?.setAttribute('data-empty', text);

  // ----- Units inside number fields, menus that close on Escape and outside clicks -----
  installUx(document.body);

  // ----- Feedback button, screen links, welcome and one-time ask (self-contained module) -----
  installFeedback({ state: host.state, message: host.message });

  // ----- First screen: the address (#/pattern) or Home. Later address changes (Back, Forward, a typed or linked address) follow. -----
  const fromHash = SCREENS.find(s => `#/${s.hash}` === location.hash);
  host.show(fromHash ? fromHash.index : 9);
  if (!fromHash) homeScreen.querySelector<HTMLElement>('h1')?.setAttribute('tabindex', '-1');
  started = true;
  const follow = () => {
    const target = SCREENS.find(s => `#/${s.hash}` === location.hash);
    if (!target || target.index === current) return;
    fromAddress = true;
    try { host.show(target.index); } finally { fromAddress = false; }
  };
  window.addEventListener('hashchange', follow);
  window.addEventListener('popstate', follow);

  return { askToken, confirm: confirmAction };
}
