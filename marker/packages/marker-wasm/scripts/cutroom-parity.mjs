// Compare the cutting-room WASM module with native reference files, byte for byte.
// cargo run -p beta-marker-api --example cutroom_parity -- <output-directory>
// node packages/marker-wasm/scripts/cutroom-parity.mjs <same-directory>
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import assert from 'node:assert/strict';
import { inflateRawSync } from 'node:zlib';
import init, * as cutroom from '../pkg/beta_marker_cutroom_wasm.js';
await init({ module_or_path: await readFile(new URL('../pkg/beta_marker_cutroom_wasm_bg.wasm', import.meta.url)) });
const dir = resolve(process.argv[2]);
const cases = JSON.parse(await readFile(resolve(dir, 'cases.json'), 'utf8'));
// Planning time is the only value that may differ between two runs.
const withoutRuntime = text => text.replace(/"runtime_s":[^,}]*/g, '"runtime_s":0');
/** XLSX entries by name, from the ZIP central directory. */
function zipEntries(buf) {
  const end = buf.lastIndexOf(Buffer.from([0x50, 0x4b, 0x05, 0x06]));
  const count = buf.readUInt16LE(end + 10); let at = buf.readUInt32LE(end + 16); const out = new Map();
  for (let i = 0; i < count; i++) {
    const method = buf.readUInt16LE(at + 10), size = buf.readUInt32LE(at + 20), nameLen = buf.readUInt16LE(at + 28);
    const extra = buf.readUInt16LE(at + 30), comment = buf.readUInt16LE(at + 32), local = buf.readUInt32LE(at + 42);
    const name = buf.toString('utf8', at + 46, at + 46 + nameLen);
    const start = local + 30 + buf.readUInt16LE(local + 26) + buf.readUInt16LE(local + 28);
    const data = buf.subarray(start, start + size);
    out.set(name, method === 8 ? inflateRawSync(data) : Buffer.from(data));
    at += 46 + nameLen + extra + comment;
  }
  return out;
}
/** Workbooks carry their creation time (docProps/core.xml); every other entry must match. */
function sameWorkbook(actual, expected, name) {
  const a = zipEntries(actual), e = zipEntries(expected);
  assert.deepEqual([...a.keys()].sort(), [...e.keys()].sort(), `${name}: entries`);
  for (const [entry, bytes] of e) {
    if (entry === 'docProps/core.xml') continue;
    assert.deepEqual(a.get(entry), bytes, `${name}: ${entry}`);
  }
}
const counts = new Map();
for (const c of cases) {
  const base = c.file.replace(/\.(bin|json)$/, '');
  const request = await readFile(resolve(dir, `${base}.request`));
  const expected = await readFile(resolve(dir, c.file));
  if (c.kind === 'plan') {
    let progress = 0;
    const actual = cutroom.plan(request.toString('utf8'), () => { progress++; return true; });
    assert.ok(progress > 0, `${c.name}: no progress`);
    assert.equal(withoutRuntime(actual), expected.toString('utf8'), c.name);
    const plan = JSON.parse(actual);
    for (const lay of plan.lays) assert.ok(lay.plies >= lay.passes && lay.plies % lay.passes === 0, `${c.name}: plies/passes`);
    console.log(`PASS: ${c.name} identical (${plan.lays.length} lays, ${plan.lays.map(l => `${l.plies} plies/${l.passes} passes`).join(', ')}, ${plan.total_fabric_m.toFixed(3)} m)`);
  } else {
    const reply = cutroom.api_request(c.url, request);
    try {
      assert.equal(reply.status, 200, `${c.name}: ${Buffer.from(reply.body).toString('utf8').slice(0, 200)}`);
      assert.equal(reply.mime, c.mime, `${c.name}: content type`);
      if (c.url.includes('format=xlsx')) sameWorkbook(Buffer.from(reply.body), expected, c.name);
      else assert.deepEqual(Buffer.from(reply.body), expected, c.name);
    } finally { reply.free(); }
  }
  const kind = c.kind === 'plan' ? 'plan' : c.url.split('?')[0].replace('/api/', '') + (c.url.includes('format=') ? `?${c.url.split('format=')[1].split('&')[0]}` : '');
  counts.set(kind, (counts.get(kind) ?? 0) + 1);
}
for (const [kind, count] of counts) console.log(`PASS: ${kind} ${count}/${count} identical`);
console.log(`PASS: ${cases.length}/${cases.length} native/WASM cutting-room results are byte-identical (XLSX: every entry except its creation time).`);
