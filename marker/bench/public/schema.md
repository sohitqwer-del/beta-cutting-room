# Public export contract

Use `beta-site-benchmark/1`, defined by the website owner in
`marker/apps/site/data/benchmark/SCHEMA.md` (first committed as `18530c0` on
`marker/opus-site`). One file represents one dataset and time budget. Numeric
differences use percentage points; saving fields use percent, not fractions.
No input geometry, marker positions or raw dataset records are exported.

Additional fields emitted by the reproduction tool:

- `task_ids`: sorted unique public task IDs, without scores or geometry.
- `x_runtime`: aggregate whole-call and validation times, search budget range,
  trial start/end Unix milliseconds, and runner wall time if available.
- `x_validation`: saved validator reports checked, failed trials and missing
  evidence counts. A valid CSV row without its successful independent report
  is an export error, never an accepted score.
- `x_provenance`: aggregate source revision/settings, input hashes and run-file
  hashes. No absolute paths, host names or IP addresses.
- `too_long_losses`: per group, failed finite-strip trials which count as
  additional losses in the Markdown report. Website `win+tie+loss=valid`
  remains true. `other_failures` excludes `too_long`.

Missing measurements are JSON null, never NaN or invented zero. A fully
attempted suite can be final even if a trial failed; failures stay visible.
Incomplete stages use `provisional`, `progress` and `status_note`.
