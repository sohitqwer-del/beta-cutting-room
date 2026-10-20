/** Where feedback goes. Change these in one place only. Nothing is sent automatically:
 * the user opens a link, an email or a file themselves. */
export const FEEDBACK_CONFIG = {
  repository: 'https://github.com/sohitqwer-del/beta-cutting-room',
  newIssue: 'https://github.com/sohitqwer-del/beta-cutting-room/issues/new',
  discussions: 'https://github.com/sohitqwer-del/beta-cutting-room/discussions',
  email: 'sohitqwer@gmail.com',
  /** Browsers and GitHub accept long links, but proxies and some mail apps do not. Keep links short. */
  maxIssueUrlBytes: 7000,
  maxMailtoBytes: 1900,
  labels: { bug: 'bug', missing: 'missing-feature', idea: 'feedback' },
} as const;

export const STORAGE_KEYS = {
  welcome: 'beta-welcome-dismissed',
  ask: 'beta-feedback-ask-version',
} as const;
