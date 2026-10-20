# GitHub files for the public repository

Copy `ISSUE_TEMPLATE/` to `.github/ISSUE_TEMPLATE/` in the public repository.

Before publishing:

1. Create the labels `bug`, `missing-feature` and `feedback`. The templates add them automatically.
2. Turn on GitHub Discussions. The app's Help screen and About box link to `/discussions`.
3. Keep `blank_issues_enabled: true`. The app's Feedback button opens a prefilled blank issue (title, body and label in the link). Without blank issues, GitHub drops the prefilled text.
4. Check that `SECURITY.md` is in the repository root or in `.github/`, so the security link in `config.yml` works.

## How the in-app Feedback button sends reports

Nothing is sent automatically. The user chooses one way:

| Way | Browser app | Desktop app |
| --- | --- | --- |
| GitHub issue | Opens a prefilled issue in a new tab (link kept under 7 KB) | Opens the default browser; copies the link if opening fails |
| Email | Opens the mail app (`mailto:`) | Opens the default mail app with the report; copies the `mailto:` link if opening fails |
| Join the discussion | Opens GitHub Discussions in a new tab | Opens the default browser; copies the link if opening fails |
| Copy report | Copies the text | Copies the text |
| Save report | Downloads a `.md` file | Native save dialog |

The desktop uses the official `tauri-plugin-opener` **2.7.2** (Apache-2.0 OR MIT).
The existing `window.__TAURI__.core.invoke` transport calls
`plugin:opener|open_url`; no npm opener package is added. Only an explicit click
opens an external app. The report still needs to be submitted in GitHub or the
mail app; opening a link does not send it automatically.

The `main` capability grants only `opener:allow-open-url`, scoped to:

```text
https://github.com/sohitqwer-del/beta-cutting-room/**
mailto:sohitqwer@gmail.com*
```

There is no opener default permission, path-opening permission, reveal-file
permission, arbitrary app selection, or automatic interception of page links.
The frontend also checks the exact repository/recipient and rejects local paths,
other hosts, path traversal and extra mail recipients. Keep the capability and
frontend configuration aligned when changing the contact. The desktop WebView's
external-navigation block remains in place.

Failure to open falls back to copying the complete link. If copying also fails,
the report preview and Copy report / Save report remain available. Long reports
still use bounded links and retain the full report for copying or saving.

Run `npx vitest run marker` for the platform/fallback regressions, and
`cargo test --manifest-path marker/Cargo.toml -p beta-cutting-room` for the native
scope tests. Tests use mocks or inspect the real scope with the plugin's glob
matcher; they do not launch the browser or mail app. Re-run the existing desktop
licence gate and full inventory after dependency changes, and ship the generated
`THIRD-PARTY-NOTICES.txt` with the release package.

`apps/web/src/feedback/opener-smoke.mjs` additionally exercises the real feedback
DOM in an isolated same-origin frame. It checks the redesigned top-bar insertion,
all three native actions, copy-link fallback and a blocked clipboard. Its OS
opener and clipboard are test doubles; no external app or message is opened.
The ordinary browser preview was also checked through the desktop-width top bar
and the phone's More panel.

For headless first-run checks, `?welcome` and `?feedback-ask` are page flags.
The static server ignores such query flags and asset cache-busters. API query
syntax and duplicate-parameter checks stay strict.

The repository URL and email address are set in one place: `marker/apps/web/src/feedback/config.ts`.
