# Stripe/check matching and shade bands

On striped or checked fabric, stripes should line up across seams. Examples are
a pocket sewn to a shirt front, the two fronts at the centre opening, and a sleeve
at an armhole notch. A **match point** marks the place to align. The planner puts
partner points at the same position within the fabric repeat. They can be one or
many whole repeats apart on the cutting table.

Shade can change along a roll or across its width. A **bundle** is all the pattern
pieces for one garment. All pieces of a bundle must come from one shade band, so
their colours match after sewing.

## API and matching convention

- `nest_with_rules(job, opts)` returns a checked marker or a descriptive error.
- `needs_rules(job)` selects jobs with matching rules or shade bands.
- `match_errors(job, result)` returns ordinary core `Issue` values. It is independent
  of the core validator's matching checks. It does not check demand or collisions.

For each active axis, matching checks the circular distance from zero of
`(placed_a - placed_b - rule.offset) mod period`. With no `b`, use the fabric
repeat offset as `placed_b`. An axis with no period has no restriction. A zero
`tolerance_mm` means 1 mm. For a 40 mm repeat, 39.8 mm and 0.2 mm differ by 0.4 mm.
Mirroring happens before rotation. Shrinkage is applied before transforming points.
All copies of a referenced piece in the same bundle must match every partner copy.
Different bundles can use different phases. Missing partners or bundle IDs are errors.

Bundle numbers follow natural apparel size order (XXXS through 5XL), then numeric sizes in numeric order, then lexical
order for other labels, then garment number within that size. This agrees with
the nesting adapter. Without an order, all pieces form bundle 0. Copy numbers remain global
within each piece/mirror state, including after shade sub-markers are joined.

## Placement method

1. Nest a baseline with `beta-marker-nest` (sparrow for ordinary jobs).
   For open fabric without user-fixed pieces, add
   half the repeat diagonal on its constrained axes as clearance around each
   matched piece. This gives
   room to move each piece up to half a repeat on each axis. If the extra clearance
   prevents nesting, retry without it. User-fixed jobs use their original buffers.
2. Connect matched copies within each bundle. For each axis, turn the match rules
   into a graph of translation phases. Relative rules link two translations;
   absolute rules link a translation to the fabric reference. Each component gets
   a freely chosen anchor phase unless an absolute rule or user-fixed piece sets it.
3. Propagate phases through the graph. Try the baseline rotation first, then allowed
   rotations. Try the baseline anchor phase and four phases starting at the lower
   fabric edge, spaced by a quarter repeat. Reject inconsistent phase cycles.
4. First try shifting matched baseline pieces to their nearest lattice positions.
   Keep unmatched pieces where they are and validate the whole result. If this
   fails, place matched pieces in descending area order. Search bottom-left positions on
   the allowed lattice. On unconstrained axes, try fabric edges, baseline positions,
   and contacts with existing piece/defect bounds. Use exact polygon intersection
   and distance tests through `geo`, with bounding-box rejection first. Reserve
   piece gaps, piece buffers, edge gaps and a 0.025 mm numerical guard.
5. Pass matched placements and user-fixed placements as `NestOptions.fixed`.
   Sparrow's adapter fills the remaining demand. Check that fixed pieces did not
   move, run the core validator, then run this crate's matching checks. Keep the
   checked shifted baseline if it is shorter than the final re-nest.

The merged `nest2` adapter uses its own f64 polygon packer when fixed pieces are
present. Sparrow itself cannot keep pieces locked or enforce repeat positions.
The buffered baseline keeps an alternative layout available if the fixed-piece
packer leaves more unused space.

Fusing blocks use their enlarged rectangle as the collision envelope. The match
points keep their original meaning. Defects are obstacles. User-fixed pieces keep
their transform and lock flag.

This is a bounded greedy search, not an optimal solver. It tries at most 512
rotation combinations and five anchor choices per combination. Small repeats use
contact events instead of visiting millions of lattice lines; at most 256 regular
positions per axis are enumerated. Free rotations use the baseline and four right
angles. Tilt is inherited from the baseline; it is not exhaustively searched.
The solver targets exact phase equality; tolerance is used by the checker, not as
extra packing slack. A failed search does not prove that a layout is impossible.
Search, setup and final validation add overhead to the engine time budget.

## Shade bands

Length bands have the fixed length `band_mm`. The scheduler assigns whole bundles
by descending garment area and balances estimated loads. It estimates the band
count from a 78% area target before assigning bundles. This pairs larger garments
with smaller garments. It nests each section separately, and splits failed sections by moving
a whole bundle to a new band. A bundle that does not fit a band returns an error.
Sections are translated by exact multiples of the band length. The final reported
marker length still follows the core contract: the largest placed x coordinate.
Unused tail space in the final shade band is not included in that length.
Internal boundaries reserve half the piece gap on each side; length-band ends
also reserve the largest piece buffer. This preserves gaps between adjacent bands.

Width lanes split the width at multiples of `band_mm`, including the last partial
lane. Whole bundles go to an eligible lane with the least area per lane width.
A lane must fit every piece in the garment. Each lane is nested separately and
shifted to its global y position. Loads are an estimate; this method does not
exhaustively try all bundle-to-lane assignments.

Fixed bundles stay in their original band. Sub-markers translate defects and the
absolute repeat reference into local coordinates. Relative matching offsets stay
unchanged. Matching and shade bands can therefore be used together.
The optimizer time budget is shared between the remaining sections. Failed
attempts use their share; each later construction gets at least 0.01 seconds.
Job setup and validation add wall-clock time to that budget.

## Fixtures and attribution

