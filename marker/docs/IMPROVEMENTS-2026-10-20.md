# Product audit: 20 October 2026

> Historical audit/backlog. Browser cutting-room work and backup arrived in
> [0.7](RELEASE-0.7.0.md); a bounded width/ratio/order-cost screen is implemented
> in unreleased 0.8. Use [features](FEATURES.md) and [roadmap](ROADMAP.md) for
> current scope. The table below retains the original findings and estimates.

Worktree role: `marker/audit`, based on `marker/mvp` at `0d7b298`.
The lead later requested merges of `ebe07fe` and `176e453`. Both are included;
the new UX layout, Pattern2, site work and refreshed lockfiles are preserved.
This is a fun project created with the help of modern AI tools. It has useful
cutting-room tools, but that does not establish commercial or factory parity.

Only built-in original demos, original test fixtures and public FreeSewing data
are allowed in this audit. No vendor files or private factory records are used.
Browser work was headless. No native app window or machine connection was used.
Nothing was pushed or published.

The execution matrix below records completed automated checks and remaining
acceptance limits. UX changes belong to `marker/ux`. The audit changes small web logic only.
Pattern/Digitize, branding/site and folded/tubular nesting belong to other agents.

## Priorities

Impact and effort are engineering estimates, not measured savings. P1 means a
data-integrity, cutting-release or adoption risk; P2 improves daily work; P3 is
useful follow-up. Days/weeks include focused tests but not factory acceptance.

