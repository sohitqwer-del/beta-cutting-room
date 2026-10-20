/** Set by build:static. Native deployments retain their existing API transport. */
export const staticMode = (): boolean => import.meta.env.VITE_BETA_STATIC === '1';
export const SERVER_FEATURE_MESSAGE =
  'This feature needs the BETA desktop app or a local BETA server. In this browser edition, use Photo to marker, the marker editor, Pattern, or local JSON files.';

/** Explain unavailable capabilities at their point of use without hiding local tools. */
export function installStaticNotices(): void {
  if (!staticMode()) return;
  const content = document.querySelector('#content');
  const notice = document.createElement('details');
  notice.id = 'static-mode-note';
  notice.className = 'notice no-print';
  const summary = document.createElement('summary');
  summary.textContent = 'Browser edition · no server or install needed';
  const text = document.createElement('p');
  text.textContent = 'Photo to marker, browser nesting, editing, Pattern and local JSON files work on this device. Quick Marker can print tiled PDFs and chalk guides. Marker DXF/PLT exchange, cut planning, roll allocation and some production reports need the desktop app or a local server.';
  notice.append(summary, text);
  content?.prepend(notice);
  for (const id of ['screen-1', 'screen-2', 'screen-5', 'screen-6', 'screen-7']) {
    const target = document.getElementById(id);
    if (!target) continue;
    const hint = document.createElement('p');
    hint.className = 'notice warn static-feature-note';
    hint.setAttribute('role', 'note');
    hint.textContent = id === 'screen-5'
      ? 'This digitizing panel needs the desktop app or server. Use Photo to marker for scanning directly in this browser.'
      : 'This panel contains desktop/server operations. You can review its forms here; operations that need a server are unavailable in the browser edition.';
    target.prepend(hint);
  }
  document.querySelector<HTMLElement>('#token-button')?.setAttribute('hidden', '');
  const runs = document.querySelector<HTMLInputElement>('#runs');
  if (runs) { runs.value = '1'; runs.disabled = true; runs.closest('label')?.setAttribute('hidden', ''); }
  for (const button of Array.from(document.querySelectorAll<HTMLButtonElement>('[data-export]'))) {
    button.disabled = true;
    button.title = 'This marker export needs the desktop app or server. Quick Marker offers local PDF and chalk guides.';
  }
  const exports = document.querySelector('[data-export]')?.parentElement;
  if (exports) {
    const hint = document.createElement('p');
    hint.className = 'hint';
    hint.textContent = 'Other marker exports need desktop/server. Save JSON here, or use Photo to marker for local PDF output.';
    exports.append(hint);
  }
}
