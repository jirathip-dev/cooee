import { chromium } from "@playwright/test";
import fs from "node:fs";
const out = process.env.OUT, base = `http://127.0.0.1:${process.env.P}`, shed = process.env.SHED;
const browser = await chromium.launch();
const errs = [];
const session = (models) => fs.writeFileSync(`${shed}/session.json`, JSON.stringify({ id: "s1", model: "opus-2", models }));
for (const w of [1440, 390]) {
  const page = await browser.newPage({ baseURL: base, viewport: { width: w, height: w === 390 ? 844 : 900 }, storageState: `${out}/state.json` });
  page.on("console", (m) => m.type() === "error" && errs.push(`${w} console: ${m.text()}`));
  page.on("response", (r) => r.status() >= 400 && errs.push(`${w} ${r.status()} ${r.url()}`));
  page.on("requestfailed", (r) => errs.push(`${w} failed ${r.url()} ${r.failure()?.errorText}`));
  const api = async (tag) => { const r = await page.request.get("/models"); const j = await r.json(); console.log(`${w} ${tag} GET /models ${r.status()}`, JSON.stringify(j)); return j; };
  const opts = async (tag) => { const o = await page.getByLabel("model", { exact: true }).locator("option").allTextContents(); const v = await page.getByLabel("model", { exact: true }).locator("option").evaluateAll(e => e.map(x => x.value)); console.log(`${w} ${tag} picker values`, JSON.stringify(v), "texts", JSON.stringify(o)); return v; };
  const start = async (pick, text) => {
    await page.goto("/new");
    await page.getByLabel("model", { exact: true }).selectOption(pick);
    await page.getByLabel("message", { exact: true }).fill(text);
    await page.getByRole("button", { name: "send" }).click();
    await page.waitForURL(/\/t\//);
    await page.locator(".tb .ds").waitFor();
    console.log(`${w} started ${pick} -> ${new URL(page.url()).pathname}; bar: ${await page.locator(".tb .ds").textContent()}`);
  };
  // 0: as seeded
  session([["opus-2", "Opus 2"], ["sonnet-2", "Sonnet 2"]]);
  await api("seeded");
  await page.goto("/new"); await opts("seeded /new");
  // AC: a new model reported by the agent appears with no code change
  session([["opus-2", "Opus 2"], ["opus-3", "Opus 3"], ["sonnet-2", "Sonnet 2"]]);
  await start("opus", `w${w} short opus`);
  await page.waitForTimeout(1500);
  const shortThread = new URL(page.url()).pathname;
  await opts("chat switcher after new model");
  await page.screenshot({ path: `${out}/${w}-chat-short-opus.png`, fullPage: true });
  const a1 = await api("after opus-3 reported");
  await page.goto("/new"); await opts("/new after opus-3 reported");
  await page.screenshot({ path: `${out}/${w}-new-after-opus3.png`, fullPage: true });
  // AC: removed pinned id falls back to its family's short name with one thread line
  await start("opus-3", `w${w} pinned opus-3`);
  session([["opus-2", "Opus 2"], ["sonnet-2", "Sonnet 2"]]);
  await page.goto("/new"); await start("opus-3", `w${w} pinned opus-3 gone`);
  await page.waitForTimeout(2000); await page.reload(); await page.locator(".tb .ds").waitFor();
  const body = await page.locator("body").innerText();
  const lines = body.split("\n").filter((l) => /is gone/.test(l));
  console.log(`${w} pinned-gone thread lines:`, JSON.stringify(lines), "bar:", await page.locator(".tb .ds").textContent());
  await page.screenshot({ path: `${out}/${w}-chat-pinned-gone.png`, fullPage: true });
  // AC: removed short-name target moves to next newest
  await api("after opus-3 removed");
  await page.goto("/new"); await opts("/new after opus-3 removed");
  await page.goto(shortThread); await page.locator(".tb .ds").waitFor();
  console.log(`${w} short-opus thread bar now: ${await page.locator(".tb .ds").textContent()}`);
  await page.close();
}
await browser.close();
fs.writeFileSync(`${out}/models-errors.log`, errs.join("\n") + "\n");
console.log("errors:", errs.length);
