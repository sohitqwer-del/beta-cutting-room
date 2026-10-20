# Open+Pattern windbreaker DXF-AAMA

Source: [US Physical Training Reflective Windbreaker, Open+Pattern](https://openpattern.io/product/us-pt-windbreaker-jacket/).
Author and attribution: **Open+Pattern**, [openpattern.io](https://openpattern.io/).
Licence: [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
The product page explicitly allows use, changes and commercial use with attribution.
No endorsement is implied. This is an independent designer's pattern, not a CAD vendor sample.

`openpattern-pt-windbreaker-M.dxf` is a size-M subset: **35 BLOCKs and 35 INSERTs**,
**509,410 bytes**. It retains the source geometry, layers, piece text and global
metadata. Other graded blocks and their INSERTs were removed; line endings were
normalized to LF. The source calls its largest size `XLL`, although the product
page says XS–XXL. The fixture keeps only `_M` blocks.

The download was discovered through the site's public media metadata:
[media 2447](https://openpattern.io/wp-json/wp/v2/media/2447).
No checkout, account, payment or personal information was submitted.

- [Original ZIP](https://openpattern.io/wp-content/uploads/2026/10/OpenPattern_US_PT_Reflective_Windbreaker_DXF.zip): 539,921 bytes.
- Original DXF: 3,597,417 bytes, 210 blocks across six sizes. Kept outside git in
  `BETA_DATA_DIR/datasets/open-dxf/`.
- ZIP SHA-256: `dbcaa94a4b2ac5f7e49e25d6300fa8f6fc8f5fab3feeb644a85f96aaa5336c66`.
- Original DXF SHA-256: `21d88d4f1ca76ea0d65329217bce8a6672f9bf3d3dbf2c3951d053935e69f444`.
- Committed M subset SHA-256: `4b2182c1c76fcfed069ccd79628d1c402bab23229b34cd5b5fe495e9afc617ae`.

Regenerate after downloading and unpacking the original ZIP on the data disk:

```powershell
node marker/tools/open-patterns/extract-dxf-size.mjs "$env:BETA_DATA_DIR/datasets/open-dxf/openpattern-windbreaker/DFX_2023_13_01_OPENPATTERN_PT_WINDBREAKER_JACKET/2023_13_01_OPENPATTERN_PT_WINDBREAKER_JACKET.dxf" marker/testdata/open/dxf/openpattern-pt-windbreaker-M.dxf M
```

The source DXF says `UNITS: METRIC` but has no `$INSUNITS`; unit interpretation
belongs to the importer. The file includes lining and reflective panels as well
as shell pieces. Do not treat all 35 blocks as one main-fabric marker. It also has
exporter text naming CLO software. That text identifies the writing application,
not the pattern's licensor. Import and nesting are not validated by this data task.
