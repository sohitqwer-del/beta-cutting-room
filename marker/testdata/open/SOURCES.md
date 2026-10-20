# Open garment sources and commercial marker research

Checked **2026-10-18**. Only MIT, Apache, BSD, ISC, CC0 and CC BY sources qualify
for this task. A free download is not enough without a reuse licence.

## Sources used

| Source | Version / licence | What was used |
|---|---|---|
| [FreeSewing](https://freesewing.eu/) / [code](https://codeberg.org/freesewing/freesewing) | npm `@freesewing/*` 4.10.2; MIT; Joost De Cock and contributors | Simon shirt, Teagan T-shirt, Titan trouser block, Hugo hoodie, Bella womenswear block. Four curated measurement sets per style. 14 jobs and five previews. |
| [GarmentCodeData project](https://igl.ethz.ch/projects/GarmentCodeData/) / [ETH v2 record](https://doi.org/10.3929/ethz-b-000690432) / [original record supplied for this task](https://www.research-collection.ethz.ch/entities/researchdata/d9df8fe3-2add-49a2-bb58-21750ca7970c) | CC BY 4.0; Korosteleva et al., ECCV 2024 | 300 JSON specs from the mirror below. 299 valid jobs with 10 mm SA; ten small examples and three previews in git. No meshes. |
| [AIpparel authors' storage](https://huggingface.co/georgeNakayama/AIpparel) / [pinned archive](https://huggingface.co/georgeNakayama/AIpparel/resolve/f238540ebc7f98dcf5cb7ed1154e8396bc5285fb/garment_specifications_archive.tar.gz) | Mirror revision `f238540ebc7f98dcf5cb7ed1154e8396bc5285fb`; original GarmentCodeData CC BY 4.0 retained | Patterns-only archive: 251,722,049 bytes. First 300 JSON entries: 8,715,725 bytes. The MIT model-card label is not treated as a new licence for GarmentCodeData. |
| [Open+Pattern PT windbreaker](https://openpattern.io/product/us-pt-windbreaker-jacket/) / [public product metadata](https://openpattern.io/wp-json/wp/v2/product/232) / [public download metadata](https://openpattern.io/wp-json/wp/v2/media/2447) | CC BY 4.0; Open+Pattern | Real DXF-AAMA, graded XS–XXL. 539,921-byte ZIP; 3,597,417-byte full DXF on the data disk. A 509,410-byte M subset with 35 blocks is committed. |

GarmentCodeData citation: Maria Korosteleva, Timur Levent Kesdogan, Fabian Kemper,
Stephan Wenninger, Jasmin Koller, Yuhan Zhang, Mario Botsch and Olga Sorkine-Hornung.
*GarmentCodeData: A Dataset of 3D Made-to-Measure Garments With Sewing Patterns.*
Computer Vision – ECCV 2024. [Paper](https://arxiv.org/abs/2405.17609).

ETH's endpoint returned a rate-limit page during collection. The mirror was
available without credentials. It supplies shifted JSON specifications, not the
ETH tar batches. The archive is hash-pinned, but exact equivalence to the v2
originals is not independently established. The sample is from one reference-body
batch and is not representative of the complete 115,000-design dataset.

GarmentCode's [MIT source](https://github.com/maria-korosteleva/GarmentCode), revision
`d449629979028123a5c4dc9e732a2ec19b7fce31`, was read to check curve serialization:
`pygarment/garmentcode/edge.py`, `pygarment/pattern/core.py`, and
`pygarment/pattern/utils.py`. No source code was copied or executed. In particular,
circle radius is absolute in source units, while Bezier controls are relative.

## Additional DXF-AAMA / ASTM search

The Open+Pattern file above is a qualifying real garment source and was downloaded.
It is a designer's CC BY file, not a software vendor's sample. Other leads:

| Lead | Licence / access finding | Decision |
|---|---|---|
| [Atacac TrailBlazer trousers](https://shop.atacac.com/collections/sharewear/products/trailblazer-trousers-pattern) | Pattern page explicitly uses CC BY-SA and offers DXF among its formats. | Excluded: share-alike is outside this task's allowed list. No download. |
| [patternForge](https://github.com/danzeeeman/patternForge) | BSD-3-Clause software. Describes AAMA/ASTM export and warns its blocks are simplified. | Repository tree had no `.dxf` files. Did not build or run it. Its bundled offset code has a different licence, so it was not adopted as a tool. |
| [ksons/astm-parser](https://github.com/ksons/astm-parser) | Many real-looking DXF test files, but no explicit repository licence was found in the tree checked. No clear data-specific reuse grant. | Not used or downloaded. A fixture in an open GitHub repository does not establish redistribution rights. |
| [FreeSewing DXF exporter](https://www.npmjs.com/package/@freesewing/plugin-export-dxf) | MIT, legacy v2 plugin. Describes outline-only ASTM export. | Useful future export route; not an additional independently sourced pattern set and not used here. |
| [Pattern-making skill project](https://github.com/adriendelmas/pattern-making-skill) | MIT proportional block generator with generic DXF layers. | No pre-existing AAMA/ASTM garment file established. Did not run its code. |
| [TWOTA zero-waste pattern](https://decode.design/twota-open-source-zero-waste-pattern) | Describes DXF through CLO projects, but no explicit qualifying reuse licence was established on the checked page. | Not used. |
| [Wikimedia sewing-pattern category](https://commons.wikimedia.org/wiki/Category:Sewing_patterns) | Many scans and photographs, with per-file terms. | No usable full-scale DXF-AAMA/ASTM found in this search. Nothing downloaded or digitized. |
| [Kaggle sewing-data discussion](https://www.kaggle.com/general/512512) and targeted dataset search | Requests for sewing datasets and unrelated image data. | No licensed real AAMA/ASTM file established. Nothing downloaded. |
| [MIT course garment work](https://fab.cba.mit.edu/classes/863.25/people/RodrigoGallardo/) | Describes a DXF welding pattern for an inflatable sleeve. A page on an MIT domain does not by itself give an MIT reuse licence. | No qualifying AAMA/ASTM file plus explicit licence established. Not used. |

Seamly2D and Valentina are not used. No GPL/LGPL/AGPL code, vendor specifications,
vendor sample files, paid patterns, personal-use-only data or non-commercial data
were brought into the generators or fixtures.

## Published commercial marker results

**No qualifying geometry/result pair was found in this search.** The reports
below publish numbers, but do not provide openly licensed, recoverable vector
pattern geometry. Their percentages must not be used as targets for our different
FreeSewing or GarmentCodeData pieces. Screenshots and garment measurements do not
define the original cut polygons closely enough for a direct benchmark.

### Pudjiati et al., Indonesian Textile Conference 2022

[Marker Efficiency and Improvement via Computer Aided Design (CAD) System in the
Garment Industry](https://pubs.itc.stttekstil.ac.id/index.php/itc/article/download/58/27/317),
pp. 143–150. Gerber AccuMark, chemical laboratory coats, sizes M/L/XL.

| Method | Efficiency | Marker length |
|---|---:|---:|
| Automatic, alternating bundle direction | 79.00% | 8.9543 m |
| Automatic, bundles in same direction | 78.03% | 9.0663 m |
| Automatic, same size and direction | 78.81% | 8.9766 m |
| Manual CAD arrangement, alternating bundles | 84.68% | 8.2633 m |

The fabric is described as unpatterned. The text does not give a dependable
numeric fabric width or full size ratio; these are not inferred from screenshots.
Spacing, exact grain tolerances and runtime are not specified. Geometry is shown
only in marker images; no DXF or licensed panel dataset is linked. This is a
published commercial-CAD result, but not a reproducible open-geometry benchmark.

### Habib, Mia and Mia, JTEFT 2026

[The impact of fabric shrinkage percentage on marker efficiency for long-sleeve
women's shirts](https://medcraveonline.com/JTEFT/JTEFT-12-00464.pdf),
12(3), 181–189, DOI `10.15406/jteft.2026.12.00464`.

| System | Without shrinkage | With shrinkage |
|---|---:|---:|
| Gerber AccuMark v12 | 88.75% | 89.00% |
| Lectra v7 / Diamino | 88.36% | 89.03% |
| Gemini | 89.29% | 89.83% |
| CLO 3D 2025.2.368 | 87.07% | 88.48% |

Fabric width: **58 in = 1473.2 mm**. Nineteen shirt components; shrinkage setup:
2% lengthwise and 1% widthwise. Pattern development was in Gerber, with DXF
transfer to other systems. The paper gives garment measurements and screenshots,
but no vector files or data download. Length, size ratio, gaps, nap and grain
tolerance are not established as reproducible constraints. Prose and figure
captions disagree about which layouts include shrinkage; values above follow the
prose/results. The article's licence footer also mixes attribution with a
non-commercial statement, so no geometry or figures were copied. It is a lead,
not a benchmark we can reproduce under this task's licence rules.
