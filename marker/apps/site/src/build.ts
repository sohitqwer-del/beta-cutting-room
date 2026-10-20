// Static build: renders every page to plain HTML in dist/. Run with `node src/build.ts`.
// The output has no module scripts and no runtime fetches, so it works when opened from disk.
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { loadRuns } from './benchmark.ts';
import { clientScript, usesNetwork } from './client-build.ts';
import { layout, pages } from './layout.ts';
import type { Raw } from './html.ts';
import type { BuildContext } from './pages/context.ts';
import { homePage } from './pages/home.ts';
import { benchmarkPage } from './pages/benchmark.ts';
import { comparePage } from './pages/compare.ts';
import { installPage } from './pages/install.ts';
import { docsPage } from './pages/docs.ts';
import { openSourcePage } from './pages/open-source.ts';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const dist = join(root, 'dist');
const dataDir = join(root, 'data', 'benchmark');
const publicDir = join(root, 'public');
const manualSrc = join(root, '..', '..', 'docs', 'CUTTING-ROOM-MANUAL.html');

const files = readdirSync(dataDir).filter((f) => f.endsWith('.json')).sort().map((name) => ({ name, text: readFileSync(join(dataDir, name), 'utf8') }));
const { runs, errors } = loadRuns(files);
if (errors.length) {
  for (const e of errors) console.error(`data/benchmark/${e.file}:\n  ${e.errors.join('\n  ')}`);
  console.error('Benchmark data is invalid. See data/benchmark/SCHEMA.md.');
  process.exit(1);
}

const screensDir = join(publicDir, 'assets', 'screens');
const screens = new Set(existsSync(screensDir) ? readdirSync(screensDir).filter((f) => f.endsWith('.png')) : []);
const ctx: BuildContext = { screens, runs };

rmSync(dist, { recursive: true, force: true });
mkdirSync(join(dist, 'assets'), { recursive: true });
cpSync(publicDir, dist, { recursive: true, filter: (src) => !src.endsWith('README.md') });
writeFileSync(join(dist, 'assets', 'site.css'), readFileSync(join(root, 'src', 'style.css')));
writeFileSync(join(dist, 'assets', 'site.js'), clientScript(readFileSync(join(root, 'src', 'client.ts'), 'utf8')));
const hasManual = existsSync(manualSrc);
if (hasManual) {
  mkdirSync(join(dist, 'docs'), { recursive: true });
  cpSync(manualSrc, join(dist, 'docs', 'cutting-room-manual.html'));
}

const renderers: Record<string, () => Raw> = {
  'index.html': () => homePage(ctx),
  'benchmark.html': () => benchmarkPage(ctx),
  'compare.html': () => comparePage(),
  'install.html': () => installPage(),
  'docs.html': () => docsPage(hasManual),
  'open-source.html': () => openSourcePage(),
};

for (const meta of pages) {
  const render = renderers[meta.file];
  if (!render) throw new Error(`No renderer for ${meta.file}`);
  writeFileSync(join(dist, meta.file), layout(meta, render()));
}

// Guard: no page may load anything from another server (links to other sites are fine).
const remote = /<(?:script|link|img|iframe|source|video|audio|embed|object)\b[^>]*\b(?:src|href|data)\s*=\s*["']?(?:https?:)?\/\//i;
const cssRemote = /url\(\s*["']?(?:https?:)?\/\//i;
const problems: string[] = [];
const walk = (dir: string): string[] => readdirSync(dir, { withFileTypes: true }).flatMap((d) => (d.isDirectory() ? walk(join(dir, d.name)) : [join(dir, d.name)]));
for (const file of walk(dist)) {
  if (!/\.(html|css|svg|js)$/.test(file)) continue;
  const text = readFileSync(file, 'utf8');
  if (remote.test(text) || cssRemote.test(text) || /@import\s/.test(text)) problems.push(file);
  if (file.endsWith('.js') && usesNetwork(text)) problems.push(file);
}
if (problems.length) {
  console.error(`These files load remote resources:\n  ${problems.join('\n  ')}`);
  process.exit(1);
}

const statuses = runs.map((r) => `${r.id} (${r.status})`).join(', ');
console.log(`Built ${pages.length} pages into dist/. Benchmark runs: ${statuses}. Screenshots found: ${screens.size}.`);
