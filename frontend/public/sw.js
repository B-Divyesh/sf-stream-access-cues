const release = new URL(self.location.href).searchParams.get('build') || 'unversioned-build';
const CACHE = `stream-access-cues-${release}`;
const SHELL = ['/', '/index.html', '/icon.svg', '/manifest.webmanifest'];
const removeStaleCaches = () => caches.keys().then((keys) => Promise.all(
  keys.filter((key) => key !== CACHE).map((key) => caches.delete(key))
));
self.addEventListener('install', (event) => {
  event.waitUntil(caches.open(CACHE).then((cache) => cache.addAll(SHELL)).then(() => self.skipWaiting()));
});
self.addEventListener('activate', (event) => {
  event.waitUntil(removeStaleCaches().then(() => self.clients.claim()));
});
self.addEventListener('fetch', (event) => {
  const requestUrl = new URL(event.request.url);
  // The update script itself must always reach the network. Caching it under
  // the outgoing release can recreate a stale cache while a new worker starts.
  if (event.request.method !== 'GET' || requestUrl.pathname === '/sw.js' || requestUrl.pathname.startsWith('/api/')) return;
  // Catch a cache write that an outgoing worker completed after activation.
  // This runs on the first request controlled by the new release.
  event.waitUntil(removeStaleCaches());
  event.respondWith(caches.match(event.request).then((cached) => cached || fetch(event.request).then((response) => {
    if (response.ok && requestUrl.origin === self.location.origin) {
      const clone = response.clone();
      // Keep the cache write in this worker's lifetime. Without waitUntil an
      // outgoing worker can recreate its old release cache after a new worker
      // has activated and removed it.
      event.waitUntil(caches.open(CACHE).then((cache) => cache.put(event.request, clone)));
    }
    return response;
  }).catch(() => caches.match('/index.html'))));
});
