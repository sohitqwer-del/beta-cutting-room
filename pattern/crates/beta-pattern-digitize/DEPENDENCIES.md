# Dependency licence review

Checked against the downloaded crates' `Cargo.toml` package licence fields.
Upstream source: [image 0.25.8](https://docs.rs/crate/image/0.25.8/source/).
Only JPEG and PNG features are enabled. All decoders and computation are Rust.
The direct dependency is pinned so later image releases do not silently change
the tested decoding stack. Transitive versions below are those resolved during
this implementation; the lead owns the final workspace lockfile.

| New resolved crate | Version | Declared licence | Permitted choice |
|---|---|---|---|
| image (direct) | 0.25.8 | MIT OR Apache-2.0 | Apache-2.0 |
| adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 | Apache-2.0 |
| autocfg | 1.5.1 | Apache-2.0 OR MIT | Apache-2.0 |
| bitflags | 2.13.2 | MIT OR Apache-2.0 | Apache-2.0 |
| bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT | Apache-2.0 |
| byteorder-lite | 0.1.0 | Unlicense OR MIT | MIT |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 | Apache-2.0 |
| crc32fast | 1.5.2 | MIT OR Apache-2.0 | Apache-2.0 |
| fdeflate | 0.3.7 | MIT OR Apache-2.0 | Apache-2.0 |
| flate2 | 1.1.10 | MIT OR Apache-2.0 | Apache-2.0 |
| miniz_oxide | 0.8.9, 0.9.1 | MIT OR Zlib OR Apache-2.0 | Apache-2.0 |
| moxcms | 0.7.11 | BSD-3-Clause OR Apache-2.0 | Apache-2.0 |
| num-traits | 0.2.19 | MIT OR Apache-2.0 | Apache-2.0 |
| png | 0.18.1 | MIT OR Apache-2.0 | Apache-2.0 |
| pxfm | 0.1.30 | BSD-3-Clause OR Apache-2.0 | Apache-2.0 |
| simd-adler32 | 0.3.10 | MIT | MIT |
| zlib-rs | 0.6.8 | Zlib | Zlib |
| zune-core | 0.4.12 | MIT OR Apache-2.0 OR Zlib | Apache-2.0 |
| zune-jpeg | 0.4.21 | MIT OR Apache-2.0 OR Zlib | Apache-2.0 |

Some resolved crates can be optional/inactive for a given target. This list
includes all newly resolved packages, rather than omitting optional lock entries.
Existing workspace dependencies `anyhow`, `serde`, `serde_json`, and
`beta-marker-core` are unchanged. `imageproc` and fiducial libraries are not used.
The new dependency stack declares Rust minimum versions no higher than 1.85;
the workspace declares Rust 1.90. The actual build toolchain is recorded with
the accuracy results.


## Quick Marker additions

The scanner and assist model add no new runtime third-party dependencies.
Marker and Pattern WASM crates now depend on this existing image stack, and
Marker WASM also uses the existing export crate. Fixed Hindi/Tamil headings are
pre-shaped at development time. The export crate's dev-only generator adds:

| Crate | Version | Declared licence |
|---|---|---|
| rustybuzz | 0.20.1 | MIT |
| ttf-parser | 0.25.1 | MIT OR Apache-2.0 |
| core_maths | 0.1.1 | MIT |
| unicode-bidi-mirroring | 0.4.0 | MIT / Apache-2.0 |
| unicode-ccc | 0.4.0 | MIT / Apache-2.0 |
| unicode-properties | 0.1.4 | MIT / Apache-2.0 |
| unicode-script | 0.5.8 | MIT OR Apache-2.0 |

Checked the installed crate manifests on 20 October 2026. None of these shaping
crates is a runtime dependency of Quick Marker. Apache-2.0 Droid font sources,
checksums and original notices are under `beta-marker-export/fonts/PROVENANCE.md`.
The stored FreeSewing-derived descriptors are MIT-source derivatives; the
GarmentCodeData-derived descriptors retain CC BY 4.0 attribution. See
`model/MODEL.md`. These are public pattern fixtures, not factory data.
