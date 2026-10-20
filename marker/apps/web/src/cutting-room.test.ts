import { describe, expect, it } from 'vitest';
import { readDocument } from './state';
import { acceptedJob, csvRows, readPattern, readRollCsv, rollColumns } from './cutting-room-data';
import pattern from '../../../packages/marker-wasm/test/fixtures/pattern.doc.json';
import fixture from '../../../packages/marker-wasm/test/fixtures/cutting-room.json';
import type { Job, MarkerResult, BundleRequest, InspectRequest, LayActuals, MarkerRequest, PatternDocument, SpreadRequest } from '@beta/marker-wasm';
const marker: MarkerRequest = { job: readDocument(JSON.stringify(fixture.job)) as Job, result: readDocument(JSON.stringify(fixture.result)) as MarkerResult };
describe('cutting-room file entry and shared Rust contracts', () => {
    it('reads quoted CSV labels, escaped quotes, CRLF and embedded newlines', () => {
        expect(csvRows('a,b\r\n"one,two","said ""yes"""\r\n"line\nbreak",end')).toEqual([['a', 'b'], ['one,two', 'said "yes"'], ['line\nbreak', 'end']]);
        expect(() => csvRows('"unclosed')).toThrow('unclosed');
        expect(() => csvRows('"closed"junk')).toThrow('quoting');
    });
    it('parses a roll template and rejects bad or duplicate values', () => {
        const csv = rollColumns.join(',') + '\nR1,LOT1,DYE1,A,100,100,1500,0,0\n';
        expect(readRollCsv(csv).rolls[0]).toEqual({ ...fixture.register.rolls[0], defects: [] });
        expect(() => readRollCsv(csv + 'R1,LOT1,DYE1,A,100,100,1500,0,0')).toThrow('unique');
        expect(() => readRollCsv(csv.replace(',100,100,', ',no,100,'))).toThrow('number');
        expect(() => readRollCsv('id,length\nR1,100')).toThrow('columns');
    });
    it('keeps formulas, annotations and all sizes in the shared pattern', () => {
        const doc = readPattern(JSON.stringify(pattern));
        expect(doc.sizes).toEqual(['S', 'M', 'L', 'XL']);
        expect(doc.pieces).toHaveLength(2);
        expect(JSON.stringify(doc)).toContain('m.chest');
        expect(() => readPattern('{"format":"wrong"}')).toThrow('beta-pattern');
        const broken = structuredClone(doc);
        broken.measurements.M!.chest = 'invalid' as unknown as number;
        expect(() => readPattern(JSON.stringify(broken))).toThrow('finite');
    });
    it('accepts only reviewed pieces with an explicit size, without mutating the source', () => {
        const job = acceptedJob(marker.job.pieces, ' L ', { width: 1500 });
        expect(job.pieces[0]!.size).toBe('L');
        expect(fixture.job.pieces[0]!.size).toBe('M');
        expect(() => acceptedJob([], 'M', { width: 1500 })).toThrow('at least');
        expect(() => acceptedJob(marker.job.pieces, ' ', { width: 1500 })).toThrow('size');
    });
    it('typechecks the exact shared API fixtures used by Rust route tests', () => {
        const inspection: InspectRequest = { register: fixture.register as InspectRequest['register'] };
        const spread: SpreadRequest = { register: inspection.register, lays: fixture.lays as SpreadRequest['lays'] };
        const bundle: BundleRequest = { lay: fixture.lay };
        const report: LayActuals = fixture.actuals;
        const doc: PatternDocument = readPattern(JSON.stringify(pattern));
        expect([marker.result.placements.length, spread.lays!.length, bundle.lay.parts.length, report.actual_pieces, doc.sizes.length]).toEqual([1, 1, 1, 2, 4]);
    });
});
