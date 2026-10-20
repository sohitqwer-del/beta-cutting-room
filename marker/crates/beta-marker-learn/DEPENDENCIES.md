# Dependency review

No new runtime algorithm package was added. The crate uses existing workspace
packages through explicit dependencies while its workspace is isolated.

| Package | Resolved version | Licence |
|---|---|---|
| beta-marker-core | local 0.1.0 | Apache-2.0 |
| anyhow | 1.0.104 | MIT OR Apache-2.0 |
| serde / serde_core / serde_derive | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.154 | MIT OR Apache-2.0 |
| itoa | 1.0.18 | MIT OR Apache-2.0 |
| memchr | 2.8.3 | MIT OR Unlicense (use MIT) |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 |
| quote | 1.0.47 | MIT OR Apache-2.0 |
| syn | 3.0.9 | MIT OR Apache-2.0 |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| zmij | 1.0.23 | MIT |

Training/transport only; no Python package is used by the product:

| Package | Version | Licence |
|---|---|---|
| numpy | 2.2.6 | BSD-3-Clause |
| pandas | 2.2.3 | BSD-3-Clause |
| scikit-learn | 1.7.2 | BSD-3-Clause |
| scipy | 1.17.1 | BSD-3-Clause |
| joblib | 1.6.0 | BSD-3-Clause |
| threadpoolctl | 3.7.0 | BSD-3-Clause |
| cloudpickle | 3.1.2 | BSD-3-Clause |

Rust licences were checked in the resolved registry Cargo.toml manifests.
Python package licences were checked from installed distribution metadata;
scikit-learn's [pinned upstream licence](https://github.com/scikit-learn/scikit-learn/blob/1.7.2/COPYING)
was also checked. Existing numerical wheels carry their own third-party notices;
training packages are not bundled into the product. The Python version pins are
in `tools/requirements-training.txt`. No Linfa, GPU or non-permissive dependency
was introduced. Cargo.lock is deliberately left to the lead.
