// cooee's pages. The thread list: the new-thread form. The chat: a window of about 9 screens of items around an
// anchor (slices by id as the reader nears an edge), scroll kept on a message rather than pixels, live events over
// SSE resumed from the last one seen, the outbox (sending → sent → failed, tap to retry), the per-thread draft in
// IndexedDB, approvals that lock once tapped, and debounced read marks (which also close the thread's notifications).
// Every page but pairing registers the service worker (sw.js: push and the last copy of each page); the thread
// list's 🔔 turns alerts on. On every page (#21): short toasts, the loading strip (with a copy's age when it is an old
// one), refresh (↻, a pull down, coming back after 30 s or the network returning), Settings' alerts, devices and
// report mode; on the pairing page, a code to approve on the Mac.
// States are theme keys (data-st) that look.js draws: the thread bar, the working pill (its sprite following the
// running tool), the live line and a turn's end.
// A plugin tab hosts the plugin's sandboxed frame.
const $ = (s, el = document) => el.querySelector(s);
const rid = () => Date.now().toString(36) + Math.random().toString(36).slice(2, 10);
// A failed answer as one plain sentence: cooee's own (marked `cooee-error`), else a general one, never raw JSON.
const why = async (r) =>
  new Error(r.headers.has("cooee-error") ? await r.text() : `cooee couldn't do that (error ${r.status}): reload and try again.`);
const post = (url, body) =>
  fetch(url, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) })
    .then(async (r) => (!r.ok ? Promise.reject(await why(r)) : r.headers.get("content-type")?.includes("json") ? r.json() : r.text()));
const make = (tag, cls, text) => { const e = document.createElement(tag); if (cls) e.className = cls; if (text !== undefined) e.textContent = text; return e; };

// A toast: one short line under the header that clears itself (an error stays longer and may wrap); a tap clears it.
let toastTimer = 0;
function toast(text, bad) {
  const t = $("#toast");
  if (!t) return;
  t.textContent = text;
  t.className = bad ? "r" : "g";
  t.hidden = false;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => { t.hidden = true; }, bad ? 6000 : 2500);
}
$("#toast")?.addEventListener("click", (e) => { e.currentTarget.hidden = true; });

// The loading strip: thin, under the header, never in the way of a tap; `at` (an old copy's time) adds its age.
const ago = (at) => { const m = Math.round((Date.now() - at) / 60000); return m < 1 ? "just now" : m < 120 ? `${m}m ago` : `${Math.round(m / 60)}h ago`; };
let copyAt = Date.now() - Number(document.body.dataset.at) > 120000 ? Number(document.body.dataset.at) : 0;
function strip(loading) {
  const s = $("#ld");
  if (!s) return;
  s.hidden = !loading && !copyAt;
  s.classList.toggle("run", loading);
  $(".ago", s).textContent = copyAt ? `as of ${ago(copyAt)}` : "";
}
strip(false);
// A fetch answered from the service worker's copy carries its time.
const copied = (r) => { copyAt = Number(r.headers.get("cooee-at")) || 0; };

// Refresh: ↻, a pull down from the very top, the app coming back after 30 s away, or the network coming back. The
// chat reloads its newest slice and stream (page.refresh); the thread list its cards; any other page reloads.
const page = { refresh: null, top: () => true };
let refreshing = null;
function refresh() {
  if (refreshing || location.pathname === "/pair") return refreshing;
  const rf = $("#rf");
  rf?.classList.add("rfs");
  strip(true);
  refreshing = (async () => {
    if (page.refresh) return page.refresh();
    const list = $("main.list");
    if (!list) return location.reload();
    const r = await fetch("/");
    if (!r.ok) throw await why(r);
    copied(r);
    const fresh = new DOMParser().parseFromString(await r.text(), "text/html");
    for (const e of list.querySelectorAll(":scope > .fc, :scope > .emp")) e.remove();
    for (const e of fresh.querySelectorAll("main.list > .emp")) list.insertBefore(e, $("#new"));
    list.append(...fresh.querySelectorAll("main.list > .fc"));
  })().catch((x) => toast(`couldn't refresh: ${x.message}`, true)).finally(() => {
    rf?.classList.remove("rfs");
    strip(false);
    refreshing = null;
  });
  return refreshing;
}
$("#rf")?.addEventListener("click", refresh);
let pullY = null, hiddenAt = 0;
addEventListener("touchstart", (e) => {
  const top = ($("#msgs") ?? document.scrollingElement).scrollTop <= 0 && page.top() && !$("#sheet:not([hidden])");
  pullY = top ? e.touches[0].clientY : null;
}, { passive: true });
addEventListener("touchmove", (e) => { if (pullY !== null && e.touches[0].clientY - pullY > 70) { pullY = null; refresh(); } }, { passive: true });
document.addEventListener("visibilitychange", () => {
  if (document.hidden) hiddenAt = Date.now();
  else if (hiddenAt && Date.now() - hiddenAt > 30000) refresh();
});
addEventListener("online", refresh);
addEventListener("pageshow", (e) => { if (e.persisted) refresh(); });

