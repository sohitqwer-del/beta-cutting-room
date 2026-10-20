import type { Job, MarkerResult, Placement, Pt } from '@beta/marker-wasm';
import type { ProjectorLayout, QuickFabric, QuickMarkerCore, QuickPiece, QuickScan, QuickScanOptions, QuickSession, QuickSuggestions, QuickWarning } from '@beta/marker-wasm';
import { area, bounds } from './geometry';

/** Typed stand-in for the Rust Quick Marker API, used by tests, the headless smoke and the "Try a sample photo"
 * practice mode. Shapes are original and synthetic (a simple shirt). Results are NOT real scans or real nests. */
export const MOCK_ENGINE = 'practice mock';

/** Sample photo layout: 0.8 px per mm, origin (mm) at pixel (120, 1100), y up. 1600 × 1200 px. */
export const SAMPLE_SIZE: [number, number] = [1600, 1200];
const PX_PER_MM = 0.8, OX = 120, OY = 1100;
export const SAMPLE_IMAGE_TO_MM = [1 / PX_PER_MM, 0, -OX / PX_PER_MM, 0, -1 / PX_PER_MM, OY / PX_PER_MM, 0, 0, 1];

const move = (pts: Pt[], dx: number, dy: number): Pt[] => pts.map(([x, y]) => [x + dx, y + dy]);
const mirrorX = (pts: Pt[]): Pt[] => { const b = bounds(pts); return pts.map(([x, y]) => [b.minX + b.maxX - x, y] as Pt).reverse(); };
const FRONT: Pt[] = [[0, 0], [300, 0], [300, 620], [210, 700], [120, 690], [60, 640], [0, 560]];
const BACK: Pt[] = [[0, 0], [560, 0], [560, 560], [480, 660], [380, 690], [180, 690], [80, 660], [0, 560]];
const SLEEVE: Pt[] = [[0, 0], [380, 0], [430, 300], [340, 480], [190, 540], [40, 480], [-50, 300]];
const COLLAR: Pt[] = [[0, 0], [420, 0], [440, 40], [420, 90], [0, 90], [-20, 40]];
const CUFF: Pt[] = [[0, 0], [250, 0], [250, 70], [0, 70]];
const POCKET: Pt[] = [[0, 30], [30, 0], [130, 0], [160, 30], [160, 170], [0, 170]];
const YOKE: Pt[] = [[0, 0], [520, 0], [500, 120], [20, 120]];

