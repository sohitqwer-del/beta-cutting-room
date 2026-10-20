// Small, self-contained review sheet. SPDX-License-Identifier: Apache-2.0
import fs from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
const here=path.dirname(fileURLToPath(import.meta.url));
const root=path.resolve(here,'../../testdata/open');
const freesewing=JSON.parse(await fs.readFile(path.join(root,'freesewing/report.json'),'utf8'));
const garmentcode=JSON.parse(await fs.readFile(path.join(root,'garmentcode/report.json'),'utf8'));
const names={simon:'Simon shirt',teagan:'Teagan T-shirt',titan:'Titan trouser block',hugo:'Hugo hoodie',bella:'Bella bodice block'};
const previews=[];
for(const [style,name] of Object.entries(names))previews.push({name,svg:await fs.readFile(path.join(root,'freesewing',style+'.svg'),'utf8')});
for(const garment of garmentcode.garments.slice(0,3))previews.push({name:garment.id,svg:await fs.readFile(path.join(root,'garmentcode',garment.id+'.svg'),'utf8')});
const rows=Object.entries(freesewing.styles).map(([style,data])=>`<tr><th scope="row">${names[style]}</th><td>${data.materials.fabric.M.cut_pieces}</td>${['S','M','L','XL'].map(s=>`<td>${data.materials.fabric[s].area_m2.toFixed(3)}</td>`).join('')}</tr>`).join('');
const html=`<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Open garment pattern review</title>
<style>
:root{color-scheme:light dark;--background:light-dark(#fff,#17191b);--foreground:light-dark(#192b39,#e6edf3);--muted-foreground:light-dark(#51616c,#b7c3cc);--border:light-dark(#d9e0e5,#404b53);--accent:light-dark(#13617e,#8dd4e8);--radius:10px}
*{box-sizing:border-box}body{margin:0;font:15px/1.5 system-ui,sans-serif;color:var(--foreground);background:transparent}main{max-width:1000px;margin:auto}h1{font-size:23px;margin:0 0 12px}h2{font-size:18px;margin:23px 0 10px}p{margin:8px 0}a{color:var(--accent)}.summary{display:flex;gap:26px;flex-wrap:wrap;border-bottom:1px solid var(--border);padding-bottom:14px}.summary strong{font-size:27px;display:block}.summary span{color:var(--muted-foreground)}table{border-collapse:collapse;width:100%;font-variant-numeric:tabular-nums}th,td{padding:8px 4px;text-align:right;border-bottom:1px solid var(--border)}th:first-child{text-align:left}thead{font-size:12px;color:var(--muted-foreground)}.scroll{overflow:auto}label{font-weight:600}select{font:inherit;color:inherit;background:var(--background);border:1px solid var(--border);border-radius:5px;padding:7px;max-width:100%}.controls{display:flex;gap:15px;align-items:center;flex-wrap:wrap;margin:10px 0}input{accent-color:var(--accent)}#preview{height:380px;overflow:auto;border-block:1px solid var(--border);background:white;overscroll-behavior:contain}#preview svg{display:block;width:100%;min-width:680px;height:auto}#preview svg text{fill:#192b39}.minor{font-size:13px;color:var(--muted-foreground)}li{margin:7px 0}@media(max-width:480px){body{font-size:13px}h1{font-size:20px}.summary{gap:17px}.summary strong{font-size:23px}th,td{padding:7px 2px}thead{font-size:11px}}
</style></head><body><main>
<h1>Open garment pattern review</h1>
<div class="summary"><div><strong>5 styles</strong><span>FreeSewing 4.10.2 · MIT</span></div><div><strong>299 / 300</strong><span>GarmentCodeData jobs · CC BY 4.0</span></div><div><strong>35 blocks</strong><span>Real M-size DXF-AAMA · CC BY 4.0</span></div></div>
<h2>FreeSewing main fabric</h2><p class="minor">Cut copies and area per garment, including seam allowance. 14 jobs across all materials. S1 M2 L2 XL1 plus M quick tests.</p>
<div class="scroll"><table><thead><tr><th>Style</th><th>Copies</th><th>S · m²</th><th>M · m²</th><th>L · m²</th><th>XL · m²</th></tr></thead><tbody>${rows}</tbody></table></div>
<p class="minor">Simon M also needs 6 interfacing copies, 0.135 m². Hugo M also needs 3 ribbing copies, 0.207 m². Menswear sets: 36/38/40/42; Bella: 32/34/36/38. Numbers are neck cm, not retail sizes.</p>
<h2>Inspect the cut pieces</h2><div class="controls"><label for="style">Style <select id="style">${previews.map((p,i)=>`<option value="${i}">${p.name}</option>`).join('')}</select></label><label for="zoom">Zoom <output id="zoom-value">100%</output> <input id="zoom" type="range" min="100" max="250" step="25" value="100"></label></div>
<div id="preview" tabindex="0" aria-label="Scrollable garment cut pieces. Arrow shows grain direction."></div><p class="minor">Separate pieces, not a nesting result. Scroll or zoom to inspect labels. FreeSewing previews show M and all materials. Red arrows show grain.</p>
<h2>Downloaded and converted</h2><ul><li>GarmentCodeData: 251,722,049-byte patterns-only archive. Sample: 300 JSON specs, 8,715,725 bytes. 299 converted; one invalid offset rejected. Ten jobs and three previews in git.</li><li>Open+Pattern PT windbreaker: 539,921-byte ZIP; full six-size DXF outside git. The M subset is 509,410 bytes and keeps 35 blocks and INSERTs.</li><li>Numerical geometry tests and Rust contract parsing pass. Product workspace tests pass; Clippy has one existing core warning.</li></ul>
<h2>Limits for benchmarking</h2><ul><li>GarmentCode grain is assumed local panel up; this sample is one reference-body batch. Its 10 mm allowance is uniform. Mirror provenance to ETH v2 is not byte-verified.</li><li>Titan and Bella are blocks. Simon's optional undercollar trim is not applied; two source cutlist corrections and three grain assumptions are recorded.</li><li>Simon M totals 1.578 m². At an assumed 85% efficiency this is 1.237 m of 1.5 m cloth, below the rough 1.6–1.8 m consumption reference. No achieved efficiency is claimed.</li><li>No published commercial-CAD result with matching openly licensed vector geometry was found. Gerber/Lectra paper results are recorded as leads in SOURCES.md.</li></ul>
<p class="minor">Sources: <a href="https://freesewing.eu/">FreeSewing</a> · <a href="https://igl.ethz.ch/projects/GarmentCodeData/">GarmentCodeData</a> · <a href="https://huggingface.co/georgeNakayama/AIpparel">AIpparel mirror</a> · <a href="https://openpattern.io/product/us-pt-windbreaker-jacket/">Open+Pattern</a>. FreeSewing: © Joost De Cock and contributors, MIT. GarmentCodeData: Korosteleva et al., ECCV 2024; converted to mm and offset for this benchmark, <a href="https://creativecommons.org/licenses/by/4.0/">CC BY 4.0</a>. Windbreaker: Open+Pattern, CC BY 4.0; filtered to M. Full sizes, attribution and regeneration commands are in the adjacent READMEs and SOURCES.md.</p>
</main><script>
const previews=${JSON.stringify(previews).replaceAll('<','\\u003c')};
const style=document.getElementById('style'),preview=document.getElementById('preview'),zoom=document.getElementById('zoom'),value=document.getElementById('zoom-value');
function update(){preview.innerHTML=previews[Number(style.value)].svg;preview.firstElementChild.style.width=zoom.value+'%';value.value=zoom.value+'%';preview.scrollTop=0;preview.scrollLeft=0;}
style.addEventListener('change',update);zoom.addEventListener('input',update);update();
</script></body></html>`;
await fs.writeFile(path.join(root,'review.html'),html);
console.log('Wrote marker/testdata/open/review.html');
