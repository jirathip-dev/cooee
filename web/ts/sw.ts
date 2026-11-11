const sw = self as unknown as ServiceWorkerGlobalScope;
type Push = { title: string; body: string; tag: string; url: string; item?: number };

const CACHE = "cooee-1";
const KEPT = /^\/($|t\/|threads$|app\.css|chat\.js|md\.js|look\.js|pair\.js|look\/|icon|manifest)/;
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

let turn: Promise<unknown> = Promise.resolve();
const inTurn = (f: () => Promise<unknown>): Promise<unknown> => {
  turn = turn.then(f, f);
  return turn;
};

sw.addEventListener("push", (e) =>
  e.waitUntil(
    inTurn(async () => {
      const n: Push | null = e.data ? e.data.json() : null;
      if (!n) return;
      const open = await sw.clients.matchAll({ type: "window", includeUncontrolled: true });
      const here = (c: WindowClient): boolean =>
        c.visibilityState === "visible" && new URL(c.url).pathname === new URL(n.url, c.url).pathname;
      if (open.some(here)) return;
      await sw.registration.showNotification(n.title, { body: n.body, tag: n.tag, data: n, icon: "/icon-192.png" });
    }),
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

sw.addEventListener("message", (e) => {
  const { thread, read } = e.data ?? {};
  if (!thread) return;
  e.waitUntil(
    inTurn(() =>
      sw.registration.getNotifications({ tag: thread }).then((ns) => {
        for (const n of ns) if ((n.data?.item ?? 0) <= read) n.close();
      }),
    ),
  );
});
