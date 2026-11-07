// cooee's look (docs/design/look) once the page is up. The server draws the device's pick from its `cooee-look`
// cookie on the first byte (theme, mode system | light | dark, style modern | terminal | scene, figures sprite | ascii |
// none, and a scheme per mode; unset: the theme's default; #35), with every theme and scheme in `<script id="looks">`. This module
// keeps it live: a pick in the look sheet applies at once without a reload (the cookie keeps it), System follows
// prefers-color-scheme, and elements added later are filled as they come, so chat.ts only names what it shows:
//   [data-st=working|needs|done|fail|idle|ended]  a state: the sprite (data-tool=edit|read|run while working) or the
//                                                 ascii figure, then the theme word (data-plain adds `· plain word`);
//                                                 figures none: a dot and the plain word
//   [data-w=<key>]                                a theme word (data-plain adds `· plain word`)
//   [data-s=loading|empty]                        the theme's 132 × 100 figure (figures none: none)
//   [data-open=look|settings|folders]             opens that sheet (#settings/<section> opens Settings there)
// The sheets are server-rendered popovers in #sheet (they open with no script); this keeps their marks in step, fills
// Settings › Folders from /folders, and fires `cooee-sheet` when one opens, for chat.ts's sections. A plugin tab
// (iframe[data-plugin]) gets the look as a message (`cooee-look`) when it asks (`cooee-look?`) and on every change.

export interface Scheme {
  label: string;
  "color-scheme": string;
  tokens: Record<string, string>;
}
export interface Theme {
  label: string;
  scheme: { dark: string; light: string };
  sprites: boolean;
  moments: boolean;
  words: Record<string, string>;
}
interface Looks {
  default: string;
  themes: Record<string, Theme>;
  schemes: Record<string, Scheme>;
  ascii: [string, [string, string]][];
}
type Key = "theme" | "mode" | "style" | "scene" | "figures" | "dark" | "light";
type Pick = Partial<Record<Key, string>>;
type Mode = "dark" | "light";

export const $ = <E extends Element = HTMLElement>(s: string, el: ParentNode = document): E | null =>
  el.querySelector<E>(s);
export const make = (tag: string, cls?: string, text?: string): HTMLElement => {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text !== undefined) e.textContent = text;
  return e;
};
export const esc = (s: unknown): string => String(s).replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);

const COOKIE = "cooee-look";
const html = document.documentElement;
const mq = matchMedia("(prefers-color-scheme: dark)");
const LOOK: Looks = JSON.parse($("#looks")?.textContent ?? "null");
const PLAIN: Record<string, string> = {
  working: "working",
  needs: "needs you",
  done: "done",
  fail: "failed",
  idle: "idle",
  ended: "ended",
  merged: "done",
  loading: "loading",
  "empty-threads": "no threads",
};
const ASCII = Object.fromEntries(LOOK.ascii);
const SEL = "[data-st],[data-w],[data-s]";

// The pick, as the cookie keeps it: `theme:kitchen/style:terminal`.
const read = (): Pick => {
  const raw = document.cookie.split("; ").find((c) => c.startsWith(`${COOKIE}=`)) ?? "";
  const pick: Pick = {};
  for (const kv of raw.slice(COOKIE.length + 1).split("/")) {
    const [k, v] = kv.split(":");
    // The Sprites switch and the terminal theme before #35.
    if (k === "sprites" && v) pick.figures = v === "on" ? "sprite" : v === "off" ? "none" : v;
    else if (k === "theme" && v === "terminal" && !LOOK.themes.terminal) pick.theme = "plain";
    else if (k && v) pick[k as Key] = v;
  }
  // A theme being built shows in its preview's frames (`?preview=<name>`).
  const preview = new URLSearchParams(location.search).get("preview");
  if (preview) pick.theme = `dev.${preview}`;
  return pick;
};
let pick = read();

