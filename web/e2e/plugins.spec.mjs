// Plugin tabs (docs/plugins.md) with the example plugin (docs/plugins/hello), run by serve.sh as `hello` and `other`:
// the tab's module runs in a sandboxed frame, reaches its backend with the plugin's token (never the cookie), gets its
// event stream and the theme, posts a push and a choice card whose answer comes back; it reaches nothing else.
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const frameOf = (page, name) => page.frames().find((f) => f.url().includes(`/p/${name}/frame`));

test("the example plugin's tab: proxied calls with its token, its stream, a push and a choice card", async ({ page, context }, info) => {
  await page.goto("/new");
  await page.getByLabel("message", { exact: true }).fill("Say hello");
  await page.getByRole("button", { name: "send" }).click();
  await page.waitForURL(/\/t\//);
  const thread = new URL(page.url()).pathname.split("/")[2];

  await page.goto("/");
  await page.getByRole("navigation", { name: "tabs" }).getByRole("link", { name: "Hello" }).click();
  await expect(page).toHaveURL(/\/p\/hello$/);
  // The thread's id as the tab's hash: a deep link, as a push's tap opens.
  await page.goto("/");
  await page.goto(`/p/hello#${thread}`);
  const tab = page.frameLocator("iframe.pf");
  await expect(tab.getByText(/^whoami/)).toHaveText('whoami 200: {"token":"ok","cookie":null}');
  await expect(tab.getByText('hello: "the stream is open"').first()).toBeVisible();
  const bg = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--bg").trim());
  await expect.poll(() => frameOf(page, "hello").evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--bg"))).toBe(bg);
  await page.screenshot({ path: `../../ui-${info.project.name}/plugin.png` });
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);

  // The backend posts a push and a choice card with its token; cooee answers each 200.
  await tab.getByRole("button", { name: "notify" }).click();
  await expect(tab.getByRole("listitem").filter({ hasText: /^notify/ })).toHaveText("notify 200: 200");
  await tab.getByRole("button", { name: "ask" }).click();
  await expect(tab.getByRole("listitem").filter({ hasText: /^ask/ })).toHaveText("ask 200: 200");

  // The card is in the thread; the owner's tap goes back to the plugin, whose stream tells the open tab.
  const chat = await context.newPage();
  await chat.goto(`/t/${thread}`);
  const card = chat.locator(".blk.ap", { hasText: "Ship it?" });
  await expect(card.locator(".k")).toHaveText("hello");
  await card.getByRole("button", { name: "yes" }).click();
  await expect(card.locator(".ans")).toHaveText(/^✓ yes · .+ · \d{1,2}:\d\d/);
  await expect(tab.getByText('choice: "yes"').first()).toBeVisible();
});

test("a plugin reaches only its own backend: not another plugin's, not cooee's storage or page", async ({ page, request }) => {
  await page.goto("/p/other");
  await expect(page.frameLocator("iframe.pf").getByText(/^whoami/)).toHaveText('whoami 200: {"token":"ok","cookie":null}');
  const tried = await frameOf(page, "other").evaluate(async () => {
    const thrown = (f) => { try { return f(); } catch (e) { return e.name; } };
    return {
      escape: (await cooee.fetch("/../../hello/api/whoami")).status,
      encoded: (await cooee.fetch("/%2e%2e/%2e%2e/hello/api/whoami")).status,
      direct: await fetch("/p/hello/api/whoami").then((r) => r.status, (e) => e.name),
      storage: thrown(() => localStorage.length),
      cookie: thrown(() => document.cookie),
      parent: thrown(() => parent.document.title),
      origin,
    };
  });
  expect(tried).toEqual({ escape: 403, encoded: 403, direct: "TypeError", storage: "SecurityError", cookie: "SecurityError", parent: "SecurityError", origin: "null" });
  // A backend without the plugin's token is refused, even from a paired browser.
  expect((await request.post("/plugin/push", { data: { text: "x" } })).status()).toBe(401);
  expect((await request.post("/plugin/push", { data: { text: "x" }, headers: { authorization: "Bearer not-the-token-0123" } })).status()).toBe(401);
});
