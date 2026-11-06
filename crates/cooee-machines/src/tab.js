// The machines tab (crates/cooee-machines): the backend's cards, asked for every 30 s. A tapped number opens its
// 24 h chart (a <details>), which stays open across refreshes.
document.head.appendChild(document.createElement("style")).textContent = `
.ml{display:grid;gap:12px;grid-template-columns:repeat(auto-fill,minmax(300px,1fr))}
.m{background:var(--card);border:1px solid var(--edge);border-radius:12px;padding:12px 14px}
.m h2{margin:0;font-size:1.05em}.m h2 i{font-style:normal}.on h2 i{color:var(--g)}.off h2 i{color:var(--r)}
.seen,.er,.tm,.mt{margin:4px 0;color:var(--mute)}.er{overflow-wrap:anywhere}
.ms{display:grid;gap:2px;margin:8px 0}
summary{display:grid;grid-template-columns:6.5em auto 1fr;align-items:center;gap:8px;min-height:44px;cursor:pointer;list-style:none}
summary::-webkit-details-marker{display:none}.k{color:var(--mute)}.bad{color:var(--r)}
svg{display:block;width:100%;overflow:visible}svg polyline{fill:none;stroke:var(--link);stroke-width:1.5;vector-effect:non-scaling-stroke}
.sp{height:20px}.ch svg{height:100px;border-bottom:1px solid var(--edge)}
.ch p{display:flex;justify-content:space-between;gap:8px;margin:4px 0 8px;font-size:.85em;color:var(--mute)}`;
const main = document.body.appendChild(document.createElement("div"));
main.className = "ml";
const load = async () => {
  const open = [...main.querySelectorAll("details[open]")].map((d) => d.id);
  const r = await cooee.fetch("/cards");
  if (!r.ok) return;
  main.innerHTML = r.text;
  for (const id of open) document.getElementById(id)?.setAttribute("open", "");
};
await load();
setInterval(load, 30_000);
