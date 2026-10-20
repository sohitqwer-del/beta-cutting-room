import { describe, expect, it } from 'vitest';
import { carryWarnings, seamlyWarnings } from './import-warnings';
describe('Seamly import warnings (QA-004)', () => {
  it('reads only non-empty strings from document meta', () => {
    expect(seamlyWarnings({ seamly_import_warnings: ['BP-004 Front: mirrored copy', '', 3, 'Background image omitted'] })).toEqual(['BP-004 Front: mirrored copy', 'Background image omitted']);
    expect(seamlyWarnings(undefined)).toEqual([]);
    expect(seamlyWarnings({ seamly_import_warnings: 'not a list' })).toEqual([]);
  });
  it('survives an edit that clones the document and carries over to a marker job', () => {
    const doc = { name: 'Shirt', meta: { seamly_import_warnings: ['BP-004 Back'], construction_order: ['A'] } };
    const edited = structuredClone(doc); edited.meta.construction_order.push('B');
    expect(seamlyWarnings(edited.meta)).toEqual(['BP-004 Back']);
    const job = carryWarnings({ name: 'job', meta: { editor_v1: true } }, edited.meta, edited.name);
    expect(job.meta).toEqual({ editor_v1: true, seamly_import_warnings: ['BP-004 Back'], seamly_source_pattern: 'Shirt' });
    const plain: { name: string; meta?: Record<string, unknown> } = { name: 'job' };
    expect(carryWarnings(plain, {}, 'x')).toEqual({ name: 'job' });
  });
});
