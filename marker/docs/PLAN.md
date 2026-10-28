# BETA Marker plan

> Historical proposal, not the current feature list or a commercial superiority
> claim. See [features](FEATURES.md), [roadmap](ROADMAP.md) and
> [measured benchmark caveats](benchmarks.md). The proposed engine choices and
> version targets below are not promises or a record of shipped behaviour.

Date: 18 October 2026.

## Goal

Make an open-source marker planner that a cutting room can use instead of TUKAmark, AccuMark/AccuNest, Diamino or Optitex Marker for plain-fabric markers first. Then add stripes and checks, defects and cut planning.

A marker is the cutting layout. **Marker efficiency** = area of all pieces / (fabric width x marker length).

## What we reuse and what we build

```text
                 REUSE (open source)                      BUILD (BETA Marker, Apache-2.0)
            +-----------------------------+      +-------------------------------------------+
 pattern -> |                             |      | DXF-AAMA/ASTM import: pieces, grain,      |
  (DXF)     |                             |      | notches, drills, sizes, quantities         |
            |                             |      +---------------------+---------------------+
            |                             |                            v
            |                             |      | Garment rules: size ratio, L/R pairs,      |
            |                             |      | grain, nap (one-way), 0/180, tilt, gaps    |
            |                             |      +---------------------+---------------------+
            | sparrow 0.3.0 (MIT)         | <----+ Nesting adapter (Rust)                     |
            |   best open nesting search  |      +---------------------+---------------------+
            | jagua-rs 1.0 (MPL-2.0)      |                            v
            |   collision engine          |      | Independent validator: counts, overlaps,   |
            +-----------------------------+      | grain, width, gaps                         |
                                                 +---------------------+---------------------+
                                                                       v
                                                 | Browser editor (TypeScript + Rust WASM)    |
                                                 | Output: PLT/HPGL, PDF 1:1, SVG              |
                                                 | Benchmark: academic sets + Lectra dataset   |
                                                 +-------------------------------------------+
```

## Licence check (verified 18 October 2026 on GitHub)

| Project | Licence | Latest release | How we use it |
|---|---|---|---|
| sparrow | MIT | v0.3.0 (1 Oct 2026), active | Cargo git dependency (tag v0.3.0) |
| jagua-rs | MPL-2.0 | v1.1.0 (9 Oct 2026); sparrow pins 1.0.0 | Unmodified crates.io dependency. MPL is file-level copyleft: if we ever change its files, we must publish those files. We do not change them. |
| PackingSolver | MIT | rolling "latest" (Jul 2026), active | Planned second engine (version 0.2) |
| Lectra nesting dataset (Zenodo 7030786) | CC BY 4.0 | 100,000 tasks | Benchmark only, with attribution |

## Why everything is in Rust

The nesting engine (sparrow) is Rust, so the whole core is Rust: import, rules, nesting, validation, output, server and benchmark. The browser screen must be TypeScript (browsers run JavaScript), but it calls Rust compiled to WebAssembly for checks. Speed is not a problem. Two small limits:

1. The Lectra dataset is stored as Python pandas files. A one-time Python script converts a sample to JSON. It runs once, so its speed does not matter.
2. Rust takes longer to write than Python for UI-like code. That is why the UI stays TypeScript.

## How we measure "better than Tukatech"

Same pieces, same fabric width, same rules, same time limit. We report four things:

| Measure | How | Target for version 0.1 |
|---|---|---|
| Marker efficiency | Our length vs your CAD's length on your real markers | Within 1 percentage point; then beat it |
| Time to solution | Seconds until the marker is within 0.5 point of our final result | Under 2 minutes on a normal PC |
| Garment rule coverage | Checklist below (grain, nap, pairs, ratio, gaps, plaid, defects, ...) | Plain fabric rules complete |
| Cost | Licence + hardware per seat per year | Free software; any Windows PC |

Benchmarks without your data:

1. **Academic sets** (shirts, trousers, swim): compare with the published sparrow results (shirts 90.92%, trousers 92.62%, swim 79.83% best).
2. **Lectra dataset**: 100,000 real fashion nesting tasks with the efficiency reached by Lectra's own nesting. This is the closest public comparison with a commercial engine. We report only tasks where we can apply the same rules.

1 percentage point matters: 90% to 91% is about 1.1% less fabric for the same marker.

## Comparison

"Yes" means documented by the vendor. Prices are published list prices where available (USD). Our column shows what version 0.1 will do.

| Feature | Tukatech TUKAmark / SMARTmark | Gerber AccuMark / AccuNest | Lectra Diamino | Optitex Marker | BETA Marker 0.1 |
|---|---|---|---|---|---|
| Automatic nesting | Yes (SMARTmark extra) | Yes | Yes (cloud: Quick Nest / Flex Offer) | Yes (Nest++ family) | Yes (sparrow) |
| Manual edit, partial re-nest | Yes | Yes | Yes | Yes | Edit yes; partial re-nest in 0.2 |
| Grain, 0/180, one-way nap | Yes | Yes | Yes | Yes | Yes |
| Left/right pairs, size ratio | Yes | Yes | Yes | Yes | Yes |
| Gaps / buffers | Yes | Yes | Yes | Yes | Yes |
| Stripe / plaid matching | Yes | Line matching | Yes | Yes (Match++) | No (later) |
| Fabric defects, shading | Yes | Yes | Yes | Yes | No (later) |
| Tubular / folded fabric | Not verified | Yes | Not verified | Yes | No (later) |
| Fusing blocks | Yes | Yes | Yes | Yes | No (later) |
| Cut order planning | TUKAcutplan | AccuPlan | Separate | CutPlan | No (later) |
| DXF-AAMA/ASTM import | Yes | Yes | Yes | Yes | Yes |
| Plotter output | PDF, plotter | Yes | HPGL | HPGL, ISO | PLT/HPGL, PDF |
| Runs offline on any PC | Windows | Windows | Windows + cloud | Windows | Windows, Mac, Linux, browser |
| Open source | No | No | No | No | Yes (Apache-2.0) |
| Price | TUKAcad Pro $199/month incl. TUKAmark; SMARTmark extra | Quote | Quote | Quote | Free |
| Public, repeatable benchmark | No | No | No | No | Yes |

Vendor fabric-saving claims (1-5%) are their own tests on their own data. Nobody has published a fair test between these tools. Our benchmark will be public and repeatable.

## Data we need from you

1. 3-5 real styles as **DXF-AAMA or ASTM** export from your current CAD (shirt, trouser, T-shirt, jeans if possible).
2. For each: the **marker your CAD made**: fabric width, size ratio, rules (nap, 0/180, gaps), **marker length and efficiency**, and how long the auto-nest ran.
3. A plotter file (PLT) from your cutting room, so we can check that our PLT plots at the right scale on your plotter.

We keep these files private. They are never committed or published without your written permission.

## Roadmap

| Version | Scope |
|---|---|
| 0.1 (this MVP) | Import, plain-fabric rules, nesting, validator, browser view/edit, PLT/PDF, benchmarks |
| 0.2 | Partial re-nest around locked pieces, PackingSolver as second engine, multi-run portfolio, efficiency report |
| 0.3 | Stripes/checks, fabric defects, shading groups, tubular/folded fabric |
| 0.4 | Cut order planning (size ratios, ply heights, lay lengths) |
