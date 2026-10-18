/* Deliberately separate from /apps/. Only explicit, committed offline copies are served. */
self.addEventListener('install', event => event.waitUntil(self.skipWaiting()));
self.addEventListener('activate', event => event.waitUntil(self.clients.claim()));
async function copies() {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open('declared-local-views', 1);
    request.onupgradeneeded = () =>
      request.result.createObjectStore('copies', { keyPath: 'viewId' });
    request.onerror = () => reject(request.error);
    request.onsuccess = () => {
      const db = request.result;
      const get = db.transaction('copies').objectStore('copies').getAll();
      get.onsuccess = () => {
        db.close();
        resolve(get.result);
      };
      get.onerror = () => {
        db.close();
        reject(get.error);
      };
    };
  });
}
self.addEventListener('fetch', event => {
  const url = new URL(event.request.url);
  if (
    url.origin !== self.location.origin ||
    event.request.method !== 'GET' ||
    url.pathname.startsWith('/api/') ||
    event.request.headers.has('X-Local-View-Pin')
  )
    return;
  event.respondWith(
    (async () => {
      const retained = await copies();
      const navigation = event.request.mode === 'navigate';
      const client = event.clientId ? await self.clients.get(event.clientId) : undefined;
      const pagePath = navigation ? url.pathname : client ? new URL(client.url).pathname : '';
      for (const copy of retained) {
        if (pagePath !== `/avodah/views/${encodeURIComponent(copy.viewId)}`) continue;
        const cache = await caches.open(copy.buildCache);
        const response = await cache.match(navigation ? '/index.html' : event.request);
        if (response) return response;
      }
      return fetch(event.request);
    })()
  );
});
