import { describe, expect, it, vi } from 'vitest';
import { browserExport } from './browser-export';
import type { MarkerExportCore } from '@beta/marker-wasm';

describe('browser factory file transport', () => {
  const job = { name: 'synthetic' }, result = { placements: [] };
  it.each(['svg', 'plt', 'hpgl', 'pdf', 'dxf', 'iso'])('preserves %s bytes and options', async format => {
    const exportMarker = vi.fn().mockReturnValue(new Uint8Array([0, 128, 255, 37]));
    const loader = async () => ({ exportMarker } as unknown as MarkerExportCore);
    const options = { rotate: true, marks: false, pdf_tile_mm: 210 };
    const response = await browserExport(`/api/export?format=${format}`, {
      method: 'POST', body: JSON.stringify({ job, result, options }),
    }, loader);
    expect(response?.status).toBe(200);
    expect(Array.from(new Uint8Array(await response!.arrayBuffer()))).toEqual([0, 128, 255, 37]);
    expect(exportMarker).toHaveBeenCalledWith(job, result, format, options);
    expect(response?.headers.get('X-Beta-Extension')).toBe(format === 'hpgl' ? 'plt' : format);
  });
  it('returns Rust failures as JSON, and leaves unrelated routes alone', async () => {
    const loader = async () => ({ exportMarker: () => { throw new Error('Unknown piece'); } } as unknown as MarkerExportCore);
    const response = await browserExport('/api/export?format=pdf', { method: 'POST', body: JSON.stringify({ job, result }) }, loader);
    expect(response?.status).toBe(400);
    expect(await response?.json()).toEqual({ error: 'Unknown piece' });
    for (const url of ['/api/plan', '/api/export?format=unknown']) {
      expect(await browserExport(url, { method: 'POST' }, loader)).toBeNull();
    }
    expect(await browserExport('/api/export', undefined, loader)).toBeNull();
  });
});
