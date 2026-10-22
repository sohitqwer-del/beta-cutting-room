/** One meaning everywhere (marker/docs/PLIES.md): a ply is one fabric layer; a pass is one length laid from the roll.
 * Open fabric: 1 ply per pass. Folded or tube fabric: 2 plies per pass.
 * Roll, spreading, bundle and Floor records count passes; their JSON fields keep the older name `plies`. */
import { label, languages, type LabelKey } from './floor-i18n';
import type { FloorLanguage } from './floor-types';
export const PASS_TIP = '1 pass = one length from the roll; folded or tube = 2 plies.';

/** "40 plies · 20 passes"; one number when they are the same (open fabric, or a v1 plan without passes). */
export function pliesAndPasses(plies: number, passes?: number | null): string {
  const ply = `${plies} ${plies === 1 ? 'ply' : 'plies'}`;
  return passes == null || passes === plies ? ply : `${ply} · ${passes} ${passes === 1 ? 'pass' : 'passes'}`;
}

/** English table headers of the Rolls, Bundles and Floor tables and their label keys (Hindi and Tamil drafts in locales/). */
const HEAD_KEYS: Record<string, LabelKey> = {'Roll': 'th_roll', 'Lot / dye lot': 'th_lot_dye', 'Shade': 'th_shade', 'Ticket / measured (m)': 'th_ticket_measured', 'Width (mm)': 'th_width_mm_paren', 'Shrinkage L / W (%)': 'th_shrinkage', 'Edit': 'th_edit', 'Raw / capped': 'th_raw_capped', 'Per 100 yd2': 'th_per_100_yd2', 'Per 100 m2': 'th_per_100_m2', 'Result': 'th_result', 'Lot / dye lot / shade': 'th_lot_dye_shade', 'Rolls': 'th_rolls', 'Mean shrinkage L / W (%)': 'th_mean_shrinkage', 'Use': 'th_use', 'Lay': 'th_lay', 'Passes': 'th_passes', 'Used (m)': 'th_used_m', 'End bit (m)': 'th_end_bit_m', 'Opened': 'th_opened', 'Remaining (m)': 'th_remaining_m', 'Bundle': 'th_bundle', 'Size': 'th_size', 'Part': 'th_part', 'Hand': 'th_hand', 'Quantity': 'th_quantity', 'Id': 'th_id', 'Order / lay': 'th_order_lay', 'Part / pass': 'th_part_pass', 'Reason': 'th_reason', 'Status': 'th_status', 'Order': 'th_order', 'Colour': 'th_colour', 'Planned m': 'th_planned_m', 'External ID': 'th_external_id', 'PO': 'th_po', 'Buyer': 'th_buyer', 'Style': 'th_style', 'Garments': 'th_garments', 'Due': 'th_due', 'Lot': 'th_lot', 'Dye': 'th_dye', 'Measured m': 'th_measured_m', 'Width mm': 'th_width_mm', 'Date': 'th_date', 'Line': 'th_line', 'Issued m': 'th_issued_m', 'Returned m': 'th_returned_m', 'Operator': 'th_operator', 'Note': 'th_note', 'Damage': 'th_damage', 'Part / ply': 'th_part_ply', 'Job': 'th_job', 'Next step': 'th_next_step', 'Row': 'th_row', 'Column': 'th_column', 'Error': 'th_error'};

/** The language chosen on the Floor screen (it sets `data-language` on the page). */
function pageLanguage(): FloorLanguage {
  const lang = typeof document === 'undefined' ? '' : document.documentElement.dataset.language ?? '';
  return lang in languages ? lang as FloorLanguage : 'en';
}

/** Table header cell in the page language. Known headers carry their key, so a language change relabels them in place;
 * "Passes" gets its tooltip. Unknown headers stay as escaped text. */
export function headCell(head: string, esc: (v: unknown) => string, language: FloorLanguage = pageLanguage()): string {
  const key = HEAD_KEYS[head];
  if (!key) return `<th scope="col">${esc(head)}</th>`;
  const tip = head === 'Passes' ? ` title="${esc(label(language, 'pass_tip'))}" data-floor-title="pass_tip"` : '';
  return `<th scope="col" data-floor-label="${key}"${tip}${language === 'en' ? '' : ` lang="${language}"`}>${esc(label(language, key))}</th>`;
}
