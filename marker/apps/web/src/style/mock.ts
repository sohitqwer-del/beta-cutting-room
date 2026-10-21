import type { BlockInfo, DrawingObject, Hit, Job, Json, MarkerCore, MarkerOverlay, MarkerResult, MatchRule, Measured, OverlayLine, OverlayOutline, PatternCore, PatternDocument,
  PatternInspection, PatternValue, Pt, RepeatBand, SeamCheck, SeamReport, SizeChart, StyleCore, StyleOperation } from '@beta/marker-wasm';
import { BLOCKS, blockDocument, presetChart } from './mock-blocks';
import { inside } from './snap';

/** Test double of the Style engine for unit tests (not used by the app): STYLE-API v1 calls built on the existing Rust Pattern and Marker cores.
 * Rust still evaluates every pattern coordinate (inspect, grade, nest, piece tools, to_job, marker drawing and checks).
 * The TypeScript here only edits the document JSON, sums lengths the core reports, and draws stripe overlays.
 * The app always uses `loadStyleCore()` through adapter.ts. */
export const PREVIEW_ENGINE = 'beta-style-preview';
import type { MatchStatus } from './adapter';
// Hit maths for this test double only (the app asks the Rust hit test).
const nearestOnPolyline = (line: Pt[], at: Pt): { point: Pt; distance: number; along: number } | null => {
  if (!line.length) return null;
  let best: { point: Pt; distance: number; along: number } | null = null, walked = 0;
  if (line.length === 1) return { point: [...line[0]!], distance: Math.hypot(line[0]![0] - at[0], line[0]![1] - at[1]), along: 0 };
  for (let i = 0; i < line.length - 1; i++) {
    const a = line[i]!, b = line[i + 1]!, vx = b[0] - a[0], vy = b[1] - a[1], len2 = vx * vx + vy * vy, len = Math.sqrt(len2);
    const t = len2 > 0 ? Math.max(0, Math.min(1, ((at[0] - a[0]) * vx + (at[1] - a[1]) * vy) / len2)) : 0;
    const p: Pt = [a[0] + t * vx, a[1] + t * vy], d = Math.hypot(p[0] - at[0], p[1] - at[1]);
    if (!best || d < best.distance) best = { point: p, distance: d, along: walked + t * len };
    walked += len;
  }
  return best;
};
const polylineLength = (line: Pt[]): number => line.slice(1).reduce((s, p, i) => s + Math.hypot(p[0] - line[i]![0], p[1] - line[i]![1]), 0);

export type { MatchStatus };
export type OverlayWithMatches = MarkerOverlay & { matches?: MatchStatus[] };

const plus = (v: PatternValue, d: number): PatternValue => {
  if (Math.abs(d) < 1e-9) return v;
  if (typeof v === 'number') return Math.round((v + d) * 1000) / 1000;
  const m = /^\((.*)\)([+-]\d+(?:\.\d+)?)$/.exec(v);
  const base = m ? m[1]! : v, k = Math.round(((m ? Number(m[2]) : 0) + d) * 1000) / 1000;
  return k === 0 ? base : `(${base})${k > 0 ? '+' : '-'}${Math.abs(k)}`;
};
function rewrite<T>(value: T, from: string, to: string): T {
  if (typeof value === 'string') return (value === from ? to : value) as T;
  if (Array.isArray(value)) return value.map(v => rewrite(v, from, to)) as T;
  if (value && typeof value === 'object') return Object.fromEntries(Object.entries(value).map(([k, v]) => [k === from ? to : k, rewrite(v, from, to)])) as T;
  return value;
}
const usesId = (value: unknown, id: string): boolean => JSON.stringify(value).includes(JSON.stringify(id));

