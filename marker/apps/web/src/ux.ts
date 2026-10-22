import { appUnit, onUnit, type AppUnit } from './units-pref';
import { findAll } from './screens';

/** Presentation helpers shared by all screens. They never change values or app state.
 * - Units: a number field whose label ends in "(mm)", "(m)", "(%)" … shows the unit inside the field.
 *   The label text keeps the unit for screen readers; the input element is never moved, so focus is safe
 *   while screens re-render their forms.
 * - One unit system: a length field labelled "(mm)" shows and accepts cm or inch, and one labelled "(m)" shows
 *   metres or yards, as chosen in the top bar. Code that reads or writes `input.value` still gets millimetres
 *   or metres, so screens, saved files and engines do not change.
 * - Menus: <details class="menu"> closes on an outside click, on Escape (focus returns to its button)
 *   and after a menu item is chosen. */
const UNITS: Record<string, string> = {
  mm: 'mm', m: 'm', cm: 'cm', '%': '%', seconds: 's', s: 's', degrees: '°', '°': '°', 'mm²': 'mm²', INR: 'INR', 'INR per metre': 'INR/m',
};
const UNIT_TEXT = /^(.*\S)\s*\(([^()]+)\)\s*$/;

/** Length fields: code unit, and the unit shown for each app unit (with the size of one shown unit in code units). */
const LENGTHS: Record<string, { shown: Record<AppUnit, [string, number]>; digits: Record<AppUnit, number> }> = {
  mm: { shown: { cm: ['cm', 10], inch: ['in', 25.4] }, digits: { cm: 2, inch: 3 } },
  m: { shown: { cm: ['m', 1], inch: ['yd', 0.9144] }, digits: { cm: 3, inch: 3 } },
};
const NATIVE = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!;
const round = (n: number, digits: number) => Number(n.toFixed(digits));
/** Code value (mm or m) → text shown in the field, and back. Empty stays empty; nonsense passes through. */
export function shownValue(code: string, codeUnit: string, unit: AppUnit): string {
  const spec = LENGTHS[codeUnit]; if (!spec || code.trim() === '') return code;
  const n = Number(code); if (!Number.isFinite(n)) return code;
  return String(round(n / spec.shown[unit][1], spec.digits[unit]));
}
export function codeValue(shown: string, codeUnit: string, unit: AppUnit): string {
  const spec = LENGTHS[codeUnit]; if (!spec || shown.trim() === '') return shown;
  const n = Number(shown.trim().replace(',', '.')); if (!Number.isFinite(n)) return shown;
  return String(round(n * spec.shown[unit][1], codeUnit === 'mm' ? 3 : 5));
}
/** Make one input show the app unit; its `value` property keeps the code unit (mm or m). */
function lengthField(label: HTMLLabelElement, input: HTMLInputElement, codeUnit: string): void {
  if (input.dataset.len) return;
  const unit = appUnit();
  input.dataset.len = codeUnit; input.dataset.lenUnit = unit;
  const current = NATIVE.get!.call(input) as string;
  // Limits keep their meaning: stored in the code unit, shown in the app unit.
  for (const [name, key] of [['min', 'lenMin'], ['max', 'lenMax']] as const) { const v = input.getAttribute(name); if (v !== null && v !== '') { input.dataset[key] = v; input.setAttribute(name, shownValue(v, codeUnit, unit)); } }
  input.step = 'any';
  Object.defineProperty(input, 'value', {
    configurable: true,
    get(this: HTMLInputElement) { return codeValue(NATIVE.get!.call(this) as string, codeUnit, (this.dataset.lenUnit as AppUnit) || 'cm'); },
    set(this: HTMLInputElement, v: string) { NATIVE.set!.call(this, shownValue(String(v ?? ''), codeUnit, (this.dataset.lenUnit as AppUnit) || 'cm')); },
  });
  Object.defineProperty(input, 'valueAsNumber', { configurable: true, get(this: HTMLInputElement) { return this.value === '' ? NaN : Number(this.value); }, set(this: HTMLInputElement, n: number) { this.value = String(n); } });
  NATIVE.set!.call(input, shownValue(current, codeUnit, unit));
  showUnit(label, codeUnit, unit);
}
function showUnit(label: HTMLLabelElement, codeUnit: string, unit: AppUnit): void {
  const [text] = LENGTHS[codeUnit]!.shown[unit];
  label.dataset.unit = text; label.style.setProperty('--unit-w', `${Math.max(2.2, text.length * 0.62 + 1.4)}em`);
  const spoken = label.querySelector<HTMLElement>(':scope > .sr-only[data-spoken-unit]'); if (spoken) spoken.textContent = ` (${text})`;
}
/** The user changed the unit: every length field keeps its length and shows it in the new unit. */
function switchUnit(unit: AppUnit): void {
  for (const input of findAll<HTMLInputElement>('input[data-len]')) {
    const code = input.value; input.dataset.lenUnit = unit; input.value = code;
    for (const [name, key] of [['min', 'lenMin'], ['max', 'lenMax']] as const) { const v = input.dataset[key]; if (v) input.setAttribute(name, shownValue(v, input.dataset.len!, unit)); }
    const label = input.closest<HTMLLabelElement>('label[data-unit]'); if (label) showUnit(label, input.dataset.len!, unit);
  }
}

