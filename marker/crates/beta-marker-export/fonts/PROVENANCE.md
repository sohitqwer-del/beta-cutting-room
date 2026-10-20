# Fonts and fixed chalk-guide labels

The two Droid fonts are from the Android Open Source Project, revision `ab51002`:
https://android.googlesource.com/platform/frameworks/base/+/ab51002/data/fonts/

The upstream README says these fonts use Apache License 2.0. Its original
`README.txt` and `NOTICE` are included. Font SHA-256:

- DroidSansDevanagari-Regular.ttf: `0270f4d3705e207788584190fd0155c43b0421ddd7041a9bb3a406ff5dfa2a95`
- DroidSansTamil-Regular.ttf: `62f992f733a7235a338be19e0837104584062159ffae83f612b6550806833d86`

`chalk-labels.json` contains derived outlines of fixed UI headings, shaped by
Rustybuzz 0.20.1 (MIT) and ttf-parser 0.25.1 (MIT/Apache-2.0). The fonts and shaper
are **not** loaded by the browser or PDF runtime. The shaper is a dev-dependency
used only to regenerate the fixed labels. Rustybuzz is archived upstream; it is
used here only with these pinned, trusted fonts and strings, never user fonts.

Regenerate from `marker/`:

```sh
cargo run -p beta-marker-export --example generate_chalk_labels -- crates/beta-marker-export/fonts/chalk-labels.json
```

Labels include UTF-16 PDF ActualText plus their vector paths. The PDF therefore
renders joined Indic glyphs offline without depending on installed fonts.
Hindi and Tamil wording is provisional and is marked **needs native review**.
English headings remain alongside it. Dynamic piece names use the existing
ASCII Helvetica export policy; unsupported characters become `?`.
