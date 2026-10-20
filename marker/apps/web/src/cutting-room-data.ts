import type { Job, PatternDocument, PhysicalLay, Roll, RollRegister } from '@beta/marker-wasm';

/** Restore the physical identity of an audited lay when it is selected again. */
export function spreadBundleFields(lay: PhysicalLay): { id: string; plies: number; runs: string } {
    const csv = (value: string) => '"' + value.replace(/"/g, '""') + '"';
    return { id: lay.id, plies: lay.plies.at(-1)!.last,
        runs: lay.plies.map(run => `${run.first},${run.last},${csv(run.shade_group)}`).join('\n') };
}

/** Include the buyer policy and grouping controls in the inspection identity. */
export function inspectionRequest(register: RollRegister, limit: number, unit: string, bandStep: number) {
    return structuredClone({ register, settings: { limit, unit, point_cap: 'per_yard' as const }, grouping: { band_step_pct: bandStep } });
}
/** Decode standard quoted CSV, including commas, escaped quotes and CRLF. */
export function csvRows(text: string): string[][] {
    const rows: string[][] = [];
    let row: string[] = [], cell = '', quoted = false, closed = false;
    const finish = () => { row.push(cell); cell = ''; closed = false; };
    for (let i = 0; i < text.length; i++) {
        const c = text[i]!;
        if (quoted) {
            if (c === '"' && text[i + 1] === '"') {
                cell += '"';
                i++;
            }
            else if (c === '"') {
                quoted = false;
                closed = true;
            }
            else
                cell += c;
        }
        else if (c === '"' && cell === '' && !closed)
            quoted = true;
        else if (c === ',')
            finish();
        else if (c === '\r' || c === '\n') {
            if (c === '\r' && text[i + 1] === '\n')
                i++;
            finish();
            if (row.some(v => v.trim()))
                rows.push(row);
            row = [];
        }
        else if (closed || c === '"')
            throw new Error('Invalid CSV quoting.');
        else
            cell += c;
    }
    if (quoted)
        throw new Error('CSV has an unclosed quote.');
    finish();
    if (row.some(v => v.trim()))
        rows.push(row);
    return rows;
}
export const rollColumns = ['id', 'lot', 'dye_lot', 'shade_group', 'ticket_length_m', 'measured_length_m', 'usable_width_mm', 'length_pct', 'width_pct'] as const;
/** Parse data entry only. Rust validates stock, calculates inspection and groups rolls. */
export function readRollCsv(text: string): RollRegister {
    const [header, ...rows] = csvRows(text.replace(/^\uFEFF/, ''));
    if (!header || rollColumns.some(k => !header.includes(k)))
        throw new Error(`CSV needs columns: ${rollColumns.join(', ')}`);
    if (new Set(header).size !== header.length)
        throw new Error('CSV column names must be unique.');
    const rolls: Roll[] = rows.map((row, i) => {
        if (row.length !== header.length)
            throw new Error(`CSV row ${i + 2} has the wrong number of cells.`);
        const value = (key: string) => row[header.indexOf(key)]!.trim();
        const numeric = (key: string) => {
            if (!value(key) || !Number.isFinite(Number(value(key))))
                throw new Error(`CSV row ${i + 2}: enter a number for ${key}.`);
            return Number(value(key));
        };
        return { id: value('id'), lot: value('lot'), dye_lot: value('dye_lot'), shade_group: value('shade_group'), ticket_length_m: numeric('ticket_length_m'), measured_length_m: numeric('measured_length_m'), usable_width_mm: numeric('usable_width_mm'), shrinkage: { length_pct: numeric('length_pct'), width_pct: numeric('width_pct') }, defects: [] };
    });
    if (!rolls.length)
        throw new Error('Add at least one roll.');
    if (new Set(rolls.map(r => r.id)).size !== rolls.length)
        throw new Error('Roll ids must be unique.');
    return { rolls };
}
/** Check the outer shape before rendering. Rust validates formulas and geometry. */
export function readPattern(text: string): PatternDocument {
    const d = JSON.parse(text) as PatternDocument;
    if (!d || d.format !== 'beta-pattern.doc/v1' || typeof d.name !== 'string' || !Array.isArray(d.sizes) || !d.sizes.length || d.sizes.length > 100 || !d.sizes.every(s => typeof s === 'string') || typeof d.base_size !== 'string' || !Array.isArray(d.pieces) || d.pieces.some(p => !p || typeof p.id !== 'string' || typeof p.name !== 'string' || !Array.isArray(p.edges)))
        throw new Error('Choose a beta-pattern.doc/v1 JSON document.');
    for (const key of ['measurements', 'variables', 'drawing'] as const) {
        d[key] ??= {};
        if (typeof d[key] !== 'object' || Array.isArray(d[key]))
            throw new Error(`Pattern ${key} must be an object.`);
    }
    for (const row of Object.values(d.measurements)) {
        if (!row || typeof row !== 'object' || Object.values(row).some(v => typeof v !== 'number' || !Number.isFinite(v)))
            throw new Error('Measurements must be finite numbers.');
    }
    return d;
}
/** A digitized piece needs a reviewed size before it can join a size order. */
export function acceptedJob(pieces: Job['pieces'], size: string, fabric: Job['fabric']): Job {
    if (!pieces.length || !size.trim())
        throw new Error('Accept at least one piece and enter its size.');
    return { format: 'beta-marker.job/v1', name: 'Digitized paper patterns', fabric: structuredClone(fabric), pieces: pieces.map(p => ({ ...structuredClone(p), size: size.trim() })), meta: { source: 'Accepted digitizer overlay; check grain and marks before cutting' } };
}
