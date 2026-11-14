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
    addEventListener("DOMContentLoaded", () => { side(); screen(s); fillLoading(); });
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

  function side() {
    const a = document.querySelector("aside.side");
    if (!a) return;
    const x = new XMLHttpRequest();
    x.open("GET", "side.html", false);
    x.send();
    a.innerHTML = x.responseText;
    for (const e of a.querySelectorAll(`[data-id="${a.dataset.cur}"]`)) e.setAttribute("aria-current", "page");
  }
  function screen(s) {
    for (const e of document.querySelectorAll("[data-only]")) if (!e.dataset.only.split(" ").includes(s)) e.remove();
    for (const e of document.querySelectorAll("[data-exp]")) e.setAttribute("aria-expanded", e.dataset.exp === s);
  }
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
