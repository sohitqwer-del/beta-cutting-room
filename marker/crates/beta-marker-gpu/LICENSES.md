# Dependency licence audit

The Windows Vulkan/DX12 dependency graph was checked with Cargo metadata.
The table lists packages absent from the parent workspace resolution.
Versions are evidence for this experiment, not a committed lockfile.
All listed expressions allow MIT, Apache-2.0, ISC, BSD, Zlib or Unicode terms.

| Package | Version | Declared licence |
|---|---|---|
| allocator-api2 | 0.2.21 | MIT OR Apache-2.0 |
| arrayvec | 0.7.8 | MIT OR Apache-2.0 |
| ash | 0.38.0+1.3.281 | MIT OR Apache-2.0 |
| bit-set | 0.10.0 | Apache-2.0 OR MIT |
| bit-vec | 0.9.1 | Apache-2.0 OR MIT |
| bytemuck_derive | 1.12.1 | Zlib OR Apache-2.0 OR MIT |
| cfg_aliases | 0.2.2 | MIT |
| codespan-reporting | 0.13.1 | Apache-2.0 |
| foldhash | 0.2.0 | Zlib |
| gpu-allocator | 0.28.0 | MIT OR Apache-2.0 |
| half | 2.7.1 | MIT OR Apache-2.0 |
| hashbrown | 0.16.1 | MIT OR Apache-2.0 |
| libloading | 0.8.9 | ISC |
| lock_api | 0.4.14 | MIT OR Apache-2.0 |
| naga | 30.0.1 | MIT OR Apache-2.0 |
| naga-types | 30.0.1 | MIT OR Apache-2.0 |
| parking_lot | 0.12.5 | MIT OR Apache-2.0 |
| parking_lot_core | 0.9.12 | MIT OR Apache-2.0 |
| pollster | 0.4.0 | Apache-2.0/MIT |
| presser | 0.3.1 | MIT OR Apache-2.0 |
| profiling | 1.0.18 | MIT OR Apache-2.0 |
| range-alloc | 0.1.5 | MIT OR Apache-2.0 |
| raw-window-handle | 0.6.2 | MIT OR Apache-2.0 OR Zlib |
| renderdoc-sys | 1.1.0 | MIT OR Apache-2.0 |
| rustc-hash | 1.1.0 | Apache-2.0/MIT |
| scopeguard | 1.2.0 | MIT OR Apache-2.0 |
| spirv | 0.4.0+sdk-1.4.341.0 | Apache-2.0 |
| static_assertions | 1.1.0 | MIT OR Apache-2.0 |
| unicode-width | 0.2.2 | MIT OR Apache-2.0 |
| wgpu | 30.0.1 | MIT OR Apache-2.0 |
| wgpu-core | 30.0.1 | MIT OR Apache-2.0 |
| wgpu-core-deps-windows-linux-android | 30.0.1 | MIT OR Apache-2.0 |
| wgpu-hal | 30.0.1 | MIT OR Apache-2.0 |
| wgpu-naga-bridge | 30.0.1 | MIT OR Apache-2.0 |
| wgpu-types | 30.0.1 | MIT OR Apache-2.0 |
| windows | 0.62.2 | MIT OR Apache-2.0 |
| windows-collections | 0.3.2 | MIT OR Apache-2.0 |
| windows-core | 0.62.2 | MIT OR Apache-2.0 |
| windows-future | 0.3.2 | MIT OR Apache-2.0 |
| windows-implement | 0.60.2 | MIT OR Apache-2.0 |
| windows-interface | 0.59.3 | MIT OR Apache-2.0 |
| windows-numerics | 0.3.1 | MIT OR Apache-2.0 |
| windows-result | 0.4.1 | MIT OR Apache-2.0 |
| windows-strings | 0.5.1 | MIT OR Apache-2.0 |
| windows-threading | 0.2.1 | MIT OR Apache-2.0 |
| zerocopy | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT |
| zerocopy-derive | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT |

