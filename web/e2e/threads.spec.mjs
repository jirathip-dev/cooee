import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

test("thread list: renders, fits the width, no axe violations", async ({ page }, info) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("cooee");
  await expect(page.locator(".fc .tr").first()).toBeVisible();
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  await page.screenshot({ path: `../../ui-${info.project.name}/threads.png`, fullPage: true });
  await page.goto("/new");
  await expect(page.locator("#nc")).toBeVisible();
  // The provider and account last used (or its agent profile's label, #40), never the agent's id; once login.spec has
  // signed in the main login, the seeded threads with no account count as its.
  await expect(page.locator("#chip-agent")).toHaveText(/^(fake-390 · (\S+@example\.com|main login)|Reviewer)/);
  await page.screenshot({ path: `../../ui-${info.project.name}/threads-new.png`, fullPage: true });
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
  // No agent picked: the server refuses the request's JSON.
  const picks = page.locator("#nc input[name=pick]");
  const was = await picks.evaluateAll((rs) => rs.findIndex((r) => r.checked));
  await picks.evaluateAll((rs) => rs.forEach((r) => { r.checked = false; }));
  await page.getByLabel("first message").fill("hi");
  await page.getByRole("button", { name: "send" }).click();
  await expect(page.locator("#new-err")).toHaveText("cooee couldn't do that (error 400): reload and try again.");
  // cooee's own refusal is shown as it is.
  await picks.nth(was).evaluate((r) => { r.checked = true; });
  await page.locator("#nc input[name=folder]").first().evaluate((r) => { r.value = "/"; });
  await page.getByRole("button", { name: "send" }).click();
  await expect(page.locator("#new-err")).toHaveText("/ is not inside a configured root");
});
