import type { Pt } from '@beta/marker-wasm';
import type { DocumentState } from './state';
import { editorSettings, type EditorCore, type EditorInspection } from './editor-core';
import { addLibraryEntry, cleanSelection, documentFromEdit, LIBRARY_KEY, readLibrary, type LibraryEntry } from './editor-state';
import { icon } from './icons';
import './editor-tools.css';
import { find, findAll } from './screens';
import { appUnit, longLength, shortLength } from './units-pref';
import { previewRegion } from './region-preview';
interface Host {
  get(): DocumentState; core(): EditorCore | undefined; busy(): boolean;
  selection(): number[]; select(indices: number[]): void;
  commit(next: DocumentState, form?: boolean): void;
  redraw(): void; message(text: string, error?: boolean): void;
  save(name: string, data: BlobPart, type: string): void;
}
const esc = (v: unknown): string => String(v ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]!));
const shortcuts = [
  ['Tab / Shift+Tab', 'Focus a piece or tool'], ['Shift+click', 'Add or remove a piece from the selection'],
  ['A', 'Select all pieces'], ['V', 'Rubber-band selection; drag empty space'], ['G', 'Move / pan mode'],
  ['Arrows', 'Nudge selected group by the chosen step'], ['Shift+Arrows', 'Nudge by 10 mm (1 cm)'], ['Alt+Arrows', 'Bump selected group to contact'],
  ['R', 'Next allowed rotation for selected group'], ['[ / ]', 'Tilt group by −0.5° / +0.5° within each piece rule'],
  ['F', 'Flip selected pieces when allowed'], ['S', 'Swap exactly two selected pieces'], ['L', 'Lock or unlock selected group'],
  ['C / E', 'Compact all / Close the end'], ['Delete', 'Return selected unlocked pieces to the unplaced list'],
  ['M', 'Measure mode; click two points'], ['Enter', 'Place armed copy at numeric X/Y, or finish numeric measure'],
  ['Ctrl+Z / Ctrl+Y', 'Undo / redo one action'], ['0 / + / −', 'Fit / zoom in / zoom out'], ['Escape', 'Cancel mode or drag and clear selection'], ['?', 'Open keyboard help'],
];
/** Editor-only controls. All geometry, demand accounting, measurements and reuse checks call WASM. */
export function installEditorTools(host: Host) {
  const $ = <T extends Element = HTMLElement>(s: string): T => find<T>(s)!;
  const n = (s: string): number => Number($<HTMLInputElement>(s).value);
  let mode: 'move' | 'select' | 'measure' | 'place' = 'move';
  let measureStart: Pt | undefined, measureEnd: Pt | undefined, inspection: EditorInspection | undefined;
  let armed = -1;
  let library: LibraryEntry[] = [];
  let libraryProblem = '';
  let areaPreview: { original: string; state: DocumentState; selected: number[] } | undefined;
  let areaSearch: AbortController | undefined;
  try { library = readLibrary(localStorage.getItem(LIBRARY_KEY)); } catch (e) { libraryProblem = String(e); }
  // Same controls and ids as before, arranged by how often they are used:
  // daily tools in the toolbar (modes, flip, remove, compact, close end, help), the rest in the Tools panel.
  const tool = (attrs: string, name: string, label: string, tip: string, keys = ''): string =>
    `<button type="button" class="icon-btn" ${attrs} aria-label="${label}"${keys ? ` aria-keyshortcuts="${keys}"` : ''} data-tip="${tip}">${icon(name)}<span class="btn-label">${label}</span></button>`;
  $('#tool-modes').insertAdjacentHTML('beforeend', `${tool('data-ed-mode="move" aria-pressed="true"', 'move', 'Move', 'Move pieces or pan · G', 'G')}${tool('data-ed-mode="select" aria-pressed="false"', 'select', 'Select box', 'Drag a box to select · V', 'V')}${tool('data-ed-mode="measure" aria-pressed="false"', 'measure', 'Measure', 'Click two points to measure · M', 'M')}`);
  $('#rotate').insertAdjacentHTML('afterend', tool('id="ed-flip"', 'flip', 'Flip', 'Flip when the fabric allows · F', 'F'));
  $('#lock').insertAdjacentHTML('afterend', tool('id="ed-remove"', 'trash', 'Remove', 'Return to the piece bin · Delete', 'Delete'));
  $('#tool-tidy').insertAdjacentHTML('beforeend', `${tool('id="ed-compact"', 'compact', 'Compact all', 'Compact all pieces left · C', 'C')}${tool('id="ed-close-end"', 'close-end', 'Close the end', 'Pull the marker end to the pieces · E', 'E')}`);
  $('#tool-tidy').insertAdjacentHTML('beforeend', '<button type="button" id="ed-area">Improve area</button>');
  $('#toggle-inspector').insertAdjacentHTML('beforebegin', tool('id="ed-help" aria-haspopup="dialog" data-tip-align="end"', 'keyboard', 'Keyboard help', 'All keyboard shortcuts · ?', '?'));
  $('#inspector-tools').insertAdjacentHTML('beforeend', `
    <section class="inspector-section editor-tools" aria-labelledby="ed-move-title"><h3 id="ed-move-title">Move and turn</h3>
      <div class="dpad" role="group" aria-label="Bump the selection until it touches">${['up','left','right','down'].map(d => `<button type="button" class="icon-btn dpad-${d}" data-ed-bump="${d}" aria-label="Bump ${d}" data-tip="Bump ${d} to contact · Alt+Arrow">${icon(`arrow-${d}`)}</button>`).join('')}<span class="dpad-hub" aria-hidden="true">${icon('move')}</span></div>
      <label>Nudge step (mm)<select id="ed-step">${[0.5, 1, 5, 10].map(x => `<option ${x === 1 ? 'selected' : ''}>${x}</option>`).join('')}</select></label>
      <div class="inspector-row"><button type="button" id="ed-tilt-minus" data-tip="Tilt −0.5° · [">${icon('tilt')}−0.5°</button><button type="button" id="ed-tilt-plus" data-tip="Tilt +0.5° · ]">${icon('tilt')}+0.5°</button><button type="button" id="ed-swap" data-tip="Swap two selected pieces · S">${icon('swap')}Swap</button></div>
      <div class="inspector-row inspector-inline"><label>Group turn (°)<input id="ed-angle" type="number" value="180" step="any"></label><button type="button" id="ed-group-rotate">${icon('rotate')}Rotate group</button></div></section>
    <section class="inspector-section editor-tools" aria-labelledby="ed-select-title"><h3 id="ed-select-title">Select</h3>
      <div class="inspector-row"><button type="button" id="ed-all" data-tip="Select all · A">Select all</button><span id="ed-selected" class="chip" role="status">0 selected</span></div>
      <div class="fields two"><label>Select by<select id="ed-select-field"><option value="size">Size</option><option value="bundle">Bundle (1-based)</option><option value="name">Piece name contains</option></select></label><label>Value<input id="ed-select-value" placeholder="M" size="8"></label></div>
      <button type="button" id="ed-select" class="wide">${icon('search')}Select matches</button></section>
    <section class="inspector-section editor-tools" aria-labelledby="ed-clear-title"><h3 id="ed-clear-title">Clearance</h3>
      <label>Selected piece buffer (mm)<input id="ed-buffer" type="number" value="0" min="0" step="0.5"></label><button type="button" id="ed-set-buffer" class="wide">Apply to selected piece types</button>
      <p class="hint">Buffers add clearance around every copy of a piece type.</p></section>
    <details class="inspector-section editor-tools" id="ed-end-details"><summary>Marker end and cut lines</summary>
      <div class="fields two"><label>Section X (mm)<input id="ed-section-x" type="number" value="1000" min="0" step="any"></label><label>Table end X (mm)<input id="ed-length" type="number" min="1" step="any"></label></div>
      <div class="inspector-row"><button type="button" id="ed-section-add">Add cut line</button><button type="button" id="ed-section-remove">Remove cut line</button><button type="button" id="ed-set-length">Set marker end</button></div>
      <p class="hint">Cut lines span the full width. Pieces cannot cross a cut line. Drag the orange end line to change the table length.</p><p id="ed-sections" class="hint"></p></details>
    <details class="inspector-section editor-tools" id="ed-measure-details"><summary>Measure and exact placement</summary>
      <div class="fields two"><label>Start X<input id="ed-from-x" type="number" value="0" step="any"></label><label>Start Y<input id="ed-from-y" type="number" value="0" step="any"></label><label>End / place X<input id="ed-to-x" type="number" value="100" step="any"></label><label>End / place Y<input id="ed-to-y" type="number" value="0" step="any"></label></div>
      <div class="inspector-row"><button type="button" id="ed-measure-numeric">${icon('measure')}Measure</button><button type="button" id="ed-place-numeric">${icon('target')}Place armed copy</button></div>
      <p id="ed-measurement" class="measurement" role="status">Choose two points on the marker, or enter coordinates.</p></details>`);
  $('.validation-panel').insertAdjacentHTML('beforeend', `<p id="ed-status" class="ed-status" role="status" aria-live="polite"></p><details class="advanced"><summary>Overlaps and gaps</summary><div class="toolgroup"><label>Display tolerance (mm²)<input id="ed-tolerance" type="number" value="0" min="0" step="0.01"></label><label>Resolve direction<select id="ed-resolve-direction"><option>right</option><option>left</option><option>up</option><option>down</option></select></label><button id="ed-resolve">Resolve by bump</button></div><p id="ed-overlap" role="status"></p><ul id="ed-pairs"></ul><p class="hint">Display tolerance labels small overlaps only. Every overlap remains a cutting error. Areas are summed per intersecting pair.</p></details>`);
  $('.under-canvas').insertAdjacentHTML('afterend', `<section class="editor-supply" id="ed-supply" aria-labelledby="ed-bin-title"><details open><summary id="ed-bin-title">Piece bin: unplaced pieces <span id="ed-unplaced-count"></span></summary><p class="hint">Remove a piece to return it here. Choose a copy, then click a clear point on the marker, or use exact X/Y placement in the Tools panel. Rust bumps it left after placing.</p><div class="toolgroup"><button id="ed-fill">${icon('fill')}Fill gaps</button><button id="ed-empty">Build by hand (remove all unlocked)</button></div><div id="ed-unplaced" class="ed-unplaced"></div></details><details><summary>${icon('library')}Marker library</summary><p class="hint">Saved in this browser with the job, piece geometry and ratio. Reuse checks all rules and validates the saved layout. Export a backup before clearing browser storage.</p><div class="toolgroup"><label>Marker name<input id="ed-library-name" value="Finished marker"></label><button id="ed-library-save">Save finished marker</button><button id="ed-library-export">Export library JSON</button><label>Restore backup<input id="ed-library-import" type="file" accept=".json"></label></div><label>Saved marker<select id="ed-library-choice"></select></label><div class="toolgroup"><button id="ed-library-reuse">Reuse marker for current job</button><button id="ed-library-delete" class="danger">Delete selected saved marker</button></div><p id="ed-library-status" role="status"></p></details></section>`);
  document.body.insertAdjacentHTML('beforeend', `<dialog id="ed-shortcuts" aria-labelledby="ed-shortcut-title"><div class="dialog-body"><h2 id="ed-shortcut-title">Editor keyboard shortcuts</h2><p>All fields and tools also work with Tab, Enter and Space. Arrows act on the canvas. Select a group using the filter, Select all, Shift+click or Select box.</p><dl>${shortcuts.map(([key,help]) => `<dt><kbd>${key}</kbd></dt><dd>${help}</dd>`).join('')}</dl><form method="dialog" class="dialog-actions"><button class="primary">Close</button></form></div></dialog>`);
  document.body.insertAdjacentHTML('beforeend', `<dialog id="ed-area-dialog" aria-labelledby="ed-area-title" aria-describedby="ed-area-how"><div class="dialog-body"><h2 id="ed-area-title">Improve one area of the marker</h2>
    <ol class="ed-area-how" id="ed-area-how"><li><strong>Choose the area.</strong> Draw a box around the pieces to tighten, or type its corners.</li><li><strong>Preview.</strong> BETA pushes the pieces inside the box closer together. Pieces outside it, locked pieces and pieces that cross the box edge do not move.</li><li><strong>Apply</strong> if it is better. Undo puts the old marker back.</li></ol>
    <div class="ed-area-pick"><button type="button" id="ed-area-box">${icon('select')}Draw a box on the marker</button><span class="hint" id="ed-area-box-state">No area chosen yet.</span></div>
    <details class="advanced"><summary>Type the corners instead</summary><div class="fields two">
    ${[['x1','Start, along the length'],['y1','Start, across the width'],['x2','End, along the length'],['y2','End, across the width']].map(([id,label])=>`<label>${label} (mm)<input id="ed-area-${id}" type="number" step="any" value="0"></label>`).join('')}</div></details>
    <p class="hint">A quick local search: up to 32 pieces in the box, markers up to 1000 pieces. It may find nothing better; then nothing changes.</p>
    <div class="ed-area-run"><button type="button" id="ed-area-preview" class="primary">${icon('eye')}Preview</button><p id="ed-area-status" role="status" aria-live="polite"></p></div><div id="ed-area-drawing"></div>
    <div class="dialog-actions"><button type="button" id="ed-area-cancel">Cancel</button><button type="button" id="ed-area-apply" class="primary" disabled>Apply · one undo step</button></div></div></dialog>`);
  const areaDialog = $<HTMLDialogElement>('#ed-area-dialog');
  const clearAreaPreview = () => { areaSearch?.abort(); areaSearch = undefined; areaPreview = undefined; $<HTMLButtonElement>('#ed-area-apply').disabled = true; $('#ed-area-drawing').replaceChildren(); };
  areaDialog.addEventListener('input', clearAreaPreview);
  areaDialog.addEventListener('close', clearAreaPreview);
  $('#ed-area-cancel').onclick = () => areaDialog.close();
  $('#ed-area').onclick = () => guarded(() => { clearAreaPreview(); $('#ed-area-status').textContent = 'The marker does not change until you choose Apply.'; areaDialog.showModal(); });
  let pickingArea = false;
  $('#ed-area-box').onclick = () => { areaDialog.close(); setMode('select'); pickingArea = true; host.message('Drag a box from empty space on the marker, around the pieces to tighten. The area dialog opens again when you let go.'); };
  $('#ed-area-preview').onclick = async () => {
    if(host.busy())return;
    clearAreaPreview();
    const original = structuredClone(host.get());
    const from: Pt = [n('#ed-area-x1'),n('#ed-area-y1')], to: Pt = [n('#ed-area-x2'),n('#ed-area-y2')];
    const search = areaSearch = new AbortController();
    $<HTMLButtonElement>('#ed-area-preview').disabled = true;
    $('#ed-area-status').textContent = 'Looking for a tighter layout in this area… Cancel stops it.';
    try {
      const output = await previewRegion(original, { op: 'improve_region', from, to }, search.signal);
      if(search.signal.aborted || JSON.stringify(host.get())!==JSON.stringify(original))throw new Error('The marker changed. Preview again.');
      const next = documentFromEdit(output);
      areaPreview = { original: JSON.stringify(original), state: next, selected: output.selected };
      const same = JSON.stringify(next.result.placements) === JSON.stringify(original.result.placements);
      const saved = original.result.length - next.result.length;
      $('#ed-area-status').textContent = same ? `Nothing better found in this area. Try a bigger box. ${output.notice}`.trim()
        : `${saved > 0.05 ? `The marker gets ${saved < 5 ? `${saved.toFixed(1)} mm` : shortLength(saved)} shorter. ` : 'The marker length stays the same; the pieces in the box sit tighter. '}${output.notice}`.trim();
      $<HTMLButtonElement>('#ed-area-apply').disabled = same;
      const draw = (state: DocumentState, label: string) => {
        const outlines = core().drawing?.(state)?.outlines;
        if (!outlines) return '';
        return `<figure><figcaption>${label} · selected area</figcaption><svg style="width:100%;height:170px" viewBox="${Math.min(from[0],to[0])} ${original.job.fabric.width-Math.max(from[1],to[1])} ${Math.abs(to[0]-from[0])} ${Math.abs(to[1]-from[1])}" role="img" aria-label="${label} selected marker area"><g transform="translate(0 ${original.job.fabric.width}) scale(1 -1)">${outlines.map((points,i)=>`<polygon points="${points.map(p=>p.join(',')).join(' ')}" fill="${output.selected.includes(i)?'var(--accent-soft)':'var(--surface-3)'}" stroke="currentColor" stroke-width="1" vector-effect="non-scaling-stroke"><title>${esc(state.result.placements[i]?.piece_id)}</title></polygon>`).join('')}</g></svg></figure>`;
      };
      $('#ed-area-drawing').innerHTML = draw(original,'Before') + draw(next,'Preview');
    } catch (e) { if(areaSearch===search){clearAreaPreview(); $('#ed-area-status').textContent = e instanceof Error ? e.message : String(e);} }
    finally { $<HTMLButtonElement>('#ed-area-preview').disabled = false; if(areaSearch===search)areaSearch=undefined; }
  };
  $('#ed-area-apply').onclick = () => guarded(() => {
    const preview = areaPreview;
    if (!preview) return;
    if (JSON.stringify(host.get()) !== preview.original) { clearAreaPreview(); $('#ed-area-status').textContent = 'The marker changed. Preview again before applying.'; return; }
    host.commit(preview.state, true); host.select(preview.selected); host.redraw(); areaDialog.close();
    host.message('Area improvement applied as one edit. Undo restores the previous marker.');
  });
  function guarded(fn: () => void): void { if (host.busy()) return; try { fn(); } catch (e) { host.message(String(e), true); } }
  function core(): EditorCore { const c = host.core(); if (!c) throw new Error('Rust editing tools are loading. Rebuild marker WASM if this message persists.'); return c; }
  function run(action: Record<string, unknown>): void {
    guarded(() => {
      const out = core().apply(host.get(), host.selection(), action);
      if (action.op === 'rectangle') {
        const from = action.from as Pt, to = action.to as Pt;
        for (const [id,value] of [['x1',from[0]],['y1',from[1]],['x2',to[0]],['y2',to[1]]] as const) $<HTMLInputElement>(`#ed-area-${id}`).value = String(value);
        clearAreaPreview();
        $('#ed-area-box-state').textContent = `Area chosen: ${shortLength(Math.abs(to[0] - from[0]))} along the length × ${shortLength(Math.abs(to[1] - from[1]))} across. ${out.selected.length} ${out.selected.length === 1 ? 'piece' : 'pieces'} inside.`;
        if (pickingArea) { pickingArea = false; window.setTimeout(() => { if (!areaDialog.open) { areaDialog.showModal(); $<HTMLButtonElement>('#ed-area-preview').focus(); } }, 0); }
      }
      if (out.measurement !== null) $('#ed-measurement').textContent = shortLength(out.measurement);
      if (!['select', 'rectangle', 'measure'].includes(String(action.op))) host.commit(documentFromEdit(out), true);
      host.select(out.selected);
      host.redraw(); if (out.notice) host.message(out.notice);
    });
  }
  function setMode(next: typeof mode): void { if (next === 'measure') { $<HTMLDetailsElement>('#ed-measure-details').open = true; window.dispatchEvent(new Event('beta-inspector')); } mode = next; measureStart = undefined; measureEnd = undefined; armed = next === 'place' ? armed : -1; findAll<HTMLButtonElement>('[data-ed-mode]').forEach(b => b.setAttribute('aria-pressed', String(b.dataset.edMode === mode))); host.redraw(); }
  function measure(): void { measureStart = [n('#ed-from-x'),n('#ed-from-y')]; measureEnd = [n('#ed-to-x'),n('#ed-to-y')]; run({ op:'measure',from:measureStart,to:measureEnd }); }
  findAll<HTMLButtonElement>('[data-ed-bump]').forEach(b => b.onclick = () => run({ op:'bump',direction:b.dataset.edBump }));
  findAll<HTMLButtonElement>('[data-ed-mode]').forEach(b => b.onclick = () => setMode(b.dataset.edMode as typeof mode));
  const clicks: Record<string, () => void> = {
    '#ed-compact': () => run({op:'compact'}), '#ed-close-end': () => run({op:'close_end'}), '#ed-all': () => run({op:'select',field:'all',value:''}),
    '#ed-select': () => run({op:'select',field:$<HTMLSelectElement>('#ed-select-field').value,value:$<HTMLInputElement>('#ed-select-value').value}),
    '#ed-flip': () => run({op:'flip'}), '#ed-swap': () => run({op:'swap'}), '#ed-tilt-minus': () => run({op:'tilt',degrees:-0.5}), '#ed-tilt-plus': () => run({op:'tilt',degrees:0.5}),
    '#ed-group-rotate': () => run({op:'rotate',degrees:n('#ed-angle')}), '#ed-remove': () => run({op:'remove'}), '#ed-help': () => $<HTMLDialogElement>('#ed-shortcuts').showModal(),
    '#ed-resolve': () => run({op:'resolve',direction:$<HTMLSelectElement>('#ed-resolve-direction').value}),
    '#ed-section-add': () => run({op:'section',x:n('#ed-section-x'),remove:false}), '#ed-section-remove': () => run({op:'section',x:n('#ed-section-x'),remove:true}), '#ed-set-length': () => run({op:'length',length:n('#ed-length')}),
    '#ed-measure-numeric': measure, '#ed-place-numeric': () => place([n('#ed-to-x'),n('#ed-to-y')]), '#ed-fill': () => run({op:'fill_gaps'}),
    '#ed-empty': () => { host.select(host.get().result.placements.flatMap((p,i) => p.locked ? [] : [i])); run({op:'remove'}); },
    '#ed-set-buffer': () => { const value = n('#ed-buffer'); if (!Number.isFinite(value) || value < 0) throw new Error('Buffer must be zero or more.'); const state = structuredClone(host.get()); const ids = new Set(host.selection().map(i => state.result.placements[i]?.piece_id)); if (!ids.size) throw new Error('Select pieces first.'); state.job.pieces.forEach(p => { if (ids.has(p.id)) p.buffer = value; }); host.commit(state, true); },
    '#ed-library-save': () => { const check = core().inspect(host.get()); if (!check.validation.ok || !check.geometry_ok) throw new Error('Finish the marker and resolve every validation issue before saving.'); const next = addLibraryEntry(library, host.get(), $<HTMLInputElement>('#ed-library-name').value, crypto.randomUUID(), new Date().toISOString()); persist(next); },
    '#ed-library-export': () => host.save('marker-library.json', JSON.stringify(library,null,2), 'application/json'),
    '#ed-library-reuse': () => { const saved = chosen(); const result = core().reuse(host.get().job, saved); const job = structuredClone(host.get().job); job.meta = { ...job.meta, editor_v1: saved.job.meta?.editor_v1 ?? {} }; host.select([]); host.commit({job,result},true); host.message(`Reused ${saved.label}. Piece geometry and rules are unchanged.`); },
    '#ed-library-delete': () => { const saved = chosen(); persist(library.filter(e => e.id !== saved.id)); },
  };
  for (const [id, fn] of Object.entries(clicks)) $(id).addEventListener('click', () => guarded(fn));
  $('#ed-tolerance').addEventListener('input', renderChecks);
  $('#ed-select-value').addEventListener('keydown', e => { if ((e as KeyboardEvent).key === 'Enter') { e.preventDefault(); clicks['#ed-select']!(); } });
  $('#ed-library-import').addEventListener('change', async e => {
    const input = e.target as HTMLInputElement;
    try { const f = input.files?.[0]; if (!f) return; if (f.size > 50*1024*1024) throw new Error('Library backup must be smaller than 50 MB.'); const entries=readLibrary(await f.text()); for (const entry of entries) core().reuse(entry.job,entry); persist(entries); host.message(`Restored ${entries.length} markers.`); } catch (e) { host.message(String(e),true); } finally { input.value=''; }
  });
  function persist(next: LibraryEntry[]): void { localStorage.setItem(LIBRARY_KEY,JSON.stringify(next)); library=next; libraryProblem=''; renderLibrary(); }
  function chosen(): LibraryEntry { const e=library.find(e=>e.id===$<HTMLSelectElement>('#ed-library-choice').value);if(!e)throw new Error('Choose a saved marker.');return e; }
  function renderLibrary(): void { const select=$<HTMLSelectElement>('#ed-library-choice');const old=select.value;select.innerHTML=library.map(e=>`<option value="${esc(e.id)}">${esc(e.label)} · ${esc(e.savedAt.slice(0,10))}</option>`).join('');if(library.some(e=>e.id===old))select.value=old;$('#ed-library-status').textContent=libraryProblem||`${library.length} / 50 markers saved.`; }
  function renderChecks(): void {
    if (!inspection) { $('#ed-status').textContent='Rust editing checks are unavailable.'; return; }
    const overlap=inspection.pairs.reduce((sum,p)=>sum+p.overlap_mm2,0), gaps=inspection.pairs.filter(p=>p.overlap_mm2<=1e-7).length;
    const tolerance=Math.max(0,n('#ed-tolerance'));
    $('#ed-overlap').textContent=`Overlap ${overlap.toFixed(3)} mm² · ${gaps} gap violations · ${inspection.section_violations.length} section violations · ${inspection.end_violations.length} pieces past end. ${overlap>0 && overlap<=tolerance?'Within display tolerance; still invalid for cutting.':''}`;
    $('#ed-pairs').innerHTML=inspection.pairs.slice(0,30).map(p=>`<li>Copies ${p.a+1} / ${p.b+1}: overlap ${p.overlap_mm2.toFixed(3)} mm²; gap ${shortLength(p.distance_mm, appUnit(), false)} / ${shortLength(p.required_mm)}</li>`).join('');
    $('#ed-status').textContent=`Length ${longLength(inspection.validation.length).value} ${longLength(inspection.validation.length).unit} · table ${longLength(inspection.table_length).value} ${longLength(inspection.table_length).unit} · efficiency ${(inspection.table_efficiency*100).toFixed(1)}% · ${host.get().result.placements.length} placed / ${inspection.unplaced.length} unplaced · ${inspection.pairs.length} gap/overlap pairs · ${inspection.validation.errors.length} rule errors`;
  }
  function refresh(): void {
    try { inspection=host.core()?.inspect(host.get()); } catch(e) { inspection=undefined; host.message(`Editor checks failed: ${String(e)}`,true); }
    selection(); renderChecks();
    const s=editorSettings(host.get().job);
    $('#ed-sections').textContent=`Cut lines: ${s.sections?.map(x=>shortLength(x)).join(', ')||'none'}`;
    if (document.activeElement!==$('#ed-length')) $<HTMLInputElement>('#ed-length').value=String(inspection?.table_length??host.get().result.length);
    $('#ed-unplaced-count').textContent=inspection?`(${inspection.unplaced.length})`:'';
    const waiting=inspection?.unplaced.length??0;$('#ed-unplaced-chip').hidden=!waiting;$('#ed-unplaced-chip').innerHTML=`<span class="tag warn">${waiting} not placed</span>`;
    const focusIndex=(document.activeElement as HTMLElement)?.dataset.edUnplaced;
    $('#ed-unplaced').innerHTML=inspection?.unplaced.map((p,i)=>`<button data-ed-unplaced="${i}" aria-pressed="${armed===i}">${esc(host.get().job.pieces.find(q=>q.id===p.piece_id)?.name??p.piece_id)} · ${esc(host.get().job.pieces.find(q=>q.id===p.piece_id)?.size??'No size')} · ${p.mirrored?'mirrored':'normal'} · copy ${p.copy+1}${p.bundle==null?'':` · bundle ${p.bundle+1}`}</button>`).join('')||'<p>No unplaced copies.</p>';
    findAll<HTMLButtonElement>('[data-ed-unplaced]').forEach(b=>b.onclick=()=>{armed=Number(b.dataset.edUnplaced);mode='place';host.message('Choose a clear point on the canvas, or enter X/Y and press Place armed copy.');refresh();});
    if(focusIndex)find<HTMLElement>(`[data-ed-unplaced="${Number(focusIndex)}"]`)?.focus({preventScroll:true});
  }
  function selection(): void { $('#ed-selected').textContent=`${host.selection().length} selected`; }
  function place(at: Pt): void { if(armed<0)throw new Error('Choose an unplaced copy first.');const before=host.get().result.placements.length;run({op:'place',unplaced:armed,at});if(host.get().result.placements.length>before){armed=-1;setMode('move');refresh();} }
  function point(at: Pt): boolean {
    if(mode==='place'){guarded(()=>place(at));return true;}
    if(mode!=='measure')return false;
    if(!measureStart||measureEnd){measureStart=at;measureEnd=undefined;$<HTMLInputElement>('#ed-from-x').value=String(at[0]);$<HTMLInputElement>('#ed-from-y').value=String(at[1]);host.message('Choose the second measurement point.');}
    else {measureEnd=at;$<HTMLInputElement>('#ed-to-x').value=String(at[0]);$<HTMLInputElement>('#ed-to-y').value=String(at[1]);run({op:'measure',from:measureStart,to:measureEnd});}host.redraw();return true;
  }
  function overlay(width: number, unit: number): string {
    const s=editorSettings(host.get().job), length=inspection?.table_length??host.get().result.length;
    return (s.sections??[]).map(x=>`<line x1="${x}" x2="${x}" y1="0" y2="${width}" stroke="#a03b7c" stroke-width="${2*unit}" stroke-dasharray="${6*unit} ${4*unit}" pointer-events="none"/>`).join('')+`<rect data-length-handle="true" role="slider" tabindex="0" aria-label="Marker end (mm)" aria-valuenow="${length}" aria-valuemin="1" aria-valuemax="${host.get().job.fabric.max_length??1000000}" x="${length-4*unit}" y="0" width="${8*unit}" height="${width}" fill="#c26912" opacity=".65" style="cursor:ew-resize"><title>Drag marker end, use Left/Right arrows, or enter Table end X.</title></rect>`+(measureStart&&measureEnd?`<line x1="${measureStart[0]}" y1="${width-measureStart[1]}" x2="${measureEnd[0]}" y2="${width-measureEnd[1]}" stroke="#1649ac" stroke-width="${2*unit}" pointer-events="none"/>`:'');
  }
  function key(event: KeyboardEvent): boolean {
    const k=event.key.toLowerCase();
    if ((event.target as Element).hasAttribute('data-length-handle') && ['ArrowLeft','ArrowRight'].includes(event.key)) {
      const step=event.shiftKey?10:n('#ed-step');
      run({op:'length',length:(inspection?.table_length??host.get().result.length)+(event.key==='ArrowRight'?step:-step)});
      find<SVGElement>('[data-length-handle]')?.focus();return true;
    }
    if(k==='?'){clicks['#ed-help']!();return true;}
    if(k==='escape'){setMode('move');return false;}
    const actions: Record<string, () => void>={a:clicks['#ed-all']!,v:()=>setMode('select'),g:()=>setMode('move'),m:()=>setMode('measure'),f:clicks['#ed-flip']!,s:clicks['#ed-swap']!,c:clicks['#ed-compact']!,e:clicks['#ed-close-end']!,'[':clicks['#ed-tilt-minus']!,']':clicks['#ed-tilt-plus']!,delete:clicks['#ed-remove']!,enter:()=>mode==='place'?guarded(()=>place([n('#ed-to-x'),n('#ed-to-y')])):measure()};
    if(actions[k]){actions[k]!();return true;}
    const directions: Record<string, [string,Pt]>={ArrowLeft:['left',[-1,0]],ArrowRight:['right',[1,0]],ArrowUp:['up',[0,1]],ArrowDown:['down',[0,-1]]};
    const d=directions[event.key];if(d&&host.selection().length){if(event.altKey)run({op:'bump',direction:d[0]});else {const step=event.shiftKey?10:n('#ed-step');run({op:'move',delta:[d[1][0]*step,d[1][1]*step]});}return true;}return false;
  }
  renderLibrary();
  return {run,refresh,selection,point,overlay,key,mode:()=>mode,inspection:()=>inspection,
    preview: (original: DocumentState, action: Record<string,unknown>): DocumentState => documentFromEdit(core().apply(original,cleanSelection(host.selection(),original.result.placements.length),action)),
  };
}