// The thread list's form: open on `#new`, start a thread, then open it.
const form = $("#new");
if (form) {
  const open = () => { if (location.hash === "#new") form.hidden = false; else if ($(".fc")) form.hidden = true; };
  addEventListener("hashchange", open);
  open();
  form.addEventListener("submit", async (e) => {
    e.preventDefault();
    const f = new FormData(form), err = $("#new-err");
    const body = { agent: f.get("agent"), folder: f.get("folder"), text: f.get("text"), random_id: rid() };
    form.querySelector("button[type=submit]").disabled = true;
    try { location.href = `/t/${(await post("/threads", body)).id}`; } catch (x) {
      err.textContent = x.message; err.hidden = false;
      form.querySelector("button[type=submit]").disabled = false;
    }
  });
}

// The folder picker: the search shows the folders whose name has it (up to 2 levels in), else the first level;
// [+ new folder] makes one inside the folder picked, and picks it.
const fq = $("#fq");
if (fq) {
  fq.oninput = () => {
    const q = fq.value.trim().toLowerCase();
    for (const l of form.querySelectorAll(".fl label")) l.hidden = q ? !l.textContent.trim().split("/").pop().toLowerCase().includes(q) : l.dataset.d > 1;
  };
  $("#mkdir").onclick = async () => {
    const at = form.querySelector(".fl :checked").parentNode, label = at.textContent.trim(), err = $("#new-err");
    const name = prompt(`New folder in ${label}:`);
    if (!name) return;
    try {
      const row = at.cloneNode(true), input = $("input", row);
      input.value = await post("/folders/new", { parent: $("input", at).value, name });
      row.lastChild.textContent = `${label}/${name}`;
      row.dataset.d = Math.min(2, Number(at.dataset.d) + 1);
      row.hidden = false; err.hidden = true;
      at.after(row);
      input.checked = true;
    } catch (x) { err.textContent = x.message; err.hidden = false; }
  };
}

// An empty picker's hint: its copy button puts the fix line on the clipboard.
document.addEventListener("click", (e) => {
  const b = e.target.closest?.("[data-copy]");
  b && navigator.clipboard?.writeText(b.dataset.copy).then(() => { b.textContent = "copied"; }, () => {});
});

// The sign-ins sheet: open on `#signins`. [sign in] starts the provider's own login in a shed terminal; its screen
// (redacted by the server) is drawn every second with its links tappable, a code typed below goes to it, and once
// it ends the provider's status is shown again.
const sheet = $("#signins");
if (sheet) {
  const open = () => { sheet.hidden = location.hash !== "#signins"; };
  addEventListener("hashchange", open);
  open();
  for (const row of sheet.querySelectorAll(".pv")) {
    const api = `/logins/${row.dataset.provider}`, lg = $(".lg", row), out = $(".out", row), code = $("input", row);
    const [state, signIn] = [$(".fh span", row), $(".si", row)], line = $(".pv > .ds", row);
    let timer = 0;
    const draw = (v) => {
      if (v.state !== "running") {
        clearInterval(timer); lg.hidden = true;
        const s = v.status ?? { signed_in: false, line: "" };
        state.className = s.signed_in ? "g" : "m";
        state.textContent = s.signed_in ? "✓ signed in" : "signed out";
        line.textContent = [v.why, s.line].filter(Boolean).join(" · ");
        signIn.hidden = s.signed_in || !!s.install;
        return;
      }
      state.className = "a"; state.textContent = "signing in"; lg.hidden = false; signIn.hidden = true;
      out.replaceChildren();
      let rest = v.screen;
      for (const url of v.links) {
        const i = rest.indexOf(url);
        if (i < 0) continue;
        const a = document.createElement("a");
        a.href = url; a.target = "_blank"; a.rel = "noopener"; a.textContent = url;
        out.append(rest.slice(0, i), a);
        rest = rest.slice(i + url.length);
      }
      out.append(rest);
    };
    const poll = async () => { const r = await fetch(api); if (r.ok) draw(await r.json()); };
    signIn.onclick = async () => {
      try { await post(api, null); } catch (x) { line.textContent = x.message; return; }
      clearInterval(timer); timer = setInterval(poll, 1000); poll();
    };
    $("form", row).onsubmit = async (e) => {
      e.preventDefault();
      try { await post(`${api}/send`, { text: code.value }); code.value = ""; poll(); } catch (x) { line.textContent = x.message; }
    };
    $(".cn", row).onclick = () => post(`${api}/cancel`, {}).then(poll, poll);
  }
}

