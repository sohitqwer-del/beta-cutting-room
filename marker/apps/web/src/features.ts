import { installCuttingRoom } from './cutting-room';
import { installFloor } from './floor-ui';
import type { CutPlan, Job, Layout, MarkerCore, MarkerResult, MatchAxes, Nap, Pt, ShadeAxis } from '@beta/marker-wasm';
import { bbox } from '@beta/marker-wasm/geometry';
import type { DocumentState } from './state';
import { readDocument } from './state';
import { icon } from './icons';
import { isDesktop } from './transport';
import { staticMode } from './static-mode';
import { localStore } from './browser-store';
import { PLAN_PROGRESS_EVENT, stopBrowserPlan } from './browser-cutroom';
import { clampDefectPoint, compareLengths, defectZone, nestPayload, planErrors, planPayload, planStatus, rectangleDefect, type Comparison, type PlanForm } from './workflows';
import { byId, find, findAll, park, unpark } from './screens';
import { bySize } from './sizes';
import { PASS_TIP } from './plies';
import { splitNotes, technicalDetailsHtml } from './engine-notes';
import { FOLD_EDGE_PREFIX, FOLD_EDGE_TEXT, FOOTPRINT_TIP, foldEdgeWarning, markerYield, YIELD_TIP } from './yield';

export interface Editor {
  get(): DocumentState; core(): MarkerCore | undefined; busy(): boolean;
  commit(next: DocumentState): void; open(next: DocumentState): void;
  redraw(): void;
  api(path: string, body: BodyInit): Promise<Response>; message(text: string, error?: boolean): void;
}
const esc = (v: unknown): string => String(v ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]!));
const number = (id: string, label: string, value: number | null | undefined, min = 0, step = 'any'): string => `<label>${label}<input id="${id}" type="number" value="${value ?? ''}" min="${min}" step="${step}"></label>`;
const errorText = (e: unknown): string => e instanceof Error ? e.message : String(e);
/** Explain search limits without exposing engine calibration diagnostics. */
function planMessage(value: string): string {
  if (value.startsWith(FOLD_EDGE_PREFIX)) return `${FOLD_EDGE_TEXT} (${value})`;
  if (value.startsWith('Budget calibration')) return 'The planner compared short and longer searches. It kept the best validated fabric total.';
  if (value.startsWith('Final comparison covers')) return 'The final search checked selected plans. Other size ratios may use less fabric.';
  const evaluated = /^Evaluated (\d+) real markers/.exec(value);
  return evaluated ? `Checked ${evaluated[1]} marker layouts. Other size ratios may use less fabric.` : value;
}

const check = (id: string, label: string, enabled: boolean): string => `<label class="check"><input id="${id}" type="checkbox" ${enabled ? 'checked' : ''}>${label}</label>`;
const sizeNames = (job: Job): string[] => [...new Set(job.pieces.flatMap(p => p.size ? [p.size] : []))].sort(bySize);

