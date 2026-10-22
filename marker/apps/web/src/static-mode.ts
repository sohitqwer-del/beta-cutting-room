import { backupFileName, makeBackup, restoreBackup } from './browser-store';
import { icon } from './icons';
import { find, byId } from './screens';
/** Set by build:static. Native deployments retain their existing API transport. */
export const staticMode = (): boolean => import.meta.env.VITE_BETA_STATIC === '1';
export const SERVER_FEATURE_MESSAGE =
  'This feature needs the BETA desktop app or a local BETA server. In this browser edition, use Photo to marker for scanning; everything else works on this device.';

type Say = (text: string, error?: boolean) => void;

/** Save a backup file of all BETA work kept in this browser (styles, cut plan, rolls, floor records, settings). */
export async function saveBackupFile(say: Say): Promise<void> {
  try {
    const blob = new Blob([JSON.stringify(await makeBackup(), null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob), link = document.createElement('a');
    link.href = url; link.download = backupFileName(); link.click();
    setTimeout(() => URL.revokeObjectURL(url), 2000);
    say('Backup saved to your downloads.');
  } catch (error) { say(`Backup failed: ${error instanceof Error ? error.message : String(error)}`, true); }
}
/** Replace this browser's BETA work with a backup file, then reload. */
export async function restoreBackupFile(file: File, say: Say): Promise<void> {
  try {
    if (!window.confirm('Replace the work saved in this browser with this backup?')) return;
    const count = await restoreBackup(JSON.parse(await file.text()));
    say(`Restored ${count} saved items. Reloading…`);
    window.location.reload();
  } catch (error) { say(`Restore failed: ${error instanceof Error ? error.message : String(error)}`, true); }
}

/** One "Save a backup file" button and one "Restore from a backup file" button (a hidden file input behind it). */
function backupButtons(prefix: string, say: Say, buttonClass = ''): HTMLElement[] {
  const save = document.createElement('button');
  save.type = 'button'; save.id = `${prefix}-save`; if (buttonClass) save.className = buttonClass;
  save.innerHTML = `${icon('download')}<span>Save a backup file</span>`;
  const file = document.createElement('input');
  file.type = 'file'; file.id = `${prefix}-restore`; file.accept = '.json,application/json'; file.hidden = true;
  file.setAttribute('aria-label', 'Backup file to restore');
  const restore = document.createElement('button');
  restore.type = 'button'; restore.id = `${prefix}-restore-button`; if (buttonClass) restore.className = buttonClass;
  restore.innerHTML = `${icon('upload')}<span>Restore from a backup file</span>`;
  save.onclick = () => void saveBackupFile(say);
  restore.onclick = () => file.click();
  file.onchange = () => { const chosen = file.files?.[0]; file.value = ''; if (chosen) void restoreBackupFile(chosen, say); };
  return [save, restore, file];
}

/** Explain unavailable capabilities at their point of use without hiding local tools. */
export function installStaticNotices(say: Say = () => {}): void {
  if (!staticMode()) return;
  // The note sits in the top-bar status popover (one click away), not as a banner above every screen.
  const where = find('#status-where') ?? find('#content');
  const notice = document.createElement('div');
  notice.id = 'static-mode-note';
  notice.className = 'static-note no-print';
  const summary = document.createElement('strong');
  summary.textContent = 'Browser edition · no server or install needed';
  const text = document.createElement('p');
  text.textContent = 'Everything runs on this device: DXF import, photo scanning, nesting, editing, Pattern, cut plans, rolls, spreading, bundles and tickets, floor tracking, reports and factory files (PLT, DXF, PDF, SVG, ISO). Your work is saved in this browser only. Save a backup file (from the ⋯ menu or the Cutting floor screen) to keep it or move it to another computer.';
  notice.append(summary, text);
  if (where?.id === 'status-where') { where.textContent = ''; where.append(notice); } else where?.prepend(notice);

  // Backup: in the "⋯" menu (every screen) and on the Cutting floor screen, next to the floor-record backup.
  const extra = find('#topbar-extra');
  if (extra) {
    extra.setAttribute('aria-label', 'Theme, backup and About');
    const about = find('#about-button');
    if (about?.parentElement === extra) about.before(...backupButtons('backup', say)); else extra.append(...backupButtons('backup', say));
    find('#topbar-more')?.setAttribute('data-tip', 'Theme, backup and About');
  }
  const floorBackups = find('.floor-backups');
  if (floorBackups) {
    // Two labelled groups, so nobody mistakes the floor-record backup for the whole-app one.
    const group = (id: string, title: string, items: Node[]) => {
      const box = document.createElement('div');
      box.className = 'backup-group'; box.setAttribute('role', 'group'); box.setAttribute('aria-labelledby', id);
      const head = document.createElement('span'); head.id = id; head.className = 'backup-group-title'; head.textContent = title;
      box.append(head, ...items); return box;
    };
    const floorOnly = group('floor-only-backup-title', 'Floor records only', Array.from(floorBackups.childNodes));
    floorBackups.append(group('app-backup-title', 'All work in this browser', backupButtons('floor-app-backup', say)), floorOnly);
  }

  // Only the board-photo digitizer still needs the desktop app or a server.
  const digitize = byId('screen-5');
  if (digitize) {
    const hint = document.createElement('p');
    hint.className = 'notice warn static-feature-note';
    hint.setAttribute('role', 'note');
    hint.textContent = 'This digitizing panel needs the desktop app or server. Use Photo to marker for scanning directly in this browser.';
    digitize.prepend(hint);
  }
  find<HTMLElement>('#token-button')?.setAttribute('hidden', '');
  const runs = find<HTMLInputElement>('#runs');
  if (runs) { runs.value = '1'; runs.disabled = true; runs.closest('label')?.setAttribute('hidden', ''); }
}
