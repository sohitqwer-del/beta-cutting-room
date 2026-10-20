# Experimental GPU nesting proposals

This crate uses headless wgpu compute on Vulkan or DX12. It does not change
Sparrow or Jagua, create a window, require CUDA, or use a runtime network service.
It has its own workspace until the lead registers it. See `RESULTS.md` for the
paired evidence and the recommendation. This is an opt-in experiment.

The partial paired results do not justify production GPU warm starts: mean
efficiency was lower at each measured allowance, and gap filling improved none
of 11 tested baselines. Keep the production default on CPU. Host coordination
costs limit this implementation. FreeSewing paired cells and much of the planned
Lectra matrix are unmeasured; see `RESULTS.md` for exact coverage and failures.

## What runs on the GPU

The CPU builds conservative piece masks at 2 mm resolution. Each x column holds
a vertical span of the polygon, expanded by half the piece gap, the piece's
buffer and a 0.02 mm guard. Concave vertical pockets can be filled by this hull.
This sacrifices positions; it does not license invalid geometry.

The strip and masks use packed u32 bitmaps. A compute invocation checks one
translated mask. An atomic minimum chooses the lowest feasible x, then y.
Rotations are tested separately; the host chooses the lowest right edge.
An early result prunes positions with a larger index. This gives the same answer
as the scalar CPU scan. Piece copies share masks. Construction places large
pieces first and checks the current strip, including earlier pockets.

The CPU checks the chosen position with full f64 cut polygons, including gaps
and unequal buffers. A full layout must then pass `beta_marker_core::validate`.
Only that result can be supplied as `NestOptions::warm_start`. The existing
adapter keeps it as an incumbent and may move its pieces during optimization.
Warm starts do not lock placements.

`fill_gaps` removes up to 32 small pieces in the last 35% of a valid layout,
rasterizes the remaining pieces, and reinserts around them. It never moves a
locked piece. A failed or longer reconstruction returns the valid incumbent.
It fills spaces between pieces, not holes inside a polygon.

CPU mode never initializes a graphics API. Automatic mode uses CPU after adapter
or device failure and records the reason. Required-GPU mode fails explicitly,
so a benchmark cannot silently measure CPU and label it GPU.
The full nesting wrapper uses native CPU nesting when automatic initialization
found no GPU. It avoids spending a short nesting budget on a slow scalar raster
warm start. Explicit CPU proposal mode remains available for measurement.

## Supported rules and bounds

Open, single-layer fabric; prescribed mirror counts; allowed discrete rotations;
per-piece buffers; size ratios and garment bundles; fusing blocks; shrinkage.
Free rotation is sampled at quarter turns. Tilt is not sampled. Construction does
not search alternative mirror allocations, piece orders or coupled bundle
directions. Folded/tubular, one-way-per-bundle, defects, shade, repeat and matching
jobs return an explicit proposal error. The wrapper uses normal nesting instead.
The wrapper also leaves existing fixed/warm-start options on the normal path.

Default strip storage is bounded to 16 MB (4 million u32 words). Piece raster
dimensions are at most one million cells, and at most 10,000 copies may be proposed.
The shared piece mask cache is bounded to 128 MB; garment bundles to 10,000.
Selected positions have at most 64 exact-check retries. Adapter setup is reusable, but
this prototype allocates and uploads buffers per orientation. It is synchronous;
call it on a worker, not an editor event thread. The search allowance includes
proposal time; importing, device initialization and final checking add overhead.

## Commands

Run from `marker/`. Set `CARGO_TARGET_DIR`, `CARGO_BUILD_JOBS=3` and
`BETA_DATA_DIR` to the assigned external locations. No lockfile or geometry is
committed.

```text
cargo test --release --manifest-path crates/beta-marker-gpu/Cargo.toml
cargo test --release --manifest-path crates/beta-marker-gpu/Cargo.toml gpu_matches_cpu -- --ignored
cargo clippy --manifest-path crates/beta-marker-gpu/Cargo.toml --all-targets -- -D warnings
cargo build --release --manifest-path crates/beta-marker-gpu/Cargo.toml --bins
cargo run --release --manifest-path crates/beta-marker-gpu/Cargo.toml --bin gpu-bench -- probe <job.json> <fresh-output-directory> [cell-mm]
cargo run --release --manifest-path crates/beta-marker-gpu/Cargo.toml --bin gpu-bench -- paired <development-40.jsonl> <fresh-output-directory> 20 10,30,60 2
cargo run --release --manifest-path crates/beta-marker-gpu/Cargo.toml --bin gpu-summary -- <evidence-directory>
```

`gpu-bench` also includes FreeSewing Simon, Teagan and Titan ratio jobs. It uses
seed 1 and two separator workers. Plain and warm runs are sequential and alternate
order by job and budget. Proposal wall time is subtracted from the warm variant's
search budget. CPU fallback construction is timed separately once per job. Gap
filling is measured after each final-budget plain result. A 0.1 percentage-point
band defines a tie. Invalid results are recorded as failures, never discarded.

Optional arguments after the cell size are a 60 s job limit and a prior evidence
directory. For example, `20 10,30,60 2 10 <prior-evidence>` runs 10/30 s for all
23 jobs and 60 s for the first ten Lectra jobs. The runner reuses only complete
prior pairs with no run overlapping a telemetry gap above five seconds inside
an actual sampler session. Program restart gaps are excluded. It
records discarded pairs in `resume-audit.json`, verifies reused geometry/rules,
and preserves old telemetry and original gap-fill budgets. Both variants of an
affected pair are rerun.
Gap filling then runs after the highest measured budget for each job.

Evidence includes full results and validation outside the repository, proposal
statistics, the exact input task manifest, revision, budget and order, and
`nvidia-smi` samples. GPU kernel time uses timestamp queries when available.
Host scan time includes allocation, transfers and synchronization. Utilization
samples cover the shared GPU and can miss short dispatches.

Lectra development geometry is CC BY 4.0 and stays in `BETA_DATA_DIR`. These tests
compare BETA against itself on converted jobs; they make no commercial comparison
or claim that undocumented source constraint codes have been verified.
FreeSewing fixtures are MIT; attribution is in `testdata/open/freesewing/`.
Dependency licences and exact resolved versions are listed in `LICENSES.md`.

## Integration proposal for the lead

For optional research integration, register this directory in the marker
workspace, remove its local `[workspace]`
and duplicate profile tables, and regenerate the workspace lockfile. No core
model change is needed. Keep GPU support behind an optional CLI Cargo feature
and an explicit experimental `beta-marker nest --gpu` flag. The CLI can reuse
one `Proposer` and call `beta_marker_gpu::nest(job, options, proposer)`.

Do not put a `gpu` flag in `NestOptions` while this crate depends on
`beta-marker-nest`; that would encourage a dependency cycle. If production data
supports promotion, define a generic proposal hook at the orchestration layer.
Expose automatic/CPU/required device policy and the measured fallback reason.
For the editor, call `fill_gaps` on a worker and present only a valid improvement.

Before any default enablement, repeat paired tests on an idle host, several
seeds, held-out styles, DX12-only machines and a machine without a GPU. Re-test
the merged core bundle order, shrinkage/block metrics, fixed layouts, and the
unsupported coupled-rule fallbacks. Do not infer throughput or density benefits
from GPU utilization alone.
