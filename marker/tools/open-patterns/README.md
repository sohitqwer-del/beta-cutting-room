# Open garment data generators

These are one-time data tools. They are not part of the Rust product or the npm workspace.
Use Node.js 22 or later. Set `BETA_DATA_DIR` to a data disk before running the commands.
Do not put large downloads, caches or generated benchmark sets in this checkout.

Run from the repository root in PowerShell:

```powershell
npm install --prefix marker/tools/open-patterns/freesewing --package-lock=false --ignore-scripts --cache "$env:BETA_DATA_DIR/npm-cache"
npm install --prefix marker/tools/open-patterns/garmentcode --package-lock=false --ignore-scripts --cache "$env:BETA_DATA_DIR/npm-cache"
node marker/tools/open-patterns/freesewing/generate.mjs
node marker/tools/open-patterns/garmentcode/sample.mjs
node marker/tools/open-patterns/garmentcode/convert.mjs
node marker/tools/open-patterns/report.mjs
node --test marker/tools/open-patterns/geometry.test.mjs
```

FreeSewing takes an optional output directory. GarmentCode sampling takes an optional
data root and sample count (1–1000, default 300). Its converter takes optional input,
benchmark output and small-example output directories, in that order. Use a new
output directory when changing the sample; old output files are not deleted.

The shared validator checks finite vertices, implicit closure, positive area,
non-adjacent edge intersections (including touching edges), grain, quantities,
unique IDs and size demands. Every job must stay below 1,000,000 bytes.
Bezier subdivision uses a 0.15 mm control-hull bound. Circular arcs use a 0.15 mm
sagitta bound. Coordinates are rounded to 0.0001 mm. Output is y-up in millimetres.

The numerical tests cover Bezier loops and inflections, relative control points,
circle sweep/major flags, a known offset area, self-intersections and composite cut paths.
They do not run the nesting engine.

Check generated JSON with the actual Rust types:

```powershell
$env:CARGO_TARGET_DIR = "$env:BETA_DATA_DIR/cargo-target/marker-patterns"
cargo run --manifest-path marker/tools/open-patterns/rust-check/Cargo.toml -- marker/testdata/open/freesewing marker/testdata/open/garmentcode "$env:BETA_DATA_DIR/marker-bench/open-patterns/garmentcode"
```

This utility has an isolated Cargo workspace. It deserializes `Job`, checks the
round trip for unknown fields, calls `check_job`, and checks positive winding and grain.
It does not change the product workspace or its lockfile.

The converter code is Apache-2.0. Source data keeps its own licence. See the
[source record](../../testdata/open/SOURCES.md) and each output directory's README.
All installed generator dependencies were checked as MIT. `hooks` declares MIT
in its older `licenses` array. `polygon-clipping` is pinned to 0.14.3 and `splaytree`
to 3.2.3. This avoids the additional public-domain dependency in later releases,
so the dependency set stays within the requested licence list.

FreeSewing 4.10.2 has known upstream npm audit findings in its pinned dependencies.
These tools only process the curated model sets and JSON data. They are not shipped
as product dependencies. No dependency audit fixes were applied because they would
change the requested upstream version.
