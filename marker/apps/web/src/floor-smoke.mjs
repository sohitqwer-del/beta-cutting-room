// One short smoke session. Uses actual form handlers and Rust WASM, no API mocks.
import fixture from '../../../packages/marker-wasm/test/fixtures/cutting-room.json';
import {loadFloorCore,emptyFloor} from './floor-state';
const $=id=>document.getElementById(id);
const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const state=()=>JSON.parse(localStorage.getItem('beta-floor-v1')||'null');
const set=(id,value)=>{$(id).value=String(value);$(id).dispatchEvent(new Event('input',{bubbles:true}));};
async function until(check,label){await pause(0);for(let i=0;i<100;i++){if(check()&&!$('screen-8').hasAttribute('aria-busy'))return;await pause(50);}throw new Error(`${label}: ${$('floor-status')?.textContent}`);}
async function upload(id,name,bytes,type='text/csv'){await pause(0);const file=new File([bytes],name,{type});const transfer=new DataTransfer();transfer.items.add(file);$(id).files=transfer.files;$(id).dispatchEvent(new Event('change',{bubbles:true}));await pause(100);}
function click(id){$(id).click();}
function tab(key){document.querySelector(`[data-floor-page="${key}"]`).click();}
function submit(id){$(id).requestSubmit();}
function assert(condition,message){if(!condition)throw new Error(message);}

export async function runFloorSmoke(resumeIntake=false){
  assert(!state()?.lays?.length && (resumeIntake || !state()?.orders?.length),'Use a clean, dedicated smoke-test origin. Existing floor records were not cleared.');
  document.querySelector('[data-screen="8"]').click();
  await until(()=>!$('floor-status').textContent.includes('Loading')&&!$('floor-status').classList.contains('floor-error'),'floor engine');
  const core=await loadFloorCore();tab('imports');
  const orders=core.request('template',emptyFloor(),{kind:'orders',format:'csv'}).download;
  await upload('floor-import-file','orders.csv',new Uint8Array(orders.bytes));
  await until(()=>!$('floor-import-commit').disabled,'order preview');click('floor-import-commit');
  await until(()=>state()?.orders.length===1,'order commit');click('floor-import-commit');
  assert(state().orders.length===1,'Order replay duplicated a row.');
  set('floor-import-kind','rolls');$('floor-import-kind').dispatchEvent(new Event('change'));
  const rolls=core.request('template',emptyFloor(),{kind:'rolls',format:'xlsx'}).download;
  await upload('floor-import-file','rolls.xlsx',new Uint8Array(rolls.bytes),'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet');
  await until(()=>!$('floor-import-commit').disabled,'Excel roll preview');click('floor-import-commit');
  await until(()=>state()?.register.rolls.length===1,'roll commit');
  click('floor-send-rolls');assert(!document.getElementById('screen-6').hidden,'Roll hand-off failed.');
  document.querySelector('[data-screen="0"]').click();
  await upload('open-file','smoke.job.json',JSON.stringify(fixture.job),'application/json');
  await upload('open-file','smoke.result.json',JSON.stringify(fixture.result),'application/json');
  document.querySelector('[data-screen="8"]').click();tab('imports');click('floor-plan-order');await pause(0);
  assert($('quantity-0').value==='20','Imported garment quantities did not reach Cut plan.');
  document.querySelector('[data-screen="8"]').click();tab('spreading');
  set('floor-lay-id','FLOOR-SMOKE-1');set('floor-order','PO1');set('floor-plies',2);set('floor-end',10);set('floor-date','2026-10-19');
  document.querySelector('[data-floor-roll]').checked=true;submit('floor-prepare-form');
  await until(()=>state()?.lays.length===1,'prepare lay');
  assert(Math.abs(state().lays[0].spread.total_used_m-0.082)<1e-9,'End allowances were counted incorrectly.');
  click('floor-sheet');await pause(200);click('floor-machine-csv');await pause(200);click('floor-tickets');await pause(200);
  tab('tracking');set('floor-issued-0',0.1);set('floor-returned-0',0.018);set('floor-track-operator','Smoke operator');submit('floor-transition-form');
  await until(()=>state().lays[0].status==='spread','spread status');set('floor-track-operator','Smoke operator');submit('floor-transition-form');
  await until(()=>state().lays[0].status==='cut','cut status');
  tab('recuts');set('floor-damage-reason','Synthetic knife-damage test');submit('floor-damage-form');
  await until(()=>state().lays[0].damage.length===1,'damage');click('floor-schedule');
  await until(()=>state().lays[0].recuts.length===1,'schedule replacement');
  assert(state().lays[0].recuts[0].job.pieces[0].quantity===1,'Replacement job has the wrong demand.');
  tab('tracking');set('floor-track-operator','Smoke operator');submit('floor-transition-form');
  await until(()=>state().lays[0].status==='bundled','bundled status');
  tab('sewing');const b=state().lays[0].bundles[0];
  const payload={v:1,s:b.style,o:b.order,z:b.size,c:b.colour,l:b.lay,h:b.shade_group,part:b.piece_id,m:b.stickers[0].piece.mirrored,b:b.number,p:b.plies,q:b.stickers.length};
  set('floor-scan-operator','Smoke operator');set('floor-scan-date','2026-10-19');set('floor-scan',JSON.stringify(payload));submit('floor-scan-form');
  await until(()=>state().issues.length===1,'sewing scan');set('floor-scan',JSON.stringify(payload));submit('floor-scan-form');
  await until(()=>$('floor-scan-feedback').textContent.includes('Already issued'),'duplicate scan');assert(state().issues.length===1,'Repeated scan increased WIP.');
  click('floor-wip');await pause(200);tab('dashboard');set('floor-day','2026-10-19');click('floor-refresh');
  const output={lays:state().lays.length,orders:state().orders.length,rolls:state().register.rolls.length,used_m:state().lays[0].spread.total_used_m,issues:state().issues.length,recuts:state().lays[0].recuts.length,wasmResources:performance.getEntriesByType('resource').filter(r=>r.name.includes('beta_marker_floor')).length,viewport:{width:innerWidth,scrollWidth:document.documentElement.scrollWidth}};
  window.__floorSmoke=output;return output;
}

export async function checkFloorLanguagesAndManual(){
  for(const language of ['hi','ta','en']){set('floor-language',language);$('floor-language').dispatchEvent(new Event('change'));assert(document.querySelector('[data-screen="8"]').lang===language,'Language switch failed.');}
  tab('manual');const doc=$('floor-manual-frame').contentDocument;assert(doc?.querySelectorAll('h2').length>=13,'Printable manual is incomplete.');assert(!doc.querySelector('script'),'Manual must remain self-contained and script free.');
  return {manualSections:doc.querySelectorAll('h2').length,manualTitle:doc.title,languages:['en','hi','ta'],viewport:{width:innerWidth,scrollWidth:document.documentElement.scrollWidth}};
}
