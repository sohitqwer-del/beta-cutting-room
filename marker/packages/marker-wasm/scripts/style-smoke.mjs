// Headless test of the real generated WASM modules, including the worker session API.
// Usage: node style-smoke.mjs [wasm folder]; default ../pkg. Pass out/static-app/wasm to
// test the files shipped in the static browser build.
import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
import assert from 'node:assert/strict';
const dir = process.argv[2] ? new URL(`${pathToFileURL(resolve(process.argv[2])).href}/`) : new URL('../pkg/', import.meta.url);
const pattern = await import(new URL('beta_pattern_wasm.js', dir).href);
const marker = await import(new URL('beta_marker_wasm.js', dir).href);
await pattern.default({ module_or_path: await readFile(new URL('beta_pattern_wasm_bg.wasm', dir)) });
await marker.default({ module_or_path: await readFile(new URL('beta_marker_wasm_bg.wasm', dir)) });
const call = request => JSON.parse(pattern.style_command(JSON.stringify(request)));
const chart = call({ action: 'size_chart', preset: 'indian_adult' });
chart.sizes = ['M']; chart.measurements = { M: chart.measurements.M };
const document = call({ action: 'create', request: { block: 't_shirt', chart } });
assert.equal(document.pieces.length, 4);
assert.equal(document.pieces.find(piece => piece.id === 'front').annotations.length, 4);
const graded = call({ action: 'grade', document });
assert.equal(graded.sizes.M.length, 4);
const reports = call({ action: 'block_seams', document });
assert.ok(reports.every(report => report.rows.every(row => row.pass)));
const settings = {
  fabric: { width: 750, layout: 'folded', nap: 'one_way', repeat: { period_x: 10, tolerance_mm: 0.1 } },
  spacing: { piece_gap: 3, edge_gap: 5 }, ratio: { M: 1 },
  lay: { face: 'up', plies: 2, end_allowance_mm: 15 },
  matches: [{ a: { piece_id: 'front', point_id: 'side_hem' }, b: { piece_id: 'back', point_id: 'side_hem' }, axes: 'x' }],
};
const job = call({ action: 'to_job', document, settings });
const session = new marker.NestSession(JSON.stringify(job), JSON.stringify({ time_limit_s: 3, round_time_s: 2, seed: 42 }));
let progress;
do {
  progress = JSON.parse(session.step());
  const validation = JSON.parse(marker.validate(JSON.stringify(job), JSON.stringify(progress.result)));
  assert.ok(validation.ok, JSON.stringify(validation.errors));
  await new Promise(resolve => setTimeout(resolve, 0));
} while (!progress.done);
session.free();
const overlay = call({ action: 'overlay', job, result: progress.result, options: {} });
assert.ok(overlay.stripes.length > 0);
assert.equal(overlay.outlines.length, progress.result.placements.length);
assert.throws(() => call({ action: 'edit', document, operations: [{ op: 'delete_object', id: 'front.A' }] }));
// Round 0.7 block quality and spec sheet actions, all five sizes, every block.
const full = call({ action: 'size_chart', preset: 'indian_adult' });
let checked = 0;
for (const block of ['t_shirt', 'shirt', 'trouser', 'a_line_skirt', 'kurta', 'polo', 'track_pant']) {
  const doc = call({ action: 'create', request: { block, chart: full } });
  const report = call({ action: 'block_checks', document: doc });
  assert.equal(report.sizes.length, 5);
  assert.ok(report.pass, `${block}: ${JSON.stringify(report.sizes.map(s => [s.size, s.errors, s.checks.filter(c => !c.pass).map(c => c.id)]))}`);
  checked += report.sizes.reduce((n, s) => n + s.checks.length, 0);
  const spec = call({ action: 'spec_sheet', document: doc });
  assert.equal(spec.schema, 'beta-pattern.spec/v1');
  assert.deepEqual(spec.sizes, full.sizes);
  assert.ok(spec.rows.every(row => row.values.length === 5 && row.values[0].grade_step_mm === null && row.values.every(v => v.pass !== false)));
  for (const [format, magic] of [['pdf', [0x25, 0x50, 0x44, 0x46]], ['csv', [0xef, 0xbb, 0xbf]], ['xlsx', [0x50, 0x4b, 3, 4]]]) {
    const file = call({ action: 'export_spec_sheet', document: doc, format });
    assert.equal(file.filename, `measurement-spec.${format}`);
    assert.deepEqual(file.bytes.slice(0, magic.length), magic, `${block} ${format}`);
  }
}
const trouser = call({ action: 'spec_sheet', document: call({ action: 'create', request: { block: 'trouser', chart: full } }) });
assert.ok(trouser.rows.some(row => row.id === 'thigh_half'));
// Replacement dart: the archived source is not cut and no longer reports green block checks.
const shirt = call({ action: 'create', request: { block: 'shirt', chart } });
// Side-seam midpoint of the M front (test-only arithmetic on Rust output), apex 55 mm inside.
const side = call({ action: 'grade', document: shirt }).sizes.M.find(p => p.piece.id === 'front-M').seam_edges[1];
const lengths = side.slice(1).map((p, i) => Math.hypot(p[0] - side[i][0], p[1] - side[i][1]));
const half = lengths.reduce((a, b) => a + b, 0) / 2;
let walked = 0, at = side[0];
for (const [i, l] of lengths.entries()) {
  if (walked + l >= half) { const t = (half - walked) / l; at = [side[i][0] + t * (side[i + 1][0] - side[i][0]), side[i][1] + t * (side[i + 1][1] - side[i][1])]; break; }
  walked += l;
}
const darted = call({ action: 'edit', document: shirt, operations: [
  { op: 'put_object', id: 'dart_apex', object: { kind: 'point', operation: { op: 'absolute', x: at[0] - 55, y: at[1] } } },
  { op: 'piece_action', request: { action: 'dart', piece: 'front', id: 'front_dart', size: 'M', seam_edge: 1, center: half, width: 20, apex: 'dart_apex' } },
] });
const source = darted.pieces.find(piece => piece.id === 'front');
assert.equal(source.quantity + source.mirror_quantity, 0);
assert.ok(darted.meta.block_review_required);
assert.equal(call({ action: 'block_checks', document: darted }).pass, false);
assert.throws(() => call({ action: 'spec_sheet', document: darted }));
console.log(JSON.stringify({ status: 'passed', sizes: 1, block_checks: checked, placements: progress.result.placements.length,
  stripes: overlay.stripes.length, rounds: progress.round, seconds: progress.seconds }));
