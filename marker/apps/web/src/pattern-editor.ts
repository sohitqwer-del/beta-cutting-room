import { apiFetch } from './transport';
import type { PatternCore, PatternDocument, PatternInspection, PatternToolSpec, PatternToolParameter, PatternPiece, PatternEdge, Json, Pt } from '@beta/marker-wasm';
import { PatternHistory, appendPieceEdge, blankPattern, constructionOrder, deleteConstruction, dependencyDelete, formulaOptions, insertFormula, rememberObject, value } from './pattern-state';
import { csvRows } from './cutting-room-data';
import './pattern-editor.css';
import { productionTools, type ProductionResult } from './pattern-production';
const esc = (v: unknown): string => String(v ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]!));
const err = (e: unknown): string => e instanceof Error ? e.message : String(e);
const colour = [0, 1, 2, 3, 4, 5].map(i => `var(--size-${i})`);
interface Host {
    get(): PatternDocument;
    saved(): boolean;
    ready(): boolean;
    core(): PatternCore | undefined;
    accept(doc: PatternDocument): Promise<void>;
    call<T>(path: string, body: unknown): Promise<T>;
    download(data: Blob | string, name: string, mime?: string): void;
    screen(id: number): void;
    exportDxf(): Promise<void>;
    sendMarker(ratio: Record<string, number>): Promise<void>;
}
interface Trace {
    data: string;
    width: number;
    height: number;
    scale: number;
    x: number;
    y: number;
    opacity: number;
    calibrated: boolean;
}
type Mode = 'draft' | 'measurements' | 'piece' | 'grading' | 'checks' | 'output' | 'guide';
/** Catalog-driven drafting surface. Rust owns formula, construction and piece geometry. */
export function installPatternEditor(root: HTMLElement, host: Host) {
    let doc = host.get(), history = new PatternHistory(doc), inspection: PatternInspection | undefined, catalog: PatternToolSpec[] = [];
    let mode: Mode = 'draft', size = doc.base_size, selected = '', pieceId = doc.pieces[0]?.id ?? '', tool: PatternToolSpec | undefined, editing = false, working = false;
    let edges: PatternEdge[] = [], collecting = false, reference = '', showLabels = true, panMode = false;
    let view = { x: -60, y: -700, w: 950, h: 800 }, trace: Trace | undefined, calibrating: Pt[] | undefined;
    let drag: {
        start: Pt;
        view: typeof view;
        id: string;
        at: Pt;
        moved: boolean;
    } | undefined;
    let lastMs = 0, renderGeneration = 0;
    root.innerHTML = `<div class="pd-toolbar"><button id="pd-new">New draft</button><button id="pd-undo" title="Undo (Ctrl+Z)">Undo</button><button id="pd-redo" title="Redo (Ctrl+Shift+Z)">Redo</button><label>View size<select id="pd-size"></select></label><label>Grid<select id="pd-grid"><option value="10">1 cm</option><option value="5">5 mm</option><option value="1">1 mm</option><option value="0">Off</option></select></label><label class="check"><input type="checkbox" id="pd-snap-points" checked>Snap points</label><label class="check"><input type="checkbox" id="pd-snap-lines" checked>Snap lines</label><button id="pd-fit">Fit</button><button id="pd-minus" aria-label="Zoom draft out">−</button><button id="pd-plus" aria-label="Zoom draft in">+</button><button id="pd-pan" aria-pressed="false">Pan</button><button id="pd-labels" aria-pressed="true">Labels</button></div>
  <nav class="pd-modes" aria-label="Pattern modes">${(['draft', 'measurements', 'piece', 'grading', 'checks', 'output', 'guide'] as Mode[]).map(m => `<button data-mode="${m}" aria-pressed="${m === 'draft'}">${({ draft: 'Draft', measurements: 'Measurements', piece: 'Piece', grading: 'Grading', checks: 'Seam checks', output: 'Output', guide: 'First shirt guide' })[m]}</button>`).join('')}</nav>
  <p id="pd-status" role="status" aria-live="polite">Loading local draft tools…</p>
  <div class="pd-layout"><aside class="pd-palette" aria-label="Drafting tools"><h2>Draft tools</h2><input id="pd-search" type="search" aria-label="Find a drafting tool" placeholder="Find a tool"><div id="pd-tools"></div></aside>
  <section class="pd-stage"><div class="pd-canvas-heading"><strong id="pd-title"></strong><span id="pd-engine"></span></div><svg id="pd-canvas" tabindex="0" role="group" aria-label="Pattern drafting canvas. Tab to objects. Enter selects. Drag numeric free points. Shift-drag pans. Wheel zooms."></svg><div class="pd-canvas-foot"><span id="pd-cursor">Millimetres · y points up</span><span>Wheel: zoom · Shift-drag: pan · Esc: cancel</span></div><div id="pd-controls"></div></section>
  <aside class="pd-history" aria-label="Construction history"><h2>Constructions</h2><p class="hint">Select to highlight and edit.</p><ol id="pd-history"></ol><div class="pd-actions"><button id="pd-edit">Edit selected</button><button id="pd-delete">Delete selected</button></div><div id="pd-checks" aria-live="polite"></div></aside></div>
  <dialog id="pd-tool-dialog" aria-labelledby="pd-tool-title"><form id="pd-tool-form"><h2 id="pd-tool-title"></h2><p id="pd-tool-help"></p><label>Object ID<input id="pd-object-id" required pattern="[A-Za-z_][A-Za-z0-9_.]*" autocomplete="off"></label><div id="pd-tool-fields" class="fields"></div><label>Measurement / variable picker<select id="pd-picker"><option value="">Choose a value to insert</option></select></label><p id="pd-formula-check" role="status" aria-live="polite"></p><div class="pd-actions"><button type="submit" class="primary">Apply tool</button><button type="button" id="pd-tool-cancel">Cancel</button></div></form></dialog>
  <dialog id="pd-delete-dialog" aria-labelledby="pd-delete-title"><h2 id="pd-delete-title">Delete construction and dependants?</h2><p id="pd-delete-details"></p><div class="pd-actions"><button id="pd-delete-confirm">Delete listed items</button><button id="pd-delete-cancel">Keep items</button></div></dialog><datalist id="pd-formulas"></datalist>`;
    const $ = <T extends Element = HTMLElement>(s: string): T => root.querySelector<T>(s)!;
    const input = (s: string): HTMLInputElement => $<HTMLInputElement>(s);
    const text = (s: string): string => input(s).value;
    const canvas = $<SVGSVGElement>('#pd-canvas'), dialog = $<HTMLDialogElement>('#pd-tool-dialog');
    let focusedFormula: HTMLInputElement | undefined;
    const status = (s: string, error = false) => { $('#pd-status').textContent = s; $('#pd-status').classList.toggle('error-text', error); };
    async function run(action: () => void | Promise<void>) { if (working)
        return; working = true; root.setAttribute('aria-busy', 'true'); try {
        await action();
    }
    catch (e) {
        status(err(e), true);
    }
    finally {
        working = false;
        root.removeAttribute('aria-busy');
    } }
    async function inspect(next: PatternDocument, currentSize = size): Promise<PatternInspection> { return host.core()?.inspect(next, currentSize) ?? host.call('pattern/inspect', { document: next, size: currentSize }); }
    const production = productionTools(root, {
        get: () => doc, commit, run, status, download: host.download,
        action: async request => (host.core()?.piece_action(doc, request) as ProductionResult | undefined)
            ?? host.call<ProductionResult>('pattern/piece-action', { ...request, document: doc }),
    });
    async function commit(next: PatternDocument, record = true) {
        const start = performance.now();
        for (const s of next.sizes)
            await inspect(next, s);
        await host.accept(next);
        if (record)
            history.apply(next);
        doc = next;
        lastMs = performance.now() - start;
        await refresh();
        status(`${host.saved() ? 'Saved on this computer.' : 'Browser storage is full. Save pattern JSON now.'} ${Object.keys(doc.drawing).length} constructions · ${lastMs.toFixed(1)} ms edit · ${doc.sizes.length} sizes.`);
    }
    function options(items: string[], current = ''): string { return items.map(n => `<option ${n === current ? 'selected' : ''} value="${esc(n)}">${esc(n)}</option>`).join(''); }
    function field(id: string, label: string, v: unknown, type = 'text', extra = ''): string { return `<label>${label}<input id="${id}" value="${esc(v)}" type="${type}" ${extra}></label>`; }
    function select(id: string, label: string, items: string[], current = ''): string { return `<label>${label}<select id="${id}">${options(items, current)}</select></label>`; }
    function pointIds() { return Object.keys(doc.drawing).filter(n => inspection?.objects[n]?.kind === 'point' || doc.drawing[n]?.kind === 'point'); }
    function pathIds() { return Object.keys(doc.drawing).filter(n => inspection?.objects[n]?.kind === 'path' || (!inspection?.objects[n] && doc.drawing[n]?.kind !== 'point')); }
    function currentPiece(): PatternPiece | undefined { return doc.pieces.find(p => p.id === pieceId) ?? doc.pieces[0]; }
    function updateToolbar() {
        $('#pd-title').textContent = doc.name;
        $('#pd-engine').textContent = `${host.core() ? 'Rust WASM' : 'Local Rust server'} · ${lastMs.toFixed(1)} ms`;
        $('#pd-size').innerHTML = options(doc.sizes, size);
        input('#pd-undo').disabled = !history.canUndo;
        input('#pd-redo').disabled = !history.canRedo;
        $('#pd-history').innerHTML = constructionOrder(doc).map(id => `<li><button data-object="${esc(id)}" aria-pressed="${id === selected}"><b>${esc(id)}</b><small>${esc(doc.drawing[id]?.kind === 'point' ? (doc.drawing[id] as {
            operation: {
                op: string;
            };
        }).operation.op : doc.drawing[id]?.kind)}</small></button></li>`).join('');
        $('#pd-edit').toggleAttribute('disabled', !selected);
        $('#pd-delete').toggleAttribute('disabled', !selected);
        $('#pd-checks').innerHTML = `<h3>Piece checks</h3>${(inspection?.pieces ?? []).map(p => `<p class="${p.ok ? 'pd-ok' : 'error-text'}">${esc(doc.pieces.find(q => q.id === p.id)?.name ?? p.id)}: ${p.ok ? 'Closed · no crossing edges' : esc(p.error)}</p>`).join('') || '<p class="hint">Build a piece from connected paths to check it.</p>'}`;
    }
    function palette() {
        const q = text('#pd-search').toLowerCase(), groups = ['Points', 'Lines', 'Curves', 'Arcs', 'Operations'];
        const toolGroup = (t: PatternToolSpec) => t.group === 'Operations' ? 'Operations' : t.point ? 'Points' : t.id.includes('arc') ? 'Arcs' : t.id === 'line' ? 'Lines' : 'Curves';
        const all = [...new Set([...groups, ...catalog.map(toolGroup)])];
        $('#pd-tools').innerHTML = all.map(group => { const tools = catalog.filter(t => toolGroup(t) === group && `${t.name} ${t.help}`.toLowerCase().includes(q)); return tools.length ? `<details open><summary>${esc(group[0]!.toUpperCase() + group.slice(1))}</summary>${tools.map(t => `<button data-tool="${esc(t.id)}" title="${esc(t.help)}">${esc(t.name)}</button>`).join('')}</details>` : ''; }).join('') || '<p class="hint">No matching tools. Build the pattern WASM assets if tools have not loaded.</p>';
        $('#pd-tools').insertAdjacentHTML('beforeend', '<details open><summary>Pieces</summary><button id="pd-piece-mode">Build or edit a piece</button></details>');
        $('#pd-piece-mode').onclick = () => void run(() => setMode('piece'));
    }
    async function refresh() {
        if (!host.ready()) {
            updateToolbar();
            draw();
            renderControls();
            return;
        }
        const generation = ++renderGeneration;
        doc = host.get();
        if (reference && !doc.drawing[reference]) {
            reference = '';
            alignedNest = undefined;
        }
        if (!doc.sizes.includes(size))
            size = doc.base_size;
        try {
            const output = await inspect(doc);
            if (generation !== renderGeneration)
                return;
            inspection = output;
            if (reference && output.objects[reference]?.kind !== 'point') {
                reference = '';
                alignedNest = undefined;
            }
            if ($('#pd-status').classList.contains('error-text'))
                status('Draft ready. Choose a tool or select a construction.');
        }
        catch (e) {
            status(err(e), true);
        }
        if (!catalog.length) {
            try {
                catalog = host.core()?.catalog() ?? await host.call<PatternToolSpec[]>('pattern/catalog', {});
                palette();
            }
            catch (e) {
                if (host.core())
                    status(`Tool catalog: ${err(e)}`, true);
            }
        }
        if (mode === 'grading') {
            try {
                const grades: Record<string, PatternInspection> = {};
                for (const s of doc.sizes) grades[s] = await inspect(doc, s);
                const aligned = reference ? (host.core()?.nest(doc, reference) ?? await host.call<ReturnType<PatternCore['nest']>>('pattern/nest', { document: doc, reference })) : undefined;
                if (generation !== renderGeneration) return;
                gradedCache = grades;
                alignedNest = aligned;
            } catch (e) {
                if (generation !== renderGeneration) return;
                gradedCache = {};
                alignedNest = undefined;
                status(err(e), true);
            }
        }
        updateToolbar();
        draw();
        renderControls();
    }
    function fit() {
        const points = mode === 'grading' ? doc.sizes.flatMap(s => (alignedNest?.sizes[s] ?? gradedCache[s]?.pieces.flatMap(p => p.graded ? [p.graded] : []) ?? []).flatMap(p => p.piece.outline)) : Object.values(inspection?.objects ?? {}).flatMap(o => o.points);
        if (!points.length) {
            view = { x: -60, y: -700, w: 950, h: 800 };
            draw();
            return;
        }
        const xs = points.map(p => p[0]), ys = points.map(p => -p[1]);
        const minX = Math.min(...xs), minY = Math.min(...ys), w = Math.max(...xs) - minX, h = Math.max(...ys) - minY;
        const padding = Math.max(w, h, 100) * .12;
        view = { x: minX - padding, y: minY - padding, w: Math.max(w + padding * 2, 100), h: Math.max(h + padding * 2, 100) };
        draw();
    }
    const path = (p: Pt[]) => p.map(([x, y]) => `${x},${-y}`).join(' ');
    function draw() {
        canvas.setAttribute('viewBox', `${view.x} ${view.y} ${view.w} ${view.h}`);
        const grid = Number(text('#pd-grid')), unit = Math.max(1, view.w / 900), r = 4 * unit;
        const defs = `<defs><pattern id="pd-grid-pattern" width="${grid || 10}" height="${grid || 10}" patternUnits="userSpaceOnUse"><path d="M ${grid || 10} 0 H 0 V ${grid || 10}" fill="none" style="stroke:var(--paper-grid)" stroke-width="${unit * .6}"/></pattern></defs>`;
        let html = defs + `<rect x="${view.x}" y="${view.y}" width="${view.w}" height="${view.h}" style="fill:${grid ? 'url(#pd-grid-pattern)' : 'var(--paper)'}"/><path d="M ${view.x} 0 H ${view.x + view.w} M 0 ${view.y} V ${view.y + view.h}" style="stroke:var(--paper-edge)" stroke-width="${unit}"/>`;
        if (trace)
            html += `<image href="${esc(trace.data)}" x="${trace.x}" y="${-trace.y - trace.height * trace.scale}" width="${trace.width * trace.scale}" height="${trace.height * trace.scale}" opacity="${trace.opacity}" pointer-events="none"/>`;
        if (mode === 'grading') {
            const aligned = alignedNest;
            // Nest alignment is performed by the engine's formula context below.
            for (let i = 0; i < doc.sizes.length; i++) {
                const s = doc.sizes[i]!, result = gradedCache[s];
                html += (aligned?.sizes[s] ?? result?.pieces.flatMap(p => p.graded ? [p.graded] : []) ?? []).map(p => `<polygon points="${path(p.piece.outline)}" fill="none" style="stroke:${colour[i % colour.length]}" stroke-width="${1.8 * unit}"><title>${esc(s)} ${esc(p.piece.name)}</title></polygon>`).join('');
            }
        }
        else {
            if (mode === 'piece')
                html += (inspection?.pieces ?? []).map(p => p.graded ? `<g><polygon points="${path(p.graded.piece.outline)}" style="fill:var(--draw-fill);stroke:var(--size-0)" stroke-width="${unit * 1.7}"/><polygon points="${path(p.graded.seam)}" fill="none" style="stroke:var(--draw-seam)" stroke-dasharray="${5 * unit} ${3 * unit}"/>${p.graded.piece.grain ? `<polyline points="${path(p.graded.piece.grain)}" fill="none" style="stroke:var(--draw-grain)" stroke-width="${unit * 2}"/>` : ''}${p.graded.piece.fold_line ? `<polyline points="${path(p.graded.piece.fold_line)}" fill="none" style="stroke:var(--draw-grain)" stroke-width="${unit * 2}" stroke-dasharray="${6 * unit} ${4 * unit}"><title>Fold line</title></polyline>` : ''}${(p.graded.piece.marks ?? []).map(m => m.points.length === 1 ? `<circle cx="${m.points[0]![0]}" cy="${-m.points[0]![1]}" r="${unit * 3}" style="fill:var(--draw-mark)"><title>${esc(m.text ?? m.kind)}</title></circle>` : `<polyline points="${path(m.points)}" fill="none" style="stroke:var(--draw-mark)" stroke-width="${unit * 3}"><title>${esc(m.text ?? m.kind)}</title></polyline>`).join('')}</g>` : '').join('');
            const hidden = new Set(Object.values(doc.groups ?? {}).filter(g => g.visible === false).flatMap(g => g.objects));
            const objects = Object.entries(inspection?.objects ?? {}).filter(([id]) => !hidden.has(id)).sort((a, b) => Number(a[1].kind === 'point') - Number(b[1].kind === 'point'));
            html += objects.map(([id, o]) => {
                const chosen = id === selected || edges.some(e => e.object === id), c = chosen ? 'var(--draw-select)' : o.kind === 'point' ? 'var(--draw-point)' : 'var(--draw-line)';
                const p = o.points[0];
                if (!p)
                    return '';
                const label = o.kind === 'point' ? p : o.points.length === 2 ? [(p[0] + o.points[1]![0]) / 2, (p[1] + o.points[1]![1]) / 2] : o.points[Math.floor(o.points.length / 2)]!;
                return `<g data-canvas-object="${esc(id)}" role="button" tabindex="0" aria-label="${esc(id)} ${o.kind}" aria-pressed="${chosen}"><title>${esc(id)} · ${esc(doc.drawing[id]?.kind)}</title>${o.kind === 'point' ? `<circle cx="${p[0]}" cy="${-p[1]}" r="${chosen ? r * 1.6 : r}" style="fill:${c};stroke:var(--draw-halo)" stroke-width="${unit}"/>` : `<polyline points="${path(o.points)}" fill="none" stroke="transparent" stroke-width="${unit * 13}"/><polyline points="${path(o.points)}" fill="none" style="stroke:${c}" stroke-width="${unit * (chosen ? 2.8 : 1.3)}" pointer-events="none"/>`}${showLabels ? `<text x="${label[0]! + r * 2}" y="${-label[1]! - r * 2}" font-size="${12 * unit}"  style="fill:var(--draw-label);stroke:var(--draw-halo)" paint-order="stroke" stroke-width="${unit * 3}" pointer-events="none">${esc(id)}</text>` : ''}</g>`;
            }).join('');
        }
        if (calibrating)
            html += calibrating.map(p => `<circle cx="${p[0]}" cy="${-p[1]}" r="${r * 2}" style="fill:var(--draw-select)"/>`).join('');
        canvas.innerHTML = html;
    }
    let alignedNest: ReturnType<PatternCore['nest']> | undefined;
    let gradedCache: Record<string, PatternInspection> = {};
    async function setMode(next: Mode) { mode = next; root.dataset.patternMode = mode; root.querySelectorAll<HTMLButtonElement>('[data-mode]').forEach(b => b.setAttribute('aria-pressed', String(b.dataset.mode === mode))); $('.pd-palette').hidden = !['draft', 'piece'].includes(mode); await refresh(); if (mode === 'grading')
        fit();
    else
        draw(); }
    function renderControls() {
        const p = currentPiece();
        if (p)
            pieceId = p.id;
        if (mode === 'draft')
            $('#pd-controls').innerHTML = `<div class="pd-info"><h2>Draft from measurements</h2><p>Choose a tool. Click reference points or paths on the canvas, then enter lengths or formulas. All formula lengths are in mm. Free points with numeric coordinates can be dragged.</p><p>Selected: <strong>${esc(selected || 'none')}</strong></p></div><details><summary>Trace a photo or scan</summary><div class="fields"><label>Background PNG/JPEG<input type="file" id="pd-trace-file" accept="image/png,image/jpeg"></label>${field('pd-known-distance', 'Known distance (mm)', 100, 'number', 'min="0.01" step="any"')}${field('pd-trace-opacity', 'Image opacity', trace?.opacity ?? .45, 'range', 'min="0.1" max="1" step="0.05"')}</div><button id="pd-calibrate" ${trace ? '' : 'disabled'}>Set scale with two image points</button><button id="pd-trace-clear">Remove background</button><button id="pd-digitize">Open Digitize</button><p class="hint">${trace?.calibrated ? 'Scale calibrated. Trace points, lines and curves over the image.' : 'Choose two image positions a known distance apart. Photo and scale are included in the JSON backup.'}</p></details>`;
        if (mode === 'draft')
            $('#pd-controls').insertAdjacentHTML('beforeend', `<details><summary>Drawing groups</summary>${Object.entries(doc.groups ?? {}).map(([id, g]) => `<label class="check"><input data-pd-group="${esc(id)}" type="checkbox" ${g.visible === false ? '' : 'checked'}>${esc(g.name)} (${g.objects.length} objects)</label>`).join('') || '<p>Use a move, rotate or mirror tool with Apply to group.</p>'}</details>`);
        if (mode === 'measurements')
            renderMeasurements();
        if (mode === 'piece') {
            renderPiece(p);
            if (p)
                $('#pd-controls').insertAdjacentHTML('beforeend', pieceMenu());
            if (p)
                $('#pd-controls').insertAdjacentHTML('beforeend', production.marks(p.id));
        }
        if (mode === 'checks')
            $('#pd-controls').innerHTML = production.checks();
        if (mode === 'grading')
            renderGrading(p);
        if (mode === 'grading')
            $('#pd-controls').insertAdjacentHTML('beforeend', production.rules());
        if (mode === 'output')
            $('#pd-controls').innerHTML = `<h2>Save and cut</h2><p>Save pattern JSON above to keep formulas, rules, measurements and the trace photo.</p><div class="pd-actions"><button id="pd-export-dxf">DXF-AAMA / ASTM-style</button><label>PDF paper<select id="pd-paper"><option>A4</option><option>A3</option></select></label><button id="pd-export-pdf">Tiled 1:1 PDF</button><button id="pd-export-plt">Plotter PLT</button><button id="pd-send-marker" class="primary">Send sizes to marker</button><button id="pd-import">Import from Seamly2D</button></div><p>DXF includes all sizes and BETA's public grade-rule comments. Other CAD systems may read the outlines without importing native grade rules. PDF uses physical A4/A3 pages and a separate 100 mm test-square page and 10 mm overlaps between tiles. Print at Actual size / 100%.</p><div id="pd-output-ratios" class="fields">${doc.sizes.map(s => field(`pd-ratio-${doc.sizes.indexOf(s)}`, `${esc(s)} garments`, s === doc.base_size ? 1 : 0, 'number', 'min="0" max="100000" step="1"')).join('')}</div>`;
        if (mode === 'guide')
            $('#pd-controls').innerHTML = `<h2>Draft your first shirt block in BETA</h2><p>This teaching block explains the workflow. Fit-test a sample before production.</p><ol class="pd-guide"><li><strong>Set measurements.</strong> Open Measurements. Enter chest, waist, back length, neck, shoulder and arm depth for each size, in mm. Add an ease variable for room around the body.</li><li><strong>Place the origin.</strong> In Draft choose Absolute point. Name it A and use x = 0, y = 0.</li><li><strong>Mark the waist and chest.</strong> Add B at x = (m.waist+v.ease)/4, y = 0. Add C at x = (m.chest+v.ease)/4, y = m.back_length-m.arm_depth.</li><li><strong>Mark the shoulder and neck.</strong> Add D at x = m.neck/6+m.shoulder, y = m.back_length-25. Add E at x = m.neck/6, y = m.back_length. Add F at x = 0, y = m.back_length-m.neck/6-10.</li><li><strong>Connect the block.</strong> Use Line for A–B, B–C, D–E and F–A. Add control points and use Cubic curve for C–D (armhole) and E–F (neck). The four point inputs are start, two controls and end.</li><li><strong>Make a piece.</strong> Open Piece, start a path, then click the six edges in order. Reverse an edge if its direction opposes the path. Finish only when the path closes.</li><li><strong>Add cutting details.</strong> Set 10 mm seam allowance, grain, quantity and pairs. Add side-seam notches and a name label.</li><li><strong>Grade.</strong> Choose a second size and compare the nest. Attach factory dx/dy rules only when measurement-based grading is not suitable.</li><li><strong>Check and export.</strong> Read the inline piece checks. Save JSON, export a DXF or print a 1:1 PDF. Check the 100 mm square with a ruler. Send a size ratio to Marker.</li></ol><button id="pd-teaching">Load the teaching bodice</button>`;
        bindControls();
        production.bind();
    }
    function renderMeasurements() {
        const keys = [...new Set(Object.values(doc.measurements).flatMap(r => Object.keys(r)))];
        $('#pd-controls').innerHTML = `<h2>Measurements</h2><p>Use mm. Edit each size for multisize or individual measurements. Increments start from the base value and follow size order.</p><div class="fields">${field('pd-doc-name', 'Pattern name', doc.name)}${field('pd-doc-label', 'Cutting instruction', doc.label ?? '')}${select('pd-base-size', 'Base size', doc.sizes, doc.base_size)}${field('pd-new-size', 'New size', '')}<button id="pd-add-size">Add size from base</button></div><div class="table-scroll"><table><caption>Body measurements (mm)</caption><thead><tr><th>Measurement</th>${doc.sizes.map(s => `<th>${esc(s)}</th>`).join('')}<th>Step per size</th></tr></thead><tbody>${keys.map(k => `<tr><th>${esc(k)}</th>${doc.sizes.map(s => `<td><input type="number" step="any" data-pd-measure="${esc(k)}" data-size="${esc(s)}" value="${esc(doc.measurements[s]?.[k] ?? '')}" aria-label="${esc(k)} ${esc(s)} mm"></td>`).join('')}<td><input type="number" step="any" value="${esc((doc.meta?.measurement_steps as Record<string, Json> | undefined)?.[k] ?? '')}" data-pd-increment="${esc(k)}" aria-label="${esc(k)} increment"><button data-apply-increment="${esc(k)}">Apply step</button></td></tr>`).join('')}</tbody></table></div><div class="fields">${field('pd-measure-name', 'New measurement name', '')}${field('pd-measure-value', 'Base measurement (mm)', 0, 'number', 'step="any"')}<button id="pd-add-measure">Add measurement</button><label>Import CSV or Seamly measurements<input type="file" id="pd-measure-file" accept=".csv,.smis,.vit,.vst,.smms"></label><button id="pd-measure-export">Export measurement CSV</button></div><h3>Variables and ease</h3><div class="fields">${Object.entries(doc.variables).map(([k, v]) => `<label>${esc(k)}<input data-pd-variable="${esc(k)}" value="${esc(v)}" list="pd-formulas"></label>`).join('')}${field('pd-variable-name', 'New variable name', '')}${field('pd-variable-value', 'Number or formula', 0)}<button id="pd-add-variable">Add variable</button></div>`;
    }
    function renderPiece(p: PatternPiece | undefined) {
        $('#pd-controls').innerHTML = `<h2>Piece builder</h2><p>Click paths in traversal order. Reverse an edge when needed. A piece must close without crossing itself.</p><div class="pd-actions"><button id="pd-piece-start">Start a new piece</button><label>Piece name<input id="pd-piece-new-name" value="New piece"></label><button id="pd-piece-finish" ${edges.length >= 2 ? '' : 'disabled'}>Finish closed piece</button><button id="pd-piece-cancel">Cancel path</button><button id="pd-internal-finish">Save path inside selected piece</button></div><ol>${edges.map((e, i) => `<li>${esc(e.object)} ${e.reversed ? '(reverse)' : ''} <button data-reverse-edge="${i}">Reverse</button><button data-remove-edge="${i}">Remove</button></li>`).join('')}</ol><p role="status">${collecting ? 'Click a path on the canvas or choose a path below.' : ''}</p>${select('pd-path-add', 'Path to append', ['', ...pathIds()])}<button id="pd-add-path">Append path</button>${p ? `<h3>Piece properties</h3>${select('pd-piece-select', 'Edit piece', doc.pieces.map(p => p.id), p.id)}<form id="pd-piece-properties"><div class="fields">${field('pd-piece-name', 'Name', p.name)}${field('pd-piece-quantity', 'Quantity', p.quantity ?? 1, 'number', 'min="0" max="100000" step="1"')}${field('pd-piece-pairs', 'Extra mirrored copies (pairs)', p.mirror_quantity ?? 0, 'number', 'min="0" max="100000" step="1"')}${field('pd-piece-material', 'Material', p.material ?? '')}${select('pd-grain-a', 'Grain from', ['', ...pointIds()], p.grain?.[0])}${select('pd-grain-b', 'Grain to', ['', ...pointIds()], p.grain?.[1])}${select('pd-fold-a', 'Fold from', ['', ...pointIds()], p.fold_line?.[0])}${select('pd-fold-b', 'Fold to', ['', ...pointIds()], p.fold_line?.[1])}${select('pd-label-point', 'Label position', ['', ...pointIds()], p.label_point ?? '')}</div><div class="table-scroll"><table><caption>Per-edge seam allowance and corners</caption><thead><tr><th>Edge</th><th>Length (mm)</th><th>Allowance (mm or formula)</th><th>Start corner</th></tr></thead><tbody>${p.edges.map((e, i) => `<tr><th>${esc(e.object)}</th><td>${inspection?.pieces.find(q => q.id === p.id)?.edges?.[i]?.length_mm.toFixed(2) ?? '?'}</td><td><input data-allowance="${i}" value="${esc(e.allowance ?? 0)}" list="pd-formulas"></td><td><select data-corner="${i}">${options(['miter', 'bevel', 'square', 'mirrored'], e.corner ?? 'miter')}</select></td></tr>`).join('')}</tbody></table></div><button type="submit">Save piece properties</button></form><h3>Marks and internal paths</h3><form id="pd-mark-form"><div class="fields">${select('pd-mark-kind', 'Mark type', ['notch', 'drill', 'button', 'line', 'text', 'seam_intersections', 'anchor'])}${select('pd-mark-point', 'Point / line start', pointIds())}${select('pd-mark-end', 'Line end', pointIds())}${field('pd-mark-edge', 'Notch edge (starts at 1)', 1, 'number', `min="1" max="${p.edges.length}" step="1"`)}${field('pd-mark-distance', 'Distance along edge (mm)', 20)}${field('pd-mark-depth', 'Notch depth / button diameter (mm)', 3)}${field('pd-mark-text', 'Label / anchor name', p.name)}${select('pd-notch-shape', 'Notch shape', ['slit', 'v', 't', 'castle'])}${field('pd-notch-width', 'Notch width (mm)', 3)}${select('pd-mark-path', 'Internal or crossing path', pathIds())}</div><button type="submit">Add mark</button></form><ul>${(p.annotations ?? []).map((a, i) => `<li>${esc(a.kind)} ${'text' in a ? esc(a.text) : ''}<button data-remove-mark="${i}">Remove</button></li>`).join('')}</ul><ul>${(p.internal_paths ?? []).map(ip => `<li>Internal path: ${esc(ip.name)} (${ip.edges.length} edges)</li>`).join('')}${Object.entries(p.anchors ?? {}).map(([name, id]) => `<li>Anchor ${esc(name)}: ${esc(id)}</li>`).join('')}</ul><p class="hint">Size is selected above and included in DXF labels. Grain and fold lines use two existing points.</p>` : '<p>No pieces yet. Build one from the drawing paths.</p>'}`;
    }
    function renderGrading(p: PatternPiece | undefined) {
        const rules = Object.keys(doc.rules ?? {}), rule = rules[0], r = rule ? doc.rules?.[rule] : undefined;
        $('#pd-controls').innerHTML = `<h2>Grade sizes</h2><div class="pd-size-key">${doc.sizes.map((s, i) => `<span style="color:${colour[i % colour.length]}">━ ${esc(s)}</span>`).join('')}</div><p>Measurement grading re-evaluates every size. Factory rules move attached outline points. Internal marks follow their nearest stitch edge.</p><div class="fields">${select('pd-reference', 'Nest reference point', ['', ...pointIds()], reference)}${select('pd-grade-piece', 'Piece', doc.pieces.map(p => p.id), p?.id)}${select('pd-grade-point', 'Attach outline point', pointIds())}${select('pd-grade-rule', 'Rule', ['', ...rules])}<button id="pd-attach-rule">Attach rule</button><button id="pd-clear-rules">Use measurement grading for piece</button></div><ul>${Object.entries(p?.grade_points ?? {}).map(([k, v]) => `<li>${esc(k)} → ${esc(v)}</li>`).join('')}</ul><h3>Factory rule library</h3><div class="fields">${select('pd-rule-select', 'Edit rule', ['', ...rules], rule)}${field('pd-rule-id', 'Rule name', rule ?? 'grade_1')}${select('pd-rule-mode', 'Interpretation', ['relative', 'incremental'], r?.mode ?? 'relative')}</div><p>Relative: total movement from the base. Incremental: movement from the previous size.</p><div id="pd-rule-deltas"></div><button id="pd-rule-save">Save grade rule</button><button id="pd-rule-remove">Delete rule</button>`;
        renderDeltas(rule ?? '');
    }
    function renderDeltas(id: string) { const r = doc.rules?.[id]; $('#pd-rule-deltas').innerHTML = `<div class="table-scroll"><table><caption>Grade movement in mm</caption><thead><tr><th>Size</th><th>dx</th><th>dy</th></tr></thead><tbody>${doc.sizes.map((s, i) => `<tr><th>${esc(s)}</th><td><input id="pd-dx-${i}" type="number" step="any" value="${r?.deltas[s]?.[0] ?? 0}" aria-label="${esc(s)} dx"></td><td><input id="pd-dy-${i}" type="number" step="any" value="${r?.deltas[s]?.[1] ?? 0}" aria-label="${esc(s)} dy"></td></tr>`).join('')}</tbody></table></div>`; }
    function finite(s: string): number { const i = input(s), n = Number(i.value); if (!i.value.trim() || !i.checkValidity() || !Number.isFinite(n))
        throw new Error(`Check ${i.labels?.[0]?.textContent ?? s}.`); return n; }
    function validId(id: string) { if (!/^[A-Za-z_][A-Za-z0-9_.]*$/.test(id))
        throw new Error('Use a name starting with a letter; use letters, numbers, dots or underscores.'); }
    function bindControls() {
        const click = (id: string, f: () => void | Promise<void>) => { const e = root.querySelector<HTMLButtonElement>(id); if (e)
            e.onclick = () => void run(f); };
        const change = (id: string, f: () => void | Promise<void>) => { const e = root.querySelector<HTMLInputElement>(id); if (e)
            e.onchange = () => void run(f); };
        root.querySelectorAll<HTMLInputElement>('[data-pd-group]').forEach(i => i.onchange = () => void run(async () => { const next = structuredClone(doc); next.groups![i.dataset.pdGroup!]!.visible = i.checked; await commit(next); }));
        bindPieceMenu();
        click('#pd-digitize', () => host.screen(5));
        click('#pd-import', () => { window.document.querySelector<HTMLInputElement>('#pattern-file')?.click(); });
        click('#pd-teaching', () => { window.document.querySelector<HTMLButtonElement>('#pattern-starter')?.click(); });
        click('#pd-piece-start', () => { collecting = true; edges = []; renderControls(); draw(); status('Click paths in order. Use Append path for keyboard entry.'); });
        click('#pd-piece-cancel', () => { collecting = false; edges = []; renderControls(); draw(); });
        click('#pd-add-path', () => addEdge(text('#pd-path-add')));
        click('#pd-internal-finish', async () => { const p = currentPiece(); if (!p || !edges.length)
            throw new Error('Choose a piece and append one or more paths.'); const next = structuredClone(doc), target = next.pieces.find(q => q.id === p.id)!; target.internal_paths = [...target.internal_paths ?? [], { name: text('#pd-piece-new-name') || 'Internal path', edges: structuredClone(edges), closed: false }]; await commit(next); edges = []; collecting = false; renderControls(); draw(); });
        click('#pd-piece-finish', async () => {
            if (edges.length < 2)
                throw new Error('Choose connected path edges first.');
            const name = text('#pd-piece-new-name').trim();
            if (!name)
                throw new Error('Enter a piece name.');
            let n = 1;
            while (doc.pieces.some(p => p.id === `piece_${n}`))
                n++;
            const next = structuredClone(doc);
            pieceId = `piece_${n}`;
            next.pieces.push({ id: pieceId, name, edges: structuredClone(edges), quantity: 1, mirror_quantity: 0, annotations: [], grade_points: {} });
            const result = await inspect(next), check = result.pieces.find(p => p.id === pieceId);
            if (!check?.ok)
                throw new Error(check?.error ?? 'The piece path is incomplete.');
            await commit(next);
            edges = [];
            collecting = false;
            renderControls();
            draw();
        });
        root.querySelectorAll<HTMLButtonElement>('[data-reverse-edge]').forEach(b => b.onclick = () => { const e = edges[Number(b.dataset.reverseEdge)]; if (e)
            e.reversed = !e.reversed; renderControls(); draw(); });
        root.querySelectorAll<HTMLButtonElement>('[data-remove-edge]').forEach(b => b.onclick = () => { edges.splice(Number(b.dataset.removeEdge), 1); renderControls(); draw(); });
        change('#pd-piece-select', () => { pieceId = text('#pd-piece-select'); renderControls(); });
        const properties = root.querySelector<HTMLFormElement>('#pd-piece-properties');
        if (properties)
            properties.onsubmit = e => {
                e.preventDefault();
                void run(async () => {
                    const next = structuredClone(doc), p = next.pieces.find(p => p.id === pieceId)!;
                    p.name = text('#pd-piece-name').trim();
                    if (!p.name)
                        throw new Error('Enter a piece name.');
                    p.quantity = finite('#pd-piece-quantity');
                    p.mirror_quantity = finite('#pd-piece-pairs');
                    p.material = text('#pd-piece-material') || null;
                    p.grain = pair('#pd-grain-a', '#pd-grain-b');
                    p.fold_line = pair('#pd-fold-a', '#pd-fold-b');
                    p.label_point = text('#pd-label-point') || null;
                    p.edges.forEach((edge, i) => { edge.allowance = value(text(`[data-allowance="${i}"]`)); edge.corner = text(`[data-corner="${i}"]`) as PatternEdge['corner']; });
                    await commit(next);
                });
            };
        const marks = root.querySelector<HTMLFormElement>('#pd-mark-form');
        if (marks)
            marks.onsubmit = e => {
                e.preventDefault();
                void run(async () => {
                    const next = structuredClone(doc), p = next.pieces.find(p => p.id === pieceId)!;
                    p.annotations ??= [];
                    const kind = text('#pd-mark-kind');
                    if (kind === 'notch')
                        p.annotations.push({ kind: 'shaped_notch', edge: finite('#pd-mark-edge') - 1, distance: value(text('#pd-mark-distance')), depth: value(text('#pd-mark-depth')), width: value(text('#pd-notch-width')), shape: text('#pd-notch-shape') as 'slit' | 'v' | 't' | 'castle' });
                    else if (kind === 'seam_intersections')
                        p.annotations.push({ kind: 'seam_intersections', path: text('#pd-mark-path'), depth: value(text('#pd-mark-depth')), width: value(text('#pd-notch-width')), shape: text('#pd-notch-shape') as 'slit' | 'v' | 't' | 'castle' });
                    else if (kind === 'anchor') {
                        const name = text('#pd-mark-text').trim();
                        if (!name)
                            throw new Error('Enter an anchor name.');
                        p.anchors = { ...p.anchors, [name]: text('#pd-mark-point') };
                    }
                    else if (kind === 'button')
                        p.annotations.push({ kind: 'button', point: text('#pd-mark-point'), diameter: value(text('#pd-mark-depth')) });
                    else
                        p.annotations.push({ kind: 'points', mark: kind as 'drill' | 'line' | 'text', points: kind === 'line' ? [text('#pd-mark-point'), text('#pd-mark-end')] : [text('#pd-mark-point')], text: kind === 'text' ? text('#pd-mark-text') : null });
                    await commit(next);
                });
            };
        root.querySelectorAll<HTMLButtonElement>('[data-remove-mark]').forEach(b => b.onclick = () => void run(async () => { const next = structuredClone(doc); next.pieces.find(p => p.id === pieceId)?.annotations?.splice(Number(b.dataset.removeMark), 1); await commit(next); }));
        change('#pd-doc-name', async () => { const next = structuredClone(doc); next.name = text('#pd-doc-name').trim(); await commit(next); });
        change('#pd-doc-label', async () => { const next = structuredClone(doc); next.label = text('#pd-doc-label') || null; await commit(next); });
        change('#pd-base-size', async () => { const next = structuredClone(doc); next.base_size = text('#pd-base-size'); await commit(next); });
        click('#pd-add-size', async () => { const next = structuredClone(doc), name = text('#pd-new-size').trim(); if (!name || name.length > 32 || next.sizes.includes(name))
            throw new Error('Enter a new size name (up to 32 characters).'); if (next.sizes.length >= 100)
            throw new Error('Use at most 100 sizes.'); next.sizes.push(name); next.measurements[name] = structuredClone(next.measurements[next.base_size] ?? {}); for (const rule of Object.values(next.rules ?? {}))
            rule.deltas[name] = [0, 0]; await commit(next); });
        root.querySelectorAll<HTMLInputElement>('[data-pd-measure]').forEach(i => i.onchange = () => void run(async () => { const next = structuredClone(doc); next.measurements[i.dataset.size!] ??= {}; next.measurements[i.dataset.size!]![i.dataset.pdMeasure!] = finite(`[data-pd-measure="${CSS.escape(i.dataset.pdMeasure!)}"][data-size="${CSS.escape(i.dataset.size!)}"]`); await commit(next); }));
        click('#pd-add-measure', async () => { const k = text('#pd-measure-name').trim(); validId(k); if (k.includes('.'))
            throw new Error('Measurement names use letters, numbers and underscores.'); const next = structuredClone(doc); for (const s of next.sizes) {
            next.measurements[s] ??= {};
            next.measurements[s]![k] = finite('#pd-measure-value');
        } await commit(next); });
        root.querySelectorAll<HTMLButtonElement>('[data-apply-increment]').forEach(b => b.onclick = () => void run(async () => {
            const name = b.dataset.applyIncrement!, step = finite(`[data-pd-increment="${CSS.escape(name)}"]`), next = structuredClone(doc), base = doc.measurements[doc.base_size]?.[name];
            if (base === undefined)
                throw new Error('Set the base measurement first.');
            for (let i = 0; i < next.sizes.length; i++) {
                const s = next.sizes[i]!, expression = `${base}+(${i - next.sizes.indexOf(next.base_size)})*(${step})`;
                next.measurements[s] ??= {};
                next.measurements[s]![name] = await formula(expression);
            }
            next.meta = { ...next.meta, measurement_steps: { ...next.meta?.measurement_steps as Record<string, Json>, [name]: step } };
            await commit(next);
        }));
        root.querySelectorAll<HTMLInputElement>('[data-pd-variable]').forEach(i => i.onchange = () => void run(async () => { const next = structuredClone(doc); next.variables[i.dataset.pdVariable!] = value(i.value); await commit(next); }));
        click('#pd-add-variable', async () => { const name = text('#pd-variable-name').trim(); validId(name); if (name.includes('.'))
            throw new Error('Variable names use letters, numbers and underscores.'); const next = structuredClone(doc); next.variables[name] = value(text('#pd-variable-value')); await commit(next); });
        click('#pd-measure-export', () => { if (host.core()) {
            host.download(host.core()!.export_measurements(doc), 'measurements.csv', 'text/csv');
            return;
        } const keys = [...new Set(Object.values(doc.measurements).flatMap(r => Object.keys(r)))]; const quote = (v: unknown) => `"${String(v ?? '').replace(/"/g, '""')}"`; host.download([['measurement', ...doc.sizes], ...keys.map(k => [k, ...doc.sizes.map(s => doc.measurements[s]?.[k] ?? '')])].map(r => r.map(quote).join(',')).join('\r\n'), 'measurements.csv', 'text/csv'); });
        change('#pd-measure-file', async () => {
            const file = input('#pd-measure-file').files?.[0];
            if (!file)
                return;
            if (file.size > 16 * 1024 * 1024)
                throw new Error('Measurement file exceeds 16 MB.');
            const source = await file.text(), next = structuredClone(doc);
            if (file.name.toLowerCase().endsWith('.csv') && !source.trimStart().startsWith('size,height_mm,name,value_mm,base')) {
                const [heads, ...rows] = csvRows(source);
                if (heads?.[0]?.toLowerCase() !== 'measurement' || heads.length < 2)
                    throw new Error('Use CSV headers: measurement,M,L (your sizes).');
                const sizes = heads.slice(1).map(s => s.trim());
                if (sizes.some(s => !s) || new Set(sizes).size !== sizes.length)
                    throw new Error('CSV size headers must be unique.');
                for (const s of sizes)
                    if (!next.sizes.includes(s)) {
                        next.sizes.push(s);
                        next.measurements[s] = {};
                    }
                for (const row of rows) {
                    const k = row[0]?.trim();
                    if (!k)
                        continue;
                    validId(k);
                    sizes.forEach((s, i) => { const v = row[i + 1]; if (!v?.trim() || !Number.isFinite(Number(v)))
                        throw new Error(`Enter a finite value for ${k}, ${s}.`); next.measurements[s] ??= {}; next.measurements[s]![k] = Number(v); });
                }
            }
            else {
                const imported = host.core()?.import_measurements(doc, source, file.name.toLowerCase().endsWith('.csv') ? 'csv' : 'xml', size) ?? await host.call<PatternDocument>('pattern/measurements-import', { document: doc, source, format: file.name.toLowerCase().endsWith('.csv') ? 'csv' : 'xml', size });
                Object.assign(next, imported);
            }
            await commit(next);
        });
        change('#pd-reference', async () => { reference = text('#pd-reference'); alignedNest = reference ? (host.core()?.nest(doc, reference) ?? await host.call('pattern/nest', { document: doc, reference })) : undefined; draw(); });
        change('#pd-grade-piece', () => { pieceId = text('#pd-grade-piece'); renderControls(); });
        click('#pd-attach-rule', async () => { const p = currentPiece(), point = text('#pd-grade-point'), rule = text('#pd-grade-rule'); if (!p || !rule)
            throw new Error('Choose a piece, outline point and rule.'); const next = structuredClone(doc); next.pieces.find(q => q.id === p.id)!.grade_points = { ...p.grade_points, [point]: rule }; await commit(next); await setMode('grading'); });
        click('#pd-clear-rules', async () => { const next = structuredClone(doc); const p = next.pieces.find(p => p.id === pieceId); if (p)
            p.grade_points = {}; await commit(next); await setMode('grading'); });
        change('#pd-rule-select', () => { const id = text('#pd-rule-select'); input('#pd-rule-id').value = id; input('#pd-rule-mode').value = doc.rules?.[id]?.mode ?? 'relative'; renderDeltas(id); });
        click('#pd-rule-save', async () => { const id = text('#pd-rule-id').trim(); if (!/^[A-Za-z0-9_][A-Za-z0-9_.-]*$/.test(id)) throw new Error('Rule IDs use letters, numbers, dots, hyphens and underscores.'); const next = structuredClone(doc); next.rules ??= {}; next.rules[id] = { mode: text('#pd-rule-mode') as 'relative' | 'incremental', deltas: Object.fromEntries(doc.sizes.map((s, i) => [s, [finite(`#pd-dx-${i}`), finite(`#pd-dy-${i}`)]])) }; await commit(next); await setMode('grading'); });
        click('#pd-rule-remove', async () => { const id = text('#pd-rule-select'); if (!id)
            return; const next = structuredClone(doc); delete next.rules?.[id]; for (const p of next.pieces)
            for (const [point, rule] of Object.entries(p.grade_points ?? {}))
                if (rule === id)
                    delete p.grade_points?.[point]; await commit(next); await setMode('grading'); });
        click('#pd-export-dxf', async () => { await host.exportDxf(); status('DXF saved with all sizes and BETA grade-rule comments.'); });
        click('#pd-export-pdf', async () => { const paper = text('#pd-paper') as 'A4' | 'A3', bytes = host.core()?.export_pdf(doc, paper); if (bytes)
            host.download(new Blob([new Uint8Array(bytes)], { type: 'application/pdf' }), 'pattern-1to1.pdf');
        else {
            const response = await apiFetch('/api/pattern/export-pdf', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ document: doc, paper }) });
            if (!response.ok)
                throw new Error(await response.text());
            host.download(await response.blob(), 'pattern-1to1.pdf');
        } status('PDF saved. Print Actual size / 100% and measure the test square.'); });
        click('#pd-export-plt', async () => { if (host.core())
            host.download(host.core()!.export_plt(doc), 'pattern.plt', 'application/vnd.hp-hpgl');
        else {
            const response = await apiFetch('/api/pattern/export-plt', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ document: doc }) });
            if (!response.ok)
                throw new Error(await response.text());
            host.download(await response.blob(), 'pattern.plt');
        } status('PLT saved. Check scale on the plotter before production.'); });
        click('#pd-send-marker', async () => { const ratio = Object.fromEntries(doc.sizes.map((s, i) => [s, finite(`#pd-ratio-${i}`)])); await host.sendMarker(ratio); });
        change('#pd-trace-file', async () => {
            const file = input('#pd-trace-file').files?.[0];
            if (!file)
                return;
            if (file.size > 3 * 1024 * 1024)
                throw new Error('Use a tracing photo up to 3 MB.');
            if (!['image/png', 'image/jpeg'].includes(file.type))
                throw new Error('Choose PNG or JPEG.');
            const data = await new Promise<string>((resolve, reject) => { const reader = new FileReader(); reader.onload = () => resolve(String(reader.result)); reader.onerror = () => reject(new Error('Could not read photo')); reader.readAsDataURL(file); });
            const image = new Image();
            image.src = data;
            await image.decode();
            trace = { data, width: image.naturalWidth, height: image.naturalHeight, scale: 1, x: 0, y: 0, opacity: .45, calibrated: false };
            fitTrace();
            await saveTrace();
        });
        change('#pd-trace-opacity', async () => { if (trace) {
            trace.opacity = finite('#pd-trace-opacity');
            await saveTrace();
        } });
        click('#pd-calibrate', () => { if (!trace)
            return; calibrating = []; status('Click the first then second known-distance point on the image.'); });
        click('#pd-trace-clear', async () => { trace = undefined; await saveTrace(); });
    }
    function pieceMenu(): string { return `<h3>Piece tools</h3><p>Mirror and unfold keep links to source geometry. Unfold needs a straight fold edge with zero allowance and measurement grading. Other shape tools create fixed outlines at the current size. Sources are retained; set both source quantities to zero if it is only a template.</p><form id="pd-piece-action"><div class="fields">${select('pd-action', 'Action', ['dart', 'dart_transfer', 'dart_close', 'slash_spread', 'pleat', 'split', 'mirror', 'unfold', 'walk', 'true_seam'])}${field('pd-action-id', 'New piece / path ID', 'result_' + (doc.pieces.length + 1))}${select('pd-action-a', 'First line point / first seam', ['', ...pointIds(), ...pathIds()])}${select('pd-action-b', 'Second line point / second seam', ['', ...pointIds(), ...pathIds()])}${select('pd-action-apex', 'Dart apex', pointIds())}${field('pd-action-edge', 'Dart / transfer / fold edge (starts at 1)', 1, 'number', 'min="1" step="1"')}${field('pd-action-center', 'Dart mouth distance on edge (mm)', 100)}${field('pd-action-width', 'Dart mouth width (mm)', 20)}${field('pd-action-vertex', 'Closure cut vertex (starts at 1)', 3, 'number', 'min="1" step="1"')}${field('pd-action-angle', 'Spread angle (degrees)', 15)}${field('pd-action-depth', 'Pleat folded depth (mm)', 20)}${select('pd-action-pleat-kind', 'Pleat type', ['knife', 'box'])}${field('pd-action-target', 'True seam length (mm)', 100)}</div><button class="primary" type="submit">Apply piece tool</button></form>`; }
    function bindPieceMenu() {
        const form = root.querySelector<HTMLFormElement>('#pd-piece-action');
        if (!form)
            return;
        form.onsubmit = e => {
            e.preventDefault();
            void run(async () => {
                const p = currentPiece();
                if (!p)
                    throw new Error('Choose a piece.');
                const request: Record<string, Json> = { action: text('#pd-action'), piece: p.id, size, id: text('#pd-action-id'), a: text('#pd-action-a'), b: text('#pd-action-b'), apex: text('#pd-action-apex'), edge: value(text('#pd-action-edge')), center: value(text('#pd-action-center')), width: value(text('#pd-action-width')), vertex: value(text('#pd-action-vertex')), angle: value(text('#pd-action-angle')), depth: value(text('#pd-action-depth')), pleat_kind: text('#pd-action-pleat-kind'), target: value(text('#pd-action-target')) };
                const result = host.core()?.piece_action(doc, request) ?? await host.call<{
                    document: PatternDocument;
                    message?: string;
                }>('pattern/piece-action', { ...request, document: doc });
                if (result.document.pieces.some(p => p.id === request.id)) pieceId = String(request.id);
                await commit(result.document);
                status(result.message ?? 'Piece tool applied.');
            });
        };
    }
    function pair(a: string, b: string): [
        string,
        string
    ] | null { const x = text(a), y = text(b); if (!x && !y)
        return null; if (!x || !y || x === y)
        throw new Error('Choose two different points for the line.'); return [x, y]; }
    function addEdge(id: string) {
        if (!id || doc.drawing[id]?.kind === 'point')
            throw new Error('Choose a line, curve or arc.');
        let reversed = false;
        const last = edges.at(-1), points = inspection?.objects[id]?.points;
        if (last && points) {
            const prev = inspection?.objects[last.object]?.points;
            const end = last.reversed ? prev?.[0] : prev?.at(-1);
            const tail = points.at(-1);
            if (end && tail)
                reversed = Math.hypot(end[0] - tail[0], end[1] - tail[1]) < .1;
        }
        const name = root.querySelector<HTMLInputElement>('#pd-piece-new-name')?.value;
        edges = appendPieceEdge(edges, id, reversed);
        renderControls();
        if (name)
            input('#pd-piece-new-name').value = name;
        draw();
    }
    async function formula(expression: string): Promise<number> { return host.core()?.check_formula(doc, size, expression) ?? host.call<{
        value: number;
    }>('pattern/formula', { document: doc, size, expression }).then(r => r.value); }
    const isRef = (p: PatternToolParameter) => ['point_ref', 'line_ref', 'curve_ref', 'point_refs', 'object_ref'].includes(p.kind);
    function referenceIds(p: PatternToolParameter): string[] { return p.kind === 'point_ref' || p.kind === 'point_refs' ? pointIds() : p.kind === 'line_ref' ? Object.keys(doc.drawing).filter(id => doc.drawing[id]?.kind === 'line') : p.kind === 'curve_ref' ? pathIds() : Object.keys(doc.drawing); }
    function showTool(spec: PatternToolSpec, id?: string) {
        tool = spec;
        editing = !!id;
        selected = id ?? selected;
        $('#pd-tool-title').textContent = spec.name;
        $('#pd-tool-help').textContent = spec.help + ' Click reference objects on the canvas or choose them from the lists.';
        let n = 1;
        while (Object.hasOwn(doc.drawing, `obj_${n}`))
            n++;
        input('#pd-object-id').value = id ?? `obj_${n}`;
        input('#pd-object-id').readOnly = editing;
        const object = id ? doc.drawing[id] : undefined, target = object?.kind === 'point' ? object.operation : object;
        $('#pd-tool-fields').innerHTML = spec.parameters.map(p => {
            const original = (target as unknown as Record<string, unknown> | undefined)?.[p.name], v = original ?? p.default ?? '';
            if (isRef(p) && p.kind !== 'point_refs')
                return `<label>${esc(p.label ?? p.name)}<select data-param="${esc(p.name)}" data-kind="${esc(p.kind)}" required>${options(['', ...referenceIds(p)], String(v))}</select></label>`;
            if (p.choices?.length)
                return `<label>${esc(p.label ?? p.name)}<select data-param="${esc(p.name)}" data-kind="${esc(p.kind)}">${options(p.choices, String(v))}</select></label>`;
            return `<label>${esc(p.label ?? p.name)}<input data-param="${esc(p.name)}" data-kind="${esc(p.kind)}" value="${esc(Array.isArray(v) ? (p.kind === 'point_refs' ? v.join(', ') : v.map(a => Array.isArray(a) ? a.join(', ') : a).join('; ')) : v)}" ${p.kind === 'index' ? 'type="number" min="0" step="1"' : 'type="text"'} ${isRef(p) ? '' : 'list="pd-formulas"'} ${p.default == null ? 'required' : ''} autocomplete="off"><small>${esc(p.kind === 'point_refs' ? 'Comma-separated point IDs' : p.kind === 'tangents' ? 'Optional derivative vectors (x,y; x,y)' : p.kind === 'cubic_segments' ? 'Four point IDs per segment; separate segments with semicolons.' : 'Use mm for lengths; degrees for angles.')}</small></label>`;
        }).join('');
        if (spec.parameters.some(p => p.name === 'source')) {
            $('#pd-tool-fields').insertAdjacentHTML('afterbegin', `<label class="check"><input type="checkbox" id="pd-group-toggle">Apply to group</label><label>Group objects (Ctrl/Command to select several)<select id="pd-group-objects" multiple size="5">${options(Object.keys(doc.drawing))}</select></label>`);
            input('#pd-group-toggle').onchange = () => { const source = root.querySelector<HTMLSelectElement>('[data-param="source"]'); if (source) {
                source.disabled = input('#pd-group-toggle').checked;
                source.required = !source.disabled;
            } };
        }
        $('#pd-picker').innerHTML = '<option value="">Choose a value to insert</option>' + options(formulaOptions(doc, ''));
        $('#pd-formula-check').textContent = '';
        focusedFormula = undefined;
        root.querySelectorAll<HTMLInputElement>('#pd-tool-fields input').forEach(i => { i.onfocus = () => { focusedFormula = i; }; i.oninput = () => void checkField(i); });
        dialog.show();
        input('#pd-object-id').focus();
        draw();
    }
    let formulaRevision = 0;
    async function checkField(i: HTMLInputElement) {
        if (!['formula', 'angle_formula', 'index'].includes(i.dataset.kind ?? ''))
            return;
        const revision = ++formulaRevision;
        $('#pd-formulas').innerHTML = formulaOptions(doc, i.value, i.selectionStart ?? i.value.length).map(n => `<option value="${esc(insertFormula(i.value, i.selectionStart ?? i.value.length, n).text)}"></option>`).join('');
        if (!i.value.trim()) {
            $('#pd-formula-check').textContent = 'Enter a formula or number.';
            return;
        }
        try {
            const n = await formula(i.value);
            if (revision !== formulaRevision)
                return;
            $('#pd-formula-check').textContent = `Formula value: ${n.toFixed(3)}`;
            $('#pd-formula-check').classList.remove('error-text');
            i.removeAttribute('aria-invalid');
        }
        catch (e) {
            if (revision !== formulaRevision)
                return;
            $('#pd-formula-check').textContent = err(e);
            $('#pd-formula-check').classList.add('error-text');
            i.setAttribute('aria-invalid', 'true');
        }
    }
    function pickObject(id: string) {
        selected = id;
        if (collecting && mode === 'piece') {
            try {
                addEdge(id);
            }
            catch (e) {
                status(err(e), true);
            }
            return;
        }
        if (tool && dialog.open) {
            const refs = Array.from(root.querySelectorAll<HTMLInputElement | HTMLSelectElement>('[data-param]')).filter(i => isRef(tool!.parameters.find(p => p.name === i.dataset.param)!));
            const active = window.document.activeElement as HTMLInputElement | HTMLSelectElement;
            const matching = (i: HTMLInputElement | HTMLSelectElement) => referenceIds(tool!.parameters.find(p => p.name === i.dataset.param)!).includes(id);
            const target = refs.includes(active) && matching(active) ? active : refs.find(i => !i.value && matching(i)) ?? refs.find(i => i.dataset.kind === 'point_refs' && matching(i));
            if (target) {
                target.value = target.dataset.kind === 'point_refs' ? [target.value, id].filter(Boolean).join(', ') : id;
                target.dispatchEvent(new Event('change', { bubbles: true }));
            }
        }
        updateToolbar();
        draw();
    }
    async function editSelected() {
        const o = doc.drawing[selected];
        if (!o)
            throw new Error('Select a construction first.');
        const kind = o.kind === 'point' ? o.operation.op : o.kind, spec = catalog.find(t => t.id === kind);
        if (!spec)
            throw new Error('This construction has no editable catalog tool yet.');
        showTool(spec, selected);
    }
    $('#pd-tool-form').addEventListener('submit', event => {
        event.preventDefault();
        void run(async () => {
            if (!tool || !$<HTMLFormElement>('#pd-tool-form').reportValidity())
                return;
            const id = text('#pd-object-id');
            validId(id);
            if (!editing && Object.hasOwn(doc.drawing, id))
                throw new Error('This object ID already exists.');
            const parameters: Record<string, Json> = {};
            root.querySelectorAll<HTMLInputElement | HTMLSelectElement>('[data-param]').forEach(i => {
                if (i.disabled)
                    return;
                const kind = i.dataset.kind!;
                parameters[i.dataset.param!] = kind === 'point_refs' ? i.value.split(',').map(x => x.trim()).filter(Boolean) : kind === 'cubic_segments' ? i.value.split(';').map(s => s.split(',').map(x => x.trim())) : kind === 'tangents' ? (i.value.trim() === '[]' || !i.value.trim() ? [] : i.value.split(';').map(s => s.split(',').map(x => value(x)))) : kind === 'index' ? Number(i.value) : isRef(tool!.parameters.find(p => p.name === i.dataset.param)!) || kind === 'enum' ? i.value : value(i.value);
            });
            const nextBase = structuredClone(doc);
            if (editing)
                delete nextBase.drawing[id];
            const group = root.querySelector<HTMLInputElement>('#pd-group-toggle')?.checked;
            const selection = group ? Array.from($<HTMLSelectElement>('#pd-group-objects').selectedOptions, o => o.value) : undefined;
            if (group && !selection?.length)
                throw new Error('Choose objects for the group.');
            const request = { tool: tool.id, id, size, parameters, selection };
            const result = host.core()?.apply_tool(nextBase, request) ?? await host.call<{
                document: PatternDocument;
                message?: string;
            }>('pattern/tool', { document: nextBase, ...request });
            if (selection)
                Object.keys(result.document.drawing).filter(k => !Object.hasOwn(doc.drawing, k)).forEach(k => rememberObject(result.document, k));
            else
                rememberObject(result.document, id);
            await commit(result.document);
            dialog.close();
            tool = undefined;
            selected = Object.hasOwn(doc.drawing, id) ? id : '';
            updateToolbar();
            draw();
            if (result.message)
                status(result.message);
        });
    });
    $('#pd-tool-cancel').onclick = () => { dialog.close(); tool = undefined; };
    $('#pd-picker').onchange = () => { if (!focusedFormula || !text('#pd-picker'))
        return; const f = focusedFormula, r = insertFormula(f.value, f.selectionStart ?? f.value.length, text('#pd-picker')); f.value = r.text; f.focus(); f.setSelectionRange(r.caret, r.caret); void checkField(f); input('#pd-picker').value = ''; };
    $('#pd-tools').onclick = e => { const id = (e.target as Element).closest<HTMLButtonElement>('[data-tool]')?.dataset.tool; const spec = catalog.find(t => t.id === id); if (spec)
        showTool(spec); };
    $('#pd-search').oninput = palette;
    $('#pd-history').onclick = e => { const id = (e.target as Element).closest<HTMLButtonElement>('[data-object]')?.dataset.object; if (id)
        pickObject(id); };
    $('#pd-edit').onclick = () => void run(editSelected);
    $('#pd-delete').onclick = () => { if (!selected)
        return; const d = dependencyDelete(doc, selected); $('#pd-delete-details').textContent = `Constructions: ${d.objects.join(', ')}. Pieces: ${d.pieces.join(', ') || 'none'}. Variables: ${d.variables.join(', ') || 'none'}. Undo restores all listed items.`; $<HTMLDialogElement>('#pd-delete-dialog').showModal(); };
    $('#pd-delete-cancel').onclick = () => $<HTMLDialogElement>('#pd-delete-dialog').close();
    $('#pd-delete-confirm').onclick = () => void run(async () => { await commit(deleteConstruction(doc, selected)); selected = ''; $<HTMLDialogElement>('#pd-delete-dialog').close(); updateToolbar(); draw(); });
    $('#pd-new').onclick = () => void run(async () => { selected = ''; edges = []; collecting = false; trace = undefined; await commit(blankPattern()); await setMode('draft'); fit(); });
    async function undo(redo = false) { const candidate = redo ? history.redo() : history.undo(); await commit(candidate, false); }
    $('#pd-undo').onclick = () => void run(() => undo());
    $('#pd-redo').onclick = () => void run(() => undo(true));
    $('#pd-size').onchange = () => void run(async () => { size = text('#pd-size'); await refresh(); });
    $('#pd-fit').onclick = fit;
    $('#pd-grid').onchange = draw;
    function zoom(f: number, at?: Pt) { const x = at?.[0] ?? view.x + view.w / 2, y = at ? -at[1] : view.y + view.h / 2; const w = Math.min(50000, Math.max(10, view.w * f)), ratio = w / view.w; view = { x: x - (x - view.x) * ratio, y: y - (y - view.y) * ratio, w, h: view.h * ratio }; draw(); }
    $('#pd-minus').onclick = () => zoom(1.25);
    $('#pd-plus').onclick = () => zoom(.8);
    $('#pd-pan').onclick = () => { panMode = !panMode; $('#pd-pan').setAttribute('aria-pressed', String(panMode)); };
    $('#pd-labels').onclick = () => { showLabels = !showLabels; $('#pd-labels').setAttribute('aria-pressed', String(showLabels)); draw(); };
    root.querySelectorAll<HTMLButtonElement>('[data-mode]').forEach(b => b.onclick = () => void run(() => setMode(b.dataset.mode as Mode)));
    function location(event: PointerEvent | WheelEvent): Pt { const point = canvas.createSVGPoint(); point.x = event.clientX; point.y = event.clientY; const matrix = canvas.getScreenCTM()?.inverse(); if (!matrix)
        return [0, 0]; const p = point.matrixTransform(matrix); return [p.x, -p.y]; }
    function snap(at: Pt, exclude?: string): Pt {
        const core = host.core();
        if (!core)
            return at;
        try {
            return core.snap(doc, size, { at, ignore: exclude, radius: view.w / 100, grid: Number(text('#pd-grid')), points: input('#pd-snap-points').checked, paths: input('#pd-snap-lines').checked }).point;
        }
        catch {
            return at;
        }
    }
    canvas.addEventListener('wheel', event => { event.preventDefault(); zoom(event.deltaY > 0 ? 1.1 : 1 / 1.1, location(event)); }, { passive: false });
    canvas.addEventListener('pointerdown', event => {
        if (event.button !== 0)
            return;
        const at = location(event), id = (event.target as Element).closest('[data-canvas-object]')?.getAttribute('data-canvas-object') ?? '';
        if (calibrating) {
            void run(async () => { calibrating!.push(at); if (calibrating!.length === 2 && trace) {
                const [a, b] = calibrating!;
                const scale = host.core()?.image_scale(a!, b!, finite('#pd-known-distance'));
                if (scale === undefined)
                    throw new Error('Load WASM to calibrate the photo.');
                trace.scale *= scale;
                trace.calibrated = true;
                calibrating = undefined;
                await saveTrace();
                status('Photo scale set. Trace with the draft tools.');
            } draw(); });
            return;
        }
        if (tool && dialog.open) {
            if (id)
                pickObject(id);
            else {
                const p = snap(at), x = root.querySelector<HTMLInputElement>('[data-param="x"]'), y = root.querySelector<HTMLInputElement>('[data-param="y"]');
                if (x && y) {
                    x.value = p[0].toFixed(2);
                    y.value = p[1].toFixed(2);
                    void checkField(x);
                }
            }
            return;
        }
        if (collecting) {
            if (id)
                pickObject(id);
            return;
        }
        if (id)
            pickObject(id);
        const o = doc.drawing[id], canMove = o?.kind === 'point' && ((o.operation.op === 'absolute' && typeof o.operation.x === 'number' && typeof o.operation.y === 'number') || (o.operation.op === 'polar' && typeof o.operation.distance === 'number' && typeof o.operation.angle === 'number'));
        drag = { start: at, at, view: { ...view }, id: !event.shiftKey && !panMode && canMove ? id : '', moved: false };
        canvas.setPointerCapture(event.pointerId);
    });
    canvas.addEventListener('pointermove', event => {
        const p = location(event);
        $('#pd-cursor').textContent = `x ${p[0].toFixed(1)} mm · y ${p[1].toFixed(1)} mm`;
        if (!drag)
            return;
        drag.moved ||= Math.hypot(p[0] - drag.start[0], p[1] - drag.start[1]) > view.w / 500;
        if (drag.id) {
            drag.at = snap(p, drag.id);
            const o = inspection?.objects[drag.id];
            if (o) {
                o.points = [drag.at];
                draw();
            }
        }
        else {
            view.x -= p[0] - drag.start[0];
            view.y += p[1] - drag.start[1];
            draw();
        }
    });
    canvas.addEventListener('pointerup', () => { const d = drag; drag = undefined; if (!d?.id || !d.moved)
        return; void run(async () => { try {
        const next = host.core()?.move_point(doc, d.id, size, d.at) ?? await host.call<PatternDocument>('pattern/move-point', { document: doc, id: d.id, size, target: d.at });
        await commit(next);
    }
    catch (e) {
        await refresh();
        throw e;
    } }); });
    canvas.addEventListener('pointercancel', () => { drag = undefined; void refresh(); });
    canvas.addEventListener('keydown', e => {
        const id = (e.target as Element).closest('[data-canvas-object]')?.getAttribute('data-canvas-object');
        if ((e.key === 'Enter' || e.key === ' ') && id) {
            e.preventDefault();
            pickObject(id);
            return;
        }
        if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(e.key)) {
            e.preventDefault();
            e.stopPropagation();
            const object = selected && doc.drawing[selected], at = inspection?.objects[selected]?.points[0];
            const editable = object && object.kind === 'point' && ((object.operation.op === 'absolute' && typeof object.operation.x === 'number' && typeof object.operation.y === 'number') || (object.operation.op === 'polar' && typeof object.operation.distance === 'number' && typeof object.operation.angle === 'number'));
            const step = e.shiftKey ? 10 : 1, dx = e.key === 'ArrowRight' ? step : e.key === 'ArrowLeft' ? -step : 0, dy = e.key === 'ArrowUp' ? step : e.key === 'ArrowDown' ? -step : 0;
            if (editable && at) {
                void run(async () => { const next = host.core()?.move_point(doc, selected, size, [at[0] + dx, at[1] + dy]) ?? await host.call<PatternDocument>('pattern/move-point', { document: doc, id: selected, size, target: [at[0] + dx, at[1] + dy] }); await commit(next); canvas.querySelector<SVGElement>(`[data-canvas-object="${CSS.escape(selected)}"]`)?.focus(); });
            }
            else {
                view.x += dx * view.w / 100;
                view.y -= dy * view.w / 100;
                draw();
            }
        }
        if (e.key === '+' || e.key === '=') {
            e.preventDefault();
            zoom(.8);
        }
        if (e.key === '-') {
            e.preventDefault();
            zoom(1.25);
        }
        if (e.key === '0') {
            e.preventDefault();
            fit();
        }
    });
    root.addEventListener('keydown', e => {
        const target = e.target as Element, typing = target.matches('input,select,textarea');
        if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z' && !typing) {
            e.preventDefault();
            e.stopPropagation();
            void run(() => undo(e.shiftKey));
        }
        if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'y' && !typing) {
            e.preventDefault();
            e.stopPropagation();
            void run(() => undo(true));
        }
        if (e.key === 'Escape') {
            dialog.close();
            tool = undefined;
            collecting = false;
            calibrating = undefined;
            edges = [];
            renderControls();
            draw();
            e.stopPropagation();
        }
        if (e.key === 'Delete' && !typing && selected) {
            e.preventDefault();
            $('#pd-delete').click();
        }
    });
    function fitTrace() { if (trace)
        view = { x: trace.x - 20, y: -trace.y - trace.height * trace.scale - 20, w: trace.width * trace.scale + 40, h: trace.height * trace.scale + 40 }; draw(); }
    async function saveTrace() { const next = structuredClone(doc); next.meta ??= {}; if (trace)
        next.meta.trace_image = trace as unknown as Json;
    else
        delete next.meta.trace_image; await commit(next); }
    function restoreTrace() { const t = doc.meta?.trace_image as unknown as Trace | undefined; trace = t && typeof t.data === 'string' && /^data:image\/(png|jpeg);base64,/.test(t.data) && Number.isFinite(t.scale) && t.scale > 0 ? t : undefined; }
    root.dataset.patternMode = mode;
    restoreTrace();
    palette();
    void refresh().then(fit);
    return { sync(next: PatternDocument) { if (JSON.stringify(next) !== JSON.stringify(doc)) {
            if (!working)
                history.apply(next);
            doc = next;
            restoreTrace();
        } void refresh(); }, activate() { void refresh(); }, get mode() { return mode; } };
}