/** Cutting-room panels. All packing, garment rules and validation call Rust. */
export function installFeatures(editor: Editor) {
  const $ = <T extends Element = HTMLElement>(selector: string): T => find<T>(selector)!;
  const n = (id: string): number => Number($<HTMLInputElement>(`#${id}`).value);
  const checked = (id: string): boolean => $<HTMLInputElement>(`#${id}`).checked;
  let pieceIndex = 0, mode: 'none' | 'rectangle' | 'polygon' = 'none', corners: Pt[] = [];
  let comparison: Comparison | undefined, plan: CutPlan | undefined, planJob: Job | undefined;
  let planEnd = 0, planCost = 0, operation = false, planJobKey = '';
  let colourByBundle = false;
  let cursor: Pt = [0, 0];
  $('.filebar').insertAdjacentHTML('afterend', `<nav class="screen-tabs" aria-label="Workspace screens">${[['Pattern',4],['Digitize',5],['Editor',0],['Cut plan',1],['Rolls',6],['Bundles',7],['Compare',2],['Help',3]].map(([s,i]) => `<button data-screen="${i}" aria-pressed="${i === 0}" aria-controls="${i === 0 ? 'editor-screen' : `screen-${i}`}">${s}</button>`).join('')}</nav>`);
  $('#setup-panel').insertAdjacentHTML('beforeend', '<section id="fabric-v2" class="v2-panel"></section><section id="piece-v2" class="v2-panel"></section>');
  $('.panel-heading').insertAdjacentHTML('afterend', '<nav class="settings-tabs" aria-label="Editor settings"><button type="button" data-settings="job" aria-pressed="true">Basics</button><button type="button" data-settings="fabric" aria-pressed="false">Fabric rules</button><button type="button" data-settings="piece" aria-pressed="false">Piece rules</button></nav>');
  $('#fabric-v2').hidden = true; $('#piece-v2').hidden = true;
  findAll<HTMLButtonElement>('[data-settings]').forEach(button => button.onclick = () => {
    const tab = button.dataset.settings;
    // The Nest dock stays visible on every tab.
    for (const selector of ['#job-form', '.import-settings']) $(selector).hidden = tab !== 'job';
    $('#fabric-v2').hidden = tab !== 'fabric'; $('#piece-v2').hidden = tab !== 'piece';
    findAll<HTMLButtonElement>('[data-settings]').forEach(b => b.setAttribute('aria-pressed', String(b === button)));
    $('#setup-panel').scrollTop = 0;
  });
  $('.workspace').insertAdjacentHTML('afterend', `
    <section id="screen-1" class="screen" hidden><div class="screen-head"><div><h1>Cut plan</h1><p>Enter garments to cut. One lay is one marker spread in several fabric layers (plies).</p></div></div>
      <form id="plan-form" class="card plan-card"><h2 class="section-title"><span class="step-badge" aria-hidden="true">1</span>Garments to cut</h2><div id="plan-quantities" class="fields size-grid"></div>
      <h2 class="section-title"><span class="step-badge" aria-hidden="true">2</span>Lay limits</h2><div class="fields plan-fields">${number('max-plies','Max plies per lay',50,1,'1')}${number('max-garments','Max garments per marker',4,1,'1')}</div><p class="hint">Plies are fabric layers on the table. ${PASS_TIP} So 40 plies of folded fabric = 20 passes.</p>
      <details class="advanced"><summary>More options: over-cut, search time, end allowance, cost</summary><div class="fields plan-fields">${number('overcut','Allowed over-cut (%)',0)}${number('plan-time','Time per marker (seconds)',5,0.1)}${number('plan-final-time','Final search per ratio (seconds)',0)}${number('end-allowance','Allowance at each end (mm)',0)}${number('lay-cost','Cost per lay (INR)',0)}</div><p class="hint">Final search 0 skips the longer search. End allowance is added at both ends of every pass. Lay cost is an estimate for comparing plans.</p></details>
      <div class="form-actions"><button id="make-plan" class="primary large" type="submit">${icon('plan')}Make cut plan</button><button id="stop-plan" type="button" hidden>${icon('stop')}Stop and keep the best plan</button><p id="plan-error" role="status"></p><progress id="plan-progress" class="plan-progress" hidden aria-label="Cut plan search running"></progress></div></form><div id="plan-result" aria-live="polite" data-empty="No cut plan yet. Enter the garments to cut for each size, then choose Make cut plan."></div></section>
    <section id="screen-2" class="screen" hidden><div class="screen-head"><div><h1>Compare fabric use</h1><p>Open a PLT marker from your current CAD. We check it with the same rules and re-nest the same pieces on the same fabric width.</p></div></div>
      <div class="card compare-card"><h2 class="section-title"><span class="step-badge" aria-hidden="true">1</span>Fabric and search time</h2><div class="fields">${number('compare-width','Fabric width (mm)',editor.get().job.fabric.width,1)}${number('compare-time','Re-nest time (seconds)',10,0.1)}</div>
      <h2 class="section-title"><span class="step-badge" aria-hidden="true">2</span>Your CAD marker</h2><label class="upload-label">Current CAD marker (.plt)<input id="compare-file" type="file" accept=".plt,.hpgl"></label>
      <div class="form-actions"><button id="compare-run" class="primary large" disabled>${icon('compare')}Re-nest and compare</button><p id="compare-status" role="status"></p></div></div><div id="compare-result" aria-live="polite" data-empty="No comparison yet. Choose a PLT marker file from your current CAD system."></div></section>
    <section id="screen-3" class="screen help-page" hidden><h1>What a marker planner does</h1><p>A marker planner arranges garment pattern pieces across the fabric width. A shorter marker uses less fabric for the same garments.</p>
      <div class="process" role="img" aria-label="Pattern pieces, then size ratio, then marker on fabric width, then efficiency, then plotter or cutter, then cut plan">${['Pattern pieces','Size ratio','Marker on fabric width','Efficiency','Plotter / cutter','Cut plan'].map((s,i) => `<div><b>${i+1}</b>${s}</div>${i<5 ? '<span aria-hidden="true">→</span>':''}`).join('')}</div>
      <dl><dt>Grain</dt><dd>The grain line follows the length of the fabric. Keep each pattern's grain line in the allowed direction so the garment hangs correctly.</dd>
      <dt>Nap</dt><dd>Pile or a one-way print must face one direction. “One way per garment” keeps all pieces of a garment facing the same way, while other garments may face the opposite way.</dd>
      <dt>Pairs and faceless fabric</dt><dd>A left and right pair needs a normal copy and a mirrored copy. On fabric with no face side, a piece may be turned over; enable “Allow flip” only when this is suitable.</dd>
      <dt>Folded and tubular fabric</dt><dd>Folded fabric has one fold edge; a tube laid flat has two. Each placement normally cuts two layers; a symmetric piece on a fold cuts one full piece.</dd>
      <dt>Stripes and checks</dt><dd>Enter the distance between repeated stripes or checks. Add named seam points and match rules so the print meets at those points.</dd>
      <dt>Shade bands</dt><dd>Fabric colour can change along a roll or across its width. Keep all pieces of one garment in the same band to reduce visible colour differences.</dd>
      <dt>Fusing blocks</dt><dd>Small parts may be cut first as rectangles for bonding with interlining, then cut to shape. The block margin adds room around the part.</dd>
      <dt>Defects</dt><dd>Draw holes or flaws on the fabric. The marker must keep cut pieces clear of these areas.</dd>
      <dt>Shrinkage</dt><dd>Fabric can shrink after washing or fusing. Enter tested percentages so the planner enlarges the patterns before cutting.</dd>
      <dt>Marker garments and floor bundles</dt><dd>A marker garment id links parts of one garment. A sewing-floor bundle holds one part, size, shade and handedness across a range of plies. Its ticket number is a separate identity.</dd>
      <dt>Plies and lays</dt><dd>A ply is one layer of fabric. A lay is a stack of plies cut using one marker: a ratio of M:2 and L:1 on 20 plies makes 40 M and 20 L garments.</dd>
      <dt>Efficiency</dt><dd>Efficiency is the area of the cut pieces divided by fabric width × marker length. Check the rules and required counts as well as the percentage.</dd></dl></section>`);
  function screen(index: number): void {
    // Only the active screen stays in the page; the others are parked with their state (screens.ts).
    for (const panel of findAll<HTMLElement>('.screen')) if (panel.id === `screen-${index}`) unpark(panel); else park(panel);
    for (const part of [$('.workspace'), $('#editor-filebar')]) if (index === 0) unpark(part); else park(part);
    $('#editor-filebar').hidden = index !== 0;
    $('.workspace').setAttribute('hidden', '');
    findAll<HTMLElement>('.screen').forEach(panel => panel.hidden = panel.id !== `screen-${index}`);
    cuttingRoom?.activate(index);
    if (index === 8) floor?.activate();
    if (index === 0) $('.workspace').removeAttribute('hidden');
    findAll<HTMLButtonElement>('[data-screen]').forEach(b => b.setAttribute('aria-pressed',String(Number(b.dataset.screen) === index)));
    if (index === 1) updatePlanSizes();
    window.scrollTo(0, 0);
    window.dispatchEvent(new CustomEvent('beta-screen', { detail: index }));
  }
  $('.workspace').id = 'editor-screen';
  const cuttingRoom = installCuttingRoom(editor, screen, () => ({plan,job:planJob,endAllowance:planEnd}));
  const floor = installFloor(editor, {
    planning: () => ({plan,job:planJob,endAllowance:planEnd}), screen,
    getRegister: () => cuttingRoom.floorRegister(), receiveRegister: register => cuttingRoom.floorReceive(register),
    loadOrder: quantities => { updatePlanSizes(); sizeNames(editor.get().job).forEach((size,i) => { $<HTMLInputElement>(`#quantity-${i}`).value = String(quantities[size] ?? 0); }); },
  });
  findAll<HTMLButtonElement>('[data-screen]').forEach(b => b.onclick = () => screen(Number(b.dataset.screen)));
  $('.screen-tabs').addEventListener('keydown', event => {
    const e = event as KeyboardEvent;
    if (!['ArrowLeft','ArrowRight','Home','End'].includes(e.key)) return;
    const buttons = Array.from(findAll<HTMLButtonElement>('[data-screen]'));
    const current = buttons.indexOf(e.target as HTMLButtonElement);
    if (current < 0) return;
    e.preventDefault();
    const next = e.key === 'Home' ? 0 : e.key === 'End' ? buttons.length - 1 : (current + (e.key === 'ArrowRight' ? 1 : -1) + buttons.length) % buttons.length;
    buttons[next]?.focus(); buttons[next]?.click();
  });
  function updatePlanSizes(): void {
    const job = editor.get().job, names = sizeNames(job), key = JSON.stringify([job.name,names]);
    if (key === planJobKey) return;
    planJobKey = key;
    $('#plan-quantities').innerHTML = names.map((s,i) => number(`quantity-${i}`,`${esc(s)} garments`,0,0,'1')).join('') || '<p>Add sizes to pieces in the editor before planning an order.</p>';
  }
  function renderFabric(): void {
    const f = editor.get().job.fabric;
    $('#fabric-v2').innerHTML = `<h2>Fabric rules</h2>${number('fabric-width','Fabric width (mm)',f.width,1)}<label>Nap / direction<select id="fabric-nap">${(['none','one_way','one_way_per_bundle'] as const).map(v=>`<option value="${v}" ${v===(f.nap??'none')?'selected':''}>${{none:'None',one_way:'One-way',one_way_per_bundle:'One way per garment'}[v]}</option>`).join('')}</select></label><label>Fabric layout<select id="layout">${(['open','folded','tubular'] as const).map(v=>`<option ${v === (f.layout ?? 'open') ? 'selected':''}>${v}</option>`).join('')}</select></label>
      <svg class="layout-diagram" viewBox="0 0 240 72" role="img" aria-label="${esc(f.layout ?? 'open')} fabric, length to the right"><rect x="8" y="16" width="210" height="40" fill="#e4eee8" stroke="#527961"/>${f.layout !== 'open' && f.layout ? '<path d="M8 16H218" stroke="#b36725" stroke-width="4"/>':''}${f.layout === 'tubular' ? '<path d="M8 56H218" stroke="#b36725" stroke-width="4"/>':''}<text x="18" y="42" font-size="12">Length → · ${f.layout === 'open' || !f.layout ? '1 layer' : '2 layers'}</text></svg><p class="hint">Orange edges are folds. Width is the laid-flat usable width.</p>
      ${check('allow-flip','Faceless fabric: allow flip',!!f.allow_flip)}
      <div class="fields">${number('shrink-length','Length shrinkage (%)',f.shrinkage?.length_pct ?? 0)}${number('shrink-width','Width shrinkage (%)',f.shrinkage?.width_pct ?? 0)}</div>
      ${check('shade-enabled','Use shade bands',!!f.shade)}<div class="fields"><label>Band axis<select id="shade-axis"><option value="length" ${f.shade?.axis !== 'width'?'selected':''}>Along length</option><option value="width" ${f.shade?.axis === 'width'?'selected':''}>Across width</option></select></label>${number('shade-band','Band size (mm)',f.shade?.band_mm ?? 1000,0.01)}</div>
      ${check('repeat-enabled','Use stripes / checks',!!f.repeat)}<div class="fields">${number('period-x','Length repeat (mm)',f.repeat?.period_x,0.01)}${number('period-y','Width repeat (mm)',f.repeat?.period_y,0.01)}${number('offset-x','Length offset (mm)',f.repeat?.offset_x ?? 0,-100000)}${number('offset-y','Width offset (mm)',f.repeat?.offset_y ?? 0,-100000)}${number('repeat-tolerance','Tolerance (mm)',f.repeat?.tolerance_mm ?? 1)}</div><p class="hint">Leave a repeat period empty for an axis with no repeat.</p>
      <h2>Fabric defects</h2><label>Drawing tool<select id="defect-mode"><option value="none">Select / move pieces</option><option value="rectangle">Rectangle: click two corners</option><option value="polygon">Polygon: click each corner</option></select></label><p id="defect-prompt" class="hint"></p><div class="toolgroup"><button id="defect-finish" type="button">Finish polygon</button><button id="defect-cancel" type="button">Cancel drawing</button></div>
      <details><summary>Enter defect corners with keyboard</summary><label>Points: x,y; x,y; x,y (mm)<textarea id="defect-coordinates" rows="3" placeholder="100,100; 200,100; 200,200; 100,200"></textarea></label><button type="button" id="defect-add">Add defect</button></details>
      <ul class="plain-list">${(f.defects ?? []).map((z,i)=>`<li>${esc(z.id || `Defect ${i+1}`)} · ${z.polygon.length} corners <button type="button" data-delete-defect="${i}">Delete</button></li>`).join('') || '<li>No defects marked.</li>'}</ul>`;
    $<HTMLSelectElement>('#defect-mode').value = mode;
    promptDrawing();
  }
  function promptDrawing(): void { $('#defect-prompt').textContent = mode === 'none' ? 'Choose a tool to draw on the marker.' : `${corners.length} corner(s). ${mode === 'rectangle' ? 'Click two opposite corners on the fabric.' : 'Click each corner, then Finish polygon. Escape cancels.'}`; }
  function renderPiece(): void {
    const pieces = editor.get().job.pieces; pieceIndex = Math.min(pieceIndex,Math.max(0,pieces.length-1));
    const p = pieces[pieceIndex];
    $('#piece-v2').innerHTML = `<h2>Piece rules & seam matching</h2><label>Edit piece<select id="edit-piece">${pieces.map((p,i)=>`<option value="${i}" ${i===pieceIndex?'selected':''}>${esc(p.name)} · ${esc(p.size)}</option>`).join('')}</select></label><div id="piece-detail"></div><h2>Match rules</h2><div id="rule-list"></div><div id="rule-form"></div>`;
    if (p) {
      const b = bbox(p.outline)!; const pad = Math.max(b[2]-b[0],b[3]-b[1],10)*0.1;
      $('#piece-detail').innerHTML = `${check('fold-enabled','Symmetric piece with fold line',!!p.fold_line)}<p class="hint">The full outline stays in the file. Verify the symmetry line in piece coordinates.</p><div class="fields">${[0,1].flatMap(i=>[0,1].map(j=>number(`fold-${i}-${j}`,`${i===0?'Start':'End'} ${j===0?'x':'y'} (mm)`,p.fold_line?.[i]?.[j] ?? (j===0?(i===0?b[0]:b[2]):(b[1]+b[3])/2),-100000))).join('')}</div>
      ${check('block-enabled','Cut as a fusing block',p.block_margin != null)}${number('block-margin','Block margin (mm)',p.block_margin ?? 0)}
      <h3>Named match points</h3><p class="hint">Click the outline view to add a seam point. For keyboard use, enter x and y below. Points use the original pattern coordinates.</p><label>Point name<input id="point-name" value="seam-${(p.match_points?.length ?? 0)+1}" maxlength="80"></label>
      <svg id="piece-editor" tabindex="0" role="group" aria-label="Piece editor. Arrow keys move the point cursor; Enter adds a point." viewBox="${b[0]-pad} ${-b[3]-pad} ${b[2]-b[0]+pad*2} ${b[3]-b[1]+pad*2}"><polygon points="${p.outline.map(([x,y])=>`${x},${-y}`).join(' ')}" fill="#c5e2d1" stroke="#3c7253" vector-effect="non-scaling-stroke"/>${p.fold_line?`<polyline points="${p.fold_line.map(([x,y])=>`${x},${-y}`).join(' ')}" fill="none" stroke="#bd651e" stroke-dasharray="6 3" vector-effect="non-scaling-stroke"/>`:''}${(p.match_points??[]).map(m=>`<circle cx="${m.point[0]}" cy="${-m.point[1]}" r="${pad/5}" fill="#824429"><title>${esc(m.id)}</title></circle>`).join('')}</svg>
      <div class="fields">${number('point-x','Point x (mm)',cursor[0],-100000)}${number('point-y','Point y (mm)',cursor[1],-100000)}</div><button type="button" id="add-point">Add named point</button><ul class="plain-list">${(p.match_points??[]).map((m,i)=>`<li>${esc(m.id)} (${m.point.map(n=>n.toFixed(1)).join(', ')}) <button data-delete-point="${i}" type="button">Delete</button></li>`).join('')}</ul>`;
    }
    const rules = editor.get().job.match_rules ?? [];
    $('#rule-list').innerHTML = rules.map((r,i)=>`<p>${esc(r.a.piece_id)}/${esc(r.a.point_id)} → ${r.b?`${esc(r.b.piece_id)}/${esc(r.b.point_id)}`:'fabric repeat'} · ${r.axes} · offset ${r.offset?.join(', ')??'0, 0'} <button type="button" data-delete-rule="${i}">Delete</button></p>`).join('');
    const refs = pieces.flatMap(p=>(p.match_points??[]).map(m=>({piece_id:p.id,point_id:m.id})));
    $('#rule-form').innerHTML = `<label>From point<select id="rule-a">${refs.map((r,i)=>`<option value="${i}">${esc(r.piece_id)}/${esc(r.point_id)}</option>`).join('')}</select></label><label>To point<select id="rule-b"><option value="">Fabric repeat</option>${refs.map((r,i)=>`<option value="${i}">${esc(r.piece_id)}/${esc(r.point_id)}</option>`).join('')}</select></label><label>Match axes<select id="rule-axes"><option value="both">Both axes (checks)</option><option value="x">Length (x)</option><option value="y">Width (y)</option></select></label><div class="fields">${number('rule-x','Length offset (mm)',0,-100000)}${number('rule-y','Width offset (mm)',0,-100000)}</div><button id="add-rule" type="button" ${refs.length?'':'disabled'}>Add match rule</button>`;
  }
  function edit(mutator: (job: Job)=>void): void {
    if (editor.busy() || operation) return;
    const next = structuredClone(editor.get()); mutator(next.job); readDocument(JSON.stringify(next.job)); editor.commit(next);
  }
  function addDefect(points: Pt[]): void {
    const zone = defectZone(points,`defect-${crypto.randomUUID()}`);
    corners=[]; edit(j=>{(j.fabric.defects??=[]).push(zone);}); promptDrawing();
  }
  $('#fabric-v2').addEventListener('change',event=>{
    const input = event.target as HTMLInputElement;
    if (input.id === 'defect-mode') { mode=input.value as typeof mode; corners=[]; promptDrawing(); editor.redraw(); return; }
    if (input.id === 'defect-coordinates') return;
    try {
      if (!input.checkValidity()) { input.reportValidity(); return; }
      edit(j=>{
        const f=j.fabric; f.width=n('fabric-width'); f.nap=$<HTMLSelectElement>('#fabric-nap').value as Nap; f.layout=$<HTMLSelectElement>('#layout').value as Layout; f.allow_flip=checked('allow-flip');
        f.shrinkage={length_pct:n('shrink-length'),width_pct:n('shrink-width')};
        f.shade=checked('shade-enabled')?{axis:$<HTMLSelectElement>('#shade-axis').value as ShadeAxis,band_mm:n('shade-band')}:null;
        f.repeat=checked('repeat-enabled')?{period_x:$<HTMLInputElement>('#period-x').value?n('period-x'):null,period_y:$<HTMLInputElement>('#period-y').value?n('period-y'):null,offset_x:n('offset-x'),offset_y:n('offset-y'),tolerance_mm:n('repeat-tolerance')}:null;
      });
    } catch(e){editor.message(errorText(e),true);}
  });
  $('#fabric-v2').addEventListener('click',event=>{
    const b=(event.target as Element).closest<HTMLButtonElement>('button'); if(!b)return;
    try {
      if(b.id==='defect-finish'){addDefect(corners);mode='none';$<HTMLSelectElement>('#defect-mode').value=mode;promptDrawing();}
      if(b.id==='defect-cancel'){corners=[];mode='none';$<HTMLSelectElement>('#defect-mode').value=mode;promptDrawing();editor.redraw();}
      if(b.id==='defect-add'){
        const points=$<HTMLTextAreaElement>('#defect-coordinates').value.split(';').map(s=>s.trim().split(/[ ,]+/).map(Number));
        if(points.some(p=>p.length!==2))throw new Error('Enter x,y for every corner, separated by semicolons.');
        addDefect(points as Pt[]);
      }
      if(b.dataset.deleteDefect!==undefined)edit(j=>{j.fabric.defects?.splice(Number(b.dataset.deleteDefect),1);});
    }catch(e){editor.message(errorText(e),true);}
  });
  function addPoint(): void {
    const id=$<HTMLInputElement>('#point-name').value.trim(),point:Pt=[n('point-x'),n('point-y')];
    if(!id || editor.get().job.pieces[pieceIndex]?.match_points?.some(m=>m.id===id))throw new Error('Use a unique point name for this piece.');
    cursor=point;edit(j=>{(j.pieces[pieceIndex]!.match_points??=[]).push({id,point});});
  }
  $('#piece-v2').addEventListener('change',event=>{
    const input=event.target as HTMLInputElement;
    try {
      if(input.id==='edit-piece'){pieceIndex=Number(input.value);renderPiece();return;}
      if(!['fold-enabled','block-enabled','block-margin'].includes(input.id)&&!input.id.startsWith('fold-'))return;
      if(!input.checkValidity()){input.reportValidity();return;}
      edit(j=>{const p=j.pieces[pieceIndex]!;p.fold_line=checked('fold-enabled')?[[n('fold-0-0'),n('fold-0-1')],[n('fold-1-0'),n('fold-1-1')]]:null;p.block_margin=checked('block-enabled')?n('block-margin'):null;});
    }catch(e){editor.message(errorText(e),true);}
  });
  $('#piece-v2').addEventListener('click',event=>{
    try {
      const b=(event.target as Element).closest<HTMLButtonElement>('button');
      if(b?.id==='add-point')addPoint();
      if(b?.dataset.deletePoint!==undefined)edit(j=>{const p=j.pieces[pieceIndex]!,id=p.match_points![Number(b.dataset.deletePoint)]!.id;p.match_points!.splice(Number(b.dataset.deletePoint),1);j.match_rules=j.match_rules?.filter(r=>![r.a,r.b].some(ref=>ref?.piece_id===p.id&&ref.point_id===id));});
      if(b?.dataset.deleteRule!==undefined)edit(j=>{j.match_rules?.splice(Number(b.dataset.deleteRule),1);});
      if(b?.id==='add-rule'){
        const refs=editor.get().job.pieces.flatMap(p=>(p.match_points??[]).map(m=>({piece_id:p.id,point_id:m.id})));
        const a=refs[n('rule-a')],bv=$<HTMLSelectElement>('#rule-b').value;
        if(!a)throw new Error('Add a named point first.');
        edit(j=>{(j.match_rules??=[]).push({a,b:bv===''?null:refs[Number(bv)],axes:$<HTMLSelectElement>('#rule-axes').value as MatchAxes,offset:[n('rule-x'),n('rule-y')]});});
      }
      const svg=(event.target as Element).closest<SVGSVGElement>('#piece-editor');
      if(svg){const matrix=svg.getScreenCTM();if(!matrix)return;const q=new DOMPoint(event.clientX,event.clientY).matrixTransform(matrix.inverse());cursor=[q.x,-q.y];$<HTMLInputElement>('#point-x').value=cursor[0].toFixed(2);$<HTMLInputElement>('#point-y').value=cursor[1].toFixed(2);addPoint();}
    }catch(e){editor.message(errorText(e),true);}
  });
  $('#piece-v2').addEventListener('keydown',event=>{
    if((event.target as Element).id!=='piece-editor')return;
    const e=event as KeyboardEvent,delta:Record<string,Pt>={ArrowLeft:[-1,0],ArrowRight:[1,0],ArrowUp:[0,1],ArrowDown:[0,-1]};
    const d=delta[e.key];if(d){e.preventDefault();cursor=[n('point-x')+d[0]*(e.shiftKey?10:1),n('point-y')+d[1]*(e.shiftKey?10:1)];$<HTMLInputElement>('#point-x').value=String(cursor[0]);$<HTMLInputElement>('#point-y').value=String(cursor[1]);}
    if(e.key==='Enter'){e.preventDefault();try{addPoint();}catch(e){editor.message(errorText(e),true);}}
  });
  function formValues():PlanForm{return {quantities:Object.fromEntries(sizeNames(editor.get().job).map((s,i)=>[s,n(`quantity-${i}`)])),maxPlies:n('max-plies'),maxGarments:n('max-garments'),overcut:n('overcut'),time:n('plan-time'),finalTime:n('plan-final-time'),endAllowance:n('end-allowance'),layCost:n('lay-cost')};}
  $('#plan-form').addEventListener('input',()=>{$('#plan-error').textContent=planErrors(formValues()).join(' ');});
  $('#plan-form').addEventListener('submit',async event=>{
    event.preventDefault();if(operation||editor.busy())return;
    try{
      const form=formValues(),payload=planPayload(editor.get().job,form);operation=true;$<HTMLButtonElement>('#make-plan').disabled=true;$('#plan-error').textContent='Rust is evaluating the cut plan…';
      $('#stop-plan').hidden=!staticMode();$('#plan-progress').hidden=false;
      plan=await(await editor.api('plan',JSON.stringify(payload))).json() as CutPlan;planJob=payload.order.job;planEnd=form.endAllowance;planCost=form.layCost;renderPlan();$('#plan-error').textContent=planStatus(plan);
      savePlan();
    }catch(e){$('#plan-error').textContent=errorText(e);}
    finally{operation=false;$<HTMLButtonElement>('#make-plan').disabled=false;$('#stop-plan').hidden=true;$('#plan-progress').hidden=true;}
  });
  // Browser plans run in a Worker: show each checked step, and allow a stop that keeps the best plan.
  window.addEventListener(PLAN_PROGRESS_EVENT,event=>{const p=(event as CustomEvent<CutPlan>).detail;if(operation)$('#plan-error').textContent=`Measured ${p.markers_evaluated} marker${p.markers_evaluated===1?'':'s'} in ${p.runtime_s.toFixed(0)} s. Best so far: ${p.total_fabric_m.toFixed(2)} m${p.complete?'':' (not complete yet)'}.`;});
  $('#stop-plan').addEventListener('click',()=>{if(stopBrowserPlan())$('#plan-error').textContent='Stopping. Keeping the best checked plan.';});
  /** In the browser the plan is kept on this device (IndexedDB), like rolls and floor records. */
  const PLAN_KEY='plan';
  function savePlan():void{if(isDesktop()||!plan)return;void localStore().set(PLAN_KEY,{plan,planJob,planEnd,planCost}).catch(()=>editor.message('Browser storage is full or blocked. Save a backup file.',true));}
  if(!isDesktop())void localStore().get<{plan?:CutPlan;planJob?:Job;planEnd?:number;planCost?:number}>(PLAN_KEY).then(saved=>{if(!saved?.plan||plan)return;plan=saved.plan;planJob=saved.planJob;planEnd=saved.planEnd??0;planCost=saved.planCost??0;renderPlan();$('#plan-error').textContent=planStatus(plan);}).catch(()=>{});
  /** Folded/tube lays: useful yield first, footprint second (fold7); open lays: efficiency. A lay's own fold-edge note follows. */
  function efficiencyCell(l:CutPlan['lays'][number]):string{
    const y=markerYield(planJob?.fabric.layout,{efficiency:l.efficiency,...(l.result??{})},null);
    const main=y.headline==='yield'?`<strong title="${YIELD_TIP}">${(y.usefulYield!*100).toFixed(1)}% yield</strong><br><small title="${FOOTPRINT_TIP}">footprint ${(l.efficiency*100).toFixed(1)}%</small>`:`${(l.efficiency*100).toFixed(1)}%`;
    const surplus=y.surplus?`<br><small class="warn-text">${y.surplus} piece${y.surplus===1?'':'s'} cut but not used</small>`:'';
    const fold=foldEdgeWarning(l.result?.warnings)?`<br><small class="warn-text" title="${esc(foldEdgeWarning(l.result?.warnings))}">Limited by the fold length</small>`:'';
    return main+surplus+fold;
  }
  function renderPlan():void{
    if(!plan)return;
    const tile=(label:string,value:string)=>`<div><span>${label}</span><strong>${value}</strong></div>`;
    const notes=splitNotes(plan.warnings);
    $('#plan-result').innerHTML=`<div class="plan-totals result-totals">${tile('Total fabric',`${plan.total_fabric_m.toFixed(3)} m`)}${tile('Lays',String(plan.lays.length))}${tile('Planned garments',Object.entries(plan.planned).map(([s,n])=>`${esc(s)} ${n}`).join(' · '))}${planCost>0?tile('Lay cost (estimate)',`INR ${(planCost*plan.lays.length).toFixed(2)}`):''}<button type="button" data-print>${icon('print')}Print cut plan</button></div><div class="table-scroll"><table><caption>${esc(plan.order_name)} — planned lays</caption><thead><tr>${['Lay','Size ratio','Plies','Passes','Length (m)','Efficiency','Fabric (m)','Marker'].map(s=>s==='Passes'?`<th scope="col" title="${PASS_TIP}">${s}</th>`:`<th scope="col">${s}</th>`).join('')}</tr></thead><tbody>${plan.lays.map((l,i)=>`<tr><th scope="row">${i+1}</th><td>${Object.entries(l.ratio).map(([s,n])=>`${esc(s)}:${n}`).join(' · ')}</td><td>${l.plies}</td><td>${l.passes??l.plies}</td><td>${(l.marker_length_mm/1000).toFixed(3)}</td><td>${efficiencyCell(l)}</td><td>${l.fabric_m.toFixed(3)}</td><td><button data-open-lay="${i}" ${l.result?'':'disabled'}>Open marker</button></td></tr>`).join('')}</tbody></table></div>${notes.shown.length?`<div class="notice"><ul>${notes.shown.map(w=>`<li>${esc(planMessage(w))}</li>`).join('')}</ul></div>`:''}${technicalDetailsHtml(notes.technical,esc)}`;
  }
  $('#plan-result').addEventListener('click',event=>{const b=(event.target as Element).closest<HTMLButtonElement>('[data-open-lay]');const lay=plan?.lays[Number(b?.dataset.openLay)];if(!lay?.result||!planJob)return;const job=structuredClone(planJob);job.order={sizes:lay.ratio};editor.open({job,result:lay.result});screen(0);});
  $('#compare-file').addEventListener('change',async()=>{
    const input=$<HTMLInputElement>('#compare-file'),file=input.files?.[0];if(!file||operation||editor.busy())return;
    comparison=undefined;$('#compare-result').innerHTML='';$<HTMLButtonElement>('#compare-run').disabled=true;
    try{
      if(file.size>50*1024*1024)throw new Error('Choose a file smaller than 50 MB.');
      if(!Number.isFinite(n('compare-width'))||n('compare-width')<=0)throw new Error('Enter a positive fabric width.');
      operation=true;$('#compare-status').textContent='Reading the PLT marker…';
      const input=await(await editor.api(`import?format=plt&width=${n('compare-width')}`,file)).json() as {job:Job;result:MarkerResult};
      const job=readDocument(JSON.stringify(input.job)),result=readDocument(JSON.stringify(input.result));if(!('pieces'in job)||!('placements'in result))throw new Error('Import did not return a job and marker.');
      comparison={job,theirs:result};await renderCompare();$<HTMLButtonElement>('#compare-run').disabled=false;$('#compare-status').textContent='Current CAD marker loaded. Re-nest to measure savings.';
    }catch(e){$('#compare-status').textContent=errorText(e);}finally{operation=false;input.value='';}
  });
  $('#compare-run').addEventListener('click',async()=>{
    if(!comparison||operation||editor.busy())return;
    try{
      if(!Number.isFinite(n('compare-time'))||n('compare-time')<=0)throw new Error('Enter a positive re-nest time.');
      operation=true;$<HTMLButtonElement>('#compare-run').disabled=true;$('#compare-status').textContent='Rust is re-nesting the same pieces…';
      const response=await editor.api('nest',JSON.stringify(nestPayload({job:comparison.job,result:comparison.theirs},{time_limit_s:n('compare-time'),seed:42,workers:2,runs:1})));
      const result=readDocument(await response.text());if(!('placements'in result))throw new Error('No marker returned.');comparison.ours=result;await renderCompare();$('#compare-status').textContent='Comparison ready. Savings use marker length, at the same width.';
    }catch(e){$('#compare-status').textContent=errorText(e);}finally{operation=false;$<HTMLButtonElement>('#compare-run').disabled=false;}
  });
  async function renderCompare():Promise<void>{
    if(!comparison)return;
    const c=comparison;
    const validate=async(result:MarkerResult)=>editor.core()?.validate(c.job,result)??await(await editor.api('validate',JSON.stringify({job:c.job,result}))).json() as ReturnType<MarkerCore['validate']>;
    const theirs=await validate(c.theirs),ours=c.ours?await validate(c.ours):undefined;
    const saving=ours&&theirs.ok&&ours.ok?compareLengths(theirs.length,ours.length):null;
    const issues = [...theirs.errors, ...theirs.warnings].map(e=>`Current CAD — ${e.code}: ${e.message}`)
      .concat([...ours?.errors??[], ...ours?.warnings??[]].map(e=>`BETA — ${e.code}: ${e.message}`),
        (c.theirs.warnings??[]).map(w=>`Current CAD — ${w}`), splitNotes(c.ours?.warnings).shown.map(w=>`BETA — ${w}`));
    $('#compare-result').innerHTML=`<div class="comparison-cards"><article><h2>Your current CAD</h2><strong>${(theirs.length/1000).toFixed(3)} m</strong><p>${(theirs.efficiency*100).toFixed(1)}% efficiency · ${theirs.ok?'Checks passed':'Check issues below'}</p></article><article><h2>BETA re-nest</h2><strong>${ours?`${(ours.length/1000).toFixed(3)} m`:'Waiting'}</strong><p>${ours?`${(ours.efficiency*100).toFixed(1)}% efficiency · ${ours.ok?'Checks passed':'Check issues below'}`:'Uses the same imported pieces and rules'}</p></article></div>${saving?`<p class="saving">${saving.metres.toFixed(3)} m · ${saving.percent.toFixed(2)}% fabric saving per marker</p>`:'<p>Savings are shown when both markers pass validation.</p>'}<ul>${[...theirs.errors,...ours?.errors??[]].map(e=>`<li>${esc(e.code)}: ${esc(e.message)}</li>`).join('')}</ul><button id="open-theirs">Open current CAD marker</button> ${c.ours?'<button id="open-ours">Open BETA marker</button>':''}`;
    $('#compare-result ul').innerHTML = issues.map(e=>`<li>${esc(e)}</li>`).join('');
    $('#open-theirs').onclick=()=>{editor.open({job:c.job,result:c.theirs});screen(0);};if(c.ours)$('#open-ours').onclick=()=>{editor.open({job:c.job,result:c.ours!});screen(0);};
  }
  updatePlanSizes();renderFabric();renderPiece();
  return {
    show:screen,
    desktopSnapshot:()=>({plan,planJob,planEnd,planCost,cuttingRoom:cuttingRoom.desktopSnapshot(),floor:floor.snapshot()}),
    desktopSaved:()=>cuttingRoom.desktopSaved(),
    desktopRestore:(value:unknown)=>{const saved=value as {floor?:unknown;cuttingRoom?:unknown;plan?:CutPlan;planJob?:Job;planEnd?:number;planCost?:number}|undefined;if(saved){if(saved.cuttingRoom)cuttingRoom.desktopRestore(saved.cuttingRoom);floor.restore(saved.floor);plan=saved.plan;planJob=saved.planJob;planEnd=saved.planEnd??0;planCost=saved.planCost??0;renderPlan();}},
    busy:()=>operation || cuttingRoom.busy() || floor.busy(),
    colourByBundle:()=>colourByBundle,
    setColourByBundle:(enabled:boolean)=>{colourByBundle=enabled;},
    refresh:()=>{
      const active=document.activeElement as HTMLInputElement|null,id=active?.id,start=active?.type==='text'?active.selectionStart:null;
      renderFabric();renderPiece();updatePlanSizes();
      if(id){const input=byId(id) as HTMLInputElement|null;input?.focus({preventScroll:true});if(start!=null&&input?.type==='text')input.setSelectionRange(start,start);}
    },
    drawing:()=>mode!=='none',
    cancel:()=>{mode='none';corners=[];promptDrawing();$<HTMLSelectElement>('#defect-mode').value=mode;},
    canvasPoint:(point:Pt,length:number,width:number):boolean=>{
      if(mode==='none')return false;
      if(editor.busy()||operation)return true;
      try{corners.push(clampDefectPoint(point,length,width));if(mode==='rectangle'&&corners.length===2){const polygon=rectangleDefect(corners[0]!,corners[1]!,length,width);if(!polygon.length)throw new Error('Rectangle needs non-zero length and width.');addDefect(polygon);mode='none';$<HTMLSelectElement>('#defect-mode').value=mode;}promptDrawing();}catch(e){corners=[];editor.message(errorText(e),true);}
      return true;
    },
    overlay:(length:number,width:number,unit:number):string=>{
      const f=editor.get().job.fabric;let out='';
      if(f.repeat){const r=f.repeat,px=r.period_x??length,py=r.period_y??width;out+=`<defs><pattern id="fabric-repeat" width="${px}" height="${py}" x="${r.offset_x??0}" y="${width-(r.offset_y??0)}" patternUnits="userSpaceOnUse"><path class="repeat-line" d="${r.period_x?`M0 0V${py}`:''} ${r.period_y?`M0 0H${px}`:''}" fill="none" stroke="#698ec8" stroke-opacity="0.45" stroke-width="${unit}"/></pattern></defs><rect width="${length}" height="${width}" fill="url(#fabric-repeat)" pointer-events="none"/>`;}
      if(f.shade){const max=f.shade.axis==='length'?length:width,band=f.shade.band_mm;for(let k=1;k*band<max&&k<1000;k++){const p=k*band;out+=f.shade.axis==='length'?`<line x1="${p}" x2="${p}" y1="0" y2="${width}" class="shade-line"/>`:`<line x1="0" x2="${length}" y1="${width-p}" y2="${width-p}" class="shade-line"/>`;}}
      if(f.layout==='folded'||f.layout==='tubular')out+=`<path d="M0 0H${length}${f.layout==='tubular'?` M0 ${width}H${length}`:''}" class="fold-edge"/>`;
      out+=(f.defects??[]).map(z=>`<polygon points="${z.polygon.map(([x,y])=>`${x},${width-y}`).join(' ')}" class="defect"><title>${esc(z.id)} ${esc(z.note)}</title></polygon>`).join('');
      if(corners.length)out+=`<polyline points="${corners.map(([x,y])=>`${x},${width-y}`).join(' ')}" class="defect"/>${corners.map(([x,y])=>`<circle cx="${x}" cy="${width-y}" r="${4*unit}" class="defect-corner"/>`).join('')}`;
      return `<g pointer-events="none">${out}</g>`;
    },
  };
}
