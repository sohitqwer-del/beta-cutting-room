import type { BlockEase, BlockInfo, BlockKind, DrawingObject, PatternDocument, PatternPiece, SeamCheck, SizeChart } from '@beta/marker-wasm';

/** Preview-engine starting blocks and example charts. Original BETA teaching drafts written as formulas
 * (`m.` measurement, `v.` ease); the Rust Pattern core evaluates every coordinate.
 * They stand in until the Style engine's own blocks ship. Not fit-tested production patterns. */
type F = string | number;
const SIZES = ['S', 'M', 'L', 'XL', 'XXL'];
const ROWS: Record<string, number[]> = {
  chest: [920, 980, 1040, 1100, 1160], waist: [780, 840, 900, 960, 1020], hip: [960, 1010, 1060, 1110, 1160],
  shoulder: [420, 440, 460, 480, 500], neck: [370, 385, 400, 415, 430], back_length: [420, 430, 440, 450, 460],
  garment_length: [680, 700, 720, 740, 760], kurta_length: [1000, 1020, 1040, 1060, 1080], armhole_depth: [210, 220, 230, 240, 250],
  sleeve_short: [190, 200, 210, 220, 230], sleeve_long: [600, 610, 620, 630, 640], bicep: [320, 340, 360, 380, 400], wrist: [170, 175, 180, 185, 190],
  rise: [260, 265, 270, 275, 280], inseam: [780, 790, 800, 810, 820], knee: [420, 435, 450, 465, 480], bottom: [380, 390, 400, 410, 420], skirt_length: [600, 610, 620, 630, 640],
};
export function presetChart(preset: 'indian_adult' | 'iso_example' = 'indian_adult'): SizeChart {
  const sizes = preset === 'iso_example' ? ['88', '92', '96', '100', '104'] : SIZES;
  const shift = preset === 'iso_example' ? -0.02 : 0;
  const measurements: Record<string, Record<string, number>> = {};
  sizes.forEach((size, i) => { measurements[size] = Object.fromEntries(Object.entries(ROWS).map(([k, v]) => [k, Math.round(v[i]! * (1 + shift))])); });
  return { name: preset === 'iso_example' ? 'Example chart (body sizes)' : 'Example chart (Indian adult)', sizes, base_size: sizes[1]!, measurements };
}

export const BLOCKS: BlockInfo[] = [
  { id: 't_shirt', name: 'T-shirt', pieces: ['Front', 'Back', 'Sleeve'], note: 'Knit tee, short sleeve, crew neck.' },
  { id: 'shirt', name: 'Shirt', pieces: ['Front', 'Back', 'Sleeve', 'Collar', 'Cuff'], note: 'Woven shirt, button front, long sleeve.' },
  { id: 'trouser', name: 'Trouser', pieces: ['Front', 'Back', 'Waistband'], note: 'Straight trouser with waistband.' },
  { id: 'a_line_skirt', name: 'Skirt', pieces: ['Front', 'Back', 'Waistband'], note: 'A-line skirt.' },
  { id: 'kurta', name: 'Kurta', pieces: ['Front', 'Back', 'Sleeve'], note: 'Straight kurta, long sleeve.' },
  { id: 'polo', name: 'Polo', pieces: ['Front', 'Back', 'Sleeve', 'Collar', 'Placket'], note: 'Knit polo with rib collar.' },
  { id: 'track_pant', name: 'Track pant', pieces: ['Front', 'Back', 'Waistband'], note: 'Knit track pant with elastic waist.' },
];

class Draft {
  drawing: Record<string, DrawingObject> = {};
  pieces: PatternPiece[] = [];
  checks: SeamCheck[] = [];
  point(id: string, x: F, y: F, dx = 0, dy = 0): string {
    const sx = dx ? (typeof x === 'number' ? x + dx : `(${x})+${dx}`) : x, sy = dy ? (typeof y === 'number' ? y + dy : `(${y})+${dy}`) : y;
    this.drawing[id] = { kind: 'point', operation: { op: 'absolute', x: sx, y: sy } };
    return id;
  }
  line(id: string, from: string, to: string): string { this.drawing[id] = { kind: 'line', from, to }; return id; }
  cubic(id: string, a: string, c1: string, c2: string, b: string): string { this.drawing[id] = { kind: 'cubic', points: [a, c1, c2, b] }; return id; }
  piece(p: PatternPiece): void { this.pieces.push(p); }
}
const neg = (x: F): F => typeof x === 'number' ? -x : `-(${x})`;
const edges = (list: Array<[string, boolean?, F?]>, allowance: F = 10) => list.map(([object, reversed, a]) => ({ object, reversed: !!reversed, allowance: a ?? allowance, corner: 'miter' as const }));