// Not while pairing: a worker installing and claiming the page as the pair redirects can leave a home-screen app on
// a bare background until a reload (the first run, #25).
if ("serviceWorker" in navigator && location.pathname !== "/pair") navigator.serviceWorker.register("/sw.js").catch(() => {});
// Closes thread `thread`'s notifications up to item `read`.
const cleared = (thread, read) => navigator.serviceWorker?.ready.then((r) => r.active?.postMessage({ thread, read }));

// Alerts, truthfully: on only when this browser has a subscription the server keeps for this device. The 🔔 and a
// dot on ⚙ show where push works and isn't on; Settings' alerts section turns them on or off and sends a test.
async function alertState() {
  if (!("PushManager" in window) || !navigator.serviceWorker) return { state: "unsupported" };
  if (Notification.permission === "denied") return { state: "blocked" };
  const reg = await navigator.serviceWorker.ready, sub = await reg.pushManager.getSubscription();
  if (!sub || Notification.permission !== "granted") return { state: "off", reg, sub };
  const mine = await fetch("/push/endpoints").then((r) => (r.ok ? r.json() : []), () => []);
  return { state: mine.includes(sub.endpoint) ? "on" : "off", reg, sub };
}
async function alertsOn() {
  if ((await Notification.requestPermission()) !== "granted") throw new Error("notifications are off for this app");
  const reg = await navigator.serviceWorker.ready, r = await fetch("/push/key");
  if (!r.ok) throw await why(r);
  const key = (await r.json()).key.replace(/-/g, "+").replace(/_/g, "/");
  const sub = await reg.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: Uint8Array.from(atob(key), (c) => c.charCodeAt(0)) });
  const { endpoint, keys } = sub.toJSON();
  await post("/push/subscribe", { endpoint, ...keys });
}
async function alertsOff() {
  const { sub } = await alertState();
  if (sub) { await post("/push/unsubscribe", { endpoint: sub.endpoint }); await sub.unsubscribe(); }
}
async function alertMarks() {
  const { state } = await alertState(), gear = $("header [data-open=settings]"), bell = $("#alerts");
  if (gear) {
    gear.toggleAttribute("data-dot", state === "off" || state === "blocked");
    gear.setAttribute("aria-label", state === "off" || state === "blocked" ? "settings · alerts are off" : "settings");
  }
  if (bell) bell.hidden = state !== "off";
  return state;
}
const ALERTS = {
  on: "alerts are on", off: "alerts are off",
  blocked: "notifications are blocked for this app: allow them in the phone's Settings",
  unsupported: "this browser can't show alerts: on iPhone, add cooee to the Home Screen and open it there",
};
async function alertsDo(f, ok) {
  try { await f(); toast(ok); } catch (x) { toast(`alerts: ${x.message}`, true); }
  await alertMarks();
  fillAlerts();
}
async function fillAlerts() {
  const box = $("#s-alerts-box");
  if (!box) return;
  const state = await alertState().then((a) => a.state, () => "unsupported"), row = make("div", "lrw");
  row.append(make("span", state === "on" ? "g" : "m", ALERTS[state]));
  const act = (label, f) => { const b = make("button", "", label); b.type = "button"; b.onclick = f; row.append(b); };
  if (state === "off") act("turn on", () => alertsDo(alertsOn, "alerts are on"));
  if (state === "on") {
    act("send a test", () => post("/push/test", {}).then((t) => toast(t), (x) => toast(x.message, true)));
    act("turn off", () => alertsDo(alertsOff, "alerts are off"));
  }
  box.replaceChildren(row);
}
const bell = $("#alerts");
if (bell) bell.onclick = () => alertsDo(alertsOn, "alerts are on");
if (location.pathname !== "/pair") alertMarks().catch(() => {});

