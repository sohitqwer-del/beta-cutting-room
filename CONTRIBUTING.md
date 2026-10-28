# Help improve BETA Cutting Room

BETA is a fun project built with modern AI tools. It is young and needs people
who make, cut, teach, test and write software. Small, clear contributions help.
Please follow our [Code of Conduct](CODE_OF_CONDUCT.md).

## You do not need to write code

- Describe one confusing step, with the screen name and expected result.
- Review Hindi/Tamil with a native-speaking factory operator.
- Check print scale or try one small order and report measured results.
- Improve a guide or send a permission-cleared synthetic example.

Use [issues](https://github.com/sohitqwer-del/beta-cutting-room/issues/new/choose)
or [Discussions](https://github.com/sohitqwer-del/beta-cutting-room/discussions)
(when enabled). Contact: sohitqwer@gmail.com. Community help has no promised
response time. Report security problems privately via [SECURITY.md](SECURITY.md).

## Report a reproducible problem

Include the version, browser/OS, screen, exact steps, expected result and actual
result. For a geometry problem, include units, counts, layout and rule settings.
Use a small public or original synthetic input. Describe whether validation
passed; keep warnings. A screenshot alone may not explain a calculation error.

**Public reports are public.** Do not upload factory patterns, buyer/style names,
roll/order records, faces, tokens, private paths, vendor specifications or
unlicensed examples. Do not read or submit anything from `private/` folders.
Anonymizing a filename does not clear rights to its geometry. Agree permission
and handling with the maintainer before sharing non-public evidence.

## Change the code or docs

1. For a substantial change, open an issue to agree the problem and scope. Small fixes may go straight to a focused pull request.
2. Fork and make a branch. Read [Marker](marker/README.md), [Pattern](pattern/README.md) and the relevant contract in their docs folders.
3. Keep geometry, nesting and production calculations in Rust. TypeScript handles UI and transport. Preserve independent validation, units, cancellation and complete-copy rules.
4. Keep the app offline: no runtime third-party service, CDN, analytics or asset download. Use accessible labels, keyboard controls and plain short English.
5. Add meaningful tests for changed calculations and failure cases. Keep synthetic fixtures labelled; never invent measured provenance.
6. Run the relevant checks below. State exact passes, failures, ignored tests and untested platforms in the PR. Do not claim physical machine acceptance from a file test.
7. Explain the problem, resulting behaviour and validation. Include public/synthetic screenshots for visible changes. Update docs and notices with the implementation.

## Local checks

Use the toolchain and WASM build steps in [INSTALL.md](marker/docs/INSTALL.md).
Run npm commands from the repository root and Cargo commands in the indicated
workspace. Put Cargo output outside the checkout; use three jobs on shared PCs.

```sh
npm install
npm test
npm run typecheck
```

In `marker/` and then `pattern/`:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The Marker workspace includes Windows desktop dependencies. Linux server-only
work may need targeted crates; report that scope. Rebuild each affected WASM
module and run real-WASM checks when its Rust source changes. Use
[release gates](marker/docs/RELEASE-CHECKLIST.md) for distributable changes.
For docs-only changes, check relative links, paths, privacy and wording;
app builds are not evidence for prose.

## Licences and benchmark claims

Submit original code/docs you may license under Apache-2.0. Keep existing
notices. Prefer MIT, Apache-2.0, BSD or ISC dependencies; review all transitive
code and assets before adding them. No GPL/AGPL/LGPL, non-commercial or unclear
new dependencies. Existing unmodified jagua-rs has an explicit MPL-2.0 exception;
it does not authorize new exceptions. See [third-party notices](marker/THIRD-PARTY-LICENSES.md).
Data retains its stated licence and attribution; code licensing does not clear data.

For a performance claim, publish frozen legal inputs, constraints, seeds,
requested and actual time, environment, failures and independent validation.
Show losses as well as wins. Compare total order consumption and useful yield
where appropriate. See [benchmark method](marker/docs/benchmarks.md).
