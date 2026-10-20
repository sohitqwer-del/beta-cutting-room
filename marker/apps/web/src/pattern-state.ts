import type { PatternDocument, PatternValue, PatternEdge } from '@beta/marker-wasm';
/** Bounded immutable snapshots; failed edits never enter history. */
export class PatternHistory {
    private past: PatternDocument[] = [];
    private future: PatternDocument[] = [];
    constructor(private current: PatternDocument) { this.current = structuredClone(current); }
    get value(): PatternDocument { return structuredClone(this.current); }
    get canUndo(): boolean { return this.past.length > 0; }
    get canRedo(): boolean { return this.future.length > 0; }
    apply(next: PatternDocument): void {
        if (JSON.stringify(next) === JSON.stringify(this.current))
            return;
        this.past.push(this.current);
        if (this.past.length > 100)
            this.past.shift();
        this.current = structuredClone(next);
        this.future = [];
    }
    undo(): PatternDocument { const d = this.past.pop(); if (d) {
        this.future.push(this.current);
        this.current = d;
    } return this.value; }
    redo(): PatternDocument { const d = this.future.pop(); if (d) {
        this.past.push(this.current);
        this.current = d;
    } return this.value; }
}
export function value(text: string): PatternValue {
    if (!text.trim())
        throw new Error('Enter a number or formula.');
    return Number.isFinite(Number(text)) ? Number(text) : text.trim();
}
/** Complete only the active m./v. token, keeping surrounding arithmetic. */
export function formulaOptions(doc: PatternDocument, input: string, caret = input.length): string[] {
    const token = /(?:m|v)\.[A-Za-z0-9_]*$/.exec(input.slice(0, caret))?.[0] ?? '';
    const names = [...new Set(Object.values(doc.measurements).flatMap(r => Object.keys(r)))].map(n => `m.${n}`).concat(Object.keys(doc.variables).map(n => `v.${n}`));
    return names.filter(n => !token || n.startsWith(token)).sort();
}
export function insertFormula(input: string, caret: number, name: string): {
    text: string;
    caret: number;
} {
    const before = input.slice(0, caret).replace(/(?:m|v)\.[A-Za-z0-9_]*$/, '');
    return { text: before + name + input.slice(caret), caret: before.length + name.length };
}
export function constructionOrder(doc: PatternDocument): string[] {
    const saved = doc.meta?.construction_order;
    const order = Array.isArray(saved) ? saved.filter((id): id is string => typeof id === 'string' && Object.hasOwn(doc.drawing, id)) : [];
    return [...new Set([...order, ...Object.keys(doc.drawing)])];
}
export function rememberObject(doc: PatternDocument, id: string): void {
    doc.meta = { ...doc.meta, construction_order: [...constructionOrder(doc), id].filter((n, i, a) => a.indexOf(n) === i) };
}
/** Find exact ID references in model fields and safe formula tokens. */
function refers(input: unknown, id: string): boolean {
    if (typeof input === 'string')
        return input === id || Array.from(input.match(/[A-Za-z_][A-Za-z0-9_.]*/g) ?? []).includes(id);
    if (Array.isArray(input))
        return input.some(v => refers(v, id));
    if (input && typeof input === 'object')
        return Object.entries(input).filter(([k]) => !['kind', 'op', 'mark', 'text', 'name', 'material', 'shape', 'corner'].includes(k)).some(([k, v]) => (k === 'grade_points' && Object.hasOwn(v as object, id)) || refers(v, id));
    return false;
}
export function dependencyDelete(doc: PatternDocument, id: string): {
    objects: string[];
    pieces: string[];
    variables: string[];
} {
    const objects = new Set([id]), variables = new Set<string>();
    let changed = true;
    while (changed) {
        changed = false;
        for (const [key, entry] of Object.entries(doc.drawing))
            if (!objects.has(key) && ([...objects].some(n => refers(entry, n)) || [...variables].some(n => refers(entry, `v.${n}`)))) {
                objects.add(key);
                changed = true;
            }
        for (const [key, entry] of Object.entries(doc.variables))
            if (!variables.has(key) && ([...objects].some(n => refers(entry, n)) || [...variables].some(n => refers(entry, `v.${n}`)))) {
                variables.add(key);
                changed = true;
            }
    }
    return { objects: [...objects], variables: [...variables], pieces: doc.pieces.filter(p => [...objects].some(n => refers({ ...p, id: undefined, name: undefined }, n)) || [...variables].some(n => refers(p, `v.${n}`))).map(p => p.id) };
}
export function deleteConstruction(doc: PatternDocument, id: string): PatternDocument {
    const next = structuredClone(doc), deps = dependencyDelete(doc, id);
    deps.objects.forEach(n => delete next.drawing[n]);
    deps.variables.forEach(n => delete next.variables[n]);
    for (const group of Object.values(next.groups ?? {}))
        group.objects = group.objects.filter(n => !deps.objects.includes(n));
    next.pieces = next.pieces.filter(p => !deps.pieces.includes(p.id));
    next.meta = { ...next.meta, construction_order: constructionOrder(next) };
    return next;
}
export function appendPieceEdge(edges: PatternEdge[], object: string, reversed = false): PatternEdge[] {
    if (edges.some(e => e.object === object))
        throw new Error('This path is already in the piece.');
    return [...edges, { object, reversed, allowance: 10, corner: 'miter' }];
}
export function blankPattern(): PatternDocument {
    return { format: 'beta-pattern.doc/v1', name: 'Untitled pattern', sizes: ['M', 'L'], base_size: 'M', measurements: { M: { waist: 720, hip: 960, length: 600 }, L: { waist: 760, hip: 1000, length: 610 } }, variables: { ease: 40 }, drawing: {}, pieces: [], rules: {}, meta: { construction_order: [] } };
}
