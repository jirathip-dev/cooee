import { chromium } from "@playwright/test";
const out = process.env.OUT, base = `http://127.0.0.1:${process.env.P}`;
const browser = await chromium.launch();
for (const w of [390, 1440]) for (let run = 1; run <= 3; run++) {
  const ctx = await browser.newContext({ baseURL: base, viewport: { width: w, height: w === 390 ? 844 : 900 }, hasTouch: true, storageState: `${out}/state.json` });
  const page = await ctx.newPage();
  await page.goto(`/t/long-${w}`);
  await page.getByLabel("message", { exact: true }).waitFor();
  await page.waitForTimeout(500);
  const reply = page.locator(".blk.ai").last();
  await reply.scrollIntoViewIfNeeded();
  const b = await reply.boundingBox();
  const cdp = await ctx.newCDPSession(page);
  await cdp.send("Input.dispatchTouchEvent", { type: "touchStart", touchPoints: [{ x: b.x + 30, y: b.y + 10 }] });
  await page.waitForTimeout(800);
  const held = await page.locator("#menu").isVisible();
  await cdp.send("Input.dispatchTouchEvent", { type: "touchEnd", touchPoints: [] });
  await page.waitForTimeout(200);
  const after = await page.locator("#menu").isVisible();
  if (run === 1) await page.screenshot({ path: `${out}/${w}-longpress.png` });
  console.log(`${w} run ${run}: x=${Math.round(b.x+30)} menu while held: ${held}, after release: ${after}`);
  await ctx.close();
}
await browser.close();
