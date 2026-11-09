import { chromium } from "@playwright/test";
const base = `http://127.0.0.1:${process.env.CP}`, code = process.env.CODE;
const b = await chromium.launch(); const p = await b.newPage({ baseURL: base });
await p.goto("/"); await p.getByLabel("digit 1").click(); await p.keyboard.type(code);
await p.waitForURL(/\/(\?paired)?$/, { waitUntil: "commit" });
const r = p.context().request;
for (const token of ["aa".repeat(32), "dd".repeat(32)]) {
  const res = await r.post(`${base}/push/apns`, { data: { token } }); console.log("POST", token.slice(0, 4), res.status(), await res.text());
}
const t = await r.post(`${base}/push/test`, { data: {} }); console.log("POST /push/test", t.status(), await t.text());
await new Promise(s => setTimeout(s, 3000));
const e = await r.get(`${base}/push/endpoints`); console.log("endpoints after send", await e.text());
const dv = await (await r.get(`${base}/devices`)).json(); const me = dv.find(d => d.me);
const rm = await r.post(`${base}/devices/remove`, { data: { id: me.id } }); console.log("remove own device", rm.status(), await rm.text());
await b.close();