/** Full symmetric bodice (front or back) with centre at x = dx. */
function bodice(d: Draft, id: string, name: string, o: { length: F; neckDrop: F; dx: number }): void {
  const W = '(m.chest+v.ease_chest)/4', L = o.length, P = (s: string) => `${id}.${s}`;
  const side = (k: 1 | -1) => {
    const s = k === 1 ? 'r' : 'l', X = (x: F) => k === 1 ? x : neg(x);
    d.point(P(`hem_${s}`), X(W), 0, o.dx);
    d.point(P(`under_${s}`), X(W), `${L}-m.armhole_depth-30`, o.dx);
    d.point(P(`sh_${s}`), X('m.shoulder/2'), `${L}-45`, o.dx);
    d.point(P(`neck_${s}`), X('m.neck/6+10'), L, o.dx);
    d.point(P(`arm1_${s}`), X('m.shoulder/2-15'), `${L}-45-m.armhole_depth*0.45`, o.dx);
    d.point(P(`arm2_${s}`), X(`${W}-35`), `${L}-m.armhole_depth-25`, o.dx);
    d.point(P(`nk1_${s}`), X(`m.neck/6+10-(${o.neckDrop})*0.35`), `${L}-(${o.neckDrop})*0.45`, o.dx);
    d.point(P(`nk2_${s}`), X('m.neck/12'), `${L}-(${o.neckDrop})`, o.dx);
    d.line(P(`side_${s}`), P(`hem_${s}`), P(`under_${s}`));
    d.cubic(P(`arm_${s}`), P(`sh_${s}`), P(`arm1_${s}`), P(`arm2_${s}`), P(`under_${s}`));
    d.line(P(`shoulder_${s}`), P(`sh_${s}`), P(`neck_${s}`));
    d.cubic(P(`neckline_${s}`), P(`neck_${s}`), P(`nk1_${s}`), P(`nk2_${s}`), P('neck_c'));
  };
  d.point(P('neck_c'), 0, `${L}-(${o.neckDrop})`, o.dx);
  side(1); side(-1);
  d.line(P('hem'), P('hem_l'), P('hem_r'));
  d.point(P('grain0'), 0, 80, o.dx); d.point(P('grain1'), 0, `${L}-m.armhole_depth-80`, o.dx);
  d.piece({ id, name, quantity: 1, mirror_quantity: 0, material: 'shell', grain: [P('grain0'), P('grain1')], annotations: [], grade_points: {}, anchors: {},
    edges: edges([[P('hem'), false, 25], [P('side_r')], [P('arm_r'), true], [P('shoulder_r')], [P('neckline_r')], [P('neckline_l'), true], [P('shoulder_l'), true], [P('arm_l')], [P('side_l'), true]]) });
}
/** Full sleeve, cut as a left/right pair. */
function sleeve(d: Draft, id: string, o: { length: F; hemHalf: F; dx: number }): void {
  const B = '(m.bicep+v.ease_bicep)/2', H = 'm.armhole_depth*0.55', S = o.length, P = (s: string) => `${id}.${s}`;
  for (const k of [1, -1] as const) {
    const s = k === 1 ? 'r' : 'l', X = (x: F) => k === 1 ? x : neg(x);
    d.point(P(`hem_${s}`), X(o.hemHalf), 0, o.dx); d.point(P(`under_${s}`), X(B), `${S}-${H}`, o.dx);
    d.point(P(`c1_${s}`), X(`${B}*0.62`), `${S}-${H}*0.55`, o.dx); d.point(P(`c2_${s}`), X(`${B}*0.3`), S, o.dx);
    d.line(P(`side_${s}`), P(`hem_${s}`), P(`under_${s}`));
    d.cubic(P(`cap_${s}`), P(`under_${s}`), P(`c1_${s}`), P(`c2_${s}`), P('top'));
  }
  d.point(P('top'), 0, S, o.dx);
  d.line(P('hem'), P('hem_l'), P('hem_r'));
  d.point(P('grain0'), 0, 40, o.dx); d.point(P('grain1'), 0, `${S}-${H}`, o.dx);
  d.piece({ id, name: 'Sleeve', quantity: 1, mirror_quantity: 1, material: 'shell', grain: [P('grain0'), P('grain1')], annotations: [], grade_points: {}, anchors: {},
    edges: edges([[P('hem'), false, 25], [P('side_r')], [P('cap_r')], [P('cap_l'), true], [P('side_l'), true]]) });
}
function rect(d: Draft, id: string, name: string, w: F, h: F, o: { dx: number; dy?: number; quantity?: number; mirror?: number; material?: string }): void {
  const P = (s: string) => `${id}.${s}`, dy = o.dy ?? 0;
  d.point(P('a'), 0, 0, o.dx, dy); d.point(P('b'), w, 0, o.dx, dy); d.point(P('c'), w, h, o.dx, dy); d.point(P('d'), 0, h, o.dx, dy);
  d.line(P('bottom'), P('a'), P('b')); d.line(P('right'), P('b'), P('c')); d.line(P('top'), P('c'), P('d')); d.line(P('left'), P('d'), P('a'));
  d.point(P('grain0'), `(${w})/2`, `(${h})*0.2`, o.dx, dy); d.point(P('grain1'), `(${w})/2`, `(${h})*0.8`, o.dx, dy);
  d.piece({ id, name, quantity: o.quantity ?? 1, mirror_quantity: o.mirror ?? 0, material: o.material ?? 'shell', grain: [P('grain0'), P('grain1')], annotations: [], grade_points: {}, anchors: {},
    edges: edges([[P('bottom')], [P('right')], [P('top')], [P('left')]]) });
}
/** One leg panel (front or back), cut as a left/right pair. */
function leg(d: Draft, id: string, name: string, o: { crotch: F; waistRise: F; dx: number; elastic?: boolean }): void {
  const H = '(m.hip+v.ease_hip)/4', P = (s: string) => `${id}.${s}`, top = `m.inseam+m.rise+(${o.waistRise})`;
  d.point(P('hem_out'), 0, 0, o.dx); d.point(P('hem_in'), 'm.bottom/2', 0, o.dx);
  d.point(P('crotch'), `${H}+(${o.crotch})`, 'm.inseam', o.dx);
  d.point(P('hip_c'), H, 'm.inseam+m.rise-90', o.dx);
  d.point(P('cr1'), `${H}+(${o.crotch})*0.35`, 'm.inseam+5', o.dx); d.point(P('cr2'), H, 'm.inseam+m.rise*0.3', o.dx);
  d.point(P('waist_c'), o.elastic ? H : `${H}-10`, top, o.dx);
  d.point(P('waist_s'), o.elastic ? -10 : `${H}-10-(m.waist+v.ease_waist)/4`, top, o.dx);
  d.point(P('hip_s'), -10, 'm.inseam+m.rise-180', o.dx);
  d.line(P('hem'), P('hem_out'), P('hem_in')); d.line(P('inseam'), P('hem_in'), P('crotch'));
  d.cubic(P('crotch_curve'), P('crotch'), P('cr1'), P('cr2'), P('hip_c')); d.line(P('centre'), P('hip_c'), P('waist_c'));
  d.line(P('waist'), P('waist_c'), P('waist_s')); d.line(P('side_top'), P('waist_s'), P('hip_s')); d.line(P('side'), P('hip_s'), P('hem_out'));
  d.point(P('grain0'), 'm.bottom/4', 60, o.dx); d.point(P('grain1'), 'm.bottom/4', 'm.inseam', o.dx);
  d.piece({ id, name, quantity: 1, mirror_quantity: 1, material: 'shell', grain: [P('grain0'), P('grain1')], annotations: [], grade_points: {}, anchors: {},
    edges: edges([[P('hem'), false, 30], [P('inseam')], [P('crotch_curve')], [P('centre')], [P('waist')], [P('side_top')], [P('side')]]) });
}
/** Full symmetric A-line skirt panel. */
function skirtPanel(d: Draft, id: string, name: string, dx: number): void {
  const P = (s: string) => `${id}.${s}`;
  for (const k of [1, -1] as const) {
    const s = k === 1 ? 'r' : 'l', X = (x: F) => k === 1 ? x : neg(x);
    d.point(P(`hem_${s}`), X('(m.hip+v.ease_hip)/4+70'), 0, dx); d.point(P(`hip_${s}`), X('(m.hip+v.ease_hip)/4'), 'm.skirt_length-180', dx);
    d.point(P(`waist_${s}`), X('(m.waist+v.ease_waist)/4+15'), 'm.skirt_length+10', dx);
    d.line(P(`side_${s}`), P(`hem_${s}`), P(`hip_${s}`)); d.line(P(`top_${s}`), P(`hip_${s}`), P(`waist_${s}`));
  }
  d.line(P('hem'), P('hem_l'), P('hem_r')); d.line(P('waist'), P('waist_r'), P('waist_l'));
  d.point(P('grain0'), 0, 60, dx); d.point(P('grain1'), 0, 'm.skirt_length-120', dx);
  d.piece({ id, name, quantity: 1, mirror_quantity: 0, material: 'shell', grain: [P('grain0'), P('grain1')], annotations: [], grade_points: {}, anchors: {},
    edges: edges([[P('hem'), false, 25], [P('side_r')], [P('top_r')], [P('waist')], [P('top_l'), true], [P('side_l'), true]]) });
}
/** Half front with a button extension, cut as a left/right pair (shirts). */
function shirtFront(d: Draft, dx: number): void {
  bodice(d, 'front_full', 'Front', { length: 'm.garment_length+60', neckDrop: 'm.neck/6+15', dx });
  // Keep only the right half and add a 20 mm button stand at the centre.
  const P = (s: string) => `front.${s}`;
  d.pieces.pop();
  for (const [k, v] of Object.entries(d.drawing)) if (k.startsWith('front_full.')) { delete d.drawing[k]; d.drawing[k.replace('front_full.', 'front.')] = JSON.parse(JSON.stringify(v).replaceAll('front_full.', 'front.')); }
  d.point(P('stand_top'), -20, '(m.garment_length+60)-(m.neck/6+15)', dx); d.point(P('stand_hem'), -20, 0, dx);
  d.line(P('stand'), P('neck_c'), P('stand_top')); d.line(P('stand_edge'), P('stand_top'), P('stand_hem')); d.line(P('hem_half'), P('stand_hem'), P('hem_r'));
  d.point(P('grain0'), '(m.chest+v.ease_chest)/8', 80, dx); d.point(P('grain1'), '(m.chest+v.ease_chest)/8', '(m.garment_length+60)-m.armhole_depth-80', dx);
  for (const s of ['hem', 'hem_l', 'side_l', 'arm_l', 'shoulder_l', 'neckline_l', 'under_l', 'sh_l', 'neck_l', 'arm1_l', 'arm2_l', 'nk1_l', 'nk2_l']) delete d.drawing[P(s)];
  d.piece({ id: 'front', name: 'Front', quantity: 1, mirror_quantity: 1, material: 'shell', grain: [P('grain0'), P('grain1')], annotations: [], grade_points: {}, anchors: {},
    edges: edges([[P('hem_half'), false, 25], [P('side_r')], [P('arm_r'), true], [P('shoulder_r')], [P('neckline_r')], [P('stand')], [P('stand_edge')]]) });
}

