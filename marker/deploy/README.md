# Server and portable packaging

These files are examples. They do not install or start a service. See
[self-hosting](../docs/SELF-HOSTING.md) for access control, HTTPS and backups.

- `beta-cutting-room.service`: a systemd **user** unit; no root service.
- `Caddyfile`: an HTTPS reverse proxy to a loopback backend.
- `nginx.conf.example`: an alternative HTTPS route.
- `package-portable.ps1`: copies an already built Windows CLI and web app to a ZIP.
- `package-linux.py`: combines the verified Linux executable with that staging
  folder's matching web assets, docs and notices in a portable tar.gz.
- `generate-licenses.py`: resolves local Cargo/npm metadata into the licence inventory.
- `verify-server.py`: starts and reaps its own temporary loopback CLI process.
- `browser-smoke.mjs`: optional headless token-dialog check against a loopback
  server; it can start and stop its own CLI child. It uses the repository's CDP
  screenshot workflow and needs no npm browser dependency.

Build the Linux server with `cargo build --release -p beta-marker-cli` from
`marker/`. It needs no desktop GUI libraries. Do not enable `target-cpu=native`
for binaries sent to other machines. Build the web assets on a development
machine, then copy the executable and `apps/web/dist/` together. Rust and Node.js
are build tools; neither is needed on a server receiving the complete package.

The Windows desktop build uses the existing Tauri script, NSIS and its zlib
compressor. It skips WebView2 downloads. WebView2 must already be installed.
Preserve the Apache-2.0 `dom_query` and `option-ext` replacements and pinned Tauri
versions. The Windows licence gate runs before the desktop build.

No Docker installation is needed. No container image is published by these files.

The examples use a 60 MB proxy limit. The application still enforces 50 MiB
(30 MiB for photos), so it may reject a request accepted by the proxy. Caddy
uses a total body-read deadline; nginx's body timeout measures idle gaps, while
the backend also has a total deadline. Validate each configuration with the
installed proxy version before use. No proxy service is started during release QA.
See [Caddy server options](https://caddyserver.com/docs/caddyfile/options) and
[nginx HTTP options](https://nginx.org/en/docs/http/ngx_http_core_module.html).
