// cooee's pages, where a browser needs script (the rest is server HTML: the sheets, the new-thread and sign-ins
// cards). The chat: a window of about 9 screens of items around an anchor (slices by id as the reader nears an edge),
// scroll kept on a message rather than pixels, live events over SSE resumed from the last one seen, the outbox
// (sending → sent → failed, tap to retry), the per-thread draft in IndexedDB, approvals that lock once tapped, and
// debounced read marks (which also close the thread's notifications). Every page but pairing registers the service
// worker (sw.ts: push and the last copy of each page); the thread list's 🔔 turns alerts on. On every page (#21):
// short toasts, the loading strip (with a copy's age when it is an old one), refresh (↻, a pull down, coming back
// after 30 s or the network returning), Settings' alerts, devices and report mode; on the pairing page, a code to
// approve on the Mac. It talks to the server only through the JSON and SSE API in docs/api.md.
// States are theme keys (data-st) that look.ts draws: the thread bar, the working pill (its sprite following the
// running tool), the live line and a turn's end.
// A plugin tab hosts the plugin's sandboxed frame (frame.ts is the frame's side).
import { $, make, show } from "./look.js";

type Option = { id: string; name: string; kind: string };
type Item = { id: number; at: number; seq?: number } & (
  | { kind: "you"; text: string; random_id: string }
  | { kind: "agent"; text: string; stop_reason: string }
  | { kind: "tool"; call: string; title: string; status: string; ended?: number }
  | { kind: "approval"; tool: string; options: Option[]; tapped?: string; chosen?: string; plugin?: string }
  | { kind: "plan"; entries: { content: string; status: string }[] }
  | { kind: "ended"; text: string }
);
type Turn = { text: string; state: string; note?: string };
type Slice = { items: Item[]; skipped_before: number; skipped_after: number; turn: Turn; cursor: string; at?: number };

const rid = (): string => Date.now().toString(36) + Math.random().toString(36).slice(2, 10);
const message = (x: unknown): string => (x instanceof Error ? x.message : String(x));
// A failed answer as one plain sentence: cooee's own (marked `cooee-error`), else a general one, never raw JSON.
const why = async (r: Response): Promise<Error> =>
  new Error(
    r.headers.has("cooee-error") ? await r.text() : `cooee couldn't do that (error ${r.status}): reload and try again.`,
  );
async function post<T = unknown>(url: string, body: unknown): Promise<T> {
  const r = await fetch(url, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!r.ok) throw await why(r);
  return r.headers.get("content-type")?.includes("json") ? r.json() : (r.text() as Promise<T>);
}

// A toast: one short line under the header that clears itself (an error stays longer and may wrap); a tap clears it.
// It is a manual popover, so it shows over an open sheet too.
let toastTimer = 0;
function toast(text: string, bad = false): void {
  const t = $("#toast");
  if (!t) return;
  t.textContent = text;
  t.className = bad ? "r" : "g";
  if (!t.matches(":popover-open")) t.showPopover();
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => hide(t), bad ? 6000 : 2500);
}
const hide = (t: HTMLElement): void => {
  if (t.matches(":popover-open")) t.hidePopover();
};
$("#toast")?.addEventListener("click", (e) => hide(e.currentTarget as HTMLElement));

// The loading strip: thin, under the header, never in the way of a tap, and only once a load takes 300 ms (a quick
// one shows nothing); `at` (an old copy's time) adds its age.
const ago = (at: number): string => {
  const m = Math.round((Date.now() - at) / 60000);
  return m < 1 ? "just now" : m < 120 ? `${m}m ago` : `${Math.round(m / 60)}h ago`;
};
const served = Number(document.body.dataset.at);
let copyAt = Date.now() - served > 120000 ? served : 0;
let stripTimer = 0;
function strip(loading: boolean, now = false): void {
  const s = $("#ld");
  if (!s) return;
  clearTimeout(stripTimer);
  if (loading && !now) {
    stripTimer = setTimeout(() => strip(true, true), 300);
    return;
  }
  s.hidden = !loading && !copyAt;
  s.classList.toggle("run", loading);
  const age = $(".ago", s);
  if (age) age.textContent = copyAt ? `as of ${ago(copyAt)}` : "";
}
strip(false);

// Refresh: ↻, a pull down from the very top, the app coming back after 30 s away, or the network coming back. The
// chat reloads its newest slice and stream (page.refresh); the thread list its cards; any other page reloads.
const page: { refresh: (() => Promise<unknown>) | null; top: () => boolean } = { refresh: null, top: () => true };
let refreshing: Promise<unknown> | null = null;
function refresh(): Promise<unknown> | null {
  if (refreshing || location.pathname === "/pair") return refreshing;
  const rf = $("#rf");
  rf?.classList.add("rfs");
  strip(true);
  refreshing = (async () => {
    if (page.refresh) return page.refresh();
    const list = $("main.list");
    if (!list) return location.reload();
    const r = await fetch("/threads");
    if (!r.ok) throw await why(r);
    copyAt = Number(r.headers.get("cooee-at")) || 0;
    drawCards(list, await r.json());
  })()
    .catch((x) => toast(`couldn't refresh: ${message(x)}`, true))
    .finally(() => {
      rf?.classList.remove("rfs");
      strip(false);
      refreshing = null;
    });
  return refreshing;
}
$("#rf")?.addEventListener("click", refresh);

