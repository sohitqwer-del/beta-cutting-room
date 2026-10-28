import { find } from '../screens';
/** Legacy mock smoke. Run only at `?quick-mock=1#/quick` (explicit practice engine).
 * The normal sample button now uses the real scanner. See sample-scan.mjs and launch8.mjs for real-engine checks.
 *   const s = await import('/src/quick/quick-smoke.mjs'); await s.runQuickSmoke();
 * Or one step at a time (the headless screenshot runner does this): await s.runStep('review').
 * Uses only visible controls and DOM text. Nesting uses the local Rust server when it answers. */
const get = s => { const e = find(s); if (!e) throw new Error(`Quick smoke: missing ${s}`); return e; };
const all = s => Array.from(document.querySelectorAll(s));
const click = s => get(s).click();
const tick = (ms = 60) => new Promise(r => setTimeout(r, ms));
const assert = (ok, label) => { if (!ok) throw new Error(`Quick smoke: ${label}. Message: ${document.querySelector('#message')?.textContent ?? ''}`); };
async function until(test, label, ms = 30000) { const end = Date.now() + ms; while (Date.now() < end) { try { if (test()) return; } catch { /* not yet */ } await tick(100); } throw new Error(`Quick smoke timed out: ${label}`); }
const step = () => document.querySelector('#screen-11')?.dataset.step;
const set = (s, v, kind = 'change') => { const e = get(s); e.value = String(v); e.dispatchEvent(new Event(kind, { bubbles: true })); };
const sheetOpen = () => document.querySelector('#q-sheet')?.open;
const pieceButton = name => all('.q-piece').find(b => b.querySelector('strong')?.textContent === name && !b.classList.contains('q-deleted'));

