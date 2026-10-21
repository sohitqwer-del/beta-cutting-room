/** Rust model.rs wire contract. Units are mm; angles are CCW degrees, y up. */
export type Pt = [number, number];
export type Json = null | boolean | number | string | Json[] | { [key: string]: Json };
export type Nap = 'none' | 'one_way' | 'one_way_per_bundle';
export type Layout = 'open' | 'folded' | 'tubular';
export interface Zone { id?: string; polygon: Pt[]; note?: string | null }
export type ShadeAxis = 'length' | 'width';
export interface ShadeRule { axis: ShadeAxis; band_mm: number }
export interface Repeat { period_x?: number | null; period_y?: number | null; offset_x?: number; offset_y?: number; tolerance_mm?: number }
export interface Shrinkage { length_pct?: number; width_pct?: number }
export type MatchAxes = 'x' | 'y' | 'both';
export interface PointRef { piece_id: string; point_id: string }
export interface MatchPoint { id: string; point: Pt }
export interface MatchRule { a: PointRef; b?: PointRef | null; axes: MatchAxes; offset?: Pt }
export type RotationRule = 'inherit' | 'fixed' | 'flip180' | 'quarter' | 'free';
export type MarkKind = 'notch' | 'drill' | 'line' | 'text';
export interface Fabric { width: number; max_length?: number | null; nap?: Nap; allow_flip?: boolean; tilt_deg?: number;
  layout?: Layout; defects?: Zone[]; shade?: ShadeRule | null; repeat?: Repeat | null; shrinkage?: Shrinkage }
export interface Spacing { piece_gap?: number; edge_gap?: number }
export interface Mark { kind: MarkKind; points: Pt[]; text?: string | null }
export interface Piece {
  id: string; name: string; style?: string | null; size?: string | null; material?: string | null;
  outline: Pt[]; grain?: [Pt, Pt] | null; quantity?: number; mirror_quantity?: number;
  rotation?: RotationRule; buffer?: number; marks?: Mark[];
  flip?: boolean | null; tilt_deg?: number | null;
  match_points?: MatchPoint[]; fold_line?: [Pt, Pt] | null; block_margin?: number | null;
}
export interface Order { sizes: Record<string, number> }
export interface Job {
  format: string; name: string; fabric: Fabric; spacing?: Spacing; pieces: Piece[];
  order?: Order | null; meta?: Record<string, Json>; match_rules?: MatchRule[];
}
/** Useful copy assigned from a physical fabric layer. */
export interface LayerAllocation { bundle: number; mirrored: boolean }
export interface Placement {
  piece_id: string; copy: number; mirrored: boolean; rotation_deg: number; translate: Pt; locked?: boolean; bundle?: number | null; on_fold?: boolean;
  /** Physical normal then mirrored layer; null is surplus. Empty/omitted uses legacy bundle allocation. */
  layer_allocations?: (LayerAllocation | null)[];
}
export interface MarkerResult {
  format: string; job_name: string; width: number; length: number; piece_area: number;
  efficiency: number; engine: string; seed?: number | null; runtime_s?: number;
  placements: Placement[]; warnings?: string[];
}
export interface Issue { code: string; message: string; piece_ids?: string[] }
/** Common match summary, also accepted from older UI adapters. */
export interface MatchSummary {
  rule: number; garment: number | null; a: Pt | null; b: Pt | null;
  error_mm: number | null; pass: boolean;
}
export interface MatchReport extends MatchSummary {
  garment: number; status: 'matched' | 'off' | 'not_placed';
  a_ref: PointRef; b_ref: PointRef | null;
  a_placement: number | null; b_placement: number | null;
  /** Signed shortest phase residual per checked axis; null if unchecked or missing. */
  offset_mm: [number | null, number | null];
  tolerance_mm: number;
}
export interface ValidationReport { matches?: MatchReport[]; ok: boolean; errors: Issue[]; warnings: Issue[]; length: number; piece_area: number; efficiency: number }
export type Units = 'auto' | 'mm' | 'cm' | 'inch';
/** beta-marker-dxf::ImportReport. */
export interface ImportReport { pieces: Piece[]; sizes: string[]; dialect: string; units: Units; warnings: string[] }
/** beta-marker-nest::NestOptions. Seeds must be safe integers in JavaScript. */
export interface NestOptions { time_limit_s: number; seed?: number | null; workers?: number | null; early_termination?: boolean;
  fixed?: Placement[]; runs?: number; warm_start?: MarkerResult | null }