| Priority | Finding / next action | Impact | Effort | Suggested owner | Evidence / status |
| --- | --- | --- | --- | --- | --- |
| P1 | Re-inspect rolls after buyer limit, score unit or shrinkage grouping changes | Avoid using an obsolete PASS and stock group | Small | audit | Fixed; real API/browser regression includes an in-flight policy change |
| P1 | Preserve audited lay ID and shade runs when switching bundle sources | Prevent a broken or incorrect physical lay handoff | Small | audit | Fixed; reselected spread now produces real bundles with the original identity |
| P1 | Show incomplete cut orders as incomplete, with missing quantities | Avoid declaring a partial plan ready for cutting | Small | audit | Fixed; real 10 mm-width order reports M:2 missing and does not claim readiness |
| P1 | Protect text in roll, cutting-report and batch CSV exports from spreadsheet formulas | Safer reviewed Excel exchange | Small | audit | Fixed; three native export regressions cover literal identifiers and numeric values |
| P1 | Reset transient inspection, spreading, comparison and report views when opening another desktop project | Avoid showing data from the previous project | 1–3 days | desktop + ux | Source concern; needs a dedicated two-project integration regression |
| P1 | Test Windows 10 installation with no WebView2, offline prerequisites, upgrade and recovery | Establish actual installability | 2–4 days plus clean PCs | release | Prior release explicitly leaves these gates open |
| P1 | Build Linux on an older glibc baseline and test that exact artifact there | Current glibc 2.43 artifact does not promise older Linux support | 1–2 days | release | Planned Ubuntu 22.04 builder; see RELEASE-BUILDS.md |
| P1 | Remove developer paths and record toolchain/lockfiles for every release | Avoid local path disclosure; improve repeatability | Small | audit + release | Fixed; final CLI and all three WASM modules pass local-path scans; two-clean-directory reproducibility remains a release gate |
| P1 | Certify printer, plotter, cutter and thermal-ticket profiles on physical devices | A downloaded file is not a verified cut | 1–3 weeks with pilot | output + factory QA | No hardware acceptance in this headless audit |
| P1 | Add approved marker revisions and an immutable released cut package | Keep the exact geometry, stock, plies and operator approval together | 2–4 weeks | core integration + floor | Local library and Floor history exist; shared approval/revision workflow remains partial |
| P1 | Finish independent server security review, including shared credentials and browser-origin policy | Establish safe factory-network deployment | 3–5 days | security + serve | Existing limits/auth tests help; no independent audit claim |
| P1 | Provide a private security-reporting channel | Give users a private channel after public launch | Small | lead + security | Resolved in merged mvp: SECURITY.md now names private GitHub reporting and the maintainer email; operational monitoring is the maintainer's responsibility |
| P2 | Expose queue priority, progress, pause/resume, failures and checkpoint recovery in the UI | Make overnight throughput usable without a developer | 1–2 weeks | ux + queue | Persistent queue already exists in Rust/CLI; do not rebuild it |
| P2 | Bring width/ratio/cost alternatives into one reviewed screen | Choose by total feasible order consumption/cost | 1–2 weeks | ux + plan/roll | Library/CLI exist; UI exposes a simpler cut plan |
| P2 | Label planning time as search per candidate and show elapsed time, phase and cancel | Avoid interpreting five seconds as total plan time | 2–4 days | ux + plan/serve | Source: several markers may be evaluated; construction/checks add time |
| P2 | Make plan lay cost semantics clear | UI INR estimate does not drive the planner's metre-equivalent objective | 1–2 days | ux + plan | `layCost` is display-only; Rust defaults `lay_cost_m` to 0.5; no fabric price conversion exists in this form |
| P2 | Gather costing, measured consumption and buyer/monthly report packs under Reports | Reduce jumps between Bundles, Floor and native-only reports | 1–2 weeks | ux + cutroom | Dedicated Reports exists but richer configuration remains native-only |
| P2 | Add safe reversal/correction, sewing receipt and transfer workflows | Correct real shift mistakes with a traceable ledger | 2–3 weeks | floor | Forward-only attestations and duplicate-safe issue exist; no receipt/transfer/reversal workflow |
| P2 | Improve remnant receiving and physical weight actuals | Reconcile end bits and kg without double counting folded layers | 1–2 weeks | roll + floor/reports | End-bit values exist; verified remnants need new identities; actual kg is not a full entry workflow |
| P2 | Put backup/export and storage scope beside production records | Operators need to know what survives clearing site data or changing URL | 2–3 days | ux + persistence | Browser profiles own drafts; server is not a shared document database |
| P2 | Keep labels readable in very short marker PDFs | Avoid clipped part/copy text on small markers | 1–2 days | export | Visually reproduced on the original 20 mm square: the page clips the part label; long header words also break into tiny columns. A4 bundle ticket is legible. Keep geometry at 1:1 when fixing text bounds |
| P2 | Support Unicode names in printed production documents | Keep local-language part/operator text readable | 3–5 days | export + operator review | Export PDF documents an ASCII Helvetica fallback that replaces other characters; UI translation alone does not provide Unicode print support |
| P2 | Show import assumptions, incomplete/failed results and production readiness persistently | Prevent status text from hiding a blocking issue | 2–4 days | ux + import | Distinguish transient success messages from current validation/inspection state |
| P2 | Reject duplicate roll CSV columns; accept case-insensitive JSON extensions | Avoid ambiguous columns and a Windows filename trap | Small | audit | Fixed; CSV unit regression and real `AUDIT.JSON` import passed |
| P2 | Profile 150–1000-piece import, validate, live edit, nest setup and export separately | Find the phase that makes a large job feel stuck | 3–5 days | performance + engine | Bounded synthetic/public probes planned; laptop timings are not minimum-PC guarantees |
| P3 | Reconcile stale launch-candidate text and public download placeholders | Help users find the right installed version and release | Small | audit + release/site | README/INSTALL/CHANGELOG corrected; release hashes kept historical; site remains outside this role |
| P3 | Explain missing benchmark load evidence and name the missing file | Make load-audit/finalize errors useful | Small | audit | Fixed after real CLI reproduction; passing regression covers conditions.json and load-audit.json |
| P3 | Accept page flags such as `?welcome` on static requests | Let welcome/feedback test links and cache-busters load the app | Small | audit, requested by ux | Fixed with a passing regression; API syntax/duplicate checks and static traversal protection stay enforced |
| P3 | Review Hindi/Tamil terms with operators and label incomplete translation scope | Avoid mixed-language production instructions | 2–4 days | ux + operator review | Draft catalogues exist; native-speaker acceptance remains open |
| P3 | List sizes in the familiar garment order consistently | Reduce wrong-size quantity entry | Small | ux | Some web lists use alphabetic sorting; Rust's garment-size ordering already exists |

## Top ten recommendations

1. Tie every roll PASS to the exact buyer policy and grouping settings used to
   calculate it. Require a fresh inspection when any of them changes.
2. Keep audited lay identity and shade runs intact through every handoff. Test
   switching sources and reopening projects, not only the first happy path.
3. Save one approved cut package with the marker revision, independent checks,
   roll sequence, plies, tickets and operator. Make later changes create a new
   revision instead of quietly changing released work.
4. Prove installation on a clean Windows 10 PC and an older Linux baseline.
   Include offline WebView2 instructions and a repeatable upgrade/recovery test.
