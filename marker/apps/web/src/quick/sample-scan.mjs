// Regression against the actual Rust WASM scanner, not the practice mock.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
const pkg=new URL('../../../../packages/marker-wasm/pkg/',import.meta.url);
const raw=await import(new URL('beta_marker_wasm.js',pkg));
raw.initSync({module:readFileSync(new URL('beta_marker_wasm_bg.wasm',pkg))});
const scan=JSON.parse(raw.quick_scan(new Uint8Array(readFileSync(new URL('./sample-tshirt.png',import.meta.url))),'\{"reference":"a4-sheet","photoId":"sample"\}'));
assert.equal(scan.pieces.length,5);
assert.ok(scan.pieces.every(p=>p.suggestions.grain));
assert.notEqual(scan.scale.confidence,'low');
assert.ok(scan.scale.expectedErrorMm<2.2);
const routine=new Set(['grain_unconfirmed','review_required']);
assert.deepEqual(scan.warnings.filter(w=>!routine.has(w.code)),[]);
console.log(JSON.stringify({pieces:5,grainArrows:5,qualityWarnings:0,confidence:scan.scale.confidence,estimatedErrorMm:scan.scale.expectedErrorMm}));
