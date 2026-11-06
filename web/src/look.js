// cooee's look (docs/design/look), a classic script in <head>, so a device's pick shows before the first paint. The
// server puts `LOOK` (every theme and scheme, and its default theme) before this file. The pick is this device's
// alone, in localStorage: theme, mode (system | light | dark), style (modern | terminal), sprites (on | ascii | off), and a
// scheme per mode (unset: the theme's default). System follows prefers-color-scheme live. Elements are filled as
// they are parsed or added, so pages and chat.js only name what they show:
//   [data-st=working|needs|done|fail|idle|ended]  a state: the sprite (data-tool=edit|read|run while working), the
//                                                 glyph (a theme without sprites) or the dot (sprites off), then the
//                                                 theme word (data-plain adds `· plain word`)
//   [data-w=<key>]                                a theme word (data-plain adds `· plain word`)
//   [data-s=loading|empty]                        the theme's 132 × 100 figure (sprites off: none)
//   [data-open=look|settings]                     opens the look sheet or Settings (#settings/<section> opens it there)
// Ascii draws a state as a 3-character figure and the big figures as text. A theme with moments sets
// html[data-moments=on] (chat.js plays them). Settings fires `cooee-sheet` once drawn, for chat.js's sections.
// A plugin tab (iframe[data-plugin]) gets the look as a message (`cooee-look`: CSS variables, style, sprites, words
// and the theme's data URLs) when it asks (`cooee-look?`) and again on every change.
(() => {
  const KEY = "cooee.look", html = document.documentElement, mq = matchMedia("(prefers-color-scheme: dark)");
  const PLAIN = { working: "working", needs: "needs you", done: "done", fail: "failed", idle: "idle", ended: "ended",
    merged: "done", loading: "loading", "empty-threads": "no threads" };
  const GLYPH = { working: "✻", needs: "!", done: "✓", fail: "✕", idle: "·", ended: "■" };
  const ASCII = { edit: "o_/", read: "o.o", run: "o>_", needs: "(!)", done: "\\o/", fail: "x_x", idle: "-.-", ended: "[ ]", merged: "\\o/",
    loading: "( o.o )\n  ...", empty: "( -.- )\n  zzz" };
  const THEMES = ["farm", "kitchen", "garden", "terminal"];
  const SCHEMES = ["github-dark", "solarized-dark", "gruvbox", "nord", "dracula", "farm", "kitchen", "garden-dark", "paper",
    "farm-light", "kitchen-light", "garden-light"];
  const SEL = "[data-st],[data-w],[data-s]";
  const esc = (s) => String(s).replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);
  // The built-in ones in the design's order, then the owner's own by name.
  const sorted = (o, order) => Object.keys(o).sort((a, b) => ((order.indexOf(a) + 1 || 99) - (order.indexOf(b) + 1 || 99)) || a.localeCompare(b));
  let pick = {};
  try { pick = JSON.parse(localStorage.getItem(KEY)) || {}; } catch { /* no storage: the defaults */ }

  const name = () => (LOOK.themes[pick.theme] ? pick.theme : LOOK.default);
  const theme = () => LOOK.themes[name()];
  const mode = () => (pick.mode === "light" || pick.mode === "dark" ? pick.mode : mq.matches ? "dark" : "light");
  const scheme = (m) => (LOOK.schemes[pick[m]]?.["color-scheme"] === m ? pick[m] : theme().scheme[m]);
  const style = () => (pick.style === "terminal" ? "terminal" : "modern");
  const figs = () => (pick.sprites === "off" || pick.sprites === "ascii" ? pick.sprites : "on");
  const sprites = () => figs() !== "off";
  const art = () => theme().sprites && figs() === "on";
  const word = (k) => theme().words[k] || PLAIN[k] || k;
  const url = (file) => new URL(`/look/${name()}/${file}`, location).href;

  function svg(id, cls) {
    const box = cls === "big" ? "0 0 132 100" : "0 0 24 24";
    return `<svg class="${cls}" viewBox="${box}" aria-hidden="true"><use href="/look/${name()}/sprites.svg#s-${id}"/></svg>`;
  }

  function fill(e) {
    const st = e.dataset.st;
    if (st !== undefined) {
      const id = st === "working" ? e.dataset.tool || "edit" : st;
      const mark = !sprites() ? '<i class="sd"></i>' : figs() === "ascii" ? `<i class="rsa">${esc(ASCII[id] ?? "")}</i> `
        : art() ? svg(id, "rsp") : `<i class="sg">${GLYPH[st] ?? ""}</i> `;
      const w = word(st), plain = "plain" in e.dataset && w !== PLAIN[st] ? `<small class="pw"> · ${PLAIN[st]}</small>` : "";
      e.innerHTML = PLAIN[st] ? `${mark}<span class="thw">${esc(w)}</span>${plain}` : "";
    } else if (e.dataset.w) {
      const k = e.dataset.w, w = word(k);
      e.innerHTML = esc(w) + ("plain" in e.dataset && w !== PLAIN[k] ? `<small class="pw"> · ${PLAIN[k]}</small>` : "");
    } else if (e.dataset.s) e.innerHTML = figs() === "ascii" ? `<i class="lsa">${esc(ASCII[e.dataset.s] ?? "")}</i>` : art() ? svg(e.dataset.s, "big") : "";
  }
  const fillAll = (root = document) => root.querySelectorAll(SEL).forEach(fill);
  new MutationObserver((ms) => {
    for (const m of ms) {
      if (m.type === "attributes") fill(m.target);
      else for (const n of m.addedNodes) if (n.nodeType === 1) { if (n.matches(SEL)) fill(n); fillAll(n); }
    }
  }).observe(html, { subtree: true, childList: true, attributes: true, attributeFilter: ["data-st", "data-tool"] });

  // The look as a plugin tab gets it.
  function forPlugin() {
    const t = theme();
    return { type: "cooee-look", theme: name(), mode: mode(), style: style(), sprites: figs(),
      vars: { ...LOOK.schemes[scheme(mode())].tokens }, words: { ...PLAIN, ...t.words },
      urls: { theme: url("theme.json"), sprites: t.sprites ? url("sprites.svg") : null, icon: url("icon.png") } };
  }
  const tell = (w) => w?.postMessage(forPlugin(), "*");
  addEventListener("message", (e) => { if (e.data?.type === "cooee-look?") tell(e.source); });

  function paint() {
    const s = LOOK.schemes[scheme(mode())];
    for (const [k, v] of Object.entries(s.tokens)) html.style.setProperty(k, v);
    html.style.colorScheme = s["color-scheme"];
    Object.assign(html.dataset, { theme: name(), mode: mode(), style: style(), sprites: figs(), moments: theme().moments ? "on" : "off" });
    document.querySelector("meta[name=theme-color]")?.setAttribute("content", s.tokens["--bg"]);
    for (const l of document.querySelectorAll("link[rel=icon],link[rel=apple-touch-icon]")) { l.href = url("icon.png"); l.type = ""; }
    fillAll();
    for (const f of document.querySelectorAll("iframe[data-plugin]")) tell(f.contentWindow);
    if (open) render();
  }

  // A change keeps the reader where they were: the row at the top of the view stays there, cut at the same share
  // of its height, however much the new style grows or shrinks it.
  function hold(change) {
    const box = document.getElementById("msgs"), rows = box ? box.children : document.querySelectorAll(".fh,.tr");
    const top = box ? box.getBoundingClientRect().top : 0;
    const anchor = [...rows].find((r) => r.getBoundingClientRect().bottom > top);
    const at = (r = anchor.getBoundingClientRect()) => (top - r.top) / (r.height || 1);
    const share = anchor && at();
    change();
    if (!anchor) return;
    const r = anchor.getBoundingClientRect(), by = r.top + share * r.height - (box ? box.getBoundingClientRect().top : 0);
    if (box) box.scrollTop += by; else scrollBy(0, by);
  }

  function set(k, v) {
    if (k === "dark" || k === "light") {
      if (v === theme().scheme[k]) delete pick[k]; else pick[k] = v;
      // A tap on the other mode's list shows it: Appearance switches to that mode.
      if (k !== mode()) pick.mode = k;
    } else pick[k] = v;
    try { localStorage.setItem(KEY, JSON.stringify(pick)); } catch { /* this page only */ }
    hold(paint);
  }

  // The sheets: the look sheet (STYLE, SPRITES, THEME, APPEARANCE, COLOURS) and Settings.
  let open = "", sheet = null;
  const tick = (on) => (on ? '<i class="tk">✓</i>' : "");
  const keys = (k, list, cur) => `<div class="seg" role="radiogroup" aria-label="${k}">${list.map((v) =>
    `<button type="button" role="radio" data-k="${k}" data-v="${v}" aria-checked="${v === cur}">${v}</button>`).join("")}</div>`;
  const swatch = (s) => `<span class="swa" style="--sbg:${esc(s.tokens["--bg"])};--sfg:${esc(s.tokens["--fg"])};--sl:${esc(s.tokens["--l"])}"><b>Aa</b>${
    ["--g", "--a", "--r"].map((k) => `<i style="background:${esc(s.tokens[k])}"></i>`).join("")}</span>`;
  const label = (n) => esc(LOOK.schemes[n].label.toLowerCase());

  function lookSheet() {
    const tiles = sorted(LOOK.themes, THEMES).map((n) => `<button type="button" role="option" class="pick" data-k="theme" data-v="${esc(n)}"
      aria-selected="${n === name()}"><img alt="" src="/look/${esc(n)}/icon.png"><span>${esc(LOOK.themes[n].label)}</span>${tick(n === name())}</button>`);
    const lists = ["dark", "light"].map((m) => {
      const rows = sorted(LOOK.schemes, SCHEMES).filter((n) => LOOK.schemes[n]["color-scheme"] === m).map((n) =>
        `<button type="button" role="option" class="pick" data-k="${m}" data-v="${esc(n)}" aria-selected="${n === scheme(m)}">${swatch(LOOK.schemes[n])}
        <span class="nm">${label(n)}</span>${n === theme().scheme[m] ? "<small>default</small>" : ""}${tick(n === scheme(m))}</button>`);
      return `<p class="ch">${m} colours${m === mode() ? '<span class="shw"> · showing</span>' : ""}</p>
        <div class="lsl" role="listbox" aria-label="${m} colours">${rows.join("")}</div>`;
    });
    return `<div class="lrw"><span>style</span>${keys("style", ["modern", "terminal"], style())}</div>
      <div class="lrw"><span>sprites</span>${keys("sprites", ["on", "ascii", "off"], figs())}</div>
      <p class="ch">theme</p><div class="lth" role="listbox" aria-label="theme">${tiles.join("")}</div>
      <p class="ch">appearance</p>${keys("mode", ["system", "light", "dark"], pick.mode || "system")}
      ${lists.join("")}<p class="lft">this device only · iOS keeps the icon it installed with</p>`;
  }

  const SECTIONS = ["sign-ins", "look", "alerts", "devices", "report"];
  function settings() {
    const row = (k, v) => `<button type="button" class="sr" data-open="look"><span class="sk">${k}</span><span class="sv">${v}</span><i class="cv">▸</i></button>`;
    const m = pick.mode || "system";
    const index = SECTIONS.map((s) => `<a href="#settings/${s}">${s}</a>`).join("");
    return `<nav class="six" aria-label="sections">${index}</nav>
      <p class="ch" id="s-sign-ins">sign-ins</p><a class="sr" href="/#signins" data-close><span class="sk">providers</span><i class="cv">▸</i></a>
      <p class="ch" id="s-look">look</p>${row("style", style())}${row("sprites", figs())}${row("theme", esc(theme().label))}
      ${row("appearance", m === "system" ? `system · ${mode()} now` : m)}${row("colours", `dark ${label(scheme("dark"))} · light ${label(scheme("light"))}`)}
      <p class="ch" id="s-alerts">alerts</p><div id="s-alerts-box"></div>
      <p class="ch" id="s-devices">devices</p><div id="s-devices-box"></div>
      <p class="ch" id="s-report">report</p><button type="button" class="sr" data-report><span class="sk">report a problem</span><i class="cv">▸</i></button>`;
  }

  function render() {
    const top = sheet.querySelector(".top"), at = top?.scrollTop ?? 0;
    sheet.className = open === "look" ? "lks" : "set";
    sheet.innerHTML = `<div class="bd"></div><div class="sh" role="dialog" aria-modal="${open !== "look"}" aria-labelledby="sht">
      <div class="hd"><b id="sht">${open}</b><button type="button" class="gl" data-close aria-label="close">×</button></div>
      <div class="top">${open === "look" ? lookSheet() : settings()}</div></div>`;
    sheet.querySelector(".top").scrollTop = at;
    for (const b of document.querySelectorAll("header [data-open]")) b.setAttribute("aria-pressed", b.dataset.open === open);
    dispatchEvent(new CustomEvent("cooee-sheet", { detail: open }));
  }

  function show(which) {
    if (!sheet) { sheet = document.createElement("div"); sheet.id = "sheet"; document.body.append(sheet); }
    open = which;
    sheet.hidden = !which;
    if (which) render();
    else for (const b of document.querySelectorAll("header [data-open]")) b.setAttribute("aria-pressed", "false");
    if (!which && location.hash.startsWith("#settings")) history.replaceState(null, "", location.pathname + location.search);
  }
  globalThis.cooeeSheet = show;
  // A deep link: #settings, or #settings/<section> scrolled to that section.
  function deep() {
    const m = location.hash.match(/^#settings(?:\/([\w-]+))?$/);
    if (!m) return;
    if (open !== "settings") show("settings");
    if (m[1]) sheet.querySelector(`#s-${m[1]}`)?.scrollIntoView({ block: "start" });
  }
  addEventListener("hashchange", deep);
  addEventListener("DOMContentLoaded", deep);

  addEventListener("click", (e) => {
    const t = e.target.closest?.("[data-open],[data-close],[data-k],.bd");
    if (!t) return;
    if (t.dataset.open) show(open === t.dataset.open ? "" : t.dataset.open);
    else if (t.dataset.k) set(t.dataset.k, t.dataset.v === "system" ? undefined : t.dataset.v);
    else show("");
  });
  addEventListener("keydown", (e) => { if (e.key === "Escape" && open) show(""); });
  // System follows the phone live; another tab's pick shows here too.
  mq.addEventListener("change", () => hold(paint));
  addEventListener("storage", (e) => {
    if (e.key !== KEY) return;
    try { pick = JSON.parse(e.newValue) || {}; } catch { pick = {}; }
    hold(paint);
  });
  paint();
})();
