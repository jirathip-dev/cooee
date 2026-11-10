import { chromium } from "@playwright/test";
const out = process.env.OUT, base = `http://127.0.0.1:${process.env.P}`;
const browser = await chromium.launch();
for (const w of [390, 1440]) {
  const ctx = await browser.newContext({ baseURL: base, viewport: { width: w, height: w === 390 ? 844 : 900 }, storageState: `${out}/state.json` });
  await ctx.addCookies([{ name: "cooee-look", value: `theme:farm/mode:light`, url: base }]);
  const page = await ctx.newPage();
  await page.goto("/new");
  await page.getByLabel("message", { exact: true }).fill(`verify #115 at ${w}`);
  await page.getByRole("button", { name: "send" }).click();
  await page.waitForURL(/\/t\//);
  console.log(`${w} sent -> ${new URL(page.url()).pathname}`);
  await page.waitForTimeout(3000);
  await ctx.addCookies([{ name: "cooee-look", value: `theme:farm/mode:light/scene:on`, url: base }]);
  await page.emulateMedia({ colorScheme: "light", reducedMotion: "reduce" });
  await page.goto("/?list");
  await page.locator("#scene .sb .sf use").first().waitFor();
  const cards = await page.locator("#scene .sb").evaluateAll((bs) => bs.map((b) => [b.querySelector(".sf")?.dataset.s, b.querySelector("use")?.getAttribute("href"), b.innerText.replace(/\s+/g, " ").trim()]));
  console.log(`${w} cards=${JSON.stringify(cards)}`);
  await page.waitForTimeout(500);
  await page.screenshot({ path: `${out}/${w}-scene-farm-after-chat.png`, fullPage: true });
  console.log(`${w} scene-farm-after-chat ok`);
  await ctx.close();
}
await browser.close();
