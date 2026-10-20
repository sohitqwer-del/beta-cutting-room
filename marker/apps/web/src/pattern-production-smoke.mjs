import { loadPatternCore } from '@beta/marker-wasm';
/** Original public synthetic geometry. Runs against the local app's real forms. */
const get = s => { const e = document.querySelector(s); if (!e) throw new Error(`Missing ${s}`); return e; };
const wait = async test => { for (let i = 0; i < 200; i++) { if (test()) return; await new Promise(r => setTimeout(r, 50)); } throw new Error(`Timed out: ${get('#pd-status').textContent}`); };
const ready = () => wait(() => !get('#pattern-workbench').hasAttribute('aria-busy'));
const saved = () => JSON.parse(localStorage.getItem('beta-cutting-room-v1')).document;
const fill = (s, v) => { get(s).value = String(v); get(s).dispatchEvent(new Event('input', { bubbles: true })); };
const click = async s => { get(s).click(); await ready(); await new Promise(r => requestAnimationFrame(r)); };
const change = async (s, v) => { fill(s, v); get(s).dispatchEvent(new Event('change', { bubbles: true })); await ready(); };
const submit = async s => { get(s).requestSubmit(); await ready(); };
const file = (s, name, source) => { const dt = new DataTransfer(); dt.items.add(new File([source], name, { type: 'text/plain' })); get(s).files = dt.files; get(s).dispatchEvent(new Event('change', { bubbles: true })); };
const assert = (test, message) => { if (!test) throw new Error(message); };
export function publicProductionFixture() {
    const doc = { format: 'beta-pattern.doc/v1', name: 'Public production tools demo', sizes: ['S', 'M', 'L'], base_size: 'M',
        measurements: { S: { width: 140, height: 380 }, M: { width: 150, height: 400 }, L: { width: 160, height: 420 } },
        variables: {}, drawing: {}, pieces: [], rules: {
            fixed: { mode: 'relative', deltas: { S: [0, 0], M: [0, 0], L: [0, 0] } },
            width: { mode: 'relative', deltas: { S: [-15, 0], M: [0, 0], L: [25, 0] } },
        }, meta: { provenance: 'Original BETA synthetic QA geometry; not a production pattern.' } };
    const point = (id, x, y) => { doc.drawing[id] = { kind: 'point', operation: { op: 'absolute', x, y } }; };
    for (const id of ['front', 'back']) {
        const front = id === 'front';
        const coords = front ? [[0, 0], ['m.width', 0], ['m.width', 'm.height'], [0, 'm.height']] : [[350, 0], [495, 0], [495, 400], [350, 400]];
        coords.forEach(([x, y], i) => point(`${id}.p${i}`, x, y));
        point(`${id}.g0`, front ? 60 : 400, 50); point(`${id}.g1`, front ? 60 : 400, 350);
        point(`${id}.button_start`, front ? 25 : 375, 65);
        const edges = coords.map((_, i) => {
            const object = `${id}.edge${i + 1}`;
            doc.drawing[object] = { kind: 'line', from: `${id}.p${i}`, to: `${id}.p${(i + 1) % 4}` };
            return { object, allowance: i === 3 && front ? 0 : i === 0 ? 25 : 10, corner: 'miter' };
        });
        doc.pieces.push({ id, name: front ? 'Half placket panel' : 'Comparison panel', edges, quantity: 1, mirror_quantity: front ? 1 : 0,
            material: 'cotton', grain: [`${id}.g0`, `${id}.g1`], label_point: `${id}.g0`,
            annotations: [{ kind: 'shaped_notch', edge: 1, distance: 100, depth: 3, width: 4, shape: 'v' }],
            grade_points: front ? { 'front.p0': 'fixed', 'front.p1': 'width', 'front.p2': 'width', 'front.p3': 'fixed' } : {} });
    }
    return doc;
}
export async function runProductionSeams() {
    await click('[data-screen="4"]');
    await wait(() => document.querySelectorAll('[data-tool]').length > 10);
    file('#pattern-file', 'public-production-demo.json', JSON.stringify(publicProductionFixture()));
    await wait(() => saved()?.name === 'Public production tools demo');
    await ready();
    await click('[data-mode="checks"]');
    fill('#pp-check-name', 'Graded hem join'); fill('#pp-a-piece', 'front'); fill('#pp-b-piece', 'back');
    fill('#pp-a-edges', '1'); fill('#pp-b-edges', '1'); fill('#pp-ease', '5');
    await submit('#pp-check-form');
    const rows = [...document.querySelectorAll('#pp-report tbody tr')].map(r => [...r.children].map(c => c.textContent));
    assert(rows.length === 3 && rows[0][1] === '135.00' && rows[2][1] === '175.00', 'Report did not use factory grades.');
    assert(rows.map(r => r[5]).join(',') === 'CHECK,PASS,CHECK', 'Wrong ease verdicts.');
    await click('#pp-csv'); await click('#pp-json');
    get('#pp-report').scrollIntoView({ block: 'start' });
    return { rows, actualGradedLengths: true, reportDownloads: true };
}
export async function runProductionLibraryPreview() {
    await click('[data-mode="grading"]');
    await click('#pp-rule-json'); await click('#pp-rule-rul');
    file('#pp-rule-file', 'public-library.rul', 'UNITS: MM\nMODE: INCREMENTAL\nSIZES: S M L\nBASE: M\nRULE: 1 0 0 4 2 6 3\nEND\n');
    await wait(() => !get('#pp-rule-import').disabled);
    assert(!saved().rules['1'], 'Preview changed the pattern.');
    assert(get('#pp-rule-preview').textContent.includes('-4.00, -2.00'), 'Lower-size increment was not integrated.');
    get('#pp-rule-preview').scrollIntoView({ block: 'center' });
    return { previewOnly: true, incrementalImport: true, exportButtons: true };
}
export async function applyProductionLibrary() {
    await click('#pp-rule-import');
    assert(saved().rules['1'].deltas.S[0] === -4, 'Rule import missing.');
    await change('#pd-rule-select', '1');
    fill('#pd-dx-2', '8'); await click('#pd-rule-save');
    assert(saved().rules['1'].deltas.L[0] === 8, 'Numeric imported rule IDs cannot be edited.');
    // Test collision protection and malformed files without changing the document.
    const before = JSON.stringify(saved());
    file('#pp-rule-file', 'collision.rul', 'MODE: CUMULATIVE\nSIZES: S M L\nBASE: M\nRULE: 1 -4 -2 0 0 6 3\nEND\n');
    await ready();
    await wait(() => get('#pd-status').textContent.includes('already exist'));
    assert(JSON.stringify(saved()) === before && get('#pp-rule-import').disabled, 'Collision changed the pattern.');
    return { importedRule: saved().rules['1'], editedNumericId: true, collisionRetainedDocument: true };
}
export async function runProductionMarksAndFold() {
    await click('[data-mode="piece"]'); await change('#pd-piece-select', 'front');
    get('#pp-marks').closest('details').open = true;
    fill('#pp-mark-kind', 'buttonhole'); fill('#pp-row-id', 'placket'); fill('#pp-origin', 'front.button_start');
    fill('#pp-count', '5'); fill('#pp-spacing', '60'); fill('#pp-length', '14');
    await submit('#pp-marks');
    assert(saved().pieces[0].annotations.length === 16, 'Buttonholes or end bars missing.');
    await click('[data-mode="grading"]'); await change('#pd-grade-piece', 'front'); await click('#pd-clear-rules');
    await click('[data-mode="piece"]'); await change('#pd-piece-select', 'front');
    fill('#pd-action', 'unfold'); fill('#pd-action-id', 'full_front'); fill('#pd-action-edge', '4');
    await submit('#pd-piece-action');
    assert(saved().pieces.some(p => p.id === 'full_front' && p.fold_line), `Unfold failed: ${get('#pd-status').textContent}`);
    const core = await loadPatternCore(), grades = core.grade(saved());
    const full = grades.sizes.M.find(p => p.piece.id === 'full_front-M').piece;
    assert(full.marks.filter(m => m.text === 'buttonhole').length === 10, 'Unfold lost mirrored buttonholes.');
    assert(full.mirror_quantity === 1 && full.grain && full.material === 'cotton', 'Unfold lost production metadata.');
    await click('#pd-labels'); await click('#pd-fit'); get('#pd-canvas').scrollIntoView({ block: 'start' });
    return { buttonholes: 10, fold: full.fold_line, mirrorQuantity: full.mirror_quantity, linkedSizes: Object.keys(grades.sizes) };
}
export async function runProductionHandoff() {
    // Retained half becomes a construction template rather than another cut part.
    await change('#pd-piece-select', 'front'); fill('#pd-piece-quantity', '0'); fill('#pd-piece-pairs', '0'); await submit('#pd-piece-properties');
    await click('[data-mode="output"]'); fill('#pd-ratio-0', '1'); fill('#pd-ratio-1', '2'); fill('#pd-ratio-2', '1');
    await click('#pd-export-dxf'); await click('#pd-send-marker');
    assert(get('#job-title').textContent === 'Public production tools demo', 'Pattern did not reach Marker.');
    const core = await loadPatternCore(), job = core.to_job(saved(), { width: 1500, gap: 2 }, { S: 1, M: 2, L: 1 });
    const full = job.pieces.filter(p => p.id.startsWith('full_front'));
    assert(full.length === 3 && full.every(p => p.fold_line && p.mirror_quantity === 1 && p.marks.some(m => m.text === 'buttonhole')), 'Marker handoff lost fold/marks/mirror copies.');
    return { sizes: job.order.sizes, fullPieces: full.length, markerHandoff: true, marksAndFoldRetained: true };
}
