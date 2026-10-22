# Repeatable release builds

Run these commands from the repository root. Use the reviewed source commit and
resolved Cargo/npm lockfiles. Keep build output outside the source tree. Record
Rust, linker, Node, npm, wasm-bindgen and operating-system versions with hashes.
The release wrapper adds no dependency and installs no tools.

## Remove local paths

Set `CARGO_TARGET_DIR`, `CARGO_BUILD_JOBS=3`, npm's cache and `WASM_BINDGEN` for
your build machine. Set `CARGO_HOME` if it is not the default user `.cargo`.
Do not commit their machine-specific values.

```sh
node marker/deploy/release-build.mjs marker-wasm
node marker/deploy/release-build.mjs pattern-wasm
node marker/deploy/release-build.mjs floor-wasm
node marker/deploy/release-build.mjs cutroom-wasm
npm run build -w @beta/marker-web
node marker/deploy/release-build.mjs cli
```

For the existing Windows desktop toolchain, first build Floor WASM as above and
set `BETA_TOOLS_DIR` as described in [INSTALL.md](INSTALL.md). Then run:

```sh
node marker/deploy/release-build.mjs desktop
```

The wrapper sets `CARGO_ENCODED_RUSTFLAGS` so paths containing spaces remain one
compiler argument. It preserves the flags Cargo would already use, then adds:

```text
--remap-path-prefix=<absolute Cargo home>=/cargo
--remap-path-prefix=<absolute repository root>=/build
```

On Windows it maps both native and forward-slash spellings. The repository root
includes Marker and the sibling Pattern crates. The setting reaches dependencies,
WASM and desktop child builds. `CARGO_INCREMENTAL=0` avoids incremental state.
The CLI uses `--locked`. The wrapper sets `SOURCE_DATE_EPOCH` from the source
commit timestamp; set it explicitly when building a source archive without Git.
The GNU Windows CLI also disables the PE linker timestamp. Keep CPU defaults;
`target-cpu=native` is rejected for distributable builds.

The CLI build scans its output for the actual repository and Cargo-home paths,
and Windows user-directory prefixes, in UTF-8 and UTF-16. A match fails the build.
Repeat the scan on the **actual packaged executable**, not just a staging copy:

```sh
node marker/deploy/release-build.mjs --verify <packaged-beta-marker-executable>
node --test marker/deploy/release-build.test.mjs
```

Remapping controls compiler-generated paths; it does not scrub an arbitrary
string produced by a build script, linker, external tool or embedded input.
Inspect new absolute-path matches before shipping. Debug symbols, PDB files,
installer metadata and archive metadata need separate inspection if distributed.
See the [rustc remapping documentation](https://doc.rust-lang.org/rustc/command-line-arguments.html#--remap-path-prefix-remap-source-names-in-output).

## Repeatability gate

1. Record the commit, dirty/clean state, lockfile hashes and exact build tools.
   Keep `--locked` for release compilation. Resolve and audit lockfile changes
   before the release; never treat local regeneration as an approved lockfile.
2. Build from two clean source directories using the same prepared toolchain,
   caches, target, release flags and `SOURCE_DATE_EPOCH`. Compare executable and
   web-asset SHA-256 values. Diagnose differences before calling them bit-for-bit
   reproducible. A same-directory cached Cargo build alone does not prove this.
3. Keep timestamps, ownership, archive entry order and installer signing under
   explicit control. The existing portable ZIP/tar scripts are packaging tools,
   not evidence of deterministic archives. Signing changes the final checksum.
4. Test the exact archived executable and web assets offline. Record matching
   versions for all three WASM modules and reject missing modules in staging.

## Planned older Linux baseline

The published 0.4.0 Linux build used glibc 2.43. Path remapping does not change
that ABI requirement. An older builder is a separate release gate.

Use a disposable **Ubuntu 22.04 x86_64 builder (glibc 2.35)** with a pinned image
digest and the recorded Rust GNU toolchain. Prepare and pin compiler/linker,
Node (build wrapper only), certificates and dependency caches in that builder.
Record package versions and base-image licence notices. Do not install anything
system-wide on the operator's PC. Docker/Podman is build infrastructure, not a
runtime dependency or an installer prerequisite.

Proposed procedure, not executed by this Windows audit:

1. Create a reviewed builder image on the build host. Pin its digest, toolchain
   versions and package snapshot. Use Ubuntu's public packages and the existing
   reviewed dependencies. Build only `beta-marker-cli`; no GTK/Tauri desktop
   libraries are required. Copy the verified matching static web assets.
2. Mount a clean public source export at `/build`, Cargo cache at `/cargo`, and
   build output at `/out`. Use a non-root user, three jobs and no host services.
   Set `CARGO_HOME=/cargo`, `CARGO_TARGET_DIR=/out`, `CARGO_NET_OFFLINE=true` and
   the source commit's `SOURCE_DATE_EPOCH`. Run the wrapper's `cli` mode with the
   container network disabled, after all dependencies are cached.
3. Check `readelf --version-info /out/release/beta-marker` and record the highest
   required `GLIBC_*` and other shared-library versions. Run `ldd` only on the
   just-built trusted artifact. Reject a requirement newer than the baseline.
4. In a fresh 22.04 runtime container, run `--help`, DXF import, a short demo nest,
   independent validation, exports and `deploy/verify-server.py`. Probe the built
   browser assets headlessly from a separate test process. Use loopback and stop
   the owned server; install no persistent service.
5. Repeat a smoke on a newer distribution. Package and checksum only after both
   pass. Publish the measured baseline, architecture and remaining platform
   limits. Do not claim support for older glibc merely because compilation passed.

`cargo zigbuild` is a possible future build-only alternative. It is not added or
installed here. Review its exact tool/dependency licences, target ABI and native
dependencies before adopting it; a target suffix alone is not runtime acceptance.

Ubuntu's [22.04 release](https://releases.ubuntu.com/22.04/) and
[libc6 package](https://packages.ubuntu.com/jammy/libc6) identify the proposed
baseline. Newer security updates may change patch versions; record the exact
builder used rather than relying on an unpinned `ubuntu:22.04` tag.

## 0.5.0 older-glibc result

The 0.5.0 release used an isolated user-folder Python environment on the Linux
build host: cargo-zigbuild 0.23.4 and Zig 0.17.0 (both MIT), with Rust 1.98.1.
No container service or system package was installed. The target was
`x86_64-unknown-linux-gnu.2.31`; `readelf --version-info` found GLIBC_2.30 as
the highest required symbol version. A linker message said Zig ignored a
deprecated linker optimization setting; it did not prevent the build.

For runtime acceptance, Ubuntu's public focal-updates `libc6` package
2.31-0ubuntu9.18 was downloaded with its package-index SHA-256 and extracted
into the owned build folder. Its `ld-2.31.so --library-path <extracted-libs>`
ran the exact release ELF. All 81 CLI audit calls, authenticated loopback
server checks and QA-006 passed there. The current host's glibc 2.43 also
passed the help and server checks. This is loader-level compatibility evidence,
not a full clean Ubuntu 20.04 installation test. Other OS libraries, kernels
and target PCs still need acceptance. The extracted libc is not distributed.

See [the release record](RELEASE-0.5.0.md) for checksums and exact results.
