# Self-host BETA Cutting Room 0.4.0

BETA can run on a factory PC or server. It makes no outbound runtime requests.
An administrator controls access, HTTPS, backups and updates. The HTTP server
provides computation and static web assets. Browser drafts stay in each browser;
it is not a shared project database or a multi-user file store.

## One PC

Extract the portable package and run `start-server.cmd`. Open
`http://127.0.0.1:7878`. The default bind address is `127.0.0.1`. With no token
configured, local applications can call its API. Stop it with Ctrl+C.
From a source build, run inside `marker/`:

```sh
beta-marker serve --web-dir apps/web/dist --port 7878
```

## API authentication

`--bind` accepts an IPv4 or IPv6 address, not a hostname. A non-loopback address
requires `--token` or `BETA_TOKEN`; startup fails without one. A configured token
protects every `/api` and `/api/*` request, including health, status and unknown
API routes. Static web assets are public. Tokens in URLs or query strings are
not accepted. A CLI token overrides the environment value.

Use a cryptographically random token with at least 32 visible ASCII characters
and no spaces. For example, on Linux:

```sh
export BETA_TOKEN="$(openssl rand -hex 32)"
beta-marker serve --bind 127.0.0.1 --web-dir ./web
```

Keep the value in a protected environment file for a service. Do not commit it.
Avoid `--token` in shared shell history or process listings. A blank, short or
malformed configured token is an error even on loopback. Rotate it by changing
the environment file and restarting; clients must enter the replacement.
There is one shared token, with no per-user roles.

An API client sends the header on **every** request:

```sh
curl --fail --header "Authorization: Bearer $BETA_TOKEN" \
  http://127.0.0.1:7878/api/health
```

Missing or incorrect tokens receive HTTP 401 with `WWW-Authenticate: Bearer`.
The application checks authentication before its body read and route dispatch. There is no
cross-origin API access policy or token query parameter for browser embedding.
Use the web app and API through the same origin. The browser opens **Server
access token** when the API returns 401. Enter the administrator's token and
choose **Save and connect**. A wrong token opens the dialog again. The top-bar
**Server token** control lets you replace or forget it. It is stored only in
that tab's session storage, never in a URL or localStorage. Use **Forget token**
when leaving a shared PC.

## Factory network and HTTPS

The server itself has **no TLS**. A bearer token sent over plain HTTP can be
read by others on the network. Prefer a loopback backend behind Caddy or nginx
for HTTPS. Configure `BETA_TOKEN` even when the proxy connects over loopback.
Restrict network access to approved factory clients with a firewall or VPN.
Never expose the backend port directly to the Internet.

If a separate host must connect directly to the backend, use a specific private
IP with `--bind`, set the token, and secure that network hop. Binding to
`0.0.0.0` or `::` listens on all interfaces and prints a warning. A wildcard
address is a listening address, not the browser URL.

Examples are in [`deploy/`](../deploy/README.md). Replace the example hostname
and certificate paths. Forward the client's `Authorization` header unchanged.
If a proxy inserts a backend token instead, it **must authenticate every client
first**; otherwise it removes the protection. Never log authorization headers
or request bodies. Caddy certificate issuance is an administrator-managed
network operation, outside the offline app.

## Linux user service

Copy the binary to `~/.local/share/beta-cutting-room/beta-marker`, and the built
web directory to `~/.local/share/beta-cutting-room/web`. Keep both on the same
release. Create `~/.config/beta-cutting-room/server.env` containing:

```text
BETA_TOKEN=replace-with-a-random-token-of-at-least-32-characters
```

```sh
chmod 600 ~/.config/beta-cutting-room/server.env
mkdir -p ~/.config/systemd/user
cp deploy/beta-cutting-room.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now beta-cutting-room
systemctl --user status beta-cutting-room
journalctl --user -u beta-cutting-room
```

This is a **user** unit. A site administrator may need to enable user lingering
for operation after logout. Review resource limits for the machine. Stop with
`systemctl --user stop beta-cutting-room`. Do not install the example unchanged
on a shared server. Release verification only starts a temporary loopback process.

## Limits and operations

- Requests are limited to 50 MiB; photo uploads to 30 MiB. Streaming bodies are
  checked too. The examples also cap proxy uploads at 60 MB; the application's
  smaller route-specific limit still applies.
- One nesting or planning job runs at a time. Another gets HTTP 409.
- A search budget excludes import, construction and final validation. Setup can
  take longer than the requested search. Native desktop cancel terminates its
  worker; the HTTP server has no equivalent cancellation endpoint.
- A shared token is access control, not quotas, audit accounts or a complete
  hostile-Internet security boundary. A security review remains required.
- The Hyper HTTP/1 transport accepts at most eight connections, 64 headers and
  16 KiB of request headers. Headers have a ten-second deadline. Body collection
  has a thirty-second total deadline; chunked bodies have the same size limit.
  Oversized declared lengths are rejected before body collection. A stalled
  response write is stopped after thirty seconds without progress.
- A valid Content-Length above the cap gets HTTP 413, including an extremely
  large numeric value. Invalid or unparseable lengths get 400. Header size/count
  violations get 431. The bounded preflight uses at most 16 KiB, including any
  incidental bytes read with the headers; allocation never follows Content-Length.
- Connections close after one request. Rejected bodies are dropped, without the
  former `tiny_http` unread-body drain/allocation. At most eight request compute
  slots exist, including disconnected clients whose computation is still running.
  Excess connections close; occupied compute capacity returns HTTP 503. These
  transport bounds do not impose a deadline on a running nesting computation.
- Keep the web root limited to the supplied static assets. Do not point it at a
  home folder, project archive or dataset directory. Static files are public.
- Back up saved projects and browser exports. Server files alone do not contain
  all drafts. Export drafts before clearing browser data or changing origin.

To update, stop the service, back up data, replace the executable and full web
directory together, check hashes, then restart and test health, import, nesting,
validation and export. Keep the previous pair for rollback.

The transport change and remaining review gates are recorded in
[SECURITY.md](../SECURITY.md). A bounded transport is not a completed external
security review. Prefer HTTPS, a firewall and approved factory clients.
