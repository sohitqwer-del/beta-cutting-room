# Security reporting and deployment

**Report security problems privately.** Use GitHub's private reporting
("Report a vulnerability" on the repository's Security tab) or email
sohitqwer@gmail.com with "BETA security" in the subject. Do not open a public
issue for a security problem. Do not publish factory data, tokens or a working
exploit. This is a small volunteer project: we will reply as soon as we can,
but there is no guaranteed response time.

Include the BETA version/source commit, OS, affected command/route, impact,
steps to reproduce and a minimal synthetic input. Remove real credentials,
customer names and private geometry. Report dependency findings with the exact
resolved version and affected deployment. Maintainers should acknowledge the
report privately, assess scope, agree on disclosure and verify the fix.

## Version scope

0.4.0 is a launch candidate. It has not completed an independent security audit.
Earlier milestones are development snapshots, without a maintenance guarantee.
See the [release checklist](docs/RELEASE-CHECKLIST.md) for required acceptance.

The desktop uses native Tauri IPC. The CLI HTTP server defaults to loopback.
Network binding requires a token, but a token is not encryption or a per-user
permission system. Use Caddy/nginx for HTTPS and restrict access to approved
factory clients. See [self-hosting](docs/SELF-HOSTING.md).

## HTTP transport finding and mitigation in 0.4.0

During release review, the former `tiny_http` 0.12 transport was found to drain
unread request bodies on cleanup. Its equal-length reader allocated from the
remaining declared Content-Length. An application-level rejection or upload
limit therefore did not fully bound cleanup allocation or slow-client time.
It also pre-buffered small bodies before application authentication. This was
a source-review finding; no attack against another service was performed.

0.4.0 removes `tiny_http` from the server and uses Hyper HTTP/1 with:

- Authentication before application body collection and route dispatch.
- A 50 MiB body cap, or 30 MiB for photos, including unknown-length/chunked data.
- A bounded header preflight returns 413 for a valid declared length above the
  route's upload cap, including `u64::MAX`. Invalid or unparseable lengths return
  400. Header count/size violations return 431. Conflicting length/framing
  headers are rejected. The preflight buffer is at most 16 KiB; it never sizes
  an allocation from the supplied length. Hyper then parses the same prefix.
- At most 64 headers and 16 KiB of request headers; ten seconds to finish headers.
- A thirty-second total upload deadline and thirty-second response write-idle
  timeout. A running computation is not limited to thirty seconds.
- Eight simultaneous connections and eight request compute slots. A compute
  slot remains held if its client disconnects. The HTTP blocking-worker pool
  is capped at three threads; calculations can also use their own engine
  workers. This is not a global CPU quota.
- One request per connection, then close. Rejected bodies are dropped without
  unbounded draining. Excess connections close; busy compute capacity gets 503.

Wire regressions cover enormous declared lengths with no body, oversized chunked
uploads, incomplete headers, stalled bodies, header size/count, duplicate auth,
connection capacity and stalled response writes. Existing route tests cover
imports, nesting/planning contracts, pattern operations, digitizing and exports.
An independent review must still check the final merged build and reverse proxy.

## Remaining boundaries

The server does not provide TLS, user accounts, roles, a shared document
database, quotas per user or an HTTP nesting-cancellation endpoint. A configured
web root is public and must contain only the intended app assets. Application
work may outlive a disconnected client; bounded capacity prevents unlimited
queued workers but does not give a hard deadline to geometry computations.
Use a trusted factory network and backups. Review image/geometry parsers and
dependency advisories before exposing the service to untrusted users.

Build tools and offline fixture generators have a separate dependency/security
surface from the shipped app. A permissive-licence gate does not certify them
free of vulnerabilities. The release record reports the executed checks and
known advisories; the public launch checklist requires a disposition.
