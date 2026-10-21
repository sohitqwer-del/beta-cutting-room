// Headless test of the real generated WASM modules, including the worker session API.
import { readFile } from 'node:fs/promises';
import assert from 'node:assert/strict';
import patternInit, * as pattern from '../pkg/beta_pattern_wasm.js';
import markerInit, * as marker from '../pkg/beta_marker_wasm.js';
await patternInit({ module_or_path: await readFile(new URL('../pkg/beta_pattern_wasm_bg.wasm', import.meta.url)) });
await markerInit({ module_or_path: await readFile(new URL('../pkg/beta_marker_wasm_bg.wasm', import.meta.url)) });
const call = request => JSON.parse(pattern.style_command(JSON.stringify(request)));
const chart = call({ action: 'size_chart', preset: 'indian_adult' });
chart.sizes = ['M']; chart.measurements = { M: chart.measurements.M };
const document = call({ action: 'create', request: { block: 't_shirt', chart } });
assert.equal(document.pieces.length, 4);
assert.equal(document.pieces.find(piece => piece.id === 'front').annotations.length, 2);
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
console.log(JSON.stringify({ status: 'passed', sizes: 1, placements: progress.result.placements.length,
  stripes: overlay.stripes.length, rounds: progress.round, seconds: progress.seconds }));
