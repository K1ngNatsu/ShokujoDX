// The service worker of the browser version (P15 step 3): it keeps the
// files of one version, so the page works offline and as an app from the
// home screen. tools/build-web.py puts the version in place of @VERSION@
// (a hash of the program); a new version gets a new cache. The new worker
// starts at once and removes the old cache, so the next load of the page
// (a reload) is the new version; a page that is open keeps the program that
// it loaded.
const CACHE = "shokujodx-@VERSION@";
const FILES = [
  "./",
  "./index.html",
  "./emb-app.js",
  "./emb-app_bg.wasm",
  "./manifest.webmanifest",
  "./icon-180.png",
  "./icon-192.png",
  "./icon-512.png",
];

self.addEventListener("install", (event) => {
  // From the network, not from the cache of the browser (it can keep the
  // files of the version before for some minutes).
  event.waitUntil(
    caches
      .open(CACHE)
      .then((cache) => cache.addAll(FILES.map((f) => new Request(f, { cache: "reload" }))))
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
