import { describe, expect, it } from 'vitest';
import { PatternHistory, appendPieceEdge, blankPattern, constructionOrder, deleteConstruction, dependencyDelete, formulaOptions, insertFormula, rememberObject, value } from './pattern-state';
describe('pattern drafting transactions', () => {
    it('undoes and redoes snapshots without leaking mutations and clears a redo branch', () => {
        const initial = blankPattern(), h = new PatternHistory(initial), next = h.value;
        next.name = 'First edit';
        h.apply(next);
        next.name = 'External mutation';
        expect(h.value.name).toBe('First edit');
        expect(h.undo().name).toBe('Untitled pattern');
        expect(h.redo().name).toBe('First edit');
        h.undo();
        h.apply({ ...initial, name: 'Branch' });
        expect(h.canRedo).toBe(false);
    });
    it('keeps construction order after JSON map keys are sorted', () => {
        const d = blankPattern();
        d.drawing.Z = { kind: 'point', operation: { op: 'absolute', x: 0, y: 0 } };
        rememberObject(d, 'Z');
        d.drawing.A = { kind: 'point', operation: { op: 'polar', from: 'Z', distance: 10, angle: 0 } };
        rememberObject(d, 'A');
        d.drawing = { A: d.drawing.A, Z: d.drawing.Z };
        expect(constructionOrder(d)).toEqual(['Z', 'A']);
    });
    it('completes only the m./v. token at the caret', () => {
        const d = blankPattern();
        expect(formulaOptions(d, '(m.wa')).toEqual(['m.waist']);
        expect(formulaOptions(d, 'v.e')).toEqual(['v.ease']);
        expect(insertFormula('(m.wa+10)/4', 5, 'm.waist')).toEqual({ text: '(m.waist+10)/4', caret: 8 });
        expect(value('0')).toBe(0);
        expect(() => value(' ')).toThrow();
    });
    it('builds an ordered directed piece path and rejects duplicate edges', () => {
        const a = appendPieceEdge([], 'waist'), b = appendPieceEdge(a, 'side', true);
        expect(a).toHaveLength(1);
        expect(b[1]).toEqual({ object: 'side', reversed: true, allowance: 10, corner: 'miter' });
        expect(() => appendPieceEdge(b, 'waist')).toThrow('already');
    });
    it('warns about transitive construction, formula-variable and piece dependencies', () => {
        const d = blankPattern();
        d.drawing.A = { kind: 'point', operation: { op: 'absolute', x: 0, y: 0 } };
        d.drawing.AA = { kind: 'point', operation: { op: 'absolute', x: 20, y: 20 } };
        d.drawing.B = { kind: 'point', operation: { op: 'polar', from: 'A', angle: 0, distance: 20 } };
        d.variables.span = 'distance(A,B)';
        d.drawing.C = { kind: 'point', operation: { op: 'absolute', x: 'v.span', y: 30 } };
        d.drawing.side = { kind: 'line', from: 'B', to: 'C' };
        d.pieces = [{ id: 'front', name: 'Front', edges: [{ object: 'side' }] }];
        expect(dependencyDelete(d, 'A')).toEqual({ objects: expect.arrayContaining(['A', 'B', 'C', 'side']), variables: ['span'], pieces: ['front'] });
        expect(dependencyDelete(d, 'A').objects).toHaveLength(4);
        const clean = deleteConstruction(d, 'A');
        expect(Object.keys(clean.drawing)).toEqual(['AA']);
        expect(clean.pieces).toEqual([]);
        expect(d.drawing.A).toBeDefined();
    });
    it('bounds undo history to 100 edits', () => { const h = new PatternHistory(blankPattern()); for (let i = 0; i < 110; i++)
        h.apply({ ...h.value, name: String(i) }); for (let i = 0; i < 100; i++)
        h.undo(); expect(h.canUndo).toBe(false); expect(h.value.name).toBe('9'); });
});