5. Calibrate the actual plotter, cutter and ticket printer. Keep profiles and
   the measured acceptance result with the output; generic ISO is illustrative.
6. Put the existing production queue into the UI. Show what is waiting, running,
   paused, failed and recovered, with bounded CPU use and a checked checkpoint.
7. Compare widths and ratios by the whole order's feasible fabric use and cost.
   Include end loss, over-cut, eligible stock and remnant value.
8. Give reports a clear route from planned to measured use. Explain net marker
   consumption, gross planned consumption and actual use separately, with units.
9. Add traceable corrections, sewing receipt and transfers before calling the
   issue ledger a complete shop-floor or ERP connection.
10. Publish honest limits and phase timings. Keep the public benchmark results
    conditional, and keep browser storage, hardware acceptance and folded-fabric
    limits visible. There is no general win claim against commercial systems.

## Public commercial references

Re-read on 20 October 2026. These describe supplier capabilities, not independent
quality or speed comparisons. No code, proprietary specifications or samples
were downloaded or used. Recommendations are this audit's engineering judgment.

- [Tukatech SMARTmark](https://tukatech.com/products/smartmark): centralized Net-Q,
  material planning and costing/report workflows support a UI for BETA's existing
  queue and connected cut planning. Testimonials are not comparative evidence.
- [Gerber AccuPlan workflow](https://gerber-help.lectra.com/AccuPlan/Lesson_3.htm):
  fabric-specific spreading/cutting settings, width alternatives and marker
  reuse support persistent production settings and reviewed alternatives.
- [Optitex O/26](https://optitex.com/optitex-o26-release/): batch workflow controls
  and repeatable PDF/DXF generation support unattended cut-package generation.
  Its nesting-speed wording does not establish an equal-rule BETA comparison.
- [Lectra Valia Fashion](https://www.lectra.com/en/library/discover-main-features-of-valia-fashion-fashion-manufacturing-solution-transforming-cutting):
  connected cutting-room preparation supports continuity from planning to
  production. BETA has file exchange, not a verified live ERP connector.

## Execution record

On the merged UX/Pattern2 tree, portable Rust passed **373 tests**, with nine
ignored and zero failures. Native desktop passed **29 tests**, with zero failed
or ignored. Vitest passed **125 tests in 16 files**. Web, desktop and marker-wasm
typechecks passed. The release helper has three passing Node tests. All three
WASM modules and the production web app are freshly built. Strict Clippy passed
for the entire workspace and all targets with warnings denied. Final production
browser and HTTP checks also passed; exact commands and evidence are below.

### Workflow evidence

| Workflow | Executed evidence | Remaining limit |
| --- | --- | --- |
| Marker editor | Real original DXF import/nest/validate through CLI; current WASM editor tool smoke passes with 8 placements, 21 shortcuts and zero rule/clearance errors; eight in-scope screens fit 390 CSS pixels | Physical plot/cut calibration remains open |
| Cut plan | Real Rust planning/pipeline; headless infeasible 10 mm-width order names M:2 shortfall; production-asset happy path captured | Short candidate budgets check correctness, not optimality |
| Compare | Original PLT export, actual form import, short re-nest, both validations and Open current CAD marker | Synthetic square only; grain assumptions remain visible; no commercial efficiency claim |
| Rolls | Thirteen real API/browser checks including uppercase JSON, all policy controls, in-flight response rejection, audited allocation and handoff | No physical inspection instrument or automatic defect repair |
| Bundles / measured report | Reselected audited source makes real bundles; 42 mm planned/actual use, two physical pieces, zero variance | Synthetic measurements, not a production shift |
| Tickets / exports | Nine CLI export formats; own-PLT round trip validated; XLSX ZIP/XML checks found no error cells; ticket QR decoded to exact order/lay/part/plies/quantity | Short-marker PDF clipping remains open; physical print and machine scale untested |
| Floor | Current-WASM intake, spreading, actuals, re-cut, sewing issue and duplicate-scan smoke passes; 0.082 m use reconciles; four exports contain real bytes; all 13 manual sections and three draft languages checked | Physical stock, replacement and sewing receipt remain operator/system responsibilities |
| Reports / Help | Both screens open at desktop and phone widths; report links and source screens inspected | Complete buyer/monthly report-pack UI and print hardware remain open |
| Queue | Add/status/pause/resume/cancel/run complete with the original public fixture | Production throughput curves not measured in this short audit |
| Bench CLI | Jobs and replay pass; audit/finalize pass with nine live Windows CPU samples recorded during a fresh original-square run | No new Lectra or academic score comparison; missing local academic data fails without downloading |
| Desktop | 29 native tests plus six TypeScript autosave/project tests pass; eight DOM feedback checks pass with mocked OS/clipboard, including all three actions and failure paths | No desktop window or installer launch; actual browser/mail association remains a target-PC acceptance check |

### CLI and performance evidence

`deploy/audit-cli.py` completed **81/81 expected outcomes**, including 37 help
checks and one expected missing-local-data failure. Additional checks cover the
re-cut CLI, jobs/replay, monitored audit/finalize and three standalone Floor
operations. An early harness submission omitted the required nesting time; it
was corrected before the complete 81-command run. That harness error is not a
product failure.
An initial benchmark load-audit/finalize call without telemetry also failed as
expected. Its vague missing-file message was fixed. The successful monitored run
has real CPU samples; no load evidence was invented to make the command pass.

The final release binary is 21,127,271 bytes, SHA-256
`f718d72f8a200cc8d34867cb9bb610c49a74879f7712a56d4625a6fd79fe1649`.
Its PE timestamp is zero. The remapped build and independent binary grep found
zero native/slash worktree or Windows-user path matches. The helper also checks
UTF-16 at both alignments and Cargo-home paths. A separate target-directory scan
also found zero matches. All three final WASM modules pass the same user,
Cargo-home and worktree scan. The release build uses `--locked`; the GNU PE
timestamp is zero. A two-clean-directory bit-for-bit comparison and deterministic
archive/signing checks remain documented release gates, not claimed results.

Observed final release timings on the shared laptop; caches and background load
were not controlled:

| Probe | Command wall time | Scope |
| --- | ---: | --- |
| New `serve` process to successful health reply | 0.510 s | File caches not controlled; not a clean-PC cold-start guarantee |
| Original DXF shirt, 0.2 s search | 0.823 s | Includes process startup, construction, checking and I/O |
| Public FreeSewing Simon, 1 s search, two workers | 1.979 s | 114 placements, 7.894248 m, 82.7018%, independently valid |
| Simon fixed + warm-start, 1 s search | 1.161 s | Exact lock retained; complete checked incumbent kept |
| 1,000 original 20 mm squares: independent validate | 0.188 s | Correctness/scaling probe, not a representative garment benchmark |
| Same 1,000-square job: SVG / XLSX export | 0.130 / 0.130 s | Includes process startup and independent validation |
| WASM bump on 300 original rectangles, 100 measured calls | 2.5 ms median, 3.5 ms p95, 5.7 ms maximum | JSON/WASM apply only; excludes drawing and full live inspection |
| Small original order-to-cut pipeline | 0.226 s | Tiny functional scenario; not factory-size throughput |

These are single observations. An earlier loaded run took 3.051 s for Simon and
0.554 s for the 1,000-square validation. No nesting algorithm was changed by this
audit; these differences are not an isolated speedup result. The search budget is not total command time.
They are not minimum-PC requirements or comparisons with commercial hardware.
The full public Lectra results in the shared brief remain unchanged, in percentage
points (pp): mean
-0.73 pp at 60 s and +0.42 pp at recorded time, with negative medians in both.
No general win claim is made.

CSV safety changes prefix formula-leading text cells with an apostrophe, matching
the existing Floor/report-pack policy. Numeric cells remain numeric. JSON keeps
exact identifiers and is the better exchange format when an importer must retain
the original text byte for byte.

The A4 ticket rendered legibly and its QR decoded correctly. Poppler reported
missing local Symbol/ArialUnicode display fonts, but the inspected original ASCII
ticket rendered. The original short marker PDF exposes the clipping finding in
the priority table. Visual previews remain in the local audit evidence folder;
they are not polished release screenshots or hardware acceptance.

The original audit fixes add no dependency. The approved feedback follow-up below
adds the official Rust opener and reuses its existing glob matcher for tests;
no npm package is added. No owned engine, Pattern/Digitize, site/brand or CSS
source is changed by the audit, apart from the explicitly requested mvp merges.
No system or service installation was performed. Both final production browser
runs recorded zero external page requests and zero uncaught exceptions.

### Final commands and evidence

Cargo commands ran from `marker/`, with the assigned external target and three
build jobs. npm used the assigned cache. Node commands ran from the repository root.

| Command / check | Result |
| --- | --- |
| `cargo test --locked --workspace --exclude beta-cutting-room --no-fail-fast -- --test-threads=2` | 373 passed; 0 failed; 9 ignored |
| `cargo test --locked -p beta-cutting-room --no-fail-fast -- --test-threads=2` | 29 passed; 0 failed/ignored; no app window |
| `cargo clippy --locked --workspace --all-targets -- -D warnings` | Passed, including desktop |
| `npx vitest run marker` | 125 passed in 16 files |
| `npm run typecheck -w @beta/marker-wasm -w @beta/marker-web -w @beta/marker-desktop` | All three passed |
| `node --test marker/deploy/release-build.test.mjs` | 3 passed |
| Public benchmark package lock | Aligned its existing 0.4.0 manifest version; `cargo metadata --locked --offline --no-deps` passed |
| Release wrapper: Marker, Pattern, Floor WASM; CLI; production web build | Passed; all four binary path scans passed |
| `deploy/audit-cli.py` on final CLI | 81/81 expected outcomes, including 37 help checks and one expected local-data failure |
| `deploy/verify-server.py` on final CLI / web assets | Help/version, bind/token, health, static index and traversal checks passed; child reaped |
| `deploy/browser-smoke.mjs` on final CLI / web assets | 7/7; zero external page requests or uncaught exceptions |
| `deploy/app-screenshots.mjs` on final CLI / web assets | 13/13; six screenshots inspected; zero external page requests or uncaught exceptions |
| Final HTTP/CLI error probe | Eight HTTP cases and two missing-evidence errors passed; `/?welcome` 200, `/api/health?welcome` 400, traversal still 404 |
| Source UI regressions on rebuilt WASM and merged UX | Editor, Floor, 13 roll/bundle checks, partial order, Compare, desktop feedback DOM, live top bar and phone More panel passed |

Evidence stays outside git at `$CARGO_TARGET_DIR/audit-evidence/`:

- `rust-portable-final.log`, `native-final.log`, `clippy-final.log`, `release-final.log`.
- `cli-final/{commands,summary}.json`, `final-wire-checks.json`, `browser-workflows.json`.
- `screens/app-screenshots-report.json` and `screens/{marker-editor,pattern,cut-plan,rolls,spreading,bundles}.png`.
- `browser-token-final/report.json` and its token-dialog captures.
- `final-binary-scan.json` with executable/WASM sizes, hashes and the zero PE timestamp.
- `cli-release-verified/{ticket-preview,marker-preview}.png`, original public export files and extra CLI/Floor checks.
- `bench-monitored/conditions.json` and audit/finalization evidence; all nine CPU samples were measured during that public-fixture run.
- `licence-audit.json` and `THIRD-PARTY-NOTICES.txt` for the lead's release packaging.

The short-marker PDF previews document an open bug and are not claimed flawless.
No clean Windows 10 installation, older-glibc runtime, physical plot/cutter or
printer, or real external-app association test was performed. No push or publish
operation was performed.
All audit-owned HTTP/Vite listeners and browser previews were stopped after the
checks. Generated Python bytecode and the unused temporary build manifest were
removed with explicit, verified paths. Only public/synthetic evidence remains.

## Approved follow-up: desktop feedback links

The lead authorized this extra task during the audit. Merge `ebe07fe` from
`marker/mvp` brings in the feedback module; `176e453` brings the redesigned top
bar and the other lead-approved work. The desktop now uses the official
`tauri-plugin-opener` 2.7.2 for GitHub issues, email and discussions. Browser/server
actions retain their existing handlers. Failed native opening copies the full
link; a blocked clipboard leaves manual copy/save available.

The main capability grants only `opener:allow-open-url` for the exact BETA
repository descendants and the specified maintainer mailto prefix. No opener
default permission, path/reveal permission or custom application is granted.
Automatic hyperlink interception is disabled. A frontend check also rejects
other hosts/repos, local paths, traversal and extra recipients. Tests use mock
IPC or the real scope matcher, so they do not launch an external app.

No npm dependency was added. The existing licence gate passed 377 Windows Rust
packages. Full inventory regeneration, including the existing desktop build tool,
found **512 selected packages**, **97 metadata-only packages**, and **zero
review-required selected packages**. New opener/open crates and the metadata-only
is-docker/is-wsl crates have allowed MIT/Apache terms. The existing unmodified
jagua-rs exception is unchanged. `NOTICE`, the inventory and the full generated
release notice bundle are updated. Native, unit and headless checks are recorded
in the execution matrix above.
