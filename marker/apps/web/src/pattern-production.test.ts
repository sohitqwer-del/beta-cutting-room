import { describe, expect, it } from 'vitest';
import { seamEdges } from './pattern-production';
describe('production seam selection', () => {
    it('keeps connected edge order including a wrapped run', () => {
        expect(seamEdges(' 3, 4, 1 ')).toEqual([2, 3, 0]);
        expect(seamEdges('2')).toEqual([1]);
    });
    it('rejects ambiguous or unsafe edge input before invoking Rust', () => {
        for (const value of ['', '0', '-1', '1.5', '1,1', '1,,2', '1e3', '999999999999999999999']) {
            expect(() => seamEdges(value)).toThrow();
        }
    });
});
