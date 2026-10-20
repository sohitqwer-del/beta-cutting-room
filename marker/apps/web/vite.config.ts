import { defineConfig } from 'vite';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';
const pkg = fileURLToPath(new URL('../../packages/marker-wasm/pkg/', import.meta.url));
// Generated files stay optional: a viewer-only build must work before Rust is built.
export default defineConfig({
  base: './',
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
  }],
});
