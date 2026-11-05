// cooee's pages. The thread list: the new-thread form. The chat: a window of about 9 screens of items around an
// anchor (slices by id as the reader nears an edge), scroll kept on a message rather than pixels, live events over
// SSE resumed from the last one seen, the outbox (sending → sent → failed, tap to retry), the per-thread draft in
// IndexedDB, approvals that lock once tapped, and debounced read marks (which also close the thread's notifications).
// Every page registers the service worker (sw.js, push only); the thread list's 🔔 turns alerts on.
// States are theme keys (data-st) that look.js draws: the thread bar, the working pill (its sprite following the
// running tool), the live line and a turn's end.
const $ = (s, el = document) => el.querySelector(s);
const rid = () => Date.now().toString(36) + Math.random().toString(36).slice(2, 10);
const post = (url, body) =>
  fetch(url, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body) })
    .then(async (r) => (r.ok ? r.json() : Promise.reject(new Error(await r.text()))));

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

if ("serviceWorker" in navigator) navigator.serviceWorker.register("/sw.js").catch(() => {});
// Closes thread `thread`'s notifications up to item `read`.
const cleared = (thread, read) => navigator.serviceWorker?.ready.then((r) => r.active?.postMessage({ thread, read }));

// Alerts: the 🔔 shows where push works and isn't on yet; a tap asks for permission and subscribes.
const bell = $("#alerts");
if (bell && "PushManager" in window) (async () => {
  const reg = await navigator.serviceWorker.ready;
  if (Notification.permission === "granted" && (await reg.pushManager.getSubscription())) return;
  bell.hidden = false;
  bell.onclick = async () => {
    try {
      if ((await Notification.requestPermission()) !== "granted") throw new Error("notifications are off for this app");
      const r = await fetch("/push/key");
      if (!r.ok) throw new Error(await r.text());
      const key = (await r.json()).key.replace(/-/g, "+").replace(/_/g, "/");
      const sub = await reg.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: Uint8Array.from(atob(key), (c) => c.charCodeAt(0)) });
      const { endpoint, keys } = sub.toJSON();
      const ok = await fetch("/push/subscribe", { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ endpoint, ...keys }) });
      if (!ok.ok) throw new Error(await ok.text());
      bell.hidden = true;
    } catch (x) { alert(`alerts failed: ${x.message}`); }
  };
})();

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
  const el = (tag, cls, text) => { const e = document.createElement(tag); if (cls) e.className = cls; if (text !== undefined) e.textContent = text; return e; };

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
      row.append(el("b", "k", agent), el("div", "md", it.tool));
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
    try { await post(`${api}/approve`, { item, option }); } catch {
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
    const s = await fetch(`${api}/messages?${q}`).then((r) => r.json());
    if (where === "replace") {
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
    es.addEventListener("item", (e) => put(JSON.parse(e.data)));
    es.addEventListener("text", (e) => text(JSON.parse(e.data).text));
    es.addEventListener("state", (e) => setState(JSON.parse(e.data)));
    // Another run or a page too far behind: load the newest slice again.
    es.addEventListener("reset", () => { es.close(); open(); });
    es.onerror = () => { if (es.readyState === EventSource.CLOSED) setTimeout(open, 2000); };
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

  function cancel() { post(`${api}/cancel`, {}).catch(() => {}); }

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
  $("#close").onclick = () => { if (confirm("Close this thread? Its agent stops.")) post(`${api}/close`, {}).catch(() => {}); };
  // Live timers: the working pill and each running tool call.
  setInterval(() => {
    $("#el").textContent = clock(Date.now() - since);
    for (const t of list.querySelectorAll(".ts[data-since]")) t.textContent = `⋯ ${clock(Date.now() - Number(t.dataset.since))}`;
  }, 1000);
  draft.load();
  open();
}
