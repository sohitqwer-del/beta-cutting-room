import type { Bundle, Damage, Job, MarkerResult, RollRegister, SpreadingPlan } from '@beta/marker-wasm';
export type FloorLanguage = 'en' | 'hi' | 'ta';
export type LayStatus = 'planned' | 'spread' | 'cut' | 'bundled';
export interface OrderLine { external_id:string; purchase_order:string; buyer:string; style:string; colour:string; size:string; quantity:number; due_date:string }
export interface ActualRoll { roll_id:string; issued_m:number; returned_m:number }
export interface Actuals { plies:number; rolls:ActualRoll[]; note:string }
export interface FloorLay {
  id:string; order:string; colour:string; date:string; table:string; status:LayStatus;
  job:Job; marker:MarkerResult; stock:RollRegister; spread:SpreadingPlan; end_allowance_mm:number;
  bundles:Bundle[]; actuals:Actuals|null; events:{status:LayStatus;date:string;operator:string;note:string}[];
  damage:Damage[]; recuts:{id:string;shade_group:string;colour:string;job:Job;damage_ids:string[]}[];
}
export interface FloorState { version:1;orders:OrderLine[];register:RollRegister;lays:FloorLay[];issues:{event_id:string;date:string;line:string;operator:string;payload:ScanPayload}[] }
export interface ScanPayload {v:1;s:string;o:string;z:string;c:string;l:string;h:string;part:string;m:boolean;b:number;p:[number,number];q:number}
export interface Preview { kind:'orders'|'rolls';headers:string[];rows:string[][];errors:{row:number;column:string;message:string}[];orders:OrderLine[];register:RollRegister }
export interface FloorDownload { name:string;mime:string;bytes:number[] }
export interface FloorResponse<T=unknown> {state:FloorState;data:T;download:FloorDownload|null}
export interface Dashboard {date:string;lays:number;by_status:Record<string,number>;planned_m:number;measured_lays:number;measured_plan_m:number;used_m:number;variance_m:number;returned_m:number;planned_end_bits_m:number;marker_efficiency_pct:number|null;cutting_efficiency_pct:number|null;open_recuts:number;replaced:number;issued_bundles:number;issued_parts:number}
export interface FloorCore {
  request<T=unknown>(op:string,state:FloorState,data?:unknown):FloorResponse<T>;
  preview(bytes:Uint8Array,format:string,kind:string,mapping:Record<string,string>):Preview;
}
