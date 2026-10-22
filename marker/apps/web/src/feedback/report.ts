import { FEEDBACK_CONFIG } from './config';

/** Pure report building. No DOM, no network: easy to test and easy to check for privacy. */
export type FeedbackKind = 'bug' | 'missing' | 'idea';
export type Platform = 'desktop' | 'server' | 'browser';

export interface FeedbackInput {
  kind: FeedbackKind;
  title: string;
  details: string;
  steps?: string;
  contact?: string;
}
/** Filled in by the app and shown to the user before sending. Never file names, paths or factory data. */
export interface FeedbackContext { screen: string; version: string; platform: Platform; os: string; language: string }
/** Optional and opt-in. Only anonymous numbers, never names or geometry. */
export interface MarkerNumbers { pieces: number | null; efficiencyPct: number | null; timeLimitS: number | null }
export interface Report { title: string; body: string; text: string }
export interface Link { url: string; truncated: boolean }

const KIND_NAME: Record<FeedbackKind, string> = { bug: 'Bug report', missing: 'Something is missing', idea: 'General feedback or idea' };
const KIND_PREFIX: Record<FeedbackKind, string> = { bug: '[Bug]', missing: '[Missing]', idea: '[Idea]' };
const DETAILS_HEADING: Record<FeedbackKind, string> = { bug: 'What happened', missing: 'What I need', idea: 'My feedback' };
const STEPS_HEADING: Record<FeedbackKind, string> = { bug: 'Steps to repeat the problem', missing: 'How I would use it', idea: 'Example' };
const PLATFORM_NAME: Record<Platform, string> = { desktop: 'Desktop app', server: 'Browser with a BETA server', browser: 'Browser only (runs on this device)' };
const LANGUAGE_NAME: Record<string, string> = { en: 'English', hi: 'Hindi', ta: 'Tamil' };
export const TITLE_MAX = 120;

export const kindName = (kind: FeedbackKind): string => KIND_NAME[kind];
export const platformName = (platform: Platform): string => PLATFORM_NAME[platform];
export const languageName = (code: string): string => LANGUAGE_NAME[code] ?? 'English';

const clean = (value: string | undefined): string => (value ?? '').replace(/\r\n?/g, '\n').replace(/[\u0000-\u0008\u000b\u000c\u000e-\u001f\u007f]/g, '').trim();

/** Keep only finite, plausible numbers. Anything else is dropped, so no other value can slip in. */
export function safeNumbers(numbers: MarkerNumbers | null | undefined): MarkerNumbers | null {
  if (!numbers) return null;
  const pick = (v: unknown, max: number, digits: number): number | null =>
    typeof v === 'number' && Number.isFinite(v) && v >= 0 && v <= max ? Number(v.toFixed(digits)) : null;
  const out: MarkerNumbers = { pieces: pick(numbers.pieces, 1e6, 0), efficiencyPct: pick(numbers.efficiencyPct, 100, 1), timeLimitS: pick(numbers.timeLimitS, 1e6, 1) };
  return out.pieces === null && out.efficiencyPct === null && out.timeLimitS === null ? null : out;
}

export function reportTitle(input: FeedbackInput, context: FeedbackContext): string {
  let title = clean(input.title).replace(/\s+/g, ' ');
  if (!title) title = clean(input.details).split('\n')[0]!.slice(0, 70).trim();
  if (!title) title = `Feedback about ${context.screen}`;
  title = `${KIND_PREFIX[input.kind]} ${title}`;
  return title.length > TITLE_MAX ? `${title.slice(0, TITLE_MAX - 1).trimEnd()}…` : title;
}

/** Markdown body. GitHub, email and the saved file all use exactly this text. */
export function buildReport(input: FeedbackInput, context: FeedbackContext, numbers?: MarkerNumbers | null): Report {
  const title = reportTitle(input, context);
  const details = clean(input.details) || '(not given)';
  const steps = clean(input.steps), contact = clean(input.contact);
  const safe = safeNumbers(numbers);
  const lines = [
    `**Type:** ${KIND_NAME[input.kind]}`,
    `**Screen:** ${clean(context.screen) || 'Unknown'}`,
    '',
    `### ${DETAILS_HEADING[input.kind]}`, details, '',
  ];
  if (steps) lines.push(`### ${STEPS_HEADING[input.kind]}`, steps, '');
  if (contact) lines.push('### Contact (given by the user)', contact, '');
  lines.push('### About this computer',
    `- BETA Cutting Room ${clean(context.version)}`,
    `- ${PLATFORM_NAME[context.platform]}`,
    `- Operating system: ${clean(context.os) || 'Unknown'}`,
    `- App language: ${languageName(context.language)}`, '');
  if (safe) {
    lines.push('### Anonymous marker numbers (added by the user)');
    if (safe.pieces !== null) lines.push(`- Pieces: ${safe.pieces}`);
    if (safe.efficiencyPct !== null) lines.push(`- Efficiency: ${safe.efficiencyPct.toFixed(1)}%`);
    if (safe.timeLimitS !== null) lines.push(`- Time limit: ${safe.timeLimitS} s`);
    lines.push('');
  }
  lines.push('_Sent from the Feedback button in BETA Cutting Room. No files, patterns, file paths or factory names are added automatically._');
  const body = lines.join('\n');
  return { title, body, text: `${title}\n\n${body}\n` };
}

