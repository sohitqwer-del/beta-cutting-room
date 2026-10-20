# Original garment DXF exercises

These files were drawn by Project BETA from our own parametric formulas. They
contain no copied commercial patterns, vendor files, or vendor specifications.
They are simple garment-like geometry for testing, not production fit patterns.

Copyright 2026 Project BETA contributors. This dataset, including expected.json,
is licensed under **Creative Commons Attribution 4.0 International (CC BY 4.0)**.
Attribution: “Original parametric garment exercises by Project BETA contributors,
2026, CC BY 4.0.” Licence: https://creativecommons.org/licenses/by/4.0/
The Rust generator is Apache-2.0, like the project code.

| File | Units | Pieces | Contents |
|---|---|---:|---|
| shirt-aama-mm.dxf | mm | 32 | Eight pieces in S, M, L, XL: front, back, yoke, sleeve, collar, collar stand, cuff, pocket |
| trouser-aama-inch.dxf | inches | 20 | Five pieces in 30, 32, 34, 36: front, back, waistband, pocket bag, fly |
| tee-astm-mm.dxf | mm | 12 | Four pieces in S, M, L: front, back, sleeve, neck band; one block per piece per size |
| generic-polylines.dxf | mm | 2 | Two outer polygons and one nested internal polygon, with grain lines |
| grade-base-mm.dxf | mm | 1 base | Original 100 x 50 panel, four numbered grade anchors, one interpolated point |
| grade-cumulative.rul | mm | 3 sizes | Original S/M/L base-relative corner offsets |
| grade-incremental.rul | mm | 3 sizes | Same offsets expressed as adjacent size steps |
| comparison-marker.plt | 40 units/mm | 2 copies | Original plotted panels with frame, labels and drills |

The shirt front and sleeve and the trouser front/back have Quantity 2. Their
annotations state that they are intended pairs. Standard AAMA has no pair flag:
import preserves Quantity 2 as two normal copies unless `--pairs` is passed.
The same option also splits other even quantities, including yokes and cuffs.
Shirt yokes have Quantity 2 and cuffs Quantity 4. Pocket-position drills appear
on the shirt fronts; the pocket also has a drill. Each garment piece has two
notches and a directed grain line. Curved armholes, necklines, sleeve caps and
crotches use 32-step original cubic formulas. Size increments are fixed in the
generator, not an external grading table. All fixtures are below 1 MB.

Run this command from `marker/` to reproduce the DXFs and area/count manifest:

```sh
cargo run -p beta-marker-dxf --example make_testdata
cargo run -p beta-marker-dxf --example make_v02_testdata
```

The generator is `crates/beta-marker-dxf/examples/make_testdata.rs`; its own
drafting formulas are in `examples/support/patterns.rs`. It writes only this
directory, deterministically, with no network access. `expected.json` records
the formula polygon areas in mm², quantity, notch counts and drill counts,
computed before DXF serialization. Tests compare imports against these values
within 0.5%. Generic outer areas are 150,000 mm² and 85,000 mm²; the inner square
is a 1,600 mm² Line mark and is not subtracted from the cut area.

The v0.2 generator writes a base panel plus equivalent cumulative/incremental
rule tables. S is 96 x 48 mm, M is 100 x 50 mm, L is 104 x 52 mm. The midpoint
on the top edge has no rule and tests interpolation. The comparison PLT has two
4,500 mm² panels, width 150 mm and cut length 230 mm: 26.0869565% efficiency.
Its 300 mm frame length is not counted as a cut piece or consumption. These
fixtures share this directory's original-data attribution and CC BY 4.0 licence.

## Independent cut and stitch grading

The grade-cut-sew and grade-sew-only fixtures are original synthetic squares
with independent cut/stitch rules and an internal drill rule. They contain no
factory geometry.
