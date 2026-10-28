# Marker quality, area improvement and order-cost comparison

Implemented on `marker/efficiency10`, based on `marker/efficiency9` at 47412d4.
This is a local experimental build. The published 0.7.0 release has not changed.

## What works

### 1. Keep a valid shorter marker after fresh search

The Marker editor and Style workflow ask Rust to compare the previous marker
with the completed search result. This works without using the previous marker
as the engine's warm start. Rust recomputes lengths from the current geometry;
cached imported metrics cannot decide the winner. The prior layout must pass
current quantity, grain, matching, spacing, defect and editor-boundary checks.
Any required fixed records must remain exactly equal. A stale, incomplete or
wrong-width incumbent cannot displace the new valid result. Invalid search
output produces an error instead of being hidden by an old layout.

The chosen marker keeps its engine provenance. The editor explains when it kept
the previous shorter marker. This protects an available checked incumbent; it is
not a promise to beat an independent cold search.

Browser constrained search now triggers a stronger proposal after three stagnant
rounds. Folded search probes another allocation after two stagnant rounds, with
an occasional fresh probe while improvements continue. Existing complete-marker
validation and cancellation recovery remain mandatory.

An **optional strategy comparison** is also available in the native editor/server
and CLI (`nest --adaptive`). It keeps a 70% baseline stage, compares a job-class
alternative, then refines the best checked incumbent with the remaining budget.
Tiny budgets use the established path. Native strategy comparison requires jobs
without stripe/check or shade matching; the ordinary matching path remains
available. Browser search uses its existing persistent sessions rather than this
native three-stage schedule.

The strategy comparison remains **off by default**. The pilot below does not
justify changing the ordinary search default.

### 2. Improve a selected area, preview, apply and undo

In Marker, choose **Improve area**. Draw a Select box from empty canvas space,
then reopen the tool, or enter two corners. Only fully enclosed unlocked pieces
are movable. Crossing pieces and every outside placement remain unchanged.

Rust tries bounded sequences of horizontal and vertical contact moves. It keeps
only complete valid candidates inside the region and preserves the whole marker
length before considering the selected area's end. Defects, cut sections and
table boundaries remain active. The bounded tool supports up to 32 movable
pieces in a marker of at most 1000 placements. It is a local contact search, not
an exhaustive remove-and-reinsert optimizer.

Preview runs in a disposable Worker. The dialog shows the selected area before
and after, with marker-end and region-end changes in millimetres. Cancel stops
the Worker and leaves the document untouched. Apply rejects a stale preview and
commits the accepted change as one history operation. Undo restores the previous
document. A no-change preview cannot be applied.

### 3. Compare fabric width, ratio and measured order cost

Open **Cut plan → Compare fabric widths, ratios and order cost**. The screen uses
the order quantities and lay limits above it. Enter width offers, prices per
metre or kilogram/GSM, lay cost, purchase allowance and the fraction of opened
remnants charged to the order. Optional ratio menus can be compared with the
automatic ratio search. Optional stock comes from the Rolls register, with
explicit eligible roll IDs.

The existing Rust planner performs the actual nesting, integer lay selection,
costing and stock allocation. A new Worker keeps this work off the UI thread.
The result lists failed, short-only and final-checked alternatives. Only complete,
feasible final alternatives have an enabled Use plan action. The source geometry,
width and order snapshot travel with the selected plan. Inputs and the complete
comparison can be downloaded as JSON.

Offer-specific stock, price or shrinkage failures now remain failed rows instead
of aborting unrelated offers. Ambiguous offer IDs and malformed request structure
still fail the request. All stock restrictions remain enforced; invalid stock is
not silently filtered into a valid offer. Callback cancellation still propagates.

Stop terminates the comparison Worker. It does not select a partial winner and
retains the previous completed comparison. The common API also exposes
`POST /api/plan/cost`; desktop requests use its isolated compute subprocess.
Interactive limits are checked in Rust: at most 8 offers, 4 ratio sets, 16 total
alternatives, 32 ratios per menu, 4 finalists and 16 measured refinements per
alternative. Short markers allow 0.05–30 seconds, final markers at most 120 seconds.