/** Add data-unit to number-field labels inside root. Safe to call many times. */
export function addUnits(root: ParentNode): void {
  root.querySelectorAll<HTMLLabelElement>('label:not([data-unit]):not(.check)').forEach(label => {
    const input = label.querySelector<HTMLInputElement>(':scope > input[type=number]');
    const text = label.firstChild;
    if (!input || !text || text.nodeType !== Node.TEXT_NODE) return;
    const match = UNIT_TEXT.exec(text.textContent ?? '');
    const unit = match ? UNITS[match[2]!.trim()] : undefined;
    if (!match || !unit) return;
    // "Fabric width (mm)" → visible "Fabric width" + hidden "(mm)" for screen readers; "mm" shows in the field.
    text.textContent = match[1]!;
    const spoken = document.createElement('span'); spoken.className = 'sr-only'; spoken.textContent = ` (${match[2]!.trim()})`;
    text.after(spoken);
    label.dataset.unit = unit;
    label.classList.add('has-unit');
    label.style.setProperty('--unit-w', `${Math.max(2.2, unit.length * 0.62 + 1.4)}em`);
    if (LENGTHS[unit]) { spoken.dataset.spokenUnit = ''; lengthField(label, input, unit); }
  });
}

function closeMenus(except?: Element | null): void {
  document.querySelectorAll<HTMLDetailsElement>('details.menu[open]').forEach(menu => { if (menu !== except) menu.open = false; });
}

export function installUx(root: HTMLElement): void {
  addUnits(root);
  onUnit(switchUnit);
  new MutationObserver(records => {
    for (const record of records) {
      // Canvas redraws are frequent and never contain form labels.
      if (record.target instanceof SVGElement) continue;
      for (const node of Array.from(record.addedNodes)) if (node instanceof HTMLElement) addUnits(node.parentElement ?? node);
    }
  }).observe(root, { childList: true, subtree: true });

  document.addEventListener('click', event => {
    const target = event.target as Element;
    const menu = target.closest('details.menu');
    closeMenus(menu);
    // Choosing an item closes its menu; the item's own click handler still runs.
    if (menu && target.closest('.menu-panel button')) (menu as HTMLDetailsElement).open = false;
  });
  document.addEventListener('keydown', event => {
    if (event.key !== 'Escape') return;
    const open = (event.target as Element).closest?.('details.menu[open]') ?? document.querySelector('details.menu[open]');
    if (!open) return;
    (open as HTMLDetailsElement).open = false;
    open.querySelector<HTMLElement>('summary')?.focus();
  });
}
