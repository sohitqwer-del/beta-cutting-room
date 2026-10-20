// Title cards, the "basics" explainer and backgrounds for the demo video, as self-contained HTML.
// Every drawing is original SVG (Apache-2.0, like BETA). No network: fonts and logos are inlined.
// A card page exposes window.beat(n): the recorder calls it when narration line n starts.
// CSS then shows step n (body[data-beat="n"]) and keeps earlier steps (body.b1, .b2, ...).
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { REPO_SHORT, SITE_SHORT } from './scenes.mjs';

const site = fileURLToPath(new URL('../../apps/site/public/assets/', import.meta.url));
const read = (p) => readFileSync(site + p, 'utf8');
const font = readFileSync(site + 'fonts/bricolage-grotesque-latin-wght-normal.woff2').toString('base64');
const stripXml = (svg) => svg.replace(/<\?xml[^>]*>/, '').replace(/\s(width|height)="[^"]*"/, '');
export const LOGO = stripXml(read('brand/logo-mark.svg'));
export const WORDMARK = stripXml(read('brand/logo-wordmark.svg'));

export const C = { green: '#176b52', green2: '#0f5440', ink: '#10231c', amber: '#f4b740', mint: '#c9ecd9', paper: '#fbfaf6', blue: '#c7d7f2', red: '#c2412d', muted: '#4c5f57' };

const baseCss = `
@font-face { font-family: 'Bricolage'; src: url(data:font/woff2;base64,${font}) format('woff2'); font-weight: 200 800; }
* { box-sizing: border-box; margin: 0; padding: 0; }
html, body { width: 100%; height: 100%; overflow: hidden; }
body { background: ${C.paper}; color: ${C.ink}; font: 500 34px/1.3 'Segoe UI', 'Nirmala UI', system-ui, sans-serif; }
h1, h2, h3, .display { font-family: 'Bricolage', 'Segoe UI', sans-serif; font-weight: 750; letter-spacing: -0.02em; line-height: 1.05; }
.ease, .ease * { transition-timing-function: cubic-bezier(.2,.8,.2,1); }
.fade { opacity: 0; transform: translateY(24px); transition: opacity .7s, transform .7s cubic-bezier(.2,.8,.2,1); }
.pill { display: inline-flex; align-items: center; gap: 12px; padding: 14px 28px; border-radius: 999px; background: #fff; border: 3px solid ${C.green}; color: ${C.green2}; font-weight: 700; font-size: 36px; }
.pill.solid { background: ${C.green}; color: #fff; }
.badge { display: inline-block; padding: 6px 18px; border-radius: 10px; background: ${C.amber}; color: ${C.ink}; font-weight: 800; font-size: 30px; letter-spacing: .04em; text-transform: uppercase; }
.logo svg { width: 100%; height: 100%; display: block; }
.selvedge { stroke: ${C.green}; stroke-width: 3; stroke-dasharray: 10 8; }
.fabric { fill: #eef4ef; stroke: none; }
.piece { stroke: ${C.ink}; stroke-width: 2.5; stroke-linejoin: round; }
.p-front { fill: ${C.blue}; } .p-back { fill: ${C.mint}; } .p-sleeve { fill: #f9e2a6; } .p-collar { fill: #f3c9bd; }
.plabel { font: 700 26px 'Segoe UI', sans-serif; fill: ${C.ink}; text-anchor: middle; dominant-baseline: middle; }
.grain { stroke: ${C.green}; stroke-width: 2.5; marker-end: none; }
`;

/** A full HTML page for one card. */
export function page({ w = 1920, h = 1080, css = '', body = '', lang = 'en' }) {
  return `<!doctype html><html lang="${lang}"><head><meta charset="utf-8"><title>card</title>
<style>${baseCss} body { width: ${w}px; height: ${h}px; } ${css}</style></head><body data-beat="0">${body}
<script>
window.beat = (n) => { document.body.dataset.beat = String(n); for (let i = 1; i <= n; i++) document.body.classList.add('b' + i); };
</script></body></html>`;
}

// ---------- Pattern pieces (original simple shirt blocks, in px) ----------
// Each piece is drawn around its own origin so CSS can move it freely.
const PIECES = {
  front: { cls: 'p-front', w: 220, h: 300, d: 'M0 0 H120 Q135 55 220 62 V300 H0 Z', label: 'Front' },
  back: { cls: 'p-back', w: 240, h: 300, d: 'M0 0 H175 L240 55 V300 H0 Z', label: 'Back' },
  sleeve: { cls: 'p-sleeve', w: 250, h: 190, d: 'M0 50 Q125 -30 250 50 L222 190 H28 Z', label: 'Sleeve' },
  collar: { cls: 'p-collar', w: 230, h: 56, d: 'M0 10 Q115 -8 230 10 V46 Q115 64 0 46 Z', label: 'Collar' },
};
function piece(id, kind, extra = '') {
  const p = PIECES[kind];
  return `<g id="${id}" class="pc ${extra}"><path class="piece ${p.cls}" d="${p.d}"/><line class="grain" x1="${p.w * 0.22}" y1="${p.h * 0.25}" x2="${p.w * 0.22}" y2="${p.h * 0.75}"/><text class="plabel" x="${p.w / 2}" y="${p.h * 0.5 + (kind === 'collar' ? 0 : 18)}">${p.label}</text></g>`;
}

