const log = document.body.appendChild(document.createElement("ul"));
const line = (text) => { log.appendChild(document.createElement("li")).textContent = text; };
const me = await cooee.fetch("/whoami");
line(`whoami ${me.status}: ${me.text}`);
cooee.events("/events", (e) => line(`${e.event}: ${e.data}`));
for (const path of ["/notify", "/ask"]) {
  const b = document.body.appendChild(document.createElement("button"));
  b.textContent = path.slice(1);
  b.onclick = async () => {
    const r = await cooee.fetch(path, { method: "POST", type: "application/json", body: JSON.stringify({ thread: location.hash.slice(1) }) });
    line(`${path.slice(1)} ${r.status}: ${r.text}`);
  };
}