/** beta-marker-plan wire types. Fabric lengths are mm unless marked _m. */
export interface CutOrder { format: string; name: string; job: Job; quantities: Record<string, number>; max_plies: number; max_garments_per_marker: number; overcut_pct?: number }
export interface Lay { ratio: Record<string, number>; plies: number; marker_length_mm: number; efficiency: number; fabric_m: number; result?: MarkerResult | null }
export interface CutPlan {
  format: string; order_name: string; options: PlanOptions; lays: Lay[];
  total_fabric_m: number; objective_m: number;
  ordered: Record<string, number>; planned: Record<string, number>;
  shortfall: Record<string, number>; overcut: Record<string, number>;
  complete: boolean; optimal_for_evaluated_markers: boolean; markers_evaluated: number;
  runtime_s: number; warnings: string[];
}
export interface PlanOptions {
  time_per_marker_s: number;
  /** Final search seconds per selected ratio; zero skips it. */
  final_time_s?: number;
  end_allowance_mm?: number;
  /** Fabric-equivalent cost of one extra lay, metres. */
  lay_cost_m?: number;
  /** Efficiency guess used before real nests, fraction. */
  estimated_efficiency?: number;
  max_candidates?: number;
  max_refinements?: number;
  search_nodes?: number;
  seed?: number;
}
export interface Metrics { length: number; piece_area: number; efficiency: number }
/** Drawing helpers from Rust; one outline per placement, in the same order. */
export interface MarkerDrawing { outlines: Pt[][]; cut_copies: number }
export interface AllowedRotations { angles: number[]; tilt_deg: number; free: boolean }
export interface Demand { piece_index: number; piece_id: string; mirrored: boolean; count: number }

