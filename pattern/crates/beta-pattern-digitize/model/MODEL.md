# Quick Marker shape model

This is a small 1-nearest-neighbour model made from public outlines. It suggests
names; it does not identify garments reliably. Every returned name has low
confidence. The operator must confirm it.

Sources: the existing `marker/testdata/open/freesewing` (MIT, Joost De Cock and
FreeSewing contributors) and `marker/testdata/open/garmentcode` (CC BY 4.0).
GarmentCodeData attribution: Maria Korosteleva, Timur Levent Kesdogan, Fabian
Kemper, Stephan Wenninger, Jasmin Koller, Yuhan Zhang, Mario Botsch and Olga
Sorkine-Hornung. *GarmentCodeData: A Dataset of 3D Made-to-Measure Garments With
Sewing Patterns*, ECCV 2024. https://doi.org/10.3929/ethz-b-000690432 .
No endorsement is implied. The stored model contains derived shape descriptors.
Its GarmentCodeData-derived descriptors retain CC BY 4.0 attribution; the Rust
implementation remains Apache-2.0. Source provenance limits remain those of the
open fixture README files; this task fetched no new garment outlines.

The descriptor uses equal arc-length boundary samples; principal-axis dimensions,
perimeter, area/convex-hull fill, radial quantiles and eight width slices. It is
invariant to rotation, translation, uniform scale and reflection. Labels are mapped from public panel names by the explicit rules in
`train_assist.rs`; they are not independently annotated factory labels. Training stores
78 example vectors (17 numbers each), with source names for audit. The classifier
uses Euclidean distance and abstains above distance 1.2. That threshold is a
heuristic, not a calibrated probability. It is deliberately not called AI accuracy
for factory photographs.

## Held-out evaluation

Run `cargo run -p beta-pattern-digitize --example train_assist --
../marker/testdata/open crates/beta-pattern-digitize/model/shape-model.json`
from `pattern/`. Training and scoring are entirely Rust and deterministic.
`evaluation.json` stores every prediction, including errors.

Each shape is tested with its **entire originating garment style withheld**.
All sizes/ratio files and Simon interfacing duplicates are excluded. Near-identical
same-label descriptor duplicates within a style are removed, including most
left/right copies. Related styles may still share construction families. This is
only five FreeSewing styles plus ten committed GarmentCodeData styles; the latter
are a convenient small sample, not a representative random corpus.

**17 / 78 correct = 21.79% top-1 accuracy.** This is below the 24.36% global
majority-class baseline (19 skirt shapes out of 78). Front and back outlines
are often indistinguishable. Several classes occur in only one held-out style,
so the remaining training set cannot name them. Do not interpret training-set
recognition or a mirrored copy of a training outline as generalization.

| Label | Correct / held out |
|---|---:|
| back | 2 / 10 |
| front | 4 / 11 |
| sleeve | 5 / 10 |
| skirt | 2 / 19 |
| waistband | 4 / 9 |
| cuff | 0 / 7 |
| collar | 0 / 2 |
| hood | 0 / 2 |
| placket | 0 / 2 |
| pocket | 0 / 2 |
| trouser | 0 / 2 |
| binding | 0 / 1 |
| yoke | 0 / 1 |

The report scores every nearest label, including distances that runtime would
abstain on. The final bundled model is trained on all 78 examples *after* this
leave-style-out measurement. No real photographed-shape classification test has
been performed. More diverse labelled styles are needed before raising confidence.

Symmetry, mirror matching, duplicate matching and grain fallback use separate
geometry heuristics, not this classifier. Their unit tests establish transformation
and demand semantics, not real-factory precision. Similar shapes can be different
physical pieces; suggestions never delete them or change quantities.
