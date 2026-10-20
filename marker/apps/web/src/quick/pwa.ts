import { isDesktop } from '../transport';

/** Offline install for phones. The service worker (public/sw.js) caches the built app and its WASM files from
 * this server only; it never caches /api/ calls and makes no other network requests.
 * Not used in the desktop app (files are local already) or in the Vite dev server. Browsers allow service workers
 * only on https:// or http://localhost, so a phone that opens `http://<shop PC>:7878` gets no offline copy. */
export interface InstallState { canInstall: boolean; offline: 'ready' | 'unsupported' | 'pending' }
type PromptEvent = Event & { prompt(): Promise<void>; userChoice: Promise<{ outcome: string }> };

export function offlineSupported(env: { desktop: boolean; prod: boolean; secure: boolean; hasWorker: boolean }): boolean {
  return !env.desktop && env.prod && env.secure && env.hasWorker;
}

export function installPwa(onChange: (state: InstallState) => void): { install(): Promise<boolean>; state(): InstallState } {
  let prompt: PromptEvent | null = null;
  const supported = offlineSupported({ desktop: isDesktop(), prod: import.meta.env.PROD, secure: window.isSecureContext, hasWorker: 'serviceWorker' in navigator });
  let current: InstallState = { canInstall: false, offline: supported ? 'pending' : 'unsupported' };
  const set = (patch: Partial<InstallState>) => { current = { ...current, ...patch }; onChange(current); };
  if (supported) {
    navigator.serviceWorker.register('./sw.js', { scope: './' })
      .then(registration => navigator.serviceWorker.ready.then(() => { void registration; set({ offline: 'ready' }); }))
      .catch(() => set({ offline: 'unsupported' }));
  }
  window.addEventListener('beforeinstallprompt', event => { event.preventDefault(); prompt = event as PromptEvent; set({ canInstall: true }); });
  window.addEventListener('appinstalled', () => { prompt = null; set({ canInstall: false }); });
  return {
    async install() {
      if (!prompt) return false;
      const p = prompt; prompt = null; set({ canInstall: false });
      await p.prompt();
      return (await p.userChoice).outcome === 'accepted';
    },
    state: () => current,
  };
}