// The thread list's cards from `GET /threads`, drawn as the server draws them (lib.rs `threads`).
type Row = {
  id: string;
  title: string;
  agent: string;
  state: string;
  last: string;
  at: number;
  unread: boolean;
  workflow?: { role: string; round: string };
};
type Card = { folder: string; rows: Row[]; done?: Row[] };
const STATE: Record<string, string> = {
  blocked: "needs",
  working: "working",
  error: "fail",
  done: "done",
  ended: "ended",
};
const age = (at: number): string => {
  const s = Math.max(0, Math.floor(Date.now() / 1000) - Math.floor(at / 1000));
  return s < 60
    ? "now"
    : s < 3600
      ? `${Math.floor(s / 60)}m`
      : s < 86400
        ? `${Math.floor(s / 3600)}h`
        : `${Math.floor(s / 86400)}d`;
};
function rowOf(r: Row): HTMLElement {
  const a = make("a", r.unread ? "tr" : "tr rd") as HTMLAnchorElement;
  const st = make("b");
  const tp = make("span", "tp");
  a.href = `/t/${r.id}`;
  st.dataset.st = STATE[r.state] ?? "";
  tp.append(st, r.last);
  a.append(
    make("i", "ud", r.unread ? "●" : ""),
    make("span", "tt", r.title),
    make(
      "span",
      "ta",
      r.workflow ? `${r.workflow.role} · round ${r.workflow.round} · ${age(r.at)}` : `${r.agent} · ${age(r.at)}`,
    ),
    tp,
  );
  return a;
}
function drawCards(list: HTMLElement, cards: Card[]): void {
  for (const e of list.querySelectorAll(":scope > .fc, :scope > .emp")) e.remove();
  if (!cards.length) {
    const emp = make("div", "emp");
    const fig = make("span");
    const w = make("p");
    fig.dataset.s = "empty";
    w.dataset.w = "empty-threads";
    w.dataset.plain = "";
    emp.append(fig, w);
    list.insertBefore(emp, $("#new"));
  }
  for (const c of cards) {
    const card = make("section", "fc");
    const fh = make("div", "fh");
    fh.append(make("b", "", c.folder), make("span", "ds", `${c.rows.length} threads`));
    card.append(fh, ...c.rows.slice(0, c.done ? undefined : 3).map(rowOf));
    if (c.done?.length) {
      const done = make("details");
      done.append(make("summary", "", `${c.done.length} done today`), ...c.done.map(rowOf));
      card.append(done);
    } else if (!c.done && c.rows.length > 3) {
      const more = make("details");
      more.append(make("summary", "", `${c.rows.length - 3} older threads`), ...c.rows.slice(3).map(rowOf));
      card.append(more);
    }
    list.append(card);
  }
}
let pullY: number | null = null;
let hiddenAt = 0;
addEventListener(
  "touchstart",
  (e) => {
    const scroller = $("#msgs") ?? document.scrollingElement;
    const top = (scroller?.scrollTop ?? 0) <= 0 && page.top() && !$("#sheet :popover-open");
    pullY = top ? (e.touches[0]?.clientY ?? null) : null;
  },
  { passive: true },
);
addEventListener(
  "touchmove",
  (e) => {
    if (pullY !== null && (e.touches[0]?.clientY ?? 0) - pullY > 70) {
      pullY = null;
      refresh();
    }
  },
  { passive: true },
);
document.addEventListener("visibilitychange", () => {
  if (document.hidden) hiddenAt = Date.now();
  else if (hiddenAt && Date.now() - hiddenAt > 30000) refresh();
});
addEventListener("online", refresh);
addEventListener("pageshow", (e) => {
  if (e.persisted) refresh();
});

// The thread list's form (open on `#new`, by CSS): start a thread, then open it.
const form = $<HTMLFormElement>("#new");
const say = (e: HTMLElement | null, text: string): void => {
  if (!e) return;
  e.textContent = text;
  e.hidden = !text;
};
if (form) {
  const start = $<HTMLButtonElement>("button[type=submit]", form);
  form.addEventListener("submit", async (e) => {
    e.preventDefault();
    const f = new FormData(form);
    const agent = f.get("agent");
    // One id per start until it succeeds, so a retry after a timeout adopts the thread it started.
    form.dataset.rid ||= rid();
    const body = {
      agent,
      account: f.get(`account:${agent}`),
      profile: f.get("profile"),
      folder: f.get("folder"),
      text: f.get("text"),
      random_id: form.dataset.rid,
    };
    if (start) start.disabled = true;
    try {
      location.href = `/t/${(await post<{ id: string }>("/threads", body)).id}`;
    } catch (x) {
      say($("#new-err"), message(x));
      if (start) start.disabled = false;
    }
  });
}

// A folder picked picks its last-used agent profile (Plain when it has none).
const pf = $<HTMLSelectElement>("select[name=profile]");
if (pf && form)
  form.addEventListener("change", (e) => {
    const t = e.target as HTMLInputElement;
    if (t.name !== "folder") return;
    const last: Record<string, string> = JSON.parse(pf.dataset.last || "{}");
    const id = last[t.value] ?? "";
    if ([...pf.options].some((o) => o.value === id)) pf.value = id;
  });

// The folder picker: the search shows the folders whose name has it (up to 2 levels in), else the first level;
// [+ new folder] makes one inside the folder picked, and picks it.
const fq = $<HTMLInputElement>("#fq");
if (fq && form) {
  fq.oninput = () => {
    const q = fq.value.trim().toLowerCase();
    for (const l of form.querySelectorAll<HTMLElement>(".fl label")) {
      const base = l.textContent?.trim().split("/").pop()?.toLowerCase() ?? "";
      l.hidden = q ? !base.includes(q) : Number(l.dataset.d) > 1;
    }
  };
  const mkdir = $<HTMLButtonElement>("#mkdir");
  if (mkdir)
    mkdir.onclick = async () => {
      const at = form.querySelector(".fl :checked")?.parentElement;
      const parent = at && $<HTMLInputElement>("input", at);
      if (!at || !parent) return;
      const label = at.textContent?.trim() ?? "";
      const name = prompt(`New folder in ${label}:`);
      if (!name) return;
      try {
        const row = at.cloneNode(true) as HTMLElement;
        const input = $<HTMLInputElement>("input", row);
        if (!input) return;
        input.value = await post<string>("/folders/new", { parent: parent.value, name });
        if (row.lastChild) row.lastChild.textContent = `${label}/${name}`;
        row.dataset.d = String(Math.min(2, Number(at.dataset.d) + 1));
        row.hidden = false;
        say($("#new-err"), "");
        at.after(row);
        input.checked = true;
      } catch (x) {
        say($("#new-err"), message(x));
      }
    };
}

// An empty picker's hint: its copy button puts the fix line on the clipboard.
document.addEventListener("click", (e) => {
  const b = (e.target as Element).closest?.<HTMLElement>("[data-copy]");
  if (!b?.dataset.copy) return;
  navigator.clipboard?.writeText(b.dataset.copy).then(
    () => {
      b.textContent = "copied";
    },
    () => {},
  );
});

