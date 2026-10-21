# Third-party licences: BETA Cutting Room 0.6.0

Generated from Cargo metadata, local npm package manifests and installed package.json files.
No network licence service is used. Re-run after lockfile or dependency changes.
Bundled Droid Sans fonts use Apache-2.0; the site Bricolage Grotesque font uses OFL-1.1.

## Scope and decisions

The main inventory covers the Windows GNU/MSVC Rust workspace graphs, Linux CLI,
Pattern workspace, all three WASM modules, Marker npm workspaces and their installed build
dependencies. It also includes optional GPU and open-pattern tooling when requested.
The desktop CLI tool is included when BETA_TOOLS_DIR is set. OS libraries and the
WebView2 Runtime are separately installed prerequisites. The bundled SDK loader
is audited separately below. NSIS's selected compressor is zlib.

npm traversal follows Marker package dependency edges plus the root's shared
development tools. Unrelated BETA Time workspace dependencies are not included.

SPDX expressions are preserved. The gate requires an allowed MIT, Apache-2.0,
BSD, ISC, Zlib or Unicode alternative. Unlicense/CC0/Boost may appear in dual
declarations; an allowed branch is chosen rather than relabelling the package.
jagua-rs 1.0.0 is the sole accepted MPL-2.0 exception and is used unmodified.
sparrow 0.3.0 omits a manifest licence; its local upstream LICENSE is MIT and was read.
Original Apache-2.0 dom_query 0.28.0 and option-ext 0.2.0 adapters retain upstream
version constraints. They are not copies of upstream MPL helper implementations.

Inventoried packages: **522**. Review-required selected packages: **0**.

