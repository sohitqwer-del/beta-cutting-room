// Headless WASM scan/export check. Uses only public synthetic photos.
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import assert from 'node:assert/strict';
const [moduleDir, photoPath, outputDir] = process.argv.slice(2);
if (!moduleDir || !photoPath || !outputDir) throw new Error('Usage: quick-smoke.mjs module-dir synthetic-photo output-dir');
const raw = await import(pathToFileURL(resolve(moduleDir, 'beta_marker_wasm.js')).href);
const wasm = raw.initSync({ module: readFileSync(resolve(moduleDir, 'beta_marker_wasm_bg.wasm')) });
const photo = new Uint8Array(readFileSync(photoPath));
const start = performance.now();
const scan = JSON.parse(raw.quick_scan(photo, JSON.stringify({reference:'a4-sheet',photoId:'a'})));
const elapsed = performance.now() - start;
assert.ok(scan.pieces.length > 0);
assert.deepEqual(scan.sourceSize, [4000,3000]);
assert.deepEqual(scan.processedSize, [2000,1500]);
const merge = JSON.parse(raw.quick_merge(JSON.stringify([{photoId:'a',pieces:scan.pieces,warnings:scan.warnings},
  {photoId:'b',pieces:scan.pieces,warnings:scan.warnings}])));
assert.ok(merge.duplicates.length > 0);
assert.equal(merge.pieces.length, scan.pieces.length * 2);
const pieces = [scan.pieces[0]];
pieces[0].size = 'M'; pieces[0].grain = pieces[0].suggestions.grain.value;
const job = JSON.parse(raw.quick_pieces_to_job(JSON.stringify(pieces), '{"width_mm":1500}', '{"M":1}'));
assert.equal(job.pieces[0].size, 'M');
// Export a separate, known-valid 200 x 300 mm rectangle at the marker origin.
const j={format:'beta-marker.job/v1',name:'Quick PDF check',fabric:{width:600},pieces:[{id:'p',name:'Panel',size:'M',outline:[[0,0],[200,0],[200,300],[0,300]],quantity:1}]};
const r={format:'beta-marker.result/v1',job_name:j.name,width:600,length:200,piece_area:60000,efficiency:0.5,placements:[{piece_id:'p',copy:0,mirrored:false,rotation_deg:0,translate:[0,0]}],engine:'test',runtime_s:0};
const projector = JSON.parse(raw.projector_layout(JSON.stringify(j),JSON.stringify(r)));
assert.equal(projector.lengthMm, 200); assert.equal(projector.calibrationSquareMm,100);
mkdirSync(outputDir,{recursive:true});
for (const [name,bytes] of [
 ['a4-tiles.pdf',raw.tiled_marker_pdf(JSON.stringify(j),JSON.stringify(r),'{"paper":"A4","overlap_mm":10}')],
 ['a3-tiles.pdf',raw.tiled_marker_pdf(JSON.stringify(j),JSON.stringify(r),'{"paper":"A3","overlap_mm":10}')],
 ['chalk-en.pdf',raw.chalk_guide_pdf(JSON.stringify(j),JSON.stringify(r),'{"language":"en"}')],
 ['chalk-hi.pdf',raw.chalk_guide_pdf(JSON.stringify(j),JSON.stringify(r),'{"language":"hi"}')],
 ['chalk-ta.pdf',raw.chalk_guide_pdf(JSON.stringify(j),JSON.stringify(r),'{"language":"ta"}')],
]) {assert.equal(Buffer.from(bytes.slice(0,4)).toString(),'%PDF');writeFileSync(resolve(outputDir,name),bytes);}
writeFileSync(resolve(outputDir,'scan-overlay.png'),new Uint8Array(scan.overlayPng));
// Repeated calls detect retained native image buffers / growing WASM memory.
const before = wasm.memory.buffer.byteLength;
const times=[];
for(let i=0;i<3;i++){const t=performance.now();raw.quick_scan(photo,'{"reference":"a4-sheet"}');times.push(performance.now()-t);}
const after=wasm.memory.buffer.byteLength;
assert.ok(after <= before + 16*1024*1024, `Repeated scans grew memory: ${before} -> ${after}`);
assert.ok(after < 300*1024*1024, `WASM linear memory exceeded 300 MB: ${after}`);
const report={scanMs:elapsed,repeatMs:times,sourceSize:scan.sourceSize,processedSize:scan.processedSize,
  pieces:scan.pieces.length,wasmMemoryBytes:after,nodeRssBytes:process.memoryUsage().rss, maxRssBytes:process.resourceUsage().maxRSS*1024,
  note:'Node/V8 WASM on laptop; not a measured phone browser. WASM pages include allocator high-water mark.'};
writeFileSync(resolve(outputDir,'wasm-timing.json'),JSON.stringify(report,null,2));
console.log(JSON.stringify(report,null,2));
