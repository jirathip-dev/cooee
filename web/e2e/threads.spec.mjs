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
  await page.screenshot({ path: `../../ui-${info.project.name}/threads-new.png`, fullPage: true });
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
});

test("installable: the manifest and icon load", async ({ page, request }) => {
  await page.goto("/");
  const href = await page.locator('link[rel="manifest"]').getAttribute("href");
  const res = await request.get(href);
  expect(res.headers()["content-type"]).toBe("application/manifest+json");
  const manifest = await res.json();
  expect(manifest).toMatchObject({ name: "cooee", start_url: "/", display: "standalone" });
  const icon = await request.get(manifest.icons[0].src);
  expect(icon.ok()).toBe(true);
  expect(icon.headers()["content-type"]).toBe("image/svg+xml");
});