// ---------- Hook ----------
export function hookCard({ vertical = false } = {}) {
  const w = vertical ? 1080 : 1920, h = vertical ? 1920 : 1080;
  const css = `
.wrap { position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 46px; padding: ${vertical ? '0 0 420px' : '90px 0 190px'}; text-align: center; }
.logo { width: ${vertical ? 260 : 230}px; height: ${vertical ? 260 : 230}px; transform: scale(.6) rotate(-8deg); opacity: 0; transition: transform 1s cubic-bezier(.2,1.4,.3,1), opacity .5s; filter: drop-shadow(0 18px 30px rgba(16,35,28,.18)); }
body.b1 .logo { transform: none; opacity: 1; }
h1 { font-size: ${vertical ? 112 : 124}px; max-width: ${vertical ? 940 : 1500}px; }
h1 .hl { background: linear-gradient(transparent 62%, ${C.amber} 62%, ${C.amber} 92%, transparent 92%); padding: 0 8px; }
body.b1 h1 { opacity: 1; transform: none; }
.pills { display: flex; gap: 22px; flex-wrap: wrap; justify-content: center; max-width: ${vertical ? 900 : 1600}px; }
.pills .pill { opacity: 0; transform: translateY(20px) scale(.95); transition: opacity .6s, transform .6s cubic-bezier(.2,1.3,.3,1); }
body.b2 .pills .pill { opacity: 1; transform: none; }
body.b2 .pills .pill:nth-child(2) { transition-delay: .25s; } body.b2 .pills .pill:nth-child(3) { transition-delay: .5s; }
.strip { position: absolute; left: 0; right: 0; ${vertical ? 'top: 90px' : 'top: 34px'}; height: 56px; display: flex; gap: 14px; padding: 0 30px; opacity: .55; }
.strip i { flex: 0 0 auto; height: 56px; border: 2.5px solid ${C.ink}; border-radius: 6px; background: ${C.mint}; transform: translateX(-140px); opacity: 0; transition: transform 1.2s cubic-bezier(.2,.8,.2,1), opacity .8s; }
.strip i:nth-child(3n) { background: ${C.blue}; } .strip i:nth-child(3n+1) { background: #f9e2a6; }
body.b1 .strip i { transform: none; opacity: 1; }
.word { font-family: 'Bricolage'; font-weight: 700; font-size: 46px; color: ${C.green2}; opacity: 0; transition: opacity .8s .4s; }
body.b1 .word { opacity: 1; }
`;
  const strip = Array.from({ length: vertical ? 9 : 16 }, (_, i) => `<i style="width:${70 + ((i * 37) % 90)}px;transition-delay:${(i * 0.06).toFixed(2)}s"></i>`).join('');
  const body = `<div class="strip">${strip}</div><div class="wrap">
  <div class="logo">${LOGO}</div>
  <p class="word">BETA Cutting Room</p>
  <h1 class="fade">Save fabric on <span class="hl">every marker</span>.</h1>
  <div class="pills"><span class="pill solid">Free</span><span class="pill">Open source</span><span class="pill">Runs in your factory</span></div>
</div>`;
  return page({ w, h, css, body });
}

