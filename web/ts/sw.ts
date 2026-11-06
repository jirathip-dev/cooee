// cooee's service worker: push, and the last copy of each page for offline (#21). A push is one line about a thread,
// tagged with the thread's id so a newer one replaces the older; none shows while that thread is open in view; a tap
// opens the thread; the page closes a thread's notifications once it has read past their item. The pages and their
// static files go to the network first; a good answer is kept, stamped `cooee-at`, and given back when the network
// fails, so the Home Screen app opens offline on its last copy and says how old it is (a thread's newest messages are
// kept by chat.ts).
// A classic worker script (no imports), so the scope is typed by a cast, not a declaration.
const sw = self as unknown as ServiceWorkerGlobalScope;
type Push = { title: string; body: string; tag: string; url: string; item?: number };

const CACHE = "cooee-1";
const KEPT = /^\/($|t\/|threads$|app\.css|chat\.js|md\.js|look\.js|look\/|icon|manifest)/;
sw.addEventListener("install", () => sw.skipWaiting());
sw.addEventListener("activate", (e) => e.waitUntil(sw.clients.claim()));

sw.addEventListener("fetch", (e) => {
  const r = e.request;
  const u = new URL(r.url);
  if (r.method !== "GET" || u.origin !== location.origin || !KEPT.test(u.pathname)) return;
  e.respondWith(
    fetch(r).then(
      async (res) => {
        if (res.ok && res.type === "basic") {
          const headers = new Headers(res.headers);
          headers.set("cooee-at", String(Date.now()));
          const copy = new Response(await res.clone().blob(), { status: res.status, headers });
          await (await caches.open(CACHE)).put(r, copy).catch(() => {});
        }
        return res;
      },
      async () => (await caches.match(r)) ?? Response.error(),
    ),
  );
});

sw.addEventListener("push", (e) =>
  e.waitUntil(
    (async () => {
      const n: Push | null = e.data ? e.data.json() : null;
      if (!n) return;
      const open = await sw.clients.matchAll({ type: "window", includeUncontrolled: true });
      const here = (c: WindowClient): boolean =>
        c.visibilityState === "visible" && new URL(c.url).pathname === new URL(n.url, c.url).pathname;
      if (open.some(here)) return;
      await sw.registration.showNotification(n.title, { body: n.body, tag: n.tag, data: n, icon: "/icon-192.png" });
    })(),
  ),
);

sw.addEventListener("notificationclick", (e) => {
  e.notification.close();
  const url: string = e.notification.data?.url ?? "/";
  e.waitUntil(
    sw.clients
      .matchAll({ type: "window" })
      .then((ws) => (ws[0] ? ws[0].navigate(url).then((w) => w?.focus()) : sw.clients.openWindow(url))),
  );
});

// `{thread, read}` from the page: that thread's notifications up to item `read` go.
sw.addEventListener("message", (e) => {
  const { thread, read } = e.data ?? {};
  if (!thread) return;
  e.waitUntil(
    sw.registration.getNotifications({ tag: thread }).then((ns) => {
      for (const n of ns) if ((n.data?.item ?? 0) <= read) n.close();
    }),
  );
});
