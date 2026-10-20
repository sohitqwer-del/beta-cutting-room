// Site settings the lead fills in before publishing. `null` shows "Coming soon".

export const site = {
  name: 'BETA Cutting Room',
  version: '0.4.0',
  /** Public source repository, e.g. "https://github.com/<org>/project-beta". */
  repoUrl: 'https://github.com/sohitqwer-del/beta-cutting-room' as string | null,
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
    server: {
      url: 'https://github.com/sohitqwer-del/beta-cutting-room/releases/download/v0.4.0/BETA-Cutting-Room_0.4.0_linux-x86_64.tar.gz' as string | null,
      file: 'BETA-Cutting-Room_0.4.0_linux-x86_64.tar.gz (Linux server, needs glibc 2.43 or newer)',
    },
  },
};

/** Link to a file in the source repository, or null when no public repository is set. */
export function repoLink(path: string): string | null {
  if (!site.repoUrl) return null;
  return `${site.repoUrl.replace(/\/$/, '')}/blob/${site.repoBranch}/${path}`;
}
