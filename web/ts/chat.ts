// cooee's pages, where a browser needs script (the rest is server HTML: the sheets, the composer and its chips'
// sheets, Settings › providers). The new thread (#48): a pick in a chip's sheet sets the chip, a folder moves the
// agent chip to that folder's last pick (scratch, no folder: the last pick overall), and send opens the chat at once. The chat: a window of about 9 screens of items around an anchor (slices by id as the reader nears an edge),
// scroll kept on a message rather than pixels, live events over SSE resumed from the last one seen, the outbox
// (sending → sent → failed, tap to retry), the per-thread draft in IndexedDB, approvals that lock once tapped, and
// debounced read marks (which also close the thread's notifications). Every page but pairing registers the service
// worker (sw.ts: push and the last copy of each page); the thread list's 🔔 turns alerts on. On every page (#21):
// short toasts, the loading strip (with a copy's age when it is an old one), refresh (↻, a pull down, coming back
// after 30 s or the network returning), Settings' alerts, devices and report mode (pairing is pair.ts). It talks to
// the server only through the JSON and SSE API in docs/api.md.
// States are theme keys (data-st) that look.ts draws: the thread bar, the working pill (its sprite following the
// running tool), the live line and a turn's end.
// A plugin tab hosts the plugin's sandboxed frame (frame.ts is the frame's side).
import { $, make, show } from "./look.js";
import { render as markdown, settled } from "./md.js";

type Option = { id: string; name: string; kind: string };
type Item = { id: number; at: number; seq?: number } & (
  | { kind: "you"; text: string; random_id: string; files?: string[] }
  | { kind: "agent"; text: string; stop_reason: string }
  | { kind: "tool"; call: string; title: string; status: string; ended?: number }
  | ({ kind: "approval"; tool: string; options: Option[]; tapped?: string; chosen?: string; plugin?: string } & Form)
  | { kind: "plan"; entries: { content: string; status: string }[] }
  | { kind: "link"; text: string; thread: string; title: string; out?: boolean }
  | { kind: "note"; text: string }
  | { kind: "ended"; text: string }
);
// Who answered and when, and a plugin card's form (#553 rows 30-33).
type Form = { by?: string; answered?: number; form?: string; detail?: string; line?: string; until?: number };
type Choice = Extract<Item, { kind: "approval" }>;
type Turn = { text: string; state: string; note?: string };
type Queued = { text: string; random_id: string; files?: string[] };
type Slice = {
  items: Item[];
  skipped_before: number;
  skipped_after: number;
  turn: Turn;
  cursor: string;
  queue?: Queued[];
  at?: number;
  waiting?: Item | null;
  pending?: { text: string; random_id: string };
};

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
// It is a manual popover, so it shows over an open sheet too. `act` adds a button (undo) and keeps it 5 s.
let toastTimer = 0;
function toast(text: string, bad = false, act?: [string, () => unknown]): void {
  const t = $("#toast");
  if (!t) return;
  t.textContent = text;
  t.className = bad ? "r" : "g";
  if (act) {
    const b = make("button", "", act[0]) as HTMLButtonElement;
    b.type = "button";
    b.onclick = (e) => {
      e.stopPropagation();
      hide(t);
      act[1]();
    };
    t.append(" ", b);
  }
  if (!t.matches(":popover-open")) t.showPopover();
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => hide(t), act ? 5000 : bad ? 6000 : 2500);
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
    if (!$("main.list")) return location.reload();
    return cardsNow();
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

// The thread list's cards and archived threads from `GET /threads`, drawn as the server draws them (lib.rs
// `threads`); the list follows them every 5 s while it shows, and the app's badge counts the unread threads.
type Row = {
  id: string;
  title: string;
  agent: string;
  state: string;
  last: string;
  at: number;
  unread: boolean;
  unseen: number;
  workflow?: { role: string; round: string };
};
type Card = { folder: string; rows: Row[]; done?: Row[] };
type Listed = { cards: Card[]; archived: Row[] };
async function cardsNow(): Promise<void> {
  const list = $("main.list");
  const r = await fetch("/threads");
  if (!r.ok) throw await why(r);
  copyAt = Number(r.headers.get("cooee-at")) || 0;
  if (list) drawCards(list, await r.json());
}
const badge = (): void => {
  const n = document.querySelectorAll("main.list .fc a.tr:not(.rd)").length;
  if ("setAppBadge" in navigator) (n ? navigator.setAppBadge(n) : navigator.clearAppBadge()).catch(() => {});
};
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
const meta = (r: Row): string =>
  r.workflow ? `${r.workflow.role} · round ${r.workflow.round} · ${age(r.at)}` : `${r.agent} · ${age(r.at)}`;
function rowOf(r: Row): HTMLElement {
  const a = make("a", r.unread ? "tr" : "tr rd") as HTMLAnchorElement;
  const st = make("b");
  const tp = make("span", "tp");
  const ta = make("span", "ta", meta(r));
  a.href = `/t/${r.id}`;
  a.dataset.id = r.id;
  st.dataset.st = STATE[r.state] ?? "";
  tp.append(st, r.last);
  if (r.unseen > 0) {
    const bd = make("b", "bd", String(r.unseen));
    bd.setAttribute("aria-label", `${r.unseen} unread`);
    ta.append(bd);
  }
  a.append(make("i", "ud", r.unread ? "●" : ""), make("span", "tt", r.title), ta, tp);
  return a;
}
function drawCards(list: HTMLElement, { cards, archived }: Listed): void {
  // A folded list that was open stays open.
  const key = (d: Element): string =>
    d.classList.contains("arc") ? "arc" : ($(".fh b", d.closest(".fc") ?? d)?.textContent ?? "");
  const open = new Set([...list.querySelectorAll("details[open]")].map(key));
  for (const e of list.querySelectorAll(":scope > .fc, :scope > .emp")) e.remove();
  if (!cards.length) {
    const emp = make("div", "emp");
    const fig = make("span");
    const w = make("p");
    fig.dataset.s = "empty";
    w.dataset.w = "empty-threads";
    w.dataset.plain = "";
    emp.append(fig, w);
    list.prepend(emp);
  }
  const scn = $("#scene");
  if (scn) {
    drawScene(scn, cards);
    return;
  }
  for (const c of cards) {
    const card = make("section", "fc");
    const fh = make("div", "fh");
    fh.append(make("b", "", c.folder), make("span", "ds", `${c.rows.length} threads`));
    card.append(fh, ...c.rows.slice(0, c.done ? undefined : 3).map(rowOf));
    if (c.done?.length) {
      const done = make("details") as HTMLDetailsElement;
      done.append(make("summary", "", `${c.done.length} done today`), ...c.done.map(rowOf));
      done.open = open.has(c.folder);
      card.append(done);
    } else if (!c.done && c.rows.length > 3) {
      const more = make("details") as HTMLDetailsElement;
      more.append(make("summary", "", `${c.rows.length - 3} older threads`), ...c.rows.slice(3).map(rowOf));
      more.open = open.has(c.folder);
      card.append(more);
    }
    list.append(card);
  }
  if (archived.length) {
    const arc = make("details", "fc arc") as HTMLDetailsElement;
    const sum = make("summary", "", `${archived.length} archived`);
    arc.append(sum);
    arc.open = open.has("arc");
    for (const r of archived) {
      const row = make("div", "arw");
      const a = make("a", "", r.title) as HTMLAnchorElement;
      const b = make("button", "cn", "restore") as HTMLButtonElement;
      a.href = `/t/${r.id}`;
      b.type = "button";
      b.dataset.restore = r.id;
      b.setAttribute("aria-label", `restore ${r.title}`);
      row.append(a, b);
      arc.append(row);
    }
    list.append(arc);
  }
  badge();
}