const name = (): string => (pick.theme && LOOK.themes[pick.theme] ? pick.theme : LOOK.default);
const theme = (): Theme => LOOK.themes[name()] as Theme;
const mode = (): Mode => (pick.mode === "light" || pick.mode === "dark" ? pick.mode : mq.matches ? "dark" : "light");
const scheme = (m: Mode): string => {
  const p = pick[m];
  return p && LOOK.schemes[p]?.["color-scheme"] === m ? p : theme().scheme[m];
};
// Figures sprite | ascii | none (a theme without sprites: ascii | none); Scene only with sprites, else the Modern or
// Terminal pick under it.
const figs = (): string =>
  pick.figures === "ascii" || pick.figures === "none" ? pick.figures : theme().sprites ? "sprite" : "ascii";
const style = (): string =>
  pick.scene === "on" && figs() === "sprite" ? "scene" : pick.style === "terminal" ? "terminal" : "modern";
const word = (k: string): string => theme().words[k] || PLAIN[k] || k;
const url = (file: string): string => new URL(`/look/${name()}/${file}`, location.href).href;

const svg = (id: string, cls: string): string => {
  const box = cls === "big" ? "0 0 132 100" : "0 0 24 24";
  return `<svg class="${cls}" viewBox="${box}" aria-hidden="true"><use href="/look/${name()}/sprites.svg#s-${id}"/></svg>`;
};

const ascii = (id: string, cls: string): string =>
  `<i class="${cls}" aria-hidden="true">${(ASCII[id] ?? ["", ""]).map((f) => `<b>${esc(f)}</b>`).join("")}</i>`;

function fill(e: HTMLElement): void {
  const { st, w, s } = e.dataset;
  const plainOf = (k: string, said: string): string =>
    "plain" in e.dataset && said !== PLAIN[k] ? `<small class="pw"> · ${PLAIN[k]}</small>` : "";
  if (st !== undefined) {
    const id = st === "working" ? e.dataset.tool || "edit" : st;
    const mark =
      figs() === "none" ? '<i class="sd"></i>' : figs() === "ascii" ? `${ascii(id, "rsa")} ` : svg(id, "rsp");
    const said = figs() === "none" ? (PLAIN[st] ?? "") : word(st);
    e.innerHTML = PLAIN[st] ? `${mark}<span class="thw">${esc(said)}</span>${plainOf(st, said)}` : "";
  } else if (w) {
    e.innerHTML = esc(word(w)) + plainOf(w, word(w));
  } else if (s) {
    e.innerHTML = figs() === "ascii" ? ascii(s, "lsa") : figs() === "sprite" ? svg(s, "big") : "";
  }
}
const fillAll = (root: ParentNode = document): void => {
  for (const e of root.querySelectorAll<HTMLElement>(SEL)) fill(e);
};
// The server filled what it rendered; this fills what comes after, and a state that changes.
new MutationObserver((ms) => {
  for (const m of ms) {
    if (m.type === "attributes") fill(m.target as HTMLElement);
    else
      for (const n of m.addedNodes) {
        if (!(n instanceof HTMLElement)) continue;
        if (n.matches(SEL)) fill(n);
        fillAll(n);
      }
  }
}).observe(html, { subtree: true, childList: true, attributes: true, attributeFilter: ["data-st", "data-tool"] });

// The look as a plugin tab gets it.
function forPlugin(): object {
  const t = theme();
  return {
    type: "cooee-look",
    theme: name(),
    mode: mode(),
    style: style(),
    figures: figs(),
    vars: { ...LOOK.schemes[scheme(mode())]?.tokens },
    words: { ...PLAIN, ...t.words },
    urls: { theme: url("theme.json"), sprites: t.sprites ? url("sprites.svg") : null, icon: url("icon.png") },
  };
}
const tell = (w: MessageEventSource | Window | null | undefined): void => {
  if (w && "postMessage" in w) (w as Window).postMessage(forPlugin(), "*");
};
addEventListener("message", (e) => {
  if (e.data?.type === "cooee-look?") tell(e.source);
});

