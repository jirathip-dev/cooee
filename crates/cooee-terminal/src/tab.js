document.head.appendChild(document.createElement("style")).textContent = `
[hidden]{display:none!important}html,body{height:100%}body{display:flex;flex-direction:column;padding:0!important;overflow:hidden}
.top{display:flex;gap:6px;padding:6px;align-items:center}.top select{flex:1;min-width:0;min-height:40px;font:inherit;color:var(--fg);background:var(--card);border:1px solid var(--edge);border-radius:8px}
button{font:inherit;min-height:40px;min-width:40px;color:var(--fg);background:var(--card);border:1px solid var(--edge);border-radius:8px;padding:0 10px}
button[aria-pressed=true]{background:var(--link);color:var(--bg)}
.view{flex:1;overflow:auto;position:relative;background:var(--bg);touch-action:pan-x pan-y;font:var(--ts,13px)/1.25 ui-monospace,Menlo,monospace;padding:4px}
.scr{position:relative;width:max-content;min-width:100%}.scr div,.sb{white-space:pre;height:1.25em;margin:0}.sb{height:auto;color:var(--mute);font:inherit}
.cur{position:absolute;width:1ch;height:1.25em;background:var(--fg);opacity:.45;pointer-events:none}
.b{font-weight:700}.i{font-style:italic}.u{text-decoration:underline}
.note{margin:0;padding:4px 8px;color:var(--mute);font-size:.9em}
.keys{display:flex;gap:4px;padding:4px;overflow-x:auto;background:var(--card);border-top:1px solid var(--edge)}.keys button{flex:none}
.in{position:absolute;bottom:0;left:0;width:1px;height:1px;opacity:0;font-size:16px;border:0;padding:0}
.hist,.paste{display:flex;flex-direction:column;gap:4px;padding:6px;background:var(--card);border-top:1px solid var(--edge)}.hist button{text-align:left;font-family:ui-monospace,Menlo,monospace}
.paste textarea{font:inherit;min-height:60px;color:var(--fg);background:var(--bg);border:1px solid var(--edge)}
body.locked .view,body.locked .keys{filter:blur(6px);pointer-events:none}`;
const el = (tag, attrs = {}, parent = document.body) => Object.assign(parent.appendChild(document.createElement(tag)), attrs);
const top = el("div", { className: "top" });
const pick = el("select", { ariaLabel: "Terminal" }, top);
const add = el("button", { textContent: "+ shell" }, top);
const shut = el("button", { textContent: "close" }, top);
const note = el("p", { className: "note", role: "status" });
const view = el("div", { className: "view", role: "log", ariaLabel: "Screen" });
const more = el("button", { textContent: "earlier lines" }, view);
const sb = el("pre", { className: "sb" }, view);
const scr = el("div", { className: "scr" }, view);
const cur = el("div", { className: "cur" }, scr);
const hist = el("div", { className: "hist", hidden: true });
const paste = el("div", { className: "paste", hidden: true });
const pasted = el("textarea", { ariaLabel: "Text to paste" }, paste);
el("button", { textContent: "paste it" }, paste).onclick = () => {
  type(pasted.value);
  pasted.value = "";
  paste.hidden = true;
};
const bar = el("div", { className: "keys", role: "toolbar", ariaLabel: "Keys" });
const inp = el("textarea", { className: "in", ariaLabel: "Type into the shell", autocapitalize: "off", autocomplete: "off", spellcheck: false });
inp.setAttribute("autocorrect", "off");

const IDLE = 10 * 60 * 1000;
let current = null;
let stop = () => {};
let rows = [];
let size = 13;
let ctrl = false;
let line = "";
let typed = [];
let used = Date.now();
let seen = 0;
let unlocking = null;

const call = async (path, body) => {
  const init = body === undefined ? {} : { method: "POST", type: "application/json", body: JSON.stringify(body) };
  const r = await cooee.fetch(path, init);
  if (r.status === 403 && r.text === "locked") {
    await lock();
    return call(path, body);
  }
  if (!r.ok) {
    note.textContent = r.text || `error ${r.status}`;
    return null;
  }
  return JSON.parse(r.text);
};
const lock = () => {
  unlocking ??= (async () => {
    document.body.classList.add("locked");
    stop();
    note.textContent = "Locked: unlock with Face ID to go on. Your shells keep running.";
    await cooee.unlock();
    document.body.classList.remove("locked");
    note.textContent = "";
    used = Date.now();
    unlocking = null;
    if (current) watch(current);
  })();
  return unlocking;
};
const touch = () => {
  used = Date.now();
  if (used - seen > 60_000 && !unlocking) {
    seen = used;
    call("/seen", {});
  }
};
for (const e of ["pointerdown", "keydown", "wheel"]) addEventListener(e, touch, { passive: true });
setInterval(() => {
  if (Date.now() - used >= IDLE && !unlocking) lock();
}, 5_000);