// The sign-ins card (open on `#signins`, by CSS). [sign in] starts the provider's own login for that account in a
// shed terminal ([+ account]: in a new account's folder); its screen (redacted by the server) is drawn every second
// with its links tappable, a code typed below goes to it, and once it ends the page reloads with the accounts' state.
// [sign out] (after a confirm) and [make default] reload it too.
type LoginView = { state: string; screen: string; links: string[] };
for (const row of document.querySelectorAll<HTMLElement>("#signins .pv")) {
  const api = `/logins/${row.dataset.provider}`;
  const lg = $(".lg", row);
  const out = $(".out", row);
  const code = $<HTMLInputElement>("input", row);
  let timer = 0;
  const tell = (m: string): void => {
    let w = $(".wy", row);
    if (!w) {
      w = make("p", "ds wy");
      $(".fh", row)?.after(w);
    }
    w.textContent = m;
  };
  const draw = (v: LoginView): void => {
    if (v.state === "ended") {
      clearInterval(timer);
      location.reload();
    }
    if (v.state !== "running" || !lg || !out) return;
    lg.hidden = false;
    for (const b of row.querySelectorAll<HTMLElement>(".si, .ad")) b.hidden = true;
    out.replaceChildren();
    let rest = v.screen;
    for (const url of v.links) {
      const i = rest.indexOf(url);
      if (i < 0) continue;
      const a = make("a", "", url) as HTMLAnchorElement;
      a.href = url;
      a.target = "_blank";
      a.rel = "noopener";
      out.append(rest.slice(0, i), a);
      rest = rest.slice(i + url.length);
    }
    out.append(rest);
  };
  const poll = async (): Promise<void> => {
    const r = await fetch(api);
    if (r.ok) draw(await r.json());
  };
  const start = async (body: object, state: HTMLElement | null): Promise<void> => {
    try {
      await post(api, body);
    } catch (x) {
      return tell(message(x));
    }
    if (state) {
      state.className = "a";
      state.textContent = "signing in";
    }
    clearInterval(timer);
    timer = setInterval(poll, 1000);
    poll();
  };
  const reload = (p: Promise<unknown>): void => {
    p.then(
      () => location.reload(),
      (x) => tell(message(x)),
    );
  };
  const add = $(".ad", row);
  if (add) add.onclick = () => start({ new: true }, null);
  for (const ac of row.querySelectorAll<HTMLElement>(".ac")) {
    const account = ac.dataset.account ?? "";
    const label = $("b", ac)?.textContent ?? "";
    const si = $(".si", ac);
    const so = $(".sg", ac);
    const df = $(".df", ac);
    if (si) si.onclick = () => start({ account }, $(".fh span", ac));
    if (so)
      so.onclick = () => {
        if (confirm(`Sign ${label} out?${account ? " Its folder is removed." : ""}`))
          reload(post(`${api}/signout`, { account }));
      };
    if (df) df.onclick = () => reload(post(`${api}/default`, { account }));
  }
  const tw = $<HTMLFormElement>("form", row);
  if (tw)
    tw.onsubmit = async (e) => {
      e.preventDefault();
      try {
        await post(`${api}/send`, { text: code?.value ?? "" });
        if (code) code.value = "";
        poll();
      } catch (x) {
        tell(message(x));
      }
    };
  const cn = $(".cn", row);
  if (cn) cn.onclick = () => post(`${api}/cancel`, {}).then(poll, poll);
}

// Not while pairing: a worker installing and claiming the page as the pair redirects can leave a home-screen app on
// a bare background until a reload (the first run, #25).
if ("serviceWorker" in navigator && location.pathname !== "/pair")
  navigator.serviceWorker.register("/sw.js").catch(() => {});
// Closes thread `thread`'s notifications up to item `read`.
const cleared = (thread: string, read: number): void => {
  navigator.serviceWorker?.ready.then((r) => r.active?.postMessage({ thread, read }));
};

// Alerts, truthfully: on only when this browser has a subscription the server keeps for this device. The 🔔 and a
// dot on ⚙ show where push works and isn't on; Settings' alerts section turns them on or off and sends a test.
type Alerts = { state: "unsupported" | "blocked" | "off" | "on"; sub?: PushSubscription | null };
async function alertState(): Promise<Alerts> {
  if (!("PushManager" in window) || !navigator.serviceWorker) return { state: "unsupported" };
  if (Notification.permission === "denied") return { state: "blocked" };
  const reg = await navigator.serviceWorker.ready;
  const sub = await reg.pushManager.getSubscription();
  if (!sub || Notification.permission !== "granted") return { state: "off", sub };
  const mine: string[] = await fetch("/push/endpoints").then(
    (r) => (r.ok ? r.json() : []),
    () => [],
  );
  return { state: mine.includes(sub.endpoint) ? "on" : "off", sub };
}
async function alertsOn(): Promise<void> {
  if ((await Notification.requestPermission()) !== "granted") throw new Error("notifications are off for this app");
  const reg = await navigator.serviceWorker.ready;
  const r = await fetch("/push/key");
  if (!r.ok) throw await why(r);
  const key: string = (await r.json()).key.replace(/-/g, "+").replace(/_/g, "/");
  const sub = await reg.pushManager.subscribe({
    userVisibleOnly: true,
    applicationServerKey: Uint8Array.from(atob(key), (c) => c.charCodeAt(0)),
  });
  const { endpoint, keys } = sub.toJSON();
  await post("/push/subscribe", { endpoint, ...keys });
}
async function alertsOff(): Promise<void> {
  const { sub } = await alertState();
  if (!sub) return;
  await post("/push/unsubscribe", { endpoint: sub.endpoint });
  await sub.unsubscribe();
}
async function alertMarks(): Promise<Alerts["state"]> {
  const { state } = await alertState();
  const gear = $("header [data-open=settings]");
  const bell = $("#alerts");
  const off = state === "off" || state === "blocked";
  if (gear) {
    gear.toggleAttribute("data-dot", off);
    gear.setAttribute("aria-label", off ? "settings · alerts are off" : "settings");
  }
  if (bell) bell.hidden = state !== "off";
  return state;
}
const ALERTS: Record<Alerts["state"], string> = {
  on: "alerts are on",
  off: "alerts are off",
  blocked: "notifications are blocked for this app: allow them in the phone's Settings",
  unsupported: "this browser can't show alerts: on iPhone, add cooee to the Home Screen and open it there",
};
async function alertsDo(f: () => Promise<void>, ok: string): Promise<void> {
  try {
    await f();
    toast(ok);
  } catch (x) {
    toast(`alerts: ${message(x)}`, true);
  }
  await alertMarks();
  fillAlerts();
}
async function fillAlerts(): Promise<void> {
  const box = $("#s-alerts-box");
  if (!box) return;
  const state = await alertState().then(
    (a) => a.state,
    () => "unsupported" as const,
  );
  const row = make("div", "lrw");
  row.append(make("span", state === "on" ? "g" : "m", ALERTS[state]));
  const act = (label: string, f: () => unknown): void => {
    const b = make("button", "", label) as HTMLButtonElement;
    b.type = "button";
    b.onclick = f;
    row.append(b);
  };
  if (state === "off") act("turn on", () => alertsDo(alertsOn, "alerts are on"));
  if (state === "on") {
    act("send a test", () =>
      post<string>("/push/test", {}).then(
        (t) => toast(t),
        (x) => toast(message(x), true),
      ),
    );
    act("turn off", () => alertsDo(alertsOff, "alerts are off"));
  }
  box.replaceChildren(row);
}
const bell = $("#alerts");
if (bell) bell.onclick = () => alertsDo(alertsOn, "alerts are on");
if (location.pathname !== "/pair") alertMarks().catch(() => {});

