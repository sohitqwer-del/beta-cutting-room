import { downloadFile, isDesktop } from './transport';
import { installPatternEditor } from './pattern-editor';
import { loadPatternCore, type Bundle, type CostResponse, type CutPlan, type CuttingReport, type Damage, type DigitizeResponse, type InspectResponse, type Job, type MarkerResult, type PatternCore, type PatternDocument, type PatternGrade, type PatternImportResponse, type PhysicalLay, type PlyRun, type RollRegister, type SpreadingPlan } from '@beta/marker-wasm';
import starter from '../../../packages/marker-wasm/test/fixtures/pattern.doc.json';
import type { Editor } from './features';
import { emptyResult } from './state';
import { acceptedJob, csvRows, readPattern, readRollCsv, rollColumns } from './cutting-room-data';
import { carryWarnings, seamlyWarnings, warningsHtml } from './import-warnings';
const esc = (v: unknown): string => String(v ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]!));
const $ = <T extends Element = HTMLElement>(s: string): T => window.document.querySelector<T>(s)!;
const text = (id: string): string => $<HTMLInputElement>(`#${id}`).value;
function num(id: string): number { const i = $<HTMLInputElement>(`#${id}`); if (!i.value.trim() || !i.checkValidity() || !Number.isFinite(Number(i.value)))
    throw new Error(`Check ${i.labels?.[0]?.textContent ?? id}.`); return Number(i.value); }
const field = (id: string, label: string, value: string | number, type = 'text', extra = ''): string => `<label>${label}<input id="${id}" type="${type}" value="${esc(value)}" ${extra}></label>`;
const number = (id: string, label: string, value: number, min = 0, step = 'any'): string => field(id, label, value, 'number', `min="${min}" step="${step}" required`);
const table = (caption: string, heads: string[], rows: string[][]): string => `<div class="table-scroll"><table><caption>${esc(caption)}</caption><thead><tr>${heads.map(h => `<th scope="col">${esc(h)}</th>`).join('')}</tr></thead><tbody>${rows.map(r => `<tr>${r.map((v, i) => `<${i ? 'td' : 'th scope="row"'}>${v}</${i ? 'td' : 'th'}>`).join('')}</tr>`).join('')}</tbody></table></div>`;
function download(value: Blob | string, name: string, mime = 'application/json'): void { void downloadFile(value instanceof Blob ? value : new Blob([value], { type: mime }), name).catch(error => { $('#pattern-status').textContent = String(error); }); }

