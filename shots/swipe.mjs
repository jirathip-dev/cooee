import { chromium } from "@playwright/test";
const out = process.env.OUT, base = `http://127.0.0.1:${process.env.P}`;
const browser = await chromium.launch();
for (const mode of ["light", "dark"]) {
  const page = await browser.newPage({ baseURL: base, viewport: { width: 390, height: 844 }, deviceScaleFactor: 3, storageState: `${out}/state.json` });
  await page.emulateMedia({ colorScheme: mode, reducedMotion: "reduce" });
  await page.goto("/t/long-390");
  await page.getByLabel("message", { exact: true }).waitFor();
  await page.waitForTimeout(400);
  const touch = (type, x) => page.locator(".cfm .chat").evaluate((el, [type, x]) => {
    el.dispatchEvent(Object.assign(new Event(type, { bubbles: true }), { touches: type === "touchend" ? [] : [{ clientX: x, clientY: 400 }] }));
  }, [type, x]);
  await touch("touchstart", 5); await touch("touchmove", 180); await page.waitForTimeout(300);
  const r = await page.evaluate(() => {
    const o = [];
    for (const [x, y] of [[250, 303], [300, 303], [330, 303], [260, 200]]) {
      const top = document.elementFromPoint(x, y);
      const chat = document.querySelector(".cfm");
      o.push(`(${x},${y}) top=<${top.tagName.toLowerCase()} class="${top.className}"> "${top.textContent.trim().slice(0, 20)}" inChat=${!!top.closest(".cfm")} inList=${!!top.closest(".sdb")}`);
    }
    return o;
  });
  r.forEach(l => console.log(mode, l));
  await page.screenshot({ path: `${out}/390-chat-swipe-${mode}-zoom.png`, clip: { x: 170, y: 260, width: 220, height: 90 } });
  await touch("touchend", 0);
  await page.close();
}
await browser.close();