// Settings' devices section: each paired device, this one marked, with [remove] (removing this one unpairs it).
async function fillDevices() {
  const box = $("#s-devices-box");
  if (!box) return;
  const r = await fetch("/devices").catch(() => null);
  if (!r?.ok) return box.replaceChildren(make("p", "ds", "devices show when online"));
  const day = (at) => new Date(at).toLocaleDateString();
  box.replaceChildren(...(await r.json()).map((d) => {
    const row = make("div", "lrw"), b = make("button", "cn", "remove");
    row.append(make("span", "", `${d.label} · ${d.id}${d.me ? " · this device" : ""}`), make("span", "ds", `paired ${day(d.created)} · seen ${day(d.used)}`));
    b.type = "button";
    b.setAttribute("aria-label", `remove ${d.label} ${d.id}`);
    b.onclick = async () => {
      if (!confirm(d.me ? "Unpair this device? It goes back to pairing." : `Unpair ${d.label} ${d.id}?`)) return;
      try { toast(await post("/devices/remove", { id: d.id })); } catch (x) { return toast(x.message, true); }
      if (d.me) location.href = "/pair"; else fillDevices();
    };
    row.append(b);
    return row;
  }));
}
addEventListener("cooee-sheet", (e) => { if (e.detail === "settings") { fillAlerts(); fillDevices(); } });

// Report mode (Settings → report a problem, or `/report` sent in a thread): the next tap picks an element, outlined;
// the note box at the bottom sends it with the page, the element and the look; `cooee reports` lists them.
const where = (n) => {
  const out = [];
  for (; n?.nodeType === 1 && out.length < 6; n = n.parentElement) {
    out.unshift(n.tagName.toLowerCase() + (n.id ? `#${n.id}` : "") + [...n.classList].filter((c) => c !== "repo").map((c) => `.${c}`).join(""));
  }
  return out.join(" > ");
};
function reportMode() {
  globalThis.cooeeSheet?.("");
  document.body.classList.add("rep");
  toast("report: tap the part that's wrong");
  const pick = (e) => {
    e.preventDefault();
    e.stopPropagation();
    removeEventListener("click", pick, true);
    const t = e.target, bar = make("form", "repf"), note = make("input"), send = make("button", "", "send"), no = make("button", "cn", "cancel");
    t.classList.add("repo");
    note.setAttribute("aria-label", "what's wrong");
    note.placeholder = "what's wrong?";
    no.type = "button";
    const done = () => { bar.remove(); t.classList.remove("repo"); document.body.classList.remove("rep"); };
    no.onclick = done;
    bar.onsubmit = async (ev) => {
      ev.preventDefault();
      const look = Object.entries(document.documentElement.dataset).map(([k, v]) => `${k}=${v}`).join(" ");
      try {
        toast(await post("/report", { note: note.value, path: location.pathname + location.hash, target: where(t),
          html: t.outerHTML.slice(0, 600), look, viewport: `${innerWidth}x${innerHeight}` }));
        done();
      } catch (x) { toast(x.message, true); }
    };
    bar.append(note, send, no);
    document.body.append(bar);
    note.focus();
  };
  setTimeout(() => addEventListener("click", pick, true));
}
addEventListener("click", (e) => { if (e.target.closest?.("[data-report]")) reportMode(); });
if (location.hash === "#report") reportMode();

// Pairing the other way round: the phone asks for a code, shows it, and polls until `cooee pair <code>` on the Mac
// approves it; then it is paired and opens the thread list.
const ask = $("#ask");
if (ask) {
  let claim = "", timer = 0;
  const wait = $("#ask-wait"), err = $("#ask-err"), go = $("#ask-go");
  const reset = (why) => { clearInterval(timer); wait.hidden = true; go.hidden = false; err.textContent = why ?? ""; err.hidden = !why; };
  go.onclick = async () => {
    try {
      const r = await post("/pair/request", {});
      claim = r.claim;
      $("#ask-code").textContent = r.code;
      $("#ask-cmd").textContent = `cooee pair ${r.code}`;
      $("#ask-copy").dataset.copy = `cooee pair ${r.code}`;
      wait.hidden = false; go.hidden = true; err.hidden = true;
      timer = setInterval(async () => {
        const c = await fetch("/pair/claim", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ claim }) }).catch(() => null);
        if (c?.status === 200) { clearInterval(timer); location.replace("/"); } else if (c?.status === 410) reset(await c.text());
      }, 2000);
    } catch (x) { reset(x.message); }
  };
  $("#ask-cancel").onclick = () => { post("/pair/cancel", { claim }).catch(() => {}); reset(); };
}