// A row swiped left is archived, with 5 s to undo; an archived one's [restore] brings it back.
let swipe: { row: HTMLElement; x: number; y: number; dx: number } | null = null;
const shelf = (id: string, on: boolean): Promise<void> => post(`/threads/${id}/archive`, { on }).then(cardsNow);
const threadList = $("main.list");
if (threadList) {
  threadList.addEventListener(
    "touchstart",
    (e) => {
      const row = (e.target as Element).closest?.<HTMLElement>(".fc a.tr");
      const t = e.touches[0];
      swipe = row && t ? { row, x: t.clientX, y: t.clientY, dx: 0 } : null;
    },
    { passive: true },
  );
  threadList.addEventListener(
    "touchmove",
    (e) => {
      const t = e.touches[0];
      if (!swipe || !t) return;
      swipe.dx = Math.min(0, t.clientX - swipe.x);
      // Up or down is a scroll.
      if (Math.abs(t.clientY - swipe.y) > Math.abs(swipe.dx)) swipe.dx = 0;
      swipe.row.style.transform = swipe.dx ? `translateX(${swipe.dx}px)` : "";
    },
    { passive: true },
  );
  threadList.addEventListener("touchend", () => {
    const s = swipe;
    swipe = null;
    const id = s?.row.dataset.id;
    if (!s || !id) return;
    s.row.style.transform = "";
    if (s.dx > -80) return;
    const title = $(".tt", s.row)?.textContent ?? "the thread";
    shelf(id, true).then(
      () => toast(`archived ${title}`, false, ["undo", () => shelf(id, false).catch((x) => toast(message(x), true))]),
      (x) => toast(message(x), true),
    );
  });
  threadList.addEventListener("click", (e) => {
    const id = (e.target as Element).closest?.<HTMLElement>("[data-restore]")?.dataset.restore;
    if (id)
      shelf(id, false).then(
        () => toast("restored"),
        (x) => toast(message(x), true),
      );
  });
  setInterval(() => {
    if (!document.hidden && !swipe && !refreshing) cardsNow().catch(() => {});
  }, 5000);
  badge();
}

// Scene (#34): the same cards as pages of sprites (lib.rs `scene`), redrawn whole so a refresh never leaves a copy;
// the dot of the page in view; night on the strip (the scene's `#night`) from 19:00 to 07:00 on this device's clock.
const POSE: Record<string, string> = { working: "edit", needs: "needs", done: "done", fail: "fail", ended: "merged" };
const night = (): string => {
  const h = new Date().getHours();
  return h < 7 || h >= 19 ? "#night" : "";
};
function spriteOf(r: Row): HTMLElement {
  const st = STATE[r.state] ?? "";
  const a = make("a", r.unread ? "sb" : "sb rd") as HTMLAnchorElement;
  const fig = make("span", "sf");
  const card = make("span", "sn");
  const w = make("span", `sw ${st}`);
  a.href = `/t/${r.id}`;
  a.dataset.id = r.id;
  fig.dataset.s = POSE[st] ?? "idle";
  w.dataset.w = st || "idle";
  card.append(make("b", "", r.title), make("span", "", meta(r)), w);
  a.append(fig, card);
  return a;
}
function dot(): void {
  const pages = $("#scene .spg");
  if (!pages) return;
  const at = Math.round(pages.scrollLeft / (pages.clientWidth || 1));
  for (const [i, d] of document.querySelectorAll("#scene .sdt a").entries())
    if (i === at) d.setAttribute("aria-current", "page");
    else d.removeAttribute("aria-current");
}
function light(): void {
  const scn = $("#scene");
  scn?.classList.toggle("night", !!night());
  for (const img of document.querySelectorAll<HTMLImageElement>("#scene .sky")) {
    const src = `${scn?.dataset.art}${night()}`;
    if (img.getAttribute("src") !== src) img.src = src;
  }
}
function drawScene(scn: HTMLElement, cards: Card[]): void {
  const pages = $(".spg", scn);
  const dots = $(".sdt", scn);
  if (!pages || !dots) return;
  const at = pages.scrollLeft;
  pages.replaceChildren(
    ...cards.map((c, i) => {
      const p = make("section", "sp");
      const sky = make("img", "sky") as HTMLImageElement;
      const h = make("h2", "fh");
      const field = make("div", "fld");
      p.id = `sp-${i}`;
      p.setAttribute("aria-label", c.folder);
      sky.alt = "";
      h.append(make("b", "", c.folder), make("span", "ds", `${c.rows.length} threads`));
      field.append(...c.rows.concat(c.done ?? []).map(spriteOf));
      p.append(sky, h, field);
      return p;
    }),
  );
  dots.replaceChildren(
    ...cards.map((c, i) => {
      const d = make("a") as HTMLAnchorElement;
      d.href = `#sp-${i}`;
      d.setAttribute("aria-label", c.folder);
      return d;
    }),
  );
  dots.hidden = cards.length < 2;
  pages.scrollLeft = at;
  light();
  dot();
}
$("#scene .spg")?.addEventListener("scroll", dot, { passive: true });
light();
setInterval(() => document.hidden || light(), 60000);

