import type { PatternDocument, Json, Pt } from '@beta/marker-wasm';

interface SeamRow {
    size: string; a_mm: number; b_mm: number; difference_mm: number; error_mm: number;
    pass: boolean; a_path: Pt[]; b_path: Pt[];
}
interface SeamReport {
    check: { name: string; ease_mm: number; tolerance_mm: number;
        a: {piece: string; edges: number[]; reversed: boolean};
        b: {piece: string; edges: number[]; reversed: boolean};
    };
    sampling_mm: number; rows: SeamRow[];
}
interface Library {
    name: string; sizes: string[]; base_size: string;
    rules: Record<string, { mode: string; deltas: Record<string, Pt> }>;
}
export interface ProductionResult {
    document: PatternDocument; message?: string; report?: SeamReport; csv?: string;
    source?: string; library?: Library; warnings?: string[];
}
interface Host {
    get(): PatternDocument;
    action(request: Record<string, Json>): Promise<ProductionResult>;
    commit(doc: PatternDocument): Promise<void>;
    run(action: () => void | Promise<void>): Promise<void>;
    status(message: string, error?: boolean): void;
    download(data: string, name: string, mime?: string): void;
}
const esc = (v: unknown) => String(v ?? '').replace(/[&<>"']/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' }[c]!));
/** Convert displayed edge numbers to the engine's zero-based IDs, without geometry work. */
export function seamEdges(text: string): number[] {
    if (!/^\s*[1-9]\d*(?:\s*,\s*[1-9]\d*)*\s*$/.test(text))
        throw new Error('Enter edge numbers separated by commas, for example 1,2,3.');
    const edges = text.split(',').map(s => Number(s.trim()) - 1);
    if (edges.some(n => !Number.isSafeInteger(n)) || new Set(edges).size !== edges.length)
        throw new Error('Use whole edge numbers without repeats.');
    return edges;
}
/** Production UI. Rust owns marks, seam lengths, alignment, grading and file parsing. */
export function productionTools(root: HTMLElement, host: Host) {
    let fingerprint = '', report: SeamReport | undefined, csv = '';
    let pending: { source: string; format: string; replace: boolean; document: string } | undefined;
    const $ = <T extends Element = HTMLElement>(id: string) => root.querySelector<T>(id)!;
    const text = (id: string) => $<HTMLInputElement>(id).value;
    const checked = (id: string) => $<HTMLInputElement>(id).checked;
    const number = (id: string) => {
        const el = $<HTMLInputElement>(id), n = Number(el.value);
        if (!el.value.trim() || !el.checkValidity() || !Number.isFinite(n)) throw new Error(`Check ${el.labels?.[0]?.textContent ?? id}.`);
        return n;
    };
    const options = (values: string[], selected = '') => values.map(s => `<option value="${esc(s)}" ${s === selected ? 'selected' : ''}>${esc(s)}</option>`).join('');
    function sync() {
        const next = JSON.stringify(host.get());
        if (next !== fingerprint) { report = undefined; csv = ''; pending = undefined; fingerprint = next; }
    }
    function checks(): string {
        sync();
        const doc = host.get(), pieces = doc.pieces.map(p => p.id);
        return `<h2>Check sewing seams</h2><p>Compare stitch edges in every size, after grade rules. Seam allowances are excluded. Positive ease means seam A should be longer than seam B.</p>
        <form id="pp-check-form"><div class="fields">
        <label>Assembly name<input id="pp-check-name" value="Side seam" required></label>
        <label>Seam A piece<select id="pp-a-piece">${options(pieces)}</select></label>
        <label>Seam A edges<input id="pp-a-edges" value="1" required aria-describedby="pp-edge-help"></label>
        <label class="check"><input type="checkbox" id="pp-a-reverse">Reverse A for preview</label>
        <label>Seam B piece<select id="pp-b-piece">${options(pieces, pieces[1])}</select></label>
        <label>Seam B edges<input id="pp-b-edges" value="1" required aria-describedby="pp-edge-help"></label>
        <label class="check"><input type="checkbox" id="pp-b-reverse">Reverse B for preview</label>
        <label>Required ease A minus B (mm)<input id="pp-ease" type="number" step="any" value="0" required></label>
        <label>Allowed error (mm)<input id="pp-tolerance" type="number" min="0" step="any" value="1" required></label></div>
        <p id="pp-edge-help">Use connected edge numbers in piece order, separated by commas. For example 3,4,1 can cross the end of a piece's edge list.</p>
        <div id="pp-edge-key" class="pp-edge-key"></div>
        <button type="submit" class="primary" ${pieces.length ? '' : 'disabled'}>Check all sizes</button></form>
        <div id="pp-report" aria-live="polite">${report ? reportHtml(report) : '<p>No report yet. Choose the seams to compare.</p>'}</div>`;
    }
    function reportHtml(r: SeamReport): string {
        return `<h3>${esc(r.check.name)}</h3><p>A: ${esc(r.check.a.piece)}, edges ${r.check.a.edges.map(i => i + 1).join(', ')}. B: ${esc(r.check.b.piece)}, edges ${r.check.b.edges.map(i => i + 1).join(', ')}.</p><p>Required ease: ${r.check.ease_mm} mm · Allowed error: ±${r.check.tolerance_mm} mm · Curve sampling: ${r.sampling_mm} mm</p>
        <div class="table-scroll"><table><caption>Graded stitch lengths in mm</caption><thead><tr><th>Size</th><th>A</th><th>B</th><th>A − B</th><th>Error from ease</th><th>Result</th></tr></thead><tbody>
        ${r.rows.map(row => `<tr><th>${esc(row.size)}</th><td>${row.a_mm.toFixed(2)}</td><td>${row.b_mm.toFixed(2)}</td><td>${row.difference_mm.toFixed(2)}</td><td>${row.error_mm.toFixed(2)}</td><td class="${row.pass ? 'pd-ok' : 'error-text'}">${row.pass ? 'PASS' : 'CHECK'}</td></tr>`).join('')}</tbody></table></div>
        <div class="pd-actions"><button id="pp-csv">Save seam report CSV</button><button id="pp-json">Save seam report JSON</button>
        <label>Alignment preview size<select id="pp-preview-size">${options(r.rows.map(row => row.size))}</select></label></div>
        <p>Start points and first tangents align. A is solid; B is dashed. Neither seam is stretched. This is a length and alignment check, not a sewn-fit simulation.</p><div id="pp-seam-preview"></div>`;
    }
    function preview() {
        if (!report) return;
        const r = report.rows.find(row => row.size === text('#pp-preview-size'))!;
        const points = [...r.a_path, ...r.b_path], xs = points.map(p => p[0]), ys = points.map(p => -p[1]);
        const pad = Math.max(20, Math.max(...xs) - Math.min(...xs), Math.max(...ys) - Math.min(...ys)) * .1;
        const path = (p: Pt[]) => p.map(([x, y]) => `${x},${-y}`).join(' ');
        $('#pp-seam-preview').innerHTML = `<svg viewBox="${Math.min(...xs) - pad} ${Math.min(...ys) - pad} ${Math.max(...xs) - Math.min(...xs) + 2 * pad} ${Math.max(...ys) - Math.min(...ys) + 2 * pad}" role="img" aria-label="${esc(r.size)} aligned seam preview. A ${r.a_mm.toFixed(2)} mm; B ${r.b_mm.toFixed(2)} mm.">
        <polyline points="${path(r.a_path)}" fill="none" style="stroke:var(--size-0)" stroke-width="3" vector-effect="non-scaling-stroke"/>
        <polyline points="${path(r.b_path)}" fill="none" style="stroke:var(--size-1)" stroke-dasharray="7 5" stroke-width="3" vector-effect="non-scaling-stroke"/>
        <circle r="${pad * .07}" style="fill:var(--text)"/></svg>`;
    }
    function reportBindings() {
        if (!report) return;
        $('#pp-csv').onclick = () => host.download(csv, 'seam-check.csv', 'text/csv');
        $('#pp-json').onclick = () => host.download(JSON.stringify(report, null, 2), 'seam-check.json', 'application/json');
        $('#pp-preview-size').onchange = preview;
        preview();
    }
    function rules(): string {
        sync();
        return `<section class="pp-panel"><h3>Reuse grade rules</h3><p>Save rules as a JSON library or BETA's limited ASCII RUL interchange. RUL uses cumulative millimetres. Point attachments stay in pattern JSON; importing a library does not attach rules automatically. Compatibility with every vendor RUL dialect is not established.</p>
        <div class="pd-actions"><button id="pp-rule-json">Save rule library JSON</button><button id="pp-rule-rul">Save rule table RUL</button></div>
        <label>Rule library JSON or RUL<input id="pp-rule-file" type="file" accept=".json,.rul"></label>
        <label class="check"><input id="pp-rule-replace" type="checkbox">Replace matching rule IDs</label>
        <p id="pp-rule-preview-status" role="status"></p><div id="pp-rule-preview"></div>
        <button id="pp-rule-import" disabled>Apply reviewed rules</button></section>`;
    }
    function marks(piece: string): string {
        const doc = host.get();
        // Transformed points are valid references too; the engine validates the choice.
        const points = Object.keys(doc.drawing).filter(k => doc.drawing[k]?.kind === 'point');
        return `<details class="pp-panel"><summary>Button, buttonhole or drill row</summary><p>Add equally spaced centres. Buttonholes export as labelled internal lines with end bars. All positions stay linked to the first point.</p>
        <form id="pp-marks" data-piece="${esc(piece)}"><div class="fields">
        <label>Mark type<select id="pp-mark-kind">${options(['button', 'buttonhole', 'drill'])}</select></label>
        <label>Row ID<input id="pp-row-id" value="row_${doc.pieces.find(p => p.id === piece)?.annotations?.length ?? 0}" required></label>
        <label>First centre<select id="pp-origin">${options(points)}</select></label>
        <label>Count<input id="pp-count" type="number" value="5" min="1" max="100" step="1" required></label>
        <label>Spacing (mm)<input id="pp-spacing" type="number" value="60" min=".01" step="any" required></label>
        <label>Row angle (degrees)<input id="pp-direction" type="number" value="90" step="any" required></label>
        <label>Button diameter / hole length (mm)<input id="pp-length" type="number" value="15" min=".01" step="any" required></label>
        <label>Buttonhole angle (degrees)<input id="pp-angle" type="number" value="0" step="any" required></label>
        </div><button type="submit">Add production marks</button></form></details>`;
    }
    function bind() {
        const checkForm = root.querySelector<HTMLFormElement>('#pp-check-form');
        if (checkForm) {
            if (report) {
                const c = report.check;
                for (const [id, value] of Object.entries({
                    'pp-check-name': c.name, 'pp-a-piece': c.a.piece, 'pp-a-edges': c.a.edges.map(i => i + 1).join(','),
                    'pp-b-piece': c.b.piece, 'pp-b-edges': c.b.edges.map(i => i + 1).join(','),
                    'pp-ease': c.ease_mm, 'pp-tolerance': c.tolerance_mm,
                })) $<HTMLInputElement>(`#${id}`).value = String(value);
                $<HTMLInputElement>('#pp-a-reverse').checked = c.a.reversed;
                $<HTMLInputElement>('#pp-b-reverse').checked = c.b.reversed;
            }
            const invalidate = () => { report = undefined; csv = ''; $('#pp-report').innerHTML = '<p>Inputs changed. Check all sizes to update the report.</p>'; };
            checkForm.addEventListener('input', invalidate);
            checkForm.addEventListener('change', invalidate);
            const key = () => {
                $('#pp-edge-key').innerHTML = ['a', 'b'].map(side => {
                    const p = host.get().pieces.find(p => p.id === text(`#pp-${side}-piece`));
                    return `<div><strong>${side.toUpperCase()}: ${esc(p?.name)}</strong><ol>${p?.edges.map(e => `<li>${esc(e.object)}${e.reversed ? ' (reverse)' : ''}</li>`).join('') ?? ''}</ol></div>`;
                }).join('');
            };
            $('#pp-a-piece').onchange = key; $('#pp-b-piece').onchange = key; key();
            checkForm.onsubmit = e => { e.preventDefault(); void host.run(async () => {
                const run = (side: string) => ({ piece: text(`#pp-${side}-piece`), edges: seamEdges(text(`#pp-${side}-edges`)), reversed: checked(`#pp-${side}-reverse`) });
                const result = await host.action({ action: 'seam_report', check: { name: text('#pp-check-name'), a: run('a'), b: run('b'), ease_mm: number('#pp-ease'), tolerance_mm: number('#pp-tolerance') } });
                if (!result.report || result.csv === undefined) throw new Error('Rust did not return a seam report.');
                report = result.report; csv = result.csv;
                $('#pp-report').innerHTML = reportHtml(report); reportBindings();
                host.status(`Checked ${report.rows.length} sizes. ${report.rows.filter(r => !r.pass).length} need review.`);
            }); };
            reportBindings();
        }
        const markForm = root.querySelector<HTMLFormElement>('#pp-marks');
        if (markForm) markForm.onsubmit = e => { e.preventDefault(); void host.run(async () => {
            const result = await host.action({ action: 'mark_row', row: { piece: markForm.dataset.piece!, id: text('#pp-row-id'), origin: text('#pp-origin'), count: number('#pp-count'), spacing: number('#pp-spacing'), direction: number('#pp-direction'), kind: text('#pp-mark-kind'), length: number('#pp-length'), angle: number('#pp-angle') } });
            await host.commit(result.document); host.status(result.message ?? 'Production marks added.');
        }); };
        for (const format of ['json', 'rul']) {
            const button = root.querySelector<HTMLButtonElement>(`#pp-rule-${format}`);
            if (button) button.onclick = () => void host.run(async () => {
                const result = await host.action({ action: 'rule_export', format, name: host.get().name });
                if (!result.source) throw new Error('No grade rules returned.');
                host.download(result.source, `grade-library.${format}`, format === 'json' ? 'application/json' : 'text/plain');
                host.status(`Grade rule library saved as ${format.toUpperCase()}.`);
            });
        }
        const file = root.querySelector<HTMLInputElement>('#pp-rule-file');
        if (file) {
            const load = async () => {
                pending = undefined; $<HTMLButtonElement>('#pp-rule-import').disabled = true; $('#pp-rule-preview').innerHTML = '';
                const f = file.files?.[0]; if (!f) return;
                if (f.size > 8 * 1024 * 1024) throw new Error('Rule library exceeds 8 MB.');
                const source = await f.text(), format = f.name.toLowerCase().endsWith('.rul') ? 'rul' : 'json', replace = checked('#pp-rule-replace');
                const result = await host.action({ action: 'rule_preview', source, format, replace });
                if (!result.library) throw new Error('Rust did not return a rule library.');
                pending = { source, format, replace, document: JSON.stringify(host.get()) };
                const library = result.library;
                $('#pp-rule-preview-status').textContent = `${Object.keys(library.rules).length} rule${Object.keys(library.rules).length === 1 ? '' : 's'}. Base size ${library.base_size}. Review before applying.`;
                $('#pp-rule-preview').innerHTML = `${(result.warnings ?? []).map(w => `<p>${esc(w)}</p>`).join('')}<div class="table-scroll"><table><caption>${esc(library.name)} — x, y in mm</caption><thead><tr><th>Rule</th><th>Mode</th>${library.sizes.map(s => `<th>${esc(s)}</th>`).join('')}</tr></thead><tbody>${Object.entries(library.rules).map(([id, r]) => `<tr><th>${esc(id)}</th><td>${esc(r.mode)}</td>${library.sizes.map(s => `<td>${r.deltas[s]?.map(n => n.toFixed(2)).join(', ') ?? 'unused'}</td>`).join('')}</tr>`).join('')}</tbody></table></div>`;
                $<HTMLButtonElement>('#pp-rule-import').disabled = false;
            };
            file.onchange = () => void host.run(load);
            $('#pp-rule-replace').onchange = () => void host.run(load);
            $('#pp-rule-import').onclick = () => void host.run(async () => {
                if (!pending || pending.document !== JSON.stringify(host.get())) throw new Error('The pattern changed. Select the library again to review it.');
                const result = await host.action({ action: 'rule_import', source: pending.source, format: pending.format, replace: pending.replace });
                await host.commit(result.document); pending = undefined;
                host.status('Rules imported and checked in all sizes. Attach them to outline points in Grading.');
            });
        }
    }
    return { checks, rules, marks, bind };
}