// Settings' devices section: each paired device, this one marked, with [remove] (removing this one unpairs it).
type Device = { id: string; label: string; created: number; used: number; me: boolean };
async function fillDevices(): Promise<void> {
  const box = $("#s-devices-box");
  if (!box) return;
  const r = await fetch("/devices").catch(() => null);
  if (!r?.ok) return box.replaceChildren(make("p", "ds", "devices show when online"));
  const day = (at: number): string => new Date(at).toLocaleDateString();
  const list: Device[] = await r.json();
  box.replaceChildren(
    ...list.map((d) => {
      const row = make("div", "lrw");
      const b = make("button", "cn", "remove") as HTMLButtonElement;
      row.append(
        make("span", "", `${d.label} · ${d.id}${d.me ? " · this device" : ""}`),
        make("span", "ds", `paired ${day(d.created)} · seen ${day(d.used)}`),
      );
      b.type = "button";
      b.setAttribute("aria-label", `remove ${d.label} ${d.id}`);
      b.onclick = async () => {
        if (!confirm(d.me ? "Unpair this device? It goes back to pairing." : `Unpair ${d.label} ${d.id}?`)) return;
        try {
          toast(await post<string>("/devices/remove", { id: d.id }));
        } catch (x) {
          return toast(message(x), true);
        }
        if (d.me) location.href = "/pair";
        else fillDevices();
      };
      row.append(b);
      return row;
    }),
  );
}
// Settings' plugins section (docs/plugins.md): each installed plugin with its version, commit, source and state, on/off,
// [update], [logs], [improve] (or "improve via an issue" for a first-party one), [roll back] and [remove]; [+ add]
// lists the index, or takes a git URL ("not reviewed"). Adding, updating and approving show what will run, to confirm.
type Plugin = {
  name: string;
  version: string;
  commit: string;
  listed: boolean;
  state: string;
  enabled: boolean;
  first_party: boolean;
  improving?: string | null;
  previous?: string | null;
  tabs: string[];
};
type Plugins = {
  installed: Plugin[];
  index: { name: string; description: string }[];
  jobs: Record<string, string>;
  issues: string;
};
let plugAdd = false;
let plugPoll = 0;
async function plugDo(path: string, body: object, then?: (r: never) => Promise<void> | void): Promise<void> {
  try {
    const r = await post<never>(`/plugins/${path}`, body);
    if (then) await then(r);
    else toast(r);
  } catch (x) {
    toast(message(x), true);
  }
  fillPlugins();
}
const plugPlan = (body: { name?: string; at?: string; source?: string }, act: "add" | "update"): Promise<void> =>
  plugDo("plan", body, async (plan: { commit: string; text: string }) => {
    if (act === "update" && plan.commit === body.at) return toast(`${body.name} is up to date`);
    if (!confirm(`${act === "add" ? "Add" : "Update to"} this?\n\n${plan.text}`)) return;
    toast(await post<string>(`/plugins/${act}`, { ...body, commit: plan.commit }));
  });
async function fillPlugins(): Promise<void> {
  const box = $("#s-plugins-box");
  if (!box) return;
  const r = await fetch("/plugins").catch(() => null);
  if (!r?.ok) return box.replaceChildren(make("p", "ds", "plugins show when online"));
  const { installed, index, jobs, issues }: Plugins = await r.json();
  const btn = (text: string, label: string, f: () => unknown): HTMLButtonElement => {
    const b = make("button", "cn", text) as HTMLButtonElement;
    b.type = "button";
    b.setAttribute("aria-label", label);
    b.onclick = f;
    return b;
  };
  const rows = installed.map((p) => {
    const head = make("div", "lrw");
    const acts = make("div", "pga");
    const log = make("pre", "plg");
    head.append(
      make("span", "", `${p.name} · ${p.version} · ${p.commit.slice(0, 7)} · ${p.listed ? "index" : "not reviewed"}`),
      make("span", "ds", p.state),
    );
    const seg = make("div", "seg");
    seg.setAttribute("role", "radiogroup");
    seg.setAttribute("aria-label", `${p.name} on or off`);
    for (const on of [true, false]) {
      const b = btn(on ? "on" : "off", `${p.name} ${on ? "on" : "off"}`, () => plugDo("enable", { name: p.name, on }));
      b.className = "";
      b.setAttribute("role", "radio");
      b.setAttribute("aria-checked", String(p.enabled === on));
      seg.append(b);
    }
    log.hidden = true;
    acts.append(
      seg,
      btn("update", `update ${p.name}`, () => plugPlan({ name: p.name, at: p.commit }, "update")),
      btn("logs", `logs of ${p.name}`, async () => {
        log.hidden = !log.hidden;
        if (!log.hidden)
          log.textContent = await (await fetch(`/plugins/log?name=${encodeURIComponent(p.name)}`)).text();
      }),
    );
    if (p.first_party) {
      const a = make("a", "", "improve via an issue") as HTMLAnchorElement;
      a.href = issues;
      a.target = "_blank";
      acts.append(a);
    } else if (p.improving) {
      const a = make("a", "", "its chat") as HTMLAnchorElement;
      a.href = `/t/${p.improving}`;
      acts.append(
        a,
        btn("approve", `approve ${p.name}'s improvement`, () => {
          const ask = `Build ${p.name}'s cooee-improve branch and swap it in? The build running now stays, to roll back to.`;
          if (confirm(ask)) plugDo("approve", { name: p.name });
        }),
      );
    } else {
      acts.append(
        btn("improve", `improve ${p.name}`, () => {
          const text = prompt(
            `What should change in ${p.name}? An agent works on a branch of its checkout; nothing runs until you approve.`,
          );
          if (text !== null)
            plugDo("improve", { name: p.name, text }, (t: { thread: string }) => {
              location.href = `/t/${t.thread}`;
            });
        }),
      );
    }
    const previous = p.previous;
    if (previous)
      acts.append(
        btn("roll back", `roll back ${p.name}`, () => {
          if (confirm(`Go back to ${p.name} ${previous.slice(0, 7)}?`)) plugDo("rollback", { name: p.name });
        }),
      );
    acts.append(
      btn("remove", `remove ${p.name}`, () => {
        if (confirm(`Remove ${p.name}? Its tabs go; its checkout stays on the Mac.`))
          plugDo("remove", { name: p.name });
      }),
    );
    return [head, acts, log];
  });
  const busy = Object.entries(jobs)
    .filter(([n]) => !installed.some((p) => p.name === n))
    .map(([n, j]) => make("p", j.startsWith("failed") ? "err" : "ds", `${n}: ${j}`));
  const add = btn(plugAdd ? "close" : "+ add", plugAdd ? "close the plugin list" : "add a plugin", () => {
    plugAdd = !plugAdd;
    fillPlugins();
  });
  const more: HTMLElement[] = [];
  if (plugAdd) {
    for (const l of index) {
      const row = make("div", "lrw");
      const have = installed.some((p) => p.name === l.name);
      row.append(
        make("span", "", `${l.name} · ${l.description}`),
        have ? make("span", "ds", "installed") : btn("add", `add ${l.name}`, () => plugPlan({ source: l.name }, "add")),
      );
      more.push(row);
    }
    const form = make("form", "lrw") as HTMLFormElement;
    const url = make("input") as HTMLInputElement;
    url.setAttribute("aria-label", "git URL");
    url.placeholder = "https://github.com/you/plugin";
    const go = btn("add by URL", "add by URL", () => {});
    go.type = "submit";
    form.append(url, go);
    form.onsubmit = (e) => {
      e.preventDefault();
      if (url.value.trim()) plugPlan({ source: url.value.trim() }, "add");
    };
    more.push(
      form,
      make(
        "p",
        "ds",
        "not reviewed: a plugin from a URL isn't in cooee's index, and its backend runs as you on this Mac",
      ),
    );
  }
  box.replaceChildren(...rows.flat(), ...busy, add, ...more);
  clearTimeout(plugPoll);
  // While a job runs or a backend is starting, the section follows it.
  const settling =
    Object.values(jobs).some((j) => j.endsWith("…")) ||
    installed.some((p) => ["starting", "not running"].includes(p.state));
  if (settling && box.isConnected) plugPoll = setTimeout(fillPlugins, 1500);
}
addEventListener("cooee-sheet", (e) => {
  if ((e as CustomEvent).detail === "settings") {
    fillAlerts();
    fillDevices();
    fillPlugins();
  }
});

