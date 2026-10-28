import type { CutOrder, CutPlan, PlanOptions, RollRegister, SpreadingPlan } from '@beta/marker-wasm';
/** Wire contract for beta_marker_plan::cost. Calculations run only in Rust. */
export interface WidthOffer {
  id: string; width_mm: number;
  price: { unit: 'metre'; price: number } | { unit: 'kilogram'; price: number; gsm: number };
  lay_cost: number; purchase_wastage_pct: number; end_bit_charge_fraction: number;
  rolls?: RollRegister | null; eligible_roll_ids?: string[];
}
export interface CostComparisonRequest {
  order: CutOrder; currency: string; offers: WidthOffer[];
  ratio_sets: { id: string; ratios: Record<string, number>[] }[];
  options: PlanOptions; finalists: number; final_time_s: number; allocation: Record<string, number>;
}
export interface OrderCost {
  purchased_m: number; consumed_m: number; net_marker_m: number; end_allowance_m: number;
  recoverable_end_bits_m: number; charged_end_bits_m: number; purchase_wastage_m: number;
  fabric_cost: number; lay_cost: number; total: number; per_ordered_garment: number;
}
export interface CostTrial {
  offer_index: number; offer_id: string; ratio_set: string; width_mm: number;
  final_evaluated: boolean; feasible: boolean; plan: CutPlan | null;
  spread: SpreadingPlan | null; cost: OrderCost | null; error: string | null;
}
export interface CostComparison { currency: string; trials: CostTrial[]; chosen: number | null; reason: string }