`tests/fixtures/simon-check.job.json` uses the original cut outlines from
`testdata/open/freesewing/simon-M.job.json`, FreeSewing 4.10.2, **MIT**, copyright
Joost De Cock and FreeSewing contributors. The original source metadata is retained.
See the source folder's `LICENSE.MIT`. We added these match points in drafted mm:

| Piece | Point | Position | Reason |
|---|---|---|---|
| Left and right front | centre | (0, -470) | Existing centre-front waist notches |
| Left front | pocket | (120, -250) | Test pocket's top-left sewing position |
| Added pocket | top_left | (0, 0) | Top-left of our 130 × 150 mm rectangle |
| Left front | armhole | (220.75, -157.5187) | Existing front armhole notch |
| Sleeve | armhole | (102.1587, 92.971) | Existing sleeve armhole notch |
| Yoke and back | join | (222.9582, -120.7893) | Existing yoke/back joining notch |

The pocket is our own small test pattern, not a FreeSewing Simon pocket. The four
rules are fronts x/y, pocket/front x/y, sleeves/front y, and yokes/back x. The fabric
has a 40 mm check repeat on both axes. These are matching demonstrations, not a
production sewing specification.

The two shade fixtures use the original `simon-ratio` pieces and S1 M2 L2 XL1 ratio.
They set 3000 mm length bands or 750 mm width lanes. Run the Rust fixture generator
from the marker workspace to reproduce all three files:

```text
cargo run -p beta-marker-match --example make_fixtures
cargo test -p beta-marker-match
cargo run -p beta-marker-match --example measure -- 0.5
```

Set `CARGO_TARGET_DIR`, `CARGO_BUILD_JOBS` and `BETA_DATA_DIR` for your environment.
The measurement tool reports efficiency, percentage-point loss, wall time and
validation for the same job with and without rules, seed 42 and two workers.

## Measured results

These runs used a 2 second optimizer budget, seed 42, two workers and the dev
profile with optimized dependencies. Wall time includes setup, all sub-markers
and validation; it excludes compilation. The machine was shared with other
builds, so wall times vary. Each result passed the core validator and this crate's
matching and shade checks. The matching run used the merged `marker/nest2`
commit `c59bc59`.

| Job | Rules | Efficiency without | With | Loss (points) | Runtime without / with (s) | Valid |
|---|---|---:|---:|---:|---:|---|
| Simon M + own pocket | 40 mm x/y checks, four rules | 75.67% | 73.99% | 1.68 | 4.360 / 22.629 | yes |
| Simon ratio | 3000 mm length bands | 80.68% | 73.60% | 7.08 | 22.200 / 9.627 | yes |
| Simon ratio | 750 mm width lanes | 80.68% | 70.09% | 10.59 | 24.931 / 9.040 | yes |

The three length bands contain S+XL, M+L and M+L. Their used lengths are
2848.7, 2866.1 and 2870.4 mm; the marker ends at 8870.4 mm. The unconstrained
marker is 8092.5 mm. Fixed band boundaries leave space that the next garment
cannot use. Width lanes hold bundles {S, M, XL} and {M, L, L}; their lengths are
9315.2 and 8948.5 mm. The check fixture grows from 1407.2 to 1439.1 mm.

An initial first-fit area assignment grouped XL+L together, failed to fit that
pair, and used four length bands (61.96%). Estimating the band count first and
balancing garment areas removed that extra band. The committed scheduler uses
the balanced assignment. These small runs demonstrate feasibility and cost;
they do not establish an optimum or a general factory efficiency claim.

## Integration checks

The suite covers circular tolerance, signed offsets, absolute fabric references,
mirroring before rotation, shrinkage at match points, every partner copy,
missing partners, inconsistent cycles, rotation-dependent cycles, defects,
buffers, user-fixed transforms, shade-boundary gaps, copy identities, natural
size order, both Simon shade fixtures and the Simon check fixture. The combined
absolute-match/shade test uses 117 mm bands and forces a nonzero band shift, so
the band spacing is deliberately not a multiple of the 40 mm repeat.

The core validator on this base still has its version 0.1 checks. Matching and
shade rules are also checked here before returning a result. After the `qa2`
validator is merged, repeat `cargo test --workspace` and the measurement command,
including combined folded/tubular, faceless, block and shrinkage cases. The
matching helper does not call the core validator's private matching implementation.

Bundle ordering now comes from `rules::bundle_sizes(&Job)`, shared with the
nesting engine and validator. No model field change is required.

## Dependencies

New direct dependency: `geo` 0.33, **MIT OR Apache-2.0**, with default features
disabled. This is already used by the nesting crate. No new transitive package is
needed beyond the workspace's existing nesting dependency graph. The geometry
checks use public `geo` APIs, not the core validator's private implementation.

## Version 0.3 shade experiments

Length sections now put the shortest measured tail last when there are no
fixed pieces, defects, or match rules. This swaps whole sections by exact band
multiples and preserves bundle IDs, rotations, spacing, and copy identities.
The last band already has a short charged length; this change chooses which
section benefits from it.

`nest_with_shade_search(job, opts, attempts)` can try up to 16 assignments.
The candidates include area balancing, packed-first length assignment, ascending
size/area order, and a deterministic mixed order. Probes share 35% of the budget;
the remaining wall time refines the best checked assignment. A failed refinement
cannot replace the valid incumbent. `nest_with_rules` keeps one assignment until
benchmarks establish a better default. To compare, run the release `measure`
example with `<seconds> shade <attempts>`; 1 keeps the base scheduler and 4 tries
the alternative assignments. All source fixtures remain unchanged.

Bundle IDs now use `rules::bundle_sizes`, including numeric size order. The
previous private apparel-size ordering code has been removed. Width lanes still
have a geometric boundary: pieces from a garment cannot cross it. Assignment
search can improve grouping, but does not remove that constraint.
