import { chromium } from "@playwright/test";
import fs from "node:fs";
const out = process.env.OUT, base = `http://127.0.0.1:${process.env.P}`;
const browser = await chromium.launch();
const pair = await browser.newPage({ baseURL: base });
await pair.goto("/");
console.log("unpaired / ->", new URL(pair.url()).pathname);
await pair.getByLabel("digit 1").click();
await pair.keyboard.type(fs.readFileSync(`${out}/code`, "utf8").trim());
await pair.waitForURL(/\/(\?paired)?$/, { waitUntil: "commit" });
await pair.context().storageState({ path: `${out}/state.json` });
console.log("paired ->", new URL(pair.url()).pathname);
// #130: the APNs token endpoint, as a paired native device would call it
const r = pair.context().request;
const tok = "AB".repeat(32);
for (const [m, body] of [
  ["POST", { token: tok, env: "sandbox" }],
  ["POST", { token: "not-hex!" }],
  ["POST", { token: tok.toLowerCase() }],
]) {
  const res = await r.fetch(`${base}/push/apns`, { method: m, data: body });
  console.log(`${m} /push/apns ${JSON.stringify(body)} -> ${res.status()} ${await res.text()}`);
}
let e = await r.get(`${base}/push/endpoints`);
console.log("GET /push/endpoints ->", e.status(), await e.text());
const t = await r.post(`${base}/push/test`, { data: {} });
console.log("POST /push/test ->", t.status(), await t.text());
let d = await r.fetch(`${base}/push/apns`, { method: "DELETE", data: { token: tok } });
console.log("DELETE /push/apns ->", d.status(), await d.text());
e = await r.get(`${base}/push/endpoints`);
console.log("GET /push/endpoints ->", e.status(), await e.text());
// re-add one so the devices screen / removal can be checked from outside
await r.fetch(`${base}/push/apns`, { method: "POST", data: { token: tok } });
const anon = await (await browser.newContext()).request.fetch(`${base}/push/apns`, { method: "POST", data: { token: tok }, maxRedirects: 0 });
console.log("unpaired POST /push/apns ->", anon.status());
for (const w of [390, 1440]) {
  const page = await browser.newPage({ baseURL: base, viewport: { width: w, height: w === 390 ? 844 : 900 }, storageState: `${out}/state.json` });
  const shot = async (name, ready) => {
    await ready.waitFor();
    await page.screenshot({ path: `${out}/${w}-${name}.png`, fullPage: true });
    console.log(`${w} ${name} ok`);
  };
  await page.goto("/?list");
  await shot("agents", page.locator(".ags .agr").first());
  await page.goto("/new");
  await shot("new", page.getByLabel("message", { exact: true }));
  await page.goto(`/t/long-${w}`);
  await shot("chat", page.getByLabel("message", { exact: true }));
  await page.goto("/#settings/sign-ins");
  await shot("settings", page.locator("#sheet .sh:popover-open .six"));
  await page.goto("/#settings/alerts");
  await shot("alerts", page.locator("#sheet .sh:popover-open .six"));
  await page.goto("/#settings/devices");
  await shot("devices", page.locator("#sheet .sh:popover-open .six"));
  await page.goto("/p/hello");
  await shot("plugin-hello", page.locator("iframe.pf"));
  await page.goto("/p/machines");
  await shot("plugin-machines", page.frameLocator("iframe.pf").locator("article.on"));
  await page.close();
}
await browser.close();
