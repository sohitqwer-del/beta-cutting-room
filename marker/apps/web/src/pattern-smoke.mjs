import { loadPatternCore } from '@beta/marker-wasm';
import { find } from './screens';
/** Browser smoke: import this module in the local app, then call runPatternSmoke().
 * Uses the same forms and click handlers as a pattern maker; never injects JSON state.
 * Take a collaborative-browser recording while it runs to retain mode screenshots.
 */
const get=s=>{const e=find(s);if(!e)throw new Error(`Missing ${s}`);return e;};
const wait=async test=>{for(let i=0;i<100;i++){if(test())return;await new Promise(r=>setTimeout(r,50));}throw new Error('Timed out: '+get('#pd-status').textContent);};
const click=async selector=>{get(selector).click();await wait(()=>!get('#pattern-workbench').hasAttribute('aria-busy'));await new Promise(r=>requestAnimationFrame(()=>r()));};
const fill=(selector,text)=>{const e=get(selector);e.value=String(text);e.dispatchEvent(new Event('input',{bubbles:true}));};
const saved=()=>JSON.parse(localStorage.getItem('beta-cutting-room-v1')).document;
async function tool(id,name,fields){await click(`[data-tool="${id}"]`);fill('#pd-object-id',name);for(const [key,value]of Object.entries(fields))fill(`[data-param="${key}"]`,value);get('#pd-tool-form').requestSubmit();await wait(()=>!get('#pd-tool-dialog').open&&!get('#pattern-workbench').hasAttribute('aria-busy'));}
export async function showPatternMode(mode){await click(`[data-mode="${mode}"]`);window.scrollTo(0,380);return {mode,time:performance.now(),status:get('#pd-status').textContent};}
export async function runPatternSmoke(){
  await click('[data-screen="4"]');await wait(()=>document.querySelectorAll('[data-tool]').length>=13);await click('#pd-new');
  await tool('absolute','A',{x:0,y:0});await tool('absolute','B',{x:'(m.hip+v.ease)/4',y:0});await tool('absolute','C',{x:'(m.hip+v.ease)/4',y:'m.length'});await tool('absolute','D',{x:0,y:'m.length'});
  await tool('line','waist',{from:'A',to:'B'});await tool('line','side',{from:'B',to:'C'});await tool('line','hem',{from:'C',to:'D'});await tool('line','centre',{from:'D',to:'A'});
  await click('#pd-fit');await showPatternMode('piece');await click('#pd-piece-start');
  for(const path of ['waist','side','hem','centre']){fill('#pd-path-add',path);await click('#pd-add-path');}
  fill('#pd-piece-new-name','Rectangle skirt block');await click('#pd-piece-finish');
  document.querySelectorAll('[data-allowance]').forEach(e=>e.value='12');get('#pd-piece-properties').requestSubmit();await wait(()=>!get('#pattern-workbench').hasAttribute('aria-busy'));
  const d=saved();if(d.pieces.length!==1||d.pieces[0].edges.length!==4||d.pieces[0].edges.some(e=>e.allowance!==12))throw new Error('Piece or allowance missing');
  await showPatternMode('grading');if(document.querySelectorAll('#pd-canvas polygon').length!==2)throw new Error('Expected two graded outlines');
  await showPatternMode('output');await click('#pd-export-dxf');
  let benchmark;if(loadPatternCore){const core=await loadPatternCore(),times=[];for(let i=0;i<30;i++){const start=performance.now();core.inspect(d,'M');core.grade(d);times.push(performance.now()-start);}benchmark={min:Math.min(...times),max:Math.max(...times),mean:times.reduce((a,b)=>a+b,0)/times.length};}
  return {objects:Object.keys(d.drawing).length,pieces:d.pieces.length,sizes:d.sizes,allowances:d.pieces[0].edges.map(e=>e.allowance),benchmark,status:get('#pd-status').textContent};
}
/** Regression: opening another pattern must drop missing alignment references and old overlays. */
export async function runPatternSwitchChecks(){
  await runPatternSmoke();
  await showPatternMode('grading');
  fill('#pd-reference','A');get('#pd-reference').dispatchEvent(new Event('change',{bubbles:true}));
  await wait(()=>!get('#pattern-workbench').hasAttribute('aria-busy'));
  await click('#pattern-starter');
  await wait(()=>document.querySelectorAll('#pd-canvas polygon').length===8);
  if(get('#pd-reference').value!==''||get('#pd-status').textContent.includes('reference point missing'))throw new Error('Old grading reference survived a new pattern');
  await showPatternMode('draft');
  await tool('move','front.shift',{source:'front.A',dx:'10',dy:'0'});
  await showPatternMode('grading');
  fill('#pd-reference','front.shift');get('#pd-reference').dispatchEvent(new Event('change',{bubbles:true}));
  await wait(()=>!get('#pattern-workbench').hasAttribute('aria-busy'));
  const before=get('#pd-canvas').innerHTML;
  await showPatternMode('measurements');
  fill('[data-pd-measure="chest"][data-size="M"]','940');get('[data-pd-measure="chest"][data-size="M"]').dispatchEvent(new Event('change',{bubbles:true}));
  await wait(()=>!get('#pattern-workbench').hasAttribute('aria-busy'));
  await showPatternMode('grading');
  if(get('#pd-reference').value!=='front.shift')throw new Error('A valid transformed point lost its grading reference');
  if(get('#pd-canvas').innerHTML===before)throw new Error('Grading kept old outlines after a measurement edit');
  return {missingReferenceCleared:true,transformedReferenceRetained:true,gradedOutlines:document.querySelectorAll('#pd-canvas polygon').length,measurementEditRedrawn:true};
}
/** Exercise editing, failure retention, marks, factory grading and the marker handoff. */
export async function runPatternChecks(){
  const result={};await showPatternMode('draft');
  get('[data-object="A"]').click();await click('#pd-edit');fill('[data-param="x"]','m.missing');await wait(()=>get('[data-param="x"]').getAttribute('aria-invalid')==='true');
  const before=JSON.stringify(saved());get('#pd-tool-form').requestSubmit();await wait(()=>!get('#pattern-workbench').hasAttribute('aria-busy'));
  result.invalidEditRetainsDocument=JSON.stringify(saved())===before&&get('#pd-tool-dialog').open;
  await click('#pd-tool-cancel');
  get('[data-object="A"]').click();await click('#pd-delete');result.dependencyWarning=get('#pd-delete-details').textContent.includes('piece_1');await click('#pd-delete-confirm');result.deletionRemovesDependentPiece=saved().pieces.length===0;
  await click('#pd-undo');result.undoRestoresPiece=saved().pieces.length===1;await click('#pd-redo');result.redoRemovesPiece=saved().pieces.length===0;await click('#pd-undo');
  await showPatternMode('piece');fill('#pd-mark-kind','notch');fill('#pd-mark-distance','50');fill('#pd-notch-shape','v');get('#pd-mark-form').requestSubmit();await wait(()=>!get('#pattern-workbench').hasAttribute('aria-busy'));
  result.shapedNotch=saved().pieces[0].annotations[0]?.shape==='v';
  fill('#pd-grain-a','A');fill('#pd-grain-b','D');fill('#pd-label-point','A');get('#pd-piece-properties').requestSubmit();await wait(()=>!get('#pattern-workbench').hasAttribute('aria-busy'));result.grainAndLabel=saved().pieces[0].grain?.[1]==='D'&&saved().pieces[0].label_point==='A';
  fill('#pd-action','walk');fill('#pd-action-a','waist');fill('#pd-action-b','hem');get('#pd-piece-action').requestSubmit();await wait(()=>!get('#pattern-workbench').hasAttribute('aria-busy'));result.walk=get('#pd-status').textContent.includes('0.000 mm');
  await showPatternMode('grading');fill('#pd-rule-id','waist_grade');fill('#pd-dx-1','10');await click('#pd-rule-save');fill('#pd-grade-point','B');fill('#pd-grade-rule','waist_grade');await click('#pd-attach-rule');fill('#pd-reference','A');get('#pd-reference').dispatchEvent(new Event('change',{bubbles:true}));await wait(()=>!get('#pattern-workbench').hasAttribute('aria-busy'));result.factoryGrading=saved().pieces[0].grade_points.B==='waist_grade';
  await click('#pd-clear-rules');result.measurementGradingRestored=Object.keys(saved().pieces[0].grade_points).length===0;
  await showPatternMode('measurements');fill('[data-pd-increment="waist"]','40');await click('[data-apply-increment="waist"]');result.measurementSteps=saved().meta.measurement_steps.waist===40&&saved().measurements.L.waist===760;
  await click('#pd-measure-export');
  await showPatternMode('output');fill('#pd-ratio-0','1');fill('#pd-ratio-1','1');await click('#pd-export-pdf');await click('#pd-export-plt');await click('#pattern-save');await click('#pd-send-marker');result.markerHandoff=get('#job-title').textContent==='Untitled pattern';
  await click('[data-screen="4"]');
  for(const [key,passed]of Object.entries(result))if(!passed)throw new Error('Browser check failed: '+key);
  return result;
}
/** Check generated group forms and trace-image scale without external image files. */
export async function runPatternTraceChecks(){
  await showPatternMode('draft');await click('[data-tool="move"]');fill('#pd-object-id','_copy');get('#pd-group-toggle').checked=true;get('#pd-group-toggle').dispatchEvent(new Event('change',{bubbles:true}));for(const o of get('#pd-group-objects').options)o.selected=['A','B','C','D'].includes(o.value);fill('[data-param="dx"]','300');fill('[data-param="dy"]','0');get('#pd-tool-form').requestSubmit();await wait(()=>!get('#pd-tool-dialog').open&&!get('#pattern-workbench').hasAttribute('aria-busy'));
  const group=saved().groups._copy?.objects.length===4;await click('#pd-undo');
  const image=document.createElement('canvas');image.width=400;image.height=300;const ctx=image.getContext('2d');ctx.fillStyle='#e7e3d6';ctx.fillRect(0,0,400,300);ctx.strokeStyle='#5a665a';ctx.strokeRect(20,20,100,200);
  const blob=await new Promise(r=>image.toBlob(r,'image/png')),files=new DataTransfer();files.items.add(new File([blob],'original-trace-test.png',{type:'image/png'}));get('#pd-trace-file').files=files.files;get('#pd-trace-file').dispatchEvent(new Event('change',{bubbles:true}));await wait(()=>!get('#pattern-workbench').hasAttribute('aria-busy'));
  fill('#pd-known-distance','200');await click('#pd-calibrate');const svg=get('#pd-canvas');for(const [x,y]of [[20,-20],[120,-20]]){const p=svg.createSVGPoint();p.x=x;p.y=y;const screen=p.matrixTransform(svg.getScreenCTM());svg.dispatchEvent(new PointerEvent('pointerdown',{clientX:screen.x,clientY:screen.y,button:0,bubbles:true}));await wait(()=>!get('#pattern-workbench').hasAttribute('aria-busy'));}
  // SVG screen transforms use floating-point coordinates; sub-pixel round-off
  // must not fail an otherwise correct 200 mm / 100 pixel calibration.
  const trace=saved().meta.trace_image;const result={group,traceCalibrated:trace.calibrated,scale:trace.scale,offlineImage:trace.data.startsWith('data:image/png;base64,')};if(!group||!result.traceCalibrated||Math.abs(result.scale-2)>1e-5)throw new Error('Group or trace calibration failed');return result;
}
export async function benchmarkPattern(){
  const core=await loadPatternCore(),doc=core.starter(),times=[];
  for(let i=0;i<35;i++){const start=performance.now();for(const size of doc.sizes)core.inspect(doc,size);core.grade(doc);const elapsed=performance.now()-start;if(i>=5)times.push(elapsed);}
  const sorted=[...times].sort((a,b)=>a-b);return{objects:Object.keys(doc.drawing).length,pieces:doc.pieces.length,sizes:doc.sizes.length,min:sorted[0],median:sorted[Math.floor(sorted.length/2)],p95:sorted[Math.floor(sorted.length*.95)],max:sorted.at(-1),mean:times.reduce((a,b)=>a+b,0)/times.length};
}
