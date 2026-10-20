// Minimal TrueType reader for the brand script: turns text into SVG path data.
// Reads WOFF 1.0 or TTF files with glyf outlines (no hinting, no GPOS; kerning is passed in).
// Used only at design time to outline the wordmark, so the logo needs no font at runtime.
import { inflateSync } from 'node:zlib';

interface Font { unitsPerEm: number; ascender: number; cmap: Map<number, number>; advance: number[]; glyph: (id: number) => Contour[] }
type Pt = { x: number; y: number; on: boolean };
type Contour = Pt[];

function tables(buf: Buffer): Map<string, Buffer> {
  const out = new Map<string, Buffer>();
  if (buf.toString('latin1', 0, 4) === 'wOFF') {
    const n = buf.readUInt16BE(12);
    for (let i = 0, o = 44; i < n; i++, o += 20) {
      const tag = buf.toString('latin1', o, o + 4);
      const off = buf.readUInt32BE(o + 4); const comp = buf.readUInt32BE(o + 8); const orig = buf.readUInt32BE(o + 12);
      const raw = buf.subarray(off, off + comp);
      out.set(tag, comp < orig ? inflateSync(raw) : Buffer.from(raw));
    }
    return out;
  }
  const n = buf.readUInt16BE(4);
  for (let i = 0, o = 12; i < n; i++, o += 16) {
    out.set(buf.toString('latin1', o, o + 4), buf.subarray(buf.readUInt32BE(o + 8), buf.readUInt32BE(o + 8) + buf.readUInt32BE(o + 12)));
  }
  return out;
}

function need(t: Map<string, Buffer>, tag: string): Buffer {
  const b = t.get(tag);
  if (!b) throw new Error(`Font has no ${tag} table (CFF fonts are not supported)`);
  return b;
}

export function loadFont(buf: Buffer): Font {
  const t = tables(buf);
  const head = need(t, 'head'); const maxp = need(t, 'maxp'); const hhea = need(t, 'hhea');
  const hmtx = need(t, 'hmtx'); const loca = need(t, 'loca'); const glyf = need(t, 'glyf'); const cmapT = need(t, 'cmap');
  const unitsPerEm = head.readUInt16BE(18); const longLoca = head.readInt16BE(50) === 1;
  const numGlyphs = maxp.readUInt16BE(4); const numH = hhea.readUInt16BE(34);
  const ascender = hhea.readInt16BE(4);
  const advance: number[] = [];
  for (let i = 0; i < numGlyphs; i++) advance.push(hmtx.readUInt16BE(4 * Math.min(i, numH - 1)));
  const offset = (i: number) => (longLoca ? loca.readUInt32BE(4 * i) : loca.readUInt16BE(2 * i) * 2);

  // cmap format 4 (Windows Unicode BMP).
  const cmap = new Map<number, number>();
  const nSub = cmapT.readUInt16BE(2);
  for (let i = 0; i < nSub; i++) {
    const pid = cmapT.readUInt16BE(4 + 8 * i); const eid = cmapT.readUInt16BE(6 + 8 * i); const so = cmapT.readUInt32BE(8 + 8 * i);
    if (!(pid === 3 && eid === 1) || cmapT.readUInt16BE(so) !== 4) continue;
    const segX2 = cmapT.readUInt16BE(so + 6); const seg = segX2 / 2;
    const ends = so + 14; const starts = ends + segX2 + 2; const deltas = starts + segX2; const ranges = deltas + segX2;
    for (let s = 0; s < seg; s++) {
      const end = cmapT.readUInt16BE(ends + 2 * s); const start = cmapT.readUInt16BE(starts + 2 * s);
      const delta = cmapT.readInt16BE(deltas + 2 * s); const ro = cmapT.readUInt16BE(ranges + 2 * s);
      for (let c = start; c <= end && c !== 0xffff; c++) {
        let g: number;
        if (ro === 0) g = (c + delta) & 0xffff;
        else { const addr = ranges + 2 * s + ro + 2 * (c - start); g = cmapT.readUInt16BE(addr); if (g) g = (g + delta) & 0xffff; }
        cmap.set(c, g);
      }
    }
  }

  function glyph(id: number): Contour[] {
    const start = offset(id); const end = offset(id + 1);
    if (end <= start) return [];
    const g = glyf.subarray(start, end);
    const nc = g.readInt16BE(0);
    if (nc < 0) {
      // Compound glyph: offsets only (enough for i, j and accents).
      const out: Contour[] = []; let p = 10; let more = true;
      while (more) {
        const flags = g.readUInt16BE(p); const gi = g.readUInt16BE(p + 2); p += 4;
        let dx: number; let dy: number;
        if (flags & 1) { dx = g.readInt16BE(p); dy = g.readInt16BE(p + 2); p += 4; } else { dx = g.readInt8(p); dy = g.readInt8(p + 1); p += 2; }
        if (flags & 8) p += 2; else if (flags & 0x40) p += 4; else if (flags & 0x80) p += 8;
        for (const c of glyph(gi)) out.push(c.map((q) => ({ x: q.x + dx, y: q.y + dy, on: q.on })));
        more = (flags & 0x20) !== 0;
      }
      return out;
    }
    const endPts: number[] = [];
    for (let i = 0; i < nc; i++) endPts.push(g.readUInt16BE(10 + 2 * i));
    const n = (endPts[nc - 1] ?? -1) + 1;
    let p = 10 + 2 * nc; p += 2 + g.readUInt16BE(p);
    const flags: number[] = [];
    while (flags.length < n) { const f = g.readUInt8(p++); flags.push(f); if (f & 8) { let r = g.readUInt8(p++); while (r--) flags.push(f); } }
    const read = (short: number, same: number) => {
      const vals: number[] = []; let v = 0;
      for (const f of flags) {
        if (f & short) { const d = g.readUInt8(p++); v += f & same ? d : -d; } else if (!(f & same)) { v += g.readInt16BE(p); p += 2; }
        vals.push(v);
      }
      return vals;
    };
    const xs = read(2, 16); const ys = read(4, 32);
    const contours: Contour[] = []; let s = 0;
    for (const e of endPts) { const c: Contour = []; for (let i = s; i <= e; i++) c.push({ x: xs[i]!, y: ys[i]!, on: (flags[i]! & 1) === 1 }); contours.push(c); s = e + 1; }
    return contours;
  }
  return { unitsPerEm, ascender, cmap, advance, glyph };
}

