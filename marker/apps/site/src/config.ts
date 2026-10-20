// Site settings the lead fills in before publishing. `null` shows "Coming soon".

export const site = {
  name: 'BETA Cutting Room',
  version: '0.5.0',
  /** Public source repository, e.g. "https://github.com/<org>/project-beta". */
  repoUrl: 'https://github.com/sohitqwer-del/beta-cutting-room' as string | null,
  /** Public address of this website, used for social preview tags. */
  siteUrl: 'https://sohitqwer-del.github.io/beta-cutting-room/' as string | null,
  /** Branch used for links into the repository. */
  repoBranch: 'main',
  /** Static app beside the website; no server or account is needed. */
  webAppUrl: 'app/' as string | null,
  /** Contact address for sample markers and questions. */
  contactEmail: 'sohitqwer@gmail.com' as string | null,
  downloads: {
    windows: {
      url: 'https://github.com/sohitqwer-del/beta-cutting-room/releases/download/v0.5.0/BETA-Cutting-Room_0.5.0_x64-setup.exe' as string | null,
      file: 'BETA-Cutting-Room_0.5.0_x64-setup.exe',
      size: '9.18 MiB',
      sha256: '4ceb1058ba57eb244594900be7b472005b279a299a2a670038c18ffa1581f2ea' as string | null,
    },
    portable: {
      url: 'https://github.com/sohitqwer-del/beta-cutting-room/releases/download/v0.5.0/BETA-Cutting-Room_0.5.0_windows-x64-portable.zip' as string | null,
      file: 'BETA-Cutting-Room_0.5.0_windows-x64-portable.zip',
      size: '9.90 MiB',
      sha256: 'bb67647fa1b5f35dccfb75cc04f96077166e44788e50a04d05b78d127c60d566' as string | null,
    },
    server: {
      url: 'https://github.com/sohitqwer-del/beta-cutting-room/releases/download/v0.5.0/BETA-Cutting-Room_0.5.0_linux-x86_64.tar.gz' as string | null,
      file: 'BETA-Cutting-Room_0.5.0_linux-x86_64.tar.gz (Linux server, glibc 2.31 target; tested with 2.31 and 2.43)',
      size: '8.91 MiB',
      sha256: '4854f75bcc3293939d554e59f0274b32d5c975a3cb75888a6b917ee842584694' as string | null,
    },
  },
  /**
   * Demo video. The site plays `public/assets/video/beta-demo-en.mp4` when that file exists (copy it there
   * before building; it is not committed). Otherwise, set `demoVideoUrl` (for example a GitHub release asset):
   * the home page then shows the poster as a link to it, so the page itself still loads nothing remote.
   * Made by `marker/deploy/video/make-video.mjs`. Keep `length` and `size` in step with the file.
   */
  demoVideo: {
    file: 'assets/video/beta-demo-en.mp4',
    demoVideoUrl: null as string | null,
    poster: 'assets/video/beta-demo-poster.jpg',
    captions: 'assets/video/beta-demo-en.vtt',
    length: '3 min 45 s',
    size: '15 MB',
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
