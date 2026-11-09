import { chromium } from "@playwright/test";
const out = process.env.OUT, base = `http://127.0.0.1:${process.env.P}`;
const browser = await chromium.launch();
const page = await browser.newPage({ baseURL: base, viewport: { width: 390, height: 844 }, storageState: `${out}/state.json` });
await page.goto("/t/long-390");
await page.getByLabel("message", { exact: true }).waitFor();
await page.waitForTimeout(400);
const touch = (type, x) => page.locator(".cfm .chat").evaluate((el, [type, x]) => {
  el.dispatchEvent(Object.assign(new Event(type, { bubbles: true }), { touches: type === "touchend" ? [] : [{ clientX: x, clientY: 400 }] }));
}, [type, x]);
await touch("touchstart", 5); await touch("touchmove", 180); await page.waitForTimeout(300);
console.log(await page.evaluate(() => {
  const pick = (el) => { const s = getComputedStyle(el); const b = el.getBoundingClientRect(); return `<${el.tagName.toLowerCase()}.${el.className}> "${el.textContent.trim().slice(0,12)}" pos=${s.position} z=${s.zIndex} transform=${s.transform} willChange=${s.willChange} x=${Math.round(b.x)}`; };
  const out = [];
  const row = [...document.querySelectorAll(".sdb *")].filter(e => /1089d/.test(e.textContent) && e.children.length < 4);
  row.slice(-4).forEach(e => { out.push(pick(e)); [...e.children].forEach(c => out.push("  " + pick(c))); });
  for (const sel of [".app", ".sdb", ".cfm", ".cfm .chat", "main", ".tbar"]) { const e = document.querySelector(sel); if (e) out.push(sel + " " + pick(e)); }
  return out.join("\n");
}));
await browser.close();
