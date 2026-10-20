# GarmentCodeData examples

These ten jobs derive from **GarmentCodeData**, licensed under
[Creative Commons Attribution 4.0 International](https://creativecommons.org/licenses/by/4.0/).
Original record: [GarmentCodeData v2, ETH Research Collection](https://doi.org/10.3929/ethz-b-000690432).
The [original v1 record](https://www.research-collection.ethz.ch/entities/researchdata/d9df8fe3-2add-49a2-bb58-21750ca7970c)
also identifies the dataset licence as CC BY 4.0.

Attribution: Maria Korosteleva, Timur Levent Kesdogan, Fabian Kemper,
Stephan Wenninger, Jasmin Koller, Yuhan Zhang, Mario Botsch and Olga Sorkine-Hornung.
**GarmentCodeData: A Dataset of 3D Made-to-Measure Garments With Sewing Patterns.**
Computer Vision – ECCV 2024. [Project and citation](https://igl.ethz.ch/projects/GarmentCodeData/).
No endorsement by the authors is implied.

## Download and sample

ETH's download endpoint was rate-limited during collection. The specification-only
[AIpparel authors' mirror](https://huggingface.co/georgeNakayama/AIpparel) was used.
It identifies these as the original GarmentCodeData sewing patterns. We retain
the original CC BY 4.0 attribution; the mirror's MIT model-card label is not used
to relicense the dataset. The shifted mirror has not been checked byte-for-byte
against the unavailable ETH archives, so its exact upstream v2 provenance remains
a limitation.

- Mirror revision: `f238540ebc7f98dcf5cb7ed1154e8396bc5285fb`.
- Archive: `garment_specifications_archive.tar.gz`, **251,722,049 bytes** (240.06 MiB).
- SHA-256: `fa7d016bde8bf036ce74575b1e8995dc85fc4a86729fad743d8514115d819c86`.
- Extracted: first **300** specification JSON entries in archive order,
  **8,715,725 bytes** total. No meshes were downloaded or unpacked.
- All 300 come from `garments_5000_15/default_body`. This is a convenience sample
  of designs on the reference body, not a representative or randomly sampled corpus.
- Converted: **299** one-garment jobs. One offset is rejected:
  `rand_JCCGDD3H4U`, self-intersection at edges 116 and 118. It is not silently repaired.
- Large outputs: `BETA_DATA_DIR/marker-bench/open-patterns/garmentcode/`.
  Source sample and full manifest: `BETA_DATA_DIR/datasets/garmentcodedata/`.

Only the first ten accepted jobs in filename order are committed. There are three
SVG previews. `report.json` lists their piece counts and areas and the rejection.
The full conversion report outside git lists all 299 jobs.

## Changes and assumptions

Panels become pieces, one copy per source panel. Source properties specify
`units_in_meter: 100` (cm), so coordinates are multiplied by ten. Native panel
y-up coordinates are retained. Quadratic and cubic Bezier controls use the
source's relative chord/perpendicular frame. Circle params are absolute radius
in source units, major-arc flag and sweep flag, as serialized by GarmentCode.
All three curve types occur in this sample. Curves have at most 0.15 mm chord error.

A uniform **10 mm round seam allowance** is added to each panel. The offset is
the union of the panel, outward edge rectangles and round convex caps. The caps
use an inscribed disk polygon with at most 0.05 mm sagitta error. Coordinates are
rounded to 0.0001 mm before union. Together, curve and disk approximation add at
most about 0.2 mm geometric approximation. This is a benchmark cut envelope, not
a seam-specific manufacturing allowance; darts, narrow gaps and hem details may
need different treatment in production. The original stitch boundary is a line mark.

GarmentCode has no textile grain annotation here. We use **local +y**, the panel's
up direction before its 3D rotation. Thus the recorded grain transforms with the
panel in 3D, but the source's placement rotation and translation are not applied
to the flat outline. This is an assumption, not ground-truth fabric grain. It can
be unsuitable for bias-cut panels or trims.

Left/right panels are already separate geometry. These specs contain no symmetry
or mirrored-copy rule, so both are kept with quantity one. Names alone do not
justify collapsing them into mirrored demands. Stitching relationships are not
converted into notches or drills. Material is `fabric`; draping-material labels
are not a per-panel factory cutlist. No dataset contents are executed.

## Regenerate

Set `BETA_DATA_DIR`, then run from the repository root:

```powershell
npm install --prefix marker/tools/open-patterns/garmentcode --package-lock=false --ignore-scripts --cache "$env:BETA_DATA_DIR/npm-cache"
node marker/tools/open-patterns/garmentcode/sample.mjs
node marker/tools/open-patterns/garmentcode/convert.mjs
```

The sampler checks archive size and SHA-256, reads only regular JSON tar members,
uses basenames rather than archive paths, and stops after 300 files. It never
extracts links or executes Python, pickle, YAML or scripts. Node.js 22 or later is required.
