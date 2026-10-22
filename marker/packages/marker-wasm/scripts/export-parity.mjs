// Compare real WASM output with independent native reference files, byte for byte.
// cargo run -p beta-marker-cli --example export_parity -- <output-directory>
// node packages/marker-wasm/scripts/export-parity.mjs <same-directory>
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import assert from 'node:assert/strict';
import markerInit, * as marker from '../pkg/beta_marker_wasm.js';
import patternInit, * as pattern from '../pkg/beta_pattern_wasm.js';
await markerInit({ module_or_path: await readFile(new URL('../pkg/beta_marker_wasm_bg.wasm', import.meta.url)) });
await patternInit({ module_or_path: await readFile(new URL('../pkg/beta_pattern_wasm_bg.wasm', import.meta.url)) });
const dir = resolve(process.argv[2]);
const cases = JSON.parse(await readFile(resolve(dir, 'cases.json'), 'utf8'));
assert.equal(cases.length, 46, 'Expected the complete synthetic export matrix');
const counts = new Map();
for (const c of cases) {
  let actual;
  if (c.kind === 'marker') actual = marker.export_marker(JSON.stringify(c.job), JSON.stringify(c.result), c.format, JSON.stringify(c.options));
  else if (c.kind === 'tiles') actual = marker.tiled_marker_pdf(JSON.stringify(c.job), JSON.stringify(c.result), JSON.stringify(c.options));
  else if (c.format === 'dxf') actual = Buffer.from(pattern.export_dxf(JSON.stringify(c.document)));
  else if (c.format === 'plt') actual = Buffer.from(pattern.export_plt(JSON.stringify(c.document)));
  else actual = pattern.export_pdf(JSON.stringify(c.document), c.format);
  assert.deepEqual(Buffer.from(actual), await readFile(resolve(dir, c.file)), c.file);
  const format = `${c.kind}/${c.format ?? c.options.paper}`;
  counts.set(format, (counts.get(format) ?? 0) + 1);
}
for (const [format, count] of counts) console.log(`PASS: ${format} ${count}/${count} byte-identical`);
console.log(`PASS: ${cases.length}/${cases.length} native/WASM factory files are byte-identical.`);
const imports = JSON.parse(await readFile(resolve(dir, 'imports.json'), 'utf8'));
assert.equal(imports.length, 6, 'Expected the complete synthetic import matrix');
for (const c of imports) {
  assert.deepEqual(JSON.parse(marker.import_dxf(new TextEncoder().encode(c.source), JSON.stringify(c.options ?? {}), c.rules ?? undefined)), c.report, c.name);
  console.log(`PASS: import/${c.name} native/WASM report identical`);
}
console.log(`PASS: ${imports.length}/${imports.length} native/WASM DXF reports are identical.`);
