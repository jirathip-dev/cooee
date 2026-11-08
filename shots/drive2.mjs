import { chromium } from "@playwright/test";
const out = process.env.OUT, base = `http://127.0.0.1:${process.env.P}`;
const browser = await chromium.launch();
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
for (const w of [390, 1440]) {
  const ctx = await browser.newContext({ baseURL: base, viewport: { width: w, height: w === 390 ? 844 : 900 }, storageState: `${out}/state.json` });
  const page = await ctx.newPage();
  const mk = async (name) => {
    const r = await page.request.post("/agents", { form: { name, profile: "" }, maxRedirects: 0 });
    return new URL(r.headers().location, "http://x").searchParams.get("profile");
  };
  const start = async (profile, text, tag) => {
    const res = await page.request.post("/threads", { data: { agent: "", profile, text, random_id: `v96b-${w}-${tag}` } });
    const { id } = await res.json();
    for (let i = 0; i < 200; i++) {
      const t = (await (await page.request.get("/threads")).json()).find((t) => t.id === id);
      if (t?.state === "done") break;
      await sleep(100);
    }
    return id;
  };
  const readAll = async (id) => {
    const items = (await (await page.request.get(`/threads/${id}/messages`)).json()).items;
    const r = await page.request.post(`/threads/${id}/read`, { data: { item: items[items.length - 1].id } });
    return r.status();
  };
  // Calm: every chat read -> no titles; newest = most recent side thread.
  const C = `Calm ${w}`, M = `Mixed ${w}`;
  const pc = await mk(C);
  const cids = [];
  for (const [n, text] of ["C ongoing", "C side old", "C side new"].entries()) cids.push(await start(pc, text, `c${n}`));
  for (const id of cids) console.log(`${w} read ${id}`, await readAll(id));
  // Mixed: S1 unread, then S2 newer but read -> newest = S1 (the newest unread), not S2.
  const pm = await mk(M);
  const mids = [];
  for (const [n, text] of ["M ongoing", "M side unread", "M side read newer"].entries()) mids.push(await start(pm, text, `m${n}`));
  console.log(`${w} read`, await readAll(mids[0]), await readAll(mids[2]));
  const threads = (await (await page.request.get("/threads")).json());
  for (const id of [...cids, ...mids]) { const t = threads.find((t) => t.id === id); console.log(`${w} thread`, id, JSON.stringify({ title: t.title, unread: t.unread, at: t.at })); }
  const list = w === 390 ? "/?list" : `/t/long-${w}`;
  await page.goto(list);
  for (const [name, want] of [[C, cids[2]], [M, mids[1]]]) {
    await page.goto(list);
    const agent = page.locator(".ags .agr", { has: page.locator(".tt", { hasText: name }) });
    const ust = agent.locator("+ .ust");
    console.log(`${w} ${name} agent class=${await agent.getAttribute("class")} dot=${await agent.locator(".tt .ur").count()} ust=`, (await ust.textContent()).replace(/\s+/g, " ").trim());
    console.log(`${w} ${name} titles:`, JSON.stringify(await ust.locator(".str:not(.nw):not(.mo) .tt").allTextContents()));
    await ust.scrollIntoViewIfNeeded();
    if (name === C) await page.screenshot({ path: `${out}/${w}-calm-mixed.png` });
    await ust.getByRole("link", { name: `newest side thread of ${name}` }).click();
    await page.waitForURL(/\/t\//);
    const got = new URL(page.url()).pathname;
    console.log(`${w} newest(${name}) -> ${got} want /t/${want} ${got === `/t/${want}` ? "OK" : "MISMATCH"}`);
  }
  await ctx.close();
}
await browser.close();
