import type { CutPlan, Job, PlanRequest, RollRegister } from '@beta/marker-wasm';
import type { CostComparison, CostComparisonRequest, WidthOffer } from './cost-types';
import { find } from './screens';

const esc = (value: unknown): string => String(value ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]!));
interface Host {
  input(): PlanRequest; width(): number; busy(): boolean; setBusy(value: boolean): void; register(): RollRegister;
  use(plan: CutPlan, job: Job, allowance: number): void;
  /** Garments per size in the Cut plan form above. */
  quantities(): Record<string, number>;
  /** "Try with a sample": fill an order when the form has none. */
  sampleOrder(): void;
}
/** Every size in the order must be measured at least once, so the ratio search needs at least that many measurements. */
export function minRefinements(quantities: Record<string, number>): number {
  return Math.min(16, Math.max(1, Object.values(quantities).filter(n => n > 0).length));
}
/** Plain words for a failed alternative. When the same order planned at other widths, an "order incomplete" error
 * at this width means the pieces could not be placed at this width, not that the order is wrong. */
export function trialReason(error: string | null, width: number, others: { width_mm: number; feasible: boolean }[]): string | null {
  if (!error) return null;
  if (/quantit|incomplete|demand/i.test(error) && others.some(o => o.feasible && o.width_mm !== width))
    return 'No marker fits this width with all pieces. The order itself is fine: other widths planned it.';
  return null;
}
export interface CardFigures { title: string; status: 'best' | 'checked' | 'short' | 'failed'; total: string; lines: [string, string][] }
/** What one alternative card shows: the recommended one first, then checked, short trials and failures. */
export function trialOrder(trials: { feasible: boolean; final_evaluated: boolean; cost: { total: number } | null }[], chosen: number | null): number[] {
  const rank = (i: number) => { const t = trials[i]!; return i === chosen ? 0 : t.feasible && t.final_evaluated ? 1 : t.feasible ? 2 : 3; };
  return trials.map((_, i) => i).sort((a, b) => rank(a) - rank(b) || (trials[a]!.cost?.total ?? Infinity) - (trials[b]!.cost?.total ?? Infinity) || a - b);
}
/** Parse operator-entered ratio menus. Rust validates demand and computes every plan. */
export function readRatios(text: string): Record<string, number>[] {
  return text.split('\n').map(line => line.trim()).filter(Boolean).map(line => {
    const entries = line.split(',').map(part => part.trim().split('='));
    const ratio: Record<string, number> = Object.create(null) as Record<string, number>;
    for (const [name, raw, extra] of entries) {
      const count = Number(raw);
      if (!name || extra !== undefined || raw === undefined || !raw.trim() || !Number.isSafeInteger(count) || count < 0 || count > 4294967295 || Object.hasOwn(ratio, name.trim()))
        throw new Error('Enter each ratio as size=count, with unique sizes and whole counts.');
      ratio[name.trim()] = count;
    }
    if (!Object.values(ratio).some(count => count > 0)) throw new Error('Each ratio needs a positive count.');
    return ratio;
  });
}
/** Existing measured-cost engine, presented as a reviewed alternative selection. */
export function installCostComparison(host: Host): void {
  const panel = find<HTMLElement>('#screen-1')!;
  panel.insertAdjacentHTML('beforeend', `<details id="cost-comparison" class="cost-card"><summary><span class="cost-summary"><strong>Compare fabric widths and order cost</strong><small>Which width and ratio make this order cheapest? Each choice is nested and checked.</small></span></summary>
    <p class="hint">Uses the garments and lay limits above. Open fabric only. Failed choices stay in the list with the reason.</p>
    <form id="cost-compare-form"><fieldset id="cost-compare-inputs"><legend class="sr-only">Comparison inputs</legend>
    <div class="cost-offers-head"><h3>Width offers</h3><button type="button" class="small" id="cost-sample">Try with a sample</button></div>
    <div id="cost-offers" class="cost-offers"></div><button type="button" id="cost-add-offer" class="small">Add width offer</button>
    <div class="fields two"><label>Currency<input name="currency" value="INR" required maxlength="12"></label></div>
    <details class="advanced"><summary>More options: ratios, search time, stock</summary>
    <label>Your own ratios: one per line, for example S=1,M=2,L=1<textarea name="ratios" rows="3" placeholder="Leave blank for automatic ratios"></textarea></label>
    <label class="check"><input name="automatic" type="checkbox" checked> Also compare automatic ratios</label>
    <div class="fields two"><label>Short search per marker (s)<input name="short" type="number" min="0.05" max="30" step="any" value="2" required></label>
    <label>Final search per marker (s)<input name="final" type="number" min="0.05" max="120" step="any" value="8" required></label>
    <label>Finalists<input name="finalists" type="number" min="1" max="4" value="2" required></label>
    <label>Ratios measured per choice<input name="refinements" type="number" min="1" max="16" value="8" required aria-describedby="cost-refinements-hint"></label></div>
    <p class="hint" id="cost-refinements-hint">At least one per size in the order.</p>
    <p class="hint">Stock is optional. Without it, availability is not checked. When an offer asks for it, the comparison uses the Rolls register: a shortage stops that choice from winning. Mapped defects still need the cutting-release check.</p></details>
    <div class="cost-actions"><button type="submit" class="primary">Compare order cost</button><button type="button" id="cost-stop" class="danger" hidden>Stop comparison</button></div></fieldset></form>
    <p id="cost-compare-status" class="cost-status" role="status" aria-live="polite">No comparison yet. Add one or more width offers with their prices, then choose Compare order cost.</p><div id="cost-compare-result"></div></details>`);
  const form = panel.querySelector<HTMLFormElement>('#cost-compare-form')!;
  const inputs = panel.querySelector<HTMLFieldSetElement>('#cost-compare-inputs')!;
  const offers = panel.querySelector<HTMLElement>('#cost-offers')!;
  const status = panel.querySelector<HTMLElement>('#cost-compare-status')!;
  const result = panel.querySelector<HTMLElement>('#cost-compare-result')!;
  const stop = panel.querySelector<HTMLButtonElement>('#cost-stop')!;
  let worker: Worker | undefined, rejectRunning: ((reason: Error) => void) | undefined;
  let snapshot: CostComparisonRequest | undefined, comparison: CostComparison | undefined;
  let offerSequence = 0;
  const addOffer = () => {
    if (offers.children.length >= 8) return;
    const id = ++offerSequence;
    offers.insertAdjacentHTML('beforeend', `<fieldset data-offer="${id}" class="cost-offer"><legend>Offer ${id}</legend><div class="fields two">
      <label>Usable width (mm)<input name="width" type="number" min="1" step="any" value="${host.width()}" required></label>
      <label>Fabric price<span class="cost-price"><input name="price" type="number" min="0" step="any" value="0" required aria-label="Fabric price"><select name="unit" aria-label="Price unit"><option value="metre">per metre</option><option value="kilogram">per kg</option></select></span></label>
      <label class="cost-gsm" hidden>Fabric weight (GSM)<input name="gsm" type="number" min="1" step="any" value="180" required></label>
      <label>Cost per lay<input name="lay" type="number" min="0" step="any" value="0" required></label></div>
      <details class="advanced"><summary>Allowance, remnants and stock</summary><div class="fields two">
      <label>Purchase allowance (%)<input name="wastage" type="number" min="0" step="any" value="0" required></label>
      <label>Charge opened remnants (%)<input name="charge" type="number" min="0" max="100" step="any" value="0" required></label>
      <label>Roll IDs to use (comma separated; blank = any matching roll)<input name="rolls"></label></div>
      <label class="check"><input name="stock" type="checkbox"> Use only measured stock from Rolls</label></details>
      <button type="button" class="ghost small cost-remove" data-remove-offer aria-label="Remove offer ${id}">Remove</button></fieldset>`);
    offers.lastElementChild!.querySelector<HTMLInputElement>('[name="gsm"]')!.disabled = true;
  };
  addOffer();
  panel.querySelector<HTMLButtonElement>('#cost-add-offer')!.onclick = () => {
    try { addOffer(); } catch (e) { status.textContent = String(e); }
  };
  offers.onclick = event => (event.target as Element).closest('[data-remove-offer]')?.closest('[data-offer]')?.remove();
  offers.onchange = event => {
    const unit = event.target as HTMLSelectElement;
    if (unit.name === 'unit') {
      const row = unit.closest('[data-offer]')!, gsm = row.querySelector<HTMLInputElement>('[name="gsm"]')!;
      gsm.disabled = unit.value !== 'kilogram'; gsm.closest('label')!.hidden = gsm.disabled;
    }
  };
  // Ratios measured per choice: never fewer than the sizes in the order (the engine refuses fewer).
  const refinements = form.querySelector<HTMLInputElement>('[name="refinements"]')!;
  const syncRefinements = () => {
    let q: Record<string, number> = {};
    try { q = host.quantities(); } catch { return; }
    const min = minRefinements(q), sizes = Object.values(q).filter(n => n > 0).length;
    refinements.min = String(min);
    if (Number(refinements.value) < min) refinements.value = String(min);
    panel.querySelector('#cost-refinements-hint')!.textContent = sizes ? `At least ${min}: one per size in the order (${sizes} ${sizes === 1 ? 'size' : 'sizes'}).` : 'At least one per size in the order.';
  };
  panel.querySelector('#cost-comparison')!.addEventListener('toggle', syncRefinements);
  panel.querySelector('#cost-sample')!.addEventListener('click', () => {
    host.sampleOrder();
    offers.textContent = ''; offerSequence = 0; addOffer(); addOffer();
    const rows = offers.querySelectorAll<HTMLFieldSetElement>('[data-offer]');
    const fill = (row: HTMLFieldSetElement, width: number, price: number, lay: number) => {
      row.querySelector<HTMLInputElement>('[name="width"]')!.value = String(width);
      row.querySelector<HTMLInputElement>('[name="price"]')!.value = String(price);
      row.querySelector<HTMLInputElement>('[name="lay"]')!.value = String(lay);
    };
    fill(rows[0]!, 1500, 180, 150); fill(rows[1]!, 1600, 195, 150);
    syncRefinements();
    status.textContent = 'Sample filled in: two widths (150 and 160 cm) with sample prices. Choose Compare order cost. Use your own prices for a real decision.';
  });
  const changed = () => { if (comparison) status.textContent = 'Inputs changed. The displayed comparison retains its saved inputs; compare again to update it.'; };
  form.oninput = changed;
  panel.querySelector('#plan-form')!.addEventListener('input',changed);
  stop.onclick = () => { worker?.terminate(); rejectRunning?.(new Error('Comparison stopped. No partial alternative was selected.')); };
  form.onsubmit = async event => {
    event.preventDefault();
    if (host.busy() || worker) { status.textContent = 'Finish the current operation first.'; return; }
    let next: CostComparisonRequest;
    syncRefinements();
    try {
      const base = host.input();
      const fields = new FormData(form), ratios = readRatios(String(fields.get('ratios') ?? ''));
      const widthOffers = Array.from(offers.querySelectorAll<HTMLFieldSetElement>('[data-offer]')).map((row): WidthOffer => {
        const number = (name: string) => Number(row.querySelector<HTMLInputElement>(`[name="${name}"]`)!.value);
        const price = number('price');
        const unit = row.querySelector<HTMLSelectElement>('[name="unit"]')!.value;
        const stock = row.querySelector<HTMLInputElement>('[name="stock"]')!.checked;
        const register = stock ? host.register() : null;
        const charge = number('charge');
        if (!Number.isFinite(charge) || charge < 0 || charge > 100) throw new Error('Remnant charge must be 0–100%.');
        return { id: `offer-${row.dataset.offer}`, width_mm: number('width'),
          price: unit === 'kilogram' ? { unit, price, gsm: number('gsm') } : { unit: 'metre', price },
          lay_cost: number('lay'), purchase_wastage_pct: number('wastage'), end_bit_charge_fraction: charge / 100,
          rolls: register, eligible_roll_ids: row.querySelector<HTMLInputElement>('[name="rolls"]')!.value.split(',').map(x => x.trim()).filter(Boolean) };
      });
      if (!widthOffers.length) throw new Error('Add at least one fabric width offer.');
      next = structuredClone({ order: base.order, currency: String(fields.get('currency') ?? '').trim(), offers: widthOffers,
        ratio_sets: [...(fields.has('automatic') || !ratios.length ? [{ id: 'automatic', ratios: [] }] : []), ...(ratios.length ? [{ id: 'operator-menu', ratios }] : [])],
        options: { ...base.options, time_per_marker_s: Number(fields.get('short')), final_time_s: 0, max_candidates: 64, max_refinements: Number(fields.get('refinements')), search_nodes: 20000 },
        finalists: Number(fields.get('finalists')), final_time_s: Number(fields.get('final')), allocation: {} });
    } catch (e) { status.textContent = e instanceof Error ? e.message : String(e); return; }
    host.setBusy(true); inputs.disabled = true; stop.hidden = false;
    status.textContent = 'Comparing measured markers. No winning alternative is available until final checks finish.';
    try {
      worker = new Worker(new URL('./cost.worker.ts', import.meta.url), { type: 'module' });
      const answer = await new Promise<CostComparison>((resolve, reject) => {
        rejectRunning = reject;
        const current = worker!;
        let measured = 0;
        current.onmessage = ({ data }: MessageEvent<{ type: string; value: CostComparison; message: string }>) => {
          if (data.type === 'done') resolve(data.value);
          else if (data.type === 'error') reject(new Error(data.message));
          else status.textContent = `${++measured} checked planning updates received. Final comparison still running.`;
        };
        current.onerror = event => reject(new Error(event.message || 'Comparison worker failed.'));
        current.postMessage({ request: next, moduleUrl: new URL('wasm/beta_marker_cutroom_wasm.js', document.baseURI).href });
      });
      snapshot = next; comparison = answer; render();
      const best = answer.chosen != null ? answer.trials[answer.chosen] : undefined;
      status.textContent = best?.cost ? `Recommended: ${(best.width_mm / 10).toFixed(1)} cm wide, ${answer.currency} ${best.cost.total.toLocaleString(undefined, { maximumFractionDigits: 0 })} in total. Every choice is listed below.` : 'No choice could plan the whole order. The reason is on each card below.';
    } catch (e) { status.textContent = `${e instanceof Error ? e.message : String(e)} Previous completed comparison, if any, is retained.`; }
    finally { worker?.terminate(); worker = undefined; rejectRunning = undefined; host.setBusy(false); inputs.disabled = false; stop.hidden = true; }
  };
  function render(): void {
    if (!comparison || !snapshot) return;
    const fmt = (n: number | undefined, places = 2) => n === undefined ? '—' : n.toFixed(places);
    const money = (n: number | undefined) => n === undefined ? '—' : `${esc(comparison!.currency)} ${n.toLocaleString(undefined, { maximumFractionDigits: 0 })}`;
    const trials = comparison.trials;
    const statusOf = (i: number) => { const t = trials[i]!; return i === comparison!.chosen ? 'best' : t.feasible && t.final_evaluated ? 'checked' : t.feasible ? 'short' : 'failed'; };
    const label: Record<string, string> = { best: 'Recommended · final checked', checked: 'Final checked', short: 'Short trial only', failed: 'Not possible' };
    const cards = trialOrder(trials, comparison.chosen).map(i => {
      const t = trials[i]!, st = statusOf(i), plain = trialReason(t.error, t.width_mm, trials.filter(o => o.ratio_set === t.ratio_set));
      const usable = t.final_evaluated && t.feasible && !!t.plan?.complete;
      const facts: [string, string][] = t.cost ? [['Fabric to buy', `${fmt(t.cost.purchased_m, 2)} m`], ['Lays', String(t.plan?.lays.length ?? '—')], ['Fabric cost', money(t.cost.fabric_cost)], ['Lay cost', money(t.cost.lay_cost)], ['Remnants kept', `${fmt(t.cost.recoverable_end_bits_m, 2)} m`], ['Over-cut', t.plan ? Object.entries(t.plan.overcut).map(([size, count]) => `${size}: ${count}`).join(', ') || '0' : '—']] : [];
      return `<li class="cost-alt cost-${st}"><div class="cost-alt-head"><span class="cost-width">${fmt(t.width_mm / 10, 1)} cm</span><span class="cost-offer-name">${esc(t.offer_id.replace(/^offer-/, 'Offer '))} · ${esc(t.ratio_set === 'automatic' ? 'automatic ratios' : 'your ratios')}</span><span class="cost-badge">${label[st]}</span></div>
        ${t.cost ? `<p class="cost-total"><span>Total</span><strong>${money(t.cost.total)}</strong>${t.cost.per_ordered_garment ? `<small>${money(t.cost.per_ordered_garment)} per garment</small>` : ''}</p><dl class="cost-facts">${facts.map(([k, v]) => `<div><dt>${k}</dt><dd>${esc(v)}</dd></div>`).join('')}</dl>` : ''}
        ${t.error ? `<p class="cost-reason">${esc(plain ?? t.error)}${plain ? `<br><small>Engine: ${esc(t.error)}</small>` : ''}</p>` : ''}
        <button type="button" class="${st === 'best' ? 'primary' : ''}" data-use-cost="${i}" ${usable ? '' : 'disabled'}>Use this plan</button></li>`;
    }).join('');
    const table = `<details class="advanced"><summary>All numbers as a table</summary><p class="hint">Engine note: ${esc(comparison.reason)}</p><div class="table-scroll"><table><caption>Measured alternatives · ${esc(comparison.currency)}</caption><thead><tr>${['Offer / ratio','Width (mm)','Status','Purchased (m)','Recoverable remnants (m)','Lays','Over-cut','Fabric cost','Lay cost','Total'].map(x => `<th scope="col">${x}</th>`).join('')}</tr></thead><tbody>${trials.map((trial, i) => `<tr><th scope="row">${esc(trial.offer_id)} / ${esc(trial.ratio_set)}</th><td>${fmt(trial.width_mm, 0)}</td><td>${esc(trial.error || label[statusOf(i)])}</td><td>${fmt(trial.cost?.purchased_m,3)}</td><td>${fmt(trial.cost?.recoverable_end_bits_m,3)}</td><td>${trial.plan?.lays.length ?? '—'}</td><td>${esc(trial.plan ? Object.entries(trial.plan.overcut).map(([size,count]) => `${size}: ${count}`).join(', ') || '0' : '—')}</td><td>${fmt(trial.cost?.fabric_cost)}</td><td>${fmt(trial.cost?.lay_cost)}</td><td>${fmt(trial.cost?.total)}</td></tr>`).join('')}</tbody></table></div></details>`;
    result.innerHTML = `<p class="cost-for"><strong>Compared for:</strong> ${esc(snapshot.order.job.name)} · ${esc(Object.entries(snapshot.order.quantities).filter(([, n]) => n > 0).map(([size, count]) => `${size} ${count}`).join(', '))}</p>
      <ul class="cost-alts">${cards}</ul>${table}
      <div class="cost-actions"><button type="button" id="cost-save-comparison" class="small">Save inputs and comparison (JSON)</button></div>
      <p class="hint">Zero prices are allowed for a fabric-only comparison; enter real prices before you treat the result as a cost decision. Remnants are stock you can reuse, not scrap. Cutting release still needs the mapped-defect check.</p>`;
    result.querySelector<HTMLButtonElement>('#cost-save-comparison')!.onclick = () => {
      const url = URL.createObjectURL(new Blob([JSON.stringify({ format: 'beta-marker.cost-comparison/v1', request: snapshot, comparison }, null, 2)], { type: 'application/json' }));
      const a = document.createElement('a'); a.href = url; a.download = 'width-ratio-cost-comparison.json'; a.click(); setTimeout(() => URL.revokeObjectURL(url), 1000);
    };
  }
  result.onclick = event => {
    const button = (event.target as Element).closest<HTMLButtonElement>('[data-use-cost]');
    if (!button || host.busy() || !snapshot || !comparison) return;
    const trial = comparison.trials[Number(button.dataset.useCost)];
    if (!trial?.feasible || !trial.final_evaluated || !trial.plan?.complete) return;
    const job = structuredClone(snapshot.order.job); job.fabric.width = trial.width_mm;
    host.use(structuredClone(trial.plan), job, snapshot.options.end_allowance_mm ?? 0);
  };
}
