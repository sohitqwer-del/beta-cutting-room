/** Browser HTTP and offline Tauri IPC share the same Response contract. */
import { staticMode, SERVER_FEATURE_MESSAGE } from './static-mode';
export interface NativeReply { status: number; mime: string; extension: string | null; body: number[] }
interface Tauri { core: { invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> }; event: { listen<T>(name: string, handler: (event: { payload: T }) => void): Promise<() => void> } }
declare global { interface Window { __TAURI__?: Tauri } }
export const isDesktop = (): boolean => !!window.__TAURI__;
export function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!window.__TAURI__) return Promise.reject(new Error('Native desktop is unavailable.'));
  return window.__TAURI__.core.invoke<T>(command, args);
}
export function listen<T>(name: string, handler: (value: T) => void): Promise<() => void> {
  if (!window.__TAURI__) return Promise.resolve(() => {});
  return window.__TAURI__.event.listen<T>(name, event => handler(event.payload));
}
const TOKEN_KEY = 'beta-server-token';
/** Server access token for `beta-marker serve --token`. Kept for this browser tab only, never in a URL. */
export function serverToken(): string {
  try { return sessionStorage.getItem(TOKEN_KEY) ?? ''; } catch { return ''; }
}
export function setServerToken(token: string): void {
  try { if (token.trim()) sessionStorage.setItem(TOKEN_KEY, token.trim()); else sessionStorage.removeItem(TOKEN_KEY); } catch { /* Storage blocked: requests stay unauthenticated. */ }
}
/** Keep byte exports and JSON errors identical across both transports. */
export async function apiFetch(url: string, init?: RequestInit): Promise<Response> {
  if (!isDesktop()) {
    const path = url.split('?')[0];
    if (path === '/api/import') {
      const { browserImport } = await import('./browser-import');
      const imported = await browserImport(url, init);
      if (imported) return imported;
    }
    if (path === '/api/export') {
      const { browserExport } = await import('./browser-export');
      const output = await browserExport(url, init);
      if (output) return output;
    }
    if (staticMode() && url.startsWith('/api/')) {
      // Cut plans, rolls, spreading, bundles, tickets, reports and PLT import run in Rust WASM here.
      const { browserCutRoom } = await import('./browser-cutroom');
      const local = await browserCutRoom(url, init);
      if (local) return local;
      return new Response(JSON.stringify({ error: SERVER_FEATURE_MESSAGE }), {
        status: 503, headers: { 'Content-Type': 'application/json' },
      });
    }
    const token = url.startsWith('/api/') ? serverToken() : '';
    if (!token) return fetch(url, init);
    const headers = new Headers(init?.headers); headers.set('Authorization', `Bearer ${token}`);
    return fetch(url, { ...init, headers });
  }
  if (!url.startsWith('/api/')) throw new Error('Only local BETA API paths are allowed.');
  const body = init?.body == null ? [] : Array.from(new Uint8Array(await new Response(init.body).arrayBuffer()));
  const reply = await invoke<NativeReply>('desktop_request', { method: init?.method ?? 'GET', path: url, body });
  const headers: Record<string, string> = { 'Content-Type': reply.mime };
  if (reply.extension) headers['X-Beta-Extension'] = reply.extension;
  return new Response(new Uint8Array(reply.body), { status: reply.status, headers });
}
/** Native exports use a save dialog. Browser exports keep their download link. */
export async function downloadFile(blob: Blob, name: string): Promise<string | null> {
  if (isDesktop()) {
    name = ({ 'marker.job.json': 'marker.betajob', 'marker.result.json': 'marker.betamarker', 'pattern.doc.json': 'pattern.betapattern' } as Record<string, string>)[name] ?? name;
    const path = await invoke<string | null>('desktop_save', { name, bytes: Array.from(new Uint8Array(await blob.arrayBuffer())) });
    window.dispatchEvent(new CustomEvent('desktop-saved', { detail: { path, name } }));
    return path;
  }
  const url = URL.createObjectURL(blob); const link = document.createElement('a'); link.href = url; link.download = name; link.click();
  setTimeout(() => URL.revokeObjectURL(url), 2000); return name;
}
