# Privacy and local data

BETA Cutting Room is open source and can run locally or be self-hosted. It needs no cloud account,
analytics, third-party CDN or runtime licence check. BETA's code makes no
outbound service requests or automatic factory-data uploads. Build tools may download dependencies; administrators
may separately configure HTTPS certificate services or updates.

Windows, the browser and the separately installed WebView2 Runtime have their
own update and diagnostic settings. These are separate from BETA's code. Review
them during deployment; an offline app-page test does not prove that the whole
PC makes no network requests. See Microsoft's
[WebView2 data and privacy documentation](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/data-privacy).

## Where work is stored

- **Desktop:** saved project folders, recent-file settings and recovery copies
  stay on the PC. Projects can include patterns, markers, plans and floor drafts.
  Autosave and the previous good copy can retain earlier work.
- **Browser:** drafts/settings stay in browser storage, per profile and origin.
  A factory HTTP server is not a shared project library.
  The static browser edition computes locally without uploading files to an API.
  Save/restore backup files include cutting-room records; see
  [browser storage and backup](BROWSER-CUTROOM-API.md#storage-and-backup).
- **CLI:** inputs and outputs use operator-chosen paths. Persistent queues and
  exports write the files requested by their commands.
- **Self-hosted API:** uploads travel to the configured factory server for
  computation, not a cloud service. The process holds data in memory; proxy
  logs, OS temporary files, swap, crash dumps and backups can still retain data
  under the site's administration.

Projects, PDFs, DXFs, spreadsheets, tickets and QR payloads may contain style
names, purchase orders, sizes, rolls and production data. Review them before
sharing. Exported files are outside the app's access control.

## Access and transport

The desktop exposes no HTTP listener. `beta-marker serve` defaults to loopback;
local software can reach an unprotected loopback API. Set a token to restrict
local access too. Network binding requires one. Use HTTPS and firewall rules as
described in [self-hosting](SELF-HOSTING.md). The shared token has no user roles
or per-person audit log. The browser keeps it in the tab's session storage;
choose **Forget token** on a shared PC. BETA does not encrypt project files or browser storage;
use OS accounts and disk encryption when needed. A user with PC/profile access
may be able to read drafts.

## Backups, deletion and support

Back up projects and browser exports to factory-controlled storage. Test recovery.
Deleting a project does not remove browser copies, desktop recovery, queue files,
exports or backups. Clear each location under the site's retention policy.
Clearing browser data can destroy the only copy of an unsaved draft.

No telemetry or automatic error report is sent. Remove private names, geometry,
QR contents and tokens before sharing support logs or screenshots. Use synthetic
or openly licensed examples in public bug reports. Never commit factory patterns,
datasets or production records without written permission.
