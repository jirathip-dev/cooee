import { chromium } from "@playwright/test";
const base = `http://127.0.0.1:${process.env.CP}`;
const b = await chromium.launch(); const p = await b.newPage({ baseURL: base });
await p.goto("/"); await p.getByLabel("digit 1").click(); await p.keyboard.type(process.env.CODE);
await p.waitForURL(/\/(\?paired)?$/, { waitUntil: "commit" });
const t = await p.context().request.post(`${base}/push/test`, { data: {} }); console.log("second device POST /push/test", t.status(), await t.text());
await new Promise(s => setTimeout(s, 3000)); await b.close();