const NAMED = ["var(--mute)", "var(--r)", "var(--g)", "var(--a)", "var(--link)", "var(--p)", "var(--link)", "var(--fg)"];
const level = (v) => (v ? 55 + v * 40 : 0);
const colour = (c) => {
  if (c == null) return "";
  if (Array.isArray(c)) return `rgb(${c.join(",")})`;
  if (c < 16) return NAMED[c % 8];
  if (c > 231) return `rgb(${Array(3).fill(8 + 10 * (c - 232)).join(",")})`;
  const n = c - 16;
  return `rgb(${level(Math.floor(n / 36))},${level(Math.floor(n / 6) % 6)},${level(n % 6)})`;
};
const row = (runs) => {
  const d = document.createElement("div");
  for (const r of runs) {
    const s = d.appendChild(document.createElement("span"));
    s.textContent = r.text;
    let [fg, bg] = [colour(r.fg), colour(r.bg)];
    if (r.inverse) [fg, bg] = [bg || "var(--bg)", fg || "var(--fg)"];
    if (fg) s.style.color = fg;
    if (bg) s.style.background = bg;
    s.className = [r.bold && "b", r.italic && "i", r.underline && "u"].filter(Boolean).join(" ");
  }
  return d;
};
const draw = (u) => {
  if (u.full) {
    for (const d of rows) d.remove();
    rows = Array.from({ length: u.rows }, () => scr.insertBefore(document.createElement("div"), cur));
  }
  for (const l of u.lines) {
    const old = rows[l.row];
    if (!old) continue;
    rows[l.row] = row(l.runs);
    old.replaceWith(rows[l.row]);
  }
  cur.style.left = `${u.cursor[1]}ch`;
  cur.style.top = `${u.cursor[0] * 1.25}em`;
  const atEnd = view.scrollHeight - view.scrollTop - view.clientHeight < 40;
  if (atEnd || u.full) view.scrollTop = view.scrollHeight;
};

const watch = (t) => {
  stop();
  sb.textContent = "";
  more.hidden = false;
  stop = cooee.events(`/watch?t=${t.name}`, async (e) => {
    if (e.event === "screen") draw(JSON.parse(e.data));
    else if (e.event === "closed") {
      if ((await call("/list")) !== null) load();
    }
  });
  if (t.owner) fit();
};
const open = (t) => {
  current = t;
  batch = null;
  hist.hidden = paste.hidden = true;
  bar.hidden = !t?.owner;
  shut.disabled = !t?.owner;
  note.textContent = t && !t.owner ? `${t.name} is an agent's terminal: read-only.` : "";
  if (t) watch(t);
};
const load = async (want) => {
  const list = await call("/list");
  if (!list) return;
  pick.replaceChildren();
  for (const [label, owner] of [["Your shells", true], ["Agents' terminals (read-only)", false]]) {
    const g = el("optgroup", { label }, pick);
    for (const t of list.filter((t) => t.owner === owner)) el("option", { value: t.name, textContent: t.cmd ? `${t.name} · ${t.cmd}` : t.name }, g);
    if (!g.children.length) g.remove();
  }
  const t = list.find((t) => t.name === (want ?? current?.name)) ?? list[0] ?? null;
  if (t) pick.value = t.name;
  if (!t) note.textContent = "No shells yet: + shell starts one.";
  if (t?.name !== current?.name || !want) open(t);
  pick.onchange = () => open(list.find((t) => t.name === pick.value));
};
pick.onfocus = () => load(current?.name);
add.onclick = async () => {
  const r = await call("/shell", {});
  if (r) load(r.terminal);
};
shut.onclick = async () => {
  if (current?.owner && (await call("/close", { terminal: current.name }))) {
    current = null;
    load();
  }
};
more.onclick = async () => {
  const r = current && (await call("/scrollback", { terminal: current.name }));
  if (!r) return;
  const lines = r.text.split("\n");
  sb.textContent = lines.slice(0, Math.max(0, lines.length - rows.length)).join("\n");
  more.hidden = true;
};