/** Things in the user's own text that may identify a factory. We warn; we never change their words silently. */
export type PrivacyHint = 'path' | 'file' | 'geometry';
export function privacyHints(...texts: Array<string | undefined>): PrivacyHint[] {
  const text = texts.filter(Boolean).join('\n');
  const hints = new Set<PrivacyHint>();
  if (/\b[A-Za-z]:[\\/]\S|\\\\[A-Za-z0-9._-]+\\|(^|[\s"'(])\/(home|Users|mnt|media|srv|var|opt|data)\/\S/m.test(text)) hints.add('path');
  if (/[\w-]+\.(betajob|betamarker|betapattern|betaproj|dxf|plt|hpgl|aama|rul|cut|iso|xlsx?|csv|pdf)\b/i.test(text)) hints.add('file');
  if ((text.match(/-?\d+(?:\.\d+)?/g) ?? []).length > 60) hints.add('geometry');
  return [...hints];
}

const bytes = (s: string): number => new TextEncoder().encode(s).length;
/** Cut at a code point, never in the middle of a surrogate pair. */
const cut = (s: string, n: number): string => Array.from(s).slice(0, n).join('');

/** Fit `body` into a URL limit. `make` builds the URL from a body. The full report is never lost:
 * the caller copies it to the clipboard when the link was shortened. */
function fitLink(make: (body: string) => string, body: string, maxBytes: number, note: string): Link {
  const full = make(body);
  if (bytes(full) <= maxBytes) return { url: full, truncated: false };
  const chars = Array.from(body);
  let lo = 0, hi = chars.length;
  while (lo < hi) {
    const mid = Math.ceil((lo + hi) / 2);
    if (bytes(make(cut(body, mid) + note)) <= maxBytes) lo = mid; else hi = mid - 1;
  }
  const url = make(cut(body, lo).trimEnd() + note);
  return { url: bytes(url) <= maxBytes ? url : make(note.trim()), truncated: true };
}

export const SHORTENED_NOTE = '\n\n…\n_This report was shortened to fit in a link. The full text was copied to the clipboard: please paste it here._';

export function issueUrl(report: Report, kind: FeedbackKind, maxBytes: number = FEEDBACK_CONFIG.maxIssueUrlBytes): Link {
  const base = `${FEEDBACK_CONFIG.newIssue}?labels=${encodeURIComponent(FEEDBACK_CONFIG.labels[kind])}&title=${encodeURIComponent(report.title)}`;
  return fitLink(body => `${base}&body=${encodeURIComponent(body)}`, report.body, maxBytes, SHORTENED_NOTE);
}

/** RFC 6068: line breaks are CRLF. Mail apps cut long links, so the limit is lower than for GitHub. */
export function mailtoUrl(report: Report, maxBytes: number = FEEDBACK_CONFIG.maxMailtoBytes): Link {
  const base = `mailto:${FEEDBACK_CONFIG.email}?subject=${encodeURIComponent(`BETA Cutting Room: ${report.title}`)}`;
  return fitLink(body => `${base}&body=${encodeURIComponent(body.replace(/\n/g, '\r\n'))}`, report.body, maxBytes, SHORTENED_NOTE);
}

export function reportFileName(kind: FeedbackKind, date = new Date()): string {
  const d = `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(date.getDate()).padStart(2, '0')}`;
  return `beta-feedback-${kind === 'missing' ? 'missing-feature' : kind}-${d}.md`;
}

/** Coarse operating system name only. No version numbers, device names or user agent strings. */
export function detectOs(userAgent: string, platform = ''): string {
  const s = `${userAgent} ${platform}`;
  if (/Android/i.test(s)) return 'Android';
  if (/iPhone|iPad|iPod/i.test(s)) return 'iOS or iPadOS';
  if (/CrOS/i.test(s)) return 'ChromeOS';
  if (/Win/i.test(s)) return 'Windows';
  if (/Mac/i.test(s)) return 'macOS';
  if (/Linux|X11/i.test(s)) return 'Linux';
  return 'Unknown';
}