Direct dependencies: `wgpu` (MIT OR Apache-2.0), `bytemuck` (Zlib OR Apache-2.0 OR MIT),
`pollster` (Apache-2.0/MIT), `geo`, `anyhow`, `serde`, `serde_json` (MIT OR Apache-2.0),
and the existing Apache-2.0 core/nest crates.

| Direct package | Resolved version | Declared licence |
|---|---|---|
| anyhow | 1.0.104 | MIT OR Apache-2.0 |
| beta-marker-core, beta-marker-nest | 0.1.0 | Apache-2.0 |
| bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT |
| geo | 0.33.1 | MIT OR Apache-2.0 |
| pollster | 0.4.0 | Apache-2.0/MIT |
| serde | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.154 | MIT OR Apache-2.0 |
| wgpu | 30.0.1 | MIT OR Apache-2.0 |

Existing packages can resolve newer patch versions in this standalone crate; their
metadata was checked too. `unicode-ident` also requires Unicode-3.0.
The unmodified Sparrow dependency has an MIT LICENSE file (its Cargo licence
field is empty). Jagua-rs remains the existing, unmodified MPL-2.0 exception.
No engine source is vendored or patched. No runtime network service is used.

Re-run the audit when the lead regenerates the workspace lockfile.
Cross-target dependencies need a fresh audit before shipping on another platform.

## Additional packages in the full target resolution

These are outside the measured Windows graph. Their declared licence terms
were checked too. Some belong to existing engine target support.
For `r-efi`, select MIT or Apache-2.0 from its OR expression; LGPL terms are
not selected. For `Unlicense OR MIT` packages, select MIT.

| Package | Version | Declared licence |
|---|---|---|
| android_system_properties | 0.1.6 | MIT OR Apache-2.0 |
| block2 | 0.6.2 | MIT |
| bumpalo | 3.20.3 | MIT OR Apache-2.0 |
| crunchy | 0.2.4 | MIT |
| dispatch2 | 0.3.1 | Zlib OR Apache-2.0 OR MIT |
| futures-core | 0.3.34 | MIT OR Apache-2.0 |
| futures-task | 0.3.34 | MIT OR Apache-2.0 |
| futures-util | 0.3.34 | MIT OR Apache-2.0 |
| hermit-abi | 0.5.3 | MIT OR Apache-2.0 |
| js-sys | 0.3.106 | MIT OR Apache-2.0 |
| libc | 0.2.191 | MIT OR Apache-2.0 |
| nix | 0.31.3 | MIT |
| objc2 | 0.6.5 | MIT |
| objc2-core-foundation | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| objc2-encode | 4.1.0 | MIT |
| objc2-foundation | 0.3.2 | MIT |
| objc2-metal | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| objc2-quartz-core | 0.3.2 | Zlib OR Apache-2.0 OR MIT |
| pin-project-lite | 0.2.17 | Apache-2.0 OR MIT |
| portable-atomic | 1.15.0 | Apache-2.0 OR MIT |
| portable-atomic-util | 0.2.8 | Apache-2.0 OR MIT |
| raw-window-metal | 1.1.0 | MIT OR Apache-2.0 |
| redox_syscall | 0.5.18 | MIT |
| r-efi | 6.0.0 | MIT OR Apache-2.0 OR LGPL-2.1-or-later |
| rustversion | 1.0.23 | MIT OR Apache-2.0 |
| slab | 0.4.12 | MIT |
| tokio | 1.53.3 | MIT |
| wasm-bindgen | 0.2.129 | MIT OR Apache-2.0 |
| wasm-bindgen-futures | 0.4.79 | MIT OR Apache-2.0 |
| wasm-bindgen-macro | 0.2.129 | MIT OR Apache-2.0 |
| wasm-bindgen-macro-support | 0.2.129 | MIT OR Apache-2.0 |
| wasm-bindgen-shared | 0.2.129 | MIT OR Apache-2.0 |
| web-sys | 0.3.106 | MIT OR Apache-2.0 |
