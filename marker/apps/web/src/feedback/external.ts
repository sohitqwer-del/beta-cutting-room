import { invoke, isDesktop } from '../transport';
import { FEEDBACK_CONFIG } from './config';

export type LinkOutcome = 'browser' | 'opened' | 'copied' | 'failed';

/** Defence in depth; native opener capabilities also enforce the configured URL scopes. */
export function allowedFeedbackLink(value: string): boolean {
  if (/[\u0000-\u0020\u007f\\]/.test(value)) return false;
  try {
    const url = new URL(value);
    if (url.protocol === 'https:') {
      const prefix = `${FEEDBACK_CONFIG.repository}/`;
      return value.startsWith(prefix) && url.href.startsWith(prefix)
        && !url.username && !url.password && !url.port
        && !/%2e|%2f|%5c/i.test(url.pathname);
    }
    return url.protocol === 'mailto:' && url.pathname === FEEDBACK_CONFIG.email
      && [...url.searchParams.keys()].every(key => ['subject', 'body'].includes(key));
  } catch { return false; }
}

/** Browser callers retain their existing new-tab/mail handler. No npm binding is needed. */
export async function openFeedbackLink(url: string, copy: (text: string) => Promise<boolean>): Promise<LinkOutcome> {
  if (!isDesktop()) return 'browser';
  if (!allowedFeedbackLink(url)) return 'failed';
  try {
    // This is the official plugin command, so Tauri applies the scoped capability.
    await invoke<void>('plugin:opener|open_url', { url, with: null });
    return 'opened';
  } catch {
    try { return await copy(url) ? 'copied' : 'failed'; } catch { return 'failed'; }
  }
}
