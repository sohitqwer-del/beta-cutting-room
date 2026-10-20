import { describe,it,expect } from 'vitest';
import { FloorStore,emptyFloor,importFormat,scannerPayload } from './floor-state';
import { languages,label } from './floor-i18n';
import type { FloorCore,FloorState,FloorResponse } from './floor-types';

function core(change:(s:FloorState,op:string)=>FloorState):FloorCore {
  return {request:<T>(op:string,s:FloorState)=>({state:change(structuredClone(s),op),data:null as T,download:null} satisfies FloorResponse<T>),preview:()=>{throw new Error('not needed');}};
}
describe('Floor accepted-state persistence',()=>{
  it('keeps the accepted snapshot when storage fails',()=>{
    const store=new FloorStore(core(s=>{s.orders.push({external_id:'X',purchase_order:'PO',buyer:'B',style:'TEE',colour:'Blue',size:'M',quantity:1,due_date:''});return s;}),()=>{throw new Error('quota');});
    expect(()=>store.apply('import')).toThrow('quota');expect(store.get()).toEqual(emptyFloor());
  });
  it('preserves the old state when Rust rejects a transition',()=>{
    const store=new FloorStore(core(()=>{throw new Error('invalid status');}),()=>{});
    expect(()=>store.apply('advance')).toThrow('invalid status');expect(store.get().lays).toHaveLength(0);
  });
  it('does not overwrite records when a backup fails validation',()=>{
    let writes=0;const store=new FloorStore(core((s)=>{if(s.version!==1)throw new Error('version');return s;}),()=>{writes++;});
    expect(()=>store.restore({version:2})).toThrow('version');expect(writes).toBe(0);expect(store.get()).toEqual(emptyFloor());
  });
  it('returns isolated snapshots and leaves exports read only',()=>{
    let writes=0;const store=new FloorStore(core(s=>s),()=>{writes++;});store.restore(emptyFloor());
    const copy=store.get();copy.orders.push({external_id:'X'} as never);expect(store.get().orders).toHaveLength(0);
    store.read('export');expect(writes).toBe(1);
  });
});
describe('Floor file and scanner entry',()=>{
  it('accepts only bounded CSV/XLSX choices',()=>{expect(importFormat('PO.XLSX',100)).toBe('xlsx');expect(importFormat('rolls.csv',1)).toBe('csv');expect(()=>importFormat('old.xls',5)).toThrow();expect(()=>importFormat('large.csv',5_000_001)).toThrow();});
  it('removes scanner terminators without changing JSON identifiers',()=>{expect(scannerPayload('  {"o":"PO  01","h":"शेड"}\r\n')).toBe('{"o":"PO  01","h":"शेड"}');});
});
describe('Draft floor language catalogues',()=>{
  it('has the same nonempty keys in English, Hindi and Tamil',()=>{for(const locale of [languages.hi,languages.ta]){expect(Object.keys(locale).sort()).toEqual(Object.keys(languages.en).sort());expect(Object.values(locale).every(v=>v.trim())).toBe(true);expect(locale._status).toContain('draft');}});
  it('keeps the review disclosure in each catalogue',()=>{expect(label('en','draft')).toContain('machine-translated drafts');expect(label('hi','floor')).not.toBe(label('en','floor'));expect(label('ta','floor')).not.toBe(label('en','floor'));});
});
