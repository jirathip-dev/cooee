// The plugin API in a plugin's frame (docs/plugins.md), put before the plugin's module. The frame is sandboxed (an
// opaque origin: no cookies, storage or parent access, no connections), so `cooee.fetch` and `cooee.events` ask the
// parent page, which calls this plugin's backend through cooee's proxy. The theme comes as CSS variables.
{
  let n = 0;
  const waiting = new Map(), streams = new Map();
  addEventListener("message", (e) => {
    if (e.source !== parent) return;
    const m = e.data ?? {};
    if (m.op === "theme") for (const [k, v] of Object.entries(m.vars)) document.documentElement.style.setProperty(k, v);
    else if (m.op === "response") { waiting.get(m.id)?.(m); waiting.delete(m.id); }
    else if (m.op === "event") streams.get(m.id)?.({ event: m.event, data: m.data });
  });
  globalThis.cooee = Object.freeze({
    // `{status, ok, type, text}` of `path` on this plugin's backend.
    fetch: (path, { method = "GET", type, body } = {}) => new Promise((ok) => {
      const id = ++n;
      waiting.set(id, ok);
      parent.postMessage({ op: "fetch", id, path, method, type, body }, "*");
    }),
    // `on({event, data})` for each event of `path`'s stream; returns a function that closes it.
    events(path, on) {
      const id = ++n;
      streams.set(id, on);
      parent.postMessage({ op: "events", id, path }, "*");
      return () => { streams.delete(id); parent.postMessage({ op: "close", id }, "*"); };
    },
  });
}
