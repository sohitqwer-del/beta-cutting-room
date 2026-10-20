import { defineConfig } from 'vite';
import { existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join, resolve } from 'node:path';
import { createHash } from 'node:crypto';
const pkg = fileURLToPath(new URL('../../packages/marker-wasm/pkg/', import.meta.url));
const base = process.env.BETA_WEB_BASE || './';
if (base !== './' && (!base.startsWith('/') || base.startsWith('//') || !base.endsWith('/'))) {
  throw new Error('BETA_WEB_BASE must be ./ or a same-origin path ending in /.');
}
// Generated files stay optional: a viewer-only build must work before Rust is built.
export default defineConfig({
  base,
  server: { watch: { ignored: ['**/.qa/**'] }, proxy: { '/api': process.env.BETA_MARKER_API_URL || 'http://127.0.0.1:7878' } },
  plugins: [{
    name: 'local-marker-wasm',
    configureServer(server) {
      server.middlewares.use('/wasm', (request, response, next) => {
        const name = request.url?.split('?')[0]?.replace(/^\//, '');
        if (!name || !['beta_marker_wasm.js', 'beta_marker_wasm_bg.wasm', 'beta_pattern_wasm.js', 'beta_pattern_wasm_bg.wasm', 'beta_marker_floor.js', 'beta_marker_floor_bg.wasm'].includes(name) || !existsSync(join(pkg, name))) return next();
        response.setHeader('Content-Type', name.endsWith('.wasm') ? 'application/wasm' : 'text/javascript');
        response.end(readFileSync(join(pkg, name)));
      });
    },
    generateBundle() {
      if (!existsSync(pkg)) { this.warn('WASM has not been built. The viewer will work without live checks.'); return; }
      for (const name of readdirSync(pkg).filter(name => name.endsWith('.js') || name.endsWith('.wasm'))) {
        this.emitFile({ type: 'asset', fileName: `wasm/${name}`, source: readFileSync(join(pkg, name)) });
      }
    },
  }, {
    name: 'static-licences',
    generateBundle() {
      if (process.env.VITE_BETA_STATIC !== '1') return;
      for (const [fileName, relative] of [
        ['LICENSE.txt', '../../../LICENSE'],
        ['NOTICE.txt', '../../NOTICE'],
        ['THIRD-PARTY-LICENSES.txt', '../../THIRD-PARTY-LICENSES.md'],
        ['THIRD-PARTY-NOTICES.txt', '../desktop/THIRD_PARTY_NOTICES.txt'],
      ]) {
        this.emitFile({ type: 'asset', fileName, source: readFileSync(fileURLToPath(new URL(relative, import.meta.url))) });
      }
    },
  }, (() => {
    // Offline copy for phones: public/sw.js precaches every built file listed here (same server only).
    // Browsers only install a new service worker when sw.js itself changes, so the build stamps its version into it.
    let outDir = '';
    let version = '';
    return {
      name: 'offline-precache',
      enforce: 'post',
      configResolved(config: { root: string; build: { outDir: string } }) { outDir = resolve(config.root, config.build.outDir); },
      generateBundle(_options, bundle) {
        const pub = fileURLToPath(new URL('./public/', import.meta.url));
        const files = [...Object.keys(bundle), ...readdirSync(pub)].filter(f => !f.endsWith('.map') && f !== 'sw.js' && f !== 'precache.json').sort();
        const hash = createHash('sha256');
        for (const f of files) { hash.update(f); const item = bundle[f]; hash.update(item ? (item.type === 'chunk' ? item.code : item.source) : readFileSync(join(pub, f))); }
        version = hash.digest('hex').slice(0, 16);
        this.emitFile({ type: 'asset', fileName: 'precache.json', source: JSON.stringify({ version, files }) });
      },
      closeBundle() {
        const sw = join(outDir, 'sw.js');
        if (version && existsSync(sw)) writeFileSync(sw, `${readFileSync(sw, 'utf8').trimEnd()}\n// build ${version}\n`);
      },
    };
  })()],
});
