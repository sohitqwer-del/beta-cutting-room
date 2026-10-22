// Build a complete, copyable browser app. Nothing is uploaded or published.
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { existsSync, readdirSync, statSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { gzipSync } from 'node:zlib';
import { build } from 'vite';
const require = createRequire(import.meta.url);
const web = fileURLToPath(new URL('../', import.meta.url));
const marker = resolve(web, '../..');
const pkg = resolve(marker, 'packages/marker-wasm/pkg');
const outDir = resolve(marker, 'out/static-app');
// Vite may empty only this dedicated output directory in the current checkout.
if (relative(marker, outDir) !== join('out', 'static-app')) throw new Error('Unsafe static output path.');
const env = { ...process.env, CARGO_BUILD_JOBS: process.env.CARGO_BUILD_JOBS || '3' };
function run(script, args = []) {
  const result = spawnSync(process.execPath, [script, ...args], {
    cwd: web, env, stdio: 'inherit', windowsHide: true,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
if (env.BETA_SKIP_WASM_BUILD !== '1') {
  run(resolve(marker, 'packages/marker-wasm/scripts/build-wasm.mjs'));
  run(resolve(marker, 'packages/marker-wasm/scripts/build-pattern-wasm.mjs'));
  run(fileURLToPath(new URL('./build-floor-wasm.mjs', import.meta.url)));
  run(resolve(marker, 'packages/marker-wasm/scripts/build-cutroom-wasm.mjs'));
}
for (const module of ['beta_marker_wasm', 'beta_pattern_wasm', 'beta_marker_floor', 'beta_marker_cutroom_wasm']) {
  for (const suffix of ['.js', '_bg.wasm']) {
    if (!existsSync(join(pkg, module + suffix))) throw new Error(`Missing ${module + suffix}. Build all four WASM modules first.`);
  }
}
run(require.resolve('typescript/bin/tsc'), ['-p', join(web, 'tsconfig.json')]);
process.env.VITE_BETA_STATIC = '1';
process.env.BETA_WEB_BASE ||= './';
await build({ root: web, configFile: join(web, 'vite.config.ts'), build: { outDir, emptyOutDir: true } });
function files(dir) {
  return readdirSync(dir).flatMap(name => {
    const path = join(dir, name);
    return statSync(path).isDirectory() ? files(path) : [path];
  });
}
const assets = files(outDir).map(path => {
  const bytes = readFileSync(path);
  return { path: relative(outDir, path).replaceAll('\\', '/'), bytes: bytes.length,
    gzip_bytes: gzipSync(bytes, { level: 9 }).length };
});
const report = {
  folder: relative(marker, outDir).replaceAll('\\', '/'),
  base: process.env.BETA_WEB_BASE, files: assets.length,
  bytes: assets.reduce((sum, a) => sum + a.bytes, 0),
  gzip_bytes: assets.reduce((sum, a) => sum + a.gzip_bytes, 0),
  assets,
};
writeFileSync(join(dirname(outDir), 'static-build-report.json'), JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