// ---------- Basics: pattern piece, marker, shorter = cheaper, CAD ----------
export function basicsCard() {
  const css = `
.title { position: absolute; left: 110px; top: 70px; right: 110px; display: flex; align-items: baseline; gap: 26px; }
.title .kicker { font: 800 30px 'Segoe UI'; letter-spacing: .12em; text-transform: uppercase; color: ${C.green}; }
.title h2 { font-size: 72px; }
.steps { position: absolute; left: 110px; top: 165px; display: flex; gap: 14px; }
.steps span { width: 64px; height: 10px; border-radius: 6px; background: #d6e2dc; transition: background .5s; }
${[1, 2, 3, 4, 5, 6].map((n) => `body.b${n} .steps span:nth-child(${n}) { background: ${C.green}; }`).join('\n')}
.head { position: absolute; left: 110px; top: 200px; font-size: 58px; font-family: 'Bricolage'; font-weight: 700; opacity: 0; transition: opacity .5s; }
${[1, 2, 3, 4, 5, 6].map((n) => `body[data-beat="${n}"] .head-${n} { opacity: 1; }`).join('\n')}
svg.stage { position: absolute; left: 0; top: 0; width: 1920px; height: 1080px; overflow: visible; }
.pc { transition: transform 1.3s cubic-bezier(.2,.8,.2,1), opacity .6s; }
.layer { transition: opacity .6s; opacity: 0; }
/* Step 1: a shirt, then its flat pieces */
#shirt { transform: translate(710px, 330px) scale(1.1); opacity: 0; }
body.b1 #shirt { animation: shirt 2.2s forwards; }
@keyframes shirt { 0% { opacity: 0; } 15% { opacity: 1; } 60% { opacity: 1; } 100% { opacity: 0; } }
body[data-beat="1"] .pc { transition: transform 1.3s cubic-bezier(.2,.8,.2,1), opacity .7s 1.5s; }
#a-front { transform: translate(560px, 400px); } #a-back { transform: translate(840px, 400px); }
#a-sleeve { transform: translate(1150px, 420px); } #a-collar { transform: translate(1150px, 660px); }
#a-front2 { transform: translate(560px, 400px); opacity: 0; } #a-sleeve2 { transform: translate(1150px, 420px); opacity: 0; }
body[data-beat="0"] .pc { opacity: 0; }
.tags text { font: 700 30px 'Segoe UI'; fill: ${C.green2}; }
/* Step 2: pieces on the fabric (loose) */
body.b2 #a-front { transform: translate(250px, 420px); } body.b2 #a-front2 { transform: translate(1000px, 420px) ; opacity: 1; }
body.b2 #a-back { transform: translate(560px, 420px); } body.b2 #a-sleeve { transform: translate(1310px, 420px); }
body.b2 #a-sleeve2 { transform: translate(1310px, 600px) ; opacity: 1; } body.b2 #a-collar { transform: translate(1290px, 470px) rotate(90deg); }
body.b2 .layer-fabric { opacity: 1; }
/* Step 3: tight layout = shorter */
body.b3 #a-front { transform: translate(250px, 420px); } body.b3 #a-back { transform: translate(480px, 420px); }
body.b3 #a-front2 { transform: translate(730px, 420px); } body.b3 #a-sleeve { transform: translate(960px, 420px); }
body.b3 #a-sleeve2 { transform: translate(960px, 540px); } body.b3 #a-collar { transform: translate(1270px, 420px) rotate(90deg); }
body.b3 .layer-short { opacity: 1; }
.waste { fill: url(#hatch); stroke: ${C.red}; stroke-width: 2; opacity: .9; transition: opacity .6s; }
.wlabel { font: 800 26px 'Segoe UI'; fill: ${C.red}; text-anchor: middle; transition: opacity .6s; }
body.b3 .wlabel { opacity: 0; }
body.b3 .waste { opacity: 0; }
#fab-end { transition: transform 1.3s cubic-bezier(.2,.8,.2,1); }
body.b3 #fab-end { transform: translateX(-300px); }
#fab-rect { transition: width 1.3s cubic-bezier(.2,.8,.2,1); }
body.b3 #fab-rect { width: 1150px; }
.dim text { font: 800 34px 'Segoe UI'; fill: ${C.ink}; }
.saved text { font: 800 34px 'Segoe UI'; fill: ${C.green2}; }
/* Steps 4-5: money example */
.money { position: absolute; left: 110px; right: 110px; top: 290px; height: 540px; align-items: start; display: grid; grid-template-columns: 1fr 1fr; gap: 50px; opacity: 0; transition: opacity .6s; pointer-events: none; }
body.b4 .money { opacity: 1; } body.b6 .money { opacity: 0; }
body.b4 .stage .pc, body.b4 .layer { opacity: 0 !important; }
.calc { background: #fff; border: 3px solid #d6e2dc; border-radius: 28px; padding: 30px 44px; box-shadow: 0 14px 40px rgba(16,35,28,.08); }
.calc .badge { margin-bottom: 26px; }
.row { display: flex; align-items: baseline; gap: 20px; font-size: 46px; font-weight: 700; margin: 4px 0; opacity: 0; transform: translateX(-20px); transition: opacity .5s, transform .5s; }
.row b { font-family: 'Bricolage'; font-size: 66px; color: ${C.green2}; }
.row small { font-size: 34px; color: ${C.muted}; font-weight: 600; }
body.b4 .r1 { opacity: 1; transform: none; } body.b4 .r2 { opacity: 1; transform: none; transition-delay: .9s; } body.b4 .r3 { opacity: 1; transform: none; transition-delay: 1.8s; }
body.b5 .r4 { opacity: 1; transform: none; } body.b5 .r5 { opacity: 1; transform: none; transition-delay: 1s; }
.r5 b { color: #fff; background: ${C.green}; padding: 4px 22px; border-radius: 16px; }
.plies { position: relative; height: 540px; }
.plies i { position: absolute; left: 40px; width: 560px; height: 22px; border-radius: 6px; background: ${C.mint}; border: 2px solid ${C.green}; transform: translateY(60px); opacity: 0; transition: transform .5s, opacity .5s; }
body.b4 .plies i { transform: none; opacity: 1; }
.plies .cap { position: absolute; left: 40px; bottom: 50px; font-size: 38px; font-weight: 700; color: ${C.green2}; }
/* Step 6: CAD */
.cad { position: absolute; left: 0; right: 0; top: 280px; display: flex; justify-content: center; align-items: center; gap: 70px; opacity: 0; transition: opacity .6s; }
body.b6 .cad { opacity: 1; }
body.b6 .stage .pc, body.b6 .layer { opacity: 0 !important; }
.screen { width: 860px; height: 520px; border-radius: 26px; background: #1d2b26; padding: 26px; box-shadow: 0 24px 50px rgba(16,35,28,.25); position: relative; }
.screen svg { width: 100%; height: 100%; background: #eef4ef; border-radius: 12px; }
.screen .ok { position: absolute; right: -40px; bottom: -36px; background: ${C.green}; color: #fff; font-weight: 800; font-size: 42px; padding: 16px 30px; border-radius: 999px; transform: scale(0); transition: transform .6s 2.4s cubic-bezier(.2,1.5,.3,1); }
body.b6 .screen .ok { transform: none; }
.cad .mini { opacity: 0; }
body.b6 .cad .mini { animation: pop .45s forwards; }
@keyframes pop { from { opacity: 0; } to { opacity: 1; } }
.cad ul { list-style: none; font-size: 46px; font-weight: 700; display: grid; gap: 26px; max-width: 640px; }
.cad li { display: flex; gap: 18px; align-items: center; opacity: 0; transition: opacity .5s; }
.cad li::before { content: '✓'; width: 62px; height: 62px; flex: none; border-radius: 50%; background: ${C.mint}; color: ${C.green2}; display: grid; place-items: center; font-size: 38px; }
body.b6 .cad li:nth-child(1) { opacity: 1; transition-delay: .6s; } body.b6 .cad li:nth-child(2) { opacity: 1; transition-delay: 1.6s; } body.b6 .cad li:nth-child(3) { opacity: 1; transition-delay: 2.8s; }
.cad .free { color: ${C.green2}; }
`;
  // Fabric: a long strip with selvedges (top/bottom dashed lines) and a width arrow.
  const fabric = `<g class="layer layer-fabric">
    <rect id="fab-rect" class="fabric" x="230" y="400" width="1450" height="400" rx="6"/>
    <line class="selvedge" x1="230" y1="400" x2="1680" y2="400"/><line class="selvedge" x1="230" y1="800" x2="1680" y2="800"/>
    <g id="fab-end"><line x1="1680" y1="380" x2="1680" y2="820" stroke="${C.red}" stroke-width="5"/></g>
    <g class="dim"><line x1="185" y1="405" x2="185" y2="795" stroke="${C.ink}" stroke-width="3"/><path d="M175 425 L185 405 L195 425 M175 775 L185 795 L195 775" fill="none" stroke="${C.ink}" stroke-width="3"/>
    <text x="150" y="600" transform="rotate(-90 150 600)" text-anchor="middle">Fabric width</text></g>
    <defs><pattern id="hatch" width="16" height="16" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><rect width="16" height="16" fill="#fbe9e4"/><line x1="0" y1="0" x2="0" y2="16" stroke="${C.red}" stroke-width="5"/></pattern></defs>
    <rect class="waste" x="810" y="425" width="180" height="370" rx="4"/><text class="wlabel" x="900" y="850">Waste</text>
  </g>
  <g class="layer layer-short saved"><path d="M1395 365 H1680" stroke="${C.green}" stroke-width="5"/><path d="M1395 350 V380 M1680 350 V380" stroke="${C.green}" stroke-width="5"/><text x="1538" y="335" text-anchor="middle">Fabric saved</text></g>`;
  const pieces = piece('a-front', 'front') + piece('a-front2', 'front') + piece('a-back', 'back') + piece('a-sleeve', 'sleeve') + piece('a-sleeve2', 'sleeve') + piece('a-collar', 'collar');
  // A simple shirt picture for step 0/1.
  const shirt = `<g id="shirt"><path d="M150 0 L60 40 L0 150 L70 185 L110 120 L110 400 H390 V120 L430 185 L500 150 L440 40 L350 0 Q250 60 150 0 Z" fill="${C.blue}" stroke="${C.ink}" stroke-width="4" stroke-linejoin="round"/></g>`;
  const plies = Array.from({ length: 10 }, (_, i) => `<i style="bottom:${110 + i * 26}px;transition-delay:${(0.2 + i * 0.08).toFixed(2)}s"></i>`).join('');
  let miniN = 0;
  const mini = (x, y, kind, r = 0) => `<g transform="translate(${x} ${y}) rotate(${r}) scale(.55)"><g class="mini" style="animation-delay:${(0.3 + 0.16 * miniN++).toFixed(2)}s"><path class="piece ${PIECES[kind].cls}" d="${PIECES[kind].d}"/></g></g>`;
  const body = `
<div class="title"><span class="kicker">The basics</span><h2>Pattern, marker and CAD</h2></div>
<div class="steps"><span></span><span></span><span></span><span></span><span></span><span></span></div>
<p class="head head-1">1 · A pattern piece is one flat part of the garment</p>
<p class="head head-2">2 · A marker = all pieces laid on the fabric width</p>
<p class="head head-3">3 · Pieces closer together → shorter marker → less fabric</p>
<p class="head head-4">4 · What does it save? An example</p>
<p class="head head-5">4 · What does it save? An example</p>
<p class="head head-6">5 · CAD software does the layout for you</p>
<svg class="stage" viewBox="0 0 1920 1080">${fabric}${shirt}${pieces}</svg>
<div class="money">
  <div class="calc"><span class="badge">Example</span>
    <p class="row r1"><b>10 cm</b> shorter marker</p>
    <p class="row r2">× <b>50</b> layers of fabric</p>
    <p class="row r3">= <b>5 m</b> less fabric</p>
    <p class="row r4">× <b>₹150</b> per metre</p>
    <p class="row r5">= <b>₹750</b> <small>saved on one lay</small></p>
  </div>
  <div class="plies">${plies}<p class="cap">50 layers, cut together</p></div>
</div>
<div class="cad">
  <div class="screen"><svg viewBox="0 0 800 470">
    <line class="selvedge" x1="20" y1="40" x2="780" y2="40"/><line class="selvedge" x1="20" y1="430" x2="780" y2="430"/>
    <g transform="translate(0 0)">${mini(30, 50, 'front')}${mini(160, 50, 'back')}${mini(30, 230, 'front')}${mini(160, 230, 'back')}${mini(300, 50, 'sleeve')}${mini(300, 160, 'sleeve')}${mini(300, 270, 'sleeve')}${mini(445, 50, 'front')}${mini(445, 230, 'back')}${mini(590, 50, 'collar')}${mini(590, 90, 'collar')}${mini(590, 130, 'sleeve')}</g>
  </svg><span class="ok">✓ Checked</span></div>
  <ul><li>Places every piece</li><li>Checks grain, gaps and overlaps</li><li class="free">BETA does this for free</li></ul>
</div>`;
  return page({ css, body });
}

