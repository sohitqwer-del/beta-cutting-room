import { spawnSync } from 'node:child_process';
import { mkdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
const marker = fileURLToPath(new URL('../../../', import.meta.url));
const pkg = resolve(marker, 'packages/marker-wasm/pkg');
if (!process.env.CARGO_TARGET_DIR) throw new Error('Set CARGO_TARGET_DIR outside the source checkout.');
const env = { ...process.env, CARGO_BUILD_JOBS: process.env.CARGO_BUILD_JOBS || '3', CARGO_PROFILE_RELEASE_OPT_LEVEL: 'z', CARGO_PROFILE_RELEASE_LTO: 'fat', CARGO_PROFILE_RELEASE_CODEGEN_UNITS: '1', CARGO_PROFILE_RELEASE_PANIC: 'abort' };
function run(command, args) { const p = spawnSync(command, args, { cwd: marker, env, stdio: 'inherit' }); if (p.error) throw p.error; if (p.status !== 0) process.exit(p.status ?? 1); }
run('cargo', ['build', '--manifest-path', 'crates/beta-marker-floor/Cargo.toml', '--lib', '--target', 'wasm32-unknown-unknown', '--release']);
mkdirSync(pkg, { recursive: true });
run(process.env.WASM_BINDGEN || 'wasm-bindgen', [resolve(marker, env.CARGO_TARGET_DIR, 'wasm32-unknown-unknown/release/beta_marker_floor.wasm'), '--target', 'web', '--out-dir', pkg]);
