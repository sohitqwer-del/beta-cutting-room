import type { FloorCore, FloorState, FloorResponse } from './floor-types';
export const FLOOR_STORAGE = 'beta-floor-v1';
export const emptyFloor = ():FloorState => ({version:1,orders:[],register:{rolls:[]},lays:[],issues:[]});
/** Domain rules are in Rust. This controller only owns snapshot and persistence order. */
export class FloorStore {
  private current:FloorState = emptyFloor();
  constructor(private core:FloorCore,private persist:(serialized:string)=>void) {}
  get():FloorState {return structuredClone(this.current);}
  restore(value:unknown):void {
    const checked=this.core.request('validate',value as FloorState).state;
    this.persist(JSON.stringify(checked)); this.current=structuredClone(checked);
  }
  apply<T=unknown>(op:string,data?:unknown):FloorResponse<T> {
    const reply=this.core.request<T>(op,this.current,data);
    // A quota/storage failure cannot appear as a saved issue or physical transition.
    this.persist(JSON.stringify(reply.state));this.current=structuredClone(reply.state);return reply;
  }
  read<T=unknown>(op:string,data?:unknown):FloorResponse<T> {return this.core.request<T>(op,this.current,data);}
}
/** Keep scanner Enter inside its form; scanners type the same payload as a keyboard. */
export function scannerPayload(input:string):string { return input.trim(); }
/** File choice is bounded before copying bytes into WASM. Rust checks it again. */
export function importFormat(name:string,size:number):'csv'|'xlsx' {
  if(size>5_000_000)throw new Error('Import exceeds 5 MB. Split the file into smaller templates.');
  const ext=name.split('.').at(-1)?.toLowerCase();
  if(ext!=='csv'&&ext!=='xlsx')throw new Error('Choose a UTF-8 CSV or XLSX template.');return ext;
}
export async function loadFloorCore():Promise<FloorCore> {
  const url=new URL('wasm/beta_marker_floor.js',document.baseURI).href;
  const raw=await import(/* @vite-ignore */url) as {default(input:{module_or_path:URL}):Promise<unknown>;floor_request(input:string):string;floor_preview(bytes:Uint8Array,format:string,kind:string,mapping:string):string};
  await raw.default({module_or_path:new URL('./beta_marker_floor_bg.wasm',url)});
  return {request:<T>(op:string,state:FloorState,data?:unknown)=>JSON.parse(raw.floor_request(JSON.stringify({op,state,data}))) as FloorResponse<T>,preview:(bytes,format,kind,mapping)=>JSON.parse(raw.floor_preview(bytes,format,kind,JSON.stringify(mapping))) as ReturnType<FloorCore['preview']>};
}