function paint(): void {
  const s = LOOK.schemes[scheme(mode())];
  if (!s) return;
  for (const [k, v] of Object.entries(s.tokens)) html.style.setProperty(k, v);
  html.style.colorScheme = s["color-scheme"];
  Object.assign(html.dataset, {
    theme: name(),
    mode: mode(),
    style: style(),
    figures: figs(),
    moments: theme().moments ? "on" : "off",
  });
  $("meta[name=theme-color]")?.setAttribute("content", s.tokens["--bg"] ?? "");
  for (const l of document.querySelectorAll<HTMLLinkElement>("link[rel=icon],link[rel=apple-touch-icon]")) {
    l.href = url("icon.png");
    l.type = "";
  }
  fillAll();
  for (const f of document.querySelectorAll<HTMLIFrameElement>("iframe[data-plugin]")) tell(f.contentWindow);
  marks();
}

// A change keeps the reader where they were: the row at the top of the view stays there, cut at the same share
// of its height, however much the new style grows or shrinks it.
function hold(change: () => void): void {
  const box = document.getElementById("msgs");
  const rows = box ? box.children : document.querySelectorAll(".fh,.tr");
  const top = box ? box.getBoundingClientRect().top : 0;
  const anchor = [...rows].find((r) => r.getBoundingClientRect().bottom > top);
  const at = (r: DOMRect): number => (top - r.top) / (r.height || 1);
  const share = anchor ? at(anchor.getBoundingClientRect()) : 0;
  change();
  if (!anchor) return;
  const r = anchor.getBoundingClientRect();
  const by = r.top + share * r.height - (box ? box.getBoundingClientRect().top : 0);
  if (box) box.scrollTop += by;
  else scrollBy(0, by);
}

function set(k: Key, v: string | undefined): void {
  if (k === "dark" || k === "light") {
    if (v === theme().scheme[k]) delete pick[k];
    else if (v) pick[k] = v;
    // A tap on the other mode's list shows it: Appearance switches to that mode.
    if (k !== mode()) pick.mode = k;
  } else if (k === "style" && v === "scene") pick.scene = "on";
  else if (v === undefined) delete pick[k];
  else {
    if (k === "style") delete pick.scene;
    pick[k] = v;
  }
  const value = Object.entries(pick)
    .map(([k, v]) => `${k}:${v}`)
    .join("/");
  // biome-ignore lint/suspicious/noDocumentCookie: the Cookie Store API is newer than the iPhones cooee serves.
  document.cookie = `${COOKIE}=${value}; path=/; max-age=315360000; samesite=lax`;
  hold(paint);
}

// The sheets' marks: each key's current value checked, the theme's default schemes, the mode showing, and Settings'
// values. The server rendered them for the pick it saw; a pick here changes them in place.
const label = (n: string): string => LOOK.schemes[n]?.label.toLowerCase() ?? n;
function marks(): void {
  const now: Record<Key, string> = {
    theme: name(),
    mode: pick.mode || "system",
    style: style(),
    scene: "",
    figures: figs(),
    dark: scheme("dark"),
    light: scheme("light"),
  };
  for (const b of document.querySelectorAll<HTMLElement>("#sheet [data-k][data-v]")) {
    const k = b.dataset.k as Key;
    const on = now[k] === b.dataset.v;
    if (b.getAttribute("role") === "radio") {
      b.setAttribute("aria-checked", String(on));
      // Not offered, so not listed: Scene without sprites, Sprite on a theme without them.
      b.hidden = (b.dataset.v === "scene" && figs() !== "sprite") || (b.dataset.v === "sprite" && !theme().sprites);
      continue;
    }
    b.setAttribute("aria-selected", String(on));
    $(".tk", b)?.remove();
    if (k === "dark" || k === "light") {
      $("small", b)?.remove();
      if (b.dataset.v === theme().scheme[k]) $(".nm", b)?.after(make("small", "", "default"));
    }
    if (on) b.append(make("i", "tk", "✓"));
  }
  for (const p of document.querySelectorAll<HTMLElement>("#sheet .ch[data-m]")) {
    $(".shw", p)?.remove();
    if (p.dataset.m === mode()) p.append(make("span", "shw", " · showing"));
  }
  const values: Record<string, string> = {
    style: now.style,
    figures: now.figures,
    theme: theme().label,
    appearance: now.mode === "system" ? `system · ${mode()} now` : now.mode,
    colours: `dark ${label(now.dark)} · light ${label(now.light)}`,
  };
  for (const v of document.querySelectorAll<HTMLElement>("#sheet [data-sv]"))
    v.textContent = values[v.dataset.sv ?? ""] ?? "";
}

