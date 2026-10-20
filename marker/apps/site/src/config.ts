// Site settings the lead fills in before publishing. `null` shows "Coming soon".

export const site = {
  name: 'BETA Cutting Room',
  version: '0.4.0',
  /** Public source repository, e.g. "https://github.com/<org>/project-beta". */
  repoUrl: 'https://github.com/sohitqwer-del/beta-cutting-room' as string | null,
  /** Public address of this website, used for social preview tags. */
  siteUrl: 'https://sohitqwer-del.github.io/beta-cutting-room/' as string | null,
  /** Branch used for links into the repository. */
  repoBranch: 'main',
  /** Contact address for sample markers and questions. */
  contactEmail: 'sohitqwer@gmail.com' as string | null,
  downloads: {
    windows: {
      url: 'https://github.com/sohitqwer-del/beta-cutting-room/releases/download/v0.4.0/BETA-Cutting-Room_0.4.0_x64-setup.exe' as string | null,
      file: 'BETA-Cutting-Room_0.4.0_x64-setup.exe',
      size: '7.86 MiB',
      sha256: '6066499c315cda9c60cf34b01c0398de119dd338cd9b5003ad8e94a7f1804df7' as string | null,
    },
    portable: {
      url: 'https://github.com/sohitqwer-del/beta-cutting-room/releases/download/v0.4.0/BETA-Cutting-Room_0.4.0_windows-x64-portable.zip' as string | null,
      file: 'BETA-Cutting-Room_0.4.0_windows-x64-portable.zip',
      size: '8.42 MiB',
      sha256: 'c5ae4817fe7913b61089c91cc5dab66f2e663d4bd5dc522ce4c4a2f6ef65c0ca' as string | null,
    },
    server: {
      url: 'https://github.com/sohitqwer-del/beta-cutting-room/releases/download/v0.4.0/BETA-Cutting-Room_0.4.0_linux-x86_64.tar.gz' as string | null,
      file: 'BETA-Cutting-Room_0.4.0_linux-x86_64.tar.gz (Linux server, needs glibc 2.43 or newer)',
      size: '7.92 MiB',
    },
  },
  /** Where people report problems and send ideas. `null` hides a link. */
  feedback: {
    bug: 'https://github.com/sohitqwer-del/beta-cutting-room/issues/new?labels=bug' as string | null,
    missing: 'https://github.com/sohitqwer-del/beta-cutting-room/issues/new?labels=missing-feature' as string | null,
    idea: 'https://github.com/sohitqwer-del/beta-cutting-room/issues/new?labels=feedback' as string | null,
    email: 'mailto:sohitqwer@gmail.com' as string | null,
  },
};

/** The three feedback links shown in the "Help us improve" section and the footer. */
export function feedbackLinks(): { key: 'bug' | 'missing' | 'idea'; label: string; text: string; url: string }[] {
  const all = [
    { key: 'bug' as const, label: 'Report a bug', text: 'Something broke or gave a wrong result? Tell us what you did and what happened.', url: site.feedback.bug },
    { key: 'missing' as const, label: 'Tell us what is missing', text: 'A step your cutting room needs that BETA cannot do yet.', url: site.feedback.missing },
    { key: 'idea' as const, label: 'Share feedback', text: 'Anything else: ideas, confusing screens, words that are hard to read.', url: site.feedback.idea },
  ];
  return all.filter((l): l is typeof l & { url: string } => typeof l.url === 'string' && l.url.length > 0);
}

/** Link to a file in the source repository, or null when no public repository is set. */
export function repoLink(path: string): string | null {
  if (!site.repoUrl) return null;
  return `${site.repoUrl.replace(/\/$/, '')}/blob/${site.repoBranch}/${path}`;
}
