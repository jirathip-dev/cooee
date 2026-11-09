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
const touch = (page, type, x) =>
  page.locator(".cfm .chat").evaluate((el, [type, x]) => {
    const e = Object.assign(new Event(type, { bubbles: true }), { touches: type === "touchend" ? [] : [{ clientX: x, clientY: 400 }] });
    el.dispatchEvent(e);
  }, [type, x]);
for (const w of [390, 1440]) {
  const page = await browser.newPage({ baseURL: base, viewport: { width: w, height: w === 390 ? 844 : 900 }, storageState: `${out}/state.json`, deviceScaleFactor: w === 390 ? 2 : 1 });
  const shot = async (name, ready, full = true) => {
    await ready.waitFor();
    await page.waitForTimeout(400);
    await page.screenshot({ path: `${out}/${w}-${name}.png`, fullPage: full });
    console.log(`${w} ${name} ok`);
  };
  await page.goto("/?list");
  await shot("agents", page.locator(".ags .agr").first());
  console.log(w, "header.bar count:", await page.locator("header.bar").count(), "refresh btn:", await page.getByRole("button", { name: "refresh" }).count());
  console.log(w, "tabs:", JSON.stringify(await page.getByRole("navigation", { name: "tabs" }).getByRole("link").allTextContents()));
  console.log(w, "tbar box:", JSON.stringify(await page.locator(".tbar").boundingBox()));
  console.log(w, "sdb box:", JSON.stringify(await page.locator(".sdb").boundingBox()));
  console.log(w, "lt:", await page.locator(".lt").first().evaluate(e => { const s = getComputedStyle(e); return `${e.textContent} ${s.fontSize}/${s.lineHeight} ${s.fontWeight}`; }));
  console.log(w, "viewport meta:", await page.locator('meta[name=viewport]').getAttribute("content"), "| status:", await page.locator('meta[name=apple-mobile-web-app-status-bar-style]').getAttribute("content"));
  console.log(w, "tbar css:", await page.locator(".tbar").evaluate(e => { const s = getComputedStyle(e); return `${s.backdropFilter} ${s.backgroundColor} border-top:${s.borderTopWidth} ${s.borderTopColor} pad-bottom:${s.paddingBottom}`; }));
  await page.goto("/new");
  await shot("new", page.getByLabel("message", { exact: true }));
  await page.goto(`/t/long-${w}`);
  await shot("chat", page.getByLabel("message", { exact: true }));
  console.log(w, "chat: tbar visible:", await page.locator(".tbar").isVisible());
  await page.goto("/#settings/sign-ins");
  await shot("settings", page.locator("#sheet .sh:popover-open .six"));
  await page.goto("/p/hello");
  await shot("plugin-hello", page.locator("iframe.pf"));
  await page.goto("/p/machines");
  await shot("plugin-machines", page.frameLocator("iframe.pf").locator("article.on"));
  // Shell frames, both modes
  for (const mode of ["light", "dark"]) {
    await page.emulateMedia({ colorScheme: mode, reducedMotion: "reduce" });
    await page.goto("/?list");
    await shot(`chats-${mode}`, page.locator(".ags .agr").first(), false);
    const nb = page.locator(".sdb > .nb");
    console.log(w, mode, "nb h:", await nb.evaluate(e => getComputedStyle(e).height), "nt opacity rest:", await nb.locator(".nt").evaluate(e => getComputedStyle(e).opacity));
    await page.locator(".sdb").evaluate((s) => { s.scrollTop = 96; });
    await page.waitForTimeout(300);
    console.log(w, mode, "scrollTop:", await page.locator(".sdb").evaluate(s => s.scrollTop), "nb class:", await nb.getAttribute("class"), "nt opacity:", await nb.locator(".nt").evaluate(e => getComputedStyle(e).opacity), "blur:", await nb.evaluate(e => getComputedStyle(e).backdropFilter));
    await shot(`chats-scrolled-${mode}`, nb, false);
    await page.goto("/more");
    await shot(`more-${mode}`, page.locator("h1.lt"), false);
    console.log(w, mode, "more rows:", JSON.stringify(await page.locator(".gs .sk").allTextContents()));
    await page.goto("/w");
    await shot(`workflow-${mode}`, page.locator("h1.lt"), false);
    if (w === 390) {
      await page.goto("/t/long-390");
      await page.getByLabel("message", { exact: true }).waitFor();
      await touch(page, "touchstart", 5);
      await touch(page, "touchmove", 180);
      await page.waitForTimeout(200);
      await page.screenshot({ path: `${out}/${w}-chat-swipe-${mode}.png` });
      console.log(w, mode, "swipe: app class:", await page.locator(".app").getAttribute("class"), "--sx:", await page.locator(".app").evaluate(a => a.style.getPropertyValue("--sx")), "tbar visible:", await page.locator(".tbar").isVisible());
      await touch(page, "touchend", 0);
      await page.waitForURL(/\/\?list/, { timeout: 3000 }).then(() => console.log(w, mode, "swipe released -> ", new URL(page.url()).pathname + new URL(page.url()).search)).catch(e => console.log(w, mode, "swipe release: no nav", e.message));
    } else {
      await page.goto("/t/long-1440");
      await shot(`chats-chat-${mode}`, page.getByLabel("message", { exact: true }), false);
    }
  }
  // press tint
  await page.goto("/more");
  const row = page.getByRole("button", { name: "Settings" });
  const before = await row.evaluate(e => getComputedStyle(e).backgroundColor);
  await row.hover(); await page.mouse.down();
  const during = await row.evaluate(e => getComputedStyle(e).backgroundColor);
  await page.mouse.up();
  console.log(w, "press tint: before", before, "pressed", during);
  await page.close();
}
await browser.close();
