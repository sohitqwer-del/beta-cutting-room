# Plies, passes and lays: one meaning everywhere

Round 0.7. Before this round, "plies" meant fabric layers on the Style Lay step,
but spread passes in the cut planner and the Floor. Style handed its layer count
to the planner as "Max plies per lay", so a folded plan could stack twice the
layers intended (audit 0.7, problem 6).

## The words

| Word | Meaning | Open fabric | Folded or tube fabric |
| --- | --- | --- | --- |
| **ply** | One layer of fabric on the table. | 1 per pass | 2 per pass |
| **pass** | One length of fabric laid from the roll. | 1 ply | 2 plies (the fabric arrives doubled) |
| **lay** (spread) | One marker spread over a number of plies. | plies = passes | plies = 2 × passes |

- Garments in a lay = marker ratio × passes. A two-layer marker already places
  each garment across both layers.
- Fabric for a lay = (marker length + 2 × end allowance) × passes.
- The table or knife limit is a number of plies.

Example: a folded T-shirt marker with ratio S1 M2 L2 XL1, 40 plies = 20 passes,
cuts 6 × 20 = 120 garments and uses 20 marker lengths of folded fabric.

Face-to-face open lays lay one ply per pass, in pairs. The Style Lay step already
requires an even number of plies for them. The planner does not yet force an even
number of plies for face-to-face lays (open issue).

## Contract

- `beta_marker_plan::plies_per_pass(layout)` returns 1 (open) or 2 (folded, tubular).
- Cut order format **`beta-marker.cutorder/v2`**: `max_plies` counts plies. The
  planner uses at most `max_plies / plies_per_pass` passes, rounded down. A folded
  or tube order needs `max_plies >= 2`.
- Older `beta-marker.cutorder/v1` files keep their old meaning: `max_plies`
  counted passes.
- Cut plan format **`beta-marker.cutplan/v2`**: each lay has `plies` (layers) and
  `passes` (lengths laid). `fabric_m` and `planned` use passes.
- A `beta-marker.cutplan/v1` lay has no `passes`; its `plies` counted passes and is
  read as `passes`.
- Roll allocation (`beta_marker_roll::LayRequest.plies`), spreading plans
  (`allocated_plies`, `RollAllocation.plies`), cutting-room shade runs
  (`PlyRun`, `Bundle.plies`) and Floor lays count **passes**. Their JSON names are
  kept so saved registers, floor records and ticket files still open. The Rust
  documentation of each field says so.
- `POST /api/roll/spread` marker sources take `passes`. The older name `plies`
  is still accepted with the same meaning.

## Callers changed

| Caller | Before | Now |
| --- | --- | --- |
| Style → Make a cut plan (`style/ui.ts`) | layers into a pass limit | layers into a ply limit (no change to the number; the meaning of the field changed) |
| Web cut plan form (`workflows.ts`) | order v1 | order v2 |
| Cut plan table | plies (passes) | plies, plus "(N passes)" when different |
| Rolls → spread a plan, Bundles → plan lay, Floor → plan lay | `lay.plies` | `lay.passes` |
| CLI `plan`, queue workbook, plan pipeline | one "Plies" column | "Plies" and "Passes" columns |
| Production report `LayTicket` | `plies` = passes | `plies` = layers, new `passes` |
