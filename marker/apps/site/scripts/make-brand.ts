// Generates every brand file from src/brand.ts: SVG marks and wordmarks, favicons, PNG app
// icons, the Windows ICO and the GitHub social preview. Run from the repository root:
//
//   npm pack @fontsource/bricolage-grotesque@5.3.0   (in an empty folder outside git, then extract)
//   node marker/apps/site/scripts/make-brand.ts --font-dir <extracted>/package/files [--tmp <dir>]
//
// The wordmark letters are outlined from Bricolage Grotesque (SIL OFL 1.1), so the SVGs need
// no font. Uses a local Chrome or Edge in headless mode; no npm packages.
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { BRAND, markSvg } from '../src/brand.ts';
import { heroMarker } from '../src/illustrations.ts';
import { loadFont, textPath } from './ttf-outline.ts';
import { Headless } from './headless.ts';

const args = process.argv.slice(2);
const opt = (name: string) => { const i = args.indexOf(name); return i >= 0 ? args[i + 1] : undefined; };
const fontDir = opt('--font-dir');
if (!fontDir) { console.error('Pass --font-dir <folder with bricolage-grotesque-latin-800-normal.woff and -500->'); process.exit(1); }

const site = resolve(fileURLToPath(new URL('..', import.meta.url)));
const apps = resolve(site, '..');
const brandDir = join(site, 'public', 'assets', 'brand');
mkdirSync(brandDir, { recursive: true });
const tmp = opt('--tmp') ?? mkdtempSync(join(tmpdir(), 'beta-brand-'));
mkdirSync(tmp, { recursive: true });

const heavy = loadFont(readFileSync(join(fontDir, 'bricolage-grotesque-latin-800-normal.woff')));
const medium = loadFont(readFileSync(join(fontDir, 'bricolage-grotesque-latin-500-normal.woff')));

