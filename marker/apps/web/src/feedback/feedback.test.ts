import { beforeEach, describe, expect, it } from 'vitest';
import { FEEDBACK_CONFIG, STORAGE_KEYS } from './config';
import { feedbackLanguages } from './i18n';
import { isMilestone, rememberAsk, rememberWelcome, resetSessionMemory, shouldAsk, shouldShowWelcome, type KeyStore } from './memory';
import { buildReport, detectOs, issueUrl, mailtoUrl, privacyHints, reportFileName, safeNumbers, SHORTENED_NOTE, TITLE_MAX,
  type FeedbackContext, type FeedbackInput, type MarkerNumbers } from './report';
import en from './locales/en.json';

const context: FeedbackContext = { screen: 'Marker editor', version: '0.4.0', platform: 'desktop', os: 'Windows', language: 'hi' };
const input: FeedbackInput = { kind: 'bug', title: 'Nest button does nothing', details: 'I pressed Nest marker and nothing happened.', steps: '1. Open demo\n2. Press Nest', contact: '' };
const bytes = (s: string) => new TextEncoder().encode(s).length;
class Store implements KeyStore { data = new Map<string, string>(); getItem(k: string) { return this.data.get(k) ?? null; } setItem(k: string, v: string) { this.data.set(k, v); } }
class BlockedStore implements KeyStore { getItem(): string | null { throw new Error('blocked'); } setItem(): void { throw new Error('blocked'); } }

describe('report text', () => {
  it('includes the type, screen, version, platform, OS and language', () => {
    const r = buildReport(input, context);
    expect(r.title).toBe('[Bug] Nest button does nothing');
    expect(r.body).toContain('**Type:** Bug report');
    expect(r.body).toContain('**Screen:** Marker editor');
    expect(r.body).toContain('- BETA Cutting Room 0.4.0');
    expect(r.body).toContain('- Desktop app');
    expect(r.body).toContain('- Operating system: Windows');
    expect(r.body).toContain('- App language: Hindi');
    expect(r.body).toContain('### Steps to repeat the problem\n1. Open demo\n2. Press Nest');
    expect(r.text.startsWith(`${r.title}\n\n${r.body}`)).toBe(true);
  });
  it('uses headings that fit each kind and leaves out empty optional parts', () => {
    const missing = buildReport({ kind: 'missing', title: 'AAMA export', details: 'We need AAMA files for our plotter.' }, context);
    expect(missing.title).toBe('[Missing] AAMA export');
    expect(missing.body).toContain('### What I need');
    expect(missing.body).not.toContain('How I would use it');
    expect(missing.body).not.toContain('Contact');
    const idea = buildReport({ kind: 'idea', title: '', details: 'Bigger buttons please\nfor the floor tablet', contact: 'Ravi' }, context);
    expect(idea.title).toBe('[Idea] Bigger buttons please');
    expect(idea.body).toContain('### Contact (given by the user)\nRavi');
  });
  it('falls back to a screen title and caps long titles', () => {
    expect(buildReport({ kind: 'idea', title: ' ', details: '' }, context).title).toBe('[Idea] Feedback about Marker editor');
    expect(buildReport({ ...input, title: 'x'.repeat(500) }, context).title.length).toBe(TITLE_MAX);
  });
  it('removes control characters and normalises line ends', () => {
    const r = buildReport({ ...input, details: 'a\r\nb\u0007c' }, context);
    expect(r.body).toContain('a\nbc');
  });
  it('names a dated Markdown file', () => {
    expect(reportFileName('missing', new Date(2026, 9, 20))).toBe('beta-feedback-missing-feature-2026-10-20.md');
  });
});