// ---------- Problem ----------
export function problemCard() {
  const css = `
.grid { position: absolute; left: 110px; right: 110px; top: 90px; display: grid; grid-template-columns: 1fr 1fr; gap: 60px; }
.panel { background: #fff; border-radius: 30px; border: 3px solid #e3e8e5; padding: 44px; height: 720px; display: flex; flex-direction: column; box-shadow: 0 14px 40px rgba(16,35,28,.07); opacity: 0; transform: translateY(30px); transition: opacity .7s, transform .7s; }
body.b1 .p1 { opacity: 1; transform: none; } body.b1 .p2 { opacity: 1; transform: none; transition-delay: 2.6s; }
.panel h2 { font-size: 62px; margin-bottom: 26px; }
.panel p { font-size: 44px; color: ${C.muted}; font-weight: 600; line-height: 1.3; }
.chalk { background: #2c3a35; border-radius: 18px; height: 400px; margin: 10px 0 30px; }
.chalk svg { width: 100%; height: 100%; }
.chalk path { fill: none; stroke: #f1efe6; stroke-width: 4; stroke-linejoin: round; stroke-dasharray: 2000; stroke-dashoffset: 2000; transition: stroke-dashoffset 2.6s ease; }
body.b1 .chalk path { stroke-dashoffset: 0; }
.chalk .gap { fill: ${C.red}; opacity: 0; transition: opacity .6s 2.2s; }
body.b1 .chalk .gap { opacity: .55; }
.money { display: flex; gap: 26px; height: 400px; margin: 10px 0 30px; align-items: center; justify-content: center; background: #fff8e6; border-radius: 18px; }
.money span { width: 150px; height: 150px; border-radius: 50%; background: ${C.amber}; display: grid; place-items: center; font: 800 70px 'Bricolage'; color: ${C.ink}; transform: scale(0); transition: transform .5s cubic-bezier(.2,1.5,.3,1); }
body.b1 .money span { transform: none; transition-delay: 3s; } body.b1 .money span:nth-child(2) { transition-delay: 3.2s; } body.b1 .money span:nth-child(3) { transition-delay: 3.4s; } body.b1 .money span:nth-child(4) { transition-delay: 3.6s; }
`;
  const body = `<div class="grid">
  <section class="panel p1"><h2>Marker by hand</h2>
    <div class="chalk"><svg viewBox="0 0 760 380">
      <path d="M40 60 H230 L270 110 V330 H40 Z"/><path d="M330 60 H500 Q510 110 560 120 V330 H330 Z"/><path d="M600 80 Q660 40 720 80 L710 200 H610 Z"/><path d="M610 240 H720 V280 H610 Z"/>
      <rect class="gap" x="272" y="120" width="56" height="210"/><rect class="gap" x="562" y="210" width="44" height="120"/><rect class="gap" x="40" y="335" width="680" height="30"/>
    </svg></div>
    <p>Gaps between pieces are fabric you paid for.</p></section>
  <section class="panel p2"><h2>Commercial CAD</h2>
    <div class="money"><span>₹</span><span>₹</span><span>₹</span><span>₹</span></div>
    <p>It works, but licences cost a lot for a small unit.</p></section>
</div>`;
  return page({ css, body });
}

