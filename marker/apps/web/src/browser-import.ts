import { loadMarkerImportCore, type MarkerImportCore, type Units } from '@beta/marker-wasm';
let core: Promise<MarkerImportCore> | undefined;
function load(): Promise<MarkerImportCore> {
  core ??= loadMarkerImportCore().catch(error => { core = undefined; throw error; });
  return core;
}
export async function browserImport(url: string, init?: RequestInit, loader = load): Promise<Response | null> {
  const route = new URL(url, 'http://beta.local');
  if (route.pathname !== '/api/import' || (init?.method ?? 'GET').toUpperCase() !== 'POST') return null;
  if ((route.searchParams.get('format') ?? 'dxf') !== 'dxf') return null;
  try {
    for (const key of ['format', 'units', 'pairs', 'width']) {
      if (route.searchParams.getAll(key).length > 1) throw new Error(`Duplicate parameter: ${key}`);
    }
    const units = route.searchParams.get('units') ?? 'auto';
    const pairs = route.searchParams.get('pairs') ?? 'true'; // Same default as native serve and ImportOptions.
    if (!['auto', 'mm', 'cm', 'inch'].includes(units)) throw new Error('Units must be auto, mm, cm or inch');
    if (pairs !== 'true' && pairs !== 'false') throw new Error('Pairs must be true or false');
    const bytes = new Uint8Array(await new Response(init?.body).arrayBuffer());
    const report = (await loader()).importDxf(bytes, { units: units as Units, pairs: pairs === 'true' });
    return new Response(JSON.stringify({ report }), { headers: { 'Content-Type': 'application/json' } });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error instanceof Error ? error.message : error) }), {
      status: 400, headers: { 'Content-Type': 'application/json' },
    });
  }
}
