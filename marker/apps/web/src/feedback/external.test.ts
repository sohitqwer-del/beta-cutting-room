import { afterEach, describe, expect, it, vi } from 'vitest';
import { FEEDBACK_CONFIG } from './config';
import { allowedFeedbackLink, openFeedbackLink } from './external';
import { buildReport, issueUrl, mailtoUrl } from './report';

afterEach(() => vi.unstubAllGlobals());
const issue = `${FEEDBACK_CONFIG.newIssue}?title=Test&body=Original%20synthetic%20report`;
const email = `mailto:${FEEDBACK_CONFIG.email}?subject=Test&body=Original%20report`;
const links = [issue, email, FEEDBACK_CONFIG.discussions];

describe('explicit feedback link opening', () => {
  it('leaves browser/server navigation to the existing handlers', async () => {
    vi.stubGlobal('window', {});
    const copy = vi.fn();
    for (const link of links) expect(await openFeedbackLink(link, copy)).toBe('browser');
    expect(copy).not.toHaveBeenCalled();
  });
  it('uses the official scoped command for all three desktop actions, with the default app only', async () => {
    const invoke = vi.fn().mockResolvedValue(undefined), copy = vi.fn();
    vi.stubGlobal('window', { __TAURI__: { core: { invoke } } });
    for (const link of links) {
      expect(await openFeedbackLink(link, copy)).toBe('opened');
      expect(invoke).toHaveBeenLastCalledWith('plugin:opener|open_url', { url: link, with: null });
    }
    expect(copy).not.toHaveBeenCalled();
  });
  it('copies the complete link when the desktop opener rejects or is unavailable', async () => {
    const invoke = vi.fn().mockRejectedValue(new Error('Denied or no handler')), copy = vi.fn().mockResolvedValue(true);
    vi.stubGlobal('window', { __TAURI__: { core: { invoke } } });
    for (const link of links) {
      expect(await openFeedbackLink(link, copy)).toBe('copied');
      expect(copy).toHaveBeenLastCalledWith(link);
    }
  });
  it('reports failed clipboard fallback without an unhandled rejection', async () => {
    vi.stubGlobal('window', { __TAURI__: { core: { invoke: vi.fn().mockRejectedValue(new Error('unavailable')) } } });
    expect(await openFeedbackLink(issue, vi.fn().mockResolvedValue(false))).toBe('failed');
    expect(await openFeedbackLink(issue, vi.fn().mockRejectedValue(new Error('blocked clipboard')))).toBe('failed');
  });
  it('never invokes the desktop opener or clipboard for paths, other hosts/repos or recipients', async () => {
    const invoke = vi.fn(), copy = vi.fn();
    vi.stubGlobal('window', { __TAURI__: { core: { invoke } } });
    for (const value of ['file:///C:/data/pattern.dxf', 'C:\\data\\pattern.dxf', '/tmp/pattern', 'https://example.com',
      'http://github.com/sohitqwer-del/beta-cutting-room/issues', 'https://github.com/sohitqwer-del/other/issues',
      'https://github.com.evil/sohitqwer-del/beta-cutting-room/issues', `${FEEDBACK_CONFIG.repository}/../other`,
      `${FEEDBACK_CONFIG.repository}/%2e%2e/other`, `${FEEDBACK_CONFIG.repository}/issues/%2fother`,
      'mailto:other@example.com', 'mailto:sohitqwer@gmail.com.evil', `${email}&cc=other@example.com`, 'javascript:alert(1)']) {
      expect(allowedFeedbackLink(value)).toBe(false);
      expect(await openFeedbackLink(value, copy)).toBe('failed');
    }
    expect(invoke).not.toHaveBeenCalled(); expect(copy).not.toHaveBeenCalled();
  });
  it('accepts real report links with Unicode, newlines and encoded path text in the report body', () => {
    const report = buildReport({ kind: 'bug', title: 'Original synthetic report', details: 'Original test\nதமிழ்: C:\\sample\\file.json' },
      { screen: 'Help', version: '0.4.0', platform: 'desktop', os: 'Windows', language: 'en' });
    expect(allowedFeedbackLink(issueUrl(report, 'bug').url)).toBe(true);
    expect(allowedFeedbackLink(mailtoUrl(report).url)).toBe(true);
  });
});