const EASE: Record<BlockKind, Record<string, number>> = {
  t_shirt: { ease_chest: 80, ease_bicep: 40 }, shirt: { ease_chest: 120, ease_bicep: 60 }, kurta: { ease_chest: 120, ease_bicep: 60 }, polo: { ease_chest: 90, ease_bicep: 40 },
  trouser: { ease_hip: 40, ease_waist: 20 }, track_pant: { ease_hip: 80, ease_waist: 20 }, a_line_skirt: { ease_hip: 40, ease_waist: 20 },
};
export function blockDocument(block: BlockKind | undefined, chart: SizeChart, name: string, ease: BlockEase = {}): PatternDocument {
  const d = new Draft();
  const sideCheck = (a: string, b: string, ea: number | number[], eb: number | number[], label = 'Side seam'): SeamCheck => ({ name: label, a: { piece: a, edges: [ea].flat() }, b: { piece: b, edges: [eb].flat() }, ease_mm: 0, tolerance_mm: 1 });
  switch (block) {
    case 't_shirt': case 'polo': case 'kurta': {
      const length = block === 'kurta' ? 'm.kurta_length' : 'm.garment_length';
      bodice(d, 'front', 'Front', { length, neckDrop: block === 'polo' ? 'm.neck/6+10' : 'm.neck/6+25', dx: 0 });
      bodice(d, 'back', 'Back', { length, neckDrop: 20, dx: 800 });
      sleeve(d, 'sleeve', { length: block === 'kurta' ? 'm.sleeve_long' : 'm.sleeve_short', hemHalf: block === 'kurta' ? 'm.wrist/2+40' : '(m.bicep+v.ease_bicep)/2-15', dx: 1550 });
      if (block === 'polo') { rect(d, 'collar', 'Collar', 'm.neck+20', 80, { dx: 1900, material: 'rib' }); rect(d, 'placket', 'Placket', 40, 160, { dx: 1900, dy: 200, quantity: 2 }); }
      d.checks.push(sideCheck('front', 'back', 1, 1), sideCheck('front', 'back', 3, 3, 'Shoulder seam'), sideCheck('sleeve', 'sleeve', 1, 4, 'Sleeve underarm seam'));
      break;
    }
    case 'shirt':
      shirtFront(d, 0);
      bodice(d, 'back', 'Back', { length: 'm.garment_length+80', neckDrop: 20, dx: 700 });
      sleeve(d, 'sleeve', { length: 'm.sleeve_long-60', hemHalf: 'm.wrist/2+60', dx: 1450 });
      rect(d, 'collar', 'Collar', 'm.neck+40', 90, { dx: 1800, quantity: 2 }); rect(d, 'cuff', 'Cuff', 'm.wrist+60', 65, { dx: 1800, dy: 200, quantity: 2, mirror: 0 });
      d.checks.push(sideCheck('front', 'back', 1, 1), sideCheck('front', 'back', 3, 3, 'Shoulder seam'));
      break;
    case 'trouser': case 'track_pant': {
      const elastic = block === 'track_pant';
      leg(d, 'front', 'Front', { crotch: 'm.hip/16', waistRise: elastic ? 40 : 0, dx: 0, elastic });
      leg(d, 'back', 'Back', { crotch: 'm.hip/10', waistRise: elastic ? 40 : 30, dx: 700, elastic });
      rect(d, 'waistband', 'Waistband', elastic ? '(m.hip+v.ease_hip)/2' : 'm.waist+v.ease_waist+40', elastic ? 90 : 80, { dx: 1500 });
      d.checks.push(sideCheck('front', 'back', 1, 1, 'Inseam'), sideCheck('front', 'back', [5, 6], [5, 6], 'Side seam'));
      break;
    }
    case 'a_line_skirt':
      skirtPanel(d, 'front', 'Front', 0); skirtPanel(d, 'back', 'Back', 900);
      rect(d, 'waistband', 'Waistband', 'm.waist+v.ease_waist+40', 70, { dx: 1600 });
      d.checks.push(sideCheck('front', 'back', 1, 1));
      break;
    default: break;
  }
  const variables: Record<string, F> = { ...(block ? EASE[block] : {}) };
  if (ease.chest != null) variables.ease_chest = ease.chest;
  if (ease.waist != null) variables.ease_waist = ease.waist;
  if (ease.hip != null) variables.ease_hip = ease.hip;
  if (ease.seam_allowance != null) for (const p of d.pieces) for (const e of p.edges) if (e.allowance === 10) e.allowance = ease.seam_allowance;
  return {
    format: 'beta-pattern.doc/v1', name, sizes: [...chart.sizes], base_size: chart.base_size, measurements: structuredClone(chart.measurements),
    variables, drawing: d.drawing, pieces: d.pieces, rules: {}, groups: {},
    meta: { beta_style_preview: true, block_seams: d.checks as unknown as never, ...(block ? { block } : {}) },
  };
}
