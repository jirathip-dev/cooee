import { chromium } from "@playwright/test";
const out = process.env.OUT, base = `http://127.0.0.1:${process.env.P}`;
const browser = await chromium.launch();
const page = await browser.newPage({ baseURL: base, viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, storageState: `${out}/state.json` });
await page.emulateMedia({ colorScheme: "light", reducedMotion: "reduce" });
await page.goto("/more");
await page.locator("h1.lt").waitFor();
const r = await page.evaluate(() => {
  const o = {};
  const g = document.querySelector(".gs");
  o.groups = [...document.querySelectorAll(".gs")].map(e => { const b = e.getBoundingClientRect(); return `${e.tagName}.${e.className} x=${b.x} w=${b.width}`; });
  o.heads = [...document.querySelectorAll("h2, .gs > h2, .gh, [class*=gh]")].map(e => { const b = e.getBoundingClientRect(); return `${e.tagName}.${e.className} '${e.textContent.trim()}' x=${b.x} pad-left=${getComputedStyle(e).paddingLeft}`; });
  o.icons = [...document.querySelectorAll(".gs .ms, .ms")].map(e => `${e.closest("a,button")?.textContent.trim().slice(0,20)}: '${e.textContent.trim()}' bg=${getComputedStyle(e).backgroundColor}`);
  return o;
});
console.log(JSON.stringify(r, null, 1));
// scrolled frame as spec does
await page.goto("/?list");
await page.locator(".ags .agr").first().waitFor();
await page.locator(".sdb").evaluate((s) => { s.style.minHeight = "0"; s.append(Object.assign(document.createElement("div"), { style: "height:2000px" })); s.scrollTop = 96; });
await page.waitForTimeout(500);
const nb = page.locator(".sdb > .nb");
console.log("scrolled: class", await nb.getAttribute("class"), "nt opacity", await nb.locator(".nt").evaluate(e => getComputedStyle(e).opacity), "nt", await nb.locator(".nt").evaluate(e => `${e.textContent} ${getComputedStyle(e).fontSize}`), "blur", await nb.evaluate(e => getComputedStyle(e).backdropFilter), "shadow", await nb.evaluate(e => getComputedStyle(e).boxShadow + " | bb " + getComputedStyle(e).borderBottomWidth));
await page.screenshot({ path: `${out}/390-chats-scrolled96-light.png` });
await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" });
await page.waitForTimeout(300);
await page.screenshot({ path: `${out}/390-chats-scrolled96-dark.png` });
console.log("tbar hairline:", await page.locator(".tbar").evaluate(e => `shadow=${getComputedStyle(e).boxShadow} before=${getComputedStyle(e, "::before").backgroundColor}/${getComputedStyle(e,"::before").height}`));
await browser.close();