// ---------- Proof ----------
export function proofCard() {
  const wins = 84, ties = 12, losses = 204, total = wins + ties + losses;
  const pct = (n) => `${((100 * n) / total).toFixed(2)}%`;
  const css = `
.wrap { position: absolute; left: 120px; right: 120px; top: 80px; }
.kicker { font: 800 30px 'Segoe UI'; letter-spacing: .12em; text-transform: uppercase; color: ${C.green}; }
h2 { font-size: 76px; margin: 12px 0 18px; }
.src { font-size: 32px; color: ${C.muted}; font-weight: 600; }
.tiles { display: grid; grid-template-columns: repeat(3, 1fr); gap: 36px; margin-top: 44px; }
.tile { background: #fff; border: 3px solid #e3e8e5; border-radius: 28px; padding: 34px 40px; opacity: 0; transform: translateY(24px); transition: opacity .6s, transform .6s; }
body.b2 .tile { opacity: 1; transform: none; } body.b2 .tile:nth-child(2) { transition-delay: .5s; } body.b2 .tile:nth-child(3) { transition-delay: 1s; }
.tile b { display: block; font: 750 96px 'Bricolage'; color: ${C.ink}; }
.tile b.neg { color: #8a3a12; }
.tile span { font-size: 34px; color: ${C.muted}; font-weight: 700; }
.wtl { margin-top: 40px; opacity: 0; transition: opacity .6s; } body.b3 .wtl { opacity: 1; }
.bar { display: flex; height: 64px; border-radius: 16px; overflow: hidden; border: 3px solid ${C.ink}; }
.bar i { display: block; height: 100%; transform-origin: left; transform: scaleX(0); transition: transform 1s cubic-bezier(.2,.8,.2,1); }
body.b3 .bar i { transform: none; }
.w { background: ${C.green}; } .t { background: #b9c4bf; } .l { background: #e7b9a6; }
.legend { display: flex; gap: 50px; font-size: 40px; font-weight: 800; margin-top: 16px; }
.legend em { font-style: normal; color: ${C.muted}; font-weight: 600; }
.verdict { margin-top: 18px; font-size: 40px; font-weight: 700; }
.check { position: absolute; left: 120px; right: 120px; top: 640px; display: flex; align-items: center; gap: 30px; background: ${C.green}; color: #fff; border-radius: 28px; padding: 30px 44px; font-size: 52px; font-weight: 800; opacity: 0; transform: scale(.96); transition: opacity .6s, transform .6s; }
body.b4 .check { opacity: 1; transform: none; } body.b4 .tiles { opacity: .3 !important; } body.b4 .wtl { opacity: 0 !important; }
.check span { font-size: 80px; }
`;
  const body = `<div class="wrap">
  <p class="kicker">Honest numbers · public benchmark</p>
  <h2>BETA vs Lectra’s recorded results</h2>
  <p class="src">Lectra open dataset (Zenodo 7030786, CC BY 4.0) · 300 tasks · 60 s per task · points = BETA efficiency minus Lectra’s</p>
  <div class="tiles">
    <div class="tile"><b>300</b><span>real nesting tasks</span></div>
    <div class="tile"><b class="neg">−0.73</b><span>points, mean</span></div>
    <div class="tile"><b class="neg">−1.22</b><span>points, median task</span></div>
  </div>
  <div class="wtl"><div class="bar"><i class="w" style="width:${pct(wins)}"></i><i class="t" style="width:${pct(ties)};transition-delay:.4s"></i><i class="l" style="width:${pct(losses)};transition-delay:.8s"></i></div>
    <p class="legend"><span>${wins} <em>wins</em></span><span>${ties} <em>ties</em></span><span>${losses} <em>losses</em></span></p>
    <p class="verdict">Close, but not better. No general win.</p></div>
</div>
<div class="check"><span>✓</span>Every marker is checked by an independent validator before export.</div>`;
  return page({ css, body });
}