// Report mode (Settings → report a problem, or `/report` sent in a thread): the next tap picks an element, outlined;
// the note box at the bottom sends it with the page, the element and the look; `cooee reports` lists them.
const where = (start: Element | null): string => {
  const out: string[] = [];
  for (let n = start; n && out.length < 6; n = n.parentElement) {
    const classes = [...n.classList].filter((c) => c !== "repo").map((c) => `.${c}`);
    out.unshift(n.tagName.toLowerCase() + (n.id ? `#${n.id}` : "") + classes.join(""));
  }
  return out.join(" > ");
};
function reportMode(): void {
  show("");
  document.body.classList.add("rep");
  toast("report: tap the part that's wrong");
  const pick = (e: MouseEvent): void => {
    e.preventDefault();
    e.stopPropagation();
    removeEventListener("click", pick, true);
    const t = e.target as Element;
    const bar = make("form", "repf");
    const note = make("input") as HTMLInputElement;
    const no = make("button", "cn", "cancel") as HTMLButtonElement;
    t.classList.add("repo");
    note.setAttribute("aria-label", "what's wrong");
    note.placeholder = "what's wrong?";
    no.type = "button";
    const done = (): void => {
      bar.remove();
      t.classList.remove("repo");
      document.body.classList.remove("rep");
    };
    no.onclick = done;
    bar.onsubmit = async (ev) => {
      ev.preventDefault();
      const look = Object.entries(document.documentElement.dataset)
        .map(([k, v]) => `${k}=${v}`)
        .join(" ");
      try {
        const body = {
          note: note.value,
          path: location.pathname + location.hash,
          target: where(t),
          html: t.outerHTML.slice(0, 600),
          look,
          viewport: `${innerWidth}x${innerHeight}`,
        };
        toast(await post<string>("/report", body));
        done();
      } catch (x) {
        toast(message(x), true);
      }
    };
    bar.append(note, make("button", "", "send"), no);
    document.body.append(bar);
    note.focus();
  };
  setTimeout(() => addEventListener("click", pick, true));
}
addEventListener("click", (e) => {
  if ((e.target as Element).closest?.("[data-report]")) reportMode();
});
if (location.hash === "#report") reportMode();

// Pairing the other way round: the phone asks for a code, shows it, and polls until `cooee pair <code>` on the Mac
// approves it; then it is paired and opens the thread list.
const go = $<HTMLButtonElement>("#ask-go");
if (go) {
  let claim = "";
  let timer = 0;
  const wait = $("#ask-wait");
  const err = $("#ask-err");
  const reset = (text = ""): void => {
    clearInterval(timer);
    if (wait) wait.hidden = true;
    go.hidden = false;
    say(err, text);
  };
  go.onclick = async () => {
    try {
      const r = await post<{ code: string; claim: string }>("/pair/request", {});
      const cmd = `cooee pair ${r.code}`;
      claim = r.claim;
      const [code, line, copy] = [$("#ask-code"), $("#ask-cmd"), $("#ask-copy")];
      if (code) code.textContent = r.code;
      if (line) line.textContent = cmd;
      if (copy) copy.dataset.copy = cmd;
      if (wait) wait.hidden = false;
      go.hidden = true;
      say(err, "");
      timer = setInterval(async () => {
        const body = JSON.stringify({ claim });
        const headers = { "content-type": "application/json" };
        const c = await fetch("/pair/claim", { method: "POST", headers, body }).catch(() => null);
        if (c?.status === 200) {
          clearInterval(timer);
          location.replace("/");
        } else if (c?.status === 410) reset(await c.text());
      }, 2000);
    } catch (x) {
      reset(message(x));
    }
  };
  const cancel = $("#ask-cancel");
  if (cancel)
    cancel.onclick = () => {
      post("/pair/cancel", { claim }).catch(() => {});
      reset();
    };
}

// A plugin tab: the frame (sandboxed, see frame.ts) asks this page to fetch or stream paths of its own backend only
// (/p/<name>/api/…; anything else is refused), gets the theme's CSS variables, and its URL's #hash (a push's link).
type Ask = { op?: string; id?: number; path?: unknown; method?: string; type?: string; body?: string };
const frame = $<HTMLIFrameElement>("iframe.pf");
if (frame) {
  const base = `/p/${frame.dataset.plugin}/api`;
  const streams = new Map<number | undefined, EventSource>();
  const to = (m: object): void => frame.contentWindow?.postMessage(m, "*");
  const own = (path: unknown): URL | null => {
    if (typeof path !== "string" || !path.startsWith("/")) return null;
    const u = new URL(base + path, location.origin);
    return u.origin === location.origin && u.pathname.startsWith(`${base}/`) ? u : null;
  };
  addEventListener("message", async (e) => {
    if (e.source !== frame.contentWindow) return;
    const m: Ask = e.data ?? {};
    const u = own(m.path);
    if (m.op === "fetch") {
      const answer = (r: object): void => to({ op: "response", id: m.id, ...r });
      if (!u || !["GET", "POST"].includes(m.method ?? ""))
        return answer({ status: 403, ok: false, text: "not this plugin's backend" });
      try {
        const init: RequestInit = { headers: m.type ? { "content-type": m.type } : {} };
        if (m.method) init.method = m.method;
        if (m.body !== undefined) init.body = m.body;
        const r = await fetch(u, init);
        answer({ status: r.status, ok: r.ok, type: r.headers.get("content-type"), text: await r.text() });
      } catch (x) {
        answer({ status: 0, ok: false, text: message(x) });
      }
    } else if (m.op === "events") {
      if (!u) return to({ op: "event", id: m.id, event: "error", data: "not this plugin's backend" });
      const es = new EventSource(u);
      es.onmessage = (ev) => to({ op: "event", id: m.id, ...JSON.parse(ev.data) });
      streams.set(m.id, es);
    } else if (m.op === "close") {
      streams.get(m.id)?.close();
      streams.delete(m.id);
    }
  });
  const vars = [
    "--bg",
    "--card",
    "--edge",
    "--line",
    "--fg",
    "--mute",
    "--l",
    "--link",
    "--g",
    "--a",
    "--r",
    "--p",
    "--font",
    "--fs",
    "--fs-meta",
    "--r-card",
    "--r-pill",
  ];
  const theme = (): void => {
    const css = getComputedStyle(document.documentElement);
    to({ op: "theme", vars: Object.fromEntries(vars.map((v) => [v, css.getPropertyValue(v).trim()])) });
  };
  frame.onload = theme;
  // look.ts sets the scheme's colours on <html>: a new pick or a system switch goes to the frame too.
  new MutationObserver(theme).observe(document.documentElement, { attributes: true });
  matchMedia("(prefers-color-scheme: light)").addEventListener("change", theme);
  frame.src = (frame.dataset.src ?? "") + location.hash;
  // An installed plugin's update, roll back or improvement reloads just this tab's frame.
  let built = "";
  setInterval(async () => {
    const r = await fetch("/plugins").catch(() => null);
    const list: Plugins | null = r?.ok ? await r.json() : null;
    const p = list?.installed.find((p) => p.tabs.includes(frame.dataset.plugin ?? ""));
    if (p && built && p.commit !== built) frame.src = (frame.dataset.src ?? "") + location.hash;
    if (p) built = p.commit;
  }, 5000);
}

