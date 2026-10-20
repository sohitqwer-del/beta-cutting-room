# Original Seamly interoperability samples

These patterns and measurement values were created by us on 2026-10-19 for this test. They were not copied from Seamly2D examples, BETA examples, a drafting textbook, a commercial pattern, or another pattern library. The geometry is a simple original test design. The garments have not been fit tested and must not be treated as production blocks.

The data files and `make_samples.py` are offered under Apache License 2.0. They may become BETA test data under that license. The user may instead release their original data under CC BY 4.0. This notice does not change the license of Seamly2D, Qt, DXF libraries, or any third-party code. No Seamly2D code may be copied into BETA, and no BETA code has been copied into Seamly2D.

- `skirt-block.sm2d`: front and back, curved hip, waist-dart internal paths, slit notches, 10 mm seam allowance.
- `shirt-front-back.sm2d`: front and back, curved neckline and armholes, internal diamond waist darts, slit notches, 10 mm seam allowance.
- `trouser-front.sm2d`: front, curved crotch/centre front, waist dart, slit notch, 10 mm seam allowance.
- `original-multisize.smms`: original synthetic base measurements at size 50 / height 176 cm; test sizes 48, 50, 52.
- `original-size-48.smis`, `original-size-50.smis`, `original-size-52.smis`: original per-size derived values for probing an importer limited to individual tables.
- `expected.json`: intended piece metadata. Quantity 2 means one normal and one mirrored piece for this test. Seamly's native permission to flip a piece is not a separate mirrored-copy count.

All three patterns use centimetres. Physical dimensions in reports and BETA jobs are millimetres. Both native XML and the multisize table have been checked against the exact develop schemas. Application loading and export are separate checks; see `../INTEROP.md` for their final results.

The label template deliberately uses public, plain text fields: `Piece Name:`, `Size:`, `Quantity:`, `Material:` and a free-text mirrored-pair note. That tests a configured interoperable workflow. An additional export with hidden labels will test whether production metadata depends on label visibility.

Regenerate native input files with `python make_samples.py`. Preserve exported DXF and result logs separately when comparing branches. The scripts do not convert or rename AAMA files into ASTM files.