interface Spec { id: string; outline: Pt[]; name: string; conf: 'high' | 'medium' | 'low'; s: Partial<QuickSuggestions> }
function sheetPieces(photoIndex: number): Spec[] {
  const symmetric = (pts: Pt[]): [Pt, Pt] => { const b = bounds(pts), cx = (b.minX + b.maxX) / 2; return [[cx, b.minY], [cx, b.maxY]]; };
  const along = (pts: Pt[]): [Pt, Pt] => { const b = bounds(pts), cx = (b.minX + b.maxX) / 2; return [[cx, b.minY + b.height * .2], [cx, b.maxY - b.height * .2]]; };
  if (photoIndex === 0) {
    const fl = move(FRONT, 330, 0), fr = move(mirrorX(FRONT), 0, 0), back = move(BACK, 700, 0), sleeve = move(SLEEVE, 1360, 0);
    return [
      { id: 'p1', outline: fr, name: 'Front', conf: 'high', s: { pair: 'mirror', pairPieceId: 'p2', quantity: { value: { normal: 1, mirrored: 1 }, confidence: 'high', reason: 'Mirror image of another piece.' }, grain: { value: along(fr), confidence: 'medium', reason: 'Long axis of the piece.' } } },
      { id: 'p2', outline: fl, name: 'Front', conf: 'high', s: { pair: 'mirror', pairPieceId: 'p1', quantity: { value: { normal: 1, mirrored: 1 }, confidence: 'high', reason: 'Mirror image of another piece.' }, grain: { value: along(fl), confidence: 'medium', reason: 'Long axis of the piece.' } } },
      { id: 'p3', outline: back, name: 'Back', conf: 'medium', s: { pair: 'symmetric', symmetryAxis: { value: symmetric(back), confidence: 'high', reason: 'Left and right halves match.' }, cutOnFold: { value: true, confidence: 'medium', reason: 'Symmetric piece: it can be cut on a fold.' }, quantity: { value: { normal: 1, mirrored: 0 }, confidence: 'medium', reason: 'One symmetric piece per garment.' }, grain: { value: symmetric(back), confidence: 'high', reason: 'Symmetry axis.' } } },
      { id: 'p4', outline: sleeve, name: 'Sleeve', conf: 'high', s: { pair: 'single', quantity: { value: { normal: 1, mirrored: 1 }, confidence: 'medium', reason: 'Sleeves are usually cut as a left and right pair.' }, grain: { value: symmetric(sleeve), confidence: 'medium', reason: 'Sleeve centre line.' } } },
      { id: 'p5', outline: move(COLLAR, 0, 780), name: 'Collar', conf: 'medium', s: { pair: 'symmetric', symmetryAxis: { value: symmetric(move(COLLAR, 0, 780)), confidence: 'medium', reason: 'Left and right halves match.' }, quantity: { value: { normal: 2, mirrored: 0 }, confidence: 'low', reason: 'Collars are often cut twice (upper and under collar).' } } },
      { id: 'p6', outline: move(CUFF, 520, 790), name: 'Cuff', conf: 'medium', s: { pair: 'symmetric', quantity: { value: { normal: 2, mirrored: 0 }, confidence: 'medium', reason: 'One cuff for each sleeve.' }, grain: { value: [[540, 825], [750, 825]], confidence: 'low', reason: 'Long axis of the piece.' } } },
      { id: 'p7', outline: move(POCKET, 840, 760), name: 'Pocket', conf: 'low', s: { pair: 'symmetric' } },
    ];
  }
  return [
    { id: 'p1', outline: move(YOKE, 0, 0), name: 'Yoke', conf: 'low', s: { pair: 'symmetric', symmetryAxis: { value: symmetric(YOKE), confidence: 'high', reason: 'Left and right halves match.' }, cutOnFold: { value: true, confidence: 'medium', reason: 'Symmetric piece: it can be cut on a fold.' }, quantity: { value: { normal: 2, mirrored: 0 }, confidence: 'low', reason: 'Yokes are often cut twice (outer and lining).' } } },
    { id: 'p2', outline: move(POCKET, 640, 0), name: 'Pocket', conf: 'low', s: { pair: 'symmetric' } },
  ];
}

const WARNINGS: QuickWarning[][] = [
  [{ code: 'glare', message: 'Bright reflection on part of the photo.' }],
  [],
];
function pieceFrom(spec: Spec, photoId: string): QuickPiece {
  return {
    id: spec.id, name: `Piece ${spec.id.slice(1)}`, size: null, outline: spec.outline, grain: null, quantity: 1, mirror_quantity: 0,
    rotation: 'inherit', buffer: 0, marks: [], fold_line: null, photoId, expectedErrorMm: 2.5, notches: [], drills: [],
    suggestions: { pair: 'single', ...spec.s, name: { value: spec.name, confidence: spec.conf, reason: 'Shape looks like this part.' } },
  };
}
const index = (photoId: string): number => { const n = Number(/\d+$/.exec(photoId)?.[0] ?? 1); return Math.max(0, n - 1) % 2; };

