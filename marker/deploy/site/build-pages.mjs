// Build one upload-ready GitHub Pages folder. This never publishes anything.
// Usage: node marker/deploy/site/build-pages.mjs --out <folder> [--video <demo.web.mp4>]
import { spawnSync } from 'node:child_process';
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { delimiter, dirname, join, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { releaseEnvironment } from '../release-build.mjs';
const marker = fileURLToPath(new URL('../../', import.meta.url));
const root = resolve(marker, '..');
const args = process.argv.slice(2);
const option = (name, fallback) => { const i = args.indexOf(name); return i < 0 ? fallback : args[i + 1]; };
const out = resolve(option('--out', join(marker, 'out/pages')));
// Only replace a prior Pages output created by this script, never an arbitrary folder.
const stamp = '.beta-pages-build';
if (out === root || out === marker || root.startsWith(out + sep)) throw new Error('Unsafe Pages output folder.');
if (existsSync(out) && readdirSync(out).length && !existsSync(join(out, stamp))) throw new Error('Output is not an existing Pages build. Choose an empty folder.');
const nearbyVideo = process.env.CARGO_TARGET_DIR ? resolve(process.env.CARGO_TARGET_DIR, '../../demo-video/beta-demo-en.web.mp4') : undefined;
const video = option('--video', process.env.BETA_DEMO_VIDEO || (nearbyVideo && existsSync(nearbyVideo) ? nearbyVideo : undefined));
if (video && !existsSync(video)) throw new Error(`Video does not exist: ${video}`);
const publicVideo = join(marker, 'apps/site/public/assets/video/beta-demo-en.mp4');
const env = { ...releaseEnvironment(), BETA_WEB_BASE: '/beta-cutting-room/app/', ...(video ? { BETA_SITE_VIDEO: '1' } : {}) };
function run(workspace, script) {
  const npmArgs = ['run', script, '-w', workspace];
  // Run npm's JavaScript entry point on Windows without a command shell.
  const npmCli = process.platform === 'win32' ? [process.env.npm_execpath, ...[dirname(process.execPath), ...(process.env.PATH || '').split(delimiter)].map(folder => join(folder, 'node_modules/npm/bin/npm-cli.js'))].find(path => path?.endsWith('.js') && existsSync(path)) : undefined;
  if (process.platform === 'win32' && !npmCli) throw new Error('Cannot locate npm-cli.js; run through npm or put Node/npm on PATH.');
  const result = spawnSync(npmCli ? process.execPath : 'npm', npmCli ? [npmCli, ...npmArgs] : npmArgs, { cwd: root, env, stdio: 'inherit', windowsHide: true });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${workspace} ${script} failed (${result.status})`);
}
// Tell the site renderer that the local video will be in the assembled output.
run('@beta/marker-web', 'build:static');
run('@beta/marker-site', 'build');
if (existsSync(out)) rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
writeFileSync(join(out, stamp), 'BETA Cutting Room Pages output; safe to rebuild.\n');
cpSync(join(marker, 'apps/site/dist'), out, { recursive: true });
cpSync(join(marker, 'out/static-app'), join(out, 'app'), { recursive: true });
if (video) {
  const target = join(out, 'assets/video/beta-demo-en.mp4');
  mkdirSync(dirname(target), { recursive: true });
  cpSync(resolve(video), target);
  // Rendered site already supports local video. Re-render with the flag when staging a supplied file.
  const home = join(out, 'index.html');
  if (!readFileSync(home, 'utf8').includes('<video')) throw new Error('The site did not render the supplied local video.');
} else if (!existsSync(publicVideo)) console.log('Demo video absent; site omits its player.');
writeFileSync(join(out, '.nojekyll'), '');
const files = dir => readdirSync(dir, { withFileTypes: true }).flatMap(e => e.isDirectory() ? files(join(dir, e.name)) : [join(dir, e.name)]);
const built = files(out);
console.log(JSON.stringify({ folder: out, files: built.length, bytes: built.reduce((sum, f) => sum + statSync(f).size, 0), appBase: env.BETA_WEB_BASE, video: Boolean(video || existsSync(publicVideo)) }, null, 2));