This first cost screen supports **open fabric**. Without supplied stock, it
checks geometry and demand, not stock availability. Mapped defects still need
cutting-release checks. The bounded shortlist is not an exact joint roll/ratio
optimum. Zero prices are allowed for fabric-only trials and must not be treated
as a priced factory quotation.
The editor's saved marker/table end is applied as a maximum length to every
alternative. A tighter existing maximum is retained. Saved cut sections are
explicitly rejected because these trials do not yet optimize around those lines.

## Paired native strategy pilot

Five frozen public FreeSewing cases, initial seeds 1 and 7, 12 requested seconds,
one requested separator worker. Baseline and adaptive order is reversed for seed
7. No builds ran during these measurements. Every result was independently
revalidated, including exact fixed-record equality: **20 valid layouts**.
Actual wall time is retained; the upstream search can overrun its allowance.

| Case | Mean fabric saved by adaptive % | Baseline wall s, seeds 1 / 7 | Adaptive wall s, seeds 1 / 7 |
| --- | ---: | ---: | ---: |
| Hoodie folded | -0.37 | 12.65 / 13.01 | 14.95 / 14.67 |
| Shirt with early lock | +0.86 | 12.10 / 12.09 | 12.13 / 12.13 |
| Shirt with interior defect | -0.53 | 12.12 / 12.11 | 12.12 / 12.12 |
| T-shirt folded | approximately 0.00 | 12.03 / 12.02 | 12.03 / 12.02 |
| T-shirt open, native execution | -0.35 | 12.36 / 12.44 | 12.53 / 12.40 |

Across the ten pairs, mean fabric saving is **-0.078%**, median **-0.260%**,
and the range is **-2.132% to +3.856%**. Negative saving means more fabric.
The small positive mean on the locked shirt hides opposite results across the
two seeds. Folded hoodies also used more wall time. This is mixed evidence,
not a general improvement. Ordinary search therefore remains the default.

The T-shirt open case retains the historical input ID `teagan-browser` but this
pilot executes it natively. It is not a browser-speed measurement. Cold strategy
trials do not use the new editor incumbent guard; that guard is tested separately.

[Every pair and actual time](efficiency10/adaptive-summary.json).
The Rust `adaptive_study` example generates raw paired results and its `--audit`
mode recomputes validation and summary values from the frozen inputs and JSONL.
The raw markers and source-case file remain in the task-owned external evidence
directory; only the small aggregate is committed.

## Verification

- Release library tests: 120 passed, zero failures; three existing performance
  or external-data tests ignored across edit, nest, API and plan.
- CLI release tests: 66 passed, zero failures; one existing ignored test.
- Web tests: 222 passed, zero failures or skips, including real WASM-backed Style
  preview checks. Web type checking passed.
- Strict Clippy passed for all targets of the changed Rust libraries and CLI.
- The native Windows desktop compiled with `cargo check --offline --locked`.
  No installer or physical cutter acceptance is claimed.
- Production marker and cutting-room WASM and the complete static app built.
  Unchanged Pattern and Floor assets come from the established local 0.7.0 build.
- Real-browser functional checks cover region preview/apply/undo/cancel, exact
  outside preservation, cost comparison with an impossible width, final-plan
  handoff, and comparison Stop retaining a completed result.

[Browser evidence](efficiency10/browser-smoke.json) and
[artifact fingerprints](efficiency10/verification.json) are recorded with this report.

The original synthetic area witness reduces the marker end from 50 to 40 mm;
the locked outside record is identical and Undo restores the original drawing.
This is tool correctness evidence, not a garment fabric-saving claim.

The shared preview screenshot command timed out. Functional browser evaluation
continued to work; visual checks used a separate Chrome tab. Only original
synthetic fixtures and public FreeSewing data were used. Private factory files
were not accessed. No dependencies, licences or proprietary formats were added,
and no upstream dependency was modified. The changes have not been published.