// ---------- Close ----------
export function closeCard({ vertical = false } = {}) {
  const w = vertical ? 1080 : 1920, h = vertical ? 1920 : 1080;
  const css = `
.wrap { position: absolute; inset: 0; display: flex; flex-direction: column; align-items: center; justify-content: ${vertical ? 'flex-start' : 'center'}; padding: ${vertical ? '170px 60px 0' : '0 0 170px'}; gap: ${vertical ? 48 : 34}px; text-align: center; }
.wm { width: ${vertical ? 880 : 760}px; }
.wm svg { width: 100%; height: auto; display: block; }
.pills { display: flex; gap: 22px; flex-wrap: wrap; justify-content: center; max-width: ${vertical ? 960 : 1700}px; }
.pills .pill { opacity: 0; transform: translateY(16px); transition: opacity .5s, transform .5s; }
body.b1 .pills .pill { opacity: 1; transform: none; } body.b1 .pills .pill:nth-child(2) { transition-delay: .3s; } body.b1 .pills .pill:nth-child(3) { transition-delay: .6s; }
.young { font-size: ${vertical ? 44 : 42}px; font-weight: 700; color: ${C.muted}; max-width: ${vertical ? 940 : 1400}px; }
.young b { color: ${C.green2}; } .nowrap { white-space: nowrap; }
body.b2 .young { opacity: 1; transform: none; }
.try { background: ${C.green}; color: #fff; border-radius: 34px; padding: 34px ${vertical ? 34 : 56}px; box-shadow: 0 18px 44px rgba(23,107,82,.3); transform: scale(.9); opacity: 0; transition: transform .7s cubic-bezier(.2,1.4,.3,1), opacity .5s; }
body.b3 .try { transform: none; opacity: 1; }
.try h2 { font-size: ${vertical ? 72 : 76}px; margin-bottom: 14px; }
.try .url { font: 700 ${vertical ? 33 : 46}px 'Segoe UI'; background: #fff; color: ${C.green2}; border-radius: 16px; padding: 10px ${vertical ? 18 : 26}px; display: inline-block; white-space: nowrap; }
.links { font-size: ${vertical ? 32 : 34}px; color: ${C.muted}; font-weight: 600; opacity: 0; transition: opacity .6s .6s; }
body.b3 .links { opacity: 1; }
.links b { color: ${C.ink}; }
`;
  const body = `<div class="wrap">
  <div class="wm">${WORDMARK}</div>
  <div class="pills"><span class="pill solid">Free</span><span class="pill">Works offline</span><span class="pill">Your data stays in your factory</span></div>
  <p class="young fade">A young project, built for fun with modern AI tools. <b class="nowrap">We need your help.</b></p>
  <div class="try"><h2>Try it now in your browser</h2><span class="url">${SITE_SHORT}</span></div>
  <p class="links">Code: <b>${REPO_SHORT}</b> · Found a problem? Press <b>Feedback</b> in the app.</p>
</div>`;
  return page({ w, h, css, body });
}