| Ecosystem | Package | Version | Declared licence | Decision |
|---|---|---|---|---|
| Font | Bricolage Grotesque | site/assets/fonts/bricolage-grotesque-latin-wght-normal.woff2 | OFL-1.1 | permissive |
| Font | Droid Sans Devanagari | export/fonts/DroidSansDevanagari-Regular.ttf | Apache-2.0 | permissive |
| Font | Droid Sans Tamil | export/fonts/DroidSansTamil-Regular.ttf | Apache-2.0 | permissive |
| Rust | adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 | permissive |
| Rust | aho-corasick | 1.1.5 | Unlicense OR MIT | permissive |
| Rust | alloc-no-stdlib | 3.0.0 | BSD-3-Clause | permissive |
| Rust | alloc-stdlib | 0.3.0 | BSD-3-Clause | permissive |
| Rust | allocator-api2 | 0.2.21 | MIT OR Apache-2.0 | permissive |
| Rust | anstream | 1.0.0 | MIT OR Apache-2.0 | permissive |
| Rust | anstyle | 1.0.14 | MIT OR Apache-2.0 | permissive |
| Rust | anstyle-parse | 1.0.0 | MIT OR Apache-2.0 | permissive |
| Rust | anstyle-query | 1.1.5 | MIT OR Apache-2.0 | permissive |
| Rust | anstyle-wincon | 3.0.11 | MIT OR Apache-2.0 | permissive |
| Rust | anyhow | 1.0.104 | MIT OR Apache-2.0 | permissive |
| Rust | approx | 0.5.1 | Apache-2.0 | permissive |
| Rust | arrayvec | 0.7.8 | MIT OR Apache-2.0 | permissive |
| Rust | ash | 0.38.0+1.3.281 | MIT OR Apache-2.0 | permissive |
| Rust | atoi_simd | 0.16.1 | MIT OR Apache-2.0 | permissive |
| Rust | atomic-waker | 1.1.2 | Apache-2.0 OR MIT | permissive |
| Rust | autocfg | 1.5.1 | Apache-2.0 OR MIT | permissive |
| Rust | base64 | 0.22.1 | MIT OR Apache-2.0 | permissive |
| Rust | base64 | 0.23.1 | MIT OR Apache-2.0 | permissive |
| Rust | beta-cutting-room | 0.6.0 | Apache-2.0 | permissive |
| Rust | beta-marker-cli | 0.6.0 | Apache-2.0 | permissive |
| Rust | beta-marker-cluster | 0.6.0 | Apache-2.0 | permissive |
| Rust | beta-marker-core | 0.6.0 | Apache-2.0 | permissive |
| Rust | beta-marker-cutroom | 0.6.0 | Apache-2.0 | permissive |
| Rust | beta-marker-dxf | 0.6.0 | Apache-2.0 | permissive |
| Rust | beta-marker-edit | 0.6.0 | Apache-2.0 | permissive |
| Rust | beta-marker-export | 0.6.0 | Apache-2.0 | permissive |
| Rust | beta-marker-floor | 0.6.0 | Apache-2.0 | permissive |
| Rust | beta-marker-gpu | 0.6.0 | Apache-2.0 | permissive |
| Rust | beta-marker-learn | 0.6.0 | Apache-2.0 | permissive |
| Rust | beta-marker-match | 0.6.0 | Apache-2.0 | permissive |
| Rust | beta-marker-nest | 0.6.0 | Apache-2.0 | permissive |
| Rust | beta-marker-plan | 0.6.0 | Apache-2.0 | permissive |
| Rust | beta-marker-queue | 0.6.0 | Apache-2.0 | permissive |
| Rust | beta-marker-roll | 0.6.0 | Apache-2.0 | permissive |
| Rust | beta-marker-wasm | 0.6.0 | Apache-2.0 | permissive |
| Rust | beta-open-patterns-check | 0.6.0 | Apache-2.0 | permissive |
| Rust | beta-pattern-core | 0.6.0 | Apache-2.0 | permissive |
| Rust | beta-pattern-digitize | 0.6.0 | Apache-2.0 | permissive |
| Rust | beta-pattern-wasm | 0.6.0 | Apache-2.0 | permissive |
| Rust | beta-public-bench | 0.6.0 | Apache-2.0 | permissive |
| Rust | bit-set | 0.10.0 | Apache-2.0 OR MIT | permissive |
| Rust | bit-vec | 0.9.1 | Apache-2.0 OR MIT | permissive |
| Rust | bitflags | 1.3.2 | MIT/Apache-2.0 | permissive |
| Rust | bitflags | 2.13.2 | MIT OR Apache-2.0 | permissive |
| Rust | block-buffer | 0.10.4 | MIT OR Apache-2.0 | permissive |
| Rust | brotli | 9.0.0 | BSD-3-Clause AND MIT | permissive |
| Rust | brotli-decompressor | 6.0.1 | BSD-3-Clause/MIT | permissive |
| Rust | bs58 | 0.5.1 | MIT/Apache-2.0 | permissive |
| Rust | bumpalo | 3.20.3 | MIT OR Apache-2.0 | permissive |
| Rust | bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT | permissive |
| Rust | bytemuck_derive | 1.12.1 | Zlib OR Apache-2.0 OR MIT | permissive |
| Rust | byteorder | 1.5.0 | Unlicense OR MIT | permissive |
| Rust | byteorder-lite | 0.1.0 | Unlicense OR MIT | permissive |
| Rust | bytes | 1.12.1 | MIT | permissive |
| Rust | calamine | 0.31.0 | MIT | permissive |
| Rust | camino | 1.2.6 | MIT OR Apache-2.0 | permissive |
| Rust | cargo-platform | 0.1.9 | MIT OR Apache-2.0 | permissive |
| Rust | cargo_metadata | 0.19.2 | MIT | permissive |
| Rust | cargo_toml | 1.0.1 | Apache-2.0 OR MIT | permissive |
| Rust | cc | 1.9.0 | MIT OR Apache-2.0 | permissive |
| Rust | cfb | 0.14.0 | MIT | permissive |
| Rust | cfg-if | 1.0.5 | MIT OR Apache-2.0 | permissive |
| Rust | cfg_aliases | 0.2.2 | MIT | permissive |
| Rust | chacha20 | 0.10.2 | MIT OR Apache-2.0 | permissive |
| Rust | chrono | 0.4.45 | MIT OR Apache-2.0 | permissive |
| Rust | clap | 4.6.7 | MIT OR Apache-2.0 | permissive |
| Rust | clap_builder | 4.6.7 | MIT OR Apache-2.0 | permissive |
| Rust | clap_derive | 4.6.7 | MIT OR Apache-2.0 | permissive |
| Rust | clap_lex | 1.1.1 | MIT OR Apache-2.0 | permissive |
| Rust | codepage | 0.1.3 | Apache-2.0 OR MIT | permissive |
| Rust | codespan-reporting | 0.13.1 | Apache-2.0 | permissive |
| Rust | colorchoice | 1.0.5 | MIT OR Apache-2.0 | permissive |
| Rust | cookie | 0.18.2 | MIT OR Apache-2.0 | permissive |
| Rust | core_detect | 1.0.0 | MIT/Apache-2.0 | permissive |
| Rust | core_maths | 0.1.1 | MIT | permissive |
| Rust | cpufeatures | 0.2.17 | MIT OR Apache-2.0 | permissive |
| Rust | cpufeatures | 0.3.1 | MIT OR Apache-2.0 | permissive |
| Rust | crc32fast | 1.5.2 | MIT OR Apache-2.0 | permissive |
| Rust | crossbeam-channel | 0.5.17 | MIT OR Apache-2.0 | permissive |
| Rust | crossbeam-deque | 0.8.8 | MIT OR Apache-2.0 | permissive |
| Rust | crossbeam-epoch | 0.9.21 | MIT OR Apache-2.0 | permissive |
| Rust | crossbeam-utils | 0.8.23 | MIT OR Apache-2.0 | permissive |
| Rust | crypto-common | 0.1.7 | MIT OR Apache-2.0 | permissive |
| Rust | csv | 1.4.0 | Unlicense/MIT | permissive |
| Rust | csv-core | 0.1.13 | Unlicense/MIT | permissive |
| Rust | ctor | 1.0.13 | Apache-2.0 OR MIT | permissive |
| Rust | ctrlc | 3.5.2 | MIT/Apache-2.0 | permissive |
| Rust | darling | 0.24.1 | MIT | permissive |
| Rust | darling_core | 0.24.1 | MIT | permissive |
| Rust | darling_macro | 0.24.1 | MIT | permissive |
| Rust | debug_unsafe | 0.1.4 | MIT OR Apache-2.0 | permissive |
| Rust | defmt | 1.2.0 | MIT OR Apache-2.0 | permissive |
| Rust | defmt-macros | 1.2.0 | MIT OR Apache-2.0 | permissive |
| Rust | defmt-parser | 1.1.0 | MIT OR Apache-2.0 | permissive |
| Rust | deranged | 0.5.9 | MIT OR Apache-2.0 | permissive |
| Rust | digest | 0.10.7 | MIT OR Apache-2.0 | permissive |
| Rust | dirs | 7.0.0 | MIT OR Apache-2.0 | permissive |
| Rust | dirs-sys | 0.5.0 | MIT OR Apache-2.0 | permissive |
| Rust | displaydoc | 0.2.7 | MIT OR Apache-2.0 | permissive |
| Rust | document-features | 0.2.12 | MIT OR Apache-2.0 | permissive |
| Rust | dom_query | 0.28.0 | Apache-2.0 | permissive |
| Rust | dpi | 0.1.2 | Apache-2.0 AND MIT | permissive |
| Rust | dtoa | 1.0.11 | MIT OR Apache-2.0 | permissive |
| Rust | dunce | 1.0.5 | CC0-1.0 OR MIT-0 OR Apache-2.0 | permissive |
| Rust | dyn-clone | 1.0.20 | MIT OR Apache-2.0 | permissive |
| Rust | either | 1.19.0 | MIT OR Apache-2.0 | permissive |
| Rust | embed-resource | 3.0.12 | MIT | permissive |
| Rust | encoding_rs | 0.8.42 | (Apache-2.0 OR MIT) AND BSD-3-Clause | permissive |
| Rust | equivalent | 1.0.3 | Apache-2.0 OR MIT | permissive |
| Rust | erased-serde | 0.4.10 | MIT OR Apache-2.0 | permissive |
| Rust | fast-float2 | 0.2.4 | MIT OR Apache-2.0 | permissive |
| Rust | fastrand | 2.5.0 | Apache-2.0 OR MIT | permissive |
| Rust | fdeflate | 0.3.7 | MIT OR Apache-2.0 | permissive |
| Rust | fern | 0.7.1 | MIT | permissive |
| Rust | find-msvc-tools | 0.1.15 | MIT OR Apache-2.0 | permissive |
| Rust | flate2 | 1.1.10 | MIT OR Apache-2.0 | permissive |
| Rust | float-cmp | 0.10.0 | MIT | permissive |
| Rust | float_next_after | 2.0.0 | MIT | permissive |
| Rust | fnv | 1.0.7 | Apache-2.0 / MIT | permissive |
| Rust | foldhash | 0.2.0 | Zlib | permissive |
| Rust | form_urlencoded | 1.2.2 | MIT OR Apache-2.0 | permissive |
| Rust | futures-channel | 0.3.34 | MIT OR Apache-2.0 | permissive |
| Rust | futures-core | 0.3.34 | MIT OR Apache-2.0 | permissive |
| Rust | futures-io | 0.3.34 | MIT OR Apache-2.0 | permissive |
| Rust | futures-macro | 0.3.34 | MIT OR Apache-2.0 | permissive |
| Rust | futures-sink | 0.3.34 | MIT OR Apache-2.0 | permissive |
| Rust | futures-task | 0.3.34 | MIT OR Apache-2.0 | permissive |
| Rust | futures-util | 0.3.34 | MIT OR Apache-2.0 | permissive |
| Rust | generic-array | 0.14.7 | MIT | permissive |
| Rust | geo | 0.33.1 | MIT OR Apache-2.0 | permissive |
| Rust | geo-types | 0.7.20 | MIT OR Apache-2.0 | permissive |
| Rust | geographiclib-rs | 0.2.7 | MIT | permissive |
| Rust | getrandom | 0.3.4 | MIT OR Apache-2.0 | permissive |
| Rust | getrandom | 0.4.3 | MIT OR Apache-2.0 | permissive |
| Rust | glob | 0.3.4 | MIT OR Apache-2.0 | permissive |
| Rust | gpu-allocator | 0.28.0 | MIT OR Apache-2.0 | permissive |
| Rust | half | 2.7.1 | MIT OR Apache-2.0 | permissive |
| Rust | hash32 | 0.3.1 | MIT OR Apache-2.0 | permissive |
| Rust | hashbrown | 0.12.3 | MIT OR Apache-2.0 | permissive |
| Rust | hashbrown | 0.16.1 | MIT OR Apache-2.0 | permissive |
| Rust | hashbrown | 0.17.1 | MIT OR Apache-2.0 | permissive |
| Rust | heapless | 0.8.0 | MIT OR Apache-2.0 | permissive |
| Rust | heck | 0.5.0 | MIT OR Apache-2.0 | permissive |
| Rust | hex | 0.4.3 | MIT OR Apache-2.0 | permissive |
| Rust | html5ever | 0.39.0 | MIT OR Apache-2.0 | permissive |
| Rust | http | 1.5.0 | MIT OR Apache-2.0 | permissive |
| Rust | http-body | 1.1.0 | MIT | permissive |
| Rust | http-body-util | 0.1.5 | MIT | permissive |
| Rust | httparse | 1.10.1 | MIT OR Apache-2.0 | permissive |
| Rust | httpdate | 1.0.3 | MIT OR Apache-2.0 | permissive |
| Rust | hyper | 1.12.0 | MIT | permissive |
| Rust | hyper-util | 0.1.21 | MIT | permissive |
| Rust | i_float | 1.16.0 | MIT | permissive |
| Rust | i_key_sort | 0.10.3 | MIT | permissive |
| Rust | i_overlay | 4.5.2 | MIT OR Apache-2.0 | permissive |
| Rust | i_shape | 1.18.0 | MIT | permissive |
| Rust | i_tree | 0.18.0 | MIT | permissive |
| Rust | ico | 0.5.0 | MIT | permissive |
| Rust | icu_collections | 2.3.0 | Unicode-3.0 | permissive |
| Rust | icu_locale_core | 2.3.0 | Unicode-3.0 | permissive |
| Rust | icu_normalizer | 2.3.0 | Unicode-3.0 | permissive |
| Rust | icu_normalizer_data | 2.3.0 | Unicode-3.0 | permissive |
| Rust | icu_properties | 2.3.0 | Unicode-3.0 | permissive |
| Rust | icu_properties_data | 2.3.0 | Unicode-3.0 | permissive |
| Rust | icu_provider | 2.3.1 | Unicode-3.0 | permissive |
| Rust | ident_case | 1.0.1 | MIT/Apache-2.0 | permissive |
| Rust | idna | 1.1.0 | MIT OR Apache-2.0 | permissive |
| Rust | idna_adapter | 1.2.2 | Apache-2.0 OR MIT | permissive |
| Rust | image | 0.25.8 | MIT OR Apache-2.0 | permissive |
| Rust | indexmap | 1.9.3 | Apache-2.0 OR MIT | permissive |
| Rust | indexmap | 2.14.2 | Apache-2.0 OR MIT | permissive |
| Rust | infer | 0.22.0 | MIT | permissive |
| Rust | ipnet | 2.12.2 | MIT OR Apache-2.0 | permissive |
| Rust | is_terminal_polyfill | 1.70.2 | MIT OR Apache-2.0 | permissive |
| Rust | itertools | 0.15.0 | MIT OR Apache-2.0 | permissive |
| Rust | itoa | 1.0.18 | MIT OR Apache-2.0 | permissive |
| Rust | jagua-rs | 1.0.0 | MPL-2.0 | MPL exception (unmodified) |
| Rust | jiff | 0.2.38 | Unlicense OR MIT | permissive |
| Rust | jiff-core | 0.1.1 | Unlicense OR MIT | permissive |
| Rust | jiff-static | 0.2.38 | Unlicense OR MIT | permissive |
| Rust | jiff-tzdb | 0.1.9 | Unlicense OR MIT | permissive |
| Rust | jiff-tzdb-platform | 0.1.3 | Unlicense OR MIT | permissive |
| Rust | js-sys | 0.3.106 | MIT OR Apache-2.0 | permissive |
| Rust | json-patch | 4.2.0 | MIT/Apache-2.0 | permissive |
| Rust | jsonptr | 0.7.1 | MIT OR Apache-2.0 | permissive |
| Rust | keyboard-types | 0.8.3 | MIT OR Apache-2.0 | permissive |
| Rust | libc | 0.2.191 | MIT OR Apache-2.0 | permissive |
| Rust | libloading | 0.8.9 | ISC | permissive |
| Rust | libm | 0.2.16 | MIT | permissive |
| Rust | litemap | 0.8.3 | Unicode-3.0 | permissive |
| Rust | litrs | 1.0.1 | MIT OR Apache-2.0 | permissive |
| Rust | lock_api | 0.4.14 | MIT OR Apache-2.0 | permissive |
| Rust | log | 0.4.34 | MIT OR Apache-2.0 | permissive |
| Rust | markup5ever | 0.39.0 | MIT OR Apache-2.0 | permissive |
| Rust | markup5ever_rcdom | 0.39.0+unofficial | MIT OR Apache-2.0 | permissive |
| Rust | matrixmultiply | 0.3.11 | MIT/Apache-2.0 | permissive |
| Rust | memchr | 2.8.3 | Unlicense OR MIT | permissive |
| Rust | mime | 0.3.17 | MIT OR Apache-2.0 | permissive |
| Rust | miniz_oxide | 0.8.9 | MIT OR Zlib OR Apache-2.0 | permissive |
| Rust | miniz_oxide | 0.9.1 | MIT OR Zlib OR Apache-2.0 | permissive |
| Rust | mio | 1.2.4 | MIT | permissive |
| Rust | moxcms | 0.7.11 | BSD-3-Clause OR Apache-2.0 | permissive |
| Rust | muda | 0.20.0 | Apache-2.0 OR MIT | permissive |
| Rust | multiversion_no_op | 1.0.0 | Apache-2.0 OR MIT | permissive |
| Rust | naga | 30.0.1 | MIT OR Apache-2.0 | permissive |
| Rust | naga-types | 30.0.1 | MIT OR Apache-2.0 | permissive |
| Rust | ndarray | 0.17.2 | MIT OR Apache-2.0 | permissive |
| Rust | new_debug_unreachable | 1.0.6 | MIT | permissive |
| Rust | nix | 0.31.3 | MIT | permissive |
| Rust | num-complex | 0.4.6 | MIT OR Apache-2.0 | permissive |
| Rust | num-conv | 0.2.2 | MIT OR Apache-2.0 | permissive |
| Rust | num-integer | 0.1.47 | MIT OR Apache-2.0 | permissive |
| Rust | num-traits | 0.2.19 | MIT OR Apache-2.0 | permissive |
| Rust | num_cpus | 1.17.0 | MIT OR Apache-2.0 | permissive |
| Rust | numfmt | 1.2.0 | MIT | permissive |
| Rust | once_cell | 1.21.4 | MIT OR Apache-2.0 | permissive |
| Rust | once_cell_polyfill | 1.70.2 | MIT OR Apache-2.0 | permissive |
| Rust | open | 5.4.4 | MIT | permissive |
| Rust | option-ext | 0.2.0 | Apache-2.0 | permissive |
| Rust | ordered-float | 5.5.0 | MIT | permissive |
| Rust | parking_lot | 0.12.5 | MIT OR Apache-2.0 | permissive |
| Rust | parking_lot_core | 0.9.12 | MIT OR Apache-2.0 | permissive |
| Rust | percent-encoding | 2.3.2 | MIT OR Apache-2.0 | permissive |
| Rust | phf | 0.13.1 | MIT | permissive |
| Rust | phf_codegen | 0.13.1 | MIT | permissive |
| Rust | phf_generator | 0.13.1 | MIT | permissive |
| Rust | phf_macros | 0.13.1 | MIT | permissive |
| Rust | phf_shared | 0.13.1 | MIT | permissive |
| Rust | pin-project-lite | 0.2.17 | Apache-2.0 OR MIT | permissive |
| Rust | plist | 1.10.1 | MIT | permissive |
| Rust | png | 0.17.16 | MIT OR Apache-2.0 | permissive |
| Rust | png | 0.18.1 | MIT OR Apache-2.0 | permissive |
| Rust | pollster | 0.4.0 | Apache-2.0/MIT | permissive |
| Rust | potential_utf | 0.1.6 | Unicode-3.0 | permissive |
| Rust | powerfmt | 0.2.1 | MIT OR Apache-2.0 | permissive |
| Rust | precomputed-hash | 0.1.1 | MIT | permissive |
| Rust | presser | 0.3.1 | MIT OR Apache-2.0 | permissive |
| Rust | proc-macro2 | 1.0.107 | MIT OR Apache-2.0 | permissive |
| Rust | profiling | 1.0.18 | MIT OR Apache-2.0 | permissive |
| Rust | pxfm | 0.1.30 | BSD-3-Clause OR Apache-2.0 | permissive |
| Rust | qrcode | 0.14.1 | MIT OR Apache-2.0 | permissive |
| Rust | quick-xml | 0.38.4 | MIT | permissive |
| Rust | quick-xml | 0.42.0 | MIT | permissive |
| Rust | quote | 1.0.47 | MIT OR Apache-2.0 | permissive |
| Rust | rand | 0.10.3 | MIT OR Apache-2.0 | permissive |
| Rust | rand_core | 0.10.1 | MIT OR Apache-2.0 | permissive |
| Rust | rand_distr | 0.6.0 | MIT OR Apache-2.0 | permissive |
| Rust | rand_pcg | 0.10.2 | MIT OR Apache-2.0 | permissive |
| Rust | range-alloc | 0.1.5 | MIT OR Apache-2.0 | permissive |
| Rust | raw-window-handle | 0.6.2 | MIT OR Apache-2.0 OR Zlib | permissive |
| Rust | rawpointer | 0.2.1 | MIT/Apache-2.0 | permissive |
| Rust | rayon | 1.12.0 | MIT OR Apache-2.0 | permissive |
| Rust | rayon-core | 1.13.0 | MIT OR Apache-2.0 | permissive |
| Rust | ref-cast | 1.0.27 | MIT OR Apache-2.0 | permissive |
| Rust | ref-cast-impl | 1.0.27 | MIT OR Apache-2.0 | permissive |
| Rust | regex | 1.13.1 | MIT OR Apache-2.0 | permissive |
| Rust | regex-automata | 0.4.18 | MIT OR Apache-2.0 | permissive |
| Rust | regex-syntax | 0.8.11 | MIT OR Apache-2.0 | permissive |
| Rust | renderdoc-sys | 1.1.0 | MIT OR Apache-2.0 | permissive |
| Rust | rfd | 0.16.0 | MIT | permissive |
| Rust | robust | 1.2.0 | MIT OR Apache-2.0 | permissive |
| Rust | roxmltree | 0.21.1 | MIT OR Apache-2.0 | permissive |
| Rust | rstar | 0.12.2 | MIT OR Apache-2.0 | permissive |
| Rust | rust_xlsxwriter | 0.95.0 | MIT OR Apache-2.0 | permissive |
| Rust | rustc-hash | 1.1.0 | Apache-2.0/MIT | permissive |
| Rust | rustc_version | 0.4.1 | MIT OR Apache-2.0 | permissive |
| Rust | rustversion | 1.0.23 | MIT OR Apache-2.0 | permissive |
| Rust | rustybuzz | 0.20.1 | MIT | permissive |
| Rust | ryu | 1.0.23 | Apache-2.0 OR BSL-1.0 | permissive |
| Rust | same-file | 1.0.6 | Unlicense/MIT | permissive |
| Rust | schemars | 0.8.22 | MIT | permissive |
| Rust | schemars | 0.9.0 | MIT | permissive |
| Rust | schemars | 1.2.3 | MIT | permissive |
| Rust | schemars_derive | 0.8.22 | MIT | permissive |
| Rust | scopeguard | 1.2.0 | MIT OR Apache-2.0 | permissive |
| Rust | semver | 1.0.28 | MIT OR Apache-2.0 | permissive |
| Rust | serde | 1.0.229 | MIT OR Apache-2.0 | permissive |
| Rust | serde-untagged | 0.1.9 | MIT OR Apache-2.0 | permissive |
| Rust | serde_core | 1.0.229 | MIT OR Apache-2.0 | permissive |
| Rust | serde_derive | 1.0.229 | MIT OR Apache-2.0 | permissive |
| Rust | serde_derive_internals | 0.29.1 | MIT OR Apache-2.0 | permissive |
| Rust | serde_json | 1.0.154 | MIT OR Apache-2.0 | permissive |
| Rust | serde_repr | 0.1.21 | MIT OR Apache-2.0 | permissive |
| Rust | serde_spanned | 1.1.2 | MIT OR Apache-2.0 | permissive |
| Rust | serde_with | 3.24.0 | MIT OR Apache-2.0 | permissive |
| Rust | serde_with_macros | 3.24.0 | MIT OR Apache-2.0 | permissive |
| Rust | serialize-to-javascript | 0.1.2 | MIT OR Apache-2.0 | permissive |
| Rust | serialize-to-javascript-impl | 0.1.2 | MIT OR Apache-2.0 | permissive |
| Rust | sha2 | 0.10.9 | MIT OR Apache-2.0 | permissive |
| Rust | shlex | 2.0.1 | MIT OR Apache-2.0 | permissive |
| Rust | sif-itree | 0.4.1 | MIT OR Apache-2.0 | permissive |
| Rust | simd-adler32 | 0.3.10 | MIT | permissive |
| Rust | simdutf8 | 0.1.5 | MIT OR Apache-2.0 | permissive |
| Rust | siphasher | 1.0.4 | MIT OR Apache-2.0 | permissive |
| Rust | slab | 0.4.12 | MIT | permissive |
| Rust | slotmap | 1.1.1 | Zlib | permissive |
| Rust | smallvec | 1.16.3 | MIT OR Apache-2.0 | permissive |
| Rust | socket2 | 0.6.5 | MIT OR Apache-2.0 | permissive |
| Rust | softbuffer | 0.4.8 | MIT OR Apache-2.0 | permissive |
| Rust | sparrow | 0.3.0 | MIT (LICENSE file; manifest omits licence) | permissive |
| Rust | spirv | 0.4.0+sdk-1.4.341.0 | Apache-2.0 | permissive |
| Rust | stable_deref_trait | 1.2.1 | MIT OR Apache-2.0 | permissive |
| Rust | static_assertions | 1.1.0 | MIT OR Apache-2.0 | permissive |
| Rust | static_vcruntime | 3.0.0 | MIT OR Apache-2.0 OR Zlib | permissive |
| Rust | string_cache | 0.9.0 | MIT OR Apache-2.0 | permissive |
| Rust | string_cache_codegen | 0.6.1 | MIT OR Apache-2.0 | permissive |
| Rust | strsim | 0.11.1 | MIT | permissive |
| Rust | svg | 0.18.0 | Apache-2.0 OR MIT | permissive |
| Rust | syn | 2.0.119 | MIT OR Apache-2.0 | permissive |
| Rust | syn | 3.0.9 | MIT OR Apache-2.0 | permissive |
| Rust | synstructure | 0.14.0 | MIT | permissive |
| Rust | tao | 0.37.1 | Apache-2.0 | permissive |
| Rust | tap | 1.0.1 | MIT | permissive |
| Rust | tauri | 2.12.4 | Apache-2.0 OR MIT | permissive |
| Rust | tauri-build | 2.7.3 | Apache-2.0 OR MIT | permissive |
| Rust | tauri-codegen | 2.7.3 | Apache-2.0 OR MIT | permissive |
| Rust | tauri-macros | 2.7.3 | Apache-2.0 OR MIT | permissive |
| Rust | tauri-plugin | 2.7.2 | Apache-2.0 OR MIT | permissive |
| Rust | tauri-plugin-dialog | 2.8.3 | Apache-2.0 OR MIT | permissive |
| Rust | tauri-plugin-fs | 2.7.1 | Apache-2.0 OR MIT | permissive |
| Rust | tauri-plugin-opener | 2.7.2 | Apache-2.0 OR MIT | permissive |
| Rust | tauri-plugin-single-instance | 2.5.4 | Apache-2.0 OR MIT | permissive |
| Rust | tauri-runtime | 2.12.2 | Apache-2.0 OR MIT | permissive |
| Rust | tauri-runtime-wry | 2.12.2 | Apache-2.0 OR MIT | permissive |
| Rust | tauri-utils | 2.10.2 | Apache-2.0 OR MIT | permissive |
| Rust | tauri-winres | 0.3.6 | MIT | permissive |
| Rust | tendril | 0.5.1 | MIT OR Apache-2.0 | permissive |
| Rust | test-case | 3.4.0 | MIT | permissive |
| Rust | test-case-core | 3.4.0 | MIT | permissive |
| Rust | test-case-macros | 3.4.0 | MIT | permissive |
| Rust | thiserror | 2.0.21 | MIT OR Apache-2.0 | permissive |
| Rust | thiserror-impl | 2.0.21 | MIT OR Apache-2.0 | permissive |
| Rust | time | 0.3.55 | MIT OR Apache-2.0 | permissive |
| Rust | time-core | 0.1.9 | MIT OR Apache-2.0 | permissive |
| Rust | time-macros | 0.2.32 | MIT OR Apache-2.0 | permissive |
| Rust | tinystr | 0.8.4 | Unicode-3.0 | permissive |
| Rust | tinyvec | 1.13.3 | Zlib OR Apache-2.0 OR MIT | permissive |
| Rust | tokio | 1.53.3 | MIT | permissive |
| Rust | toml | 1.1.8+spec-1.1.0 | MIT OR Apache-2.0 | permissive |
| Rust | toml_datetime | 1.1.2+spec-1.1.0 | MIT OR Apache-2.0 | permissive |
| Rust | toml_parser | 1.1.5+spec-1.1.0 | MIT OR Apache-2.0 | permissive |
| Rust | toml_writer | 1.1.3+spec-1.1.0 | MIT OR Apache-2.0 | permissive |
| Rust | tower-service | 0.3.3 | MIT | permissive |
| Rust | tracing | 0.1.44 | MIT | permissive |
| Rust | tracing-attributes | 0.1.31 | MIT | permissive |
| Rust | tracing-core | 0.1.36 | MIT | permissive |
| Rust | tray-icon | 0.25.1 | MIT OR Apache-2.0 | permissive |
| Rust | try-lock | 0.2.5 | MIT | permissive |
| Rust | ttf-parser | 0.25.1 | MIT OR Apache-2.0 | permissive |
| Rust | typed-path | 0.12.3 | MIT OR Apache-2.0 | permissive |
| Rust | typeid | 1.0.3 | MIT OR Apache-2.0 | permissive |
| Rust | typenum | 1.20.1 | MIT OR Apache-2.0 | permissive |
| Rust | unicode-bidi-mirroring | 0.4.0 | MIT/Apache-2.0 | permissive |
| Rust | unicode-ccc | 0.4.0 | MIT/Apache-2.0 | permissive |
| Rust | unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 | permissive |
| Rust | unicode-properties | 0.1.4 | MIT/Apache-2.0 | permissive |
| Rust | unicode-script | 0.5.8 | MIT OR Apache-2.0 | permissive |
| Rust | unicode-segmentation | 1.13.3 | MIT OR Apache-2.0 | permissive |
| Rust | unicode-width | 0.2.2 | MIT OR Apache-2.0 | permissive |
| Rust | url | 2.5.8 | MIT OR Apache-2.0 | permissive |
| Rust | urlpattern | 0.6.0 | MIT | permissive |
| Rust | utf8_iter | 1.0.4 | Apache-2.0 OR MIT | permissive |
| Rust | utf8parse | 0.2.2 | Apache-2.0 OR MIT | permissive |
| Rust | uuid | 1.28.0 | Apache-2.0 OR MIT | permissive |
| Rust | version_check | 0.9.5 | MIT/Apache-2.0 | permissive |
| Rust | vswhom | 0.1.0 | MIT | permissive |
| Rust | vswhom-sys | 0.1.3 | MIT | permissive |
| Rust | walkdir | 2.5.0 | Unlicense/MIT | permissive |
| Rust | want | 0.3.2 | MIT | permissive |
| Rust | wasm-bindgen | 0.2.129 | MIT OR Apache-2.0 | permissive |
| Rust | wasm-bindgen-macro | 0.2.129 | MIT OR Apache-2.0 | permissive |
| Rust | wasm-bindgen-macro-support | 0.2.129 | MIT OR Apache-2.0 | permissive |
| Rust | wasm-bindgen-shared | 0.2.129 | MIT OR Apache-2.0 | permissive |
| Rust | web-time | 1.1.0 | MIT OR Apache-2.0 | permissive |
| Rust | web_atoms | 0.2.6 | MIT OR Apache-2.0 | permissive |
| Rust | webview2-com | 0.39.1 | MIT | permissive |
| Rust | webview2-com-macros | 0.8.1 | MIT | permissive |
| Rust | webview2-com-sys | 0.39.1 | MIT | permissive |
| Rust | wgpu | 30.0.1 | MIT OR Apache-2.0 | permissive |
| Rust | wgpu-core | 30.0.1 | MIT OR Apache-2.0 | permissive |
| Rust | wgpu-core-deps-windows-linux-android | 30.0.1 | MIT OR Apache-2.0 | permissive |
| Rust | wgpu-hal | 30.0.1 | MIT OR Apache-2.0 | permissive |
| Rust | wgpu-naga-bridge | 30.0.1 | MIT OR Apache-2.0 | permissive |
| Rust | wgpu-types | 30.0.1 | MIT OR Apache-2.0 | permissive |
| Rust | winapi-util | 0.1.11 | Unlicense OR MIT | permissive |
| Rust | window-vibrancy | 0.8.1 | Apache-2.0 OR MIT | permissive |
| Rust | windows | 0.62.2 | MIT OR Apache-2.0 | permissive |
| Rust | windows-collections | 0.3.2 | MIT OR Apache-2.0 | permissive |
| Rust | windows-core | 0.62.2 | MIT OR Apache-2.0 | permissive |
| Rust | windows-future | 0.3.2 | MIT OR Apache-2.0 | permissive |
| Rust | windows-implement | 0.60.2 | MIT OR Apache-2.0 | permissive |
| Rust | windows-interface | 0.59.3 | MIT OR Apache-2.0 | permissive |
| Rust | windows-link | 0.2.1 | MIT OR Apache-2.0 | permissive |
| Rust | windows-numerics | 0.3.1 | MIT OR Apache-2.0 | permissive |
| Rust | windows-result | 0.4.1 | MIT OR Apache-2.0 | permissive |
| Rust | windows-strings | 0.5.1 | MIT OR Apache-2.0 | permissive |
| Rust | windows-sys | 0.60.2 | MIT OR Apache-2.0 | permissive |
| Rust | windows-sys | 0.61.2 | MIT OR Apache-2.0 | permissive |
| Rust | windows-targets | 0.53.5 | MIT OR Apache-2.0 | permissive |
| Rust | windows-threading | 0.2.1 | MIT OR Apache-2.0 | permissive |
| Rust | windows-version | 0.1.7 | MIT OR Apache-2.0 | permissive |
| Rust | windows_x86_64_gnu | 0.53.1 | MIT OR Apache-2.0 | permissive |
| Rust | windows_x86_64_msvc | 0.53.1 | MIT OR Apache-2.0 | permissive |
| Rust | winnow | 1.0.4 | MIT | permissive |
| Rust | winreg | 0.56.0 | MIT | permissive |
| Rust | writeable | 0.6.4 | Unicode-3.0 | permissive |
| Rust | wry | 0.57.0 | Apache-2.0 OR MIT | permissive |
| Rust | xml5ever | 0.39.0 | MIT OR Apache-2.0 | permissive |
| Rust | yoke | 0.8.3 | Unicode-3.0 | permissive |
| Rust | yoke-derive | 0.8.4 | Unicode-3.0 | permissive |
| Rust | zerocopy | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | permissive |
| Rust | zerocopy-derive | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | permissive |
| Rust | zerofrom | 0.1.8 | Unicode-3.0 | permissive |
| Rust | zerofrom-derive | 0.1.8 | Unicode-3.0 | permissive |
| Rust | zerotrie | 0.2.5 | Unicode-3.0 | permissive |
| Rust | zerovec | 0.11.8 | Unicode-3.0 | permissive |
| Rust | zerovec-derive | 0.11.6 | Unicode-3.0 | permissive |
| Rust | zip | 4.6.1 | MIT | permissive |
| Rust | zip | 7.4.0 | MIT | permissive |
| Rust | zlib-rs | 0.6.8 | Zlib | permissive |
| Rust | zmij | 1.0.23 | MIT | permissive |
| Rust | zopfli | 0.8.4 | Apache-2.0 | permissive |
| Rust | zune-core | 0.4.12 | MIT OR Apache-2.0 OR Zlib | permissive |
| Rust | zune-jpeg | 0.4.21 | MIT OR Apache-2.0 OR Zlib | permissive |
| npm | @beta/marker-desktop | 0.6.0 | Apache-2.0 | permissive |
| npm | @beta/marker-site | 0.6.0 | Apache-2.0 | permissive |
| npm | @beta/marker-wasm | 0.6.0 | Apache-2.0 | permissive |
| npm | @beta/marker-web | 0.6.0 | Apache-2.0 | permissive |
| npm | @esbuild/win32-x64 | 0.25.12 | MIT | permissive |
| npm | @esbuild/win32-x64 | 0.28.2 | MIT | permissive |
| npm | @freesewing/bella | 4.10.2 | MIT | permissive |
| npm | @freesewing/brian | 4.10.2 | MIT | permissive |
| npm | @freesewing/config | 4.10.2 | MIT | permissive |
| npm | @freesewing/core | 4.10.2 | MIT | permissive |
| npm | @freesewing/core-plugins | 4.10.2 | MIT | permissive |
| npm | @freesewing/hugo | 4.10.2 | MIT | permissive |
| npm | @freesewing/library | 4.10.2 | MIT | permissive |
| npm | @freesewing/models | 4.10.2 | MIT | permissive |
| npm | @freesewing/plugin-annotations | 4.10.2 | MIT | permissive |
| npm | @freesewing/plugin-bin-pack | 4.10.2 | MIT | permissive |
| npm | @freesewing/plugin-bust | 4.10.2 | MIT | permissive |
| npm | @freesewing/plugin-flip | 4.10.2 | MIT | permissive |
| npm | @freesewing/plugin-measurements | 4.10.2 | MIT | permissive |
| npm | @freesewing/plugin-mirror | 4.10.2 | MIT | permissive |
| npm | @freesewing/plugin-round | 4.10.2 | MIT | permissive |
| npm | @freesewing/plugin-sprinkle | 4.10.2 | MIT | permissive |
| npm | @freesewing/plugin-transform | 4.10.2 | MIT | permissive |
| npm | @freesewing/simon | 4.10.2 | MIT | permissive |
| npm | @freesewing/snapseries | 4.10.2 | MIT | permissive |
| npm | @freesewing/teagan | 4.10.2 | MIT | permissive |
| npm | @freesewing/titan | 4.10.2 | MIT | permissive |
| npm | @jridgewell/sourcemap-codec | 1.6.0 | MIT | permissive |
| npm | @rollup/rollup-win32-x64-gnu | 4.64.10 | MIT | permissive |
| npm | @rollup/rollup-win32-x64-msvc | 4.64.10 | MIT | permissive |
| npm | @tauri-apps/cli | 2.12.3 | Apache-2.0 OR MIT | permissive |
| npm | @tauri-apps/cli-win32-x64-msvc | 2.12.3 | Apache-2.0 OR MIT | permissive |
| npm | @types/chai | 5.2.3 | MIT | permissive |
| npm | @types/deep-eql | 4.0.2 | MIT | permissive |
| npm | @types/estree | 1.0.9 | MIT | permissive |
| npm | @types/node | 26.6.5 | MIT | permissive |
| npm | @vitest/expect | 3.2.7 | MIT | permissive |
| npm | @vitest/mocker | 3.2.7 | MIT | permissive |
| npm | @vitest/pretty-format | 3.2.7 | MIT | permissive |
| npm | @vitest/runner | 3.2.7 | MIT | permissive |
| npm | @vitest/snapshot | 3.2.7 | MIT | permissive |
| npm | @vitest/spy | 3.2.7 | MIT | permissive |
| npm | @vitest/utils | 3.2.7 | MIT | permissive |
| npm | assertion-error | 2.0.1 | MIT | permissive |
| npm | beta-open-patterns-freesewing | 0.6.0 | Apache-2.0 | permissive |
| npm | beta-open-patterns-garmentcode | 0.6.0 | Apache-2.0 | permissive |
| npm | bezier-js | 6.1.4 | MIT | permissive |
| npm | cac | 6.7.14 | MIT | permissive |
| npm | chai | 5.3.3 | MIT | permissive |
| npm | check-error | 2.1.3 | MIT | permissive |
| npm | debug | 4.4.3 | MIT | permissive |
| npm | deep-eql | 5.0.2 | MIT | permissive |
| npm | es-module-lexer | 1.7.0 | MIT | permissive |
| npm | esbuild | 0.25.12 | MIT | permissive |
| npm | esbuild | 0.28.2 | MIT | permissive |
| npm | estree-walker | 3.0.3 | MIT | permissive |
| npm | expect-type | 1.4.0 | Apache-2.0 | permissive |
| npm | fdir | 6.5.0 | MIT | permissive |
| npm | hooks | 0.3.2 | MIT | permissive |
| npm | js-tokens | 9.0.1 | MIT | permissive |
| npm | lodash.clonedeep | 4.5.0 | MIT | permissive |
| npm | lodash.get | 4.4.2 | MIT | permissive |
| npm | lodash.set | 4.3.2 | MIT | permissive |
| npm | lodash.unset | 4.5.2 | MIT | permissive |
| npm | loupe | 3.2.1 | MIT | permissive |
| npm | magic-string | 0.30.21 | MIT | permissive |
| npm | ms | 2.1.3 | MIT | permissive |
| npm | nanoid | 3.3.20 | MIT | permissive |
| npm | pathe | 2.0.3 | MIT | permissive |
| npm | pathval | 2.0.1 | MIT | permissive |
| npm | picocolors | 1.1.1 | ISC | permissive |
| npm | picomatch | 4.0.7 | MIT | permissive |
| npm | polygon-clipping | 0.14.3 | MIT | permissive |
| npm | postcss | 8.5.31 | MIT | permissive |
| npm | rollup | 4.64.10 | MIT | permissive |
| npm | siginfo | 2.0.0 | ISC | permissive |
| npm | source-map-js | 1.2.2 | BSD-3-Clause | permissive |
| npm | splaytree | 3.2.3 | MIT | permissive |
| npm | stackback | 0.0.2 | MIT | permissive |
| npm | std-env | 3.10.0 | MIT | permissive |
| npm | strip-literal | 3.1.0 | MIT | permissive |
| npm | tinybench | 2.9.0 | MIT | permissive |
| npm | tinyexec | 0.3.2 | MIT | permissive |
| npm | tinyglobby | 0.2.17 | MIT | permissive |
| npm | tinypool | 1.1.1 | MIT | permissive |
| npm | tinyrainbow | 2.0.0 | MIT | permissive |
| npm | tinyspy | 4.0.6 | MIT | permissive |
| npm | typescript | 5.9.3 | Apache-2.0 | permissive |
| npm | undici-types | 8.9.0 | MIT | permissive |
| npm | vite | 6.4.4 | MIT | permissive |
| npm | vite | 7.3.7 | MIT | permissive |
| npm | vite-node | 3.2.4 | MIT | permissive |
| npm | vitest | 3.2.7 | MIT | permissive |
| npm | why-is-node-running | 2.3.0 | MIT | permissive |

