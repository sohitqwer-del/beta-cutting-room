/** Local byte transport for factory exports in the static browser edition. */
import { loadMarkerExportCore, type Job, type MarkerExportCore, type MarkerExportFormat,
  type MarkerExportOptions, type MarkerResult } from '@beta/marker-wasm';

const formats: Record<MarkerExportFormat, [string, string]> = {
  svg: ['image/svg+xml', 'svg'], plt: ['application/vnd.hp-hpgl', 'plt'],
  hpgl: ['application/vnd.hp-hpgl', 'plt'], pdf: ['application/pdf', 'pdf'],
  dxf: ['image/vnd.dxf', 'dxf'], iso: ['text/plain; charset=utf-8', 'iso'],
};
let core: Promise<MarkerExportCore> | undefined;
function load(): Promise<MarkerExportCore> {
  core ??= loadMarkerExportCore().catch(error => { core = undefined; throw error; });
  return core;
}

/** Null means this route remains owned by the normal transport. */
export async function browserExport(url: string, init?: RequestInit, loader = load): Promise<Response | null> {
  const route = new URL(url, 'http://beta.local');
  if (route.pathname !== '/api/export' || (init?.method ?? 'GET').toUpperCase() !== 'POST') return null;
  const format = (route.searchParams.get('format') ?? 'svg').toLowerCase();
  if (!Object.hasOwn(formats, format)) return null;
  try {
    const input = await new Response(init?.body).json() as { job: Job; result: MarkerResult; options?: MarkerExportOptions };
    const bytes = (await loader()).exportMarker(input.job, input.result, format as MarkerExportFormat, input.options);
    const [mime, extension] = formats[format as MarkerExportFormat];
    return new Response(new Uint8Array(bytes), { headers: { 'Content-Type': mime, 'X-Beta-Extension': extension } });
  } catch (error) {
    return new Response(JSON.stringify({ error: String(error instanceof Error ? error.message : error) }), {
      status: 400, headers: { 'Content-Type': 'application/json' },
    });
  }
}
