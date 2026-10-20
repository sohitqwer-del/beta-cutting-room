# Original permissive adapters

These small Apache-2.0 crates replace transitive MPL-only helpers in the Windows Tauri build. No upstream helper source is copied. They do not patch jagua-rs.

- `dom-query` implements the tree operations and four selector patterns used by the pinned Tauri 2.12.4 / tauri-codegen 2.7.3 / tauri-utils 2.10.2 versions. HTML parsing, script text handling and escaping come from the MIT/Apache-2.0 html5ever and markup5ever_rcdom crates. Tests cover CSP attribute escaping, injected script text, nonce selection and existing document content. It is deliberately not a general CSS selector library. `script[src^='http']`, `script[src]`, `script:not(:empty)` and basic tag selectors are supported. Keep upstream codegen pinned and recheck every selector/call before upgrading.
- `option-ext` implements borrowed `Option::contains`, which dirs-sys requires on Unix; its Windows build imports the crate without calling it. No other Option extension methods are exposed.

The source is original work for Project BETA. Local package versions match the upstream package constraints only so Cargo can substitute them. Copy both patches to the top-level workspace before removing this app's standalone workspace.
