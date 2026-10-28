import type { QuickPiece } from '@beta/marker-wasm';
import { defaultFabric, fabricProblem, initialState, materials, type PaperFabric, type QuickState } from './state';

const FORMAT = 'beta-marker.paper-set/v1';
const fail = (): never => { throw new Error('This is not a valid paper pattern set. Open a BETA paper-set.json file.'); };
const object = (v: unknown): Record<string, unknown> => v && typeof v === 'object' && !Array.isArray(v) ? v as Record<string, unknown> : fail();
const text = (v: unknown, max = 160): string => typeof v === 'string' && v.length > 0 && v.length <= max ? v : fail();
const safeKey = (v: string): boolean => !['__proto__','constructor','prototype'].includes(v);

/** Portable local outlines and settings; no source photos, object URLs or saved marker. */
export function savePatternSet(state: QuickState): string {
  if (state.photos.some(p => p.status === 'reading')) throw new Error('Wait for photo reading to finish before saving.');
  const fabrics = Object.fromEntries(materials(state).map(m => [m, m === state.material ? state.fabric : Object.hasOwn(state.fabricProfiles,m) ? state.fabricProfiles[m]! : defaultFabric()]));
  if (Object.values(fabrics).some(fabricProblem)) throw new Error('Correct the fabric GSM and gaps before saving.');
  if (!state.pieces.some(p => p.status !== 'deleted') || state.pieces.filter(p => p.status !== 'deleted').length > 300
    || state.photos.filter(p => p.status === 'done').length > 100 || materials(state).length > 40) throw new Error('Keep this set to 1..300 pieces, 100 photos and 40 fabric groups.');
  const source = JSON.stringify({ format: FORMAT, name: state.setName.trim() || 'Paper pattern set', practice: state.practice,
    reference: state.reference, selectedMaterial: state.material,
    fabrics, garments: state.garments,
    pieces: state.pieces.filter(p => p.status !== 'deleted').map(p => p.piece),
    photos: state.photos.filter(p => p.status === 'done').map(({ id, name, size }) => ({ id, name, size })),
  }, null, 2);
  if (new TextEncoder().encode(source).length > 8_000_000) throw new Error('Pattern set exceeds 8 MB. Split it into smaller styles.');
  return source;
}

function fabric(value: unknown): PaperFabric {
  const v = object(value), f = defaultFabric();
  if (v.widthMm !== null && (typeof v.widthMm !== 'number' || !Number.isFinite(v.widthMm) || v.widthMm <= 0 || v.widthMm > 10000)) fail();
  if (!['inch','cm'].includes(String(v.unit)) || !['open','folded','tubular'].includes(String(v.layout))
    || !['unknown','woven','knit'].includes(String(v.construction)) || typeof v.nap !== 'boolean') fail();
  if (v.gsm !== null && (typeof v.gsm !== 'number' || !Number.isFinite(v.gsm) || v.gsm <= 0 || v.gsm > 5000)) fail();
  for (const key of ['pieceGap','edgeGap']) if (typeof v[key] !== 'number' || !Number.isFinite(v[key]) || (v[key] as number) < 0 || (v[key] as number) > 100) fail();
  return { ...f, widthMm: v.widthMm as number | null, unit: v.unit as PaperFabric['unit'], layout: v.layout as PaperFabric['layout'],
    nap: v.nap as boolean, construction: v.construction as PaperFabric['construction'], gsm: v.gsm as number | null,
    pieceGap: v.pieceGap as number, edgeGap: v.edgeGap as number };
}

/** Imported geometry goes through Rust before use. Review and tape checks restart. */
export async function openPatternSet(source: string, assist: (p: QuickPiece[]) => Promise<QuickPiece[]>): Promise<QuickState> {
  if (source.length > 8_000_000) throw new Error('Pattern set exceeds 8 MB. Split it into smaller styles.');
  const v = object(JSON.parse(source));
  if (v.format !== FORMAT || typeof v.practice !== 'boolean' || !['a4-sheet','beta-target'].includes(String(v.reference))) fail();
  if (!Array.isArray(v.pieces) || !v.pieces.length || v.pieces.length > 300 || !Array.isArray(v.photos) || v.photos.length > 100) fail();
  const profiles = object(v.fabrics);
  if (Object.keys(profiles).length > 40) fail();
  const fabricProfiles = Object.fromEntries(Object.entries(profiles).map(([key,value]) => {
    if (!safeKey(key) || !key.trim() || key.length > 40) fail();
    return [key, fabric(value)];
  }));
  const photos = (v.photos as unknown[]).map(value => { const p = object(value); return { id: text(p.id,100), name: text(p.name,255), size: text(p.size,12), status: 'done' as const }; });
  const photoIds = new Set(photos.map(p => p.id));
  if (photos.some(p => !/^[a-zA-Z0-9_-]{1,100}$/.test(p.id))) fail();
  if (photoIds.size !== photos.length) fail();
  const ids = new Set<string>();
  for (const value of v.pieces as unknown[]) {
    const p = object(value), id = text(p.id), photoId = text(p.photoId,100);
    if (ids.has(id) || !photoIds.has(photoId) || !Array.isArray(p.outline) || p.outline.length < 3 || p.outline.length > 5000) fail();
    ids.add(id); text(p.name); if (!safeKey(text(p.size,12))) fail();
    const material = p.material == null ? 'main' : text(p.material,40);
    if (!safeKey(material) || !Object.hasOwn(fabricProfiles, material)) fail();
  }
  const rawQuantities = object(v.garments);
  if (Object.keys(rawQuantities).length > 100) fail();
  const garments = Object.fromEntries(Object.entries(rawQuantities).map(([size,n]) => {
    if (!safeKey(size) || size.length > 12 || !Number.isInteger(n) || (n as number) < 0 || (n as number) > 99) fail();
    return [size, n as number];
  }));
  // Assist validates outlines and existing review fields, without accepting suggestions.
  const pieces = await assist(v.pieces as QuickPiece[]);
  if (pieces.length !== ids.size || pieces.some(p => !ids.has(p.id))) fail();
  let state: QuickState = { ...initialState(), setName: text(v.name,80), practice: v.practice as boolean,
    reference: v.reference as QuickState['reference'], fabricProfiles, garments, photos,
    pieces: pieces.map(piece => ({ piece, status: 'pending', dismissed: [] })), step: 'review' };
  const material = typeof v.selectedMaterial === 'string' && materials(state).includes(v.selectedMaterial) ? v.selectedMaterial : materials(state)[0]!;
  state = { ...state, material, fabric: fabricProfiles[material]! };
  return state;
}