let pullY: number | null = null;
let hiddenAt = 0;
addEventListener(
  "touchstart",
  (e) => {
    const scroller = $("#msgs") ?? $("main.list") ?? document.scrollingElement;
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

const say = (e: HTMLElement | null, text: string): void => {
  if (!e) return;
  e.textContent = text;
  e.hidden = !text;
};

// The composer: each chip's sheet is a popover of radios in the form. A tap closes the sheet, a pick sets its chip; a
// folder moves the agent chip to the pick last used there (else Plain's newest overall); send starts the thread in the
// background and opens its chat at once.
const nc = $<HTMLFormElement>("#nc");
if (nc) {
  const send = $<HTMLButtonElement>("button[type=submit]", nc);
  const picked = (name: string): HTMLInputElement | null => $<HTMLInputElement>(`input[name=${name}]:checked`, nc);
  const chip = (id: string, label: string, title?: string): void => {
    const c = $(`#${id}`);
    const s = c && $("span", c);
    if (!c || !s) return;
    s.textContent = label;
    c.setAttribute("aria-label", `${id === "chip-agent" ? "agent" : "folder"}: ${title ?? label}`);
    if (title !== undefined) c.title = title;
  };
  const agentChip = (): void => {
    const p = picked("pick");
    if (!p) return;
    chip("chip-agent", p.dataset.label ?? "");
    const box = $<HTMLTextAreaElement>("textarea", nc);
    if (box) box.placeholder = `message ${p.dataset.name ?? ""}`;
  };
  const folderChip = (): void => {
    const f = picked("folder");
    const label = f?.parentElement && $(".sk", f.parentElement)?.textContent;
    if (label) chip("chip-folder", label.split("/").pop() || label, label);
  };
  const choose = (key: string | undefined): void => {
    const r = key ? $<HTMLInputElement>(`input[name=pick][value="${CSS.escape(key)}"]`, nc) : null;
    if (r && !r.disabled) r.checked = true;
    agentChip();
  };
  nc.addEventListener("change", (e) => {
    const t = e.target as HTMLInputElement;
    if (t.name === "folder") {
      choose(t.dataset.last || nc.dataset.def);
      folderChip();
    } else if (t.name === "pick") agentChip();
  });
  // A tap on a sheet's row closes it, the row already picked too.
  nc.addEventListener("click", (e) => {
    const t = e.target as HTMLInputElement;
    if (t.type === "radio") t.closest<HTMLElement>("[popover]")?.hidePopover();
  });
  for (const p of nc.querySelectorAll<HTMLElement>("[popover]"))
    p.addEventListener("toggle", (e) => {
      const c = nc.querySelector(`[popovertarget="${p.id}"]`);
      c?.setAttribute("aria-expanded", String((e as ToggleEvent).newState === "open"));
    });
  // An account just signed in from the agent sheet is picked.
  const after = sessionStorage.getItem("cooee.pick");
  if (after) {
    sessionStorage.removeItem("cooee.pick");
    choose(after);
  }
  nc.addEventListener("submit", async (e) => {
    e.preventDefault();
    const f = new FormData(nc);
    const p = picked("pick");
    // One id per start until it succeeds, so a retry after a timeout adopts the thread it started.
    nc.dataset.rid ||= rid();
    const body = {
      agent: p?.dataset.agent,
      account: p?.dataset.account,
      profile: p?.dataset.profile ?? "",
      model: f.get(`model:${p?.dataset.agent}`) ?? "",
      effort: f.get(`effort:${p?.dataset.agent}`) ?? "",
      folder: f.get("folder"),
      text: f.get("text"),
      random_id: nc.dataset.rid,
      background: true,
    };
    if (send) send.disabled = true;
    try {
      location.href = `/t/${(await post<{ id: string }>("/threads", body)).id}`;
    } catch (x) {
      say($("#new-err"), message(x));
      if (send) send.disabled = false;
    }
  });
}

// The folder sheet: the search shows the folders whose name has it (up to 2 levels in), else the first level;
// [+ new folder] makes one inside the folder picked (scratch picked: inside the first root), and picks it.
const fq = $<HTMLInputElement>("#fq");
const fl = $(".fl");
if (fq && fl && nc) {
  fq.oninput = () => {
    const q = fq.value.trim().toLowerCase();
    for (const l of fl.querySelectorAll<HTMLElement>("label")) {
      const base = $(".sk", l)?.textContent?.trim().split("/").pop()?.toLowerCase() ?? "";
      l.hidden = q ? !base.includes(q) : Number(l.dataset.d) > 1;
    }
  };
  const mkdir = $<HTMLButtonElement>("#mkdir");
  if (mkdir)
    mkdir.onclick = async () => {
      const parent =
        $<HTMLInputElement>("input[name=folder]:checked:not([value=''])", fl) ??
        $<HTMLInputElement>("input[name=folder]:not([value=''])", fl);
      const at = parent?.parentElement;
      if (!at || !parent) return;
      const label = $(".sk", at)?.textContent?.trim() ?? "";
      const name = prompt(`New folder in ${label}:`);
      if (!name) return;
      try {
        const path = await post<string>("/folders/new", { parent: parent.value, name });
        const row = make("label", "sr pk");
        const input = make("input") as HTMLInputElement;
        Object.assign(input, { type: "radio", name: "folder", value: path });
        row.dataset.d = String(Math.min(2, Number(at.dataset.d) + 1));
        row.append(input, make("span", "sk", `${label}/${name}`), make("i", "tk", "✓"));
        say($("#new-err"), "");
        at.after(row);
        input.checked = true;
        input.dispatchEvent(new Event("change", { bubbles: true }));
      } catch (x) {
        say($("#new-err"), message(x));
      }
    };
}

// A hint's copy button puts its line on the clipboard (Settings › providers' [copy install] too).
document.addEventListener("click", (e) => {
  const b = (e.target as Element).closest?.<HTMLElement>("[data-copy]");
  if (!b?.dataset.copy) return;
  navigator.clipboard?.writeText(b.dataset.copy).then(
    () => {
      const was = b.textContent;
      b.textContent = "copied";
      setTimeout(() => (b.textContent = was), 1500);
    },
    () => {},
  );
});

// Sign-ins, in Settings › providers and the agent sheet: [sign in] starts the provider's own login for that account
// in a shed terminal ([+ account]: in a new account's folder); its terminal opens under the row, its screen
// (redacted by the server) drawn every second with its links tappable, and a code typed there goes to it. Once it
// ends the page reloads with the accounts' state (Settings › providers open again; from the agent sheet, the account
// picked). [sign out] (after a confirm) reloads it too.
type LoginView = { state: string; screen: string; links: string[] };
let loginTimer = 0;
const tell = (row: HTMLElement, m: string): void => {
  let w = row.nextElementSibling?.classList.contains("wy") ? (row.nextElementSibling as HTMLElement) : null;
  if (!w) {
    w = make("p", "err wy");
    row.after(w);
  }
  w.textContent = m;
};
const again = (pick?: string): void => {
  if (pick) sessionStorage.setItem("cooee.pick", pick);
  if (!$("#nc") || !pick) location.hash = "#settings/providers";
  location.reload();
};
function signIn(row: HTMLElement, provider: string, body: object, pick?: string): void {
  const api = `/logins/${provider}`;
  const lg = row.closest(".top")?.querySelector<HTMLElement>(`.lg[data-provider="${CSS.escape(provider)}"]`);
  const out = lg && $(".out", lg);
  if (!lg || !out) return;
  const draw = (v: LoginView): void => {
    if (v.state === "ended") {
      clearInterval(loginTimer);
      again(pick);
    }
    if (v.state !== "running") return;
    lg.hidden = false;
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
  const code = $<HTMLInputElement>("input", lg);
  const go = $<HTMLButtonElement>(".lgs", lg);
  const cn = $<HTMLButtonElement>(".cn", lg);
  if (go)
    go.onclick = async () => {
      try {
        await post(`${api}/send`, { text: code?.value ?? "" });
        if (code) code.value = "";
        poll();
      } catch (x) {
        tell(lg, message(x));
      }
    };
  if (cn) cn.onclick = () => post(`${api}/cancel`, {}).then(poll, poll);
  row.after(lg);
  post(api, body).then(
    () => {
      const st = $(".st, .sv", row);
      if (st) {
        st.className = st.classList.contains("st") ? "st a" : "sv a";
        st.textContent = "signing in";
      }
      clearInterval(loginTimer);
      loginTimer = setInterval(poll, 1000);
      poll();
    },
    (x) => tell(row, message(x)),
  );
}
document.addEventListener("click", (e) => {
  const b = (e.target as Element).closest?.<HTMLElement>(".si, .sgo, .ad");
  const row = b?.closest<HTMLElement>(".acr, .pk");
  if (!b || !row) return;
  e.preventDefault();
  const provider = b.dataset.provider ?? row.dataset.provider ?? "";
  const account = b.dataset.account ?? row.dataset.account ?? "";
  if (b.classList.contains("ad")) signIn(row, provider, { new: true });
  else if (b.classList.contains("si")) signIn(row, provider, { account }, b.dataset.pick);
  else if (confirm(`Sign ${$(".sk", row)?.textContent ?? account} out?${account ? " Its folder is removed." : ""}`))
    post(`/logins/${provider}/signout`, { account }).then(
      () => again(),
      (x) => tell(row, message(x)),
    );
});

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
      // A stream refused for good (a locked tab's 403) says so: `closed`.
      es.onerror = () => {
        if (es.readyState === EventSource.CLOSED) to({ op: "event", id: m.id, event: "closed", data: "" });
      };
      streams.set(m.id, es);
    } else if (m.op === "close") {
      streams.get(m.id)?.close();
      streams.delete(m.id);
    } else if (m.op === "unlock") lock.hidden = false;
  });
  // The passkey gate (#28): the ceremony runs here, never in the sandboxed frame. The first time, this device
  // registers its passkey; then each unlock signs a fresh challenge. The tap is on this page's own button.
  const lock = document.createElement("button");
  lock.className = "pkl";
  lock.hidden = true;
  lock.textContent = "Unlock with Face ID";
  frame.after(lock);
  const b64 = (b: ArrayBuffer): string =>
    btoa(String.fromCharCode(...new Uint8Array(b)))
      .replace(/\+/g, "-")
      .replace(/\//g, "_")
      .replace(/=+$/, "");
  const bytes = (s: string): Uint8Array<ArrayBuffer> =>
    Uint8Array.from(atob(s.replace(/-/g, "+").replace(/_/g, "/")), (c) => c.charCodeAt(0));
  lock.onclick = async () => {
    try {
      const o = await post<{ challenge: string; id: string | null }>("/passkey/options", {});
      const challenge = bytes(o.challenge);
      const rpId = location.hostname;
      if (o.id) {
        const c = (await navigator.credentials.get({
          publicKey: {
            challenge,
            rpId,
            allowCredentials: [{ type: "public-key", id: bytes(o.id) }],
            userVerification: "required",
          },
        })) as PublicKeyCredential;
        const r = c.response as AuthenticatorAssertionResponse;
        await post("/passkey/unlock", {
          id: c.id,
          client: b64(r.clientDataJSON),
          auth: b64(r.authenticatorData),
          sig: b64(r.signature),
        });
      } else {
        const c = (await navigator.credentials.create({
          publicKey: {
            challenge,
            rp: { id: rpId, name: "cooee" },
            user: { id: crypto.getRandomValues(new Uint8Array(16)), name: "owner", displayName: "cooee owner" },
            pubKeyCredParams: [{ type: "public-key", alg: -7 }],
            authenticatorSelection: { userVerification: "required", residentKey: "preferred" },
          },
        })) as PublicKeyCredential;
        const r = c.response as AuthenticatorAttestationResponse;
        const key = r.getPublicKey();
        if (!key) throw new Error("This passkey has no P-256 key.");
        await post("/passkey/register", {
          id: c.id,
          client: b64(r.clientDataJSON),
          auth: b64(r.getAuthenticatorData()),
          key: b64(key),
        });
      }
      lock.hidden = true;
      to({ op: "unlocked" });
    } catch (x) {
      toast(message(x), true);
    }
  };
  // The tab fills what the keyboard leaves (#443): a bar at the frame's foot sits on the keyboard.
  const vv = visualViewport;
  const tab = frame.parentElement;
  const fit = (): void => {
    if (!vv || !tab) return;
    tab.style.height = `${vv.height}px`;
    tab.style.transform = `translateY(${vv.offsetTop}px)`;
  };
  vv?.addEventListener("resize", fit);
  vv?.addEventListener("scroll", fit);
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
  // A row's text as it came (the long-press menu's copy as markdown and quote).
  const raw = new WeakMap<Element, string>();
  const fileUrl = (name: string): string => `${api}/files/${encodeURIComponent(name)}`;
  const IMAGE = /\.(png|jpe?g|gif|webp|heic|avif)$/i;
  // A message's files: an image as a thumbnail that opens the viewer, anything else as its name to download.
  const thumbs = (files: [string, string][]): HTMLElement => {
    const box = el("div", "fis");
    for (const [name, url] of files) {
      const a = el("a", IMAGE.test(name) ? "fi im" : "fi", IMAGE.test(name) ? undefined : name) as HTMLAnchorElement;
      a.href = url;
      if (IMAGE.test(name)) {
        const img = el("img") as HTMLImageElement;
        img.src = url;
        img.alt = name;
        img.loading = "lazy";
        a.append(img);
      } else a.download = name;
      box.append(a);
    }
    return box;
  };
  // The stored name less its stamp: `1730000000000-0-photo.jpg` is photo.jpg.
  const shown = (name: string): string => name.replace(/^\d+-\d+-/, "");

  // Runs of 2 or more tool calls fold under one row, `▸ 3 commands`; one opened stays open as the run grows.
  const opened = new Set<string>();
  function fold(): void {
    // Most changes (a chunk of text, a slice of replies) have nothing to fold: no walk.
    if (!list.querySelector(":scope > .run, :scope > .fold")) return;
    const used = new Set<Element>();
    let run: HTMLElement[] = [];
    const close = (): void => {
      const first = run[0];
      const prev = first?.previousElementSibling as HTMLElement | null | undefined;
      const head = prev?.classList.contains("fold") ? prev : null;
      if (first && run.length > 1) {
        const key = first.dataset.id ?? "";
        const open = opened.has(key);
        const h = head ?? el("li", "fold");
        if (!head) {
          const b = el("button") as HTMLButtonElement;
          b.type = "button";
          h.append(b);
          list.insertBefore(h, first);
        }
        h.dataset.key = key;
        const b = $("button", h);
        if (b) {
          b.textContent = `${open ? "▾" : "▸"} ${run.length} commands`;
          b.setAttribute("aria-expanded", String(open));
        }
        used.add(h);
        for (const r of run) r.classList.toggle("fd", !open);
      } else for (const r of run) r.classList.remove("fd");
      run = [];
    };
    for (const c of [...list.children] as HTMLElement[]) {
      if (c.classList.contains("fold")) continue;
      if (c.classList.contains("run")) run.push(c);
      else close();
    }
    close();
    for (const h of list.querySelectorAll(":scope > .fold")) if (!used.has(h)) h.remove();
  }
  list.addEventListener("click", (e) => {
    const key = (e.target as Element).closest?.<HTMLElement>(".fold")?.dataset.key;
    if (key === undefined) return;
    if (!opened.delete(key)) opened.add(key);
    keep(() => {});
  });
  // Thread refs (#84): a whole-word id of a thread cooee knows opens it; code spans and unknown ids stay plain.
  const known = new Set((main.dataset.ids ?? "").split(" "));
  const refs = (text: string): (string | Node)[] =>
    text.split(/(```[\s\S]*?```|`[^`\n]*`)/).flatMap((part, i) =>
      i % 2
        ? [part]
        : part.split(/(?<![\w-])(t[0-9a-z]+)(?![\w-])/).map((w, j) => {
            if (j % 2 === 0 || !known.has(w)) return w;
            const a = el("a", "ref", w) as HTMLAnchorElement;
            a.href = `/t/${w}`;
            return a;
          }),
    );
  // The same in rendered markdown `src`: its text outside code and links, walked only when it names a known id.
  const linkRefs = (root: DocumentFragment, src: string): DocumentFragment => {
    if (!src.match(/(?<![\w-])t[0-9a-z]+(?![\w-])/g)?.some((w) => known.has(w))) return root;
    const w = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    const texts: Text[] = [];
    while (w.nextNode()) if (!w.currentNode.parentElement?.closest("code, a")) texts.push(w.currentNode as Text);
    for (const x of texts) {
      const parts = refs(x.data);
      if (parts.some((n) => typeof n !== "string")) x.replaceWith(...parts);
    }
    return root;
  };
  const withRefs = (tag: string, cls: string, s: string): HTMLElement => {
    const e = el(tag, cls);
    e.append(...refs(s));
    return e;
  };
  // Back goes to the thread a ref or chip came from, else to the list.
  const from = document.referrer ? new URL(document.referrer) : null;
  const bk = $<HTMLAnchorElement>(".th .bk");
  if (bk && from?.origin === location.origin && from.pathname.startsWith("/t/") && from.pathname !== location.pathname)
    bk.href = from.pathname;
  // A config card that waits (#553 row 32): the bar under the header names it, wherever it is; a tap brings it in.
  const cw = $("#cw") as HTMLButtonElement;
  function waiting(it?: Item | null): void {
    if (it?.kind === "approval" && it.form === "config" && !it.chosen && !it.line) {
      cw.textContent = `${it.tool} waiting`;
      cw.dataset.id = String(it.id);
      cw.hidden = false;
    } else if (!it || cw.dataset.id === String(it.id)) cw.hidden = true;
  }
  cw.onclick = async () => {
    const at = cw.dataset.id ?? "";
    if (!rowOf(at)) await load(`around=${at}&before=${SLICE / 2}&after=${SLICE}`, "replace");
    rowOf(at)?.scrollIntoView({ block: "center" });
  };

  // Scroll is kept on a message: the first row in view and its offset are restored after any change of layout.
  function keep(change: () => void): void {
    const stick = atBottom();
    const anchor = rows().find((r) => r.offsetTop + r.offsetHeight > list.scrollTop);
    const offset = anchor ? anchor.offsetTop - list.scrollTop : 0;
    change();
    fold();
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
    connecting: "loading",
    unconnected: "fail",
  };
  const who = main.dataset.who ?? agent;

  // While the agent connects (#48): the connecting line where the reply will come, in the theme's loading word,
  // until the agent's first output; a failed connect puts its card there instead, with why and [retry].
  let greeting = state === "connecting";
  function connecting(): void {
    $(".cnx, .cfl", list)?.remove();
    if (state === "connecting") greeting = true;
    else if (state !== "working") greeting = false;
    const pill = $("#pill");
    if (pill) pill.hidden = state !== "working" || greeting;
    if (after > 0) return;
    if (greeting) {
      const line = el("li", "cnx");
      line.dataset.st = "loading";
      line.dataset.who = who;
      list.append(line);
    } else if (state === "unconnected") {
      const card = el("li", "cfl");
      const box = el("div");
      const head = el("p", "cnx");
      const mark = el("span");
      const retry = el("button", "", "retry") as HTMLButtonElement;
      box.setAttribute("role", "alert");
      mark.dataset.st = "fail";
      head.append(mark, el("span", "", ` · couldn't connect ${who}`));
      retry.type = "button";
      retry.onclick = () => {
        retry.disabled = true;
        post(`${api}/retry`, {}).catch((x) => {
          retry.disabled = false;
          toast(message(x), true);
        });
      };
      box.append(head, el("p", "ds", note), retry);
      card.append(box);
      list.append(card);
    }
  }
  // The agent answers: its first output replaces the connecting line.
  const answered = (): void => {
    if (!greeting) return;
    greeting = false;
    connecting();
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
      raw.set(row, it.text);
      row.append(withRefs("div", "md pl", it.text));
      if (it.files?.length) row.append(thumbs(it.files.map((n) => [shown(n), fileUrl(n)])));
      row.append(el("span", "ct", `${time(it.at)} ✓`));
    } else if (it.kind === "agent") {
      row.className = "blk ai";
      raw.set(row, it.text);
      const box = el("div", "md");
      box.append(linkRefs(markdown(it.text), it.text));
      row.append(el("b", "k", agent), box);
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
      row.className = `blk ap ${it.form ?? ""}`;
      row.append(el("b", "k", it.plugin ?? agent), withRefs("div", "md pl", it.tool));
      if (it.form === "secret") row.append(el("p", "m", `secret ${it.detail}`));
      if (it.form === "config" && it.detail) {
        const pre = el("pre", "diff");
        for (const l of it.detail.split("\n"))
          pre.append(el("span", l.startsWith("+") ? "g" : l.startsWith("-") ? "r" : "", `${l}\n`));
        row.append(pre);
      }
      if (it.chosen) {
        // The ✓ line: what was picked (or the plugin's own line), by whom and when; a conductor's answer keeps
        // [change] for its 30 minutes.
        const name = it.options.find((o) => o.id === it.chosen)?.name ?? it.chosen;
        const said =
          it.line ??
          (it.form === "secret"
            ? `secret ${it.detail} ${it.chosen === "save" ? "set ✓" : "dismissed"}`
            : it.form === "config"
              ? `${it.tool} ${it.chosen === "apply" ? "applied" : "discarded"}`
              : `✓ ${name}`);
        const ans = withRefs(
          "p",
          "ans",
          [said, it.by, it.answered ? time(it.answered) : ""].filter(Boolean).join(" · "),
        );
        row.append(ans);
        if (it.until && Date.now() < it.until) {
          const change = el("button", "chg", "change") as HTMLButtonElement;
          change.type = "button";
          change.onclick = () => change.replaceWith(choices(row, it));
          ans.after(change);
        }
      } else if (it.line) row.append(el("p", "ans st", it.line));
      else row.append(choices(row, it));
    } else if (it.kind === "link") {
      // A handoff or a line from another thread: its chip opens that thread (#553 row 35).
      row.className = "qs ln";
      const chip = el("a", "chip", `${it.out ? "continued in" : "from"} ${it.title}`) as HTMLAnchorElement;
      chip.href = `/t/${encodeURIComponent(it.thread)}`;
      if (it.text) row.append(...refs(it.text), " ");
      row.append(chip);
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

  // An open card's answers: a button per choice (the recommended one first), and on a plugin's card an "other"
  // box; a secret card is a masked field with [save] and [dismiss]. Locked once tapped.
  function choices(row: HTMLElement, it: Choice): HTMLElement {
    const chc = el("div", "chc");
    const button = (text: string, then: () => void, cls = ""): HTMLButtonElement => {
      const b = el("button", cls, text) as HTMLButtonElement;
      b.type = "button";
      b.onclick = then;
      return b;
    };
    const field = (label: string, type: string, then: (v: string) => void): HTMLFormElement => {
      const f = el("form", "oth") as HTMLFormElement;
      const v = el("input") as HTMLInputElement;
      Object.assign(v, { type, autocomplete: "off", required: true, placeholder: label });
      v.setAttribute("aria-label", label);
      f.onsubmit = (e) => {
        e.preventDefault();
        then(v.value);
        v.value = "";
      };
      f.append(v);
      return f;
    };
    if (it.form === "secret") {
      const f = field(`value of ${it.detail}`, "password", (v) => approve(row, it.id, "save", v));
      const save = button("save", () => f.requestSubmit());
      f.append(
        save,
        button("dismiss", () => approve(row, it.id, "dismiss")),
      );
      chc.append(f);
    } else {
      for (const o of it.options)
        chc.append(button(o.name, () => approve(row, it.id, o.id), o.kind === "recommended" ? "rec" : ""));
      // A ref in an option's label opens its thread from a link after the buttons (a link can't sit inside one).
      chc.append(...it.options.flatMap((o) => refs(o.name).filter((n) => typeof n !== "string")));
      if (it.plugin && !it.form) {
        const f = field("other", "text", (v) => approve(row, it.id, v));
        f.append(button("send", () => f.requestSubmit()));
        chc.append(f);
      }
    }
    if (it.tapped) {
      chc.classList.add("busy");
      for (const b of chc.querySelectorAll<HTMLButtonElement | HTMLInputElement>("button, input")) b.disabled = true;
    }
    return chc;
  }

  // A tap locks the row's answers (spinner) until the server answers; the server sends it to shed once.
  async function approve(row: HTMLElement, item: number, option: string, value?: string): Promise<void> {
    const chc = $(".chc", row);
    if (!chc) return;
    const lock = (on: boolean): void => {
      chc.classList.toggle("busy", on);
      for (const b of chc.querySelectorAll<HTMLButtonElement | HTMLInputElement>("button, input")) b.disabled = on;
    };
    lock(true);
    // The answer comes back over the event stream (tapped, then chosen), never older than what is shown.
    try {
      await post(`${api}/approve`, { item, option, value });
    } catch (x) {
      toast(message(x), true);
      lock(false);
    }
  }

  // An item, new or changed: replaced in place, appended at the newest end, or counted beyond the window.
  function put(it: Item): void {
    if (it.kind === "approval" && it.form === "config") waiting(it);
    const row = rowOf(it.id);
    if (row) {
      keep(() => fill(row, it));
      return;
    }
    if (after > 0) {
      after++;
      return;
    }
    if (it.kind !== "you") answered();
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

  // The reply so far: its whole blocks rendered as they settle (up to its last blank line outside a code fence),
  // each once; the open tail as plain text, so a half-typed table or fence never shows broken.
  let liveText = "";
  let liveAt = 0;
  function text(chunk: string): void {
    if (after > 0) return;
    answered();
    keep(() => {
      if (!live) {
        $(".te", list)?.remove();
        live = el("li", "blk ai live");
        live.dataset.cid = rid();
        const stop = el("button", "stop", "stop") as HTMLButtonElement;
        stop.type = "button";
        stop.onclick = cancel;
        const box = el("div", "md");
        box.append(el("span", "tl"));
        live.append(el("b", "k", agent), box, stop);
        list.append(live);
        liveText = "";
        liveAt = 0;
      }
      liveText += chunk;
      const box = md(live);
      const tail = $(".tl", box) as HTMLElement;
      const at = settled(liveText);
      if (at > liveAt) {
        box.insertBefore(markdown(liveText.slice(liveAt, at)), tail);
        liveAt = at;
      }
      tail.textContent = liveText.slice(liveAt);
    });
  }

  function setState(t: Turn): void {
    if (t.state !== state) since = Date.now();
    if (t.state !== state && ["done", "blocked", "error", "unconnected"].includes(t.state)) recopy();
    state = t.state;
    note = t.note ?? "";
    const [st, wait, close] = [$("#st"), $("#wait"), $("#close")];
    if (st) st.dataset.st = KEY[state] ?? "";
    main.dataset.state = state;
    if (state !== "working") stopping(false);
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
    keep(() => {
      connecting();
      turnEnd();
    });
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
  // The newest slice is kept on the device, so the thread opens offline on its last copy; a turn that settles
  // after the page opened (a thread that connected behind it, #48) keeps it again.
  const kept = `cooee.slice.${id}`;
  const keepCopy = (s: Slice): void => {
    try {
      localStorage.setItem(kept, JSON.stringify({ ...s, at: Date.now() }));
    } catch {
      // Full: no copy.
    }
  };
  const recopy = (): void => {
    fetch(`${api}/messages?before=${SLICE}`).then(
      async (r) => r.ok && keepCopy(await r.json()),
      () => {},
    );
  };
  const load = (q: string, where: "replace" | "before" | "after"): Promise<Slice> =>
    inOrder(async () => {
      const newest = where === "replace" && q === `before=${SLICE}`;
      const r = await fetch(`${api}/messages?${q}`).catch((x) => {
        if (newest && localStorage.getItem(kept)) return null;
        throw x;
      });
      if (r && !r.ok) throw await why(r);
      const s: Slice = r ? await r.json() : JSON.parse(localStorage.getItem(kept) ?? "null");
      if (newest && r) keepCopy(s);
      if (where === "replace") {
        copyAt = r ? 0 : (s.at ?? 0);
        // The outbox's rows (sending, failed) stay, unless the server has them.
        const stored = new Set(s.items.map((it) => (it.kind === "you" ? it.random_id : "")));
        const outbox = [...list.querySelectorAll<HTMLElement>(":scope > [data-rid]")];
        const kept = outbox.filter((r) => !stored.has(r.dataset.rid ?? ""));
        list.replaceChildren(render(s.items), ...kept);
        // The first message while the thread connects: the item it becomes takes its row.
        const p = s.pending;
        if (p && !s.skipped_after && !kept.some((r) => r.dataset.rid === p.random_id)) {
          const row = el("li", "blk me");
          row.dataset.rid = p.random_id;
          row.append(el("div", "md", p.text));
          list.append(row);
        }
        live = null;
        before = s.skipped_before;
        after = s.skipped_after;
        cursor = s.cursor;
        fold();
        if (s.turn.text && after === 0) text(s.turn.text);
        setState(s.turn);
        drawQueue(s.queue ?? []);
        waiting(s.waiting);
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

  // The sync dot: amber until the stream is open, green while it is, amber again when it drops, with a word:
  // reconnecting (it was open), or offline.
  let wasOpen = false;
  const sync = (on: boolean): void => {
    const dot = $(".dt");
    const net = $("#net");
    if (dot) dot.dataset.sync = on ? "on" : "off";
    wasOpen ||= on;
    if (net) net.textContent = on ? "" : !navigator.onLine ? "offline · " : wasOpen ? "reconnecting… · " : "";
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
    stream.addEventListener("queue", (e) => drawQueue(JSON.parse(e.data)));
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

  // A message's files for the API: each one's name and its bytes in base64.
  const payload = (files: File[]): Promise<{ name: string; data: string }[]> =>
    Promise.all(
      files.map(
        (f) =>
          new Promise<{ name: string; data: string }>((ok, no) => {
            const r = new FileReader();
            r.onload = () => ok({ name: f.name, data: String(r.result).replace(/^[^,]*,/, "") });
            r.onerror = () => no(r.error);
            r.readAsDataURL(f);
          }),
      ),
    );

  // The outbox: a row per send, `sending…` until the server stored it (then ✓), `failed` with tap-to-retry.
  function deliver(row: HTMLElement, text: string, random: string, files: File[]): void {
    row.className = "blk me sending";
    inOrder(async () => post<Item>(`${api}/send`, { text, random_id: random, files: await payload(files) })).then(
      (it) => {
        if (!row.dataset.rid) return;
        delete row.dataset.rid;
        fill(row, it);
      },
      () => {
        row.className = "blk me failed";
        const ct = $(".ct", row);
        if (ct) ct.textContent = "not sent";
        const retry = el("button", "retry", "failed · tap to retry") as HTMLButtonElement;
        retry.type = "button";
        retry.onclick = () => {
          retry.remove();
          if (ct) ct.textContent = "sending…";
          deliver(row, text, random, files);
        };
        row.append(retry);
      },
    );
  }

  // The files picked for the next message, shown above the box with [×].
  let picked: File[] = [];
  let urls: string[] = [];
  function drawPicked(): void {
    const box = $("#picked");
    if (!box) return;
    for (const u of urls) URL.revokeObjectURL(u);
    urls = picked.map((f) => URL.createObjectURL(f));
    box.hidden = !picked.length;
    box.replaceChildren(
      ...picked.map((f, i) => {
        const chip = el("span", "fi fpk", IMAGE.test(f.name) ? undefined : f.name);
        if (IMAGE.test(f.name)) {
          const img = el("img") as HTMLImageElement;
          img.src = urls[i] ?? "";
          img.alt = f.name;
          chip.append(img);
        }
        const x = el("button", "", "×") as HTMLButtonElement;
        x.type = "button";
        x.setAttribute("aria-label", `remove ${f.name}`);
        x.onclick = () => {
          picked.splice(i, 1);
          drawPicked();
          draft.changed();
        };
        chip.append(x);
        return chip;
      }),
    );
  }
  const filesIn = $<HTMLInputElement>("#files");
  if (filesIn)
    filesIn.onchange = () => {
      picked.push(...(filesIn.files ?? []));
      filesIn.value = "";
      drawPicked();
      draft.changed();
    };

  // Messages queued during a turn (the server sends each as a turn ends): [send now] steers it into the turn in
  // progress, [edit] takes it back into the box, [×] drops it.
  function drawQueue(queue: Queued[]): void {
    const q = $("#queue");
    if (!q) return;
    q.hidden = !queue.length;
    const act = (label: string, name: string, f: () => Promise<unknown>): HTMLButtonElement => {
      const b = el("button", "cn", label) as HTMLButtonElement;
      b.type = "button";
      b.setAttribute("aria-label", name);
      b.onclick = () => f().catch((x) => toast(message(x), true));
      return b;
    };
    q.replaceChildren(
      ...queue.map((m) => {
        const li = el("li", "qc");
        const files = m.files?.length ? ` · ${m.files.length} file${m.files.length > 1 ? "s" : ""}` : "";
        const take = (path: string): Promise<void> =>
          post<Queued[]>(`${api}/${path}`, { random_id: m.random_id }).then(drawQueue);
        li.append(
          el("span", "qt", `queued · ${m.text}${files}`),
          act("send now", `send now: ${m.text}`, () => take("steer")),
          act("edit", `edit: ${m.text}`, async () => {
            await take("unqueue");
            box.value = box.value ? `${box.value}\n${m.text}` : m.text;
            grow();
            draft.changed();
            box.focus();
          }),
          act("×", `remove: ${m.text}`, () => take("unqueue")),
        );
        return li;
      }),
    );
  }

  async function send(e: Event): Promise<void> {
    e.preventDefault();
    const value = box.value.trim();
    const files = picked;
    if ((!value && !files.length) || box.disabled) return;
    if (value === "/report") {
      box.value = "";
      return reportMode();
    }
    const random = rid();
    box.value = "";
    picked = [];
    drawPicked();
    grow();
    draft.save(true);
    // During a turn it waits for the turn to end.
    if (state === "working" || state === "blocked") {
      try {
        drawQueue(
          await post<Queued[]>(`${api}/queue`, { text: value, random_id: random, files: await payload(files) }),
        );
      } catch (x) {
        box.value = value;
        picked = files;
        drawPicked();
        toast(message(x), true);
      }
      return;
    }
    if (after > 0) await open();
    const row = el("li", "blk me");
    row.dataset.rid = random;
    row.append(el("div", "md pl", value));
    if (files.length) row.append(thumbs(files.map((f, i) => [f.name, urls[i] ?? URL.createObjectURL(f)])));
    row.append(el("span", "ct", "sending…"));
    keep(() => list.insertBefore(row, live));
    list.scrollTop = list.scrollHeight;
    deliver(row, value, random, files);
  }

  // [stop]: `stopping…` until the turn is no longer working.
  function stopping(on: boolean): void {
    const b = $<HTMLButtonElement>("#pill .stop");
    if (!b) return;
    b.textContent = on ? "stopping…" : "stop";
    b.disabled = on;
  }
  function cancel(): void {
    stopping(true);
    post(`${api}/cancel`, {}).catch((x) => {
      stopping(false);
      toast(message(x), true);
    });
  }

  // The box grows with its text up to 6 lines (the style sheet's max-height), then scrolls.
  function grow(): void {
    box.style.height = "auto";
    box.style.height = `${box.scrollHeight + 2}px`;
  }

  // The keyboard: where the browser doesn't shrink the page for it (iOS), the chat takes the visual viewport's
  // height, so the composer sits on the keyboard, and the page never scrolls away under it.
  // Elsewhere the page itself shrinks, and 100dvh is right.
  const vv = window.visualViewport;
  if (vv) {
    const fit = (): void => {
      const style = document.documentElement.style;
      if (vv.height < innerHeight - 1) style.setProperty("--vvh", `${Math.round(vv.height)}px`);
      else style.removeProperty("--vvh");
      if (scrollY) scrollTo(0, 0);
    };
    vv.addEventListener("resize", fit);
    vv.addEventListener("scroll", fit);
    addEventListener("resize", fit);
    fit();
  }

  // A long press (or right click) on a message: copy, copy as markdown, quote (as `> ` lines in the box).
  const menu = $("#menu");
  let pressed: HTMLElement | null = null;
  let hold = 0;
  let held = false;
  let holdAt = [0, 0];
  const press = (row: HTMLElement, x: number, y: number): void => {
    if (!menu) return;
    pressed = row;
    menu.style.left = `${Math.max(8, Math.min(x - 16, innerWidth - 216))}px`;
    menu.style.top = `${Math.max(8, Math.min(y - 16, innerHeight - 168))}px`;
    if (!menu.matches(":popover-open")) menu.showPopover();
  };
  const bubble = (t: EventTarget | null): HTMLElement | null =>
    (t as Element).closest?.<HTMLElement>(".blk.ai:not(.live), .blk.me[data-id]") ?? null;
  list.addEventListener("contextmenu", (e) => {
    const row = bubble(e.target);
    if (!row) return;
    e.preventDefault();
    press(row, e.clientX, e.clientY);
  });
  list.addEventListener(
    "touchstart",
    (e) => {
      const row = bubble(e.target);
      const t = e.touches[0];
      clearTimeout(hold);
      if (!row || !t) return;
      holdAt = [t.clientX, t.clientY];
      hold = setTimeout(() => {
        held = true;
        press(row, t.clientX, t.clientY);
      }, 500);
    },
    { passive: true },
  );
  list.addEventListener(
    "touchmove",
    (e) => {
      const t = e.touches[0];
      if (t && Math.hypot(t.clientX - (holdAt[0] ?? 0), t.clientY - (holdAt[1] ?? 0)) > 10) clearTimeout(hold);
    },
    { passive: true },
  );
  // The release of a press that opened the menu clicks nothing under the finger.
  list.addEventListener("touchend", (e) => {
    clearTimeout(hold);
    if (held) e.preventDefault();
    held = false;
  });
  list.addEventListener("touchcancel", () => clearTimeout(hold), { passive: true });
  // A manual popover: a touch's release lands on the reply it pressed, which would light-dismiss an auto one.
  // A press elsewhere, or Esc, closes it.
  document.addEventListener("pointerdown", (e) => {
    if (menu?.matches(":popover-open") && !menu.contains(e.target as Node)) menu.hidePopover();
  });
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && menu?.matches(":popover-open")) menu.hidePopover();
  });
  menu?.addEventListener("click", (e) => {
    const m = (e.target as Element).closest?.<HTMLElement>("[data-m]")?.dataset.m;
    const row = pressed;
    if (!m || !row) return;
    menu.hidePopover();
    const said = raw.get(row) ?? md(row).textContent ?? "";
    if (m === "quote") {
      const lines = said.split("\n").map((l) => `> ${l}`);
      box.value = `${box.value ? `${box.value.trimEnd()}\n` : ""}${lines.join("\n")}\n\n`;
      grow();
      draft.changed();
      box.focus();
      return;
    }
    // biome-ignore lint/performance/useDomNodeTextContent: the copy keeps the rendered line breaks.
    navigator.clipboard?.writeText(m === "md" ? said : md(row).innerText).then(
      () => toast("copied"),
      () => toast("couldn't copy", true),
    );
  });

  // The image viewer: a thumbnail opens it full size; back (or a tap, or Esc) closes it.
  const viewer = $("#viewer");
  const shot = viewer && $<HTMLImageElement>("img", viewer);
  list.addEventListener("click", (e) => {
    const a = (e.target as Element).closest?.<HTMLAnchorElement>("a.fi.im");
    if (!a || !viewer || !shot) return;
    e.preventDefault();
    shot.src = a.href;
    shot.alt = $("img", a)?.getAttribute("alt") ?? "";
    viewer.showPopover();
    history.pushState({ viewer: true }, "");
  });
  viewer?.addEventListener("click", () => history.back());
  viewer?.addEventListener("toggle", (e) => {
    if ((e as ToggleEvent).newState === "closed" && history.state?.viewer) history.back();
  });
  addEventListener("popstate", () => {
    if (viewer?.matches(":popover-open")) viewer.hidePopover();
  });

  // Rename: the title opens a box with it.
  const rt = $<HTMLFormElement>("#rt");
  if (rt)
    rt.onsubmit = async (e) => {
      e.preventDefault();
      const title = String(new FormData(rt).get("title") ?? "").trim();
      try {
        await post(`${api}/title`, { title });
      } catch (x) {
        return toast(message(x), true);
      }
      const b = $(".th .ttl");
      if (b) {
        b.textContent = title;
        b.setAttribute("aria-label", `${title} · rename`);
      }
      document.title = title;
      rt.hidePopover();
      toast("renamed");
    };

  // The draft: kept per thread in IndexedDB with its picked files, written 1 s after typing stops and when the page
  // hides; newest wins.
  type Draft = { text: string; at: number; files?: File[] };
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
        if (!d || box.value || picked.length) return;
        box.value = d.text;
        picked = d.files ?? [];
        drawPicked();
        grow();
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
        const mine: Draft = { text: box.value, at, files: picked };
        const old = await get();
        if (!old || old.at <= mine.at) await tx("readwrite", (s) => s.put(mine, id)).catch(() => {});
      },
    };
  })();

  box.addEventListener("input", () => {
    grow();
    draft.changed();
  });
  document.addEventListener("visibilitychange", () => {
    if (document.visibilityState === "hidden") draft.save();
  });
  $("#composer")?.addEventListener("submit", send);
  box.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) send(e);
  });
  // The model and effort chips (#553 row 34): a pick is kept on the thread, which says from when it applies.
  for (const s of document.querySelectorAll<HTMLSelectElement>(".mc select"))
    s.onchange = () => {
      const pick = (n: string): string => $<HTMLSelectElement>(`#${n}`)?.value ?? "";
      post(`${api}/model`, { model: pick("model"), effort: pick("effort") }).catch((x) => toast(message(x), true));
    };
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
