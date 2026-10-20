# Public launch checklist: 0.4.0

This is a gate list, not a statement that every item passed. Record the exact
commit, platform, command, result and reviewer. Rebuild packages after merging
engine, UI or benchmark changes. Do not publish before approval.

See the [0.4.0 release record](RELEASE-0.4.0.md) for executed checks, artifact
sizes and hashes. Use that evidence to separate executed checks from remaining
acceptance gates.

## Automated checks and package records

- [ ] Verify all BETA workspace/npm/bundle/About versions are 0.4.0. Compatibility
  adapters retain upstream-compatible versions; document that exception.
- [ ] Regenerate Cargo/npm lockfiles in the lead's checkout and review the diff.
- [ ] Run `cargo test --workspace --no-fail-fast` in `marker/` and `pattern/`.
- [ ] Run `cargo clippy --workspace --all-targets -- -D warnings` in both workspaces. Investigate
  warnings, especially in changed files.
- [ ] Run `npx vitest run marker` and Marker TypeScript checks.
- [ ] Rebuild Marker, Pattern and Floor WASM; check all three in `web/wasm/`.
- [ ] Run the desktop licence gate and regenerate the complete dependency
  inventory. Retain upstream notices and jagua-rs source access. Audit optional
  platform dependencies separately.
- [ ] Build NSIS, portable ZIP and Linux CLI. Record path, bytes, SHA-256,
  source commit, flags, compiler and Linux glibc version.
- [ ] Check Linux help and a temporary loopback server, then stop it. Do not
  install a service on the verification server.
- [ ] Review npm security advisories and provenance. Record accepted risks and
  fixes; a licence gate is not a security audit.
- [ ] Confirm packages exclude datasets, private data, tokens, development
  servers and debug endpoints.

## Clean-machine and hardware acceptance

- [ ] Install, launch, save, reopen, upgrade and uninstall on a **clean Windows
  PC** with a standard account. Test file associations and path handling.
- [ ] Test **Windows 10** x64 and Windows 11. Include a PC without WebView2 and
  check the standalone offline prerequisite.
- [ ] Work with network access disabled.
- [ ] Review OS/browser/WebView2 diagnostic settings and capture outbound traffic
  on the target PC. Separate BETA page/API traffic from platform background services.
- [ ] Test recovery after forced close, including a large project, pending
  recovery and explicit replacement of a recovered session.
- [ ] Print a scale square and a tiled 1:1 pattern on a **real printer**.
- [ ] Plot a known-size PLT on the factory's **real plotter**. Check feed, axes,
  rotation, units, labels and clipped edges.
- [ ] Prove a safe sample on the **real cutter** with an approved profile. Check
  blade offsets, origin, units, sequence and emergency stop. Generic ISO output
  is not machine acceptance.
- [ ] Complete pattern -> marker -> plan -> rolls -> spread -> bundles -> re-cut
  -> sewing issue with a trained operator.
- [ ] Review Hindi and Tamil with native speakers: garment terms, errors, small
  screens and printed documents. Check English fallback.
- [ ] Check keyboard use, focus, screen-reader labels and contrast in the final
  UI, including token prompts and errors.

## Security, licences and publication

- [ ] Review `serve`: authentication on every API, loopback defaults, encoded
  paths, symlinks, Host/Origin handling, body limits, slow uploads, resource bounds, token rotation
  and proxy settings. Test missing/wrong/duplicate headers.
- [ ] Independently review the replacement of `tiny_http` by bounded Hyper:
  early declared-length rejection, streaming limits, 10 s header / 30 s body
  deadlines, 30 s write-idle timeout, 64 headers / 16 KiB, eight connections and
  eight compute slots held across disconnects. Verify rejection closes a client
  without draining its body. Regression tests are evidence, not a full audit.
- [ ] Verify status separation: valid oversized Content-Length and chunked
  bodies return 413; invalid/unparseable lengths return 400; header size/count
  violations return 431. Include `u64::MAX` and conflicting framing headers.
- [ ] Replace the private-reporting placeholder in [SECURITY.md](../SECURITY.md)
  with a monitored contact before public launch.
- [ ] Check HTTPS, firewall rules, backup/restore and log retention on the target
  network. Confirm no unauthenticated token-injecting proxy.
- [ ] Review MPL-2.0 jagua-rs notices and exact unmodified source access. Verify
  the two Apache-2.0 adapters replace the intended MPL helper graph.
- [ ] Sign and timestamp the Windows installer/executable. Verify signatures
  and recalculate artifact hashes.
- [ ] Replace README placeholders with reviewed public-demo screenshots.
- [ ] Use only the final public benchmark report. Preserve sample, time,
  hardware and constraint caveats. Remove private evidence from packages.
- [ ] Resolve or explicitly accept open QA findings against the final build.
- [ ] Get the lead/user's publication approval. No public upload, deployment,
  package publication or release tag is authorized by this checklist.