export function previewCore(pattern: PatternCore, marker?: MarkerCore): StyleCore {
  const inspectAll = (doc: PatternDocument): Record<string, PatternInspection> => Object.fromEntries(doc.sizes.map(s => [s, pattern.inspect(doc, s)]));
  function validate(doc: PatternDocument): void {
    if (!doc.sizes.includes(doc.base_size)) throw new Error('Choose a base size that is in the size chart.');
    for (const size of doc.sizes) {
      const ins = pattern.inspect(doc, size);
      const bad = ins.pieces.find(p => !p.ok);
      if (bad) throw new Error(`${doc.pieces.find(p => p.id === bad.id)?.name ?? bad.id} (${size}): ${bad.error ?? 'the outline does not close.'}`);
    }
  }
  function apply(doc: PatternDocument, op: StyleOperation): PatternDocument {
    const next = structuredClone(doc);
    switch (op.op) {
      case 'put_object':
        if (next.drawing[op.id] && !op.replace) throw new Error(`The name ${op.id} is already used.`);
        next.drawing[op.id] = op.object; return next;
      case 'delete_object': {
        const { [op.id]: _gone, ...rest } = next.drawing; void _gone;
        const user = Object.entries(rest).find(([, o]) => usesId(o, op.id))?.[0] ?? next.pieces.find(p => usesId(p, op.id))?.name;
        if (user) throw new Error(`${op.id} is used by ${user}. Delete that first.`);
        next.drawing = rest; return next;
      }
      case 'rename_object': {
        if (!next.drawing[op.id]) throw new Error(`${op.id} does not exist.`);
        if (next.drawing[op.new_id]) throw new Error(`The name ${op.new_id} is already used.`);
        const drawing: Record<string, DrawingObject> = {};
        for (const [k, o] of Object.entries(next.drawing)) drawing[k === op.id ? op.new_id : k] = rewrite(o, op.id, op.new_id);
        return { ...next, drawing, pieces: rewrite(next.pieces, op.id, op.new_id) };
      }
      case 'move_point': {
        const o = next.drawing[op.id];
        if (o?.kind !== 'point') throw new Error('Choose a point to move.');
        if (op.preserve_formulas === false) return pattern.move_point(next, op.id, op.size, op.target);
        const at = pattern.inspect(next, op.size).objects[op.id]?.points[0];
        if (!at) throw new Error('This point has no position in this size.');
        const dx = op.target[0] - at[0], dy = op.target[1] - at[1], c = o.operation;
        if (c.op === 'absolute') { c.x = plus(c.x, dx); c.y = plus(c.y, dy); return next; }
        if (c.op === 'polar' && typeof c.distance === 'number' && typeof c.angle === 'number') return pattern.move_point(next, op.id, op.size, op.target);
        throw new Error('This point is built from other points. Use "Free this point" in the inspector to move it by hand.');
      }
      case 'put_piece': {
        const at = next.pieces.findIndex(p => p.id === op.piece.id);
        if (at >= 0 && !op.replace) throw new Error(`A piece called ${op.piece.id} already exists.`);
        if (at >= 0) next.pieces[at] = op.piece; else next.pieces.push(op.piece);
        return next;
      }
      case 'delete_piece': next.pieces = next.pieces.filter(p => p.id !== op.id); return next;
      case 'set_allowance': {
        const p = next.pieces.find(q => q.id === op.piece); if (!p) throw new Error('Choose a piece.');
        for (const e of op.edges) { const edge = p.edges[e]; if (!edge) throw new Error(`Edge ${e + 1} does not exist.`); edge.allowance = op.width; edge.corner = op.corner; }
        return next;
      }
      case 'add_annotation': { const p = next.pieces.find(q => q.id === op.piece); if (!p) throw new Error('Choose a piece.'); (p.annotations ??= []).push(op.annotation); return next; }
      case 'mark_row': {
        const r = op.row, p = next.pieces.find(q => q.id === r.piece); if (!p) throw new Error('Choose a piece.');
        if (!(r.count >= 1 && r.count <= 60)) throw new Error('Use 1 to 60 marks in a row.');
        for (let i = 0; i < r.count; i++) {
          const id = `${r.id}_${i + 1}`;
          next.drawing[id] = { kind: 'point', operation: { op: 'polar', from: r.origin, angle: r.direction, distance: typeof r.spacing === 'number' ? r.spacing * i : `(${r.spacing})*${i}` } };
          if (r.kind === 'button') (p.annotations ??= []).push({ kind: 'button', point: id, diameter: r.length });
          else if (r.kind === 'drill') (p.annotations ??= []).push({ kind: 'points', mark: 'drill', points: [id], text: null });
          else {
            next.drawing[`${id}_end`] = { kind: 'point', operation: { op: 'polar', from: id, angle: r.angle, distance: r.length } };
            (p.annotations ??= []).push({ kind: 'points', mark: 'line', points: [id, `${id}_end`], text: 'buttonhole' });
          }
        }
        return next;
      }
      case 'set_chart':
        if (!op.chart.sizes.length) throw new Error('Keep at least one size.');
        return { ...next, sizes: [...op.chart.sizes], base_size: op.chart.base_size, measurements: structuredClone(op.chart.measurements),
          rules: Object.fromEntries(Object.entries(next.rules ?? {}).map(([k, r]) => [k, { ...r, deltas: Object.fromEntries(op.chart.sizes.map(s => [s, r.deltas[s] ?? [0, 0] as Pt])) }])) };
      case 'set_variable': next.variables[op.name] = op.value; return next;
      case 'set_rule': (next.rules ??= {})[op.id] = op.rule; return next;
      case 'attach_rule': {
        const p = next.pieces.find(q => q.id === op.piece); if (!p) throw new Error('Choose a piece.');
        p.grade_points ??= {};
        if (op.rule) p.grade_points[op.point] = op.rule; else delete p.grade_points[op.point];
        return next;
      }
      case 'offset_piece': case 'join_pieces':
        throw new Error('Offset and join need the new Style engine. Update BETA to use them.');
      case 'piece_action': return pattern.piece_action(next, op.request).document;
    }
  }
  function report(doc: PatternDocument, check: SeamCheck, cache: Record<string, PatternInspection> = inspectAll(doc)): SeamReport {
    const run = (ins: PatternInspection, r: SeamCheck['a']) => {
      const p = ins.pieces.find(q => q.id === r.piece);
      if (!p?.edges) throw new Error(`Piece ${r.piece} is not ready in every size.`);
      const mm = r.edges.reduce((sum, e) => sum + (p.edges![e]?.length_mm ?? NaN), 0);
      if (!Number.isFinite(mm)) throw new Error(`Check the edges chosen for ${r.piece}.`);
      const path = r.edges.flatMap(e => p.graded?.seam_edges?.[e] ?? []);
      return { mm, path: r.reversed ? path.reverse() : path };
    };
    return { check, sampling_mm: 0.1, rows: doc.sizes.map(size => {
      const a = run(cache[size]!, check.a), b = run(cache[size]!, check.b), difference = a.mm - b.mm, error = Math.abs(difference - check.ease_mm);
      return { size, a_mm: a.mm, b_mm: b.mm, difference_mm: difference, error_mm: error, pass: error <= (check.tolerance_mm || 1), a_path: a.path, b_path: b.path };
    }) };
  }
  const blankChart = (): SizeChart => presetChart('indian_adult');
  return {
    blocks: (): BlockInfo[] => structuredClone(BLOCKS),
    size_chart: preset => presetChart(preset),
    create: (request = {}) => {
      const doc = blockDocument(request.block, request.chart ?? blankChart(), request.name?.trim() || 'New style', request.ease);
      validate(doc); return doc;
    },
    edit: (document, operations) => { let next = structuredClone(document); for (const op of operations) next = apply(next, op); validate(next); return next; },
    save: document => JSON.stringify(document, null, 2),
    load: source => {
      const doc = JSON.parse(source) as PatternDocument;
      if (doc?.format !== 'beta-pattern.doc/v1' || !Array.isArray(doc.pieces) || !Array.isArray(doc.sizes)) throw new Error('This is not a BETA pattern file.');
      validate(doc); return doc;
    },
    grade: document => pattern.grade(document),
    hit_test: (document, size, at, radius) => {
      const ins = pattern.inspect(document, size), hits: Hit[] = [];
      for (const [id, o] of Object.entries(ins.objects)) {
        if (o.kind === 'point') { const d = Math.hypot(o.points[0]![0] - at[0], o.points[0]![1] - at[1]); if (d <= radius) hits.push({ id, kind: 'point', point: o.points[0]!, distance_mm: d }); }
        else { const h = nearestOnPolyline(o.points, at); if (h && h.distance <= radius) hits.push({ id, kind: 'path', point: h.point, distance_mm: h.distance }); }
      }
      for (const p of ins.pieces) if (p.graded && inside(p.graded.piece.outline, at)) hits.push({ id: p.id, kind: 'piece', point: at, distance_mm: radius });
      return hits.sort((a, b) => a.distance_mm - b.distance_mm || ['point', 'path', 'piece'].indexOf(a.kind) - ['point', 'path', 'piece'].indexOf(b.kind));
    },
    measure: (document, size, request): Measured => {
      const o = pattern.inspect(document, size).objects, at = (id: string): Pt => { const p = o[id]?.points[0]; if (!p) throw new Error(`${id} has no position.`); return p; };
      if (request.kind === 'length') { const pts = o[request.object]?.points; if (!pts) throw new Error('Choose a line or curve.'); return { value: polylineLength(pts), unit: 'mm', points: pts }; }
      if (request.kind === 'distance') { const a = at(request.a), b = at(request.b); return { value: Math.hypot(b[0] - a[0], b[1] - a[1]), unit: 'mm', points: [a, b] }; }
      const a = at(request.a), v = at(request.vertex), b = at(request.b);
      let deg = (Math.atan2(b[1] - v[1], b[0] - v[0]) - Math.atan2(a[1] - v[1], a[0] - v[0])) * 180 / Math.PI;
      deg = Math.abs(((deg + 540) % 360) - 180);
      return { value: deg, unit: 'deg', points: [a, v, b] };
    },
    seam_report: (document, check) => report(document, check),
    block_seams: document => {
      const checks = (document.meta?.block_seams ?? []) as unknown as SeamCheck[], cache = inspectAll(document);
      return checks.flatMap(c => { try { return [report(document, c, cache)]; } catch { return []; } });
    },
    to_job: (document, settings) => {
      const job = pattern.to_job(document, settings.fabric, settings.ratio);
      job.name = document.name;
      const material = (settings as { material?: string }).material, groups = new Set(document.pieces.map(p => p.material || 'shell'));
      if (!material && groups.size > 1) throw new Error('This style uses more than one fabric. Choose the fabric for this marker.');
      if (material) job.pieces = job.pieces.filter(p => (document.pieces.find(q => p.id === q.id || p.id.startsWith(`${q.id}-`))?.material || 'shell') === material);
      if (!job.pieces.length) throw new Error('No pieces use this fabric.');
      job.fabric = { ...job.fabric, ...settings.fabric };
      job.spacing = { ...settings.spacing };
      const sizes = Object.keys(settings.ratio).filter(s => settings.ratio[s]! > 0);
      const cache = Object.fromEntries(sizes.map(s => [s, pattern.inspect(document, s)]));
      for (const piece of job.pieces) {
        const size = piece.size ?? '', base = size && piece.id.endsWith(`-${size}`) ? piece.id.slice(0, -size.length - 1) : piece.id;
        const extra = settings.piece_settings?.[base];
        if (extra?.buffer != null) piece.buffer = extra.buffer;
        if (extra && 'block_margin' in extra) piece.block_margin = extra.block_margin ?? null;
        const source = document.pieces.find(p => p.id === base), graded = cache[size]?.pieces.find(p => p.id === base)?.graded;
        const anchors = Object.entries(source?.anchors ?? {});
        if (anchors.length) piece.match_points = anchors.flatMap(([name, point]) => {
          const at = graded?.points[point] ?? cache[size]?.objects[point]?.points[0];
          return at ? [{ id: name, point: at }] : [];
        });
      }
      job.match_rules = (settings.matches ?? []).flatMap(rule => sizes.map((s): MatchRule => ({ ...rule, a: { piece_id: `${rule.a.piece_id}-${s}`, point_id: rule.a.point_id }, b: rule.b ? { piece_id: `${rule.b.piece_id}-${s}`, point_id: rule.b.point_id } : null })));
      job.meta = { ...job.meta, beta_style_preview: true, ...(settings.lay ? { beta_style_lay: settings.lay as unknown as Json } : {}), ...(settings.repeat_bands?.length ? { beta_repeat_bands: settings.repeat_bands as unknown as Json } : {}) };
      return job;
    },
    overlay: (job, result, options = {}) => overlay(marker, job, result, options),
  };
}

