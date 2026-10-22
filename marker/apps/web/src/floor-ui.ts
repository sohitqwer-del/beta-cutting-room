import type { CutPlan, Job, MarkerResult, RollRegister } from '@beta/marker-wasm';
import type { Editor } from './features';
import { emptyResult } from './state';
import { downloadFile, isDesktop } from './transport';
import { FloorStore, FLOOR_STORAGE, importFormat, loadFloorCore, scannerPayload } from './floor-state';
import type { Dashboard, FloorCore, FloorLanguage, FloorLay, Preview } from './floor-types';
import { label, translateLabels, type LabelKey } from './floor-i18n';
import manual from '../../../docs/CUTTING-ROOM-MANUAL.html?raw';
import './floor-ui.css';
import { byId, find, findAll, whenIdle } from './screens';
import { headCell, pliesAndPasses } from './plies';

interface FloorHost {
  planning():{plan:CutPlan|undefined;job:Job|undefined;endAllowance:number};
  loadOrder(quantities:Record<string,number>):void;
  getRegister():RollRegister;
  receiveRegister(register:RollRegister):void;
  screen(index:number):void;
}
const esc=(v:unknown):string=>String(v??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const $=<T extends HTMLElement=HTMLElement>(id:string):T=>byId(id) as T;
const value=(id:string):string=>$<HTMLInputElement>(id).value;
const n=(id:string):number=>{const e=$<HTMLInputElement>(id);if(!e.value.trim()||!e.checkValidity()||!Number.isFinite(Number(e.value)))throw new Error(`Check ${e.labels?.[0]?.textContent??id}.`);return Number(e.value);};
const localDate=():string=>{const d=new Date();return `${d.getFullYear()}-${String(d.getMonth()+1).padStart(2,'0')}-${String(d.getDate()).padStart(2,'0')}`;};
const field=(id:string,title:string,v:string|number='',type='text',extra=''):string=>`<label>${title}<input id="${id}" type="${type}" value="${esc(v)}" ${extra}></label>`;
const translated=(key:LabelKey):string=>`<span data-floor-label="${key}">${label('en',key)}</span>`;
const button=(id:string,key:LabelKey,extra=''):string=>`<button id="${id}" type="button" ${extra}>${translated(key)}</button>`;
const table=(caption:string,headers:string[],rows:string[][]):string=>`<div class="floor-scroll"><table><caption>${esc(caption)}</caption><thead><tr>${headers.map(h=>headCell(h,esc)).join('')}</tr></thead><tbody>${rows.map(row=>`<tr>${row.map((c,i)=>`<${i?'td':'th scope="row"'}>${c}</${i?'td':'th'}>`).join('')}</tr>`).join('')}</tbody></table></div>`;

/** New floor tab. Rust owns all production calculations and accepted state changes. */
export function installFloor(editor:Editor,host:FloorHost) {
  let core:FloorCore|undefined,store:FloorStore|undefined,working=false,ready=false;
  let preview:Preview|undefined,importBytes:Uint8Array|undefined,format='csv',importKind='orders';
  let language:FloorLanguage='en',pendingRestore:unknown;
  try {const saved=localStorage.getItem('beta-floor-language');if(saved==='hi'||saved==='ta')language=saved;} catch { /* English is still available. */ }
  find('.screen-tabs')!.insertAdjacentHTML('beforeend','<button data-screen="8" aria-pressed="false" aria-controls="screen-8">Floor</button>');
  find('.workspace')!.insertAdjacentHTML('afterend',`<section class="screen floor-workbench" id="screen-8" hidden aria-labelledby="floor-heading">
  <div class="floor-top"><div><span class="eyebrow">BETA CUTTING ROOM</span><h1 id="floor-heading">${translated('floor')}</h1><p>Receive an order. Release a lay. Record the cut. Issue traceable bundles.</p></div><label>${translated('language')}<select id="floor-language"><option value="en">English</option><option value="hi">हिन्दी · draft</option><option value="ta">தமிழ் · draft</option></select></label></div>
  <p id="floor-draft" class="floor-note" data-floor-label="draft"></p>
  <nav class="floor-nav" aria-label="Floor workflow">${(['dashboard','imports','spreading','tracking','recuts','sewing','manual'] as const).map((key,i)=>`<button type="button" data-floor-page="${key}" aria-controls="floor-page-${key}" aria-pressed="${i===0}">${translated(key)}</button>`).join('')}</nav>
  <p id="floor-status" role="status" aria-live="polite">Loading the local floor engine…</p>
  <div class="floor-backups">${button('floor-backup','save_backup')}<label class="floor-file">${translated('restore')}<input id="floor-restore" type="file" accept=".json"></label></div>
  <section id="floor-page-dashboard" class="floor-page"><div class="floor-actions">${field('floor-day',translated('date'),localDate(),'date')}${button('floor-refresh','refresh')}${button('floor-consumption','download_consumption')}</div><div id="floor-dashboard"></div></section>
  <section id="floor-page-imports" class="floor-page" hidden><h2>${translated('imports')}</h2><p>Use one row per purchase order, style, colour and size. A roll packing list is receiving data; check inspection, shade and shrinkage in Rolls before release.</p>
  <div class="floor-actions"><label>Template<select id="floor-import-kind"><option value="orders">Purchase orders</option><option value="rolls">Roll packing list</option></select></label>${button('floor-template-csv','template_csv')}${button('floor-template-xlsx','template_xlsx')}<label>CSV UTF-8 or XLSX, up to 5 MB<input id="floor-import-file" type="file" accept=".csv,.xlsx"></label>${button('floor-preview','preview')}</div>
  <details><summary>Column mapping</summary><p>Map each canonical column to one source header. Units do not change with the column name.</p><div id="floor-mapping" class="floor-fields"></div></details><div id="floor-preview-result"></div>${button('floor-import-commit','import','disabled')}
  <h3>Imported order lines</h3><div id="floor-orders"></div><div class="floor-actions"><label>PO / style / colour<select id="floor-order-group"></select></label><button id="floor-plan-order" type="button">Send quantities to Cut plan</button></div>
  <h3>Received rolls</h3><div id="floor-stock"></div><div class="floor-actions"><button id="floor-send-rolls" type="button">Open stock in Rolls for inspection</button><button id="floor-use-rolls" type="button">Use inspected Rolls register</button></div></section>
  <section id="floor-page-spreading" class="floor-page" hidden><h2>${translated('spreading')}</h2><p>Prepare only a checked marker. Opened rolls are reserved for this lay. Measured returned ends need new remnant IDs before reuse; the app does not assume a return happened.</p>
  <form id="floor-prepare-form"><div class="floor-fields"><label>Marker source<select id="floor-source"><option value="editor">Current Editor marker</option></select></label>${field('floor-lay-id',translated('lay'),'LAY-01','text','required')}${field('floor-order',translated('order'),'ORDER-01','text','required')}${field('floor-colour',translated('colour'),'Blue','text','required')}${field('floor-date',translated('date'),localDate(),'date','required')}${field('floor-table','Cutting table','T1','text','required')}${field('floor-plies',`<span title="${esc(label(language,'pass_tip'))}" data-floor-title="pass_tip">${translated('passes')}</span>`,2,'number','min="1" max="10000" step="1" required')}${field('floor-end','Allowance at each end (mm)',10,'number','min="0" step="any" required')}${field('floor-bundle-size','Pieces per bundle',20,'number','min="1" max="10000" step="1" required')}</div>
  <p class="hint" data-floor-label="pass_tip">${label('en','pass_tip')}</p>
  <fieldset><legend>Inspected rolls available to this lay</legend><div id="floor-roll-choices"></div></fieldset>
  <details><summary>Optional safe splice zones</summary><p>Enter table coordinates in metres, including head allowance. A full-width zone must clear all cut pieces. Blank means whole passes only.</p><div class="floor-fields">${field('floor-splice-start','Safe zone start (m)','','number','min="0" step="any"')}${field('floor-splice-end','Safe zone end (m)','','number','min="0" step="any"')}${field('floor-overlap','Splice overlap (m)',0.1,'number','min="0" step="any"')}</div></details><button id="floor-prepare" class="primary" type="submit">${translated('prepare')}</button></form>
  <h3>Release files</h3><label>Saved lay<select id="floor-sheet-lay"></select></label><div id="floor-sheet-summary"></div><div class="floor-actions">${button('floor-sheet','sheet')}<button id="floor-machine-csv">Spreader CSV</button><button id="floor-machine-json">Spreader JSON</button><button id="floor-sticker-order">Ply sticker order CSV</button>${button('floor-tickets','tickets')}${button('floor-stickers','stickers')}</div><p class="floor-note">Spreader files are generic instructions. A factory-approved adapter and dry run are required for machine use. No speed or tension commands are supplied.</p></section>
  <section id="floor-page-tracking" class="floor-page" hidden><h2>${translated('tracking')}</h2><p>Planned → spread → cut → bundled. Record measured issue and reusable returns at spread completion. The issued quantity includes returned ends; do not subtract them twice.</p><label>Lay<select id="floor-track-lay"></select></label><div id="floor-tracking"></div></section>
  <section id="floor-page-recuts" class="floor-page" hidden><h2>${translated('recuts')}</h2><p>Record an exact damaged part after cutting. Replacement jobs keep its original lot, dye lot and shade identity. Open a replacement job in Editor, nest and validate it, then prepare a separate floor lay using matching stock.</p><label>Original lay<select id="floor-recut-lay"></select></label><div id="floor-recut-content"></div></section>
  <section id="floor-page-sewing" class="floor-page" hidden><h2>${translated('sewing')}</h2><p>Scan the whole-bundle QR with a keyboard or USB scanner, then press Enter. The bundle must be registered and bundled. An identical scan on the same line is a no-op; another line is a conflict.</p><form id="floor-scan-form"><div class="floor-fields">${field('floor-line',translated('line'),'LINE-1','text','required')}${field('floor-scan-date',translated('date'),localDate(),'date','required')}${field('floor-scan-operator',translated('operator'),'','text','required')}</div><label>Bundle QR payload<input id="floor-scan" type="text" autocomplete="off" spellcheck="false" required placeholder='Scan the JSON payload from a bundle ticket'></label><button type="submit" id="floor-issue" class="primary">${translated('issue')}</button></form><p id="floor-scan-feedback" role="status" aria-live="polite"></p>${button('floor-wip','download_wip')}<p class="floor-note">WIP means work in progress. This CSV records issued parts, not complete garments, sewing receipt, or an accounting posting. BETA Time import mapping is documented in the manual.</p><div id="floor-issues"></div></section>
  <section id="floor-page-manual" class="floor-page" hidden><h2>${translated('manual')}</h2><div class="floor-actions">${button('floor-manual-print','print')}<button id="floor-manual-save">Download self-contained HTML</button></div><iframe id="floor-manual-frame" title="BETA Cutting Room printable user manual"></iframe></section>
  </section>`);
  $<HTMLIFrameElement>('floor-manual-frame').srcdoc=manual;
  $<HTMLSelectElement>('floor-language').value=language;translateLabels(language);
  function say(text:string,error=false):void {$('floor-status').textContent=text;$('floor-status').classList.toggle('floor-error',error);}
  function page(key:string):void {findAll<HTMLElement>('.floor-page').forEach(e=>e.hidden=e.id!==`floor-page-${key}`);findAll<HTMLButtonElement>('[data-floor-page]').forEach(e=>e.setAttribute('aria-pressed',String(e.dataset.floorPage===key)));if(key==='sewing')$('floor-scan').focus();}
  findAll<HTMLButtonElement>('[data-floor-page]').forEach(e=>e.onclick=()=>page(e.dataset.floorPage!));
  $<HTMLSelectElement>('floor-language').onchange=()=>{language=value('floor-language') as FloorLanguage;translateLabels(language);try{localStorage.setItem('beta-floor-language',language);}catch{say('Language is active for this session; browser settings could not be saved.',true);}};
  async function task(action:()=>Promise<void>|void):Promise<void> {
    if(working)return;if(!store||!core){say('Floor engine is not ready. Build the floor WASM package, then reload.',true);return;}
    working=true;$('screen-8').setAttribute('aria-busy','true');say('Working locally…');
    try{await action();}catch(e){say(e instanceof Error?e.message:String(e),true);}finally{working=false;$('screen-8').removeAttribute('aria-busy');}
  }
  const click=(id:string,action:()=>Promise<void>|void):void=>{$(id).onclick=()=>void task(action);};
  async function exportFile(kind:string,lay?:string):Promise<void> {const r=store!.read('export',{kind,lay});if(!r.download)throw new Error('Export returned no file.');await downloadFile(new Blob([new Uint8Array(r.download.bytes)],{type:r.download.mime}),r.download.name);say('File prepared from the saved floor records.');}
  function selectOptions(id:string,items:{value:string;label:string}[]):void {const select=$<HTMLSelectElement>(id),old=select.value;select.innerHTML=items.map(o=>`<option value="${esc(o.value)}">${esc(o.label)}</option>`).join('');if(items.some(o=>o.value===old))select.value=old;}
  const savedLay=(select:string):FloorLay|undefined=>store?.get().lays.find(l=>l.id===value(select));
  function renderDashboard():void {
    if(!store)return;const d=store.read<Dashboard>('dashboard',{date:value('floor-day')}).data;
    const metric=(title:string,v:string,detail:string):string=>`<article><span>${esc(title)}</span><strong>${esc(v)}</strong><small>${esc(detail)}</small></article>`;
    $('floor-dashboard').innerHTML=`<div class="floor-metrics">${metric('Today’s lays',String(d.lays),Object.entries(d.by_status).map(([k,v])=>`${v} ${k}`).join(' · '))}${metric('Planned fabric',`${d.planned_m.toFixed(3)} m`,'All lays scheduled on this date')}${metric('Measured fabric used',`${d.used_m.toFixed(3)} m`,`${d.measured_lays} measured lays; comparable plan ${d.measured_plan_m.toFixed(3)} m`)}${metric('Fabric variance',`${d.variance_m.toFixed(3)} m`,'Measured use − plan for measured lays only')}${metric('Marker efficiency',d.marker_efficiency_pct==null?'—':`${d.marker_efficiency_pct.toFixed(1)}%`,'Area weighted; excludes end and splice loss')}${metric('Cutting yield',d.cutting_efficiency_pct==null?'—':`${d.cutting_efficiency_pct.toFixed(1)}%`,'Geometric cut area / actual roll area; includes damaged parts')}${metric('Returned end bits',`${d.returned_m.toFixed(3)} m`,`Planned opened-roll remainder ${d.planned_end_bits_m.toFixed(3)} m`)}${metric('Re-cuts pending',String(d.open_recuts),`${d.replaced} replaced; original lays scheduled today`)}${metric('Issued to sewing',`${d.issued_bundles} bundles`,`${d.issued_parts} physical parts issued on this date`)}</div>`;
    const rows=store.get().lays.filter(l=>l.date===d.date).map(l=>[esc(l.id),esc(l.order),esc(l.colour),esc(label(language,l.status)),String(l.spread.lays[0]?.allocated_plies??0),l.spread.total_used_m.toFixed(3)]);
    $('floor-dashboard').insertAdjacentHTML('beforeend',rows.length?table('Lays for the selected date',['Lay','Order','Colour','Status','Passes','Planned m'],rows):`<p>${esc(label(language,'no_lays'))}</p>`);
  }
  function renderLists():void {
    if(!store)return;const s=store.get();
    $('floor-orders').innerHTML=table('Purchase order lines',['External ID','PO','Buyer','Style','Colour','Size','Garments','Due'],s.orders.map(o=>[o.external_id,o.purchase_order,o.buyer,o.style,o.colour,o.size,String(o.quantity),o.due_date].map(esc)));
    const groups=[...new Set(s.orders.map(o=>JSON.stringify([o.purchase_order,o.style,o.colour])))];selectOptions('floor-order-group',groups.map(k=>({value:k,label:(JSON.parse(k) as string[]).join(' / ')})));
    $('floor-stock').innerHTML=table('Received stock',['Roll','Lot','Dye','Shade','Measured m','Width mm'],s.register.rolls.map(r=>[r.id,r.lot,r.dye_lot,r.shade_group,String(r.measured_length_m),String(r.usable_width_mm)].map(esc)));
    const checked=new Set(Array.from(findAll<HTMLInputElement>('[data-floor-roll]:checked'),e=>e.value));
    const reserved=new Set(s.lays.flatMap(l=>l.spread.rolls.filter(r=>r.opened).map(r=>r.roll_id)));
    $('floor-roll-choices').innerHTML=s.register.rolls.map(r=>`<label class="floor-check"><input type="checkbox" data-floor-roll value="${esc(r.id)}" ${reserved.has(r.id)?'disabled':checked.has(r.id)?'checked':''}>${esc(r.id)} · ${esc(r.lot)} / ${esc(r.dye_lot)} / ${esc(r.shade_group)} · ${r.measured_length_m} m × ${r.usable_width_mm} mm ${reserved.has(r.id)?' (reserved)':''}</label>`).join('')||'<p>Import rolls, then inspect them in Rolls.</p>';
    for(const id of ['floor-sheet-lay','floor-track-lay','floor-recut-lay'])selectOptions(id,s.lays.map(l=>({value:l.id,label:`${l.id} / ${l.order} / ${l.status}`})));
    $('floor-issues').innerHTML=table('Bundle issue history',['Date','Line','Order','Lay','Bundle','Part','Size','Quantity'],s.issues.map(i=>[i.date,i.line,i.payload.o,i.payload.l,String(i.payload.b),i.payload.part,i.payload.z,String(i.payload.q)].map(esc)));
    renderSources();renderSheet();renderTracking();renderRecuts();renderDashboard();translateLabels(language);
  }
  function renderSources():void {const p=host.planning();selectOptions('floor-source',[{value:'editor',label:'Current Editor marker'},...(p.plan?.lays??[]).flatMap((l,i)=>l.result?[{value:String(i),label:`Cut plan lay ${i+1} · ${pliesAndPasses(l.plies,l.passes)}`}]:[])]);}
  function source():{job:Job;marker:MarkerResult;plies:number;end:number} {
    const index=value('floor-source');if(index==='editor'){const s=editor.get();return {job:s.job,marker:s.result,plies:n('floor-plies'),end:n('floor-end')};}
    const p=host.planning(),l=p.plan?.lays[Number(index)];if(!l?.result||!p.job)throw new Error('Cut plan source is no longer available.');
    return {job:{...structuredClone(p.job),order:{sizes:l.ratio}},marker:l.result,plies:l.passes??l.plies,end:p.endAllowance};
  }
  $('floor-source').onchange=()=>{try{const s=source();$<HTMLInputElement>('floor-plies').value=String(s.plies);$<HTMLInputElement>('floor-end').value=String(s.end);}catch(e){say(String(e),true);}};
  function renderSheet():void {const l=savedLay('floor-sheet-lay');$('floor-sheet-summary').innerHTML=l?`<p><b>${esc(l.id)}</b> · ${l.spread.lays[0]?.allocated_plies} passes · ${l.spread.total_used_m.toFixed(3)} m planned · ${l.bundles.length} bundles. Print at 100% scale. Files retain the saved marker and stock identities.</p>`:'<p>Prepare a lay to create its release files.</p>';}
  $('floor-sheet-lay').onchange=renderSheet;
  function renderTracking():void {
    const l=savedLay('floor-track-lay');if(!l){$('floor-tracking').innerHTML='<p>No saved lays.</p>';return;}
    const next=({planned:'spread',spread:'cut',cut:'bundled',bundled:null} as const)[l.status];
    $('floor-tracking').innerHTML=`<p>Current state: <b>${esc(label(language,l.status))}</b></p>${l.status==='planned'?`<div class="floor-fields">${field('floor-actual-plies',`<span title="${esc(label(language,'pass_tip'))}" data-floor-title="pass_tip">${translated('actual_passes')}</span>`,l.spread.lays[0]!.allocated_plies,'number','min="1" max="10000" step="1" required')}</div><p>Enter measured values. Use zero only for a verified zero return. Fewer passes require a measurement note and new bundle/sticker prints. Extra passes require replanning.</p>${l.spread.rolls.filter(r=>r.opened).map((r,i)=>`<fieldset data-actual-roll="${esc(r.roll_id)}"><legend>${esc(r.roll_id)} (planned use ${r.used_m.toFixed(3)} m)</legend><div class="floor-fields">${field(`floor-issued-${i}`,'Measured issued length (m)','','number','min="0" step="any" required')}${field(`floor-returned-${i}`,'Reusable end bits returned (m)','','number','min="0" step="any" required')}</div></fieldset>`).join('')}`:table('Frozen spread measurements',['Roll','Issued m','Returned m'],l.actuals?.rolls.map(r=>[esc(r.roll_id),String(r.issued_m),String(r.returned_m)])??[])}
    ${next?`<form id="floor-transition-form"><div class="floor-fields">${field('floor-track-date',translated('date'),localDate(),'date','required')}${field('floor-track-operator',translated('operator'),'','text','required')}${field('floor-track-note','Note')}</div><button class="primary" type="submit">Record ${esc(label(language,next))}</button></form>`:'<p>Ready for bundle issue to sewing.</p>'}${table('Status history',['Date','Status','Operator','Note'],l.events.map(e=>[e.date,e.status,e.operator,e.note].map(esc)))}`;
    if(next)$('floor-transition-form').onsubmit=e=>{e.preventDefault();void task(()=>{const actuals=l.status==='planned'?{plies:n('floor-actual-plies'),rolls:Array.from(findAll<HTMLElement>('[data-actual-roll]'),(e,i)=>({roll_id:e.dataset.actualRoll!,issued_m:n(`floor-issued-${i}`),returned_m:n(`floor-returned-${i}`)})),note:value('floor-track-note')}:null;store!.apply('advance',{id:l.id,status:next,actuals,date:value('floor-track-date'),operator:value('floor-track-operator'),note:value('floor-track-note')});renderLists();say(`${l.id}: ${next} recorded. If actual plies were reduced, print the new tickets and stickers before bundling.`);});};
  }
  $('floor-track-lay').onchange=renderTracking;
  function renderRecuts():void {
    const l=savedLay('floor-recut-lay');if(!l){$('floor-recut-content').innerHTML='<p>No saved lays.</p>';return;}
    const pieces=l.bundles.flatMap(b=>b.stickers.map(s=>({bundle:b.number,piece:s.piece,label:`Bundle ${b.number} · ${b.size} · ${b.piece_id} · ply ${s.piece.ply} · garment ${s.number}${s.piece.mirrored?' · mirrored':''}`})));
    $('floor-recut-content').innerHTML=`<form id="floor-damage-form"><label>Exact physical piece<select id="floor-damage-piece">${pieces.map((p,i)=>`<option value="${i}">${esc(p.label)}</option>`).join('')}</select></label>${field('floor-damage-reason','Damage reason','','text','required')}<button type="submit">${translated('record_damage')}</button></form><div class="floor-actions">${field('floor-recut-batch','New re-cut batch id',`RC-${l.id}-${l.recuts.length+1}`)}${button('floor-schedule','schedule')}<button id="floor-recut-export">Export replacement jobs + ledger</button></div>
    ${table('Damage and replacement ledger',['Damage','Part / ply','Reason','Status','Job','Next step'],l.damage.map(d=>[esc(d.id),`${esc(d.piece.piece_id)} / ${d.piece.ply}`,esc(d.reason),esc(d.status),esc(d.job_id),d.status==='scheduled'||d.status==='cut'?`<button type="button" data-damage-next="${esc(d.id)}" data-next-status="${d.status==='scheduled'?'cut':'replaced'}">Mark ${d.status==='scheduled'?'cut':'replaced'}</button>`:'—']))}<div class="floor-actions">${l.recuts.map((r,i)=>`<button type="button" data-open-recut="${i}">Open ${esc(r.id)} in Editor</button>`).join('')}</div>`;
    $('floor-damage-form').onsubmit=e=>{e.preventDefault();void task(()=>{const p=pieces[Number(value('floor-damage-piece'))];if(!p)throw new Error('Choose a physical piece.');store!.apply('damage',{lay:l.id,id:`D-${crypto.randomUUID()}`,bundle:p.bundle,piece:p.piece,reason:value('floor-damage-reason')});renderLists();say('Damage recorded for the exact physical piece.');});};
    click('floor-schedule',()=>{store!.apply('schedule',{lay:l.id,batch:value('floor-recut-batch')});renderLists();say('Replacement jobs scheduled. Nest and validate each job before cutting.');});
    click('floor-recut-export',()=>exportFile('recuts',l.id));
    findAll<HTMLButtonElement>('[data-damage-next]').forEach(b=>b.onclick=()=>void task(()=>{store!.apply('recut_advance',{lay:l.id,id:b.dataset.damageNext,status:b.dataset.nextStatus});renderLists();say('Replacement status recorded.');}));
    findAll<HTMLButtonElement>('[data-open-recut]').forEach(b=>b.onclick=()=>void task(()=>{const r=l.recuts[Number(b.dataset.openRecut)];if(!r)return;editor.open({job:r.job,result:emptyResult(r.job)});host.screen(0);editor.message(`Replacement ${r.id}: use stock from ${r.shade_group}. Nest and validate before cutting.`);}));
  }
  $('floor-recut-lay').onchange=renderRecuts;
  function mapping():Record<string,string> {return Object.fromEntries(Array.from(findAll<HTMLSelectElement>('[data-floor-map]'),e=>[e.dataset.floorMap!,e.value]));}
  const columns={orders:['schema_version','external_id','purchase_order','buyer','style','colour','size','quantity','due_date'],rolls:['schema_version','id','lot','dye_lot','shade_group','ticket_length_m','measured_length_m','usable_width_mm','length_pct','width_pct']};
  function showPreview():void {
    if(!core||!importBytes)throw new Error('Choose a CSV or XLSX file first.');
    importKind=value('floor-import-kind');preview=core.preview(importBytes,format,importKind,mapping());
    const previous=mapping();$('floor-mapping').innerHTML=columns[importKind as keyof typeof columns].map(k=>`<label>${esc(k)}<select data-floor-map="${esc(k)}">${preview!.headers.map(h=>`<option value="${esc(h)}" ${(previous[k]??k)===h?'selected':''}>${esc(h)}</option>`).join('')}</select></label>`).join('');
    $('floor-preview-result').innerHTML=`<p>${preview.rows.length} data rows. ${preview.errors.length} errors. Showing the first 30 rows.</p>${table('Import preview',preview.headers,preview.rows.slice(0,30).map(r=>r.map(esc)))}${preview.errors.length?table('Fix these row errors',['Row','Column','Error'],preview.errors.map(e=>[String(e.row),esc(e.column),esc(e.message)])):'<p>Rows pass validation. Check identifiers, sizes, colours and units before importing.</p>'}`;
    $<HTMLButtonElement>('floor-import-commit').disabled=preview.errors.length>0;
    findAll<HTMLSelectElement>('[data-floor-map]').forEach(e=>e.onchange=()=>{$<HTMLButtonElement>('floor-import-commit').disabled=true;preview=undefined;say('Column mapping changed. Preview the file again.');});
  }
  $('floor-import-kind').onchange=()=>{preview=undefined;importBytes=undefined;$<HTMLInputElement>('floor-import-file').value='';$('floor-mapping').innerHTML='';$('floor-preview-result').innerHTML='';$<HTMLButtonElement>('floor-import-commit').disabled=true;};
  $('floor-import-file').onchange=()=>void task(async()=>{const file=$<HTMLInputElement>('floor-import-file').files?.[0];if(!file)return;format=importFormat(file.name,file.size);importBytes=new Uint8Array(await file.arrayBuffer());$('floor-mapping').innerHTML='';showPreview();say('Preview ready. No records have been imported.');});
  click('floor-preview',()=>{showPreview();say('Preview updated.');});
  click('floor-import-commit',()=>{if(!preview||preview.errors.length||!importBytes)throw new Error('Preview this file and fix its errors first.');const r=store!.apply<{added:number}>('import',{kind:importKind,format,bytes:Array.from(importBytes),mapping:mapping()});renderLists();say(`${r.data.added} new records imported. Identical records were kept once.`);});
  for(const ext of ['csv','xlsx'])click(`floor-template-${ext}`,async()=>{const r=store!.read('template',{kind:value('floor-import-kind'),format:ext});const d=r.download!;await downloadFile(new Blob([new Uint8Array(d.bytes)],{type:d.mime}),d.name);say('Template ready. Replace the sample row before importing.');});
  click('floor-plan-order',()=>{const group=JSON.parse(value('floor-order-group')) as [string,string,string];const [order,style,colour]=group;const r=store!.read<Record<string,number>>('quantities',{order,style,colour,job:editor.get().job});host.loadOrder(r.data);$<HTMLInputElement>('floor-order').value=order;$<HTMLInputElement>('floor-colour').value=colour;host.screen(1);editor.message(`Loaded ${order} / ${style} / ${colour}. Review planner limits, then make a cut plan.`);});
  click('floor-send-rolls',()=>{host.receiveRegister(store!.get().register);host.screen(6);editor.message('Received rolls loaded. Review measurements, defects, shade and shrinkage, then inspect.');});
  click('floor-use-rolls',()=>{store!.apply('sync_register',host.getRegister());renderLists();say('Rolls register copied. The floor checks inspection, shrinkage and consumed defects again on release.');});
  $('floor-prepare-form').onsubmit=e=>{e.preventDefault();void task(()=>{const s=source();const start=value('floor-splice-start'),end=value('floor-splice-end');if(Boolean(start)!==Boolean(end))throw new Error('Enter both ends of the safe splice zone.');store!.apply('prepare',{id:value('floor-lay-id'),order:value('floor-order'),colour:value('floor-colour'),date:value('floor-date'),table:value('floor-table'),job:s.job,marker:s.marker,plies:n('floor-plies'),end_allowance_mm:n('floor-end'),roll_ids:Array.from(findAll<HTMLInputElement>('[data-floor-roll]:checked'),e=>e.value),splice_zones:start?[{start_m:n('floor-splice-start'),end_m:n('floor-splice-end')}]:[],splice_overlap_m:n('floor-overlap'),pieces_per_bundle:n('floor-bundle-size')});const id=value('floor-lay-id');renderLists();for(const s of ['floor-sheet-lay','floor-track-lay','floor-recut-lay'])$<HTMLSelectElement>(s).value=id;renderSheet();renderTracking();renderRecuts();say(`${id} prepared. Review the lay sheet before spreading.`);});};
  for(const [id,kind] of [['floor-sheet','lay_sheet'],['floor-machine-csv','machine_csv'],['floor-machine-json','machine_json'],['floor-sticker-order','sticker_order'],['floor-tickets','tickets'],['floor-stickers','stickers']])click(id!,()=>exportFile(kind!,value('floor-sheet-lay')));
  click('floor-refresh',()=>{renderLists();say('Dashboard refreshed.');});$('floor-day').onchange=()=>void task(()=>{renderDashboard();say('Date filter updated.');});
  click('floor-consumption',()=>exportFile('consumption'));click('floor-wip',()=>exportFile('wip'));click('floor-backup',()=>exportFile('backup'));
  $('floor-scan-form').onsubmit=e=>{e.preventDefault();void task(()=>{const r=store!.apply<{added:boolean}>('issue',{payload:scannerPayload(value('floor-scan')),event_id:`ISSUE-${crypto.randomUUID()}`,date:value('floor-scan-date'),line:value('floor-line'),operator:value('floor-scan-operator')});$('floor-scan-feedback').textContent=r.data.added?'Bundle issued. Scan the next ticket.':'Already issued to this line. No duplicate quantity was recorded.';$<HTMLInputElement>('floor-scan').value='';renderLists();$('floor-scan').focus();say('Sewing issue checked and saved.');});};
  $('floor-restore').onchange=()=>{void (async()=>{if(working)return;if(!core)throw new Error('Wait for the floor engine before restoring a backup.');const file=$<HTMLInputElement>('floor-restore').files?.[0];if(!file)return;if(file.size>50_000_000)throw new Error('Backup exceeds 50 MB.');const candidate=makeStore(core);candidate.restore(JSON.parse(await file.text()));store=candidate;ready=true;renderLists();say('Floor backup restored and checked.');})().catch(e=>say(String(e),true));};
  $('floor-manual-print').onclick=()=>{$<HTMLIFrameElement>('floor-manual-frame').contentWindow?.print();};
  $('floor-manual-save').onclick=()=>void downloadFile(new Blob([manual],{type:'text/html;charset=utf-8'}),'BETA-Cutting-Room-Manual.html').catch(e=>say(String(e),true));
  function makeStore(c:FloorCore):FloorStore {return new FloorStore(c,serialized=>{try{localStorage.setItem(FLOOR_STORAGE,serialized);}catch{throw new Error('Floor records could not be saved. Browser storage is full or blocked. Free space or restore a smaller backup before recording production.');}});}
  // Browser: the floor engine (4 MB) loads when Floor opens or when the browser is idle. Desktop: at start, so project autosave always has the records.
  let floorLoading=false;
  const loadFloor=():void=>{if(floorLoading)return;floorLoading=true;void loadFloorCore().then(c=>{
    core=c;const candidate=makeStore(c);
    const raw=localStorage.getItem(FLOOR_STORAGE);if(pendingRestore!==undefined)candidate.restore(pendingRestore);else if(raw)candidate.restore(JSON.parse(raw));
    store=candidate;
    ready=true;renderLists();say('Local Rust floor engine ready. Records stay on this computer.');
  }).catch(e=>{ready=false;say(`Floor could not start: ${String(e)}. Build with npm run build:floor-wasm -w @beta/marker-web. Saved records were not replaced.`,true);});};
  if(isDesktop())loadFloor();else whenIdle(loadFloor);
  return {
    busy:()=>working,
    activate:()=>{loadFloor();if(ready){renderSources();renderDashboard();}},
    snapshot:()=>store?.get(),
    restore:(state:unknown)=>{if(state==null)return;if(store){store.restore(state);renderLists();}else pendingRestore=state;},
  };
}

