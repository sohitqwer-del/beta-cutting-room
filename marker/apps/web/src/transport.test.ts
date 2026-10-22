import { afterEach, describe, expect, it, vi } from 'vitest';
import { apiFetch, serverToken, setServerToken } from './transport';
afterEach(() => { vi.unstubAllGlobals(); vi.unstubAllEnvs(); });
describe('desktop transport', () => {
 it('explains server-only features locally in a static build without a network request', async () => {
  vi.stubEnv('VITE_BETA_STATIC', '1'); vi.stubGlobal('window', {});
  const fetch = vi.fn(); vi.stubGlobal('fetch', fetch);
  // Cut plans now run in the browser; only the board-photo digitizer still needs a server.
  const response = await apiFetch('/api/digitize', { method: 'POST', body: '{}' });
  expect(response.status).toBe(503);
  expect((await response.json() as {error:string}).error).toContain('desktop app');
  expect(fetch).not.toHaveBeenCalled();
 });
 it('preserves the HTTP request in a browser', async () => {
  vi.stubGlobal('window', {});const fetch = vi.fn().mockResolvedValue(new Response('ok'));vi.stubGlobal('fetch',fetch);
  const init={method:'POST',body:'{}'};expect(await (await apiFetch('/api/validate',init)).text()).toBe('ok');expect(fetch).toHaveBeenCalledWith('/api/validate',init);
 });
 it('passes raw file bytes and returns binary export headers', async () => {
  const invoke=vi.fn().mockResolvedValue({status:200,mime:'application/pdf',extension:'pdf',body:[37,80,68,70,0,255]});vi.stubGlobal('window',{__TAURI__:{core:{invoke}}});
  const response=await apiFetch('/api/digitize',{method:'POST',body:new Uint8Array([0,128,255])});
  expect(invoke).toHaveBeenCalledWith('desktop_request',{method:'POST',path:'/api/digitize',body:[0,128,255]});expect(response.headers.get('X-Beta-Extension')).toBe('pdf');expect([...new Uint8Array(await response.arrayBuffer())]).toEqual([37,80,68,70,0,255]);
 });
 it('keeps JSON errors and cancellation responses', async () => {
  const invoke=vi.fn().mockResolvedValue({status:499,mime:'application/json',extension:null,body:[...new TextEncoder().encode('{"error":"Cancelled"}')]});vi.stubGlobal('window',{__TAURI__:{core:{invoke}}});
  const response=await apiFetch('/api/nest',{method:'POST',body:'{}'});expect(response.ok).toBe(false);expect(response.status).toBe(499);expect(await response.json()).toEqual({error:'Cancelled'});
 });
 it('uses GET for status and rejects external URLs', async () => {
  const invoke=vi.fn().mockResolvedValue({status:200,mime:'application/json',extension:null,body:[123,125]});vi.stubGlobal('window',{__TAURI__:{core:{invoke}}});await apiFetch('/api/nest/status');expect(invoke).toHaveBeenCalledWith('desktop_request',{method:'GET',path:'/api/nest/status',body:[]});await expect(apiFetch('https://example.com')).rejects.toThrow('Only local');
 });
 it('adds a session token as a bearer header only for API paths', async () => {
  const store=new Map<string,string>();vi.stubGlobal('sessionStorage',{getItem:(k:string)=>store.get(k)??null,setItem:(k:string,v:string)=>store.set(k,v),removeItem:(k:string)=>store.delete(k)});
  vi.stubGlobal('window',{});const fetch=vi.fn().mockResolvedValue(new Response('ok'));vi.stubGlobal('fetch',fetch);
  setServerToken(' secret ');await apiFetch('/api/health',{method:'GET',headers:{'Content-Type':'application/json'}});
  const [url,init]=fetch.mock.calls[0]! as [string,RequestInit];expect(url).toBe('/api/health');expect(new Headers(init.headers).get('Authorization')).toBe('Bearer secret');expect(new Headers(init.headers).get('Content-Type')).toBe('application/json');
  setServerToken('');expect(serverToken()).toBe('');
 });
});