describe('privacy', () => {
  it('adds only whitelisted anonymous numbers', () => {
    const leaky = { pieces: 24, efficiencyPct: 81.234, timeLimitS: 30, name: 'Secret Buyer Style 42', path: 'C:\\Factory\\orders\\style.dxf', outline: [[0, 0], [100, 0], [100, 200]] } as unknown as MarkerNumbers;
    const r = buildReport(input, context, leaky);
    expect(r.body).toContain('- Pieces: 24');
    expect(r.body).toContain('- Efficiency: 81.2%');
    expect(r.body).toContain('- Time limit: 30 s');
    for (const secret of ['Secret Buyer', 'Factory', 'style.dxf', '100, 200', 'outline']) expect(r.text).not.toContain(secret);
  });
  it('drops impossible or non-numeric values', () => {
    expect(safeNumbers({ pieces: NaN, efficiencyPct: 140, timeLimitS: -1 })).toBeNull();
    expect(safeNumbers({ pieces: '12' as unknown as number, efficiencyPct: null, timeLimitS: Infinity })).toBeNull();
    expect(buildReport(input, context, null).body).not.toContain('Anonymous marker numbers');
  });
  it('never adds paths, geometry or file names on its own', () => {
    const r = buildReport(input, { ...context, platform: 'server' });
    expect(r.text).not.toMatch(/[A-Za-z]:\\|\/home\/|\/Users\/|\.dxf|\.betajob|\.plt/);
    expect(r.text).not.toMatch(/\[\s*-?\d+(\.\d+)?\s*,\s*-?\d+/);
  });
  it('warns about paths, file names and pasted geometry in the user text', () => {
    expect(privacyHints('Saved to C:\\Users\\Ravi\\Desktop\\x')).toContain('path');
    expect(privacyHints('see /home/factory/jobs/a')).toContain('path');
    expect(privacyHints('\\\\server\\share\\file')).toContain('path');
    expect(privacyHints('The file BUYER-SS26-jacket.dxf fails')).toContain('file');
    expect(privacyHints(Array.from({ length: 80 }, (_, i) => `${i}.5`).join(','))).toContain('geometry');
    expect(privacyHints('The nest button is slow on 12 pieces and 3 sizes.')).toEqual([]);
  });
});

describe('links', () => {
  it('builds a prefilled GitHub issue link with the right label', () => {
    const r = buildReport(input, context);
    const link = issueUrl(r, 'bug');
    expect(link.truncated).toBe(false);
    expect(link.url.startsWith(`${FEEDBACK_CONFIG.newIssue}?labels=bug&title=`)).toBe(true);
    const url = new URL(link.url);
    expect(url.searchParams.get('title')).toBe(r.title);
    expect(url.searchParams.get('body')).toBe(r.body);
    expect(new URL(issueUrl(r, 'missing').url).searchParams.get('labels')).toBe('missing-feature');
    expect(new URL(issueUrl(r, 'idea').url).searchParams.get('labels')).toBe('feedback');
  });
  it('keeps long reports under the URL limit and says so', () => {
    for (const details of ['word '.repeat(4000), 'हिन्दी तमிழ் '.repeat(2000), '😀'.repeat(5000)]) {
      const r = buildReport({ ...input, details }, context);
      const link = issueUrl(r, 'bug');
      expect(link.truncated).toBe(true);
      expect(bytes(link.url)).toBeLessThanOrEqual(FEEDBACK_CONFIG.maxIssueUrlBytes);
      expect(bytes(link.url)).toBeGreaterThan(FEEDBACK_CONFIG.maxIssueUrlBytes - 200);
      const body = new URL(link.url).searchParams.get('body')!;
      expect(body.endsWith(SHORTENED_NOTE.trim())).toBe(true);
      expect(body.startsWith('**Type:** Bug report')).toBe(true);
    }
  });
  it('builds a short mailto link with CRLF line breaks', () => {
    const r = buildReport(input, context);
    const link = mailtoUrl(r);
    expect(link.url.startsWith(`mailto:${FEEDBACK_CONFIG.email}?subject=`)).toBe(true);
    expect(link.url).toContain('%0D%0A');
    expect(link.truncated).toBe(false);
    const long = mailtoUrl(buildReport({ ...input, details: 'x'.repeat(5000) }, context));
    expect(long.truncated).toBe(true);
    expect(bytes(long.url)).toBeLessThanOrEqual(FEEDBACK_CONFIG.maxMailtoBytes);
  });
});

describe('first run and the one-time ask', () => {
  beforeEach(() => resetSessionMemory());
  it('shows the welcome once and remembers dismissal', () => {
    const store = new Store();
    expect(shouldShowWelcome(store)).toBe(true);
    rememberWelcome(store, new Date('2026-10-20T10:00:00Z'));
    expect(store.getItem(STORAGE_KEYS.welcome)).toBe('2026-10-20');
    resetSessionMemory();
    expect(shouldShowWelcome(store)).toBe(false);
  });
  it('skips the welcome in automated test browsers', () => {
    expect(shouldShowWelcome(new Store(), true)).toBe(false);
  });
  it('still hides the welcome for the session when storage is blocked', () => {
    const blocked = new BlockedStore();
    expect(shouldShowWelcome(blocked)).toBe(true);
    rememberWelcome(blocked);
    expect(shouldShowWelcome(blocked)).toBe(false);
    expect(shouldShowWelcome(null)).toBe(false);
  });
  it('asks at most once for each version', () => {
    const store = new Store();
    expect(shouldAsk(store, '0.4.0')).toBe(true);
    rememberAsk(store, '0.4.0');
    resetSessionMemory();
    expect(shouldAsk(store, '0.4.0')).toBe(false);
    expect(shouldAsk(store, '0.7.0')).toBe(true);
    expect(shouldAsk(new Store(), '0.4.0', true)).toBe(false);
  });
  it('recognises a finished nest or a saved export, but not a cancelled save', () => {
    expect(isMilestone('Nesting finished. Review the validation checks before cutting.')).toBe(true);
    expect(isMilestone('Saved: C:\\x\\marker.plt')).toBe(true);
    expect(isMilestone('Save cancelled.')).toBe(false);
    expect(isMilestone('Nesting: 3.0 s · best 1.2 m')).toBe(false);
  });
});

describe('languages and platform', () => {
  it('has every English string in Hindi and Tamil, marked for native review', () => {
    const keys = Object.keys(en);
    for (const code of ['hi', 'ta'] as const) {
      const dict = feedbackLanguages[code];
      expect(Object.keys(dict).sort()).toEqual([...keys].sort());
      for (const key of keys) expect(dict[key]?.trim(), `${code}.${key}`).toBeTruthy();
      expect(dict._status).toMatch(/Native .* review required/);
    }
  });
  it('detects only a coarse operating system name', () => {
    expect(detectOs('Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36')).toBe('Windows');
    expect(detectOs('Mozilla/5.0 (Linux; Android 14; Pixel 8)')).toBe('Android');
    expect(detectOs('Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X)')).toBe('iOS or iPadOS');
    expect(detectOs('Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)')).toBe('macOS');
    expect(detectOs('Mozilla/5.0 (X11; Linux x86_64)')).toBe('Linux');
    expect(detectOs('')).toBe('Unknown');
  });
});
