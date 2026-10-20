# FreeSewing cut pieces

Source: [FreeSewing](https://freesewing.eu/) and the `@freesewing/*` npm packages,
all version **4.10.2**. Licence: **MIT**, copyright Joost De Cock and FreeSewing contributors.
See [LICENSE.MIT](LICENSE.MIT). These are generated tailoring drafts, not factory patterns.

## Files and sizes

There are five styles and 14 jobs. Each material has one `-M.job.json` quick test
and one `-ratio.job.json` marker with **S1 M2 L2 XL1**. Fabric width is 1500 mm;
nap is none; both gaps are zero. There is one SVG preview per style, with all materials.
`report.json` records the piece counts and area for every size and material.

The size labels are benchmark labels, not a retail size chart. v4 exports the
following actual named measurement sets. The numbers are neck circumference in cm.

| Label | Menswear set | Chest, mm | Bella set | Chest, mm |
|---|---|---:|---|---:|
| S | cisMaleAdult36 | 947 | cisFemaleAdult32 | 871 |
| M | cisMaleAdult38 | 1000 | cisFemaleAdult34 | 925 |
| L | cisMaleAdult40 | 1053 | cisFemaleAdult36 | 979 |
| XL | cisMaleAdult42 | 1105 | cisFemaleAdult38 | 1034 |

Counts below are physical copies per garment, including mirrored copies. Area
includes every demanded copy and the drafted seam allowance.

| Style / material | Piece types | Cut pieces | S, m² | M, m² | L, m² | XL, m² |
|---|---:|---:|---:|---:|---:|---:|
| Simon shirt / fabric | 10 | 19 | 1.474948 | 1.577737 | 1.683976 | 1.794651 |
| Simon / interfacing | 3 | 6 | 0.125753 | 0.135097 | 0.144793 | 0.154599 |
| Teagan T-shirt / fabric | 3 | 4 | 0.794844 | 0.861435 | 0.930965 | 1.004111 |
| Titan trouser block / fabric | 2 | 4 | 1.271845 | 1.337404 | 1.395212 | 1.467851 |
| Hugo hoodie / fabric | 8 | 14 | 1.249736 | 1.336834 | 1.426560 | 1.520840 |
| Hugo / ribbing | 2 | 3 | 0.192414 | 0.207300 | 0.222712 | 0.238484 |
| Bella bodice block / fabric | 3 | 5 | 0.646370 | 0.697626 | 0.751010 | 0.807339 |

## Conversion choices

- `sa: 10` enables cut lines. FreeSewing's larger hems and flat-felled allowances
  are retained. The exporter joins seam and SA linework and traces its outer face
  when the cut line consists of several paths. It does not offset the seam again.
- Curves are flattened with a 0.15 mm error bound, below the requested 0.2 mm.
  Small offset self-crossings in the source sleeve are resolved by nonzero union.
  Output is checked again for self-intersections.
- Fold pieces are reflected about the actual fold macro line and joined into a
  full piece. Marks are reflected too. An inherited fold record without a fold
  instruction is ignored; the full Simon yoke must not be unfolded again.
- Two non-identical cuts become `quantity: 1, mirror_quantity: 1`. Identical or
  unfolded copies keep their full normal quantity. Separate cut instructions are
  counted separately. Materials are separate jobs, including Hugo ribbing.
- Grain comes from macro path endpoints or the cutlist's grain angle. Simon has
  no grain macro for its cuff and two sleeve plackets. Their drafted x axis is
  used as the lengthwise grain. Each job records this assumption.
- Notch snippet anchors and button/buttonhole centres are retained as marks.
  These anchors are source sewing marks; some lie on the stitch line inside the SA.
  Buttons and buttonholes are drill centres, not their full shapes. Internal dart
  lines are not exported. Part names are text marks. SVG y-down coordinates are
  reflected into the contract's y-up coordinates with all marks and grain.
- The collar and undercollar share FreeSewing's full collar cutting envelope.
  The source's optional smaller undercollar trim line is not applied separately.
- No drawn rectangle is omitted. Simon's default plackets are cut on the fronts,
  so its two empty separate-placket parts are not emitted. Titan and Bella are
  blocks; Titan does not include the waistband, fly or pockets of finished chinos.

Two v4.10.2 cutlist errors are corrected using the official design instructions:
[Hugo](https://freesewing.eu/docs/designs/hugo/) says one pocket on fold, although
its package says two and draws a half pocket. This output contains one unfolded
pocket. [Bella](https://freesewing.eu/docs/designs/bella/) says two back parts.
Its curved centre back has no fold macro, despite `onFold: true` in the package.
This output contains a mirrored pair of back parts. Both changes are in job metadata.

## Shirt area check

The M shirt has 1.577737 m² of main fabric pieces. At an assumed 85% nesting
efficiency on 1.5 m fabric, that would require **1.237 m** of marker length.
This is an estimate, not a nesting result. The suggested 1.6–1.8 m consumption
at 85% corresponds to 2.04–2.295 m² of pieces, so this fitted default draft is
below that rough reference (even XL is 1.795 m²). The piece area is physically
plausible, and the source's complete default cutlist is present, but it does not
confirm that typical consumption figure. Do not use it as a production allowance
or as evidence of 85% efficiency. The source itself suggests about two metres of
fabric, which also includes layout and purchasing allowances.

## Regenerate

From the repository root, after setting `BETA_DATA_DIR`:

```powershell
npm install --prefix marker/tools/open-patterns/freesewing --package-lock=false --ignore-scripts --cache "$env:BETA_DATA_DIR/npm-cache"
node marker/tools/open-patterns/freesewing/generate.mjs
```

The package is isolated from the root npm workspace. `@freesewing/config` and
`@freesewing/plugin-transform` are explicit dependencies because the upstream
packages import them without declaring them. Node.js 22 or later is required.