// A plugin tab: the frame (sandboxed, see frame.js) asks this page to fetch or stream paths of its own backend only
// (/p/<name>/api/…; anything else is refused), gets the theme's CSS variables, and its URL's #hash (a push's link).
const frame = $("iframe.pf");
if (frame) {
  const base = `/p/${frame.dataset.plugin}/api`, streams = new Map();
  const to = (m) => frame.contentWindow.postMessage(m, "*");
  const own = (path) => {
    if (typeof path !== "string" || !path.startsWith("/")) return null;
    const u = new URL(base + path, location.origin);
    return u.origin === location.origin && u.pathname.startsWith(`${base}/`) ? u : null;
  };
  addEventListener("message", async (e) => {
    if (e.source !== frame.contentWindow) return;
    const m = e.data ?? {}, u = own(m.path);
    if (m.op === "fetch") {
      const answer = (r) => to({ op: "response", id: m.id, ...r });
      if (!u || !["GET", "POST"].includes(m.method)) return answer({ status: 403, ok: false, text: "not this plugin's backend" });
      try {
        const r = await fetch(u, { method: m.method, body: m.body, headers: m.type ? { "content-type": m.type } : {} });
        answer({ status: r.status, ok: r.ok, type: r.headers.get("content-type"), text: await r.text() });
      } catch (x) { answer({ status: 0, ok: false, text: x.message }); }
    } else if (m.op === "events") {
      if (!u) return to({ op: "event", id: m.id, event: "error", data: "not this plugin's backend" });
      const es = new EventSource(u);
      es.onmessage = (ev) => to({ op: "event", id: m.id, ...JSON.parse(ev.data) });
      streams.set(m.id, es);
    } else if (m.op === "close") { streams.get(m.id)?.close(); streams.delete(m.id); }
  });
  const vars = ["--bg", "--card", "--edge", "--line", "--fg", "--mute", "--l", "--link", "--g", "--a", "--r", "--p", "--font", "--fs", "--fs-meta", "--r-card", "--r-pill"];
  const theme = () => {
    const css = getComputedStyle(document.documentElement);
    to({ op: "theme", vars: Object.fromEntries(vars.map((v) => [v, css.getPropertyValue(v).trim()])) });
  };
  frame.onload = theme;
  // look.js sets the scheme's colours on <html>: a new pick or a system switch goes to the frame too.
  new MutationObserver(theme).observe(document.documentElement, { attributes: true });
  matchMedia("(prefers-color-scheme: light)").addEventListener("change", theme);
  frame.src = frame.dataset.src + location.hash;
}

const main = $("main.chat");
if (main) chat(main);