// ---- Wordmark: [mark] BETA Cutting Room, on a 64-unit-high canvas ----
const KERN = { TA: -45, AT: -45, Ro: -10 };
function wordmark(kind: 'colour' | 'mono' | 'white'): string {
  const size = 38; const baseline = 45.5; const x0 = 80;
  const beta = textPath(heavy, 'BETA', size, x0, baseline, { tracking: 10, kern: KERN });
  const room = textPath(medium, 'Cutting Room', size, x0 + beta.width + 11, baseline, { kern: KERN });
  const width = Math.ceil(x0 + beta.width + 11 + room.width + 2);
  const ink = kind === 'white' ? BRAND.white : BRAND.ink;
  const second = kind === 'colour' ? BRAND.green : ink;
  const mark = markSvg({ variant: kind === 'colour' ? 'tile' : kind === 'mono' ? 'mono' : 'mono-white', size: 64, detail: kind === 'colour' })
    .replace('<svg', '<svg x="0" y="0"').replace(/ aria-hidden="true" focusable="false"/, '');
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${width} 64" width="${width * 2}" height="128" role="img" aria-label="BETA Cutting Room">
<title>BETA Cutting Room</title>
${mark}
<path fill="${ink}" d="${beta.d}"/>
<path fill="${second}" d="${room.d}"/>
</svg>
`;
}

const files: Record<string, string> = {
  'logo-mark.svg': markSvg({ variant: 'tile', size: 512, detail: true, standalone: true, label: 'BETA Cutting Room logo' }),
  'logo-mark-colour.svg': markSvg({ variant: 'colour', size: 512, detail: true, standalone: true, label: 'BETA Cutting Room logo' }),
  'logo-mark-mono.svg': markSvg({ variant: 'mono', size: 512, standalone: true, label: 'BETA Cutting Room logo' }),
  'logo-mark-mono-white.svg': markSvg({ variant: 'mono-white', size: 512, standalone: true, label: 'BETA Cutting Room logo' }),
  'logo-wordmark.svg': wordmark('colour'),
  'logo-wordmark-mono.svg': wordmark('mono'),
  'logo-wordmark-white.svg': wordmark('white'),
  'favicon.svg': markSvg({ variant: 'tile', size: 32, standalone: true }),
};
for (const [name, svg] of Object.entries(files)) writeFileSync(join(brandDir, name), svg.endsWith('\n') ? svg : `${svg}\n`);
copyFileSync(join(brandDir, 'favicon.svg'), join(site, 'public', 'assets', 'favicon.svg'));
copyFileSync(join(brandDir, 'favicon.svg'), join(apps, 'web', 'public', 'favicon.svg'));

// ---- PNGs ----
const page = (body: string, w: number, h: number) => `data:text/html;base64,${Buffer.from(`<!doctype html><html><head><style>html,body{margin:0;width:${w}px;height:${h}px;overflow:hidden;background:transparent}svg{display:block}</style></head><body>${body}</body></html>`).toString('base64')}`;
const icon = (size: number) => page(markSvg({ variant: 'tile', size, detail: size >= 48 }), size, size);

/** Windows ICO with PNG entries (supported since Windows Vista). */
function ico(images: { size: number; png: Buffer }[]): Buffer {
  const head = Buffer.alloc(6 + 16 * images.length);
  head.writeUInt16LE(0, 0); head.writeUInt16LE(1, 2); head.writeUInt16LE(images.length, 4);
  let offset = head.length;
  images.forEach((im, i) => {
    const o = 6 + 16 * i;
    head.writeUInt8(im.size >= 256 ? 0 : im.size, o); head.writeUInt8(im.size >= 256 ? 0 : im.size, o + 1);
    head.writeUInt8(0, o + 2); head.writeUInt8(0, o + 3); head.writeUInt16LE(1, o + 4); head.writeUInt16LE(32, o + 6);
    head.writeUInt32LE(im.png.length, o + 8); head.writeUInt32LE(offset, o + 12);
    offset += im.png.length;
  });
  return Buffer.concat([head, ...images.map((im) => im.png)]);
}

const h = await new Headless().start();
try {
  writeFileSync(join(brandDir, 'favicon-32.png'), await h.png(icon(32), 32, 32, { transparent: true }));
  writeFileSync(join(brandDir, 'apple-touch-icon.png'), await h.png(page(`<div style="width:180px;height:180px;background:${BRAND.green}">${markSvg({ variant: 'tile', size: 180, detail: true })}</div>`, 180, 180), 180, 180));
  writeFileSync(join(brandDir, 'icon-512.png'), await h.png(icon(512), 512, 512, { transparent: true }));

  const icoSizes = [16, 24, 32, 48, 64, 128, 256];
  const images = [];
  for (const size of icoSizes) images.push({ size, png: await h.png(icon(size), size, size, { transparent: true }) });
  const iconsDir = join(apps, 'desktop', 'src-tauri', 'icons');
  writeFileSync(join(iconsDir, 'icon.ico'), ico(images));
  writeFileSync(join(iconsDir, 'icon.png'), images.find((i) => i.size === 256)!.png);

  // Social preview, 1280 x 640, drawn with the site stylesheet and font.
  const css = pathToFileURL(join(site, 'src', 'style.css')).href;
  const font = pathToFileURL(join(site, 'public', 'assets', 'fonts', 'bricolage-grotesque-latin-wght-normal.woff2')).href;
  const social = `<!doctype html><html lang="en"><head><meta charset="utf-8"><link rel="stylesheet" href="${css}">
<style>@font-face{font-family:"Bricolage Grotesque";src:url("${font}") format("woff2");font-weight:200 800}
html,body{margin:0;width:1280px;height:640px;overflow:hidden}*{animation:none!important;transition:none!important}
body.social{background:linear-gradient(var(--mat-line) 1px,transparent 1px) 0 0/32px 32px,linear-gradient(90deg,var(--mat-line) 1px,transparent 1px) 0 0/32px 32px,radial-gradient(ellipse 70% 90% at 95% 10%,var(--brand-soft),transparent 70%),var(--bg)}
.social-card{box-sizing:border-box;width:1280px;height:640px;padding:64px 64px 56px 72px;display:grid;grid-template-columns:560px 1fr;gap:40px;align-items:center}
.social-brand{display:flex;align-items:center;gap:18px;font-family:var(--display);font-size:34px;font-weight:500;color:var(--brand);letter-spacing:-.02em;margin-bottom:30px}
.social-brand .brand-name{font-size:34px}
.social h1{font-size:54px;line-height:1.05;margin:0 0 22px;letter-spacing:-.035em}
.social-copy p{font-size:23px;color:var(--ink-2);margin:0 0 22px;font-weight:550}
.social-chips{display:flex;gap:10px;list-style:none;padding:0;margin:0 0 26px}.social-chips li{margin:0;padding:6px 16px;border-radius:999px;background:var(--surface);border:1px solid var(--line);font-size:19px;font-weight:650;color:var(--ink-2)}
.social-copy .social-fun{display:inline-block;padding:8px 16px;border-radius:999px;background:var(--amber);color:#3b2800;font-size:19px;font-weight:750;margin:0}
.social-art{background:var(--surface);border:1px solid var(--line);border-radius:22px;padding:22px;box-shadow:var(--shadow-l)}</style></head>
<body class="social">
<div class="social-card">
  <div class="social-copy">
    <div class="social-brand">${markSvg({ variant: 'tile', size: 72, detail: true })}<span class="brand-name"><b>BETA</b> Cutting Room</span></div>
    <h1>Open-source cutting&#8209;room software for garment factories</h1>
    <p>Pattern · Marker · Cut plan · Rolls · Spread · Bundles</p>
    <ul class="social-chips"><li>Self-hosted</li><li>Works offline</li><li>Apache-2.0</li></ul>
    <p class="social-fun">A fun project, built with the help of modern AI tools</p>
  </div>
  <div class="social-art">${heroMarker().value}</div>
</div>
</body></html>`;
  const socialFile = join(tmp, 'social.html');
  writeFileSync(socialFile, social);
  writeFileSync(join(brandDir, 'social-preview.png'), await h.png(pathToFileURL(socialFile).href, 1280, 640, { wait: 400 }));
} finally {
  await h.stop();
}
console.log(`Brand files written to ${brandDir}, desktop icons updated.`);
