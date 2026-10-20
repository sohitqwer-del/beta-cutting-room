import { STORAGE_KEYS } from './config';

/** First-run and "ask once" memory. Local to this computer; never sent anywhere. */
export interface KeyStore { getItem(key: string): string | null; setItem(key: string, value: string): void }

const read = (store: KeyStore | null, key: string): string | null => { try { return store?.getItem(key) ?? null; } catch { return null; } };
const write = (store: KeyStore | null, key: string, value: string): void => { try { store?.setItem(key, value); } catch { /* Storage blocked: remember for this session only. */ } };
const session = new Set<string>();

/** Show the welcome once. Automated test browsers skip it unless asked, so other screen checks are not covered by it. */
export function shouldShowWelcome(store: KeyStore | null, automated = false): boolean {
  return !automated && !session.has(STORAGE_KEYS.welcome) && !read(store, STORAGE_KEYS.welcome);
}
export function rememberWelcome(store: KeyStore | null, date = new Date()): void {
  session.add(STORAGE_KEYS.welcome);
  write(store, STORAGE_KEYS.welcome, date.toISOString().slice(0, 10));
}

/** The gentle "How did that go?" ask appears at most once for each app version. */
export function shouldAsk(store: KeyStore | null, version: string, automated = false): boolean {
  return !automated && !session.has(`${STORAGE_KEYS.ask}:${version}`) && read(store, STORAGE_KEYS.ask) !== version;
}
export function rememberAsk(store: KeyStore | null, version: string): void {
  session.add(`${STORAGE_KEYS.ask}:${version}`);
  write(store, STORAGE_KEYS.ask, version);
}

/** A first successful nest or export. Matches the app's own status messages; paths after "Saved:" are not kept. */
export function isMilestone(message: string): boolean {
  const text = message.trim();
  return /^Nesting finished\b/.test(text) || (/^Saved: /.test(text) && !/cancel/i.test(text));
}

/** Testing only: forget the in-memory session flags. */
export function resetSessionMemory(): void { session.clear(); }
