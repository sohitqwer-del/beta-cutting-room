// Social cards for LinkedIn and X: a real app screen (the sample T-shirt marker, public/assets/screens), the logo and
// one line. Writes public/assets/brand/social-card.png (1200 x 630, Open Graph and X) and social-square.png
// (1080 x 1080, LinkedIn and other square posts). Uses a local Chrome or Edge in headless mode; no npm packages.
//
//   node marker/apps/site/scripts/make-social.ts [--out <extra folder for copies>]
import { copyFileSync, mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { markSvg } from '../src/brand.ts';
import { site } from '../src/config.ts';
import { Headless } from './headless.ts';

const args = process.argv.slice(2);
const extra = args.includes('--out') ? args[args.indexOf('--out') + 1] : undefined;
const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
const brandDir = join(root, 'public', 'assets', 'brand');
const file = (p: string) => pathToFileURL(join(root, p)).href;
const css = file('src/style.css');
const font = file('public/assets/fonts/bricolage-grotesque-latin-wght-normal.woff2');
const shot = file('public/assets/screens/path-marker-1200.webp');
const host = (site.siteUrl ?? '').replace(/^https?:\/\//, '').replace(/\/$/, '');

function page(w: number, h: number, square: boolean): string {
  return `<!doctype html><html lang="en"><head><meta charset="utf-8"><link rel="stylesheet" href="${css}">
<style>@font-face{font-family:"Bricolage Grotesque";src:url("${font}") format("woff2");font-weight:200 800}
html,body{margin:0;width:${w}px;height:${h}px;overflow:hidden}*{animation:none!important;transition:none!important}
body{background:linear-gradient(var(--mat-line) 1px,transparent 1px) 0 0/32px 32px,linear-gradient(90deg,var(--mat-line) 1px,transparent 1px) 0 0/32px 32px,radial-gradient(ellipse 70% 90% at 95% 5%,var(--brand-soft),transparent 70%),var(--bg)}
.card{box-sizing:border-box;width:${w}px;height:${h}px;padding:${square ? '72px 72px 64px' : '56px 0 48px 64px'};display:grid;${square ? 'grid-template-rows:auto auto 1fr auto;gap:28px' : 'grid-template-columns:520px 1fr;gap:44px;align-items:center'}}
.brand{display:flex;align-items:center;gap:16px;font-family:var(--display);font-size:${square ? 40 : 32}px;font-weight:500;color:var(--brand);letter-spacing:-.02em;margin-bottom:${square ? 0 : 26}px}
.brand b{color:var(--ink);font-weight:800;letter-spacing:.01em}
h1{font-size:${square ? 76 : 52}px;line-height:1.04;margin:0 0 ${square ? 0 : 20}px;letter-spacing:-.035em}
h1 .hl{color:var(--brand)}
.line{font-size:${square ? 32 : 24}px;line-height:1.3;color:var(--ink-2);margin:${square ? '-8px 0 0' : '0 0 22px'};font-weight:550}
.chips{display:flex;flex-wrap:wrap;gap:10px;list-style:none;padding:0;margin:0 0 20px}.chips li{margin:0;padding:7px 16px;border-radius:999px;background:var(--surface);border:1px solid var(--line);font-size:${square ? 24 : 19}px;font-weight:650;color:var(--ink-2)}
.fun{display:inline-block;padding:8px 16px;border-radius:999px;background:var(--amber);color:#3b2800;font-size:${square ? 24 : 18}px;font-weight:750;margin:0}
.foot{display:flex;align-items:center;justify-content:space-between;gap:16px;flex-wrap:wrap}
.url{font-size:${square ? 24 : 17}px;font-weight:650;color:var(--muted);margin:${square ? 0 : '18px 0 0'}}
.win{background:var(--surface);border:1px solid var(--line);border-radius:${square ? 24 : 20}px 0 0 ${square ? 24 : 20}px;box-shadow:var(--shadow-l);overflow:hidden;${square ? 'border-radius:24px;' : 'height:470px;'}}
.bar{display:flex;align-items:center;gap:8px;padding:12px 16px;border-bottom:1px solid var(--line);background:var(--surface-2);font-size:${square ? 20 : 16}px;font-weight:650;color:var(--ink-2)}
.bar i{width:12px;height:12px;border-radius:50%;background:#ec6a52}.bar i+i{background:#f2b740}.bar i+i+i{background:#3fb27f}.bar span{margin-left:8px}
.bar b{margin-left:auto;font-size:${square ? 18 : 14}px;padding:3px 10px;border-radius:999px;background:var(--ok-soft);color:var(--ok)}
.shotbox{overflow:hidden;position:relative}.shot{display:block;width:${square ? '100%' : '820px'};height:${square ? '430px' : '420px'};object-fit:cover;object-position:${square ? '50% 30%' : '-150px -60px'};${square ? 'transform:scale(1.22);transform-origin:92% 30%' : ''}}
</style></head><body>
<div class="card">
  <div class="copy">
    <div class="brand">${markSvg({ variant: 'tile', size: square ? 84 : 66, detail: true })}<span><b>BETA</b> Cutting Room</span></div>
    ${square ? '' : `<h1>Free, open-source <span class="hl">cutting&#8209;room software</span> for garment factories</h1>
    <p class="line">Patterns, markers and cut plans, in your browser.</p>
    <ul class="chips"><li>No install</li><li>No account</li><li>Apache-2.0</li></ul>
    <p class="fun">A fun project, built with modern AI tools</p>
    <p class="url">${host}</p>`}
  </div>
  ${square ? `<h1>Free, open-source <span class="hl">cutting&#8209;room software</span> for garment factories</h1>
  <p class="line">Patterns, markers and cut plans, in your browser. See a real T&#8209;shirt marker in 60 seconds.</p>` : ''}
  <div class="win"><div class="bar"><i></i><i></i><i></i><span>Sample T-shirt · marker, sizes S–XL</span><b>Real app</b></div><div class="shotbox"><img class="shot" src="${shot}" alt=""></div></div>
  ${square ? `<div class="foot"><ul class="chips" style="margin:0"><li>No install</li><li>No account</li><li>Apache-2.0</li></ul><p class="fun">A fun project, built with modern AI tools</p></div>` : ''}
</div></body></html>`;
}

const tmp = mkdtempSync(join(tmpdir(), 'beta-social-'));
const h = await new Headless().start();
try {
  for (const [name, w, hgt, square] of [['social-card.png', 1200, 630, false], ['social-square.png', 1080, 1080, true]] as const) {
    const html = join(tmp, `${name}.html`);
    writeFileSync(html, page(w, hgt, square));
    const out = join(brandDir, name);
    writeFileSync(out, await h.png(pathToFileURL(html).href, w, hgt, { wait: 500 }));
    if (extra) { mkdirSync(extra, { recursive: true }); copyFileSync(out, join(extra, name)); }
    console.log(`${out} (${w} x ${hgt})`);
  }
} finally { await h.stop(); }
