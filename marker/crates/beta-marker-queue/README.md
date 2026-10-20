# Persistent production queue

`Queue::open(folder)` stores immutable typed requests in `jobs/ID.json` and
results in `results/ID/`. Job JSON contains status, priority, attempts, timestamps,
summary, last error, search seconds, wall seconds, and occupied worker seconds.
The last metric is engine-call wall time multiplied by declared engine workers;
it is not measured hardware CPU utilisation.

`add`, `list`, `status`, `pause`, `cancel`, `resume`, `checkpoint`, and `run`
are public Rust APIs. `run` accepts `RunOptions` and a synchronous `Progress`
callback. All inputs/statuses support serde JSON. Requests embed the patterns,
order, stock and options, so reopening a queue does not depend on source paths.
No request can invoke a shell or a supplied executable.

Jobs run serially, highest priority first, then creation time and id. A queue-wide
OS file lock rejects a second runner. Short state locks serialize operator and
worker updates. A process exit releases the OS lock; on the next run, abandoned
`running` jobs become queued and retain checked results. A leftover lock file
alone does not block recovery. JSON writes use a flushed sibling temp file and
atomic rename. Unknown versions and corrupt job files are reported and skipped.
This protects process-restart recovery; it is not a power-loss guarantee on every
removable filesystem. Use one local queue folder, not a replicated network share.

## Time, workers and checkpoints

Tiers: draft 20 s, costing 60 s, production 300 s, overnight custom (up to 24
hours). A single Nest task uses that total search budget. Planning/pipeline tiers
apply per marker, matching the existing planner's `time_per_marker_s` convention;
the short budget stays at or below the supplied planner short budget and the
final budget uses the chosen tier. Width trials use the request's final budget
capped by the tier. Search time excludes construction, validation and exports.
The JSON records total search and wall time separately.

**Checkpoint slicing is disabled with the current engine.** Every engine phase
runs continuously for its requested budget. A cold marker is constructed once;
refinement/resume uses an independently checked warm start. The experiment found
that warm-started calls still rebuild engine state and can spend far longer in
setup than a short search slice. Repeatedly calling the engine to checkpoint is
therefore unsafe for both latency and quality. `slice_seconds` remains a reserved
interval request for API compatibility and does not split a call, even when set
below 60 s. Short draft/test budgets remain one continuous short run.

A **60 s soft construction guard** records `construction_guard_triggered` and
sets a clear continuous-fallback status when no first feasible-layout callback
arrives. It never restarts construction. `first_layout_s` records the first
reported cold feasible layout (or successful return), including any preceding
search; this is a conservative construction bound rather than a pure profiler.
The runner independently validates and flushes the best geometry before sending
`checkpoint_saved=true`. Length-only engine callbacks never claim a disk save.

Known limit: the in-process API cannot safely abort or retime construction. The
guard is observable and keeps the same continuous call; it is not a hard timeout.
Pause/cancel waits for that call, including construction/search/checking overhead.
Single long nests cannot save intermediate full geometry until the quality agent
adds a full-geometry progress callback or persistent engine session. Planning and
batch jobs still save checked markers between phases/trials. No future slice may
be below 60 s or below measured construction time, and it must start from a valid
layout. A future callback can restore responsive continuous checkpoints without
changing the queue/job/report contracts.

Nest tasks write `checkpoint.json` and `result.json`. Plan/pipeline tasks also
write `markers/HASH.json`, containing the exact job plus best validated geometry
for each ratio. The stable hash is only a filename: cache hits compare the full
job, preventing incompatible reuse on a hash collision. Ratio markers survive a
pause before the first complete plan. Completed partial-plan callbacks save a
whole-plan checkpoint; complete plans beat partial plans, then physical objective
breaks ties. A failure cannot erase an earlier checked checkpoint.

`max_workers` limits engine workers in this queue. Execution is serial and the
current planner uses two workers, so Plan/Pipeline/WidthTrials require at least
two. Nest can run with one. Values above two are currently an upper bound; the
queue deliberately uses at most two on the shared machine. The coordinator is
separate from engine workers. Different queue folders have separate limits.

## Typed tasks

- `Task::Nest`: a job and NestOptions. Fixed placements/rules and checked warm
  starts are retained; one continuous call uses the remaining search budget.
