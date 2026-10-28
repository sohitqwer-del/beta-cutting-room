// Hero picture: real markers recorded from the browser app while it searched (data/hero-marker.json, made by the
// launch QA capture of the sample tour). Each frame is one "best so far" marker; the last one is the final result.
// Without JavaScript or with reduced motion only the last frame shows. assets/site.js plays the frames in order.
import { esc, raw, type Raw } from './html.ts';

export interface HeroFrame { length_mm: number; pieces: { name: string; size: string; d: string }[] }
export interface HeroData { source: string; recorded: string; width_mm: number; sizes: string[]; efficiency: number | null; frames: HeroFrame[] }

/** Size colours: the same fills and outlines as the app's marker drawing (S, M, L, XL). */
export const SIZE_CLASS = ['hz-0', 'hz-1', 'hz-2', 'hz-3', 'hz-4', 'hz-5'];

/** Validates the recorded file, so a broken capture fails the build instead of the page. */
export function readHero(text: string): HeroData {
  const data = JSON.parse(text) as HeroData;
  if (!Array.isArray(data.frames) || !data.frames.length || !(data.width_mm > 0)) throw new Error('hero-marker.json: no frames');
  for (const f of data.frames) {
    if (!(f.length_mm > 0) || !Array.isArray(f.pieces) || !f.pieces.length) throw new Error('hero-marker.json: empty frame');
    for (const p of f.pieces) if (!/^-?\d+,-?\d+( -?\d+,-?\d+){2,}$/.test(p.d)) throw new Error(`hero-marker.json: bad outline for ${p.name}`);
  }
  for (let i = 1; i < data.frames.length; i++) if (data.frames[i]!.length_mm > data.frames[i - 1]!.length_mm) throw new Error('hero-marker.json: frames must get shorter');
  return data;
}

export function metres(mm: number): string { return `${(mm / 1000).toFixed(2)} m`; }

/** The recorded markers as one SVG: fabric, pieces per frame and a length line. */
export function heroLive(data: HeroData): Raw {
  const W = data.width_mm, L = data.frames[0]!.length_mm, pad = 40, last = data.frames.length - 1;
  const sizeIndex = (s: string) => Math.max(0, data.sizes.indexOf(s));
  const frames = data.frames.map((f, i) => `<g class="hz-frame${i === last ? ' is-final' : ''}" data-frame="${i}" data-length="${f.length_mm}">
<rect class="hz-fabric" x="0" y="0" width="${f.length_mm}" height="${W}"/>
${f.pieces.map((p, k) => `<polygon class="hz-pc ${SIZE_CLASS[sizeIndex(p.size) % SIZE_CLASS.length]}" style="--k:${k}" points="${esc(p.d)}"/>`).join('')}
<path class="hz-end" d="M${f.length_mm} -24 V${W + 24}"/>
</g>`).join('\n');
  const final = data.frames[last]!;
  return raw(`<svg class="hz-svg" viewBox="${-pad} ${-pad} ${L + 2 * pad} ${W + 2 * pad + 70}" role="img" aria-labelledby="hz-title hz-desc">
<title id="hz-title">A real T-shirt marker made by BETA in the browser</title>
<desc id="hz-desc">Sizes ${esc(data.sizes.join(', '))} on fabric ${(W / 10).toFixed(1)} cm wide. While the search ran, the best marker got shorter: ${data.frames.map((f) => metres(f.length_mm)).join(', ')}. Final length ${metres(final.length_mm)}${data.efficiency ? `, ${data.efficiency}% of the fabric in pieces` : ''}.</desc>
<rect class="hz-table" x="0" y="0" width="${L}" height="${W}"/>
${frames}
<g class="hz-dim"><path d="M0 ${W + 40} H${final.length_mm}"/><path d="M0 ${W + 28} V${W + 52} M${final.length_mm} ${W + 28} V${W + 52}"/></g>
</svg>`);
}

/** Size key under the picture. */
export function heroLegend(data: HeroData): Raw {
  return raw(`<ul class="hz-legend" aria-label="Sizes">${data.sizes.map((s, i) => `<li><i class="${SIZE_CLASS[i % SIZE_CLASS.length]}"></i>${esc(s)}</li>`).join('')}</ul>`);
}
