# Seamly2D export regression data

All native patterns, measurement tables and exports here are original synthetic
test data, offered under Apache-2.0. See LICENSE and the unchanged
SOURCE_README.md. No GPL source, upstream examples, icons or documentation text
is included. These are not fit-tested production blocks.

## Source and export provenance

The independent Seamly interop run authored the inputs on 2026-10-19. The
original files were copied unchanged from its `samples/` directory. The small
`*.copy-rules.json` mappings were added in BETA to express the intent already in
expected.json: total quantity 2 means one normal and one mirrored copy.
`make_samples.py` regenerates native inputs only; it does not run a GUI.

The DXFs were copied unchanged from the retained `results/` exports. Each
relative source location and SHA-256 digest is recorded in exports.json.
Local Git attributes preserve original bytes, including export line endings.
Nine main R12 exports cover all three garments at sizes 48/50/52, height 176 cm.
Three shirt visibility variants, a baseline R12 and a modern-format (27) export
cover metadata, grain visibility and unit handling. Total fixture data is below
2 MB. No application was launched to create or test this copy.

The source report identifies Seamly2D develop commit
`03d85588404d73c9f4f5bdc378b4ba85ec61cd3d`, Qt 6.11.1 and MinGW GCC 13.1.0.
The saved runtime manifests report version **0.6.0.1**, revision
**77c5fb193005** for both retained builds; this embedded revision alone does not
identify the patch tree. The baseline uses the batch-data-path patch; metadata
exports also use the cutting-metadata patch (reported final commit `78f8bfa`).
The exported executable SHA-256 values in the original run manifests are:

- baseline: `aaf7f782f485cfd2c0c1455d0a8322caa8051489727a0598c22c12ddc5934135`
- metadata: `dbc9af7455bace20806b418abd8f43e23ed92b5e4a714d118b062792a08fc023`

The prior run used the following command shape, with the original relative
input/output directories substituted below. Do not rerun Seamly on a shared PC
without a separately arranged headless/manual session.

```text
seamly2d --basename shirt-front-back-50 --destination output --format 20 --exportOnlyDetails --gsize 50 --gheight 176 shirt-front-back.sm2d
```

Format 20 is AAMA R12; format 27 uses the modern DXF writer. Neither is ASTM.
Native construction units are cm; exported AAMA geometry is mm. The metadata
build supplies Units: MM for R12 independently of label visibility. Baseline
R12 lacks that field and requires an importer unit-inference warning. Hidden
labels omit fabric: a default shell assignment is an assumption, not a bill of
materials. Hidden grain must remain unknown and require operator confirmation.

## Regression contract

Tests live in beta-pattern-core/tests/seamly_exports.rs and
beta-marker-dxf/src/tests.rs. They compare actual exports with native grading;
they do not export through BETA and call that an external round trip.
expected.json is preserved as the independent metadata oracle. See the core
README for the native/DXF distance budget and the remaining grain warning.
