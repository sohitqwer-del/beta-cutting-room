/* BETA Cutting Room offline copy (Apache-2.0).
 * Precaches the files listed in precache.json (written by the build) from this same server.
 * Same-origin app assets only. API requests bypass the cache; static mode blocks them in the app.
 * No third-party requests. A new build has a new version, so the old cache is removed on activate. */
const ROOT = new URL(self.registration.scope);
const PREFIX = 'beta-cutting-room-' + encodeURIComponent(ROOT.pathname) + '-';

self.addEventListener('install', event => {
  event.waitUntil((async () => {
    const response = await fetch('./precache.json', { cache: 'no-store' });
    if (!response.ok) throw new Error('No precache list.');
    const list = await response.json();
    const cache = await caches.open(PREFIX + list.version);
    await cache.addAll(['./', ...list.files.map(f => './' + f)]);
    await self.skipWaiting();
  })());
});

self.addEventListener('activate', event => {
  event.waitUntil((async () => {
    const list = await (await fetch('./precache.json', { cache: 'no-store' }).catch(() => null))?.json().catch(() => null);
    const keep = list ? PREFIX + list.version : null;
    for (const name of await caches.keys()) if (name.startsWith(PREFIX) && keep && name !== keep) await caches.delete(name);
    await self.clients.claim();
  })());
});

self.addEventListener('fetch', event => {
  const request = event.request;
  const url = new URL(request.url);
  if (request.method !== 'GET' || url.origin !== ROOT.origin || !url.pathname.startsWith(ROOT.pathname) || url.pathname.includes('/api/')) return;
  event.respondWith((async () => {
    const names = (await caches.keys()).filter(name => name.startsWith(PREFIX)).reverse();
    for (const name of names) {
      const cached = await (await caches.open(name)).match(request, { ignoreSearch: request.mode === 'navigate' });
      if (cached) return cached;
    }
    try {
      return await fetch(request);
    } catch (error) {
      // Offline page loads (any address inside the app) get the cached app shell.
      if (request.mode === 'navigate') {
        for (const name of names) {
          const shell = await (await caches.open(name)).match(ROOT.href);
          if (shell) return shell;
        }
      }
      throw error;
    }
  })());
});
// build 1a6c541b6595e0a7