// ---------- Backgrounds (static PNGs for compositing) ----------
/** 16:9 background for the phone scene. The phone video goes into the frame at PHONE_BOX. */
export const PHONE_BOX = { x: 290, y: 22, w: 480, h: 1038 };
export function phoneBackground() {
  const b = PHONE_BOX, pad = 18;
  const css = `
.phone { position: absolute; left: ${b.x - pad}px; top: ${b.y - pad}px; width: ${b.w + 2 * pad}px; height: ${b.h + 2 * pad}px; border-radius: 58px; background: #12201b; box-shadow: 0 30px 70px rgba(16,35,28,.35), inset 0 0 0 3px #2f4740; }
.side { position: absolute; left: 880px; top: 90px; right: 110px; }
.kicker { font: 800 30px 'Segoe UI'; letter-spacing: .12em; text-transform: uppercase; color: ${C.green}; }
h2 { font-size: 84px; margin: 14px 0 34px; }
ol { list-style: none; display: grid; gap: 22px; counter-reset: s; }
li { counter-increment: s; display: flex; align-items: center; gap: 24px; font-size: 44px; font-weight: 700; }
li::before { content: counter(s); width: 70px; height: 70px; flex: none; border-radius: 50%; background: ${C.mint}; color: ${C.green2}; display: grid; place-items: center; font: 800 38px 'Bricolage'; }
.sample { margin-top: 36px; display: inline-flex; gap: 14px; align-items: center; background: #fff4d6; border: 3px solid ${C.amber}; border-radius: 18px; padding: 14px 24px; font-size: 32px; font-weight: 700; }
`;
  const body = `<div class="phone"></div><div class="side">
  <p class="kicker">Quick Marker · on a phone</p><h2>Photo to marker</h2>
  <ol><li>Photo of paper pieces + an A4 sheet</li><li>BETA finds the outlines</li><li>Fabric width and garments</li><li>Marker: print, chalk or projector</li></ol>
  <p class="sample"><span class="badge">Sample</span> Practice photos. Not a real factory pattern.</p></div>`;
  return page({ css, body });
}

/** 9:16 background for 16:9 footage in the vertical teaser. Footage goes into VBOX. */
export const VBOX = { x: 0, y: 400, w: 1080, h: 810 };
export function verticalBackground(title) {
  const css = `
.top { position: absolute; left: 60px; right: 60px; top: 120px; display: flex; align-items: center; gap: 26px; }
.top .logo { width: 110px; height: 110px; flex: none; }
.top h2 { font-size: 76px; }
.frame { position: absolute; left: ${VBOX.x}px; top: ${VBOX.y - 10}px; width: ${VBOX.w}px; height: ${VBOX.h + 20}px; background: #12201b; }
.foot { position: absolute; left: 60px; right: 60px; top: 1250px; font-size: 40px; font-weight: 700; color: ${C.muted}; }
`;
  return page({ w: 1080, h: 1920, css, body: `<div class="top"><div class="logo">${LOGO}</div><h2>${title}</h2></div><div class="frame"></div><p class="foot">BETA Cutting Room · free and open source</p>` });
}
/** 9:16 background for phone footage in the vertical teaser. */
export const VPHONE = { x: 240, y: 250, w: 600, h: 1298 };
export function verticalPhoneBackground() {
  const b = VPHONE, pad = 22;
  const css = `
.top { position: absolute; left: 60px; right: 60px; top: 80px; display: flex; align-items: center; gap: 24px; }
.top .logo { width: 100px; height: 100px; flex: none; }
.top h2 { font-size: 68px; }
.phone { position: absolute; left: ${b.x - pad}px; top: ${b.y - pad}px; width: ${b.w + 2 * pad}px; height: ${b.h + 2 * pad}px; border-radius: 72px; background: #12201b; box-shadow: 0 30px 70px rgba(16,35,28,.35); }
.s { position: absolute; right: 50px; top: 230px; }
`;
  return page({ w: 1080, h: 1920, css, body: `<div class="top"><div class="logo">${LOGO}</div><h2>Photo to marker</h2></div><div class="phone"></div><span class="badge s">Sample</span>` });
}