const r = (n: number) => Math.round(n * 100) / 100;

/** SVG path data for one contour (quadratic B-splines with implied on-curve points). */
function contourPath(c: Contour, map: (p: { x: number; y: number }) => { x: number; y: number }): string {
  if (!c.length) return '';
  // Rotate so the contour starts on an on-curve point (insert one if all points are off-curve).
  const i0 = c.findIndex((q) => q.on);
  let pts: Pt[];
  if (i0 < 0) { const a = c[0]!; const b = c[1 % c.length]!; pts = [{ x: (a.x + b.x) / 2, y: (a.y + b.y) / 2, on: true }, ...c.slice(1), a]; }
  else pts = [...c.slice(i0), ...c.slice(0, i0)];
  const seq = pts.slice(1);
  const m = map(pts[0]!);
  let d = `M${r(m.x)} ${r(m.y)}`;
  let ctrl: { x: number; y: number } | null = null;
  for (const q of seq) {
    if (q.on) {
      const e = map(q);
      if (ctrl) { const k = map(ctrl); d += `Q${r(k.x)} ${r(k.y)} ${r(e.x)} ${r(e.y)}`; ctrl = null; } else d += `L${r(e.x)} ${r(e.y)}`;
    } else if (ctrl) {
      const mid = { x: (ctrl.x + q.x) / 2, y: (ctrl.y + q.y) / 2 };
      const k = map(ctrl); const e = map(mid);
      d += `Q${r(k.x)} ${r(k.y)} ${r(e.x)} ${r(e.y)}`; ctrl = q;
    } else ctrl = q;
  }
  if (ctrl) { const k = map(ctrl); d += `Q${r(k.x)} ${r(k.y)} ${r(m.x)} ${r(m.y)}`; }
  return `${d}Z`;
}

/**
 * Lays out `text` at `size` px with its baseline at (x, y). `tracking` is extra space per
 * letter in font units; `kern` maps letter pairs such as "TA" to a font-unit adjustment.
 * Returns path data and the advance width in px.
 */
export function textPath(font: Font, text: string, size: number, x: number, y: number, opts: { tracking?: number; kern?: Record<string, number> } = {}): { d: string; width: number } {
  const scale = size / font.unitsPerEm;
  let pen = 0; const parts: string[] = [];
  for (let i = 0; i < text.length; i++) {
    const ch = text[i]!; const gid = font.cmap.get(ch.codePointAt(0)!) ?? 0;
    const ox = x + pen * scale;
    for (const c of font.glyph(gid)) parts.push(contourPath(c, (q) => ({ x: ox + q.x * scale, y: y - q.y * scale })));
    pen += font.advance[gid]! + (opts.tracking ?? 0) + (opts.kern?.[text.slice(i, i + 2)] ?? 0);
  }
  return { d: parts.join(''), width: (pen - (opts.tracking ?? 0)) * scale };
}
