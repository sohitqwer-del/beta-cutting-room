import { expect, it, vi } from 'vitest';
import { browserImport } from './browser-import';
import type { MarkerImportCore } from '@beta/marker-wasm';

it('passes original DXF bytes and import settings to Rust and retains the complete report', async () => {
  const report = { pieces: [{ name: 'Panel', size: 'M', quantity: 2, marks: [{ kind: 'notch', points: [[0, 1]] }] }], units: 'inch', dialect: 'astm', sizes: ['M'], warnings: ['Verify scale'] };
  const importDxf = vi.fn().mockReturnValue(report);
  const response = await browserImport('/api/import?format=dxf&units=inch&pairs=true&width=1500', {
    method: 'POST', body: new Uint8Array([0, 65, 255]),
  }, async () => ({ importDxf }));
  expect(importDxf).toHaveBeenCalledWith(new Uint8Array([0, 65, 255]), { units: 'inch', pairs: true });
  expect(await response?.json()).toEqual({ report });
});

it('rejects ambiguous controls and Rust parse errors without network fallback', async () => {
  const loader = async () => ({ importDxf: () => { throw new Error('Invalid DXF'); } } as MarkerImportCore);
  for (const query of ['units=feet', 'pairs=yes', 'pairs=true&pairs=false', '']) {
    const response = await browserImport(`/api/import?${query}`, { method: 'POST', body: 'bad' }, loader);
    expect(response?.status).toBe(400);
    expect((await response?.json()).error).toBeTruthy();
  }
  expect(await browserImport('/api/import?format=plt', { method: 'POST' }, loader)).toBeNull();
});
