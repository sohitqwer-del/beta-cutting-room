/** Run in the local Vite app: (await import('/src/editor-smoke.mjs')).runEditorSmoke().
 * Uses editor forms, file entry, focus and keyboard handlers. No application-state hooks.
 * Capture the canvas and shortcut dialog with the collaborative preview after this run.
 */
import { demoJob, demoResult } from './demo';
import { loadEditorCore } from './editor-core';
const get=s=>{const e=document.querySelector(s);if(!e)throw new Error(`Missing ${s}`);return e;};
const fill=(s,v)=>{get(s).value=String(v);};
const click=s=>get(s).click();
const key=(k,options={})=>{const c=get('#canvas');c.focus();c.dispatchEvent(new KeyboardEvent('keydown',{key:k,bubbles:true,...options}));};
const assert=(ok,label)=>{if(!ok)throw new Error(`Editor smoke: ${label}. ${get('#message').textContent}`);};
const count=()=>document.querySelectorAll('#canvas [data-index]').length;
const wait=async test=>{for(let i=0;i<100;i++){if(test())return;await new Promise(r=>setTimeout(r,50));}throw new Error('Editor smoke wait timed out');};
async function openJson(document){const dt=new DataTransfer();dt.items.add(new File([JSON.stringify(document)],'editor-smoke.json',{type:'application/json'}));get('#open-file').files=dt.files;get('#open-file').dispatchEvent(new Event('change',{bubbles:true}));await new Promise(r=>setTimeout(r,80));}
export async function runEditorSmoke(){
  await wait(()=>get('#wasm-status').textContent.includes('Rust WASM is active'));
  click('[data-screen="0"]');click('#demo');assert(count()===8,'demo loads eight placements');
  fill('#ed-select-field','name');fill('#ed-select-value','Cuff');click('#ed-select');assert(get('#ed-selected').textContent==='2 selected','select by name');
  fill('#ed-step',0.5);key('ArrowRight');assert(get('#ed-selected').textContent==='2 selected','group nudge keeps selection');
  key('ArrowLeft',{altKey:true});assert(get('#message').textContent.includes('polygon contact'),'keyboard polygon bump');
  key('Delete');assert(count()===6&&get('#ed-unplaced-count').textContent==='(2)','remove returns two copies');
  key('z',{ctrlKey:true});assert(count()===8,'one undo restores removed group');
  key('y',{ctrlKey:true});assert(count()===6,'redo removes group');
  click('[data-ed-unplaced="0"]');fill('#ed-to-x',1100);fill('#ed-to-y',900);click('#ed-place-numeric');assert(count()===7,'numeric place with auto bump');
  click('#ed-fill');assert(count()===8,'fill gaps places final copy');
  click('#demo');
  fill('#ed-from-x',0);fill('#ed-from-y',0);fill('#ed-to-x',3);fill('#ed-to-y',4);click('#ed-measure-numeric');assert(get('#ed-measurement').textContent==='5.000 mm','Rust ruler distance');
  fill('#ed-length',1700);click('#ed-set-length');assert(get('#ed-status').textContent.includes('table 1.700 m'),'length updates table efficiency');
  fill('#ed-section-x',1600);click('#ed-section-add');assert(get('#ed-sections').textContent.includes('1600.0 mm'),'section line added');
  click('#ed-section-remove');assert(get('#ed-sections').textContent==='Cut lines: none','section removed');
  click('#demo');fill('#ed-library-name','Editor smoke original');click('#ed-library-save');
  const entries=JSON.parse(localStorage.getItem('beta-marker-library-v1')||'[]');assert(entries.some(e=>e.label==='Editor smoke original'),'finished marker library saves job and layout');
  fill('#ed-library-choice',entries.findLast(e=>e.label==='Editor smoke original').id);
  get('#canvas [data-index="6"]').focus();key('ArrowRight');click('#ed-library-reuse');assert(get('#message').textContent.includes('Piece geometry and rules are unchanged'),'library reuse checks geometry');
  const overlap=structuredClone(demoResult);overlap.placements[7]={...overlap.placements[6],copy:1,translate:[1070,910]};
  const overlapJob=structuredClone(demoJob);overlapJob.fabric.allow_flip=true;
  await openJson(overlapJob);await openJson(overlap);assert(get('#ed-overlap').textContent.includes('24000.000 mm²'),'live exact overlap area');
  get('#canvas [data-index="7"]').focus();fill('#ed-resolve-direction','right');fill('#ed-length',1900);click('#ed-set-length');click('#ed-resolve');assert(get('#ed-overlap').textContent.startsWith('Overlap 0.000'),'resolve by bump');
  click('#demo');get('#canvas').scrollIntoView({block:'center'});click('#fit');
  const result={placed:count(),checks:get('#check-status').textContent,status:get('#ed-status').textContent,shortcuts:document.querySelectorAll('#ed-shortcuts dt').length};
  click('#ed-help');assert(get('#ed-shortcuts').open,'keyboard help opens');get('#ed-shortcuts').close();
  return result;
}
/** Measures the full JSON/WASM apply call, including decode, job checks, geometry and encode.
 * Drawing/DOM and full live inspection are deliberately outside the bump timing.
 */
export async function benchmarkEditorBump(){
  const core=await loadEditorCore();const job={...structuredClone(demoJob),name:'Original 300 rectangles',order:null,spacing:{piece_gap:2,edge_gap:1},fabric:{width:500,nap:'none',tilt_deg:0,allow_flip:false},pieces:[{id:'rect',name:'Rectangle',quantity:300,mirror_quantity:0,rotation:'fixed',buffer:0,outline:[[0,0],[10,0],[10,10],[0,10]]}]};
  const result={...structuredClone(demoResult),job_name:job.name,width:500,length:881,piece_area:30000,efficiency:30000/(500*881),placements:Array.from({length:300},(_,i)=>({piece_id:'rect',copy:i,mirrored:false,rotation_deg:0,translate:[1+(i%30)*30,1+Math.floor(i/30)*30]}))};
  const times=[];for(let i=0;i<110;i++){const start=performance.now();const out=core.apply({job,result},[299],{op:'bump',direction:'left'});const elapsed=performance.now()-start;if(i>=10)times.push(elapsed);assert(Math.abs(out.result.placements[299].translate[0]-853)<.001,'300-piece bump contact');}
  times.sort((a,b)=>a-b);return{pieces:300,runs:times.length,median:times[50],p95:times[95],max:times[99],under20ms:times[95]<20};
}