## Bundled Windows SDK loader

The NSIS desktop package also contains **WebView2Loader.dll 1.0.3800.47**.
Its bytes match the x64 DLL in Microsoft's official SDK of that version.
The SDK licence grants redistribution under BSD three-clause terms; this
is separate from the Rust binding crate's MIT declaration. The loader is
permissive and adds no copyleft exception. It is not the WebView2 Runtime.
The Runtime is a separately installed Microsoft prerequisite, not bundled.

- [Exact SDK licence](https://www.nuget.org/packages/Microsoft.Web.WebView2/1.0.3800.47/License).
- [Bundled licence text](deploy/licenses/WebView2-SDK-1.0.3800.47.txt).
- Loader SHA-256: `86545b66cdb0603bc26b626fb9ad610cb6e71f28d468f5ea66df23b03dda96d5`.


## Resolved outside the shipped target graphs

The following metadata-only packages are not in the selected build graphs.
In particular, a Linux Tauri desktop is not shipped. Its GTK/WebKit platform
stack has additional system-library terms and needs a separate review.
Do not describe the unrestricted all-platform desktop graph as permissive-only.

| Package | Version | Declared licence |
|---|---|---|
| async-broadcast | 0.7.2 | MIT OR Apache-2.0 |
| async-channel | 2.5.0 | Apache-2.0 OR MIT |
| async-executor | 1.14.0 | Apache-2.0 OR MIT |
| async-io | 2.6.0 | Apache-2.0 OR MIT |
| async-lock | 3.4.2 | Apache-2.0 OR MIT |
| async-process | 2.5.0 | Apache-2.0 OR MIT |
| async-recursion | 1.2.0 | MIT OR Apache-2.0 |
| async-signal | 0.2.14 | Apache-2.0 OR MIT |
| async-task | 4.7.1 | Apache-2.0 OR MIT |
| async-trait | 0.1.92 | MIT OR Apache-2.0 |
| atk | 0.18.2 | MIT |
| atk-sys | 0.18.2 | MIT |
| blocking | 1.7.0 | Apache-2.0 OR MIT |
| cairo-rs | 0.18.5 | MIT |
| cairo-sys-rs | 0.18.2 | MIT |
| cfg-expr | 0.15.8 | MIT OR Apache-2.0 |
| concurrent-queue | 2.5.0 | Apache-2.0 OR MIT |
| dbus | 0.9.12 | Apache-2.0/MIT |
| dlopen2 | 0.8.2 | MIT |
| dlopen2_derive | 0.4.3 | MIT |
| endi | 1.1.1 | MIT |
| enumflags2 | 0.7.12 | MIT OR Apache-2.0 |
| enumflags2_derive | 0.7.12 | MIT OR Apache-2.0 |
| errno | 0.3.14 | MIT OR Apache-2.0 |
| event-listener | 5.4.2 | Apache-2.0 OR MIT |
| event-listener-strategy | 0.5.4 | Apache-2.0 OR MIT |
| field-offset | 0.3.6 | MIT OR Apache-2.0 |
| futures-executor | 0.3.34 | MIT OR Apache-2.0 |
| futures-lite | 2.6.1 | Apache-2.0 OR MIT |
| gdk | 0.18.2 | MIT |
| gdk-pixbuf | 0.18.5 | MIT |
| gdk-pixbuf-sys | 0.18.0 | MIT |
| gdk-sys | 0.18.2 | MIT |
| gdkwayland-sys | 0.18.2 | MIT |
| gdkx11 | 0.18.2 | MIT |
| gdkx11-sys | 0.18.2 | MIT |
| gio | 0.18.4 | MIT |
| gio-sys | 0.18.1 | MIT |
| glib | 0.18.5 | MIT |
| glib-macros | 0.18.5 | MIT |
| glib-sys | 0.18.1 | MIT |
| gobject-sys | 0.18.0 | MIT |
| gtk | 0.18.2 | MIT |
| gtk-sys | 0.18.2 | MIT |
| gtk3-macros | 0.18.2 | MIT |
| heck | 0.4.1 | MIT OR Apache-2.0 |
| iana-time-zone | 0.1.65 | MIT OR Apache-2.0 |
| is-docker | 0.2.0 | MIT |
| is-wsl | 0.4.0 | MIT |
| javascriptcore-rs | 1.1.2 | MIT |
| javascriptcore-rs-sys | 1.1.1 | MIT |
| libappindicator | 0.9.0 | Apache-2.0 OR MIT |
| libappindicator-sys | 0.9.0 | Apache-2.0 OR MIT |
| libdbus-sys | 0.2.7 | Apache-2.0/MIT |
| libloading | 0.7.4 | ISC |
| linux-raw-sys | 0.12.1 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| memoffset | 0.9.1 | MIT |
| ordered-stream | 0.2.0 | MIT OR Apache-2.0 |
| pango | 0.18.3 | MIT |
| pango-sys | 0.18.0 | MIT |
| parking | 2.2.1 | Apache-2.0 OR MIT |
| piper | 0.2.5 | MIT OR Apache-2.0 |
| pkg-config | 0.3.34 | MIT OR Apache-2.0 |
| polling | 3.11.0 | Apache-2.0 OR MIT |
| proc-macro-crate | 1.3.1 | MIT OR Apache-2.0 |
| proc-macro-crate | 2.0.2 | MIT OR Apache-2.0 |
| proc-macro-crate | 3.5.0 | MIT OR Apache-2.0 |
| proc-macro-error | 1.0.4 | MIT OR Apache-2.0 |
| proc-macro-error-attr | 1.0.4 | MIT OR Apache-2.0 |
| rustix | 1.1.5 | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT |
| serde_spanned | 0.6.9 | MIT OR Apache-2.0 |
| signal-hook-registry | 1.4.8 | MIT OR Apache-2.0 |
| soup3 | 0.5.0 | MIT |
| soup3-sys | 0.5.0 | MIT |
| syn | 1.0.109 | MIT OR Apache-2.0 |
| system-deps | 6.2.2 | MIT OR Apache-2.0 |
| target-lexicon | 0.12.16 | Apache-2.0 WITH LLVM-exception |
| thiserror | 1.0.69 | MIT OR Apache-2.0 |
| thiserror-impl | 1.0.69 | MIT OR Apache-2.0 |
| toml | 0.8.2 | MIT OR Apache-2.0 |
| toml_datetime | 0.6.3 | MIT OR Apache-2.0 |
| toml_edit | 0.19.15 | MIT OR Apache-2.0 |
| toml_edit | 0.20.2 | MIT OR Apache-2.0 |
| toml_edit | 0.25.17+spec-1.1.0 | MIT OR Apache-2.0 |
| version-compare | 0.2.1 | MIT |
| webkit2gtk | 2.0.2 | MIT |
| webkit2gtk-sys | 2.0.2 | MIT |
| winnow | 0.5.40 | MIT |
| x11 | 2.21.0 | MIT |
| x11-dl | 2.21.0 | MIT |
| zbus | 5.19.0 | MIT |
| zbus_macros | 5.19.0 | MIT |
| zbus_names | 4.3.5 | MIT |
| zcheapstr | 1.1.0 | MIT |
| zvariant | 5.15.1 | MIT |
| zvariant_derive | 5.15.1 | MIT |
| zvariant_utils | 4.2.0 | MIT |

## Source and notice delivery

Package sources are fixed by the release Cargo/npm lockfiles. Cargo registry
sources are at crates.io; npm package sources are at npmjs.com. The sparrow git
revision is recorded in Cargo.lock. The exact unmodified jagua-rs source is
available from https://crates.io/api/v1/crates/jagua-rs/1.0.0/download.
Retain access to that source when distributing executables under MPL-2.0.
Release packages include THIRD-PARTY-NOTICES.txt with local upstream licence
and notice files. Dataset attributions are separate in NOTICE and fixture READMEs.

Regenerate from the repository root with installed npm/tool dependencies:

```sh
python marker/deploy/generate-licenses.py --include-optional --texts <output>/THIRD-PARTY-NOTICES.txt --json <output>/licenses.json
```
