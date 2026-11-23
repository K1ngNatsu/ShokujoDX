// The service worker of the browser version (P15 step 3): it keeps the
// files of one version, so the page works offline and as an app from the
// home screen. tools/build-web.py puts the version in place of @VERSION@
// (a hash of the program); a new version gets a new cache. The new worker
// starts at once and removes the old cache, so the next load of the page
// (a reload) is the new version; a page that is open keeps the program that
// it loaded.
const CACHE = "shokujodx-@VERSION@";
// The files of the page (tools/build-web.py writes the list).
const FILES = @FILES@;

self.addEventListener("install", (event) => {
  // The program and its glue have the version in their address: the cache
  // of the browser can give them (the page has just downloaded them, so a
  // first visit downloads them one time). The other files come from the
  // network (the cache of the browser can keep the files of the version
  // before for some minutes).
  const request = (f) => new Request(f, { cache: f.includes("?v=") ? "default" : "reload" });
  event.waitUntil(
    caches
      .open(CACHE)
      .then((cache) => cache.addAll(FILES.map(request)))
      .then(() => self.skipWaiting()),
  );
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k.startsWith("shokujodx-") && k !== CACHE).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  );
});

self.addEventListener("fetch", (event) => {
  if (event.request.method !== "GET") return;
  // The files of this version first; the address of a measure (?stress=)
  // is the page too.
  event.respondWith(caches.match(event.request, { ignoreSearch: true }).then((hit) => hit || fetch(event.request)));
});
