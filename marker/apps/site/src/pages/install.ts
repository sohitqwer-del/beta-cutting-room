import { html, type Raw } from '../html.ts';
import { ext, pageHead, section } from '../layout.ts';
import { site } from '../config.ts';
import { downloadButton } from './home.ts';

export function installPage(): Raw {
  const win = site.downloads.windows;
  return html`
${pageHead('Install and self-host', 'BETA Cutting Room runs on computers you control. Install the desktop app on a Windows PC, or run the server on your own machine. No account, no cloud and no internet connection are needed for daily work.', 'Self-hosting')}

<div class="wrap">
  <nav class="toc" aria-label="On this page"><ul>
    <li><a href="#windows">Windows desktop app</a></li><li><a href="#server">Server</a></li><li><a href="#requirements">System requirements</a></li><li><a href="#privacy">Privacy</a></li><li><a href="#offline">Offline use</a></li>
  </ul></nav>
</div>

${section('windows', 'Windows desktop app', html`
<div class="split">
  <ol class="steps">
    <li><h3>Download the installer</h3><p>File: <code>${win.file}</code> (${win.size}).</p>${downloadButton(win.url, 'Download for Windows')}</li>
    <li><h3>Check the file (recommended)</h3><p>Open PowerShell in your Downloads folder and run:</p><pre><code>Get-FileHash -Algorithm SHA256 '.\\${win.file}'</code></pre><p>The result must match the published checksum${win.sha256 ? html`: <code class="break">${win.sha256}</code>` : html` <span class="placeholder-text">(published with the download)</span>`}.</p></li>
    <li><h3>Run the installer</h3><p>It installs for your Windows user only, so you do not need administrator rights. The installer is not code-signed yet, so Windows SmartScreen may warn you. If the checksum matches, choose <strong>More info</strong>, then <strong>Run anyway</strong>.</p></li>
    <li><h3>Check WebView2 (Windows 10 only)</h3><p>The app uses Microsoft’s WebView2 runtime to draw its screens. Windows 11 already has it. Most Windows 10 PCs have it too. If the app does not open on Windows 10, install the <strong>WebView2 Runtime, Evergreen Standalone Installer</strong> from Microsoft. You can copy that installer to the factory PC on a USB drive; the BETA installer never downloads anything.</p></li>
    <li><h3>Start working</h3><p>Open <strong>BETA Cutting Room</strong> from the Start menu. Open a DXF pattern or a project and go from Pattern to Bundles. Projects (<code>.betaproj</code>) save automatically every few seconds and can be recovered after a crash.</p></li>
  </ol>
  <aside class="callout callout-plain">
    <h3>Uninstall</h3>
    <p>Windows Settings → Apps → Installed apps → BETA Cutting Room → Uninstall. Your project files stay where you saved them.</p>
    <h3>Updates</h3>
    <p>Install the new version over the old one. Copy it by USB if the PC has no internet.</p>
  </aside>
</div>`)}

${section('server', 'Server: beta-marker serve', html`
<div class="prose">
  <p>The <code>beta-marker</code> command runs nesting, checks and exports, and can serve the full app to a web browser. Use it on a Linux or Windows machine where you prefer a browser to the desktop app, or for batch work and benchmarks.</p>
  <div class="callout callout-warn" role="note"><p><strong>Factory network:</strong> by default the server listens only on the same computer (address <code>127.0.0.1</code>). To let other factory PCs connect, start it with <code>--bind</code> and an access token (<code>--token</code> or <code>BETA_TOKEN</code>); it will not start on the network without one. The server has no HTTPS of its own, so put it behind Caddy or nginx and a firewall. Never expose the server to the internet. See <code>marker/docs/SELF-HOSTING.md</code>.</p></div>
  <h3>Build from source</h3>
  <p>A ready-made Linux server download is ${site.downloads.server.url ? html`available on the <a href="index.html#download">download section</a>` : 'coming soon'}. It needs a recent Linux (glibc 2.43, for example Ubuntu 26.04). On other systems, build it from the source code. You need <strong>Rust</strong> (with the <code>wasm32-unknown-unknown</code> target and a matching <code>wasm-bindgen</code> tool) and <strong>Node.js</strong>.</p>
<pre><code># From the repository root: install and build the web app
npm install
npm run build:wasm -w @beta/marker-wasm
npm run build:pattern-wasm -w @beta/marker-wasm
npm run build -w @beta/marker-web

# From marker/: build and start the server
cargo build --release -p beta-marker-cli
cargo run --release -p beta-marker-cli -- serve --port 7878</code></pre>
  <p>Open the address the server prints (for example <code>http://127.0.0.1:7878</code>) in a browser on the same computer. Stop it with <kbd>Ctrl</kbd>+<kbd>C</kbd>. Full details: <code>marker/apps/web/README.md</code>.</p>
  <h3>Command-line work</h3>
  <p>Run <code>beta-marker --help</code> to see every command: import, nest, validate, export, plan, roll, cutroom, batch and bench. Each command also has its own <code>--help</code>.</p>
</div>`, { cls: 'section-alt' })}

${section('requirements', 'System requirements', html`
<div class="table-wrap" tabindex="0" role="region" aria-label="System requirements"><table class="data-table">
<caption class="sr-only">Minimum and suggested systems</caption>
<thead><tr><th scope="col"></th><th scope="col">Desktop app</th><th scope="col">Server</th></tr></thead>
<tbody>
<tr><th scope="row">Operating system</th><td>Windows 10 (22H2) or Windows 11, 64-bit</td><td>Linux or Windows, 64-bit</td></tr>
<tr><th scope="row">Processor</th><td>Any modern 64-bit processor. More cores give better markers in the same time.</td><td>Many cores help when several nests run at once.</td></tr>
<tr><th scope="row">Memory</th><td>8 GB suggested</td><td>8 GB or more suggested</td></tr>
<tr><th scope="row">Disk</th><td>About 12 MB to download, plus space for your projects</td><td>Space for the build and your data</td></tr>
<tr><th scope="row">Other</th><td>Microsoft WebView2 runtime (built into Windows 11)</td><td>Rust and Node.js to build from source</td></tr>
<tr><th scope="row">Internet</th><td>Not needed</td><td>Not needed after the build</td></tr>
</tbody></table></div>
<p class="small muted">Memory and processor values are suggestions, not tested minimums. We tested the installer on Windows 11. A clean Windows 10 PC has not been tested yet.</p>`)}

${section('privacy', 'Privacy', html`
<div class="prose">
  <ul class="check-list">
    <li>Your patterns, markers, orders and roll data stay on your computers. BETA has no cloud service.</li>
    <li>The app does not send usage data, crash reports or files anywhere. There is no tracking and no account.</li>
    <li>The server listens only on the computer it runs on.</li>
    <li>Files you export (PLT, PDF, CSV, Excel) go only where you save them.</li>
    <li>This website has no trackers or cookies and loads nothing from other servers.</li>
  </ul>
  <p>If you send us sample markers for a comparison, we keep them private and never publish them without your written permission.</p>
</div>`, { cls: 'section-alt' })}

${section('offline', 'Offline use', html`
<div class="prose">
  <p>Everything the app needs is inside the installer: screens, help, fonts and the Rust engine. Daily work needs no internet connection. You can install and update from a USB drive. Help pages are included, and this website also works when opened from a folder on disk.</p>
  <p>For the WebView2 runtime on Windows 10, see ${ext('https://developer.microsoft.com/microsoft-edge/webview2/', 'Microsoft’s WebView2 page')}.</p>
</div>`)}
`;
}
