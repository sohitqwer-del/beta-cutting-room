# Fabric rolls and spreading plans

Before cutting, the factory checks each fabric roll: its length, usable width and
defects. The four-point inspection system assigns one to four penalty points to a
flaw. The factory sorts rolls into shade groups because slightly different colours
must not mix in one garment. It also tests shrinkage along and across the cloth.
The cutting room then chooses rolls for each lay (a stack of fabric plies), decides
where to splice pieces, and tries to leave small end bits.

This crate does those calculations in Rust. It has no runtime network access and
does not depend on the cut-plan crate. It uses the existing core `Shrinkage` and
`Zone` types. The main CLI adapters are in `beta-marker-cli::cmd::roll`.

## Data and units

All public input and output types support serde JSON. Distances along rolls and
markers are **metres**. Usable widths, defect sizes, margins and core marker
polygons are **millimetres**. Percent values are percentage points, not fractions.
`RollDefect.along_m` and `across_mm` locate the start of its rectangular footprint.
Points are supplied by the inspector, rather than inferred from defect size.

`RollRegister { rolls: [...] }` contains `Roll` records with `id`, `lot`, `dye_lot`,
`shade_group`, `ticket_length_m`, `measured_length_m`, `usable_width_mm`,
`shrinkage: { length_pct, width_pct }` and `defects`. Each defect has `id`,
`along_m`, `across_mm`, `length_mm`, `width_mm`, `points`, `defect_type` and
`action` (`cut_around`, `cut_out` or `ignore`; default `cut_around`).
IDs must be unique. Measurements must be finite and positive; positions must be
nonnegative and defect rectangles must fit inside the measured roll.

`inspect`, `group_rolls`, `allocate`, `allocate_first_come`, `compare_allocations`,
`validate_plan` and `map_defects` are the main entry points. Invalid input returns
an `anyhow::Error`. Shortage returns an explicit partial plan, with `complete:
false` and warnings. A caller must check completeness before production.

## Inspection formula and buyer policy

