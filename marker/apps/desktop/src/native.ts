import { projectPanels } from './project-state';
import { projectAutosave } from './autosave';
/** Native controls supplement the existing cutting-room screens. */
import { apiFetch, invoke, isDesktop, listen } from '../../web/src/transport';
import { emptyResult, readDocument, type DocumentState } from '../../web/src/state';
import type { Job, MarkerResult } from '@beta/marker-wasm';
interface Settings { fabric_width_mm:number;units:'mm'|'cm'|'inch';plotter_units_per_mm:number;iso_profile:unknown;worker_threads:number;language:string }
interface Project { format:string;editor:DocumentState;storage:Record<string,string>;planning?:unknown;settings:Settings }
interface Host { get():DocumentState;open(state:DocumentState):void;busy():boolean;message(text:string,error?:boolean):void;planning():unknown;saved():void;restorePlanning(value:unknown):void }
interface Opened { kind:'project'|'file';path:string;value?:Project;name?:string;bytes?:number[] }
const wait = (ms:number) => new Promise(resolve => setTimeout(resolve,ms));
export async function installDesktop(host:Host):Promise<void> {
 if (!isDesktop()) return;
 document.querySelector<HTMLElement>('#nest-form .hint')!.textContent = 'Lock pieces with L, then re-nest unlocked pieces around them. Cancel search stops the nesting job.';
 document.querySelector<HTMLElement>('.export-panel .hint')!.textContent = 'Choose a format, then choose where to save the file.';
 document.querySelector<HTMLElement>('.file-hint')!.textContent = 'DXF, PLT, job, marker or pattern files';
 const $ = <T extends HTMLElement>(id:string) => document.getElementById(id) as T;
 const failure = (e:unknown) => host.message(e instanceof Error ? e.message : String(e),true);
 try {
  const boot=await invoke<{settings:Settings;recent:string[];pending:string[];recovery:Project|null;project:Project|null;smoke:boolean;rust_start_ms:number}>('desktop_bootstrap');
  let settings=boot.settings,lastPdf:string|null=null;
  const bar=document.createElement('div');bar.className='filebar';bar.style.flexWrap='wrap';bar.setAttribute('aria-label','Desktop files and settings');
  bar.innerHTML='<button id="native-open">Open native file</button><button id="project-open">Open project</button><button id="project-save">Save project</button><label>Recent files<select id="native-recent"><option value="">Choose a file</option></select></label><button id="native-recover" hidden>Recover autosave</button><div class="push"></div><output id="native-dimensions"></output><button id="native-print" disabled>Open last PDF to print</button><button id="native-settings">Settings</button><button id="native-cancel" disabled>Cancel search</button>';
  document.querySelector('header')!.after(bar);
  const keepCurrent=document.createElement('button');keepCurrent.id='native-keep-current';keepCurrent.textContent='Use current work instead';keepCurrent.title='Replace the previous autosave with the current work.';keepCurrent.hidden=true;$('native-recover').after(keepCurrent);
  const recent=$<HTMLSelectElement>('native-recent');recent.style.maxWidth='16rem';
  for(const path of boot.recent){const option=document.createElement('option');option.value=path;option.textContent=path;recent.append(option);}
  const project=():Project=>({format:'beta-cutting-room.project/v1',editor:structuredClone(host.get()),storage:Object.fromEntries(Object.keys(localStorage).filter(k=>k.startsWith('beta-')&&k!=='beta-cutting-room-v1').map(k=>[k,localStorage.getItem(k)!])),planning:structuredClone(host.planning()),settings});
  const autosave=projectAutosave({read:project,busy:()=>host.busy(),write:(value:Project)=>invoke<void>('desktop_autosave',{value}),saved:()=>host.saved()},!!boot.recovery);
  const apply=async(value:Project)=>{
   if(host.busy())throw new Error('Wait for the current operation.');
   if(value.format!=='beta-cutting-room.project/v1')throw new Error('Unsupported project.');
   const job=readDocument(JSON.stringify(value.editor.job)),result=readDocument(JSON.stringify(value.editor.result));
   if(!('pieces'in job)||!('placements'in result))throw new Error('Invalid project marker.');
   await invoke('desktop_stage_project',{value});location.reload();
  };
  const opened=async(value:Opened|null)=>{
   if(!value)return;if(host.busy())throw new Error('Wait for the current operation.');
   if(!Array.from(recent.options).some(option=>option.value===value.path)){const option=document.createElement('option');option.value=value.path;option.textContent=value.path;recent.add(option,1);}
   if(value.kind==='project'){await apply(value.value!);return;}
   const file=new File([new Uint8Array(value.bytes!)],value.name!);
   if(/\.plt$/i.test(file.name)){
    const response=await apiFetch(`/api/import?format=plt&width=${settings.fabric_width_mm}`,{method:'POST',body:file});const data=await response.json() as {job:Job;result:MarkerResult;error?:string};if(!response.ok)throw new Error(data.error);host.open({job:data.job,result:data.result});document.querySelector<HTMLButtonElement>('[data-screen="0"]')?.click();return;
   }
   const text=/\.dxf$/i.test(file.name)?'':await file.text();
   const pattern=/\.betapattern$/i.test(file.name)||(!/\.dxf$/i.test(file.name)&&JSON.parse(text).drawing);
   const input=$<HTMLInputElement>(pattern?'pattern-file':'open-file');const transfer=new DataTransfer();transfer.items.add(file);input.files=transfer.files;input.dispatchEvent(new Event('change',{bubbles:true}));
   document.querySelector<HTMLButtonElement>(`[data-screen="${pattern?4:0}"]`)?.click();
  };
  const open=async(path?:string,folder=false)=>opened(await invoke<Opened|null>('desktop_open',{path:path??null,project:folder}));
  $('native-open').onclick=()=>void open().catch(failure);$('project-open').onclick=()=>void open(undefined,true).catch(failure);
  $('open-button').addEventListener('click',e=>{e.preventDefault();e.stopImmediatePropagation();void open().catch(failure);},true);
  recent.onchange=()=>{if(recent.value)void open(recent.value).catch(failure);recent.value='';};
  $('project-save').onclick=()=>void invoke<string|null>('desktop_project_save',{value:project()}).then(path=>{if(path)host.message(`Project saved: ${path}`);}).catch(failure);
  if(boot.recovery){$('native-recover').hidden=false;keepCurrent.hidden=false;$('native-recover').onclick=()=>void apply(boot.recovery!).catch(failure);keepCurrent.onclick=()=>{if(host.busy()){host.message('Wait for the current operation.');return;}autosave.useCurrent();$('native-recover').hidden=true;keepCurrent.hidden=true;host.message('Current work selected. Autosave has resumed.');};host.message('A previous autosave is available. Recover it or choose the current work before autosave resumes.');}
  if(boot.project){host.restorePlanning(projectPanels(boot.project));host.open(boot.project.editor);settings=boot.project.settings;host.message('Project restored. Review validation before cutting.');}
  $('native-cancel').onclick=()=>void apiFetch('/api/nest/cancel',{method:'POST'}).then(async r=>{if(!r.ok)throw new Error((await r.json() as {error:string}).error);host.message('Stopping search...');}).catch(failure);
  await listen<{running:boolean;elapsed_s:number;best_length_mm:number|null;best_efficiency:number|null}>('nest-progress',p=>{
   $<HTMLButtonElement>('native-cancel').disabled=!p.running;
   if(p.running)host.message(`Searching: ${p.elapsed_s.toFixed(1)} s${p.best_length_mm==null?'':` / best ${(p.best_length_mm/1000).toFixed(3)} m`}${p.best_efficiency==null?'':` / ${(p.best_efficiency*100).toFixed(1)}%`}`);
  });
  await listen<string[]>('desktop-open',paths=>{for(const path of paths)void open(path).catch(failure);});
  await listen<{paths?:string[]}>('tauri://drag-drop',event=>{for(const path of event.paths??[])void open(path).catch(failure);});
  for(const path of boot.pending)await open(path);
  window.addEventListener('desktop-saved',event=>{const data=(event as CustomEvent<{path:string|null;name:string}>).detail;if(data.path){host.message(`Saved: ${data.path}`);if(data.name.endsWith('.pdf')){lastPdf=data.path;$<HTMLButtonElement>('native-print').disabled=false;}}else{host.message('Save cancelled.');}});
  $('native-print').onclick=()=>{if(lastPdf)void invoke('desktop_view_pdf',{path:lastPdf}).catch(failure);};
  const dialog=document.createElement('dialog');dialog.setAttribute('aria-label','Desktop settings');
  dialog.innerHTML='<form id="desktop-settings-form"><h2>Desktop settings</h2><label>Default fabric width (mm)<input name="width" type="number" min="1" required></label><label>Display units<select name="units"><option value="mm">Millimetres</option><option value="cm">Centimetres</option><option value="inch">Inches</option></select></label><p>Marker dimensions above use these units. Geometry fields retain their labelled units.</p><label>Plotter units per mm<input name="plotter" type="number" min="0.01" max="10000" step="any" required></label><label>Worker threads<input name="workers" type="number" min="1" max="64" required></label><label>Cutter ISO profile (JSON)<textarea name="iso" rows="8" cols="50" required></textarea></label><label>Language<select name="language"><option value="en">English</option><option disabled>Hindi - planned</option><option disabled>Tamil - planned</option></select></label><p>Use a tested ISO profile for your controller.</p><button type="submit">Save settings</button><button id="settings-close" type="button">Close</button></form>';
  document.body.append(dialog);const form=dialog.querySelector<HTMLFormElement>('form')!;
  const control=(name:string)=>form.elements.namedItem(name) as HTMLInputElement;
  $('native-settings').onclick=()=>{control('width').value=String(settings.fabric_width_mm);control('units').value=settings.units;control('plotter').value=String(settings.plotter_units_per_mm);control('workers').value=String(settings.worker_threads);control('iso').value=JSON.stringify(settings.iso_profile,null,2);dialog.showModal();};
  $('settings-close').onclick=()=>dialog.close();
  form.onsubmit=event=>{event.preventDefault();void(async()=>{const next:Settings={fabric_width_mm:Number(control('width').value),units:control('units').value as Settings['units'],plotter_units_per_mm:Number(control('plotter').value),worker_threads:Number(control('workers').value),iso_profile:JSON.parse(control('iso').value),language:'en'};await invoke('desktop_settings',{settings:next});settings=next;dialog.close();host.message('Settings saved. Fabric width applies to new imports.');})().catch(failure);};
  $('open-file').addEventListener('change',()=>{const file=$<HTMLInputElement>('open-file').files?.[0];if(!file||!/\.dxf$/i.test(file.name)||host.busy())return;const state=structuredClone(host.get());state.job.fabric.width=settings.fabric_width_mm;state.result=emptyResult(state.job);host.open(state);},true);
  const update=()=>{const s=host.get();const factor=settings.units==='cm'?10:settings.units==='inch'?25.4:1;$('native-dimensions').textContent=`${(s.job.fabric.width/factor).toFixed(2)} x ${(s.result.length/factor).toFixed(2)} ${settings.units}`;};update();
  window.setInterval(()=>{update();void autosave.tick().catch(failure);},5000);
  if(boot.smoke){
   try {
    const deadline=Date.now()+20000;while($('wasm-status')?.textContent?.includes('Loading')&&Date.now()<deadline)await wait(100);
    $('demo').click();$<HTMLInputElement>('time').value='5';document.querySelector<HTMLButtonElement>('[data-screen="0"]')?.click();
    const started=performance.now();$<HTMLFormElement>('nest-form').requestSubmit($<HTMLButtonElement>('nest'));
    await wait(100);while(host.busy()&&performance.now()-started<60000)await wait(100);
    if(host.busy()||host.get().result.engine==='synthetic demo')throw new Error('Smoke nesting did not finish.');
    const response=await apiFetch('/api/validate',{method:'POST',body:JSON.stringify(host.get())});const validation=await response.json() as {ok:boolean};if(!validation.ok)throw new Error('Smoke marker failed validation.');
    const saved=new Promise<string|null>(resolve=>window.addEventListener('desktop-saved',event=>resolve((event as CustomEvent<{path:string|null}>).detail.path),{once:true}));
    document.querySelector<HTMLButtonElement>('[data-export="plt"]')!.click();const path=await Promise.race([saved,wait(20000).then(()=>null)]);if(!path)throw new Error('Smoke PLT was not saved.');
    const nestElapsed=performance.now()-started;
    $<HTMLInputElement>('time').value='30';$<HTMLFormElement>('nest-form').requestSubmit($<HTMLButtonElement>('nest'));
    const cancelStarted=performance.now();while($<HTMLButtonElement>('native-cancel').disabled&&performance.now()-cancelStarted<10000)await wait(50);
    $('native-cancel').click();while(host.busy()&&performance.now()-cancelStarted<10000)await wait(50);
    if(host.busy())throw new Error('Cancel did not stop the worker.');
    await invoke('desktop_smoke_finish',{result:{ok:true,cold_start_ms:boot.rust_start_ms,nest_elapsed_ms:nestElapsed,cancel_elapsed_ms:performance.now()-cancelStarted,result:host.get().result,plt:path}});
   }catch(error){await invoke('desktop_smoke_finish',{result:{ok:false,cold_start_ms:boot.rust_start_ms,error:String(error)}});}
  }
 }catch(error){failure(error);}
}