// ---------- One-page explainer image (1080 x 1350) ----------
const EXPLAINER = {
  en: {
    lang: 'en', title: 'What is BETA?', lead: 'Free software for the cutting room. It plans markers so you use less fabric.',
    steps: [['Pattern', 'The flat shapes of a garment: front, back, sleeve.'], ['Marker', 'All pieces laid on the fabric width. Shorter = less fabric.'], ['Cut', 'Lays, plies and metres for your order. Print or plot.'], ['Bundles', 'Number the plies. QR ticket for each bundle.']],
    example: '10 cm shorter × 50 layers = 5 m less fabric. At ₹150 a metre = ₹750 saved on one lay.',
    facts: ['Free and open source', 'Works offline', 'Your data stays in your factory'],
    howTitle: 'How to try', how: ['Open the website on a PC or phone', 'Load the demo shirt and press Nest', 'Press Feedback to tell us what to fix'],
    young: 'A young project, built for fun with modern AI tools. We need your help.', note: '',
  },
  hi: {
    lang: 'hi', title: 'BETA क्या है?', lead: 'कटिंग रूम के लिए मुफ़्त सॉफ़्टवेयर। यह मार्कर प्लान करता है ताकि कपड़ा कम लगे।',
    steps: [['पैटर्न', 'कपड़े के सपाट आकार: आगे, पीछे, बाँह।'], ['मार्कर', 'सारे पीस कपड़े की चौड़ाई पर। छोटा मार्कर = कम कपड़ा।'], ['कटिंग', 'ऑर्डर के लिए ले, प्लाई और मीटर। प्रिंट या प्लॉट।'], ['बंडल', 'प्लाई पर नंबर। हर बंडल का QR टिकट।']],
    example: '10 सेमी छोटा × 50 परतें = 5 मीटर कम कपड़ा। ₹150 प्रति मीटर पर = एक ले पर ₹750 की बचत।',
    facts: ['मुफ़्त और ओपन-सोर्स', 'बिना इंटरनेट चलता है', 'आपका डेटा आपकी फ़ैक्टरी में'],
    howTitle: 'कैसे आज़माएँ', how: ['PC या फ़ोन पर वेबसाइट खोलिए', 'डेमो शर्ट खोलिए और Nest दबाइए', 'Feedback दबाकर बताइए क्या ठीक करना है'],
    young: 'यह एक नया प्रोजेक्ट है, आधुनिक AI टूल्स की मदद से मज़े के लिए बना। हमें आपकी मदद चाहिए।', note: 'हिंदी ड्राफ़्ट · मूल भाषी से जाँच ज़रूरी (Hindi draft, needs native review)',
  },
};
export function explainerImage(lang = 'en') {
  const t = EXPLAINER[lang];
  const css = `
body { padding: 54px 64px 0; font-family: ${lang === 'hi' ? "'Nirmala UI', 'Segoe UI'" : "'Segoe UI'"}, sans-serif; }
${lang === 'hi' ? 'h1, h2, .display { font-family: "Nirmala UI", "Segoe UI", sans-serif; font-weight: 800; letter-spacing: 0; line-height: 1.25; }' : ''}
header { display: flex; align-items: center; gap: 26px; }
header .logo { width: 116px; height: 116px; flex: none; }
h1 { font-size: 84px; }
.lead { font-size: 34px; font-weight: 600; color: ${C.muted}; margin: 16px 0 24px; line-height: 1.3; }
.flow { display: grid; grid-template-columns: repeat(4, 1fr); gap: 0; position: relative; }
.st { text-align: center; padding: 0 8px; position: relative; }
.st .n { width: 96px; height: 96px; margin: 0 auto 14px; border-radius: 50%; background: ${C.green}; color: #fff; display: grid; place-items: center; }
.st .n svg { width: 58px; height: 58px; }
.st:not(:last-child)::after { content: '→'; position: absolute; right: -22px; top: 22px; font-size: 48px; font-weight: 800; color: ${C.amber}; }
.st h3 { font-size: 34px; font-family: inherit; font-weight: 800; letter-spacing: 0; }
.st p { font-size: 24px; color: ${C.muted}; font-weight: 600; line-height: 1.3; margin-top: 6px; }
.ex { margin: 24px 0 20px; background: #fff; border: 3px solid ${C.amber}; border-radius: 24px; padding: 22px 28px; font-size: 30px; font-weight: 700; line-height: 1.35; display: flex; gap: 20px; align-items: flex-start; }
.ex .badge { flex: none; font-size: 22px; margin-top: 4px; }
.facts { display: flex; gap: 14px; flex-wrap: wrap; }
.facts .pill { font-size: 28px; padding: 10px 22px; }
.how { margin-top: 22px; background: ${C.green}; color: #fff; border-radius: 28px; padding: 24px 34px; }
.how h2 { font-size: 44px; font-family: inherit; margin-bottom: 8px; }
.how ol { margin-left: 40px; font-size: 30px; font-weight: 600; line-height: 1.4; }
.how .url { display: inline-block; margin-top: 16px; background: #fff; color: ${C.green2}; font-weight: 800; font-size: 30px; padding: 8px 18px; border-radius: 12px; }
.young { margin-top: 16px; font-size: 26px; font-weight: 600; color: ${C.muted}; }
.note { position: absolute; left: 0; right: 0; bottom: 0; background: ${C.amber}; color: ${C.ink}; font-size: 24px; font-weight: 800; text-align: center; padding: 10px; }
`;
  const icons = [
    '<svg viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2" stroke-linejoin="round"><path d="M5 3h9l5 5v13H5z"/><path d="M12 8v9"/></svg>',
    '<svg viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2" stroke-linejoin="round"><rect x="2" y="5" width="20" height="14" rx="1"/><path d="M6 8h5v8H6zM13 8h5v4h-5z"/></svg>',
    '<svg viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round"><circle cx="6" cy="18" r="3"/><circle cx="18" cy="18" r="3"/><path d="M8 16 19 3M16 16 5 3"/></svg>',
    '<svg viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2" stroke-linejoin="round"><path d="M3 7h18v12H3z"/><path d="M7 7V4h10v3"/><path d="M7 12h4v4H7z"/></svg>',
  ];
  const body = `<header><div class="logo">${LOGO}</div><h1>${t.title}</h1></header>
<p class="lead">${t.lead}</p>
<div class="flow">${t.steps.map(([h, p], i) => `<div class="st"><div class="n">${icons[i]}</div><h3>${h}</h3><p>${p}</p></div>`).join('')}</div>
<div class="ex"><span class="badge">${lang === 'hi' ? 'उदाहरण' : 'Example'}</span><span>${t.example}</span></div>
<div class="facts">${t.facts.map((f, i) => `<span class="pill${i === 0 ? ' solid' : ''}">${f}</span>`).join('')}</div>
<div class="how"><h2>${t.howTitle}</h2><ol>${t.how.map((h) => `<li>${h}</li>`).join('')}</ol><span class="url">${SITE_SHORT}</span></div>
<p class="young">${t.young}</p>
${t.note ? `<p class="note">${t.note}</p>` : ''}`;
  return page({ w: 1080, h: 1350, css, body, lang: t.lang });
}

export const CARDS = { hook: hookCard, basics: basicsCard, problem: problemCard, proof: proofCard, close: closeCard };
