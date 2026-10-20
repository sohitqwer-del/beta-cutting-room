// Isolate each throttled renderer trial. A stuck upstream call cannot block the
// matrix or discard the last Rust-validated checkpoint.
// node wasm-benchmark.mjs <cases.json> <output-dir> [round-seconds=2]
import { fork, spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync, appendFileSync, existsSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
const [casesPath, outputPath, roundArg = '2'] = process.argv.slice(2);
if (!casesPath || !outputPath) throw new Error('Pass cases.json and an output directory.');
const cases = JSON.parse(readFileSync(resolve(casesPath), 'utf8'));
const out = resolve(outputPath); mkdirSync(out, { recursive: true });
const results = join(out, 'wasm-results.jsonl');
// Resume only this explicitly selected output directory. Different round
// settings require a different directory; never mix measurements silently.
const recorded = existsSync(results) ? readFileSync(results, 'utf8').trim().split(/\r?\n/).filter(Boolean).map(JSON.parse) : [];
if (recorded.some(row => row.round_time_s !== Number(roundArg))) throw new Error('Round setting differs from saved results.');
const done = new Set(recorded.map(row => `${row.id}:${row.budget_s}`));
function terminateTree(child) {
  if (child.exitCode !== null) return;
  if (process.platform === 'win32') {
    spawnSync('taskkill', ['/PID', String(child.pid), '/T', '/F'], { windowsHide: true, stdio: 'ignore' });
  } else {
    try { process.kill(-child.pid, 'SIGKILL'); } catch (error) { if (error.code !== 'ESRCH') throw error; }
  }
}
async function trial(item, seconds) {
  const folder = join(out, `${String(item.id).replace(/[^a-zA-Z0-9_.-]/g, '_')}-${seconds}`);
  mkdirSync(folder, { recursive: true });
  const input = join(folder, 'case.json'); writeFileSync(input, JSON.stringify([item]));
  const script = fileURLToPath(new URL('./browser-test.mjs', import.meta.url));
  const child = fork(script, [folder, input, roundArg, String(seconds)], {
    windowsHide: true, detached: process.platform !== 'win32', stdio: ['ignore', 'pipe', 'pipe', 'ipc'],
  });
  let log = '', started, latest, row, metadata;
  const rounds = [];
  child.stdout.on('data', data => { log += data; });
  child.stderr.on('data', data => { log += data; });
  let timer = setTimeout(() => { terminateTree(child); }, 60000);
  child.on('message', message => {
    if (message.type === 'metadata') metadata = message.report;
    if (message.type === 'checkpoint') {
      latest = message.progress;
      rounds.push({ seconds: latest.round_seconds, memory_bytes: latest.memory_bytes });
    }
    if (message.type === 'start') {
      started = performance.now(); clearTimeout(timer);
      timer = setTimeout(() => {
        const wall_s = (performance.now() - started) / 1000;
        row = { id: item.id, budget_s: seconds, round_time_s: Number(roundArg), cpu_throttle_rate: 4,
          wall_s, watchdog: true, rounds };
        if (latest) {
          row.result = { ...latest.result, runtime_s: wall_s,
            warnings: [...(latest.result.warnings ?? []), 'Browser time limit reached; last independently checked marker retained.'] };
          row.memory_bytes = latest.memory_bytes;
          row.validation = { ok: true, source: 'Rust checkpoint; native replay follows the matrix' };
        } else {
          row.error = 'Deadline exceeded before a checked complete marker was available.';
          row.validation = { ok: false };
        }
        terminateTree(child);
      }, (seconds + 5) * 1000);
    }
    if (message.type === 'result') {
      row = message.row; clearTimeout(timer);
      if (row.error && latest) {
        row.recovered_error = row.error; delete row.error;
        row.result = { ...latest.result, runtime_s: row.wall_s,
          warnings: [...(latest.result.warnings ?? []), 'Browser search failed; last independently checked marker retained.'] };
        row.memory_bytes = latest.memory_bytes;
        row.validation = { ok: true, source: 'Rust checkpoint; native replay follows the matrix' };
      }
    }
  });
  await new Promise((resolve, reject) => {
    child.on('error', reject);
    child.on('exit', code => {
      clearTimeout(timer);
      writeFileSync(join(folder, 'trial.log'), log);
      if (metadata) writeFileSync(join(folder, 'metadata.json'), JSON.stringify(metadata, null, 2));
      if (!row) { reject(new Error(`Browser fixture failed for ${item.id}, exit ${code}. See ${folder}/trial.log`)); return; }
      appendFileSync(results, JSON.stringify(row) + '\n');
      console.log(`${item.id} ${seconds}s: ${row.result ? (row.result.efficiency * 100).toFixed(2) + '%' : 'FAILED'}, ${row.wall_s.toFixed(2)}s${row.watchdog ? ', checkpoint retained by watchdog' : ''}`);
      resolve();
    });
  });
}
for (const item of cases) {
  for (const seconds of [30, 60]) {
    if (!done.has(`${item.id}:${seconds}`)) await trial(item, seconds);
  }
}