/** Roll register: roll distances in metres; width and defect dimensions in mm. */
export interface RollDefect { id: string; along_m: number; across_mm: number; length_mm: number; width_mm: number; points: number; defect_type: string; action?: 'cut_around'|'cut_out'|'ignore' }
export interface Roll { id: string; lot: string; dye_lot: string; shade_group: string; ticket_length_m: number; measured_length_m: number; usable_width_mm: number; shrinkage: Shrinkage; defects: RollDefect[] }
export interface RollRegister { rolls: Roll[] }
export interface InspectionSettings { limit: number; unit: 'per100_square_yards'|'per100_square_metres'; point_cap: 'none'|'per_yard'|'per_metre' }
export interface InspectionScore { raw_points: number; scored_points: number; area_m2: number; points_per_100_sq_yd: number; points_per_100_m2: number; passed: boolean }
export interface RollInspection { roll_id: string; lot: string; dye_lot: string; length_difference_m: number; score: InspectionScore }
export interface LotInspection { lot: string; dye_lot: string; roll_count: number; failed_rolls: number; score: InspectionScore }
export interface RollGroup { lot: string; dye_lot: string; shade_group: string; length_band: number; width_band: number; roll_ids: string[]; average_shrinkage: Shrinkage }
export interface InspectRequest { register: RollRegister; settings?: InspectionSettings; grouping?: {band_step_pct: number} }
export interface InspectResponse { inspection: {settings: InspectionSettings; rolls: RollInspection[]; lots: LotInspection[]}; groups: RollGroup[] }
export interface ShadeIdentity { lot: string; dye_lot: string; shade_group: string }
export type ShadePolicy = {mode: 'same_group'|'allow_mixing'} | {mode:'required';identity:ShadeIdentity};
export interface LayRequest { id: string; marker_length_m: number; plies: number; fabric_width_mm: number; shade_rule?: ShadePolicy; eligible_roll_ids?: string[]; allow_splicing?: boolean; splice_zones?: {start_m:number;end_m:number}[] }
export interface AllocationSettings { splice_overlap_m?: number; exact_max_plies?: number; exact_max_rolls?: number; exact_node_limit?: number; heuristic_trials?: number; splice_node_limit?: number }
export interface SpreadRequest { register: RollRegister; lays?: LayRequest[]; markers?: SpreadMarker[]; shade_rule?: ShadePolicy; eligible_roll_ids?: string[]; settings?: AllocationSettings }
export interface RollAllocation { roll_id: string; plies: number; full_plies: number; used_m: number; end_bit_m: number; shade_group: string }
export interface SpreadSegment { roll_id: string; ply: number; roll_start_m: number; marker_start_m: number; length_m: number }
export interface LaySpread { lay: LayRequest; allocated_plies: number; allocations: RollAllocation[]; segments: SpreadSegment[]; splices: {ply:number;from_roll:string;to_roll:string;marker_position_m:number;overlap_m:number}[] }
export interface SpreadingPlan { lays: LaySpread[]; complete: boolean; rolls: {roll_id:string;opened:boolean;used_m:number;remaining_m:number}[]; total_used_m:number;total_end_bits_m:number;splice_overlap_waste_m:number;total_waste_m:number;unopened_stock_m:number;roll_changes:number;warnings:string[];search:{exact_nodes:number;optimal_whole_ply:boolean;heuristic_trials:number} }
export interface CutPart { piece_id:string;size:string;garment:number;slot:number;mirrored:boolean }
export interface PlyRun { first:number;last:number;shade_group:string }
export interface PhysicalLay { id:string;style:string;order:string;colour:string;plies:PlyRun[];parts:CutPart[] }
export interface PhysicalLayRequest { job:Job;result:MarkerResult;id:string;order:string;colour:string;plies:PlyRun[] }
export interface BundleRules {pieces_per_bundle:number;first_bundle:number;sticker_offset:number}
export interface PieceKey {piece_id:string;ply:number;garment:number;slot:number;mirrored:boolean}
export interface Sticker {piece:PieceKey;number:number;last_number:number}
export interface Bundle {lay:string;style:string;order:string;colour:string;size:string;piece_id:string;shade_group:string;number:number;plies:[number,number];stickers:Sticker[]}
export interface BundleRequest {lay:PhysicalLay;rules?:BundleRules}
export interface TicketRequest {bundles:Bundle[];paper?:'a4'|'label100x50'}
export interface Damage {id:string;order:string;lay:string;bundle:number;piece:PieceKey;size:string;reason:string;status:'open'|'scheduled'|'cut'|'replaced';job_id:string|null}
export interface RecutRequest {id:string;bundle:Bundle;piece:PieceKey;reason:string}
export interface LayActuals {lay:string;order:string;width_mm:number;marker_length_mm:number;plies:number;end_allowance_per_ply_mm:number;piece_area_per_ply_mm2:number;pieces_per_ply:number;actual_piece_area_mm2:number;actual_pieces:number;recut_count:number;spreading:{issued_mm:number;end_bits_mm:number}[]}
export interface CuttingReport {scope:string;id:string;order:string;planned_mm:number;actual_mm:number;issued_mm:number;end_bits_mm:number;variance_mm:number;cutting_efficiency_pct:number|null;marker_efficiency_pct:number;planned_pieces:number;pieces_cut:number;recut_count:number;planned_piece_area_mm2:number;marker_fabric_area_mm2:number;actual_fabric_area_mm2:number;actual_piece_area_mm2:number}
export interface CostRequest {job:Job;garments:number;price_per_m:number}
export interface ConsumptionEstimate {marker_length_mm:number;fabric_per_garment_mm:number;expected_efficiency:number;total_piece_area_mm2:number;features:{piece_count:number;mean_piece_area:number;largest_width_ratio:number;convexity_ratio:number;rotation_freedom:number};warnings:string[]}
export interface CostResponse {estimate:ConsumptionEstimate;fabric_cost_per_garment:number;price_per_m:number}
export interface Calibration { homography:[number,number,number,number,number,number,number,number,number];radial:{k1:number;centre:Pt;scale:number};inlier_corners:number;rms_px:number }
export interface DigitizeResponse {pieces:Piece[];report:{detected_markers:number;calibration:Calibration;threshold:number;runtime_s:number;warnings:string[]};overlay_png:string}
/** POST /api/digitize sends raw PNG/JPEG bytes, up to 30 MB. */
export type DigitizeRequest = Blob;
export interface MarkerRequest {job:Job;result:MarkerResult}
export interface NestRequest {job:Job;options:NestOptions}
export interface PlanRequest {order:CutOrder;options:PlanOptions}
export interface ExportOptions {marks?:boolean;labels?:boolean;pdf_tile_mm?:number|null;rotate?:boolean}
export interface ExportRequest extends MarkerRequest {options?:ExportOptions}

export interface MarkerActualsRequest {source:PhysicalLayRequest;issued_mm:number;end_bits_mm:number;actual_piece_area_mm2:number;actual_pieces:number;recut_count:number;end_allowance_per_ply_mm:number}

export interface SpreadMarker {id:string;job:Job;result:MarkerResult;plies:number;end_allowance_mm?:number}

export interface FromSpreadRequest {job:Job;result:MarkerResult;register:RollRegister;spread:SpreadingPlan;lay_id:string;order:string;colour:string}