const DEFAULT_LINE: RepeatBand = { axis: 'x', offset_mm: 0, width_mm: 1, colour: '#1d4f8c' };
/** Cut a straight repeat line with a closed outline: the parts inside (even-odd). */
export function clipLine(outline: Pt[], axis: 'x' | 'y', at: number): [Pt, Pt][] {
  const hits: number[] = [];
  for (let i = 0, j = outline.length - 1; i < outline.length; j = i++) {
    const a = outline[j]!, b = outline[i]!, k: 0 | 1 = axis === 'x' ? 0 : 1, o: 0 | 1 = axis === 'x' ? 1 : 0;
    if ((a[k] > at) !== (b[k] > at)) hits.push(a[o] + (at - a[k]) * (b[o] - a[o]) / (b[k] - a[k]));
  }
  hits.sort((p, q) => p - q);
  const out: [Pt, Pt][] = [];
  for (let i = 0; i + 1 < hits.length; i += 2) out.push(axis === 'x' ? [[at, hits[i]!], [at, hits[i + 1]!]] : [[hits[i]!, at], [hits[i + 1]!, at]]);
  return out;
}
function overlay(marker: MarkerCore | undefined, job: Job, result: MarkerResult, options: { bands?: RepeatBand[]; graded?: Job['pieces']; previous?: { job: Job; result: MarkerResult } }): OverlayWithMatches {
  if (!marker) throw new Error('Marker drawing is not loaded.');
  const outlines = marker.drawing(job, result.placements).outlines;
  const repeat = job.fabric.repeat, length = result.length, width = job.fabric.width;
  const bands = options.bands ?? (job.meta?.beta_repeat_bands as unknown as RepeatBand[] | undefined) ?? [];
  const stripes: OverlayLine[] = [];
  for (const axis of ['x', 'y'] as const) {
    const period = axis === 'x' ? repeat?.period_x : repeat?.period_y; if (!period || period <= 0) continue;
    const offset = axis === 'x' ? repeat?.offset_x ?? 0 : repeat?.offset_y ?? 0, span = axis === 'x' ? length : width;
    if (span / period > 5000) throw new Error('The repeat is too small to draw on this marker.');
    const own = bands.filter(b => b.axis === axis), list = own.length ? own : [{ ...DEFAULT_LINE, axis }];
    for (let k = Math.floor(-offset / period) - 1; offset + k * period <= span + period; k++) {
      for (const band of list) {
        const at = offset + k * period + band.offset_mm; if (at < 0 || at > span) continue;
        outlines.forEach((outline, placement) => {
          for (const points of clipLine(outline, axis, at)) stripes.push({ placement, piece_id: result.placements[placement]!.piece_id, axis, points, colour: band.colour, width_mm: band.width_mm });
        });
      }
    }
  }
  const sizeOf = (id: string) => job.pieces.find(p => p.id === id)?.size ?? null;
  const out: OverlayOutline[] = outlines.map((points, placement) => ({ piece_id: result.placements[placement]!.piece_id, size: sizeOf(result.placements[placement]!.piece_id), points, placement }));
  const previous: MarkerOverlay['previous'] = [];
  if (options.previous) {
    const before = marker.drawing(options.previous.job, options.previous.result.placements).outlines;
    const key = (p: MarkerResult['placements'][number]) => `${p.piece_id}#${p.copy}#${p.mirrored}`;
    const old = new Map(options.previous.result.placements.map((p, i) => [key(p), before[i]!]));
    result.placements.forEach((p, i) => {
      const was = old.get(key(p)); old.delete(key(p));
      const moved = was && was.some((q, k) => Math.hypot(q[0] - (outlines[i]![k]?.[0] ?? NaN), q[1] - (outlines[i]![k]?.[1] ?? NaN)) > 0.5);
      previous.push({ kind: !was ? 'added' : moved || was.length !== outlines[i]!.length ? 'moved' : 'unchanged', piece_id: p.piece_id, before: was ?? [], after: outlines[i]! });
    });
    for (const [k, was] of old) previous.push({ kind: 'removed', piece_id: k.split('#')[0]!, before: was, after: [] });
  }
  const lay = job.meta?.beta_style_lay as { end_allowance_mm?: number } | undefined;
  return { stripes, outlines: out, graded: (options.graded ?? []).map(p => ({ piece_id: p.id, size: p.size ?? null, points: p.outline })), previous,
    spread_length_mm: length + 2 * (lay?.end_allowance_mm ?? 0), matches: matchStatus(marker, job, result) };
}
/** Preview of the per-rule match check: where each match point landed and how far it is from the repeat phase. */
function matchStatus(marker: MarkerCore, job: Job, result: MarkerResult): MatchStatus[] {
  const r = job.fabric.repeat, rules = job.match_rules ?? []; if (!r || !rules.length) return [];
  const tol = r.tolerance_mm || 1, px = r.period_x ?? 0, py = r.period_y ?? 0;
  const placedPoint = (piece: string, point: string, placement: MarkerResult['placements'][number]): Pt | null => {
    const local = job.pieces.find(p => p.id === piece)?.match_points?.find(m => m.id === point)?.point; if (!local) return null;
    return marker.transform([local], placement.mirrored, placement.rotation_deg, placement.translate)[0] ?? null;
  };
  const phase = (d: number, period: number) => { if (!period) return 0; const m = ((d % period) + period) % period; return Math.min(m, period - m); };
  const out: MatchStatus[] = [];
  rules.forEach((rule, index) => {
    const as = result.placements.filter(p => p.piece_id === rule.a.piece_id);
    for (const pa of as) {
      const a = placedPoint(rule.a.piece_id, rule.a.point_id, pa); if (!a) continue;
      const garment = pa.bundle ?? pa.copy;
      let b: Pt | null = null, dx = a[0] - (r.offset_x ?? 0), dy = a[1] - (r.offset_y ?? 0);
      if (rule.b) {
        const pb = result.placements.find(p => p.piece_id === rule.b!.piece_id && (p.bundle ?? p.copy) === garment) ?? result.placements.find(p => p.piece_id === rule.b!.piece_id);
        b = pb ? placedPoint(rule.b.piece_id, rule.b.point_id, pb) : null; if (!b) continue;
        dx = a[0] - b[0] - (rule.offset?.[0] ?? 0); dy = a[1] - b[1] - (rule.offset?.[1] ?? 0);
      }
      const ex = rule.axes === 'y' ? 0 : phase(dx, px), ey = rule.axes === 'x' ? 0 : phase(dy, py), error = Math.hypot(ex, ey);
      out.push({ rule: index, garment: pa.bundle ?? null, a, b, error_mm: error, pass: error <= tol + 1e-6 });
    }
  });
  return out;
}
