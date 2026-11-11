import http from "node:http";

const { PORT, COOEE, TOKEN, COOKIE } = process.env;
const seen = new Set();
const post = (thread, card) => fetch(`${COOEE}/plugin/card`, {
  method: "POST", headers: { authorization: `Bearer ${TOKEN}`, "content-type": "application/json" },
  body: JSON.stringify({ thread, ...card }),
});
const cards = [
  { text: "Allow fetch sec.gov?", options: ["Allow once", "Always for sec.gov", "Deny"], recommend: "Allow once" },
  { text: "EDGAR plugin settings", form: "config", detail: "+user_agent = cooee research\n+rate = 5 / s" },
  { text: "EDGAR_API_KEY", form: "secret", detail: "EDGAR_API_KEY" },
  { text: "Install the edgar plugin's Python deps", form: "run-card",
    detail: "uv pip install -r plugins/edgar/requirements.txt", sha: "3b7e0c55a1d9" },
];

setInterval(async () => {
  const threads = await fetch(`${COOEE}/threads`, { headers: { cookie: `cooee=${COOKIE}` } }).then((r) => r.json(), () => []);
  for (const t of threads) {
    if (!t.title.startsWith("Show me the cards") || seen.has(t.id)) continue;
    seen.add(t.id);
    for (const card of cards) console.log(`card ${card.text}: ${(await post(t.id, card)).status}`);
  }
}, 1000);

http.createServer(async (req, res) => {
  let body = "";
  for await (const chunk of req) body += chunk;
  if (req.headers.authorization !== `Bearer ${TOKEN}` || req.url !== "/cooee/choice") return res.writeHead(404).end();
  const { option, value } = JSON.parse(body);
  const line = option === "save" ? "EDGAR_API_KEY set ✓" : option === "approve" ? `ran at ${value}: exit 0` : `${option} ✓`;
  res.writeHead(200, { "content-type": "application/json" }).end(JSON.stringify({ line }));
}).listen(Number(PORT), "127.0.0.1");
