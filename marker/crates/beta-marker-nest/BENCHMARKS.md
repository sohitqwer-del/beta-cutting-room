> Historical round-1 report. Round 2 supersedes the stub-validator and
> `max_length` behavior described below; see [ROUND2.md](ROUND2.md).

# Nest adapter verification

Verified on 18 October 2026 with Rust 1.98.1, Windows GNU target, and an Intel
Core i9-11900H (8 cores / 16 logical processors). The host also runs other agent
work. These are single-seed sanity runs, not a statistical quality study.

## Checks

All Cargo commands ran from `marker`, with `CARGO_TARGET_DIR` set to the assigned
external build directory. No build output is committed.

- `cargo test --workspace --jobs 4`: passed; 18 passed, one long test ignored.
- `cargo clippy --workspace --all-targets --jobs 4`: passed; no warnings in the
  nesting crate, its examples/tests or the nesting CLI command. There is one
  existing core warning at `crates/beta-marker-core/src/rules.rs:107`
  (`neg_cmp_op_on_partial_ord`), outside this role's ownership.
- `cargo run --release -p beta-marker-cli -- nest examples/jobs/gaps-buffers.json --time 1 --seed 7 --workers 2 --early-termination -o crates/beta-marker-nest/out/cli-result.json`: passed; 1,149.83 mm, 74.18%, 1.47 seconds.
- Manual CLI checks: invalid fabric width exits 1 with `fabric_width` and leaves
  no output file. A 1 mm max_length returns exit 0 with a warning and a result.
  No engine SVG/log folder is created in the calling directory.
- The short integration test uses one-second search budgets and seed 1. All
  four synthetic jobs pass, including unequal buffers and a panel that requires
  -2 degrees of tilt to fit. The integration test completed in 4.74 seconds.
- Unit checks cover mirror/grain composition, reversal of jagua centring,
  deterministic initial construction, allowed tilt samples, detailed outline
  conservativeness, copy indices/counts, gaps, overlap, edges, invalid inputs,
  progress, max_length warnings and reproducible example files.

## Academic runs

Sparrow source: v0.3.0, git revision `c5e64b3b`. Jagua-rs: 1.0.0, unmodified.
The raw CLI was built from an unchanged source snapshot in an ignored output
folder, because the read-only Cargo checkout does not contain a Cargo.lock.
The snapshot manifest only gained an empty `[workspace]` table so it can build
inside this repository; no engine source changed. Raw CLI release settings are
its upstream defaults. The adapter uses this workspace's release settings.

Each run uses seed 1, three workers, 48 seconds exploration and 12 seconds
compression. Import, initial construction and final checking add wall time.
Adapter and raw runs execute sequentially. The Rust `sanity` example converts
the academic polygons into Jobs with the fixed strip height as fabric width,
zero gaps and 0/180-degree rotations. It checks the adapter result and writes
both results and the improvement history to the supplied ignored output folder.
Raw efficiency is recomputed from the same original piece area and the raw
CLI's final strip width; adapter efficiency uses original-outline marker length.

Shirts: adapter **87.1564%**, raw CLI **87.1158%**; adapter difference
**+0.0406 percentage points**. Adapter cut-outline length: 61.957569; raw
strip width: 61.986477 (input units). Adapter total runtime: 67.705 seconds;
119 feasible length improvements were recorded. Paper: best **90.92%**, mean
**89.66%**. This meets the -0.5 point adapter-regression limit.

Trousers: adapter **90.4154%**, raw CLI **90.6367%**; adapter difference
**-0.2214 percentage points**. Adapter cut-outline length: 240.892487; raw
strip width: 240.304120 (input units). Adapter total runtime: 65.627 seconds;
116 feasible length improvements were recorded. Paper: best **92.62%**, mean
**91.73%**. This also meets the -0.5 point adapter-regression limit.

Paper values are those supplied with the task. The paper uses 1,200 seconds
and three threads, so these 60-second runs must not be read as equivalent
experiments. No attempt was made to tune the adapter for either dataset.

Reproduce from `marker` with the required external CARGO_TARGET_DIR:

```text
cargo run --release -p beta-marker-nest --example sanity -- <sparrow-source>/data/input/shirts.json <raw-sparrow-executable> crates/beta-marker-nest/out/shirts 60
cargo run --release -p beta-marker-nest --example sanity -- <sparrow-source>/data/input/trousers.json <raw-sparrow-executable> crates/beta-marker-nest/out/trousers 60
```

The measured runs invoked those compiled release executables directly, using
the supplied read-only input files. Evidence remains in the ignored `out/shirts`
and `out/trousers` folders, including `summary.json`, `adapter.json`,
`progress.json`, `raw.log` and the raw CLI output.

## Dependencies and integration

New direct dependencies: rand 0.10.3 and geo 0.33.1, both MIT OR Apache-2.0.
Both packages and all their dependencies were already in the workspace's resolved
sparrow/jagua dependency graph. No new transitive package is needed. Sparrow
is MIT; jagua remains its unmodified MPL-2.0 dependency.

No Job or MarkerResult contract changes are needed. The lead must regenerate
`marker/Cargo.lock` after merging so the nesting package lists its new direct
geo/rand edges. The generated two-line lock change is outside this agent's owned
files and is not part of the commit. The adapter and CLI guard quantity overflow
before calling the core demand helper; the lead should also make core's public
`check_job`/`demands` path reject overflow and non-finite scalar inputs directly.

Known limits: the seed fixes random streams and initial construction, but
wall-clock stopping prevents guaranteed byte-identical final layouts. The time
limit is a search budget, not a hard end-to-end deadline. Tilt is sampled at up
to five directions per base, rather than searched continuously; more directions
cost search effort. Very large coordinates can exceed jagua's f32 precision; the
adapter returns an error if its original-outline safety check finds a violation.
The core validator is a stub on this branch; its `not_implemented` issue is
ignored, while the adapter's independent safety check remains mandatory.
