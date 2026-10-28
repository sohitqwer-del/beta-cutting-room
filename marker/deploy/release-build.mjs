// Release builds without developer paths. No packages or system tools are installed.
import { spawnSync } from 'node:child_process';
import { readFileSync, realpathSync } from 'node:fs';
import { homedir } from 'node:os';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const marker = fileURLToPath(new URL('../', import.meta.url));
const workspace = resolve(marker, '..');

/** Cargo splits RUSTFLAGS on whitespace; encoded flags preserve paths with spaces. */
export function releaseEnvironment(env = process.env, root = workspace, cargoHome = env.CARGO_HOME || join(homedir(), '.cargo')) {
  const flags = env.CARGO_ENCODED_RUSTFLAGS !== undefined
    ? env.CARGO_ENCODED_RUSTFLAGS.split('\x1f').filter(Boolean)
    : (env.RUSTFLAGS || '').split(/\s+/).filter(Boolean);
  for (const [source, destination] of [[resolve(cargoHome), '/cargo'], [resolve(root), '/build']]) {
    // rustc diagnostics and include paths can use either separator on Windows.
    for (const prefix of new Set([source, source.replaceAll('\\', '/')])) {
      flags.push(`--remap-path-prefix=${prefix}=${destination}`);
    }
  }
  const result = { ...env, CARGO_HOME: resolve(cargoHome), CARGO_ENCODED_RUSTFLAGS: flags.join('\x1f'), CARGO_INCREMENTAL: '0' };
  delete result.RUSTFLAGS;
  return result;
}

/** Scan actual executable bytes, including UTF-16 paths, rather than a build log. */
export function localPathLeaks(bytes, roots = [workspace, process.env.CARGO_HOME || join(homedir(), '.cargo')]) {
  const utf8 = bytes.toString('utf8').toLowerCase();
  const utf16 = bytes.subarray(0, bytes.length - bytes.length % 2).toString('utf16le').toLowerCase();
  const utf16Odd = bytes.subarray(1, 1 + Math.floor((bytes.length - 1) / 2) * 2).toString('utf16le').toLowerCase();
  const needles = ['C:\\Users\\', 'C:/Users/', '/home/', ...roots.flatMap(root => [resolve(root), resolve(root).replaceAll('\\', '/')])];
  return [...new Set(needles)].filter(needle => [utf8, utf16, utf16Odd].some(text => text.includes(needle.toLowerCase())));
}

function main(args) {
  if (args[0] === '--verify' && args.length === 2) {
    const leaks = localPathLeaks(readFileSync(args[1]));
    if (leaks.length) throw new Error(`Local paths remain in the executable: ${leaks.join(', ')}`);
    console.log('PASS: no user, Cargo-home or workspace paths in executable bytes (UTF-8 / UTF-16).');
    return;
  }
  const mode = args[0] || 'cli';
  if (args.length > 1) throw new Error('Usage: node deploy/release-build.mjs [cli|desktop|marker-wasm|pattern-wasm|floor-wasm|cutroom-wasm] or --verify <binary>');
  const scripts = {
    desktop: 'apps/desktop/scripts/tauri.mjs',
    'marker-wasm': 'packages/marker-wasm/scripts/build-wasm.mjs',
    'pattern-wasm': 'packages/marker-wasm/scripts/build-pattern-wasm.mjs',
    'floor-wasm': 'apps/web/scripts/build-floor-wasm.mjs',
    'cutroom-wasm': 'packages/marker-wasm/scripts/build-cutroom-wasm.mjs',
  };
  if (mode !== 'cli' && !scripts[mode]) throw new Error(`Unknown release build: ${mode}`);
  if (!process.env.CARGO_TARGET_DIR) throw new Error('Set CARGO_TARGET_DIR to external build output.');
  const env = releaseEnvironment();
  if (!env.SOURCE_DATE_EPOCH) {
    const revision = spawnSync('git', ['log', '-1', '--format=%ct'], { cwd: workspace, encoding: 'utf8', windowsHide: true });
    if (revision.status !== 0 || !/^\d+$/.test(revision.stdout.trim())) throw new Error('Set SOURCE_DATE_EPOCH to the source commit timestamp when building an exported source archive.');
    env.SOURCE_DATE_EPOCH = revision.stdout.trim();
  }
  if (!/^\d+$/.test(env.SOURCE_DATE_EPOCH)) throw new Error('SOURCE_DATE_EPOCH must be whole Unix seconds.');
  if (env.CARGO_ENCODED_RUSTFLAGS.includes('target-cpu=native')) throw new Error('Release builds must not use target-cpu=native.');
  if (mode === 'cli') {
    const compiler = spawnSync('rustc', ['-vV'], { encoding: 'utf8', windowsHide: true });
    if (compiler.status !== 0) throw new Error('Cannot identify the release compiler.');
    const target = env.CARGO_BUILD_TARGET || /^host: (.+)$/m.exec(compiler.stdout)?.[1];
    // GNU PE timestamps otherwise change when the same object files are relinked.
    if (target?.endsWith('windows-gnu')) env.CARGO_ENCODED_RUSTFLAGS += '\x1f-C\x1flink-arg=-Wl,--no-insert-timestamp';
  }
  const command = mode === 'cli' ? 'cargo' : process.execPath;
  const commandArgs = mode === 'cli'
    ? ['build', '--locked', '--release', '-p', 'beta-marker-cli']
    : [resolve(marker, scripts[mode]), ...(mode === 'desktop' ? ['build'] : [])];
  const result = spawnSync(command, commandArgs, { cwd: mode === 'desktop' ? resolve(marker, 'apps/desktop') : marker, env, stdio: 'inherit', windowsHide: true });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
  if (mode === 'cli') {
    const target = env.CARGO_BUILD_TARGET;
    const windows = target ? target.includes('windows') : process.platform === 'win32';
    const binary = resolve(marker, env.CARGO_TARGET_DIR, ...(target ? [target] : []), 'release', `beta-marker${windows ? '.exe' : ''}`);
    const leaks = localPathLeaks(readFileSync(binary));
    if (leaks.length) throw new Error(`Local paths remain in release output: ${leaks.join(', ')}`);
    console.log('PASS: CLI release path scan.');
  }
}

if (process.argv[1] && pathToFileURL(realpathSync(process.argv[1])).href === import.meta.url) main(process.argv.slice(2));