The public [Cotton Incorporated fabric-defect inspection
guide](https://www.cottoninc.com/quality-products/textile-resources/fabric-defects-classification/)
describes area-normalised penalty scores and four-point linear caps. In our words:
divide the scored defect points by the inspected usable area, then multiply by
100 in the selected area unit. For length `L` metres and usable width `W`
millimetres:

```text
area_m2                 = L * W / 1000
points_per_100_m2       = scored_points * 100 / area_m2
points_per_100_sq_yd    = points_per_100_m2 * 0.83612736
```

The conversion is exact because one yard is 0.9144 metres. Equivalently, the yard
score is `points * 3600 / (length_yards * width_inches)`. These two numeric scores
are different. Never compare a square-yard limit with a square-metre score.

`InspectionSettings` defaults to limit 40, unit `per100_square_yards`, and
`point_cap: per_yard`. The limit is inclusive and configurable. This is a default
buyer policy, not a claim that every fabric must meet the same limit. Use
`per100_square_metres` for a metric limit. A zero limit is valid.

For `per_yard` and `per_metre`, defect **start positions** are placed into half-open
linear intervals measured from roll start; each interval contributes at most four
points. `none` uses the inspector's supplied points without a linear cap. Reports
retain both raw and scored points. Enter a continuous, long-running flaw as
separate scored observations in each affected inspection interval; this crate
does not invent points for unrecorded intervals or impose a buyer's hole policy.

Roll scores use measured length, never ticket length. The report also gives
measured minus ticket length. Lot summaries group by `(lot, dye_lot)` and divide
total scored points by total inspected area. They report how many individual
rolls failed even if the lot average passes. Inspection does not automatically
remove a roll from stock: pass approved roll IDs to the lay input.

## Shade and shrinkage

`group_rolls` keeps `(lot, dye_lot, shade_group)` separate. Each shrinkage axis is
assigned band `floor(percent / band_step_pct + 0.5)`. Default step 2 means centres
0%, 2%, 4%, etc., with a +/-1 percentage-point half-width. Bands are half-open:
[-1%, 1%) belongs to centre 0; exactly 1% enters the next band. Negative test
results work the same way. Both length and width bands must match.

Each output group gives roll IDs and the arithmetic mean test result as
`beta_marker_core::Shrinkage`. Assign it to `job.fabric.shrinkage` **before**
nesting. Positive values follow the core convention: enlarge the pieces by that
percent. This crate does not reinterpret a laboratory shrinkage percentage as a
different compensation formula. Feed a group's IDs into `eligible_roll_ids` to
keep its stock separate during allocation.

## Lay input and allocation

`allocate` takes a register, a slice of `LayRequest`, and `AllocationSettings`.
For example, one lay request is:

```json
{
  "id": "LAY-01",
  "marker_length_m": 6.2,
  "plies": 30,
  "fabric_width_mm": 1480,
  "shade_rule": { "mode": "same_group" },
  "eligible_roll_ids": [],
  "allow_splicing": true,
  "splice_zones": [{ "start_m": 2.9, "end_m": 3.2 }]
}
```

`same_group` permits any one `(lot, dye_lot, shade_group)` for the whole lay.
`required` adds an `identity` object with those three fields. `allow_mixing` permits
multiple identities and produces a warning if they are used. A narrow roll is
always excluded. Empty `eligible_roll_ids` means all otherwise compatible stock;
a nonempty list is an additional restriction.

The search minimises, in order: unfilled marker metres, opened-roll end bits plus
splice overlaps, then roll transitions. Whole plies cannot be split unless the
lay explicitly allows splicing. The deterministic portfolio includes:

1. The same input-order first-come baseline used for comparison.
2. Descending marker-length first-fit and best-fit passes, width-first passes,
   input order and reproducibly shuffled lay/roll priorities.
3. Residual-aware passes that can join two opened tails instead of opening another
   roll, plus bounded backtracking for plies needing more than two rolls.
4. Repacking with adjacent lay swaps as a local improvement.
5. A branch-and-bound search for small whole-ply requests (default at most 14
   plies, 12 rolls and 100,000 nodes). This enumerates compatible assignments and
   prunes by already-opened capacity. It can recover a feasible plan missed by
   greedy passes. `search.optimal_whole_ply` is true only if this search finishes
   and finds a complete plan. It proves minimum end-bit waste for the whole-ply
   problem; it does not prove globally minimum roll transitions across every
   possible lay ordering.

Large requests and spliced requests are heuristic. A partial result is not a proof
that no feasible allocation exists. Defaults use 24 heuristic passes and 4,000
backtracking nodes per spliced ply. At most 64 roll pieces can form one ply.
Settings bound search dimensions and reject more than 100,000 requested plies.

All segments are spread forward, face up, with the usable left edges aligned.
`SpreadingPlan.lays` is the **execution order**, which may differ from input order.
Each segment records a one-based ply, its roll start, marker start and consumed
length. A roll continues from its previous measured offset across lays.

For a splice at marker position `s` with overlap `o`, the outgoing piece ends at
`s + o`; the incoming piece starts at `s`. The full overlap `[s, s + o]` must fit
in a supplied safe zone. Default overlap is 0.1 m; zero is permitted for a butt
joint. Every overlap consumes real stock and appears separately in the totals.
No operator-specific end allowance, reverse spreading or selvedge recentering is
assumed. Safe zones must be checked against the final marker's pattern pieces.

`validate_plan` independently checks physical roll order and stock balance, ply
coverage, shade, width, stock restrictions, splice coverage/zones, allocation rows,
completeness and totals. It accepts valid partial plans; the caller must still
check `complete`. It does not check nesting geometry.

## End bits, waste and comparison

`total_end_bits_m` is the remaining length on **opened** rolls after all lays.
These end bits are recoverable stock, not a claim that all of it is scrap.
`splice_overlap_waste_m` is the cloth consumed by overlaps.
`total_waste_m` adds these two quantities. Unopened rolls are reported separately
as `unopened_stock_m`; counting them as waste would make every complete
whole-ply allocation have the same apparent waste. Width trim is not included.
Do not use this linear-length objective to compare different marker widths.

For every complete plan:

```text
initial stock = total_used_m + total_end_bits_m + unopened_stock_m
total_used_m  = sum(marker_length_m * allocated_plies) + splice_overlap_waste_m
```

Table rows give lay, roll, contributing plies, used metres, end-bit metres and
shade. Spliced plies appear under each contributing roll; `full_plies` separately
counts uninterrupted plies. End bits appear only at the final use of a roll, so
the table column can be summed. Both report types have `to_csv()` and `to_text()`.
CSV quotes commas, line breaks and quotes in labels. JSON preserves full precision;
tables round displayed values.

`compare_allocations` reports a saving only when **both** plans are complete. It
does not call an incomplete allocation a waste improvement. The synthetic demo
uses an explicitly seeded generator: 40 rolls, pseudo-random lengths from
40.00 to 120.00 m in 0.01 m steps, three shade groups, and 18 lays of 14 plies.
Marker lengths are 5.7, 7.3, 9.4, 6.1, 8.6 and 10.2 m, repeated three times.
Each lay requests a shade identity. This is generated data, not a factory dataset.
With the default settings, both plans complete all 252 plies (1,986.60 m of
marker demand). First come leaves 166.90 m of opened-roll end bits; the portfolio
leaves 133.13 m. The saving is **33.77 m (20.23%)**. Splicing is disabled in this
comparison, so no overlap is counted. Roll changes fall from 40 to 32.
These are end-bit exposure savings on this
fixture, not a claim about every factory order. Run the command below to reproduce
the comparison.

## Defects in a spread

Call `map_defects` for each lay. It intersects each roll flaw with the actual
consumed segment, translates its along-roll distance to marker x, converts metres
to millimetres, and clips to the marker's usable width. A defect crossing a ply
boundary appears separately in each affected ply.

For `cut_around` and at most `cut_around_max_plies` (default 4 requested plies),
the result contains core `Zone` rectangles. Assign the full list to
`job.fabric.defects` and nest again. The marker avoids every affected footprint
on every ply, a conservative union. An optional millimetre margin enlarges and
clips these zones.

For higher-ply lays, or an explicit `cut_out`, the result gives per-ply, full-width
cut-out/replacement instructions. It says whether a supplied splice zone covers
the affected interval. Otherwise it asks for a re-cut or a revised plan.
`requires_replanning` is true: these are pending instructions, **not an already
repaired allocation**. Their replacement cloth and additional overlap are not
silently added to allocation totals. Even a safe-zone match still needs the
configured overlap and replacement stock checked before execution. `ignore`
affects only cutting instructions; its inspection points are still counted.

## Standalone driver and proposed CLI

Run these from the `marker` workspace with the target directory and two build
jobs set as required by the calling environment:

```sh
cargo run -p beta-marker-roll --example roll -- demo --text
cargo run -p beta-marker-roll --example roll -- inspect register.json --csv
cargo run -p beta-marker-roll --example roll -- group register.json
cargo run -p beta-marker-roll --example roll -- allocate register.json lays.json --text
cargo run -p beta-marker-roll --example roll -- compare register.json lays.json
cargo run -p beta-marker-roll --example roll -- defects register.json spread.json
```

Inputs `register.json` and `spread.json` are objects; `lays.json` is an array of
`LayRequest`. An optional JSON settings file follows the other positional input
paths for each command. Output defaults to JSON. `inspect` and `allocate` also
accept `--text` or `--csv`; `demo` supports all three formats. Output goes to
stdout so the caller can choose a destination. JSON partial plans are reviewable
results; the driver does not automatically start production.

Allocation warns when the consumed cloth contains a recorded cutting defect.
`complete` means the requested ply lengths were allocated; it does not mean that
defect repairs or the final marker geometry have already been approved.

The proposed main CLI is `beta-marker roll inspect|group|allocate|compare|defects`
with the same inputs and `--format json|csv|text --output PATH`. The lead can wire
these library entry points later. A future cut-plan adapter should map marker
length and width from millimetres to metres/mm as above, copy the ply count and
assign an explicit shade/shrinkage stock rule. No core contract change is needed.

## Verification and dependencies

Unit tests cover both area formulas, limits and unit conversion, point caps,
area-weighted lots, grouping boundaries and means, known allocation optima,
an independent exhaustive oracle on small instances, shade/width exclusions,
shared stock, multiple splices, physical stock accounting, defect clipping,
pending cut-outs, invalid inputs, serde round trips, table escaping and the
40-roll comparison.

```sh
cargo test -p beta-marker-roll -- --nocapture
cargo test --workspace
cargo clippy --workspace --all-targets
```

No dependencies were added. The existing stub dependencies are the local
`beta-marker-core` (Apache-2.0), `anyhow` (MIT OR Apache-2.0), `serde` (MIT OR
Apache-2.0), and `serde_json` (MIT OR Apache-2.0). The demo's generator is written
here; it needs no random-number dependency.

After merge, re-test the lead's CLI adapter and the cut-plan adapter, then re-nest
and validate jobs using the exported shrinkage and defect zones. Operator review
of splice zones and pending defect repairs remains necessary before a physical
spread. This crate neither controls a spreader machine nor exports a vendor's
spreader protocol.

## Main CLI and adapters

The CLI is now wired:

```text
beta-marker roll inspect rolls.json --format text
beta-marker roll inspect rolls.json --settings inspection.json --format csv -o inspection.csv
beta-marker roll group rolls.json -o groups.json
beta-marker roll allocate rolls.json lays.json -o spread.json
beta-marker roll spread rolls.json cutplan.json --width 1500 -o spread.json
beta-marker roll defects-to-marker rolls.json spread.json -o defects.json
beta-marker roll defects-to-marker rolls.json spread.json --job job.json --lay L001 -o job-with-defects.json
```

`allocate` and `spread` are aliases. Inspection/allocation support JSON, CSV and
text; grouping/defects use JSON. Output defaults to stdout. `--settings` accepts
the corresponding typed settings JSON. CutPlan input needs cuttable `--width`
in mm and includes plan end allowances in every ply. Raw `LayRequest` arrays
retain their own explicit stock restrictions. A bare CutPlan does not carry
stock shrinkage restrictions: use pipeline or the library adapter when these
matter. Partial allocation is saved and exits with an error.

Defect output defaults to per-lay maps. With `--job`, select `--lay` for multi-lay
plans; zones are appended to that job. `--marker-offset-mm` removes the head
allowance from mapped x coordinates. Pending replacement instructions reject
job output; export the map first. Re-nest and independently validate the revised
job before cutting. `demo_register()` returns the original 40-roll synthetic
stock used by the standalone demo and integration measurements.

The plan crate supplies `spreading_requests` and `apply_roll_group`. The cutroom
crate supplies `integration::from_spread`; it uses stable lay IDs rather than
execution-order indices, traces lot/dye/shade per ply, and expands folded cuts.
