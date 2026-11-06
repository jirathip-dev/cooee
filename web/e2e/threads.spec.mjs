import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

test("thread list: renders, fits the width, no axe violations", async ({ page }, info) => {
  await page.goto("/");
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("cooee");
  await expect(page.locator(".fc .tr").first()).toBeVisible();
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  await page.screenshot({ path: `../../ui-${info.project.name}/threads.png`, fullPage: true });
  await page.getByRole("link", { name: "new thread" }).click();
  await expect(page.locator("#new")).toBeVisible();
  await expect(page.getByRole("radio", { name: "fake-acp" })).toBeChecked();
  // The account picker, the default (the system default here) preselected.
  await expect(page.getByLabel("fake-acp account")).toHaveValue("");
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
  await page.goto("/#new");
  // No agent picked: the server refuses the request's JSON.
  await page.locator("#new input[name=agent]").evaluateAll((rs) => rs.forEach((r) => { r.checked = false; }));
  await page.getByLabel("first message").fill("hi");
  await page.getByRole("button", { name: "start" }).click();
  await expect(page.locator("#new-err")).toHaveText("cooee couldn't do that (error 400): reload and try again.");
  // cooee's own refusal is shown as it is.
  await page.locator("#new input[name=agent]").first().check();
  await page.locator("#new input[name=folder]").first().evaluate((r) => { r.value = "/"; });
  await page.getByRole("button", { name: "start" }).click();
  await expect(page.locator("#new-err")).toHaveText("/ is not inside a configured root");
});
