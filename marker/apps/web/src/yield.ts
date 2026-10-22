/** What a marker's numbers mean on folded and tube lays (fold7). All numbers come from Rust; nothing is estimated here.
 * - `efficiency` is the occupied footprint: the share of the table the pieces cover, including copies no garment uses.
 * - `useful_yield` is the share of the fabric (both layers on folded/tube) that ends up in garments.
 * - `surplus_copies` counts full copies cut but not used. */
import type { Layout, MarkerResult, ValidationReport } from '@beta/marker-wasm';

export interface MarkerYield {
  /** "yield" on folded/tube lays when Rust reports useful yield; otherwise "efficiency". */
  headline: 'yield' | 'efficiency';
  usefulYield: number | null;
  efficiency: number;
  surplus: number;
}
type Fields = { useful_yield?: number; surplus_copies?: number };
const finite = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);

/** Prefer the validator report (fresh checks), then the result. Older saved results have neither field. */
export function markerYield(layout: Layout | string | undefined, result: Pick<MarkerResult, 'efficiency'> & Fields, report?: (Pick<ValidationReport, 'warnings'> & Fields) | null): MarkerYield {
  const usefulYield = finite(report?.useful_yield) ? report!.useful_yield! : finite(result.useful_yield) ? result.useful_yield! : null;
  const counted = (report?.warnings ?? []).filter(w => w.code === 'surplus_copy').length;
  const surplus = finite(report?.surplus_copies) ? report!.surplus_copies! : finite(result.surplus_copies) ? result.surplus_copies! : counted;
  const twoLayers = !!layout && layout !== 'open';
  return { headline: twoLayers && usefulYield != null ? 'yield' : 'efficiency', usefulYield, efficiency: result.efficiency, surplus };
}

export const YIELD_TIP = 'Fabric that ends up in garments, both layers counted.';
export const FOOTPRINT_TIP = 'Table area the pieces cover, including copies no garment uses.';

export const FOLD_EDGE_PREFIX = 'Fold edge:';
/** Plain words for the engine's "Fold edge:" warning (folded lays whose length is set by the fold, not by search time). */
export const FOLD_EDGE_TEXT = 'This order is limited by the fold length, not by search time. Try more garments per marker (even counts per size), a tube, or an open lay.';
/** The engine's fold-edge warning, if any (shown as detail under the plain words). */
export function foldEdgeWarning(warnings: readonly string[] | undefined): string | null {
  return warnings?.find(w => w.startsWith(FOLD_EDGE_PREFIX)) ?? null;
}
