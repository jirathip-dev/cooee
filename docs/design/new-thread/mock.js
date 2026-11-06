// #48 mockup: the app's own look on static pages. Served from the repo root (`node docs/design/new-thread/shoot.mjs
// --serve`), so the pages load web/src/app.css, web/src/look.js, web/themes/ and web/schemes/ as they are today:
// nothing here copies the design system. A classic script before look.js: it builds `LOOK` (as the server does) and
// puts the URL's pick in localStorage, where look.js reads it.
//   ?theme=farm|kitchen|garden|terminal  &mode=system|light|dark  &style=modern|terminal  &sprites=on|ascii|off
//   [data-only="a b"]  shown only on those screens; [data-exp=<screen>] aria-expanded on that screen
//   [data-ld="claude · jk@work.com"]  the connecting line: what look.js gains for the `loading` state (below)
//   ?s=<screen>  one screen (html[data-screen]); without it the page stacks every screen in <html data-screens> as frames,
//                each one phone screen (844 px) below 1000 px wide, else one 900 px desktop screen.
(() => {
  const q = new URLSearchParams(location.search);
  const get = (u) => { const x = new XMLHttpRequest(); x.open("GET", u, false); x.send(); return JSON.parse(x.responseText); };
  const THEMES = ["farm", "kitchen", "garden", "terminal"];
  const SCHEMES = ["github-dark", "solarized-dark", "gruvbox", "nord", "dracula", "farm", "kitchen", "garden-dark", "paper",
    "farm-light", "kitchen-light", "garden-light"];
  globalThis.LOOK = {
    default: "farm",
    themes: Object.fromEntries(THEMES.map((t) => [t, get(`/web/themes/${t}/theme.json`)])),
    schemes: Object.fromEntries(SCHEMES.map((s) => [s, get(`/web/schemes/${s}.json`)])),
  };
  const pick = {};
  for (const k of ["theme", "mode", "style", "sprites"]) if (q.get(k)) pick[k] = q.get(k);
  localStorage.setItem("cooee.look", JSON.stringify(pick));
  const html = document.documentElement, s = q.get("s");
  if (s) {
    html.dataset.screen = s;
    addEventListener("DOMContentLoaded", () => { screen(s); fillLoading(); });
    return;
  }
  html.classList.add("gallery");
  addEventListener("DOMContentLoaded", () => {
    const h = innerWidth < 1000 ? 844 : 900;
    const frames = html.dataset.screens.split("|").map((x) => {
      const id = x.slice(0, x.indexOf(":")), cap = x.slice(x.indexOf(":") + 1), u = new URLSearchParams(q);
      u.set("s", id);
      return `<p class="cap">${cap}</p><iframe title="${cap}" src="?${u}" style="height:${h}px"></iframe>`;
    });
    document.body.innerHTML = frames.join("");
    let left = frames.length;
    for (const f of document.querySelectorAll("iframe")) f.onload = () => { if (!--left) setTimeout(() => (document.body.dataset.ready = 1), 300); };
  });

  function screen(s) {
    for (const e of document.querySelectorAll("[data-only]")) if (!e.dataset.only.split(" ").includes(s)) e.remove();
    for (const e of document.querySelectorAll("[data-exp]")) e.setAttribute("aria-expanded", e.dataset.exp === s);
  }
  // The loading state on a line: the theme's loading figure at line height (32 × 24), the glyph ↻ in a theme without
  // sprites, the dot with sprites off, `o.o` in ascii; then the theme's loading word and `· connecting <who>`. A theme
  // with no loading word (terminal) says `connecting <who>` alone. `data-ld=""` (the thread bar): the mark and word only.
  function fillLoading() {
    const t = LOOK.themes[LOOK.themes[pick.theme] ? pick.theme : LOOK.default], figs = pick.sprites || "on";
    const name = pick.theme || LOOK.default;
    const mark = figs === "off" ? '<i class="sd"></i>' : figs === "ascii" ? '<i class="rsa">o.o</i> '
      : t.sprites ? `<svg class="rsp ld" viewBox="0 0 132 100" aria-hidden="true"><use href="/look/${name}/sprites.svg#s-loading"/></svg>`
      : '<i class="sg ld">↻</i> ';
    for (const e of document.querySelectorAll("[data-ld]")) {
      const w = t.words.loading;
      const who = e.dataset.ld;
      e.innerHTML = `<span class="ldst">${mark}<span class="thw">${w || "connecting"}</span></span>`
        + (who ? `<span>${w ? "· connecting " : ""}${who}</span>` : "");
    }
  }
})();
