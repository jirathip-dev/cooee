// cooee's example plugin backend (docs/plugins.md): `PORT=4800 COOEE=http://127.0.0.1:4747 TOKEN=… node server.mjs`.
import fs from "node:fs";
import http from "node:http";

const { PORT = "4800", COOEE = "http://127.0.0.1:4747", TOKEN } = process.env;
const streams = new Set();
const tell = (event, data) => { for (const s of streams) s.write(`event: ${event}\ndata: ${JSON.stringify(data)}\n\n`); };
const json = (res, status, v) => res.writeHead(status, { "content-type": "application/json" }).end(JSON.stringify(v));
const toCooee = async (path, body) => (await fetch(COOEE + path, {
  method: "POST", headers: { authorization: `Bearer ${TOKEN}`, "content-type": "application/json" }, body: JSON.stringify(body),
})).status;
const read = async (req) => { let b = ""; for await (const c of req) b += c; return b ? JSON.parse(b) : {}; };

http.createServer(async (req, res) => {
  // Only cooee calls this backend, and it sends the plugin's token.
  if (req.headers.authorization !== `Bearer ${TOKEN}`) return json(res, 401, "no token");
  if (req.url === "/tab.js") return res.writeHead(200, { "content-type": "text/javascript" }).end(fs.readFileSync(new URL("tab.js", import.meta.url)));
  if (req.url === "/whoami") return json(res, 200, { token: "ok", cookie: req.headers.cookie ?? null });
  if (req.url === "/events") {
    res.writeHead(200, { "content-type": "text/event-stream" }).write(": open\n\n");
    streams.add(res);
    req.on("close", () => streams.delete(res));
    return tell("hello", "the stream is open");
  }
  const body = await read(req);
  if (req.url === "/notify") return json(res, 200, await toCooee("/plugin/push", { text: "hello from the plugin", path: "hi" }));
  if (req.url === "/ask") return json(res, 200, await toCooee("/plugin/card", { thread: body.thread, text: "Ship it?", options: ["yes", "no"] }));
  if (req.url === "/cooee/choice") {
    tell("choice", body.option);
    // A run card's [approve] carries its sha; herdsman would run it and answer the exit code and output tail.
    if (body.option === "approve" && body.value) return json(res, 200, { line: `ran: exit 0\n\`\`\`\nat ${body.value.slice(0, 12)}\n\`\`\`` });
    return json(res, 200, "ok");
  }
  json(res, 404, "no such path");
}).listen(Number(PORT), "127.0.0.1");