let queue = Promise.resolve();
let batch = null;
const later = (f) => {
  queue = queue.then(f, f);
};
const send = (text) => {
  if (!current?.owner) return;
  if (!batch) {
    const b = (batch = { name: current.name, text: "" });
    later(() => {
      if (batch === b) batch = null;
      return call("/input", { terminal: b.name, text: b.text });
    });
  }
  batch.text += text;
};
const keys = (names) => {
  if (!current?.owner) return;
  const name = current.name;
  batch = null;
  later(() => call("/keys", { terminal: name, keys: names }));
};
const type = (text) => {
  if (!text) return;
  if (ctrl && /^[a-z]$/i.test(text)) {
    setCtrl(false);
    return keys([`ctrl+${text.toLowerCase()}`]);
  }
  line += text;
  send(text);
};
const enter = () => {
  if (line.trim()) typed = [line, ...typed.filter((l) => l !== line)].slice(0, 20);
  line = "";
  keys(["enter"]);
};
inp.addEventListener("beforeinput", (e) => {
  if (e.isComposing) return;
  e.preventDefault();
  const t = e.inputType;
  if (t === "insertText" || t === "insertReplacementText" || t === "insertFromPaste")
    type(e.data ?? e.dataTransfer?.getData("text/plain") ?? "");
  else if (t === "deleteContentBackward") {
    line = line.slice(0, -1);
    keys(["backspace"]);
  } else if (t === "insertLineBreak" || t === "insertParagraph") enter();
});
inp.addEventListener("compositionend", (e) => {
  type(e.data);
  inp.value = "";
});
view.addEventListener("click", () => current?.owner && inp.focus({ preventScroll: true }));

const ctrlKey = el("button", { textContent: "Ctrl", ariaPressed: "false" }, bar);
const setCtrl = (on) => {
  ctrl = on;
  ctrlKey.ariaPressed = String(on);
};
ctrlKey.onclick = () => setCtrl(!ctrl);
for (const [text, name, label] of [
  ["Esc", "esc"],
  ["Tab", "tab"],
  ["↑", "up", "up"],
  ["↓", "down", "down"],
  ["←", "left", "left"],
  ["→", "right", "right"],
  ["^C", "ctrl+c", "control C"],
]) {
  const b = el("button", { textContent: text }, bar);
  if (label) b.ariaLabel = label;
  b.onclick = () => {
    if (name === "ctrl+c") line = "";
    keys([name]);
    inp.focus({ preventScroll: true });
  };
}
bar.insertBefore(ctrlKey, bar.children[2]);
el("button", { textContent: "paste" }, bar).onclick = async () => {
  const text = await navigator.clipboard?.readText().catch(() => null);
  if (text) type(text);
  else {
    paste.hidden = false;
    pasted.focus();
  }
};
el("button", { textContent: "↑ history", ariaLabel: "history" }, bar).onclick = () => {
  hist.replaceChildren();
  for (const l of typed) el("button", { textContent: l }, hist).onclick = () => (type(l), (hist.hidden = true));
  if (!typed.length) el("p", { className: "note", textContent: "Nothing typed here yet." }, hist);
  hist.hidden = !hist.hidden;
};
document.body.append(hist, paste, bar);

const probe = el("span", { textContent: "M".repeat(10) }, view);
probe.style.cssText = "position:absolute;visibility:hidden";
let fitting = 0;
const fit = () => {
  clearTimeout(fitting);
  fitting = setTimeout(() => {
    if (!current?.owner) return;
    const cols = Math.floor((view.clientWidth - 8) / (probe.offsetWidth / 10));
    const lines = Math.floor((view.clientHeight - 8) / (size * 1.25));
    if (cols > 0 && lines > 0) call("/resize", { terminal: current.name, cols: Math.min(cols, 500), rows: Math.min(lines, 500) });
  }, 300);
};
new ResizeObserver(fit).observe(view);
const dist = (e) => Math.hypot(e.touches[0].clientX - e.touches[1].clientX, e.touches[0].clientY - e.touches[1].clientY);
let pinch = null;
view.addEventListener("touchstart", (e) => e.touches.length === 2 && (pinch = { d: dist(e), size }), { passive: true });
view.addEventListener(
  "touchmove",
  (e) => {
    if (!pinch || e.touches.length !== 2) return;
    e.preventDefault();
    size = Math.min(28, Math.max(8, (pinch.size * dist(e)) / pinch.d));
    document.documentElement.style.setProperty("--ts", `${size}px`);
  },
  { passive: false },
);
view.addEventListener("touchend", () => {
  if (pinch) fit();
  pinch = null;
});

await load(location.hash.slice(1) || undefined);
