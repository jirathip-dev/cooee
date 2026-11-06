// cooee's service worker: push, and the last copy of each page for offline (#21). A push is one line about a thread,
// tagged with the thread's id so a newer one replaces the older; none shows while that thread is open in view; a tap
// opens the thread; the page closes a thread's notifications once it has read past their item. The pages and their
// static files go to the network first; a good answer is kept, stamped `cooee-at`, and given back when the network
// fails, so the Home Screen app opens offline on its last copy and says how old it is (a thread's newest messages are
// kept by chat.js).
const CACHE = "cooee-1", KEPT = /^\/($|t\/|app\.css|chat\.js|look\.js|look\/|icon|manifest)/;
self.addEventListener("install", () => self.skipWaiting());
self.addEventListener("activate", (e) => e.waitUntil(self.clients.claim()));

self.addEventListener("fetch", (e) => {
  const r = e.request, u = new URL(r.url);
  if (r.method !== "GET" || u.origin !== location.origin || !KEPT.test(u.pathname)) return;
  e.respondWith(fetch(r).then(async (res) => {
    if (res.ok && res.type === "basic") {
      const headers = new Headers(res.headers);
      headers.set("cooee-at", String(Date.now()));
      const copy = new Response(await res.clone().blob(), { status: res.status, headers });
      await (await caches.open(CACHE)).put(r, copy).catch(() => {});
    }
    return res;
  }, async () => (await caches.match(r)) ?? Response.error()));
});

self.addEventListener("push", (e) => e.waitUntil((async () => {
  const n = e.data ? e.data.json() : null;
  if (!n) return;
  const open = await self.clients.matchAll({ type: "window", includeUncontrolled: true });
  if (open.some((c) => c.visibilityState === "visible" && new URL(c.url).pathname === new URL(n.url, c.url).pathname)) return;
  await self.registration.showNotification(n.title, { body: n.body, tag: n.tag, data: n, icon: "/icon-192.png" });
})()));

self.addEventListener("notificationclick", (e) => {
  e.notification.close();
  const url = e.notification.data?.url ?? "/";
  e.waitUntil(self.clients.matchAll({ type: "window" }).then((ws) =>
    ws.length ? ws[0].navigate(url).then((w) => w?.focus()) : self.clients.openWindow(url)));
});

// `{thread, read}` from the page: that thread's notifications up to item `read` go.
self.addEventListener("message", (e) => {
  const { thread, read } = e.data ?? {};
  if (!thread) return;
  e.waitUntil(self.registration.getNotifications({ tag: thread }).then((ns) => {
    for (const n of ns) if ((n.data?.item ?? 0) <= read) n.close();
  }));
});
