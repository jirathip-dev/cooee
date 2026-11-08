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
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
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

  // #96: an agent with 5 unread side threads + its ongoing chat; one with side threads all read.
  const mk = async (name) => {
    const r = await page.request.post("/agents", { form: { name, profile: "" }, maxRedirects: 0 });
    return new URL(r.headers().location, "http://x").searchParams.get("profile");
  };
  const start = async (profile, text, tag) => {
    const res = await page.request.post("/threads", { data: { agent: "", profile, text, random_id: `v96-${w}-${tag}` } });
    if (!res.ok()) throw new Error(`start ${text}: ${res.status()}`);
    const { id } = await res.json();
    const want = text === "approve an edit" ? "blocked" : "done";
    for (let i = 0; i < 200; i++) {
      const t = (await (await page.request.get("/threads")).json()).find((t) => t.id === id);
      if (t?.state === want) break;
      await sleep(100);
    }
    return id;
  };
  const A = `Busy ${w}`, B = `Quiet ${w}`;
  const pa = await mk(A);
  const ids = [];
  for (const [n, text] of ["Ongoing", "Topic 1", "Topic 2", "Topic 3", "Topic 4", "approve an edit"].entries()) ids.push(await start(pa, text, `a${n}`));
  const pb = await mk(B);
  const qids = [];
  for (const [n, text] of ["Q ongoing", "Q side 1", "Q side 2"].entries()) qids.push(await start(pb, text, `b${n}`));
  // Read every Quiet thread so nothing is unread.
  for (const id of qids) { await page.goto(`/t/${id}`); await page.getByLabel("message", { exact: true }).waitFor(); await sleep(400); }
  const threads = (await (await page.request.get("/threads")).json());
  for (const id of [...ids, ...qids]) { const t = threads.find((t) => t.id === id); console.log(`${w} thread`, id, JSON.stringify({ title: t.title, state: t.state, unread: t.unread, at: t.at })); }

  const dump = async (label) => {
    const rows = await page.locator(".ags").evaluate((nav) => {
      const o = [];
      for (const el of nav.querySelectorAll(".agr, .ust, .sth")) {
        if (el.classList.contains("agr")) o.push(`AGR[${el.className}] ${el.querySelector(".tt")?.textContent} dot=${!!el.querySelector(".tt .ur")} chs=${!!el.querySelector(".chs")}`);
        else o.push(`  ${el.className}: ` + [...el.children].map((c) => `[${c.className}${c.querySelector(".tt .ur") ? " •" : ""}${c.querySelector(".chs") ? " choose" : ""}] ${c.textContent.replace(/\s+/g, " ").trim()}`).join(" | "));
      }
      return o.join("\n");
    });
    console.log(`--- ${w} ${label}\n${rows}`);
  };
  const styles = [["modern", {}], ["terminal", { style: "terminal" }], ["light", { theme: "kitchen", mode: "light" }], ["scene", { scene: "on" }]];
  for (const [name, pick] of styles) {
    await ctx.clearCookies({ name: "cooee-look" });
    if (Object.keys(pick).length) await ctx.addCookies([{ name: "cooee-look", value: Object.entries(pick).map(([k, v]) => `${k}:${v}`).join("/"), url: base }]);
    await page.goto(w === 390 ? "/?list" : `/t/long-${w}`);
    await page.locator(".ags").waitFor();
    console.log(`${w} ${name} html style=${await page.locator("html").getAttribute("data-style")} theme=${await page.locator("html").getAttribute("data-theme")}`);
    if (name !== "scene") {
      await dump(name);
      const busy = page.locator(".ags .agr", { has: page.locator(".tt", { hasText: A }) });
      await busy.locator("+ .ust").scrollIntoViewIfNeeded();
      await page.screenshot({ path: `${out}/${w}-unread-${name}.png` });
      await page.screenshot({ path: `${out}/${w}-unread-${name}-full.png`, fullPage: true });
    } else {
      const chips = await page.locator(".scc .chip").allTextContents().catch(() => []);
      console.log(`${w} scene chips:`, JSON.stringify(chips), "ust count", await page.locator(".ust").count());
      await page.screenshot({ path: `${out}/${w}-unread-scene.png`, fullPage: true });
    }
    console.log(`${w} unread-${name} ok`);
  }
  await ctx.clearCookies({ name: "cooee-look" });
  await page.goto(w === 390 ? "/?list" : `/t/long-${w}`);
  const busyUst = page.locator(".ags .agr", { has: page.locator(".tt", { hasText: A }) }).locator("+ .ust");
  await busyUst.getByRole("link", { name: `newest side thread of ${A}` }).click();
  await page.waitForURL(/\/t\//);
  console.log(`${w} newest(${A}) ->`, new URL(page.url()).pathname, "want", `/t/${ids[5]}`, new URL(page.url()).pathname === `/t/${ids[5]}` ? "OK" : "MISMATCH");
  await page.screenshot({ path: `${out}/${w}-newest-busy.png`, fullPage: false });
  await page.goto(w === 390 ? "/?list" : `/t/long-${w}`);
  const quietUst = page.locator(".ags .agr", { has: page.locator(".tt", { hasText: B }) }).locator("+ .ust");
  console.log(`${w} quiet titles:`, await quietUst.locator(".str:not(.nw):not(.mo) .tt").count());
  await quietUst.getByRole("link", { name: `newest side thread of ${B}` }).click();
  await page.waitForURL(/\/t\//);
  console.log(`${w} newest(${B}) ->`, new URL(page.url()).pathname, "want", `/t/${qids[2]}`, new URL(page.url()).pathname === `/t/${qids[2]}` ? "OK" : "MISMATCH");
  // After opening the newest unread, Busy's list should drop it.
  await page.goto(w === 390 ? "/?list" : `/t/long-${w}`);
  await page.locator(".ags").waitFor();
  await dump("after opening newest");
  // Busy on show: its side threads (sth) rows with the dot/choose, and newest.
  await page.goto(`/t/${ids[0]}`);
  await page.getByLabel("message", { exact: true }).waitFor();
  if (w === 390) await page.goto(`/?list`);
  await dump("busy on show");
  await page.screenshot({ path: `${out}/${w}-busy-on-show.png`, fullPage: true });
  await ctx.close();
}
await browser.close();
