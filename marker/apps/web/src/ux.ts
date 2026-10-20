/** Presentation helpers shared by all screens. They never change values or app state.
 * - Units: a number field whose label ends in "(mm)", "(m)", "(%)" … shows the unit inside the field.
 *   The label text keeps the unit for screen readers; the input element is never moved, so focus is safe
 *   while screens re-render their forms.
 * - Menus: <details class="menu"> closes on an outside click, on Escape (focus returns to its button)
 *   and after a menu item is chosen. */
const UNITS: Record<string, string> = {
  mm: 'mm', m: 'm', cm: 'cm', '%': '%', seconds: 's', s: 's', degrees: '°', '°': '°', 'mm²': 'mm²', INR: 'INR', 'INR per metre': 'INR/m',
};
const UNIT_TEXT = /^(.*\S)\s*\(([^()]+)\)\s*$/;

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
  });
}

function closeMenus(except?: Element | null): void {
  document.querySelectorAll<HTMLDetailsElement>('details.menu[open]').forEach(menu => { if (menu !== except) menu.open = false; });
}

export function installUx(root: HTMLElement): void {
  addUnits(root);
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
