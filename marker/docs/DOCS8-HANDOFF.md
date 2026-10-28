# Documentation handoff for 0.8

Scope: documentation and repository proposals only. Public version remains 0.7;
development additions are marked experimental/unreleased. No app code, build,
push, publication or GitHub setting change is part of this work.

## New files

- [Public-root README](public-root/README.md): plain platform introduction, three screenshot placeholders, personas, grouped features, historical benchmark summary, install/privacy/status and credits.
- [Features](FEATURES.md): current screen map plus native/research tools, linked to source contracts and evidence; unreleased work is marked.
- [FAQ](FAQ.md): 36 questions, including the proposed shared top eight.
- [Getting started](GETTING-STARTED.md): three 10-minute learning guides with expected results and stop conditions.
- [Roadmap](ROADMAP.md): now/next/later, without delivery promises.
- [Contributing](public-root/CONTRIBUTING.md): public-export-specific instructions, Rust calculation boundary, checks, rights and privacy.
- [Code of Conduct](public-root/CODE_OF_CONDUCT.md): official Contributor Covenant 2.1 text, credited, with the permitted contact configured.
- [GitHub settings proposal](public-root/GITHUB-SETTINGS.md): description, topics, Discussions, labels and two pinned-issue drafts.
- [Public-root security proposal](public-root/SECURITY.md): mirrors the existing policy with current version scope and public-root link paths.

Copy the root proposals to the public repository root. Links in those proposals
are deliberately relative to that destination. Keep the proposals in this docs
folder as well: the FAQ links to the proposed contribution guide. The public
root is assembled separately from the monorepo root.

## Stale statements fixed

| Files | Change |
|---|---|
| Marker README | Lead with local browser use; add guide/FAQ/feature/roadmap links and distinguish unreleased work. |
| Pattern README | Replace unsupported market-wide absence claim and future-tense file reading with supported Seamly subset and limits. |
| INSTALL | Add no-install browser route; remove the assumed 0.7 Linux archive instruction because the release record says none was produced. |
| PRIVACY, SELF-HOSTING | Remove stale 0.4 title/version; distinguish static local work, optional API computation and backup scope. |
| Marker SECURITY and root proposal | Replace stale “0.4 is public” with public 0.7/development 0.8; retain independent-review and maintenance limits. |
| RELEASE-CHECKLIST | Replace hard-coded 0.4 version gate with candidate version; require all four WASM modules, including Cutting-room. |
| AUDIT-0.7, CUTTING-ROOM-STATUS, IMPROVEMENTS-2026-10-20 | Add explicit supersession notes for browser parity, backup, layer meaning and new bounded cost/area screens; keep original evidence intact. |
| RELEASE-0.4, 0.5, 0.6 | Mark historical scope; direct current browser claims to 0.7 and prevent reuse of old performance as 0.8 evidence. |
| QA-2026-10-19 | Link later QA-006 and browser/export retest evidence without rewriting the old observation. |
| PLAN, V04-RESEARCH, CUTTING-ROOM-PLAN | Mark proposal/backlog scope; acknowledge that Pattern now exists. |
| benchmarks | Date and pin the headline evidence; separate selected October experiments from a full 0.8 Lectra rerun. |
| Pattern PARITY, PRODUCTION-TOOLS | Explain later supported linked derivatives/notch projection; preserve the earlier tool-specific findings. |
| PAPER-PATTERNS-MSME | Remove one machine-specific external evidence path. |

Historical benchmark figures, tests and package hashes were not rewritten.
The public-root README also avoids the private one-style Gemini comparison;
it adds no new factory evidence or superiority claim.

## Verification

The relative-link check walks Markdown under Marker and Pattern, excluding
private, dependency and build folders. It ignores fenced code and external URLs.
For root proposals it checks a virtual public root: proposed files, worktree
export paths, then existing public-root metadata. It checks file existence,
not GitHub fragments, external uptime or dynamic app routes.

Final check: **151 Markdown files, 329 relative links, zero unresolved file
links**, with exactly the three requested README screenshot paths recorded
separately. The changed files are Markdown only.

The email/path scan over that same scope found **zero disallowed emails or
machine-specific user/project paths** after the one removal. Only
sohitqwer@gmail.com is used. Manual review found no factory identities in the
new text; vendor, dataset and public fixture names are attribution, not factory
disclosures. No private directory was accessed. This text scan cannot prove
rights or identify every possible confidential name in historical prose.

Contributor Covenant text was fetched from its official 2.1 Markdown source
and compared: **exact match after reversing only the contact substitution**.
Its own attribution and source links are retained. `git diff --check` passed.
No application tests or screenshots were produced for these prose changes.

## Open decisions for the lead

1. Add the three real, cleared screenshots at `docs/images/pattern-workspace.png`, `marker-workspace.png` and `cutting-room.png` in the public export. These are deliberate missing assets until filled.
2. Merge the final bench8 evidence and review every headline number before changing release labels. Current numbers explicitly remain historical. Shared requests describe additive 0.8 comparison data; no finished refresh was available during this doc review.
3. Confirm Linux package availability and Windows acceptance on the assembled release. Do not rename an older package as 0.8.
4. Confirm who handles conduct and security reports at the listed contact, and whether Discussions/private vulnerability reporting are enabled. These proposals do not enable them.
5. Confirm launch-ui's final navigation, sample-tour wording and top-eight FAQ agreement. The proposed wording was sent early through the shared requests file; no launch-ui acknowledgement was available at handoff.
6. Copy the root security proposal too: the existing public-root SECURITY still says 0.4. Keep all root links relative to the public export, not the unrelated monorepo root.
7. Read the independent qa8 report before presenting new features as release-accepted. These docs describe code and reported coverage, not a fresh factory trial.
