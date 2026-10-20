# Width and ratio planning by real cost

`cost::compare(&CostRequest)` compares `WidthOffer`s and explicit `RatioSet`
menus. Empty menus enable automatic ratio search. Offers provide usable width,
`FabricPrice::Metre` or `Kilogram { price, gsm }`, currency lay cost, purchase
wastage, and optional measured roll stock/restrictions. All prices use one named
currency. This width-cost API currently requires open fabric.

The result keeps every short trial, geometry failure, quantity shortage and
stock shortage. It shortlists feasible alternatives by total order cost,
re-nests distinct selected ratios for the longer final budget, retains shorter
validated incumbents, recomputes integer lays, then freshly allocates/audits rolls.
Only feasible, successfully final-evaluated alternatives can win. `chosen` indexes
`trials`; `reason` explains the width, ratio set and cost components.

```text
purchased metres = consumed cloth + charged end bits + purchase allowance
consumed cloth   = marker length x plies + head/tail allowance + splice overlap
total cost       = fabric purchase cost + number of lays x lay cost
kg               = linear metres x physical width in m x GSM / 1000
```

Opened-roll remainders are recoverable inventory. `end_bit_charge_fraction`
determines the value borne by this order: zero means full recovery, one means
fully charged. Unopened rolls are never waste. Kg pricing uses actual measured
widths of consumed rolls and charged ends, so wider stock trim is not free.
`OrderCost` exposes each component. Purchase wastage affects the fabric/lay
price conversion used by the integer solver.

Eligible stock must be in one default roll-library shrinkage band pair and already
match the job's shrinkage compensation. Stock needing
more compensation is rejected. Shade identity stays under the allocator's policy.
Source order, offers, ratios, seed, budgets and allocation settings are explicit.
Recorded defects still need mapped-defect checks before physical cutting release.

Calibration now probes each measured garment-count class, selecting its best
measured length per garment, then interpolating gains piecewise between classes.
This avoids applying one small marker's search gain to every larger ratio.
Calibration changes ranking only. Cost and cutting geometry always use actual
independently validated lengths. Final decisions never publish efficiency guesses.

Public APIs:

- `plan_with_progress` and `plan_ratios_with_progress`: checked partial plans.
- `plan_ratios_with_engine`: explicit ratio menus and an application-owned
  `NestExecutor` for queue checkpoints/cancellation. Direct nests stay continuous.
- `refine_plan_with_progress` / `refine_plan_with_engine`: warm-refine selected
  short-plan ratios, preserve feasible original ply actions, recompute integer lays.
- `cost::compare`, `compare_with_progress`, `compare_with_engine`: the priced
  width/ratio matrix and a finally measured selection.
- `cost::price_plan`: re-price and re-allocate an existing measured plan.

Limits: the ratio solver uses a linear fabric/lay objective. Roll-end effects are
scored between supplied width/ratio-menu alternatives. This bounded shortlist is
not an exact joint roll/ratio optimisation. Shortlisted ratios and widths can still
improve with a larger search; an infeasible short stock allocation is not proof of
permanent infeasibility. The API does not silently split lays or change stock
shade/shrinkage groups to fill a shortage. Folded/tubular purchase conventions
remain explicit future width-cost work; production report mass supports them.

No new external dependency is added to the planner. See `OPS_REPORT.md` for the
integration handoff, tests and measured short-nest/budget results.