- `Task::Batch`: independent jobs and trial widths. Each trial has a checked
  result or an isolated error, saved job/result JSON and a summary CSV. Resume
  skips completed successes and retries failures; a partial batch remains explicit.
- `Task::Plan`: cut order and planner options. Cancellation retains ratio markers
  and the best checked partial/complete cut plan.
- `Task::Pipeline`: compensated order, approved rolls/ids, colour and optional
  `ReportSettings`. Plans, audits stock, maps and checks allocated defects, expands
  physical parts, bundles them, and writes JSON, QR tickets/stickers, planned XLSX, PLT/ISO/mini-marker
  files. With report settings it also writes the merchandiser XLSX/PDF/CSV pack.
  It rejects insufficient shrinkage compensation. Pending defect repairs or
  shortages fail with reviewable JSON, before production print/cutter output.
- `Task::WidthTrials`: `beta_marker_plan::cost::CostRequest`, with several fabric
  offers and optional explicit ratio menus. Final stock/cost selection is saved.

The queue uses one warm search sequence per marker, with `runs` normalized to
one to enforce the total-worker limit. Add separate jobs for independent portfolio
seeds. Batch tiers apply per job/width trial, as plan tiers apply per marker.

`Failed`, `Paused`, and `Cancelled` jobs can resume; immutable inputs and earlier
results remain. For a changed order or new budget, add a new id. Cancelling does
not delete results. A draining run continues after independent job failures and
writes `summary.json`. It returns when no queued jobs remain (not a daemon).

## CLI registration and use

The command module lives in `beta-marker-cli/src/cmd/queue.rs`; the lead owns its
registration. Until workspace integration, use the crate's standalone workspace
from the marker directory:

```text
cargo run --manifest-path crates/beta-marker-queue/Cargo.toml --example cli -- --folder queue add submission.json
cargo run --manifest-path crates/beta-marker-queue/Cargo.toml --example cli -- --folder queue list
cargo run --manifest-path crates/beta-marker-queue/Cargo.toml --example cli -- --folder queue run --workers 2
```

After registration: `beta-marker queue add|list|run|cancel|pause|resume|status`.
Without `--folder`, CLI storage is in the operating-system user-data directory,
outside the repository; embedded factory patterns are not put in a checkout by default.
`add` takes a Submission JSON. `--priority`, `--tier`, and `--seconds` optionally
override scheduling. `run` writes progress to stderr and a JSON summary to stdout.
Individual failures produce a nonzero CLI exit after the queue drains.

An example nest request (replace the job object with a complete core Job):

```json
{
  "id": "PO-1042-draft",
  "name": "PO 1042 draft marker",
  "priority": 10,
  "time": {"tier": "draft"},
  "task": {
    "kind": "nest",
    "job": {"format": "beta-marker.job/v1", "name": "style", "fabric": {"width": 1500}, "pieces": []},
    "options": {"time_limit_s": 20, "seed": 42, "workers": 2, "early_termination": false}
  }
}
```

The empty-piece object above illustrates nesting/schema only; use real patterns
before running. Overnight time is `{"tier":"overnight","seconds":3600}`.

## Integration and licences

Remove this crate's temporary `[workspace]` table and add it to marker workspace
members/dependencies. No core contract edit is required. The integration report
in `beta-marker-plan/OPS_REPORT.md` gives exact CLI lines and check results.

No new external package is introduced by this queue. Direct external dependencies
are existing anyhow/serde/serde_json (MIT OR Apache-2.0); the CLI harness uses
existing clap (MIT OR Apache-2.0). Internal marker crates are Apache-2.0. Existing
Sparrow is MIT, with the documented unmodified Jagua MPL-2.0 dependency exception.

Quality comparison driver: `cargo run -p beta-marker-plan --example slice_bench
-- inputs.json`. It compares continuous 60 s, 12 x 5 s warm starts and 4 x 15 s
warm starts, two workers, seed 42; rotates strategy order; independently validates
every result; saves per-slice curves/results under `BETA_DATA_DIR/marker-bench/ops`.
The manifest uses the pinned public Simon job and five development-split Lectra
jobs. No geometry or generated benchmark data belongs in git.
