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
| GitHub issue | Opens a prefilled issue in a new tab (link kept under 7 KB) | Copies the prefilled link |
| Email | Opens the mail app (`mailto:`) | Copies the email address |
| Copy report | Copies the text | Copies the text |
| Save report | Downloads a `.md` file | Native save dialog |

The desktop window blocks external navigation, and the Tauri opener plugin is not a dependency. To open links directly in the desktop app later, add `tauri-plugin-opener` (MIT or Apache-2.0) with `opener:allow-open-url` limited to `https://github.com/sohitqwer-del/*` and `mailto:*`.

The repository URL and email address are set in one place: `marker/apps/web/src/feedback/config.ts`.
