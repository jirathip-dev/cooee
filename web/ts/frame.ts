// The plugin API in a plugin's frame (docs/plugins.md), put before the plugin's module. The frame is sandboxed (an
// opaque origin: no cookies, storage or parent access, no connections), so `cooee.fetch` and `cooee.events` ask the
// parent page, which calls this plugin's backend through cooee's proxy. The theme comes as CSS variables.
type Answer = { status: number; ok: boolean; type?: string | null; text: string };
type Heard = { event: string; data: unknown };
type FromPage = {
  op?: string;
  id?: number;
  vars?: Record<string, string>;
  event?: string;
  data?: unknown;
} & Partial<Answer>;
declare var cooee: {
  fetch(path: string, init?: { method?: string; type?: string; body?: string }): Promise<Answer>;
  events(path: string, on: (e: Heard) => void): () => void;
  unlock(): Promise<void>;
};
{
  let n = 0;
  const waiting = new Map<number | undefined, (a: Answer) => void>();
  const streams = new Map<number | undefined, (e: Heard) => void>();
  let unlocked = (): void => {};
  addEventListener("message", (e) => {
    if (e.source !== parent) return;
    const m: FromPage = e.data ?? {};
    if (m.op === "theme")
      for (const [k, v] of Object.entries(m.vars ?? {})) document.documentElement.style.setProperty(k, v);
    else if (m.op === "response") {
      waiting.get(m.id)?.(m as Answer);
      waiting.delete(m.id);
    } else if (m.op === "event") streams.get(m.id)?.({ event: m.event ?? "", data: m.data });
    else if (m.op === "unlocked") unlocked();
  });
  globalThis.cooee = Object.freeze({
    // `{status, ok, type, text}` of `path` on this plugin's backend.
    fetch: (path: string, { method = "GET", type, body }: { method?: string; type?: string; body?: string } = {}) =>
      new Promise<Answer>((ok) => {
        const id = ++n;
        waiting.set(id, ok);
        parent.postMessage({ op: "fetch", id, path, method, type, body }, "*");
      }),
    // `on({event, data})` for each event of `path`'s stream; returns a function that closes it.
    events(path: string, on: (e: Heard) => void) {
      const id = ++n;
      streams.set(id, on);
      parent.postMessage({ op: "events", id, path }, "*");
      return () => {
        streams.delete(id);
        parent.postMessage({ op: "close", id }, "*");
      };
    },
    // Shows cooee's own "Unlock with Face ID" over the tab (a gated tab answered 403); resolves once unlocked.
    unlock: () =>
      new Promise<void>((ok) => {
        unlocked = ok;
        parent.postMessage({ op: "unlock" }, "*");
      }),
  });
}
