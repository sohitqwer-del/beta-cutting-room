// Cutting-room module: cut plans (with the nester), rolls, spreading, bundles, tickets, reports, PLT import.
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { mkdirSync, statSync } from 'node:fs';
import { resolve } from 'node:path';
const marker = fileURLToPath(new URL('../../../', import.meta.url));
const pkg = fileURLToPath(new URL('../pkg/', import.meta.url));
if (!process.env.CARGO_TARGET_DIR) throw new Error('Set CARGO_TARGET_DIR to a build directory with enough free space.');
// The planner nests many markers: optimize for speed, like the marker module.
const env = { ...process.env, CARGO_BUILD_JOBS: process.env.CARGO_BUILD_JOBS || '3', CARGO_PROFILE_RELEASE_OPT_LEVEL: process.env.CARGO_PROFILE_RELEASE_OPT_LEVEL || '3',
  CARGO_PROFILE_RELEASE_LTO: 'fat', CARGO_PROFILE_RELEASE_CODEGEN_UNITS: '1', CARGO_PROFILE_RELEASE_PANIC: 'abort', CARGO_PROFILE_RELEASE_STRIP: 'debuginfo' };
function run(command, args) {
  const result = spawnSync(command, args, { cwd: marker, env, stdio: 'inherit' });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
run('cargo', ['build', '-p', 'beta-marker-cutroom-wasm', '--lib', '--target', 'wasm32-unknown-unknown', '--release']);
mkdirSync(pkg, { recursive: true });
run(process.env.WASM_BINDGEN || 'wasm-bindgen', [resolve(marker, env.CARGO_TARGET_DIR,
  'wasm32-unknown-unknown/release/beta_marker_cutroom_wasm.wasm'), '--target', 'web', '--out-dir', pkg]);
console.log(`WASM: ${statSync(resolve(pkg, 'beta_marker_cutroom_wasm_bg.wasm')).size} bytes`);