export function mockScan(photo: Uint8Array, options: QuickScanOptions): QuickScan {
  if (!photo.length) throw new Error('The photo is empty.');
  const photoId = options.photoId ?? 'photo-1', i = index(photoId);
  const pieces = sheetPieces(i).slice(0, options.maxPieces ?? 40).map(s => pieceFrom(s, photoId));
  return {
    photoId, pieces, overlayPng: new Uint8Array(),
    scale: { mmPerPx: 1 / PX_PER_MM, confidence: options.reference === 'beta-target' ? 'high' : 'medium', method: options.reference, expectedErrorMm: options.reference === 'beta-target' ? 0.5 : 3, tiltScore: 0.1, referenceCorners: [], imageToMm: SAMPLE_IMAGE_TO_MM },
    warnings: WARNINGS[i]!.map(w => ({ ...w, photoId })), sourceSize: SAMPLE_SIZE, processedSize: SAMPLE_SIZE, runtimeMs: 5,
  };
}
export function mockMerge(scans: Pick<QuickScan, 'photoId' | 'pieces' | 'warnings'>[]): QuickSession {
  if (new Set(scans.map(s => s.photoId)).size !== scans.length) throw new Error('Photo IDs must be unique.');
  const pieces = scans.flatMap(s => s.pieces.map(p => ({ ...p, id: `${s.photoId}:${p.id}`, suggestions: { ...p.suggestions, pairPieceId: p.suggestions.pairPieceId ? `${s.photoId}:${p.suggestions.pairPieceId}` : undefined } })));
  const duplicates: QuickSession['duplicates'] = [];
  pieces.forEach((p, k) => {
    const first = pieces.findIndex(q => q.photoId !== p.photoId && q.suggestions.name?.value === p.suggestions.name?.value && Math.abs(area(q.outline) - area(p.outline)) < 1);
    if (first >= 0 && first < k) duplicates.push({ pieceId: p.id, duplicateOf: pieces[first]!.id, confidence: 'medium' });
  });
  return { pieces, warnings: scans.flatMap(s => s.warnings), duplicates };
}
export function mockToJob(pieces: QuickPiece[], fabric: QuickFabric, garments: Record<string, number>): Job {
  if (!pieces.length) throw new Error('Accept at least one piece.');
  for (const p of pieces) if (!p.size || !(p.size in garments)) throw new Error(`Piece ${p.name} has no ordered size.`);
  for (const p of pieces) if (!p.grain) throw new Error(`Piece ${p.name} has no confirmed grain line.`);
  for (const size of Object.keys(garments)) if (!pieces.some(p => p.size === size)) throw new Error(`Size ${size} has no pieces.`);
  return {
    format: 'beta-marker.job/v1', name: 'Quick marker', fabric: { width: fabric.width_mm, nap: fabric.nap ?? 'none', layout: fabric.layout ?? 'open' },
    spacing: { piece_gap: 3, edge_gap: 5 }, order: { sizes: garments }, meta: { source: 'Quick Marker practice mock' },
    pieces: pieces.map(p => ({ id: p.id, name: p.name, size: p.size, outline: p.outline, grain: p.grain ?? null, quantity: p.quantity, mirror_quantity: p.mirror_quantity, rotation: p.rotation ?? 'inherit', buffer: p.buffer ?? 0, marks: p.marks ?? [], fold_line: p.fold_line ?? null })),
  };
}
/** Shelf layout of bounding boxes. Only for practice mode; the real marker always comes from the Rust nester. */
export function mockNest(job: Job): MarkerResult {
  const width = job.fabric.width, gap = 10, placements: Placement[] = [];
  let x = gap, y = gap, column = 0, area2 = 0;
  for (const piece of job.pieces) {
    const n = job.order?.sizes[piece.size ?? ''] ?? 1, b = bounds(piece.outline);
    const copiesN = (piece.quantity ?? 1) * n, copiesM = (piece.mirror_quantity ?? 0) * n;
    for (let c = 0; c < copiesN + copiesM; c++) {
      const mirrored = c >= copiesN;
      if (y + b.height + gap > width) { x += column + gap; y = gap; column = 0; }
      placements.push({ piece_id: piece.id, copy: mirrored ? c - copiesN : c, mirrored, rotation_deg: 0, translate: mirrored ? [x + b.maxX, y - b.minY] : [x - b.minX, y - b.minY] });
      y += b.height + gap; column = Math.max(column, b.width); area2 += area(piece.outline);
    }
  }
  const length = x + column + gap;
  return { format: 'beta-marker.result/v1', job_name: job.name, width, length, piece_area: area2, efficiency: area2 / (width * length), engine: MOCK_ENGINE, placements, warnings: ['Practice layout: not a real nest.'] };
}
/** A tiny valid one-page PDF that says it is a practice file. */
export function mockPdf(title: string): Uint8Array {
  const text = `BT /F1 18 Tf 72 760 Td (${title.replace(/[()\\]/g, '')} - practice file, not for cutting) Tj ET`;
  const objects = ['<< /Type /Catalog /Pages 2 0 R >>', '<< /Type /Pages /Kids [3 0 R] /Count 1 >>', '<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>', `<< /Length ${text.length} >>\nstream\n${text}\nendstream`, '<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>'];
  let out = '%PDF-1.4\n'; const offsets: number[] = [];
  objects.forEach((o, k) => { offsets.push(out.length); out += `${k + 1} 0 obj\n${o}\nendobj\n`; });
  const xref = out.length;
  out += `xref\n0 ${objects.length + 1}\n0000000000 65535 f \n${offsets.map(o => `${String(o).padStart(10, '0')} 00000 n \n`).join('')}trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`;
  return new TextEncoder().encode(out);
}
export function mockProjector(job: Job, result: MarkerResult): ProjectorLayout {
  const byId = new Map(job.pieces.map(p => [p.id, p]));
  return {
    format: 'beta-marker.projector/v1', units: 'mm', lengthMm: result.length, widthMm: job.fabric.width, calibrationSquareMm: 100,
    pieces: result.placements.map(pl => {
      const p = byId.get(pl.piece_id)!;
      const outline = p.outline.map(([x, y]) => [(pl.mirrored ? -x : x) + pl.translate[0], y + pl.translate[1]] as Pt);
      return { pieceId: p.id, name: p.name, size: p.size ?? null, copy: pl.copy, outline, referencePoint: outline[0]!, rotationDeg: pl.rotation_deg, mirrored: pl.mirrored, onFold: !!pl.on_fold };
    }),
  };
}
export function mockCore(): QuickMarkerCore {
  return {
    quickScan: mockScan,
    quickAssist: pieces => pieces.map(p => ({ ...p })),
    quickMerge: mockMerge,
    quickPiecesToJob: mockToJob,
    tiledMarkerPdf: (_job, _result, o) => mockPdf(`Tiled marker ${o.paper}`),
    chalkGuidePdf: () => mockPdf('Chalk guide'),
    projectorLayout: mockProjector,
  };
}

