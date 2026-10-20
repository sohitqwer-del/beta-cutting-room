# Integration proposals

No planner, costing or shared contract change is made on this branch.

## Planner: improve initial length estimates, keep measured final plans

The two initial candidate constructors in
`beta-marker-plan/src/lib.rs` currently use
`area / (width * opts.estimated_efficiency)` (the default efficiency is 0.75).
For each candidate, construct its exact `Job.order.sizes`, then call a cached
`Model::estimate_job(&candidate_job)`. On success, put `estimate.length_mm` into
`Candidate.length`; keep `Candidate.result = None`. On unsupported jobs or an
out-of-range warning, retain the configured fallback or do a short real nest.
Do not silently replace a calibrated factory model.

Keep single-size feasibility routes and the existing ratio/quantity checks.
The first integration should change the estimated table used to choose which
ratios to refine. Do not remove all nearby alternatives merely because their
predictions differ slightly; the residual interval is wider than many savings.
Keep `quality::checked_nest` and independent validation before any candidate can
appear in the final `CutPlan`. Keep exact compatible library hits ahead of a
prediction, and retain all relevant stock, ply, end-loss and lay-cost terms.

Assess this behind an opt-in estimator interface, such as:

```rust
// Proposed planner interface, owned by the planner agent; not added here.
trait CandidateEstimator {
    fn estimate_length_mm(&self, job: &beta_marker_core::Job) -> Option<f64>;
}
```

Model inference is fast, but a better global efficiency R² does not establish a
better cut plan. Before replacing short nests, freeze held-out factory orders and
compare total fabric, over-cut, feasibility, selected ratios, actual runtime and
number of refined candidates. Give both arms the same total search budget and
paired seeds. Include ratios that differ only slightly and constrained fallbacks.
Do not rank by efficiency alone: minimize full order consumption/cost.

## Costing: explicit estimated consumption and uncertainty

The existing `beta-marker-cutroom::costing::estimate` takes a linear
`EfficiencyModel`. Add a separate opt-in model provider or overload in that
crate; do not reinterpret its six coefficients as these trees. Clone the job,
set the requested width and exact size ratio, and call `Model::estimate_job`.
Use `length_mm / 1000 / garment_count` for net metres per garment, with an
explicit positive garment count checked against the order. Convert both ends of
`length_interval_mm` in the same way. Include end/splice/roll/stock losses as
separate terms; do not claim that the interval covers those losses.

Label the output “Lectra-reference estimate”, display model version/provenance,
keep the range warning and interval, and distinguish it from an approved marker.
Use real short nesting or the existing locally calibrated estimator for unsupported
jobs. Calibrate against the factory's machine/budget/material mix before quoting.
The source's approximately five-minute result is not a calibrated BETA 20-second
result. The lower-efficiency end of the interval gives a conservative length
scenario, not a purchasing guarantee.

## Search: diagnostics only

The model predicts a recorded outcome, not an analytic upper bound. A valid
BETA layout can beat it. Conversely, an overprediction can waste time on an
unreachable target. Do not set `early_termination`, change compression share or
cancel a run simply because it is within one point of the prediction.

A progress display may show `prediction - current_valid_efficiency` as a labelled
reference gap. Keep the best valid incumbent and the user's deadline. Learn a
stopping/allocation rule only from trajectories or paired randomized experiments
that record task features, hardware/load, seed, strategy, total thread budget,
elapsed time, gains and failures. A single Lectra result per task has no
counterfactual strategy labels. The historical small paired study is a diagnostic,
not 100,000 training examples for strategy selection.

No learned construction order is proposed for release. No cluster implementation
was available on `marker/cluster` when inspected (tip `ccf29ed` was the paired
benchmark analyzer), so this branch does not claim a cluster comparison.

## Lead-owned registration

1. Add `crates/beta-marker-learn` to `marker/Cargo.toml` workspace members and a
   workspace dependency if desired. Remove this crate's temporary `[workspace]`.
2. Regenerate Cargo.lock. Keep the model attribution with packaged artifacts.
3. Add a dependency only in the planner/costing crate that opts in; no shared
   `Job` or `MarkerResult` field is required.
4. Rerun crate tests, all-workspace tests and Clippy after registration. Repeat
   the job/source feature audit when orientation or unit conversion changes.
5. Run paired planner/costing factory acceptance before changing any default.
