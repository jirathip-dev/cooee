import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

test("agents: the list renders, fits the width, no axe violations; + opens a new chat with nothing to pick", async ({ page }, info) => {
  await page.goto("/?list");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("cooee");
  await expect(page.locator(".ags .agr").first()).toBeVisible();
  await expect(page.locator(".ags .agr .av").first()).toBeVisible();
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  await page.screenshot({ path: `../../ui-${info.project.name}/agents.png`, fullPage: true });
  await page.getByRole("link", { name: "new chat" }).first().click();
  await page.waitForURL(/\/new\?/);
  await expect(page.locator("#new")).toBeVisible();
  await expect(page.locator(".hi")).toContainText("New chat with");
  await expect(page.getByRole("button", { name: "folder: scratch ▾" })).toBeVisible();
  await expect(page.getByRole("textbox", { name: "message", exact: true })).toBeFocused();
  await page.screenshot({ path: `../../ui-${info.project.name}/new-chat.png`, fullPage: true });
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
});

test("installable: the manifest, its icons and the service worker load", async ({ page, request }) => {
  await page.goto("/");
  const href = await page.locator('link[rel="manifest"]').getAttribute("href");
  const res = await request.get(href);
  expect(res.headers()["content-type"]).toBe("application/manifest+json");
  const manifest = await res.json();
  expect(manifest).toMatchObject({ name: "cooee", start_url: "/", display: "standalone" });
  for (const { src, type } of manifest.icons) {
    const icon = await request.get(src);
    expect(icon.ok()).toBe(true);
    expect(icon.headers()["content-type"]).toBe(type);
  }
  expect(manifest.icons.map((i) => i.sizes)).toEqual(expect.arrayContaining(["192x192", "512x512"]));
  expect(await page.evaluate(async () => (await navigator.serviceWorker.ready).active.scriptURL)).toMatch(/\/sw\.js$/);
});

test("a failed start reads as one plain sentence, never the server's JSON error", async ({ page }) => {
  await page.goto("/new");
  // No message field: the server refuses the request's JSON.
  await page.locator("#new textarea").evaluate((t) => { t.name = "x"; });
  await page.getByLabel("message", { exact: true }).fill("hi");
  await page.getByRole("button", { name: "send" }).click();
  await expect(page.locator("#new-err")).toHaveText("cooee couldn't do that (error 400): reload and try again.");
  // cooee's own refusal is shown as it is.
  await page.goto("/new");
  await page.locator("#new input[name=folder]").nth(1).evaluate((r) => { r.value = "/"; r.checked = true; });
  await page.getByLabel("message", { exact: true }).fill("hi");
  await page.getByRole("button", { name: "send" }).click();
  await expect(page.locator("#new-err")).toHaveText("/ is not inside a configured root");
});

test("the current agent's row reads at AA while its chat works, on the light themes", async ({ page, context, baseURL }, info) => {
  test.skip(info.project.name !== "1440", "the sidebar shows beside a chat from 1000 px");
  const r = await page.request.post("/agents", { form: { name: `Busy ${info.project.name}`, profile: "" }, maxRedirects: 0 });
  await page.goto(r.headers().location);
  await page.getByLabel("message", { exact: true }).fill("hang on");
  await page.getByRole("button", { name: "send" }).click();
  await page.waitForURL(/\/t\//);
  await expect(page.locator("#pill")).toBeVisible();
  const row = page.locator(".ags .agr.on");
  for (const theme of ["farm", "kitchen", "garden"]) {
    await context.addCookies([{ name: "cooee-look", value: `theme:${theme}/mode:light`, url: baseURL }]);
    await page.reload();
    await expect(row.locator('[data-st="working"]')).toBeVisible();
    const { violations } = await new AxeBuilder({ page }).include(".ags").withRules(["color-contrast"]).analyze();
    expect(violations.map((v) => `${theme} ${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
  }
  await page.locator("#pill .stop").click();
  await expect(page.locator("#pill")).toBeHidden();
});
