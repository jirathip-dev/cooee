import { chromium } from "@playwright/test";
import fs from "node:fs";
const out = process.env.OUT, base = `http://127.0.0.1:${process.env.P}`;
const browser = await chromium.launch();
const pair = await browser.newPage({ baseURL: base });
await pair.goto("/");
console.log("unpaired / ->", new URL(pair.url()).pathname);
await pair.getByLabel("pairing code").fill(fs.readFileSync(`${out}/code`, "utf8").trim());
await pair.getByRole("button", { name: "pair" }).click();
await pair.waitForURL(/\/$/, { waitUntil: "commit" });
await pair.context().storageState({ path: `${out}/state.json` });
console.log("paired ->", new URL(pair.url()).pathname);
for (const w of [390, 1440]) {
  const page = await browser.newPage({ baseURL: base, viewport: { width: w, height: w === 390 ? 844 : 900 }, storageState: `${out}/state.json` });
  const shot = async (name, ready) => {
    await ready.waitFor();
    await page.screenshot({ path: `${out}/${w}-${name}.png`, fullPage: true });
    console.log(`${w} ${name} ok`);
  };
  await page.goto("/");
  await shot("threads", page.locator(".fc .tr").first());
  await page.goto("/#new");
  await shot("new", page.getByLabel("first message"));
  await page.goto(`/t/long-${w}`);
  await shot("chat", page.getByLabel("message", { exact: true }));
  await page.goto("/#settings/sign-ins");
  await shot("settings", page.locator("#sheet .sh:popover-open .six"));
  await page.goto("/p/hello");
  await shot("plugin-hello", page.locator("iframe.pf"));
  await page.goto("/p/machines");
  await shot("plugin-machines", page.frameLocator("iframe.pf").locator("article.on"));
  await page.close();
}
await browser.close();
