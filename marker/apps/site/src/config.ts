// Site settings the lead fills in before publishing. `null` shows "Coming soon".

export const site = {
  name: 'BETA Cutting Room',
  version: '0.8.0',
  /** One line that says what BETA is and who it is for (hero, social cards, page descriptions). */
  tagline: 'Free, open-source cutting-room software for garment factories: patterns, markers and cut plans, in your browser.',
  /** Public source repository, e.g. "https://github.com/<org>/project-beta". */
  repoUrl: 'https://github.com/sohitqwer-del/beta-cutting-room' as string | null,
  /** Public address of this website, used for social preview tags. */
  siteUrl: 'https://sohitqwer-del.github.io/beta-cutting-room/' as string | null,
  /** Branch used for links into the repository. */
  repoBranch: 'main',
  /** Static app beside the website; no server or account is needed. */
  webAppUrl: 'app/' as string | null,
  /** The app's 60-second sample tour (real engine, sample T-shirt): the first thing a new visitor should see. */
  tourUrl: 'app/#/tour' as string | null,
  /** Contact address for sample markers and questions. */
  contactEmail: 'sohitqwer@gmail.com' as string | null,
  /** Release candidates. The lead publishes them after verification. Update URL, size and SHA-256 together. */
  downloads: {
    windows: {
      url: 'https://github.com/sohitqwer-del/beta-cutting-room/releases/download/v0.8.0/BETA-Cutting-Room_0.8.0_x64-setup.exe' as string | null,
      file: 'BETA-Cutting-Room_0.8.0_x64-setup.exe',
      size: '11.65 MiB',
      sha256: '2cf736d93aaf2e7ed395fc738f1d6fe0b451b9f00884ddba67e31d567f2a2307' as string | null,
    },
    portable: {
      url: 'https://github.com/sohitqwer-del/beta-cutting-room/releases/download/v0.8.0/BETA-Cutting-Room_0.8.0_windows-x64-portable.zip' as string | null,
      file: 'BETA-Cutting-Room_0.8.0_windows-x64-portable.zip',
      size: '14.41 MiB',
      sha256: '179547aa43e990d60f3eb98e3440d11a9412f13a03556943c62821dfbb2f1116' as string | null,
    },
    server: {
      url: 'https://github.com/sohitqwer-del/beta-cutting-room/releases/download/v0.8.0/BETA-Cutting-Room_0.8.0_linux-x86_64.tar.gz' as string | null,
      file: 'BETA-Cutting-Room_0.8.0_linux-x86_64.tar.gz (Linux server, glibc 2.31 target; tested with 2.31 and 2.43)',
      size: '13.26 MiB',
      sha256: '6ac89e9fb44e92f166b4e2a62713e237bd3ceddfa2d5eccab6953cf35492e4dc' as string | null,
    },
  },
  /**
   * Demo video. The site plays `public/assets/video/beta-08-walkthrough.mp4` when that file exists (copy it there
   * before building; it is not committed). Otherwise, set `demoVideoUrl` (for example a GitHub release asset):
   * the home page then shows the poster as a link to it, so the page itself still loads nothing remote.
   * Made by `marker/deploy/video/make-video.mjs`. Keep `length` and `size` in step with the file.
   */
  demoVideo: {
    file: 'assets/video/beta-08-walkthrough.mp4',
    demoVideoUrl: null as string | null,
    poster: 'assets/video/beta-08-poster-1280x720.png',
    captions: 'assets/video/beta-08-walkthrough.vtt',
    length: '3 min 9 s',
    size: '6.4 MiB',
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