/** Draw the sample photo (dark cloth, A4 sheet, paper pieces) so practice mode shows a believable picture. Browser only. */
export async function samplePhoto(photoIndex: number): Promise<Blob> {
  const canvas = document.createElement('canvas'); [canvas.width, canvas.height] = SAMPLE_SIZE;
  const g = canvas.getContext('2d')!;
  const grad = g.createLinearGradient(0, 0, 1600, 1200); grad.addColorStop(0, '#2b2f3a'); grad.addColorStop(1, '#1d2028');
  g.fillStyle = grad; g.fillRect(0, 0, 1600, 1200);
  for (let k = 0; k < 2200; k++) { g.fillStyle = `rgba(255,255,255,${(k % 7) / 400})`; g.fillRect((k * 7919) % 1600, (k * 104729) % 1200, 2, 2); }
  const px = ([x, y]: Pt): Pt => [OX + x * PX_PER_MM, OY - y * PX_PER_MM];
  const poly = (pts: Pt[], fill: string) => { g.beginPath(); pts.map(px).forEach(([x, y], k) => k ? g.lineTo(x, y) : g.moveTo(x, y)); g.closePath(); g.fillStyle = fill; g.fill(); g.strokeStyle = 'rgba(80,60,30,.5)'; g.lineWidth = 2; g.stroke(); };
  g.save(); g.shadowColor = 'rgba(0,0,0,.45)'; g.shadowBlur = 14; g.shadowOffsetY = 6;
  const a4: Pt[] = photoIndex === 0 ? [[1250, 640], [1460, 640], [1460, 937], [1250, 937]] : [[1000, 0], [1297, 0], [1297, 210], [1000, 210]];
  poly(a4, '#fbfbf7');
  for (const s of sheetPieces(photoIndex)) poly(s.outline, '#e9dcc0');
  g.restore();
  g.fillStyle = '#6b6252'; g.font = '600 26px sans-serif';
  const [ax, ay] = px([a4[0]![0] + 12, a4[2]![1] - 40]); g.fillText('A4', ax, ay);
  return new Promise((resolve, reject) => canvas.toBlob(b => b ? resolve(b) : reject(new Error('Sample photo failed.')), 'image/jpeg', 0.86));
}
