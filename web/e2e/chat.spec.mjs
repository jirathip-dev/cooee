import net from "node:net";
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const shot = (page, info, name) => page.screenshot({ path: `../../ui-${info.project.name}/${name}.png` });

async function start(page, text, base = "") {
  await page.goto(`${base}/new`);
  await page.getByLabel("message", { exact: true }).fill(text);
  await page.getByRole("button", { name: "send" }).click();
  await page.waitForURL(/\/t\//);
}

async function say(page, text) {
  await page.getByLabel("message", { exact: true }).fill(text);
  await page.getByRole("button", { name: "send" }).click();
}

test("each event kind renders: reply, tool call, plan, approval, state, stop", async ({ page }, info) => {
  await start(page, "Say hello");
  const msgs = page.locator("#msgs");
  await expect(msgs.locator(".blk.me").first()).toHaveText(/Say hello/);
  await expect(msgs.locator(".blk.ai").first()).toContainText("Hello");
  await expect(page.locator("#st")).toHaveAttribute("data-st", "done");
  await expect(page.locator("#st")).toHaveText("in the stable");

  await say(page, "use a tool");
  const tool = msgs.locator(".blk.run", { hasText: "Read src/lib.rs" });
  await expect(tool).toHaveCount(1);
  await expect(tool.locator(".ts")).toHaveText("✓");
  await expect(msgs.locator(".blk.ai").last()).toContainText("Read it.");

  await say(page, "make a plan");
  await expect(msgs.locator(".blk.plan li")).toHaveText(["Read the code", "Fix the bug"]);

  await say(page, "approve an edit");
  const ask = msgs.locator(".blk.ap");
  await expect(ask.getByRole("button")).toHaveText(["Allow", "Always allow", "Reject"]);
  await expect(page.locator("#st")).toHaveText("at the gate");
  await expect(page.locator("#wait")).toHaveText("at the gate · needs you");
  await expect(page.locator("#wait")).toBeVisible();
  await shot(page, info, "chat-approval");
  const taps = [];
  page.on("request", (r) => { if (r.url().endsWith("/approve")) taps.push(r); });
  await ask.getByRole("button", { name: "Allow", exact: true }).dblclick();
  await expect(ask.locator(".ans")).toHaveText(/^✓ Allow · .+ · \d{1,2}:\d\d/);
  expect(taps).toHaveLength(1);
  await expect(msgs.locator(".blk.ai").last()).toContainText("Edited.");

  await say(page, "hang on");
  await expect(page.locator("#pill")).toBeVisible();
  await expect(msgs.locator(".blk.run", { hasText: "cargo test" }).locator(".ts")).toHaveText(/⋯ \d+:\d\d/);
  await expect(msgs.locator(".blk.live")).toContainText("w0");
  await shot(page, info, "chat-working");
  await page.locator("#pill .stop").click();
  await expect(msgs.locator(".blk.ai").last()).toContainText("cancelled");
  await expect(msgs.locator(".blk.run", { hasText: "cargo test" }).locator(".ts")).toHaveText("✕ cancelled");
  await expect(page.locator("#pill")).toBeHidden();

  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  await shot(page, info, "chat");
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
});

test("a reconnect mid-stream loses and doubles no text", async ({ page }, info) => {
  test.setTimeout(90_000);
  const sockets = new Set();
  const proxy = net.createServer((client) => {
    const up = net.connect(new URL(info.project.use.baseURL).port, "127.0.0.1");
    for (const [a, b] of [[client, up], [up, client]]) {
      a.pipe(b);
      a.on("error", () => b.destroy());
      a.on("close", () => b.destroy());
    }
    sockets.add(client);
    client.on("close", () => sockets.delete(client));
  });
  await new Promise((ok) => proxy.listen(0, "127.0.0.1", ok));
  const streams = [];
  page.on("request", async (r) => {
    if (r.url().includes("/events")) streams.push((await r.allHeaders())["last-event-id"]);
  });
  await start(page, "Say hello", `http://127.0.0.1:${proxy.address().port}`);
  await expect(page.locator(".blk.ai")).toContainText("Hello");
  await say(page, "slow 150");
  const live = page.locator(".blk.live .md");
  await expect(live).toContainText("w20 ");
  for (const s of sockets) s.destroy();
  await expect(live).toContainText("w149 ", { timeout: 60_000 });
  expect(streams.length).toBeGreaterThanOrEqual(2);
  expect(streams.at(-1)).toMatch(/^[0-9a-f]+\.\d+$/);
  const want = Array.from({ length: 150 }, (_, i) => `w${i} `).join("");
  expect(await live.textContent()).toBe(want);
  await page.locator(".blk.live .stop").click();
  await expect(page.locator(".blk.ai").last()).toContainText("streamed 150 words");
  await expect(page.locator(".blk.live")).toHaveCount(0);
  proxy.close();
});

test("a failed send shows retry, and the retry sends it", async ({ page }) => {
  await start(page, "Say hello");
  await expect(page.locator(".blk.ai")).toContainText("Hello");
  await page.route("**/send", (route) => route.abort());
  await say(page, "once more");
  const row = page.locator(".blk.me.failed");
  await expect(row).toContainText("once more");
  await page.unroute("**/send");
  await row.getByRole("button", { name: /retry/ }).click();
  await expect(page.locator(".blk.me.failed")).toHaveCount(0);
  await expect(page.locator(".blk.ai").last()).toContainText("Hello");
  await expect(page.locator(".blk.me")).toHaveCount(2);
});

test("the draft survives a reload", async ({ page }) => {
  await start(page, "Say hello");
  await page.getByLabel("message", { exact: true }).fill("half a thought");
  await page.waitForTimeout(1300);
  await page.reload();
  await expect(page.getByLabel("message", { exact: true })).toHaveValue("half a thought");
});

test("1,000 messages: a window around the anchor, no long tasks, the anchor kept", async ({ page }, info) => {
  await page.goto(`/t/long-${info.project.name}`);
  await expect(page.locator("#msgs .unread")).toBeVisible();
  await expect(page.locator(".unread + li")).toContainText("#991");
  await shot(page, info, "chat-long");

  const list = page.locator("#msgs");
  const anchor = () => list.evaluate((l) => {
    const r = [...l.children].find((r) => r.dataset.id && r.offsetTop + r.offsetHeight > l.scrollTop);
    return { id: r.dataset.id, top: Math.round(r.getBoundingClientRect().top) };
  });
  const minId = () => list.evaluate((l) => Math.min(...[...l.children].filter((r) => r.dataset.id).map((r) => +r.dataset.id)));
  const trace = await page.context().newCDPSession(page);
  const events = [];
  trace.on("Tracing.dataCollected", ({ value }) => events.push(...value));
  await trace.send("Tracing.start", { categories: "disabled-by-default-devtools.timeline", transferMode: "ReportEvents" });
  const held = [];
  await page.route(/\/messages\?around=/, (route) => held.push(route));
  for (let i = 0; i < 100 && (await minId()) > 1; i++) {
    const first = await minId();
    await list.evaluate((l) => { l.scrollTop = Math.max(1, l.clientHeight * 1.5); });
    await expect.poll(() => held.length).toBeGreaterThan(0);
    const before = await anchor();
    await held.shift().continue();
    await expect.poll(minId).toBeLessThan(first);
    expect(await anchor()).toEqual(before);
  }
  await page.unroute(/\/messages\?around=/);
  for (const r of held) await r.continue();
  expect(await minId()).toBe(1);
  expect(await list.locator("li").count()).toBeLessThan(200);

  const size = page.viewportSize();
  const before = await anchor();
  await page.setViewportSize({ width: size.width, height: size.height - 300 });
  await page.waitForTimeout(100);
  expect(await anchor()).toEqual(before);
  const cdp = await page.context().browser().newBrowserCDPSession();
  const { processInfo } = await cdp.send("SystemInfo.getProcessInfo");
  const renderers = processInfo.filter((p) => p.type === "renderer").map((p) => p.id);
  await page.evaluate(() => setTimeout(() => { const end = performance.now() + 40; while (performance.now() < end); }));
  await new Promise((r) => setTimeout(r, 15));
  for (const pid of renderers) process.kill(pid, "SIGSTOP");
  await new Promise((r) => setTimeout(r, 100));
  for (const pid of renderers) process.kill(pid, "SIGCONT");
  await page.waitForTimeout(100);
  const traced = new Promise((r) => trace.once("Tracing.tracingComplete", r));
  await trace.send("Tracing.end");
  await traced;
  const main = new Set(events.filter((e) => e.name === "thread_name" && e.args.name === "CrRendererMain").map((e) => `${e.pid}.${e.tid}`));
  const tasks = events.filter((e) => e.name === "RunTask" && main.has(`${e.pid}.${e.tid}`));
  expect(tasks.length).toBeGreaterThan(100);
  expect(tasks.map((e) => Math.round(e.tdur / 1000)).filter((ms) => ms > 50)).toEqual([]);
});