function chat(main) {
  const id = main.dataset.thread, agent = main.dataset.agent, list = $("#msgs"), box = $("#box");
  const api = `/threads/${id}`, SLICE = 20, KEEP = 9, EDGE = 2;
  let before = 0, after = 0, cursor = "", es = null, live = null, state = main.dataset.state, since = Date.now(), note = "";
  let read = Number(main.dataset.read), seen = read, readTimer = 0;
  // Sends and history loads of this thread run one after another.
  let chain = Promise.resolve();
  const inOrder = (f) => (chain = chain.then(f, f));
  const screen = () => list.clientHeight || innerHeight;
  const atBottom = () => after === 0 && list.scrollHeight - list.scrollTop - list.clientHeight < 8;
  const rowOf = (itemId) => list.querySelector(`[data-id="${itemId}"]`);
  const rows = () => [...list.children].filter((r) => r.dataset.id);
  const ids = () => rows().map((r) => Number(r.dataset.id));
  const time = (at) => new Date(at).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
  const clock = (ms) => { const s = Math.max(0, Math.floor(ms / 1000)); return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`; };
  const el = make;

  // Scroll is kept on a message: the first row in view and its offset are restored after any change of layout.
  function keep(change) {
    const stick = atBottom();
    const anchor = rows().find((r) => r.offsetTop + r.offsetHeight > list.scrollTop);
    const offset = anchor ? anchor.offsetTop - list.scrollTop : 0;
    change();
    if (stick && after === 0) list.scrollTop = list.scrollHeight;
    else if (anchor && anchor.isConnected) list.scrollTop = anchor.offsetTop - offset;
  }

  // The working sprite follows the tool that runs: reading and searching, running a command, else editing.
  const toolKind = (title) => (/^(read|search|grep|glob|find|list|fetch|view)\b/i.test(title) ? "read"
    : /^(\$|run|bash|exec|execute|terminal)\b/i.test(title) ? "run" : "edit");
  const KEY = { blocked: "needs", working: "working", done: "done", error: "fail", ended: "ended" };

  // A turn's end: one quiet line under its last block, `[sprite] in the stable · done 09:21 · 9 min · 6 tools`.
  function turnEnd() {
    $(".te", list)?.remove();
    if ((state !== "done" && state !== "error") || after > 0) return;
    const all = rows(), me = all.findLastIndex((r) => r.classList.contains("me")), last = all.at(-1);
    if (me < 0 || !last) return;
    const tools = all.slice(me).filter((r) => r.classList.contains("run")).length;
    const mins = Math.round((Number(last.dataset.at) - Number(all[me].dataset.at)) / 60000);
    const te = el("li", "te"), st = el("span", "", "");
    st.dataset.st = KEY[state];
    st.dataset.plain = "";
    const more = state === "error" ? [note] : [mins >= 1 ? `${mins} min` : "", tools ? `${tools} tool${tools > 1 ? "s" : ""}` : ""];
    te.append(st, el("span", "ct", [time(Number(last.dataset.at)), ...more].filter(Boolean).join(" · ")));
    list.append(te);
  }

  function fill(row, it) {
    row.replaceChildren();
    row.dataset.id = it.id;
    row.dataset.at = it.at;
    const k = it.kind;
    if (k === "you") {
      row.className = "blk me";
      row.append(el("div", "md", it.text), el("span", "ct", time(it.at)));
    } else if (k === "agent") {
      row.className = "blk ai";
      row.append(el("b", "k", agent), el("div", "md", it.text));
      if (it.stop_reason !== "end_turn") row.append(el("span", "ct", it.stop_reason));
    } else if (k === "tool") {
      const s = it.status, ok = s === "completed", bad = s === "failed" || s === "cancelled";
      row.className = "blk run";
      const ts = el("span", `ts ${ok ? "g" : bad ? "r" : "a"}`, ok ? "✓" : bad ? `✕ ${s}` : "⋯");
      if (!ok && !bad) { ts.dataset.since = it.at; $("#pst").dataset.tool = toolKind(it.title); }
      row.append(el("span", "k", "tool"), el("span", "cmd", it.title), ts);
    } else if (k === "approval") {
      row.className = "blk ap";
      row.append(el("b", "k", it.plugin ?? agent), el("div", "md", it.tool));
      const chosen = it.chosen ?? it.tapped;
      if (chosen && it.chosen) {
        const o = it.options.find((o) => o.id === chosen);
        row.append(el("p", "ans", `✓ ${o ? o.name : chosen}`));
      } else {
        const chc = el("div", "chc");
        for (const o of it.options) {
          const b = el("button", "", o.name);
          b.type = "button";
          b.disabled = !!chosen;
          b.onclick = () => approve(row, it.id, o.id);
          chc.append(b);
        }
        if (chosen) chc.classList.add("busy");
        row.append(chc);
      }
    } else if (k === "plan") {
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
  async function approve(row, item, option) {
    const chc = $(".chc", row);
    chc.classList.add("busy");
    for (const b of chc.querySelectorAll("button")) b.disabled = true;
    // The answer comes back over the event stream (tapped, then chosen), never older than what is shown.
    try { await post(`${api}/approve`, { item, option }); } catch (x) {
      toast(x.message, true);
      chc.classList.remove("busy");
      for (const b of chc.querySelectorAll("button")) b.disabled = false;
    }
  }

  // An item, new or changed: replaced in place, appended at the newest end, or counted beyond the window.
  function put(it) {
    const row = rowOf(it.id);
    if (row) return keep(() => fill(row, it));
    if (after > 0) { after++; return; }
    keep(() => {
      $(".te", list)?.remove();
      let row = it.kind === "you" && list.querySelector(`[data-rid="${it.random_id}"]`);
      // The final reply takes over the streaming row's node.
      if (!row && it.kind === "agent" && live) { row = live; live = null; }
      if (row) { delete row.dataset.rid; fill(row, it); } else list.insertBefore(fill(el("li"), it), live);
      if (live) list.append(live);
      turnEnd();
    });
    markRead();
  }

  function text(chunk) {
    if (after > 0) return;
    keep(() => {
      if (!live) {
        $(".te", list)?.remove();
        live = el("li", "blk ai live");
        live.dataset.cid = rid();
        const stop = el("button", "stop", "stop");
        stop.type = "button";
        stop.onclick = cancel;
        live.append(el("b", "k", agent), el("div", "md"), stop);
        list.append(live);
      }
      $(".md", live).textContent += chunk;
    });
  }

  function setState(t) {
    if (t.state !== state) since = Date.now();
    state = t.state;
    note = t.note ?? "";
    $("#st").dataset.st = KEY[state] ?? "";
    main.dataset.state = state;
    $("#pill").hidden = state !== "working";
    $("#wait").hidden = state !== "blocked";
    if (state === "ended") {
      box.disabled = $("#composer button[type=submit]").disabled = true;
      $("#close").hidden = true;
    }
    if (state !== "working" && state !== "blocked" && live && !$(".md", live).textContent) { live.remove(); live = null; }
    keep(turnEnd);
  }

  // Drops the rows more than (KEEP - 1) / 2 screens past the viewport, on the side away from the reader, so the
  // window is about KEEP screens around the anchor. Called inside keep(), with the scroll position current.
  function trim(fromTop) {
    const far = ((KEEP - 1) / 2) * screen(), top = list.scrollTop, bottom = top + list.clientHeight;
    const out = rows().filter((r) => (fromTop ? r.offsetTop + r.offsetHeight < top - far : r.offsetTop > bottom + far));
    for (const r of out) r.remove();
    if (fromTop) before += out.length; else after += out.length;
  }

  function render(items) {
    const frag = document.createDocumentFragment();
    for (const it of items) frag.append(fill(el("li"), it));
    return frag;
  }

  // Loads a slice: `q` is the query; `where` is "replace", "before" (older) or "after" (newer).
  const load = (q, where) => inOrder(async () => {
    // The newest slice is kept on the device, so the thread opens offline on its last copy.
    const kept = `cooee.slice.${id}`, newest = where === "replace" && q === `before=${SLICE}`;
    const r = await fetch(`${api}/messages?${q}`).catch((x) => (newest && localStorage.getItem(kept) ? null : Promise.reject(x)));
    if (r && !r.ok) throw await why(r);
    const s = r ? await r.json() : JSON.parse(localStorage.getItem(kept));
    if (newest && r) try { localStorage.setItem(kept, JSON.stringify({ ...s, at: Date.now() })); } catch { /* full: no copy */ }
    if (where === "replace") {
      copyAt = r ? 0 : s.at;
      list.replaceChildren(render(s.items));
      live = null;
      before = s.skipped_before; after = s.skipped_after; cursor = s.cursor;
      if (s.turn.text && after === 0) text(s.turn.text);
      setState(s.turn);
      return s;
    }
    const have = new Set(ids()), fresh = s.items.filter((it) => !have.has(it.id));
    keep(() => {
      if (where === "before") { list.prepend(render(fresh)); before = s.skipped_before; }
      else { list.insertBefore(render(fresh), live); after = s.skipped_after; }
    });
    keep(() => trim(where === "after"));
    if (where === "after" && after === 0 && s.turn.text && !live) text(s.turn.text);
    return s;
  });

  // The first slice is held up to ~300 ms and painted at once; a skeleton shows only after that.
  async function open() {
    const skeleton = setTimeout(() => {
      list.classList.add("skeleton");
      const fig = el("li", "emp"), w = el("p");
      fig.append(el("span"), w);
      fig.firstChild.dataset.s = "loading";
      w.dataset.w = "loading";
      w.dataset.plain = "";
      list.append(fig);
    }, 300);
    const unread = ids().length === 0 && read > 0 ? read + 1 : 0;
    const s = await load(unread ? `around=${unread}&before=${SLICE / 2}&after=${SLICE}` : `before=${SLICE}`, "replace");
    clearTimeout(skeleton);
    list.classList.remove("skeleton");
    const first = unread && rowOf(unread);
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

  function listen() {
    es?.close();
    es = new EventSource(`${api}/events?since=${encodeURIComponent(cursor)}`);
    sync(false);
    es.onopen = () => sync(true);
    es.addEventListener("item", (e) => put(JSON.parse(e.data)));
    es.addEventListener("text", (e) => text(JSON.parse(e.data).text));
    es.addEventListener("state", (e) => { const t = JSON.parse(e.data); moment(t.state); setState(t); });
    // Another run or a page too far behind: load the newest slice again.
    es.addEventListener("reset", () => { es.close(); open(); });
    es.onerror = () => { sync(false); if (es.readyState === EventSource.CLOSED) setTimeout(open, 2000); };
  }
  // The sync dot: amber until the stream is open, green while it is, amber again when it drops.
  const sync = (on) => { $(".dt").dataset.sync = on ? "on" : "off"; };
  addEventListener("offline", () => sync(false));

  // A moment (the theme's, when it has them): a turn done hops the bar's sprite with the theme word as a tag, needs
  // you flashes the bar, a failed turn shakes it; 2.4 s, silent, still under reduced motion.
  function moment(next) {
    const key = { done: "done", blocked: "needs", error: "fail" }[next], bar = $(".th");
    if (!key || next === state || document.documentElement.dataset.moments !== "on") return;
    bar.dataset.mo = key;
    $(".mot", bar)?.remove();
    const tag = key === "needs" ? null : el("i", "mot");
    if (tag) { tag.dataset.w = key === "done" ? "merged" : "fail"; bar.append(tag); }
    setTimeout(() => { if (bar.dataset.mo === key) { delete bar.dataset.mo; tag?.remove(); } }, 2400);
  }

  // Within EDGE screens of an end of the window, the next slice loads, until there are EDGE screens to go.
  let loading = false;
  async function preload() {
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
  list.addEventListener("scroll", () => { preload(); markRead(); }, { passive: true });
  // The keyboard opening (or any resize) keeps the anchor: the first row in view stays put, and a reader at the
  // newest message stays there.
  let stuck = true;
  list.addEventListener("scroll", () => { stuck = atBottom(); }, { passive: true });
  new ResizeObserver(() => { if (stuck && after === 0) list.scrollTop = list.scrollHeight; }).observe(list);

  // Read marks: the newest row in view, sent at most every ~3 s.
  function markRead() {
    const bottom = list.scrollTop + list.clientHeight;
    const was = seen;
    for (const r of rows()) if (r.offsetTop < bottom) seen = Math.max(seen, Number(r.dataset.id));
    if (seen > was) cleared(id, seen);
    if (seen > read && !readTimer) {
      readTimer = setTimeout(() => {
        readTimer = 0;
        if (seen > read) { read = seen; post(`${api}/read`, { item: read }).catch(() => {}); }
      }, 3000);
    }
  }

  // The outbox: a row per send, `sending` until the server stored it, `failed` with tap-to-retry.
  function deliver(row, text, random) {
    row.className = "blk me sending";
    inOrder(() => post(`${api}/send`, { text, random_id: random })).then(
      (it) => { if (row.dataset.rid) { delete row.dataset.rid; fill(row, it); } },
      () => {
        row.className = "blk me failed";
        const retry = el("button", "retry", "failed · tap to retry");
        retry.type = "button";
        retry.onclick = () => { retry.remove(); deliver(row, text, random); };
        row.append(retry);
      },
    );
  }

  async function send(e) {
    e.preventDefault();
    const value = box.value.trim();
    if (!value || box.disabled) return;
    if (value === "/report") { box.value = ""; return reportMode(); }
    if (after > 0) await open();
    const random = rid(), row = el("li", "blk me");
    row.dataset.rid = random;
    row.append(el("div", "md", value));
    keep(() => list.insertBefore(row, live));
    list.scrollTop = list.scrollHeight;
    box.value = "";
    draft.save(true);
    deliver(row, value, random);
  }

  function cancel() { post(`${api}/cancel`, {}).then(() => toast("stopping…"), (x) => toast(x.message, true)); }

  // The draft: kept per thread in IndexedDB, written 1 s after typing stops and when the page hides; newest wins.
  const draft = (() => {
    const db = new Promise((ok, no) => {
      const r = indexedDB.open("cooee", 1);
      r.onupgradeneeded = () => r.result.createObjectStore("drafts");
      r.onsuccess = () => ok(r.result);
      r.onerror = () => no(r.error);
    });
    const tx = async (mode, f) => {
      const t = (await db).transaction("drafts", mode), store = t.objectStore("drafts");
      return new Promise((ok) => { const r = f(store); r.onsuccess = () => ok(r.result); r.onerror = () => ok(undefined); });
    };
    let timer = 0, at = 0;
    return {
      async load() {
        const d = await tx("readonly", (s) => s.get(id)).catch(() => undefined);
        if (d && !box.value) { box.value = d.text; at = d.at; }
      },
      changed() { at = Date.now(); clearTimeout(timer); timer = setTimeout(() => draft.save(), 1000); },
      async save(now) {
        clearTimeout(timer);
        if (now) at = Date.now();
        const mine = { text: box.value, at };
        const old = await tx("readonly", (s) => s.get(id)).catch(() => undefined);
        if (!old || old.at <= mine.at) await tx("readwrite", (s) => s.put(mine, id)).catch(() => {});
      },
    };
  })();

  box.addEventListener("input", () => draft.changed());
  document.addEventListener("visibilitychange", () => { if (document.visibilityState === "hidden") draft.save(); });
  $("#composer").addEventListener("submit", send);
  box.addEventListener("keydown", (e) => { if (e.key === "Enter" && (e.metaKey || e.ctrlKey)) send(e); });
  $("#pill .stop").onclick = cancel;
  $("#close").onclick = () => { if (confirm("Close this thread? Its agent stops.")) post(`${api}/close`, {}).then(() => toast("thread closed"), (x) => toast(x.message, true)); };
  // Live timers: the working pill and each running tool call.
  setInterval(() => {
    $("#el").textContent = clock(Date.now() - since);
    for (const t of list.querySelectorAll(".ts[data-since]")) t.textContent = `⋯ ${clock(Date.now() - Number(t.dataset.since))}`;
  }, 1000);
  draft.load();
  page.refresh = open;
  page.top = () => before === 0;
  open().catch((x) => toast(x.message, true)).finally(() => strip(false));
}