// Settings › Folders: the roots, each with [remove], and the Mac's folders from the home folder down (names only,
// hidden ones behind a toggle), the one shown with [add as root]. Closing it after a change reloads the page.
interface Listing {
  path: string;
  home: string;
  dirs: string[];
  roots: string[];
}
const fol: { path: string; hidden: boolean; at: Listing | null; err: string; changed: boolean } = {
  path: "",
  hidden: false,
  at: null,
  err: "",
  changed: false,
};
const tilde = (p: string): string => (fol.at && p.startsWith(fol.at.home) ? `~${p.slice(fol.at.home.length)}` : p);
async function folders(change?: { path: string; remove?: boolean }): Promise<void> {
  try {
    const ok = async <T>(r: Response): Promise<T> => {
      if (!r.ok) throw new Error(await r.text());
      return r.json();
    };
    if (change) {
      const body = JSON.stringify(change);
      await ok(await fetch("/roots", { method: "POST", headers: { "content-type": "application/json" }, body }));
      fol.changed = true;
    }
    const q = [fol.path && `path=${encodeURIComponent(fol.path)}`, fol.hidden && "hidden=1"].filter(Boolean).join("&");
    fol.at = await ok<Listing>(await fetch(`/folders?${q}`));
    fol.path = fol.at.path;
    fol.err = "";
  } catch (x) {
    fol.err = (x as Error).message;
  }
  drawFolders();
}
function drawFolders(): void {
  const top = $("#sh-folders .top");
  if (!top) return;
  const a = fol.at;
  const err = `<p class="err"${fol.err ? "" : " hidden"}>${esc(fol.err)}</p>`;
  if (!a) {
    top.innerHTML = err;
    return;
  }
  const go = (p: string, k: string, more = ""): string =>
    `<button type="button" class="sr" data-go="${esc(p)}"><span class="sk">${esc(k)}</span>${more}</button>`;
  const roots = a.roots.map(
    (r) =>
      `<div class="lrw"><span class="sk">${esc(tilde(r))}</span><button type="button" data-rm="${esc(r)}">remove</button></div>`,
  );
  const hidden = ["off", "on"].map(
    (v) =>
      `<button type="button" role="radio" data-f="hidden" data-v="${v}" aria-checked="${(v === "on") === fol.hidden}">${v}</button>`,
  );
  const add = a.roots.includes(a.path)
    ? "<small>root</small>"
    : `<button type="button" data-add="${esc(a.path)}">add as root</button>`;
  top.innerHTML = `<p class="ch">roots</p>${roots.join("") || '<p class="lft">none yet: add a folder below</p>'}${err}
    <p class="ch">browse</p><div class="lrw"><b class="sk">${esc(tilde(a.path))}</b>${add}</div>
    <div class="lrw"><span>hidden folders</span><div class="seg" role="radiogroup" aria-label="hidden">${hidden.join("")}</div></div>
    ${a.path === a.home ? "" : go(a.path.replace(/\/[^/]*$/, ""), "‹ up")}${a.dirs.map((d) => go(`${a.path}/${d}`, d, '<i class="cv">▸</i>')).join("")}`;
}