export const STEPS = {
  async start() {
    click('[data-screen="11"]'); await tick(200);
    if (step() !== 'start') { const restart = document.querySelector('#q-restart'); if (restart) { window.confirm = () => true; restart.click(); } }
    await until(() => step() === 'start' && !get('#screen-11').hidden, 'start screen');
    assert(all('.q-guide figure').length === 3, 'three-picture guide');
    assert(all('input[name=q-mode]').length === 2, 'quick and accurate modes');
    assert(get('#q-camera').getAttribute('capture') === 'environment' && get('#q-camera').accept === 'image/*', 'camera input');
    assert(document.querySelector('[data-screen="11"]'), 'nav item');
    return { title: get('#q-title').textContent };
  },
  async capture() {
    click('#q-sample');
    await until(() => step() === 'capture' && all('.q-photo-card').length === 2 && !document.querySelector('.q-status.q-reading'), 'two sample photos read');
    assert(get('.q-practice-chip'), 'practice mode is labelled');
    assert(all('.q-warnings li').length >= 1, 'quality warning with a fix');
    assert(get('.q-warnings li span span').textContent.length > 10, 'warning has a fix');
    return { photos: all('.q-photo-card').length, warnings: all('.q-warnings li').map(li => li.textContent) };
  },
  async review() {
    click('#q-to-review');
    await until(() => step() === 'review', 'review step');
    assert(all('.q-piece').length === 9, 'nine pieces found');
    assert(all('.q-shape').length === 7, 'outlines drawn over photo 1');
    click('#q-to-order'); await tick();
    assert(step() === 'review', 'cannot leave review with pieces waiting');
    return { pieces: all('.q-piece').length };
  },
  async sheet() {
    pieceButton('Piece 1').click(); await until(() => sheetOpen(), 'piece sheet');
    assert(get('#q-chip-pair') && get('#q-chip-name'), 'pair and name suggestions');
    click('#q-chip-name'); await until(() => get('#q-sheet-title').textContent === 'Front', 'name chip applied');
    click('#q-chip-pair'); await until(() => !document.querySelector('#q-chip-pair'), 'pair chip applied');
    assert(get('#q-pair').checked, 'pair switch on');
    assert(get('#q-copies').value === '1', 'one pair per garment');
    return { chips: all('.q-chip').length };
  },
  async decide() {
    click('#q-accept'); await tick(120);
    // The sheet moves to the next waiting piece. Back: accept fold.
    const back = () => pieceButton('Piece 3');
    get('#q-sheet').close(); await tick();
    assert(all('.q-piece.q-deleted').length === 1, 'pair mate removed');
    back().click(); await until(() => sheetOpen() && document.querySelector('#q-chip-fold'), 'back sheet with fold chip');
    click('#q-chip-fold'); await until(() => get('#q-fold').checked, 'fold applied');
    click('#q-chip-name'); click('#q-accept'); await tick(120);
    get('#q-sheet').close(); await tick();
    // Sleeve: accept "cut 1 + 1" and the + stepper.
    pieceButton('Piece 4').click(); await until(() => sheetOpen(), 'sleeve sheet');
    click('#q-chip-quantity'); await until(() => get('#q-pair').checked, 'quantity chip makes a pair');
    click('#q-copies-plus'); await until(() => get('#q-copies').value === '2', 'stepper adds a pair');
    click('#q-copies-minus'); await until(() => get('#q-copies').value === '1', 'stepper removes a pair');
    click('#q-accept'); await tick(120); get('#q-sheet').close(); await tick();
    // Duplicate pocket in photo 2: remove by chip.
    const dupes = all('.q-piece').filter(b => b.querySelector('strong')?.textContent === 'Piece 2' && b.classList.contains('q-pending'));
    let removed = false;
    for (const b of dupes) { b.click(); await until(() => sheetOpen(), 'piece 2 sheet'); const chip = document.querySelector('#q-chip-duplicate'); if (chip) { chip.click(); removed = true; } get('#q-sheet').close(); await tick(); if (removed) break; }
    assert(removed, 'duplicate chip removes the second pocket');
    click('#q-accept-all'); await tick();
    assert(!document.querySelector('.q-piece.q-pending'), 'all decided');
    return { accepted: all('.q-piece.q-accepted').length, removed: all('.q-piece.q-deleted').length };
  },
  async order() {
    click('#q-to-order'); await until(() => step() === 'order', 'order step');
    // One app unit (0.7): the app starts in cm, so switch to inch for the inch presets.
    click('#q-unit-inch'); await tick();
    click('[data-q="preset"][data-value="58"]'); await tick();
    assert(get('#q-width').value === '58', 'preset fills width');
    click('#q-unit-cm'); await tick();
    assert(get('#q-width').value === '147.3', 'unit toggle converts width');
    set('#q-width', '150', 'input');
    assert(get('#q-width-error').textContent.includes('150.0 cm'), 'typed width');
    click('[data-q="garments"][data-delta="1"]'); await tick();
    assert(get('[data-q-change="garments-set"]').value === '2', 'garment stepper');
    get('#q-nap').click(); await tick();
    click('#q-budget-20');
    return { width: get('#q-width').value };
  },
  async nestStart() {
    click('#q-make');
    await until(() => step() === 'nest' || step() === 'use', 'nest step');
    await until(() => step() === 'use' || document.querySelector('#q-preview .q-marker') || document.querySelector('.notice.error'), 'first result', 60000);
    return { live: document.querySelector('#q-best-len')?.textContent ?? 'done' };
  },
  async nestFinish() {
    if (step() === 'nest' && document.querySelector('#q-stop')) { await tick(1000); click('#q-stop'); }
    await until(() => step() === 'use' || document.querySelector('.notice.error'), 'marker ready', 90000);
    assert(step() === 'use', `nest result (${document.querySelector('.notice.error')?.textContent ?? ''})`);
    return { tiles: all('.q-result strong').map(s => s.textContent), engine: document.querySelector('.q-marker') ? 'drawn' : 'none' };
  },
  async use() {
    set('#q-usual', '1.8', 'input');
    assert(get('#q-saving').textContent.length > 5, 'saving vs usual shown');
    click('#q-paper-A3'); click('#q-tiles'); await tick(300);
    click('#q-chalk'); await tick(300);
    return { saving: get('#q-saving').textContent, practice: !!document.querySelector('.q-practice-chip') };
  },
  async projector() {
    localStorage.removeItem('beta-quick-projector');
    click('#q-projector-btn'); await until(() => !get('#q-projector').hidden && document.querySelector('.q-proj-square'), 'projector with calibration square');
    set('#q-cal-w', '120'); set('#q-cal-h', '110');
    get('#q-proj-form').requestSubmit(); await tick(120);
    const cal = JSON.parse(localStorage.getItem('beta-quick-projector'));
    assert(cal.measured && Math.abs(cal.pxPerMmX - 96 / 25.4 * 100 / 120) < 1e-9, 'calibration corrected');
    click('#q-proj-cal'); await tick(80);
    return { parts: get('.q-proj-part strong').textContent, pieces: all('.q-proj-piece').length };
  },
  async projectorNext() {
    if (!get('#q-proj-next').disabled) { click('#q-proj-next'); await tick(80); }
    const part = get('.q-proj-part strong').textContent;
    click('#q-proj-exit'); await until(() => get('#q-projector').hidden, 'projector closed');
    return { part };
  },
  async editor() {
    click('#q-editor'); await until(() => !document.querySelector('.workspace').hidden, 'editor open');
    await until(() => document.querySelectorAll('#canvas [data-index]').length > 0, 'placements in the editor');
    return { placed: document.querySelectorAll('#canvas [data-index]').length };
  },
};
export async function runStep(name) { return STEPS[name](); }
export async function runQuickSmoke() {
  const out = {};
  for (const name of Object.keys(STEPS)) out[name] = await STEPS[name]();
  return out;
}
