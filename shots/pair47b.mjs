import { chromium } from "@playwright/test";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
const out = process.env.OUT, P = process.env.P;
const ip = `http://127.0.0.1:${P}`, lh = `http://localhost:${P}`;
const rd = (n) => fs.readFileSync(`${out}/${n}`, "utf8").trim();
const newCode = () => execFileSync(rd("code.bin"), ["pair", "--config", rd("code.config")], { encoding: "utf8" }).match(/^code {2}(\d{3}) (\d{3})/m).slice(1).join("");
const log = (...a) => console.log(...a);
const SAF = (v) => `Mozilla/5.0 (iPhone; CPU iPhone OS ${v}_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/${v}.0 Mobile/15E148 Safari/604.1`;
const HOME = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148";
const browser = await chromium.launch();
const unpaired = { storageState: { cookies: [], origins: [] } };
const ctx = (base, extra = {}) => browser.newContext({ baseURL: base, viewport: { width: 390, height: 844 }, ...unpaired, ...extra });
async function shots(page, name, mode = "dark", widths = [390, 1440]) {
  await page.emulateMedia({ colorScheme: mode, reducedMotion: "reduce" });
  for (const w of widths) { await page.setViewportSize({ width: w, height: w === 390 ? 844 : 900 }); await page.waitForTimeout(150); await page.screenshot({ path: `${out}/shots/${w}-${name}.png`, fullPage: true }); }
  await page.setViewportSize({ width: 390, height: 844 }); log(`shot ${name} (${mode})`);
}
for (const [n, ua] of [["safari18", SAF(18)], ["safari26", SAF(26)], ["homescreen", HOME]]) {
  const p = await (await ctx(ip, { userAgent: ua })).newPage(); await p.goto("/pair");
  log(`hint ${n}:`, await p.locator(".hint").count(), JSON.stringify((await p.locator(".hint").allInnerTexts()).join("")));
  if (n === "safari26") await shots(p, "pair-ios26-hint", "light", [390]);
}
// Paired via QR, iPhone UA: offer copy; settings pair QR
const A = await ctx(ip, { userAgent: SAF(18) }); const pa = await A.newPage();
await pa.goto(`/pair?code=${newCode()}`); await pa.waitForURL(/\/$/);
await pa.locator("#pairing .pc").waitFor();
log("iPhone offer:", JSON.stringify(await pa.locator("#pairing").innerText()));
await shots(pa, "pair-offer-iphone", "dark", [390]);
await pa.goto("/#settings/devices");
await pa.getByRole("button", { name: "pair a device" }).click();
const panel = pa.locator("#s-pair-box .pc"); await panel.waitFor();
log("panel:", JSON.stringify(await panel.innerText()));
await panel.scrollIntoViewIfNeeded();
await shots(pa, "settings-pair-qr");
const box = await panel.locator("img").boundingBox(); log("qr img box:", JSON.stringify(box));
await panel.locator(".qrt").screenshot({ path: `${out}/shots/qr-tile.png` });
const r = await pa.request.post("/devices/pair", { data: {} }); const j = await r.json();
log("/devices/pair:", r.status(), "link:", j.link, "host:", j.host, "code:", j.code);
fs.writeFileSync(`${out}/qr.svg`, j.svg);
// the new link's QR pairs a fresh device at once
const B = await ctx(ip); const pb = await B.newPage(); await pb.goto(new URL(j.link).pathname + new URL(j.link).search); await pb.waitForURL(/\/$/);
log("fresh device via settings QR link ->", new URL(pb.url()).pathname, "devices:", (await pb.request.get("/devices")).status());
// declined card copy after its fetch settles
const C = await ctx(ip); const pc = await C.newPage(); await pc.goto("/pair"); await pc.getByRole("button", { name: "ask to pair" }).click(); await pc.locator("#ask-wait").waitFor();
const word = (await pc.locator("#ask-word").innerText()).trim();
await pa.goto("/"); const card = pa.getByRole("group", { name: new RegExp(`asks to pair: ${word}`) }); await card.waitFor({ timeout: 10000 });
log("no-passkey card:", JSON.stringify(await card.innerText()));
await shots(pa, "approve-no-passkey", "dark");
await card.getByRole("button", { name: "decline" }).click(); await pa.waitForTimeout(1500);
log("declined card:", JSON.stringify(await pa.locator("#pairing").innerText()));
await shots(pa, "approve-declined-settled", "dark", [390]);
await browser.close();
