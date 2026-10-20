// Headless DOM integration test. Mount in an isolated same-origin iframe with
// .topbar-tools/#about-button, #screen-3 and #about-dialog .dialog-body.
// The real feedback UI runs; only OS opening and clipboard access are replaced.
import { installFeedback } from './index';
import { FEEDBACK_CONFIG, STORAGE_KEYS } from './config';

export async function runDesktopFeedbackChecks() {
  const calls = [], copies = [], checks = [];
  let rejectOpen = false, rejectCopy = false;
  const oldTauri = window.__TAURI__, clipboard = Object.getOwnPropertyDescriptor(navigator, 'clipboard');
  const oldExec = document.execCommand;
  const saved = Object.fromEntries(Object.values(STORAGE_KEYS).map(key => [key, localStorage.getItem(key)]));
  const assert = (ok, label) => { if (!ok) throw new Error(`${label}: ${document.querySelector('#fb-status')?.textContent}`); checks.push(label); };
  const until = async condition => { for (let i = 0; i < 100; i++) { if (condition()) return; await new Promise(r => setTimeout(r, 20)); } throw new Error('Feedback integration timed out.'); };
  window.__TAURI__ = { core: { invoke: async (command, args) => { calls.push({ command, args }); if (rejectOpen) throw new Error('Synthetic opener failure'); } } };
  Object.defineProperty(navigator, 'clipboard', { configurable: true, value: { writeText: async text => { if (rejectCopy) throw new Error('Synthetic clipboard failure'); copies.push(text); } } });
  document.execCommand = () => false;
  try {
    const api = installFeedback(); document.querySelector('#fb-welcome')?.close();
    const top = document.querySelector('#feedback-button');
    assert(top?.parentElement.classList.contains('topbar-tools') && top.nextElementSibling.id === 'about-button', 'feedback installs before About in the redesigned top bar');
    function preview() {
      api.open();
      document.querySelector('#fb-title-input').value = 'Original synthetic opener test';
      document.querySelector('#fb-details').value = 'Headless test. No factory data and no real message.';
      document.querySelector('#fb-form').requestSubmit();
    }
    preview();
    for (const [id, prefix] of [['fb-github', FEEDBACK_CONFIG.newIssue], ['fb-email', `mailto:${FEEDBACK_CONFIG.email}`]]) {
      const count = calls.length; document.getElementById(id).click(); await until(() => calls.length > count);
      assert(calls.at(-1).command === 'plugin:opener|open_url' && calls.at(-1).args.url.startsWith(prefix) && calls.at(-1).args.with === null, `${id} uses the scoped plugin command`);
    }
    document.querySelector('#fb-dialog').close();
    let count = calls.length; document.querySelector('#screen-3 [data-fb-external]').click(); await until(() => calls.length > count);
    assert(calls.at(-1).args.url === FEEDBACK_CONFIG.discussions, 'Join the discussion uses the scoped plugin command');
    rejectOpen = true; preview();
    for (const [id, prefix] of [['fb-github', FEEDBACK_CONFIG.newIssue], ['fb-email', `mailto:${FEEDBACK_CONFIG.email}`]]) {
      const count = copies.length; document.getElementById(id).click(); await until(() => copies.length > count);
      assert(copies.at(-1).startsWith(prefix), `${id} copies the full link after opener failure`);
    }
    document.querySelector('#fb-dialog').close();
    count = copies.length; document.querySelector('#screen-3 [data-fb-external]').click(); await until(() => copies.length > count);
    assert(copies.at(-1) === FEEDBACK_CONFIG.discussions, 'Join the discussion copies its link after opener failure');
    rejectCopy = true; preview(); document.querySelector('#fb-github').click();
    await until(() => document.querySelector('#fb-status').textContent.includes('Could not copy'));
    assert(!document.querySelector('#fb-preview-step').hidden && !document.querySelector('#fb-save').disabled, 'blocked clipboard keeps report preview and Save report available');
    document.querySelector('#fb-dialog').close();
    return { checks, nativeCalls: calls.length, clipboardFallbacks: copies.length, realExternalAppsOpened: 0 };
  } finally {
    window.__TAURI__ = oldTauri; document.execCommand = oldExec;
    if (clipboard) Object.defineProperty(navigator, 'clipboard', clipboard); else delete navigator.clipboard;
    for (const [key, value] of Object.entries(saved)) { if (value === null) localStorage.removeItem(key); else localStorage.setItem(key, value); }
  }
}