// The open sheet. A sheet opens with its header button (also with no script); from inside another sheet, it takes
// that one's place.
const sheets = (): HTMLElement[] => [...document.querySelectorAll<HTMLElement>("#sheet > .sh")];
const sheetOf = (which: string): HTMLElement | null => document.getElementById(`sh-${which}`);
let open = "";
export function show(which: string): void {
  for (const s of sheets()) if (s.matches(":popover-open") && s.id !== `sh-${which}`) s.hidePopover();
  const s = which ? sheetOf(which) : null;
  if (s && !s.matches(":popover-open")) {
    const source = $<HTMLElement>(`header [data-open=${which}]`);
    s.showPopover(source ? { source } : undefined);
  }
}
for (const s of sheets()) {
  s.addEventListener("toggle", (e) => {
    const which = s.id.slice(3);
    const shown = (e as ToggleEvent).newState === "open";
    if (shown) open = which;
    else if (open === which) open = "";
    for (const b of document.querySelectorAll("header [data-open]"))
      b.setAttribute("aria-pressed", String(b.getAttribute("data-open") === open));
    if (shown) {
      if (which === "folders") folders();
      dispatchEvent(new CustomEvent("cooee-sheet", { detail: which }));
      return;
    }
    if (which === "folders" && fol.changed) location.reload();
    if (!open && location.hash.startsWith("#settings"))
      history.replaceState(null, "", location.pathname + location.search);
  });
}
// A deep link: #settings, or #settings/<section> scrolled to that section.
function deep(): void {
  const m = location.hash.match(/^#settings(?:\/([\w-]+))?$/);
  if (!m) return;
  show("settings");
  if (m[1]) $(`#s-${m[1]}`)?.scrollIntoView({ block: "start" });
}
addEventListener("hashchange", deep);
addEventListener("DOMContentLoaded", deep);

addEventListener("click", (e) => {
  const t = (e.target as Element).closest?.<HTMLElement>(
    "[data-open],[data-close],[data-k],[data-f],[data-go],[data-add],[data-rm]",
  );
  if (!t) return;
  const d = t.dataset;
  if (d.open) {
    // In a sheet, the next one takes its place; a header button toggles its sheet itself.
    if (t.closest(".sh")) {
      e.preventDefault();
      show(d.open);
    }
  } else if (d.k) {
    // Applied here at once; with no script the button posts the pick instead.
    e.preventDefault();
    set(d.k as Key, d.v === "system" ? undefined : d.v);
  } else if (d.f) {
    fol.hidden = d.v === "on";
    folders();
  } else if (d.go) {
    fol.path = d.go;
    folders();
  } else if (d.add) folders({ path: d.add });
  else if (d.rm) {
    if (confirm(`Remove ${tilde(d.rm)} from the roots? Its folders and threads stay.`))
      folders({ path: d.rm, remove: true });
  } else if ("close" in d) show("");
});
// A tap outside an open sheet closes it (the popover's light dismiss) and goes no further, as a scrim's tap did.
let outside = false;
const inSheet = (e: Event): boolean => !!(e.target as Element).closest?.("#sheet .sh, header [data-open]");
addEventListener(
  "pointerdown",
  (e) => {
    outside = !!$("#sheet :popover-open") && !inSheet(e);
  },
  true,
);
addEventListener(
  "pointerup",
  () =>
    setTimeout(() => {
      outside = false;
    }),
  true,
);
addEventListener(
  "click",
  (e) => {
    if (!outside) return;
    e.preventDefault();
    e.stopPropagation();
  },
  true,
);
// System follows the phone live; another tab's pick shows here once this one is in view again.
mq.addEventListener("change", () => hold(paint));
document.addEventListener("visibilitychange", () => {
  if (document.hidden || JSON.stringify(read()) === JSON.stringify(pick)) return;
  pick = read();
  hold(paint);
});
html.dataset.mode = mode();
marks();
// A theme's live preview (/themes/dev/<name>) reloads on each save.
const building = $("[data-preview]")?.dataset.preview;
if (building) {
  const saves = new EventSource(`/themes/dev/${building}/events`);
  saves.addEventListener("saved", () => location.reload());
  // [keep] moves it away: no reload races the form.
  addEventListener("submit", () => saves.close());
}
