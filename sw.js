// Network first, so a new push reaches installed copies. The cache is the offline fallback.
// Paths are relative to this file: the site lives under /near-me-demo/, not at the root.
// Other sites share this origin, so only caches named near-me-* are ever touched.
const CACHE = 'near-me-v5';
const SHELL = ['./', 'app.css', 'app.js', 'manifest.json', 'icon-192.png', 'icon-512.png']
  .map((path) => new URL(path, self.location).href);

self.addEventListener('install', (e) => {
  e.waitUntil(caches.open(CACHE).then((c) => c.addAll(SHELL)).then(() => self.skipWaiting()));
});

self.addEventListener('activate', (e) => {
  e.waitUntil(
    caches.keys()
      .then((keys) => keys.filter((k) => k.startsWith('near-me-') && k !== CACHE))
      .then((old) => Promise.all(old.map((k) => caches.delete(k))))
      .then(() => self.clients.claim())
  );
});

self.addEventListener('fetch', (e) => {
  const url = new URL(e.request.url);
  url.search = '';
  if (e.request.method !== 'GET' || !SHELL.includes(url.href)) return;
  e.respondWith(
    fetch(e.request)
      .then((res) => {
        if (res.ok) {
          const copy = res.clone();
          caches.open(CACHE).then((c) => c.put(url.href, copy));
        }
        return res;
      })
      .catch(() => caches.match(url.href))
  );
});
