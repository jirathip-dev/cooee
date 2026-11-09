import { chromium } from "@playwright/test";
const base = `http://127.0.0.1:${process.env.CP}`;
const b = await chromium.launch(); const p = await b.newPage({ baseURL: base });
await p.goto("/"); await p.getByLabel("digit 1").click(); await p.keyboard.type(process.env.CODE);
await p.waitForURL(/\/(\?paired)?$/, { waitUntil: "commit" });
const r = await p.context().request.post(`${base}/push/apns`, { data: { token: "bb".repeat(32), env: "production" } }); console.log("POST /push/apns", r.status(), await r.text());
await b.close();
