// The service worker of the browser version (P15 step 3): it keeps the
// files of one version, so the page works offline and as an app from the
// home screen. tools/build-web.py puts the version in place of @VERSION@
// (a hash of the program); a new version gets a new cache, and the old one
// goes when the new worker starts (when all pages of the old one are
// closed).
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
  event.waitUntil(caches.open(CACHE).then((cache) => cache.addAll(FILES)));
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k.startsWith("shokujodx-") && k !== CACHE).map((k) => caches.delete(k)))),
  );
});

self.addEventListener("fetch", (event) => {
  if (event.request.method !== "GET") return;
  // The files of this version first; the address of a measure (?stress=)
  // is the page too.
  event.respondWith(caches.match(event.request, { ignoreSearch: true }).then((hit) => hit || fetch(event.request)));
});