interface Planning {
    plan: CutPlan | undefined;
    job: Job | undefined;
    endAllowance: number;
}
interface Saved {
    document: PatternDocument;
    register: RollRegister;
    bundles: Bundle[];
    ledger: Damage[];
    bundleSource?: {
        job: Job;
        result: MarkerResult;
        lay: PhysicalLay;
    };
}
/** Offline cutting-room panels. All garment calculations call Rust. */
export function installCuttingRoom(editor: Editor, screen: (index: number) => void, planning: () => Planning) {
    let doc = readPattern(JSON.stringify(starter)), register: RollRegister = { rolls: [] }, bundles: Bundle[] = [], ledger: Damage[] = [];
    let patternUI: ReturnType<typeof installPatternEditor> | undefined;
    let patternSaved = true, patternReady = false;
    let projectGeneration = 0;
    let core: PatternCore | undefined, scan: DigitizeResponse | undefined, inspection: InspectResponse | undefined, spread: SpreadingPlan | undefined;
    let spreadSources: ReturnType<typeof spreadSource> = [];
    let importedSpread: {job:Job;result:MarkerResult;lay:PhysicalLay} | undefined;
    let bundleSource: Saved['bundleSource'], spreadKey = '', inspectionKey = '', working = false;
    try {
        const raw = localStorage.getItem('beta-cutting-room-v1');
        if (raw) {
            const s = JSON.parse(raw) as Saved;
            doc = readPattern(JSON.stringify(s.document));
            if (Array.isArray(s.register?.rolls))
                register = s.register;
            if (Array.isArray(s.bundles))
                bundles = s.bundles;
            if (Array.isArray(s.ledger))
                ledger = s.ledger;
            bundleSource = s.bundleSource;
        }
    }
    catch {
        editor.message('Saved draft could not be read. Open your JSON backup files to restore it.', true);
    }
    function persist(): boolean { try {
        localStorage.setItem('beta-cutting-room-v1', JSON.stringify({ document: doc, register, bundles, ledger, bundleSource } satisfies Saved));
        return true;
    }
    catch {
        if (!isDesktop()) editor.message('Browser storage is full or blocked. Save your JSON backups.', true);
        return false;
    } }
    async function json<T>(path: string, body: unknown): Promise<T> { return await (await editor.api(path, JSON.stringify(body))).json() as T; }
    async function task(status: string, action: () => Promise<void>): Promise<void> { if (working || editor.busy())
        return; working = true; const p = $(status); p.textContent = 'Working locally...'; p.classList.remove('error-text'); p.setAttribute('aria-busy', 'true'); window.document.querySelectorAll<HTMLButtonElement>('.floor-screen button[data-action]').forEach(b => b.disabled = true); try {
        await action();
    }
    catch (e) {
        p.textContent = e instanceof Error ? e.message : String(e);
        p.classList.add('error-text');
    }
    finally {
        working = false;
        p.removeAttribute('aria-busy');
        window.document.querySelectorAll<HTMLButtonElement>('.floor-screen button[data-action]').forEach(b => b.disabled = false);
    } }
    const panel = (id: number, title: string, description: string, body: string) => `<section id="screen-${id}" class="screen floor-screen" hidden aria-labelledby="floor-title-${id}"><div class="floor-heading"><div><span class="eyebrow">BETA CUTTING ROOM</span><h1 id="floor-title-${id}" tabindex="-1">${title}</h1><p>${description}</p></div><span class="tag">Offline &middot; local Rust</span></div>${body}</section>`;
    $('.workspace').insertAdjacentHTML('afterend', panel(4, 'Pattern', 'Review measurements, grade sizes, then send cut pieces to the marker.', `
    <div class="floor-actions"><label>Open pattern JSON or Seamly2D<input id="pattern-file" type="file" accept=".json,.betapattern,.sm2d,.val"></label><label>Seamly measurements (optional)<input id="pattern-measurements-file" type="file" accept=".smis,.vit,.smms,.vst"></label><button data-action id="pattern-starter">Load teaching bodice</button><button data-action id="pattern-save">Save pattern JSON</button></div><p id="pattern-status" role="status" aria-live="polite">Loading pattern engine | </p>
    <section id="pattern-warnings" class="notice warn" aria-labelledby="pattern-warnings-heading" hidden></section>
    <div id="pattern-workbench"></div>`) +
        panel(5, 'Digitize', 'Recover paper patterns from a PNG or JPEG on the BETA A3 target.', `<div class="floor-grid"><section class="floor-card"><h2>1. Print and check</h2><p>Print A3 landscape at 100%. Disable fit to page. Ten checker cells must measure 50 mm in both directions, within 0.2 mm.</p><button data-action id="digitize-target">Print target PDF</button><h2>2. Photograph</h2><p>Use dark matte backing. Leave the outer 36 mm clear and at least four codes visible. Separate pieces by 3 mm. Draw grain lines and notch ticks.</p><label>PNG/JPEG photo, up to 30 MB<input id="digitize-photo" type="file" accept="image/png,image/jpeg"></label><p id="digitize-status" role="status" aria-live="polite">Choose a photo. Processing stays on this computer.</p><div id="digitize-warnings"></div></section><section class="floor-card"><h2>3. Review and accept</h2><img id="digitize-overlay" class="scan-overlay" alt="Scan overlay: green outlines, blue grain, orange notches and red drill candidates" hidden><p class="hint">Check scale, outlines, grain and marks. Size, material and pairs are not inferred.</p><div id="digitize-pieces"></div>${field('digitize-size', 'Size for accepted pieces', 'M')}<label class="check"><input id="digitize-confirm" type="checkbox">I checked the overlay and scale.</label><div class="floor-actions"><button data-action id="digitize-pattern">Accept to Pattern</button><button data-action id="digitize-marker" class="primary">Accept to Marker</button></div></section></div>`) +
        panel(6, 'Rolls', 'Inspect cloth, keep shade and shrinkage groups separate, and allocate rolls.', `<div class="floor-actions"><label>Import roll CSV or register JSON<input id="roll-file" type="file" accept=".csv,.json"></label><button data-action id="roll-template">Download CSV template</button><button data-action id="roll-save">Save register JSON</button></div><details class="floor-card" open><summary>Add or update a roll</summary><form id="roll-form"><div class="fields">${field('roll-id', 'Roll id', 'R1')}${field('roll-lot', 'Lot', 'LOT1')}${field('roll-dye', 'Dye lot', 'DYE1')}${field('roll-shade', 'Shade group', 'A')}${number('roll-ticket', 'Ticket length (m)', 100, .001)}${number('roll-length', 'Measured length (m)', 100, .001)}${number('roll-width', 'Usable width (mm)', 1500, 1)}${number('roll-shrink-length', 'Length shrinkage (%)', 0, -99)}${number('roll-shrink-width', 'Width shrinkage (%)', 0, -99)}</div><button data-action type="submit">Save roll</button></form></details><div id="roll-table"></div><div class="floor-grid"><section class="floor-card"><h2>Four-point inspection</h2><p class="hint">Enter each scored observation. Rust caps points per yard and uses the buyer policy.</p><form id="defect-form"><div class="fields"><label>Roll<select id="defect-roll"></select></label>${field('defect-id', 'Observation id', 'D1')}${number('defect-along', 'Along roll (m)', 1)}${number('defect-across', 'Across width (mm)', 50)}${number('defect-length', 'Defect length (mm)', 50, .01)}${number('defect-width', 'Defect width (mm)', 20, .01)}<label>Penalty points<select id="defect-points"><option>1</option><option>2</option><option>3</option><option>4</option></select></label>${field('defect-type', 'Defect type', 'Hole')}</div><button data-action type="submit">Add observation</button></form><div id="defect-table"></div><div class="fields">${number('inspection-limit', 'Buyer limit', 40)}<label>Score unit<select id="inspection-unit"><option value="per100_square_yards">Per 100 square yards</option><option value="per100_square_metres">Per 100 square metres</option></select></label>${number('shrink-band', 'Shrinkage band step (%)', 2, .01)}</div><button data-action id="inspect-rolls" class="primary">Inspect and group rolls</button><p id="roll-status" role="status" aria-live="polite"></p><div id="inspection-result"></div></section><section class="floor-card"><h2>Spreading plan</h2><p id="spread-source">Make a cut plan, or use the current valid marker.</p><div class="fields"><label>Source<select id="spread-source-select"><option value="plan">Current cut plan</option><option value="marker">Current editor marker</option></select></label>${number('spread-plies', 'Plies for current marker', 20, 1, '1')}<label>Compatible stock group<select id="spread-group"><option value="">Inspect rolls first</option></select></label></div><label class="check"><input type="checkbox" id="failed-stock">Include failed rolls after buyer approval</label><p class="hint">Whole plies only. Splicing needs reviewed safe zones. Allocation does not repair defects.</p><button data-action id="spread-rolls" class="primary">Allocate rolls</button><p id="spread-status" role="status" aria-live="polite"></p><div id="spread-result"></div></section></div>`) +
        panel(7, 'Bundles', 'Number cut parts, print tickets, record damage and reconcile fabric use.', `<section class="floor-card"><h2>Prepare a lay</h2><div class="fields"><label>Marker source<select id="bundle-source"><option value="marker">Current editor marker</option></select></label>${field('bundle-lay', 'Lay id', 'LAY-01')}${field('bundle-order', 'Order id', 'ORDER-01')}${field('bundle-colour', 'Colour', 'Blue')}${number('bundle-plies', 'Plies', 20, 1, '1')}${number('bundle-size', 'Pieces per bundle', 20, 1, '1')}${number('bundle-first', 'First floor bundle number', 1, 1, '1')}</div><label>Shade runs: first,last,shade_group (inclusive plies)<textarea id="bundle-runs" rows="3">1,20,A</textarea></label><p class="hint">Verify runs against actual spreading. Folded markers expand into physical parts. The marker must pass checks and have garment identities.</p><button data-action id="make-bundles" class="primary">Make bundles</button><p id="bundle-status" role="status" aria-live="polite"></p></section><div class="floor-actions"><label>Paper<select id="ticket-paper"><option value="a4">A4: ten 100 x 50 mm labels</option><option value="label100x50">Label printer: 100 x 50 mm</option></select></label><button data-action id="bundle-tickets">Print tickets PDF</button><button data-action id="bundle-stickers">Print stickers PDF</button><button data-action id="bundle-save">Save bundles and re-cut ledger</button></div><p class="hint">Print at 100%. QR codes carry offline sewing WIP identities: order, lay, bundle. Marker garment ids and floor bundle numbers differ.</p><div id="bundle-table"></div><div class="floor-grid"><section class="floor-card"><h2>Re-cut entry</h2><form id="recut-form"><label>Physical piece<select id="recut-piece"></select></label>${field('recut-reason', 'Reason', 'Knife damage')}<button data-action type="submit">Record damaged piece</button></form><div id="recut-ledger"></div></section><section class="floor-card"><h2>Cost estimate</h2><p class="hint">Uses the editor job. The early geometry estimate excludes end allowances and roll losses.</p><div class="fields">${number('cost-garments', 'Garments in current marker', 2, 1, '1')}${number('cost-price', 'Fabric price (INR per metre)', 150)}</div><button data-action id="cost-estimate">Estimate fabric per garment</button><div id="cost-result"></div></section></div><section class="floor-card"><h2>Cutting report</h2><p class="hint">Uses the saved bundle lay. Include replacements consistently in the count and area.</p><form id="report-form"><div class="fields">${number('report-issued', 'Fabric issued (mm)', 0)}${number('report-end', 'End bits returned (mm)', 0)}${number('report-pieces', 'Actual physical pieces cut', 0, 0, '1')}${number('report-area', 'Actual outline area cut (mm²)', 0)}${number('report-allowance', 'Head + tail allowance per ply (mm)', 0)}</div><button data-action type="submit">Reconcile cutting report</button></form><div id="report-result"></div></section>`));
    async function evaluate(next: PatternDocument): Promise<PatternGrade> { return core ? core.grade(next) : json<PatternGrade>('pattern/grade', { document: next }); }
    async function acceptPattern(next: PatternDocument): Promise<void> { const generation = projectGeneration; await evaluate(next); if (generation !== projectGeneration) return; patternReady = true; doc = next; patternSaved = persist(); renderPattern(); $('#pattern-status').textContent = `${doc.sizes.length} size${doc.sizes.length === 1 ? '' : 's'} evaluated by Rust ${core ? 'WASM' : isDesktop() ? 'native' : 'server'}. ${patternSaved ? 'Draft saved on this computer.' : isDesktop() ? 'Waiting for native autosave.' : 'Browser storage is full. Save pattern JSON now.'}`; }
    function renderPattern(): void { patternUI?.sync(doc); renderImportWarnings(); }
    /** QA-004: read the warnings from the document every time, so they stay through edits, saves, reloads and exports. */
    function renderImportWarnings(): void {
        const warnings = seamlyWarnings(doc.meta), panel = $('#pattern-warnings');
        panel.hidden = !warnings.length;
        panel.innerHTML = warningsHtml(warnings, 'These notes come from the Seamly2D import. They stay with this pattern through edits, saves and exports, and go with it to the Marker editor. Check each one before cutting.', 'pattern-warnings-heading', esc);
    }
    $('#pattern-file').addEventListener('change', () => void task('#pattern-status', async () => { const file = $<HTMLInputElement>('#pattern-file').files?.[0]; if (!file)
        return; if (file.size > 16 * 1024 * 1024)
        throw new Error('Pattern file exceeds 16 MB.'); let next: PatternDocument, warnings: string[] = []; if (/\.(json|betapattern)$/i.test(file.name))
        next = readPattern(await file.text());
    else {
        const m = $<HTMLInputElement>('#pattern-measurements-file').files?.[0];
        if (m && m.size > 16 * 1024 * 1024)
            throw new Error('Measurement file exceeds 16 MB.');
        const patternXml = await file.text(), measurementXml = m ? await m.text() : undefined;
        const response = core ? core.import_seamly(patternXml, measurementXml, 'M') : await json<PatternImportResponse>('pattern/import', { pattern: patternXml, measurements: measurementXml, size: 'M' });
        next = readPattern(JSON.stringify(response.document));
        warnings = response.warnings;
    } await acceptPattern(next); if (warnings.length)
        $('#pattern-status').textContent += ` Import notes: ${warnings.join(' ')}`; $<HTMLInputElement>('#pattern-file').value = ''; }));
    $('#pattern-starter').onclick = () => void task('#pattern-status', () => acceptPattern(readPattern(JSON.stringify(starter))));
    $('#pattern-save').onclick = () => download(JSON.stringify(doc, null, 2), 'pattern.doc.json');
    async function exportPatternDxf(): Promise<void> {
        if(core) download(core.export_dxf(doc), 'pattern.dxf', 'application/dxf');
        else download(await (await editor.api('pattern/export-dxf',JSON.stringify({document:doc}))).blob(),'pattern.dxf');
        const open = seamlyWarnings(doc.meta).length;
        $('#pattern-status').textContent=`DXF downloaded with all sizes and BETA grade-rule comments.${open ? ` ${open} Seamly2D import warning${open === 1 ? ' is' : 's are'} still open (see the panel).` : ''}`;
    }
    async function sendPattern(ratio: Record<string,number>): Promise<void> {
        const made=core?core.to_job(doc,editor.get().job.fabric,ratio):await json<Job>('pattern/to-job',{document:doc,fabric:editor.get().job.fabric,ratio});
        const job=carryWarnings(made,doc.meta,doc.name);
        editor.open({job,result:emptyResult(job)});screen(0);editor.message('Pattern sent to Editor. Nest the pieces, then make a cut plan.');
    }
    $('#digitize-target').onclick = () => void task('#digitize-status', async () => { download(await (await editor.api('digitize/target', '{}')).blob(), 'beta-a3-target.pdf'); $('#digitize-status').textContent = 'Print A3 landscape at 100%, then check the 50 mm scale.'; });
    $('#digitize-photo').addEventListener('change', () => void task('#digitize-status', async () => { scan = undefined; $<HTMLInputElement>('#digitize-confirm').checked = false; $('#digitize-overlay').hidden = true; $('#digitize-pieces').innerHTML = ''; const file = $<HTMLInputElement>('#digitize-photo').files?.[0]; if (!file)
        return; if (file.size > 30 * 1024 * 1024)
        throw new Error('Choose a photo no larger than 30 MB.'); const next = await (await editor.api('digitize', file)).json() as DigitizeResponse; if (!next.overlay_png.startsWith('data:image/png;base64,'))
        throw new Error('Scan did not return an overlay PNG.'); scan = next; $<HTMLImageElement>('#digitize-overlay').src = scan.overlay_png; $('#digitize-overlay').hidden = false; $('#digitize-warnings').innerHTML = `<ul>${scan.report.warnings.map(w => `<li>${esc(w)}</li>`).join('')}</ul>`; $('#digitize-pieces').innerHTML = scan.pieces.map((p, i) => `<label class="check"><input type="checkbox" data-scan-piece="${i}" checked>${esc(p.name)}  |  ${p.outline.length} outline points  |  ${p.grain ? 'grain found' : 'review missing grain'}</label>`).join(''); $('#digitize-status').textContent = `${scan.pieces.length} pieces; ${scan.report.detected_markers} codes. Review the overlay before accepting.`; }));
    function scanJob(): Job { if (!scan || !$<HTMLInputElement>('#digitize-confirm').checked)
        throw new Error('Review the overlay and check the acceptance box first.'); return acceptedJob(Array.from(window.document.querySelectorAll<HTMLInputElement>('[data-scan-piece]:checked'), i => scan!.pieces[Number(i.dataset.scanPiece)]!), text('digitize-size'), editor.get().job.fabric); }
    $('#digitize-pattern').onclick = () => void task('#digitize-status', async () => { const job = scanJob(); await acceptPattern(core ? core.from_job(job) : await json<PatternDocument>('pattern/from-job', job)); screen(4); $('#digitize-status').textContent = 'Accepted pieces sent to Pattern.'; });
    $('#digitize-marker').onclick = () => void task('#digitize-status', async () => { const job = scanJob(); editor.open({ job, result: emptyResult(job) }); screen(0); editor.message('Paper pieces sent to Editor. Check grain and quantities before nesting.'); });
    function invalidateRolls(): void { inspection = undefined; inspectionKey = ''; spread = undefined; spreadKey = ''; $('#inspection-result').innerHTML = ''; $('#spread-result').innerHTML = ''; $('#spread-group').innerHTML = '<option value="">Inspect rolls first</option>'; persist(); renderRolls(); }
    function renderRolls(): void {
        $('#roll-table').innerHTML = table('Roll register', ['Roll', 'Lot / dye lot', 'Shade', 'Ticket / measured (m)', 'Width (mm)', 'Shrinkage L / W (%)', 'Edit'], register.rolls.map((r, i) => [esc(r.id), `${esc(r.lot)} / ${esc(r.dye_lot)}`, esc(r.shade_group), `${r.ticket_length_m} / ${r.measured_length_m}`, String(r.usable_width_mm), `${r.shrinkage.length_pct ?? 0} / ${r.shrinkage.width_pct ?? 0}`, `<button data-edit-roll="${i}">Edit</button> <button data-delete-roll="${i}">Delete</button>`]));
        const selected = text('defect-roll');
        $('#defect-roll').innerHTML = register.rolls.map(r => `<option ${selected === r.id ? 'selected' : ''}>${esc(r.id)}</option>`).join('');
        renderDefects();
    }
    function renderDefects(): void { const r = register.rolls.find(r => r.id === text('defect-roll')); $('#defect-table').innerHTML = r ? table(`Observations for ${r.id}`, ['Id', 'Position (m / mm)', 'Points', 'Type', ''], r.defects.map((d, i) => [esc(d.id), `${d.along_m} / ${d.across_mm}`, String(d.points), esc(d.defect_type), `<button data-delete-observation="${i}">Delete</button>`])) : ''; }
    $('#defect-roll').addEventListener('change', renderDefects);
    $('#roll-form').addEventListener('submit', event => { event.preventDefault(); void task('#roll-status', async () => { const id = text('roll-id').trim(); if (!id)
        throw new Error('Enter a roll id.'); const existing = register.rolls.find(r => r.id === id), roll = { id, lot: text('roll-lot').trim(), dye_lot: text('roll-dye').trim(), shade_group: text('roll-shade').trim(), ticket_length_m: num('roll-ticket'), measured_length_m: num('roll-length'), usable_width_mm: num('roll-width'), shrinkage: { length_pct: num('roll-shrink-length'), width_pct: num('roll-shrink-width') }, defects: existing?.defects ?? [] }; const candidate = { rolls: [...register.rolls.filter(r => r.id !== id), roll] }; await json('roll/inspect', { register: candidate }); register = candidate; invalidateRolls(); $('#roll-status').textContent = `Roll ${id} saved. Inspect again before spreading.`; }); });
    $('#roll-table').addEventListener('click', event => { const b = (event.target as Element).closest<HTMLButtonElement>('button'); if (!b || working)
        return; const r = register.rolls[Number(b.dataset.editRoll ?? b.dataset.deleteRoll)]; if (!r)
        return; if (b.dataset.deleteRoll !== undefined) {
        register.rolls.splice(Number(b.dataset.deleteRoll), 1);
        invalidateRolls();
        return;
    } for (const [id, v] of Object.entries({ 'roll-id': r.id, 'roll-lot': r.lot, 'roll-dye': r.dye_lot, 'roll-shade': r.shade_group, 'roll-ticket': r.ticket_length_m, 'roll-length': r.measured_length_m, 'roll-width': r.usable_width_mm, 'roll-shrink-length': r.shrinkage.length_pct ?? 0, 'roll-shrink-width': r.shrinkage.width_pct ?? 0 }))
        $<HTMLInputElement>(`#${id}`).value = String(v); $('#roll-id').focus(); });
    $('#defect-form').addEventListener('submit', event => { event.preventDefault(); void task('#roll-status', async () => { const next = structuredClone(register), roll = next.rolls.find(r => r.id === text('defect-roll')); if (!roll)
        throw new Error('Add a roll first.'); roll.defects.push({ id: text('defect-id').trim(), along_m: num('defect-along'), across_mm: num('defect-across'), length_mm: num('defect-length'), width_mm: num('defect-width'), points: num('defect-points'), defect_type: text('defect-type').trim(), action: 'cut_around' }); await json('roll/inspect', { register: next }); register = next; invalidateRolls(); $('#roll-status').textContent = 'Observation saved. Inspect again to update pass/fail.'; }); });
    $('#defect-table').addEventListener('click', event => { const b = (event.target as Element).closest<HTMLButtonElement>('[data-delete-observation]'); if (!b || working)
        return; register.rolls.find(r => r.id === text('defect-roll'))?.defects.splice(Number(b.dataset.deleteObservation), 1); invalidateRolls(); });
    $('#roll-file').addEventListener('change', () => void task('#roll-status', async () => { const file = $<HTMLInputElement>('#roll-file').files?.[0]; if (!file)
        return; if (file.size > 10 * 1024 * 1024)
        throw new Error('Roll register exceeds 10 MB.'); const next = file.name.endsWith('.json') ? JSON.parse(await file.text()) as RollRegister : readRollCsv(await file.text()); await json('roll/inspect', { register: next }); register = next; invalidateRolls(); $('#roll-status').textContent = `${register.rolls.length} rolls imported. Inspect before spreading.`; $<HTMLInputElement>('#roll-file').value = ''; }));
    $('#roll-template').onclick = () => download(`${rollColumns.join(',')}\nR1,LOT1,DYE1,A,100,100,1500,0,0\n`, 'rolls.csv', 'text/csv');
    $('#roll-save').onclick = () => download(JSON.stringify(register, null, 2), 'roll-register.json');
    $('#inspect-rolls').onclick = () => void task('#roll-status', async () => { if (!register.rolls.length)
        throw new Error('Add or import rolls first.'); inspection = await json<InspectResponse>('roll/inspect', { register, settings: { limit: num('inspection-limit'), unit: text('inspection-unit'), point_cap: 'per_yard' }, grouping: { band_step_pct: num('shrink-band') } }); inspectionKey = JSON.stringify(register); $('#inspection-result').innerHTML = table('Inspection results', ['Roll', 'Raw / capped', 'Per 100 yd2', 'Per 100 m2', 'Result'], inspection.inspection.rolls.map(r => [esc(r.roll_id), `${r.score.raw_points} / ${r.score.scored_points}`, r.score.points_per_100_sq_yd.toFixed(2), r.score.points_per_100_m2.toFixed(2), `<strong class="${r.score.passed ? '' : 'error-text'}">${r.score.passed ? 'PASS' : 'FAIL'}</strong>`])) + table('Shade / shrinkage groups', ['Lot / dye lot / shade', 'Rolls', 'Mean shrinkage L / W (%)', 'Use'], inspection.groups.map((g, i) => [`${esc(g.lot)} / ${esc(g.dye_lot)} / ${esc(g.shade_group)}`, g.roll_ids.map(esc).join(', '), `${g.average_shrinkage.length_pct?.toFixed(2)} / ${g.average_shrinkage.width_pct?.toFixed(2)}`, `<button data-group-to-marker="${i}">Apply shrinkage to marker</button>`])); $('#spread-group').innerHTML = inspection.groups.map((g, i) => `<option value="${i}">${esc(g.lot)} / ${esc(g.dye_lot)} / ${esc(g.shade_group)}  |  ${g.roll_ids.length} rolls</option>`).join(''); $('#roll-status').textContent = 'Inspection ready. Select one compatible group for spreading.'; });
    $('#inspection-result').addEventListener('click', event => { const b = (event.target as Element).closest<HTMLButtonElement>('[data-group-to-marker]'), g = inspection?.groups[Number(b?.dataset.groupToMarker)]; if (!b || !g || working)
        return; const next = structuredClone(editor.get()); next.job.fabric.shrinkage = g.average_shrinkage; next.result = emptyResult(next.job); editor.open(next); screen(0); editor.message('Group shrinkage applied. Nest again and remake the cut plan.'); });
    function spreadSource(): {
        id: string;
        job: Job;
        result: MarkerResult;
        plies: number;
        end_allowance_mm: number;
    }[] { const p = planning(); if (text('spread-source-select') === 'plan')
        return (p.plan?.lays ?? []).flatMap((l, i) => l.result && p.job ? [{ id: `LAY-${String(i + 1).padStart(2, '0')}`, job: { ...structuredClone(p.job), order: { sizes: l.ratio } }, result: l.result, plies: l.plies, end_allowance_mm: p.endAllowance }] : []); const s = editor.get(); return [{ id: 'LAY-01', job: s.job, result: s.result, plies: num('spread-plies'), end_allowance_mm: 0 }]; }
    $('#spread-rolls').onclick = () => void task('#spread-status', async () => {
        if (!inspection || inspectionKey !== JSON.stringify(register))
            throw new Error('Inspect the current roll register first.');
        const g = inspection.groups[Number(text('spread-group'))];
        if (!g)
            throw new Error('Choose a compatible stock group.');
        const passed = inspection.inspection.rolls.filter(r => r.score.passed || $<HTMLInputElement>('#failed-stock').checked).map(r => r.roll_id), ids = g.roll_ids.filter(id => passed.includes(id));
        if (!ids.length)
            throw new Error('This group has no approved rolls.');
        const source = spreadSource();
        if (!source.length)
            throw new Error('Make a cut plan with saved markers, or choose the current marker.');
        for (const s of source) {
            const sh = s.job.fabric.shrinkage;
            if (Math.abs((sh?.length_pct ?? 0) - (g.average_shrinkage.length_pct ?? 0)) > 1e-6 || Math.abs((sh?.width_pct ?? 0) - (g.average_shrinkage.width_pct ?? 0)) > 1e-6)
                throw new Error('Apply group shrinkage to the marker, then nest and remake the cut plan.');
            if (!(await json<{
                ok: boolean;
            }>('validate', { job: s.job, result: s.result })).ok)
                throw new Error(`${s.id} marker must pass validation.`);
        }
        spread = await json<SpreadingPlan>('roll/spread', { register, markers: source, shade_rule: { mode: 'required', identity: { lot: g.lot, dye_lot: g.dye_lot, shade_group: g.shade_group } }, eligible_roll_ids: ids });
        spreadSources = structuredClone(source);
        spreadKey = JSON.stringify(source);
        renderSpread();
        $('#spread-status').textContent = spread.complete ? 'All plies allocated. Review defects and actual shade runs before cutting.' : 'Stock shortage: partial allocation. Do not spread an incomplete lay.';
    });
    function renderSpread(): void { if (!spread)
        return; $('#spread-result').innerHTML = `<p class="${spread.complete ? '' : 'error-text'}"><strong>${spread.complete ? 'COMPLETE' : 'PARTIAL'}</strong>  |  used ${spread.total_used_m.toFixed(3)} m  |  end bits ${spread.total_end_bits_m.toFixed(3)} m</p>` + table('Rolls for each lay', ['Lay', 'Roll', 'Plies', 'Used (m)', 'End bit (m)', 'Shade'], spread.lays.flatMap(l => l.allocations.map(a => [esc(l.lay.id), esc(a.roll_id), String(a.plies), a.used_m.toFixed(3), a.end_bit_m.toFixed(3), esc(a.shade_group)]))) + `<div class="floor-actions">${spread.lays.map(l => `<button data-spread-bundles="${esc(l.lay.id)}">Send ${esc(l.lay.id)} to Bundles</button>`).join('')}</div>` + table('Remaining stock', ['Roll', 'Opened', 'Remaining (m)'], spread.rolls.map(r => [esc(r.roll_id), r.opened ? 'Yes' : 'No', r.remaining_m.toFixed(3)])) + `<ul>${spread.warnings.map(w => `<li>${esc(w)}</li>`).join('')}</ul>`; }
    $('#spread-result').addEventListener('click', event => {
        const button = (event.target as Element).closest<HTMLButtonElement>('[data-spread-bundles]');
        if (!button) return;
        void task('#spread-status', async () => {
            if (!spread || spreadKey !== JSON.stringify(spreadSource()) || inspectionKey !== JSON.stringify(register)) throw new Error('The source changed. Allocate rolls again.');
            const source = spreadSources.find(s => s.id === button.dataset.spreadBundles);
            if (!source) throw new Error('Source marker is unavailable. Allocate rolls again.');
            const lay = await json<PhysicalLay>('cutroom/from-spread', {job:source.job,result:source.result,register,spread,lay_id:source.id,order:text('bundle-order').trim(),colour:text('bundle-colour').trim()});
            importedSpread = {job:source.job,result:source.result,lay};
            screen(7);
            $<HTMLSelectElement>('#bundle-source').value = 'spread';
            $<HTMLInputElement>('#bundle-lay').value = lay.id;
            $<HTMLInputElement>('#bundle-plies').value = String(lay.plies.at(-1)!.last);
            const csv = (v:string) => '"' + v.replace(/"/g,'""') + '"';
            $<HTMLTextAreaElement>('#bundle-runs').value = lay.plies.map(r => `${r.first},${r.last},${csv(r.shade_group)}`).join('\n');
            $('#bundle-status').textContent = 'Actual allocated shade identities loaded from the audited spread. Review them before bundling.';
        });
    });
    function markerSource(): {
        job: Job;
        result: MarkerResult;
        plies: number;
    } | undefined { const choice = text('bundle-source'); if (choice === 'spread') return importedSpread ? {...importedSpread,plies:importedSpread.lay.plies.at(-1)!.last} : undefined; if (choice === 'marker')
        return { ...editor.get(), plies: num('bundle-plies') }; const p = planning(), l = p.plan?.lays[Number(choice)]; return l?.result && p.job ? { job: { ...structuredClone(p.job), order: { sizes: l.ratio } }, result: l.result, plies: l.plies } : undefined; }
    function shadeRuns(): PlyRun[] { return csvRows(text('bundle-runs')).map(row => { if (row.length !== 3)
        throw new Error('Use first,last,shade_group for each shade run.'); return { first: Number(row[0]), last: Number(row[1]), shade_group: row[2]!.trim() }; }); }
    $('#bundle-source').addEventListener('change', () => { const s = markerSource(); if (!s)
        return; $<HTMLInputElement>('#bundle-plies').value = String(s.plies); const i = text('bundle-source'); if (i !== 'marker')
        $<HTMLInputElement>('#bundle-lay').value = `LAY-${String(Number(i) + 1).padStart(2, '0')}`; });
    $('#make-bundles').onclick = () => void task('#bundle-status', async () => { const s = markerSource(); if (!s)
        throw new Error('Choose a marker with a saved result.'); const runs = shadeRuns(); if (runs.at(-1)?.last !== num('bundle-plies'))
        throw new Error('Shade runs must end at the entered ply count.'); if (text('bundle-source') !== 'marker' && s.plies !== num('bundle-plies'))
        throw new Error('Ply count must match the cut-plan lay.'); if(text('bundle-source') === 'spread' && (text('bundle-lay') !== importedSpread?.lay.id || JSON.stringify(runs) !== JSON.stringify(importedSpread?.lay.plies))) throw new Error('Keep the audited spread identity and shade runs, or choose a manual marker source.'); const lay = text('bundle-source') === 'spread' ? structuredClone(importedSpread!.lay) : await json<PhysicalLay>('cutroom/lay', { job: s.job, result: s.result, id: text('bundle-lay').trim(), order: text('bundle-order').trim(), colour: text('bundle-colour').trim(), plies: runs }); lay.order=text('bundle-order').trim(); lay.colour=text('bundle-colour').trim(); const next = await json<Bundle[]>('cutroom/bundles', { lay, rules: { pieces_per_bundle: num('bundle-size'), first_bundle: num('bundle-first'), sticker_offset: 0 } }); bundles = next; bundleSource = { job: s.job, result: s.result, lay }; persist(); renderBundles(); $('#bundle-status').textContent = `${bundles.length} bundles ready. Review shade, identifiers and print scale.`; });
    for (const [id, route] of [['bundle-tickets', 'tickets'], ['bundle-stickers', 'stickers']])
        $(`#${id}`).onclick = () => void task('#bundle-status', async () => { if (!bundles.length)
            throw new Error('Make bundles first.'); download(await (await editor.api(`cutroom/${route}`, JSON.stringify({ bundles, paper: text('ticket-paper') }))).blob(), `bundle-${route}.pdf`); $('#bundle-status').textContent = `${route} downloaded. Print at 100% scale.`; });
    $('#bundle-save').onclick = () => download(JSON.stringify({ bundles, ledger, source: bundleSource }, null, 2), 'bundles-and-recuts.json');
    function renderBundles(): void { $('#bundle-table').innerHTML = table('Sewing-floor bundles', ['Bundle', 'Lay', 'Size', 'Part', 'Hand', 'Shade', 'Plies', 'Quantity'], bundles.map(b => [String(b.number), esc(b.lay), esc(b.size), esc(b.piece_id), b.stickers[0]?.piece.mirrored ? 'Mirrored' : 'Normal', esc(b.shade_group), b.plies.join(' to '), String(b.stickers.length)])); $('#recut-piece').innerHTML = bundles.flatMap((b, i) => b.stickers.map((s, j) => `<option value="${i}:${j}">Bundle ${b.number}  |  ${esc(b.size)} ${esc(b.piece_id)}  |  ply ${s.piece.ply}  |  garment ${s.piece.garment + 1}  |  slot ${s.piece.slot + 1}${s.piece.mirrored ? ' mirrored' : ''}</option>`)).join(''); renderLedger(); }
    function renderLedger(): void { $('#recut-ledger').innerHTML = table('Replacement ledger', ['Id', 'Order / lay', 'Bundle', 'Part / ply', 'Reason', 'Status'], ledger.map(d => [esc(d.id), `${esc(d.order)} / ${esc(d.lay)}`, String(d.bundle), `${esc(d.piece.piece_id)} / ${d.piece.ply}`, esc(d.reason), esc(d.status)])); }
    $('#recut-form').addEventListener('submit', event => { event.preventDefault(); void task('#bundle-status', async () => { const [bi, si] = text('recut-piece').split(':').map(Number), bundle = bundles[bi!], piece = bundle?.stickers[si!]?.piece; if (!bundle || !piece)
        throw new Error('Choose a bundled physical piece.'); if (ledger.some(d => d.order === bundle.order && d.lay === bundle.lay && d.bundle === bundle.number && JSON.stringify(d.piece) === JSON.stringify(piece)))
        throw new Error('Damage is already recorded for this physical piece.'); ledger.push(await json<Damage>('cutroom/recut', { id: crypto.randomUUID(), bundle, piece, reason: text('recut-reason').trim() })); persist(); renderLedger(); $('#bundle-status').textContent = 'Damage recorded. Save the ledger for replacement tracking.'; }); });
    $('#cost-estimate').onclick = () => void task('#bundle-status', async () => { const c = await json<CostResponse>('cutroom/cost', { job: editor.get().job, garments: num('cost-garments'), price_per_m: num('cost-price') }); $('#cost-result').innerHTML = `<p class="floor-number">${(c.estimate.fabric_per_garment_mm / 1000).toFixed(3)} m / garment</p><p>INR ${c.fabric_cost_per_garment.toFixed(2)} fabric per garment  |  ${(c.estimate.expected_efficiency * 100).toFixed(1)}% estimated efficiency</p><ul>${c.estimate.warnings.map(w => `<li>${esc(w)}</li>`).join('')}</ul>`; $('#bundle-status').textContent = 'Geometry estimate ready. Review limits before quoting.'; });
    $('#report-form').addEventListener('submit', event => { event.preventDefault(); void task('#bundle-status', async () => { if (!bundleSource)
        throw new Error('Make bundles to save the source marker and lay.'); const s = bundleSource, input = { source: { job: s.job, result: s.result, id: s.lay.id, order: s.lay.order, colour: s.lay.colour, plies: s.lay.plies }, end_allowance_per_ply_mm: num('report-allowance'), actual_piece_area_mm2: num('report-area'), actual_pieces: num('report-pieces'), recut_count: ledger.filter(d => d.order === s.lay.order && d.lay === s.lay.id).length, issued_mm: num('report-issued'), end_bits_mm: num('report-end') }; const r = await json<CuttingReport>('cutroom/report-marker', input); $('#report-result').innerHTML = table(`Cutting report  |  ${r.id}`, ['Planned (m)', 'Actual (m)', 'Variance (m)', 'Pieces planned / cut', 'Re-cuts', 'Cut efficiency'], [[(r.planned_mm / 1000).toFixed(3), (r.actual_mm / 1000).toFixed(3), (r.variance_mm / 1000).toFixed(3), `${r.planned_pieces} / ${r.pieces_cut}`, String(r.recut_count), r.cutting_efficiency_pct == null ? 'No fabric issued' : `${r.cutting_efficiency_pct.toFixed(2)}%`]]); $('#bundle-status').textContent = 'Cutting report reconciled by Rust.'; }); });
    $('#screen-3').insertAdjacentHTML('beforeend', `<h2>Day in the cutting room</h2><button id="print-day" type="button">Print the day walkthrough</button><ol class="day-walk"><li><strong>Pattern:</strong> open the draft. Check measurements, seam allowance (cloth outside the stitch line), grain and notches.</li><li><strong>Grade:</strong> change measurements or formulas. Compare overlaid sizes and save a DXF backup.</li><li><strong>Marker:</strong> send the size ratio to Editor. Set width, shrinkage, direction and gaps. Nest and review every issue.</li><li><strong>Cut plan:</strong> enter the order by size. Review lays, ply counts, over-cut and fabric use.</li><li><strong>Roll check:</strong> measure rolls and enter defects. Use the buyer's four-point limit with the correct area unit. Keep lot, dye lot, shade and shrinkage groups separate.</li><li><strong>Spread:</strong> choose a compatible group. Apply its shrinkage before nesting. Allocate whole plies and review end bits, shortages and defects.</li><li><strong>Cut:</strong> approve the physical lay and marker. Record issued fabric, returned remnants, pieces cut and damage.</li><li><strong>Bundles:</strong> verify actual shade runs. Bundle by part, size, handedness and shade. Print tickets and stickers at 100%.</li><li><strong>Sewing:</strong> send bundles with tickets. The QR contains an offline order / lay / bundle identity. Save the re-cut ledger and cutting report.</li></ol><h2>Pattern</h2><p>JSON keeps measurements and formulas editable. Seamly2D import supports a documented subset. Unsupported operations return errors. Drawings re-evaluate locally in Rust WASM. DXF carries graded outlines and BETA's public rule-table extension.</p><h2>Digitize</h2><p>Print the BETA A3 target and check it with a ruler. Photograph flat paper on dark backing. Review the overlay before accepting pieces. Automatic grain and small marks need review. This screen supports one A3 board.</p><h2>Rolls</h2><p>Import the CSV template or enter rolls. Scores use measured length and usable width. Yard and metre area scores differ. A passing lot average does not approve every roll. Apply the compatible group's shrinkage before nesting. Spreading leaves recoverable end bits; these are not all scrap.</p><h2>Bundles</h2><p>A floor bundle holds one part, size, shade and handedness over consecutive plies. All parts of a garment share its sticker number. A sticker 1-60 means garment 1 out of 60, not a ply range. Re-cut entries retain physical piece identities. Costing is an uncalibrated estimate and excludes roll loss and end allowances.</p><h2>Offline files and recovery</h2><p>All assets, WASM and calculations stay on this computer. Pattern, roll, bundle and re-cut drafts are saved in this browser. Save JSON backups before clearing browser data. Nesting, photo digitizing, roll planning and PDF printing require the local Rust server.</p>`);
    $('#print-day').onclick = () => window.print();
    function activate(index: number): void { if (index === 4)
        { patternUI?.activate(); } if (index === 6) {
        const p = planning();
        $('#spread-source').textContent = p.plan ? `${p.plan.lays.length} lays in ${p.plan.order_name}. Uses saved marker lengths.` : 'Make a cut plan, or choose the current valid editor marker.';
        try {
            if (spread && spreadKey !== JSON.stringify(spreadSource()))
                $('#spread-status').textContent = 'Marker or cut plan changed. Allocate rolls again.';
        }
        catch {
            $('#spread-status').textContent = 'Check the ply count.';
        }
    } if (index === 7) {
        const old = text('bundle-source');
        $('#bundle-source').innerHTML = '<option value="marker">Current editor marker</option>' + (importedSpread ? '<option value="spread">Audited spreading lay</option>' : '') + (planning().plan?.lays ?? []).map((l, i) => `<option value="${i}">Cut plan lay ${i + 1}  |  ${l.plies} plies</option>`).join('');
        $<HTMLSelectElement>('#bundle-source').value = old || 'marker';
    } }
    patternUI = installPatternEditor($('#pattern-workbench'), { get: () => doc, core: () => core, accept: acceptPattern, call: json, download, screen, exportDxf: exportPatternDxf, sendMarker: sendPattern, saved: () => patternSaved, ready: () => patternReady });
    renderPattern();
    renderRolls();
    renderBundles();
    void loadPatternCore().then(async (loaded) => { core = loaded; await acceptPattern(doc); }).catch(async () => { try {
        await acceptPattern(doc);
        $('#pattern-status').textContent += ' WASM unavailable; local server evaluation active.';
    }
    catch {
        $('#pattern-status').textContent = 'Pattern engine unavailable. Build WASM assets or start the local server.';
    } });
    return {
        busy: () => working, activate,
        floorRegister: () => structuredClone(register),
        floorReceive: (incoming: RollRegister) => {
            if (working) throw new Error('Wait for the current Rolls operation before importing stock.');
            const next = structuredClone(register);
            for (const roll of incoming.rolls) {
                const old = next.rolls.find(r => r.id === roll.id);
                if (old && JSON.stringify(old) !== JSON.stringify(roll)) throw new Error(`Roll ${roll.id} already has different inspection data. Use inspected Rolls register from Floor to keep those measurements.`);
                if (!old) next.rolls.push(structuredClone(roll));
            }
            const previous = register; register = next;
            if (!persist()) { register = previous; throw new Error('The received stock could not be saved. Free browser storage, then try again.'); }
            inspection = undefined; spread = undefined; renderRolls();
        },
        desktopSnapshot: () => ({ document: doc, register, bundles, ledger, bundleSource } satisfies Saved),
        desktopSaved: () => { patternSaved = true; renderPattern(); },
        desktopRestore: (value: unknown) => {
            const saved = value as Saved;
            const restored = readPattern(JSON.stringify(saved.document));
            projectGeneration++; doc = restored;
            register = Array.isArray(saved.register?.rolls) ? saved.register : { rolls: [] };
            bundles = Array.isArray(saved.bundles) ? saved.bundles : [];
            ledger = Array.isArray(saved.ledger) ? saved.ledger : [];
            bundleSource = saved.bundleSource;
            patternReady = false; patternSaved = false;
            renderPattern(); renderRolls(); renderBundles();
            void acceptPattern(doc).catch(error => { $('#pattern-status').textContent = String(error); });
        },
    };
}