const main = $("main.chat");
if (main) chat(main);

function chat(main: HTMLElement): void {
  const id = main.dataset.thread ?? "";
  const agent = main.dataset.agent ?? "";
  const list = $("#msgs") as HTMLElement;
  const box = $<HTMLTextAreaElement>("#box") as HTMLTextAreaElement;
  const api = `/threads/${id}`;
  const SLICE = 20;
  const KEEP = 9;
  const EDGE = 2;
  let before = 0;
  let after = 0;
  let cursor = "";
  let es: EventSource | null = null;
  let live: HTMLElement | null = null;
  let state = main.dataset.state ?? "";
  let since = Date.now();
  let note = "";
  let read = Number(main.dataset.read);
  let seen = read;
  let readTimer = 0;
  // Sends and history loads of this thread run one after another.
  let chain: Promise<unknown> = Promise.resolve();
  const inOrder = <T>(f: () => Promise<T>): Promise<T> => {
    const next = chain.then(f, f);
    chain = next.catch(() => {});
    return next;
  };
  const screen = (): number => list.clientHeight || innerHeight;
  const atBottom = (): boolean => after === 0 && list.scrollHeight - list.scrollTop - list.clientHeight < 8;
  const rowOf = (itemId: number | string): HTMLElement | null => list.querySelector(`[data-id="${itemId}"]`);
  const rows = (): HTMLElement[] => [...list.children].filter((r): r is HTMLElement => !!(r as HTMLElement).dataset.id);
  const ids = (): number[] => rows().map((r) => Number(r.dataset.id));
  const time = (at: number): string => new Date(at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  const clock = (ms: number): string => {
    const s = Math.max(0, Math.floor(ms / 1000));
    return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
  };
  const el = make;
  const md = (row: HTMLElement): HTMLElement => $(".md", row) as HTMLElement;

  // Scroll is kept on a message: the first row in view and its offset are restored after any change of layout.
  function keep(change: () => void): void {
    const stick = atBottom();
    const anchor = rows().find((r) => r.offsetTop + r.offsetHeight > list.scrollTop);
    const offset = anchor ? anchor.offsetTop - list.scrollTop : 0;
    change();
    if (stick && after === 0) list.scrollTop = list.scrollHeight;
    else if (anchor?.isConnected) list.scrollTop = anchor.offsetTop - offset;
  }

  // The working sprite follows the tool that runs: reading and searching, running a command, else editing.
  const toolKind = (title: string): string =>
    /^(read|search|grep|glob|find|list|fetch|view)\b/i.test(title)
      ? "read"
      : /^(\$|run|bash|exec|execute|terminal)\b/i.test(title)
        ? "run"
        : "edit";
  const KEY: Record<string, string> = {
    blocked: "needs",
    working: "working",
    done: "done",
    error: "fail",
    ended: "ended",
  };

  // A turn's end: one quiet line under its last block, `[sprite] in the stable · done 09:21 · 9 min · 6 tools`.
  function turnEnd(): void {
    $(".te", list)?.remove();
    if ((state !== "done" && state !== "error") || after > 0) return;
    const all = rows();
    const me = all.findLastIndex((r) => r.classList.contains("me"));
    const first = all[me];
    const last = all.at(-1);
    if (!first || !last) return;
    const tools = all.slice(me).filter((r) => r.classList.contains("run")).length;
    const mins = Math.round((Number(last.dataset.at) - Number(first.dataset.at)) / 60000);
    const te = el("li", "te");
    const st = el("span", "", "");
    st.dataset.st = KEY[state] ?? "";
    st.dataset.plain = "";
    const more =
      state === "error"
        ? [note]
        : [mins >= 1 ? `${mins} min` : "", tools ? `${tools} tool${tools > 1 ? "s" : ""}` : ""];
    te.append(st, el("span", "ct", [time(Number(last.dataset.at)), ...more].filter(Boolean).join(" · ")));
    list.append(te);
  }

  function fill(row: HTMLElement, it: Item): HTMLElement {
    row.replaceChildren();
    row.dataset.id = String(it.id);
    row.dataset.at = String(it.at);
    if (it.kind === "you") {
      row.className = "blk me";
      row.append(el("div", "md", it.text), el("span", "ct", time(it.at)));
    } else if (it.kind === "agent") {
      row.className = "blk ai";
      row.append(el("b", "k", agent), el("div", "md", it.text));
      if (it.stop_reason !== "end_turn") row.append(el("span", "ct", it.stop_reason));
    } else if (it.kind === "tool") {
      const s = it.status;
      const ok = s === "completed";
      const bad = s === "failed" || s === "cancelled";
      row.className = "blk run";
      const ts = el("span", `ts ${ok ? "g" : bad ? "r" : "a"}`, ok ? "✓" : bad ? `✕ ${s}` : "⋯");
      const pst = $("#pst");
      if (!ok && !bad) {
        ts.dataset.since = String(it.at);
        if (pst) pst.dataset.tool = toolKind(it.title);
      }
      row.append(el("span", "k", "tool"), el("span", "cmd", it.title), ts);
    } else if (it.kind === "approval") {
      row.className = "blk ap";
      row.append(el("b", "k", it.plugin ?? agent), el("div", "md", it.tool));
      const chosen = it.chosen ?? it.tapped;
      if (chosen && it.chosen) {
        const o = it.options.find((o) => o.id === chosen);
        row.append(el("p", "ans", `✓ ${o ? o.name : chosen}`));
      } else {
        const chc = el("div", "chc");
        for (const o of it.options) {
          const b = el("button", "", o.name) as HTMLButtonElement;
          b.type = "button";
          b.disabled = !!chosen;
          b.onclick = () => approve(row, it.id, o.id);
          chc.append(b);
        }
        if (chosen) chc.classList.add("busy");
        row.append(chc);
      }
    } else if (it.kind === "plan") {
      row.className = "blk plan";
      const ol = el("ol");
      for (const e of it.entries) ol.append(el("li", e.status, e.content));
      row.append(ol);
    } else {
      row.className = "qs";
      row.textContent = it.text;
    }
    return row;
  }

  // A tap locks the row's buttons (spinner) until the server answers; the server sends it to shed once.
  async function approve(row: HTMLElement, item: number, option: string): Promise<void> {
    const chc = $(".chc", row);
    if (!chc) return;
    const lock = (on: boolean): void => {
      chc.classList.toggle("busy", on);
      for (const b of chc.querySelectorAll("button")) b.disabled = on;
    };
    lock(true);
    // The answer comes back over the event stream (tapped, then chosen), never older than what is shown.
    try {
      await post(`${api}/approve`, { item, option });
    } catch (x) {
      toast(message(x), true);
      lock(false);
    }
  }

  // An item, new or changed: replaced in place, appended at the newest end, or counted beyond the window.
  function put(it: Item): void {
    const row = rowOf(it.id);
    if (row) {
      keep(() => fill(row, it));
      return;
    }
    if (after > 0) {
      after++;
      return;
    }
    keep(() => {
      $(".te", list)?.remove();
      let row = it.kind === "you" ? list.querySelector<HTMLElement>(`[data-rid="${it.random_id}"]`) : null;
      // The final reply takes over the streaming row's node.
      if (!row && it.kind === "agent" && live) {
        row = live;
        live = null;
      }
      if (row) {
        delete row.dataset.rid;
        fill(row, it);
      } else list.insertBefore(fill(el("li"), it), live);
      if (live) list.append(live);
      turnEnd();
    });
    markRead();
  }

  function text(chunk: string): void {
    if (after > 0) return;
    keep(() => {
      if (!live) {
        $(".te", list)?.remove();
        live = el("li", "blk ai live");
        live.dataset.cid = rid();
        const stop = el("button", "stop", "stop") as HTMLButtonElement;
        stop.type = "button";
        stop.onclick = cancel;
        live.append(el("b", "k", agent), el("div", "md"), stop);
        list.append(live);
      }
      md(live).textContent += chunk;
    });
  }

  function setState(t: Turn): void {
    if (t.state !== state) since = Date.now();
    state = t.state;
    note = t.note ?? "";
    const [st, pill, wait, close] = [$("#st"), $("#pill"), $("#wait"), $("#close")];
    if (st) st.dataset.st = KEY[state] ?? "";
    main.dataset.state = state;
    if (pill) pill.hidden = state !== "working";
    if (wait) wait.hidden = state !== "blocked";
    if (state === "ended") {
      const send = $<HTMLButtonElement>("#composer button[type=submit]");
      box.disabled = true;
      if (send) send.disabled = true;
      if (close) close.hidden = true;
    }
    if (state !== "working" && state !== "blocked" && live && !md(live).textContent) {
      live.remove();
      live = null;
    }
    keep(turnEnd);
  }

  // Drops the rows more than (KEEP - 1) / 2 screens past the viewport, on the side away from the reader, so the
  // window is about KEEP screens around the anchor. Called inside keep(), with the scroll position current.
  function trim(fromTop: boolean): void {
    const far = ((KEEP - 1) / 2) * screen();
    const top = list.scrollTop;
    const bottom = top + list.clientHeight;
    const out = rows().filter((r) => (fromTop ? r.offsetTop + r.offsetHeight < top - far : r.offsetTop > bottom + far));
    for (const r of out) r.remove();
    if (fromTop) before += out.length;
    else after += out.length;
  }

  function render(items: Item[]): DocumentFragment {
    const frag = document.createDocumentFragment();
    for (const it of items) frag.append(fill(el("li"), it));
    return frag;
  }

  // Loads a slice: `q` is the query; `where` is "replace", "before" (older) or "after" (newer).
  const load = (q: string, where: "replace" | "before" | "after"): Promise<Slice> =>
    inOrder(async () => {
      // The newest slice is kept on the device, so the thread opens offline on its last copy.
      const kept = `cooee.slice.${id}`;
      const newest = where === "replace" && q === `before=${SLICE}`;
      const r = await fetch(`${api}/messages?${q}`).catch((x) => {
        if (newest && localStorage.getItem(kept)) return null;
        throw x;
      });
      if (r && !r.ok) throw await why(r);
      const s: Slice = r ? await r.json() : JSON.parse(localStorage.getItem(kept) ?? "null");
      if (newest && r)
        try {
          localStorage.setItem(kept, JSON.stringify({ ...s, at: Date.now() }));
        } catch {
          // Full: no copy.
        }
      if (where === "replace") {
        copyAt = r ? 0 : (s.at ?? 0);
        list.replaceChildren(render(s.items));
        live = null;
        before = s.skipped_before;
        after = s.skipped_after;
        cursor = s.cursor;
        if (s.turn.text && after === 0) text(s.turn.text);
        setState(s.turn);
        return s;
      }
      const have = new Set(ids());
      const fresh = s.items.filter((it) => !have.has(it.id));
      keep(() => {
        if (where === "before") {
          list.prepend(render(fresh));
          before = s.skipped_before;
        } else {
          list.insertBefore(render(fresh), live);
          after = s.skipped_after;
        }
      });
      keep(() => trim(where === "after"));
      if (where === "after" && after === 0 && s.turn.text && !live) text(s.turn.text);
      return s;
    });

  // The first slice is held up to ~300 ms and painted at once; a skeleton shows only after that.
  async function open(): Promise<Slice> {
    const skeleton = setTimeout(() => {
      list.classList.add("skeleton");
      const fig = el("li", "emp");
      const figure = el("span");
      const w = el("p");
      figure.dataset.s = "loading";
      w.dataset.w = "loading";
      w.dataset.plain = "";
      fig.append(figure, w);
      list.append(fig);
    }, 300);
    const unread = ids().length === 0 && read > 0 ? read + 1 : 0;
    const s = await load(unread ? `around=${unread}&before=${SLICE / 2}&after=${SLICE}` : `before=${SLICE}`, "replace");
    clearTimeout(skeleton);
    list.classList.remove("skeleton");
    const first = unread ? rowOf(unread) : null;
    if (first) {
      const bar = el("li", "unread", "unread");
      list.insertBefore(bar, first);
      list.scrollTop = bar.offsetTop;
    } else list.scrollTop = list.scrollHeight;
    cleared(id, read);
    markRead();
    listen();
    preload();
    return s;
  }

  // The sync dot: amber until the stream is open, green while it is, amber again when it drops.
  const sync = (on: boolean): void => {
    const dot = $(".dt");
    if (dot) dot.dataset.sync = on ? "on" : "off";
  };
  addEventListener("offline", () => sync(false));
  function listen(): void {
    es?.close();
    const stream = new EventSource(`${api}/events?since=${encodeURIComponent(cursor)}`);
    es = stream;
    sync(false);
    stream.onopen = () => sync(true);
    stream.addEventListener("item", (e) => put(JSON.parse(e.data)));
    stream.addEventListener("text", (e) => text(JSON.parse(e.data).text));
    stream.addEventListener("state", (e) => {
      const t: Turn = JSON.parse(e.data);
      moment(t.state);
      setState(t);
    });
    // Another run or a page too far behind: load the newest slice again.
    stream.addEventListener("reset", () => {
      stream.close();
      open();
    });
    stream.onerror = () => {
      sync(false);
      if (stream.readyState === EventSource.CLOSED) setTimeout(open, 2000);
    };
  }

  // A moment (the theme's, when it has them): a turn done hops the bar's sprite with the theme word as a tag, needs
  // you flashes the bar, a failed turn shakes it; 2.4 s, silent, still under reduced motion.
  function moment(next: string): void {
    const key = ({ done: "done", blocked: "needs", error: "fail" } as Record<string, string>)[next];
    const bar = $(".th");
    if (!key || !bar || next === state || document.documentElement.dataset.moments !== "on") return;
    bar.dataset.mo = key;
    $(".mot", bar)?.remove();
    const tag = key === "needs" ? null : el("i", "mot");
    if (tag) {
      tag.dataset.w = key === "done" ? "merged" : "fail";
      bar.append(tag);
    }
    setTimeout(() => {
      if (bar.dataset.mo !== key) return;
      delete bar.dataset.mo;
      tag?.remove();
    }, 2400);
  }

  // Within EDGE screens of an end of the window, the next slice loads, until there are EDGE screens to go.
  let loading = false;
  async function preload(): Promise<void> {
    if (loading) return;
    const top = before > 0 && list.scrollTop < EDGE * screen();
    const bottom = after > 0 && list.scrollHeight - list.scrollTop - list.clientHeight < EDGE * screen();
    if (!top && !bottom) return;
    loading = true;
    const edge = top ? `around=${ids()[0]}&before=${SLICE}&after=0` : `around=${ids().at(-1)}&before=0&after=${SLICE}`;
    await load(edge, top ? "before" : "after").catch(() => {});
    loading = false;
    preload();
  }
  list.addEventListener(
    "scroll",
    () => {
      preload();
      markRead();
    },
    { passive: true },
  );
  // The keyboard opening (or any resize) keeps the anchor: the first row in view stays put, and a reader at the
  // newest message stays there.
  let stuck = true;
  list.addEventListener(
    "scroll",
    () => {
      stuck = atBottom();
    },
    { passive: true },
  );
  new ResizeObserver(() => {
    if (stuck && after === 0) list.scrollTop = list.scrollHeight;
  }).observe(list);

  // Read marks: the newest row in view, sent at most every ~3 s.
  function markRead(): void {
    const bottom = list.scrollTop + list.clientHeight;
    const was = seen;
    for (const r of rows()) if (r.offsetTop < bottom) seen = Math.max(seen, Number(r.dataset.id));
    if (seen > was) cleared(id, seen);
    if (seen > read && !readTimer) {
      readTimer = setTimeout(() => {
        readTimer = 0;
        if (seen <= read) return;
        read = seen;
        post(`${api}/read`, { item: read }).catch(() => {});
      }, 3000);
    }
  }

  // The outbox: a row per send, `sending` until the server stored it, `failed` with tap-to-retry.
  function deliver(row: HTMLElement, text: string, random: string): void {
    row.className = "blk me sending";
    inOrder(() => post<Item>(`${api}/send`, { text, random_id: random })).then(
      (it) => {
        if (!row.dataset.rid) return;
        delete row.dataset.rid;
        fill(row, it);
      },
      () => {
        row.className = "blk me failed";
        const retry = el("button", "retry", "failed · tap to retry") as HTMLButtonElement;
        retry.type = "button";
        retry.onclick = () => {
          retry.remove();
          deliver(row, text, random);
        };
        row.append(retry);
      },
    );
  }

  async function send(e: Event): Promise<void> {
    e.preventDefault();
    const value = box.value.trim();
    if (!value || box.disabled) return;
    if (value === "/report") {
      box.value = "";
      return reportMode();
    }
    if (after > 0) await open();
    const random = rid();
    const row = el("li", "blk me");
    row.dataset.rid = random;
    row.append(el("div", "md", value));
    keep(() => list.insertBefore(row, live));
    list.scrollTop = list.scrollHeight;
    box.value = "";
    draft.save(true);
    deliver(row, value, random);
  }

  function cancel(): void {
    post(`${api}/cancel`, {}).then(
      () => toast("stopping…"),
      (x) => toast(message(x), true),
    );
  }

  // The draft: kept per thread in IndexedDB, written 1 s after typing stops and when the page hides; newest wins.
  type Draft = { text: string; at: number };
  const draft = (() => {
    const db = new Promise<IDBDatabase>((ok, no) => {
      const r = indexedDB.open("cooee", 1);
      r.onupgradeneeded = () => r.result.createObjectStore("drafts");
      r.onsuccess = () => ok(r.result);
      r.onerror = () => no(r.error);
    });
    const tx = async <T>(mode: IDBTransactionMode, f: (s: IDBObjectStore) => IDBRequest<T>): Promise<T | undefined> => {
      const store = (await db).transaction("drafts", mode).objectStore("drafts");
      return new Promise((ok) => {
        const r = f(store);
        r.onsuccess = () => ok(r.result);
        r.onerror = () => ok(undefined);
      });
    };
    const get = (): Promise<Draft | undefined> => tx<Draft>("readonly", (s) => s.get(id)).catch(() => undefined);
    let timer = 0;
    let at = 0;
    return {
      async load(): Promise<void> {
        const d = await get();
        if (!d || box.value) return;
        box.value = d.text;
        at = d.at;
      },
      changed(): void {
        at = Date.now();
        clearTimeout(timer);
        timer = setTimeout(() => draft.save(), 1000);
      },
      async save(now = false): Promise<void> {
        clearTimeout(timer);
        if (now) at = Date.now();
        const mine: Draft = { text: box.value, at };
        const old = await get();
        if (!old || old.at <= mine.at) await tx("readwrite", (s) => s.put(mine, id)).catch(() => {});
      },
    };
  })();

  box.addEventListener("input", () => draft.changed());
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "hidden") draft.save();
  });
  $("#composer")?.addEventListener("submit", send);
  box.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) send(e);
  });
  const stop = $("#pill .stop");
  if (stop) stop.onclick = cancel;
  const close = $("#close");
  if (close)
    close.onclick = () => {
      if (!confirm("Close this thread? Its agent stops.")) return;
      post(`${api}/close`, {}).then(
        () => toast("thread closed"),
        (x) => toast(message(x), true),
      );
    };
  // Live timers: the working pill and each running tool call.
  setInterval(() => {
    const elapsed = $("#el");
    if (elapsed) elapsed.textContent = clock(Date.now() - since);
    for (const t of list.querySelectorAll<HTMLElement>(".ts[data-since]"))
      t.textContent = `⋯ ${clock(Date.now() - Number(t.dataset.since))}`;
  }, 1000);
  draft.load();
  page.refresh = open;
  page.top = () => before === 0;
  open()
    .catch((x) => toast(message(x), true))
    .finally(() => strip(false));
}
