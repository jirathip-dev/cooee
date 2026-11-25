import "./blocks.js";

const NAMED = ["#1d1f21", "var(--r)", "var(--g)", "var(--a)", "var(--l)", "var(--p)", "var(--c)", "var(--fg)"];
const colour = (c) => (c === null || c === undefined ? "" : c < 16 ? NAMED[c % 8] : "");

customElements.define("cooee-terminal", class extends HTMLElement {
  connectedCallback() {
    const props = JSON.parse(this.dataset.props || "{}");
    const scr = document.createElement("pre");
    scr.className = "term";
    for (const runs of props.rows || []) {
      const d = scr.appendChild(document.createElement("div"));
      for (const r of runs) {
        const s = d.appendChild(document.createElement("span"));
        s.textContent = r.text;
        const fg = colour(r.fg);
        if (fg) s.style.color = fg;
        if (r.bold) s.style.fontWeight = "600";
      }
    }
    const cur = scr.lastChild?.appendChild(document.createElement("span"));
    if (cur) cur.className = "cur";
    this.replaceChildren(scr);
  }
});
