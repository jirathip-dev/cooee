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
const clockAt = (night) => { const at = new Date(); const h = at.getHours();
  if (night && h >= 7 && h < 19) at.setHours(5);
  if (!night && (h < 7 || h >= 19)) at.setHours(h >= 19 ? 12 : -12); return at; };
for (const w of [390, 1440]) {
  const ctx = await browser.newContext({ baseURL: base, viewport: { width: w, height: w === 390 ? 844 : 900 }, storageState: `${out}/state.json` });
  const page = await ctx.newPage();
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
  await page.goto("/p/hello");
  await shot("plugin-hello", page.locator("iframe.pf"));
  await page.goto("/p/machines");
  await shot("plugin-machines", page.frameLocator("iframe.pf").locator("article.on"));
  await page.close();
  // #115: Scene in farm light/dark/night and kitchen light/dark.
  for (const [theme, mode, night] of [["farm","light",false],["farm","dark",false],["farm","dark",true],["kitchen","light",false],["kitchen","dark",false]]) {
    await ctx.clearCookies({ name: "cooee-look" });
    await ctx.addCookies([{ name: "cooee-look", value: `theme:${theme}/mode:${mode}/scene:on`, url: base }]);
    const sp = await ctx.newPage();
    await sp.emulateMedia({ colorScheme: mode, reducedMotion: "reduce" });
    await sp.clock.setFixedTime(clockAt(night));
    await sp.goto("/?list");
    await sp.locator("#scene .sb .sf use").first().waitFor();
    const hrefs = await sp.locator("#scene .sb .sf use").evaluateAll((us) => us.map((u) => u.getAttribute("href")));
    const states = await sp.locator("#scene .sb").evaluateAll((bs) => bs.map((b) => b.innerText.replace(/\s+/g, " ").trim()));
    const nightCls = await sp.locator("#scene").evaluate((s) => s.classList.contains("night"));
    const name = `scene-${theme}-${night ? "night" : mode}`;
    await sp.waitForTimeout(500);
    await sp.screenshot({ path: `${out}/${w}-${name}.png`, fullPage: true });
    console.log(`${w} ${name} ok night=${nightCls} hrefs=${JSON.stringify(hrefs)}`);
    console.log(`  cards=${JSON.stringify(states)}`);
    if (w === 390 && mode === "dark" && !night) {
      const svg = await (await sp.request.get(`/look/${theme}/sprites.svg`)).text();
      for (const k of ["edit","idle","done","needs","merged","fail"]) {
        const sym = new RegExp(`<symbol id="s-scene-${k}" viewBox="([^"]*)">(.*?)</symbol>`).exec(svg);
        console.log(`  ${theme} s-scene-${k}: viewBox=${sym?.[1]} row=${/class="row /.test(sym?.[2] ?? "")} crispEdges=${(sym?.[2] ?? "").includes("crispEdges")} figure=${/class="(horse|chef|moment)[^"]*"/.exec(sym?.[2] ?? "")?.[0]}`);
      }
      // A gallery of every Scene pose, drawn from the served sheet at Scene size, beside the old s-<k> row poses.
      await sp.evaluate((theme) => {
        const ns = "http://www.w3.org/2000/svg";
        const d = document.createElement("div");
        d.id = "gal"; d.style.cssText = "position:fixed;inset:0;z-index:99999;background:#fff;display:flex;flex-wrap:wrap;gap:8px;padding:8px;align-content:flex-start;color:#000;font:12px sans-serif";
        for (const pre of ["s-scene-", "s-"]) for (const k of ["edit","idle","done","needs","merged","fail"]) {
          const f = document.createElement("figure"); f.style.margin = "0";
          const s = document.createElementNS(ns, "svg"); s.setAttribute("width", "132"); s.setAttribute("height", "100");
          const u = document.createElementNS(ns, "use"); u.setAttribute("href", `/look/${theme}/sprites.svg#${pre}${k}`);
          s.append(u); f.append(s); const c = document.createElement("figcaption"); c.textContent = pre + k; f.append(c); d.append(f);
        }
        document.body.append(d);
      }, theme);
      await sp.waitForTimeout(800);
      await sp.screenshot({ path: `${out}/${w}-poses-${theme}.png`, fullPage: false });
      console.log(`${w} poses-${theme} ok`);
    }
    await sp.close();
  }
  await ctx.close();
}
await browser.close();
