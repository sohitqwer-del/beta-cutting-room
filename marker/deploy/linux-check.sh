#!/bin/sh
# Run from an isolated source copy. Does not install a service.
set -eu
. "$HOME/.cargo/env"
export CARGO_BUILD_JOBS=3
export CARGO_TARGET_DIR="$(pwd)/target"
export BETA_DATA_DIR="$(pwd)/test-data"
export TMPDIR="$(pwd)/tmp"
mkdir -p "$BETA_DATA_DIR" "$TMPDIR"
rustc -vV > toolchain.txt
ldd --version > glibc.txt
nice -n 10 cargo build --manifest-path marker/Cargo.toml --release -p beta-marker-cli > linux-build.log 2>&1
mkdir -p probe-web
printf '<!doctype html><title>BETA release probe</title>\n' > probe-web/index.html
nice -n 10 python3 marker/deploy/verify-server.py "$CARGO_TARGET_DIR/release/beta-marker" --web-dir probe-web > linux-smoke.log 2>&1
cat linux-smoke.log
