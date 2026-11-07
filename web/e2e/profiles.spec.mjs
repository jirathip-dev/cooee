// Agent profiles (#40) as agents (#86): the sidebar lists Plain on the agent and each profile (serve.sh: `reviewer`
// beside the config); a new chat with Reviewer runs as its profile and its bar says so. Each viewport's chats go in
// its own ~/root/project/prof-<viewport> (serve.sh).
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const shot = (page, info, name) => page.screenshot({ path: `../../ui-${info.project.name}/${name}.png` });

async function audit(page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)).toBeLessThanOrEqual(0);
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
}

test.beforeEach(async ({ page }) => { await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" }); });

test("a profile is an agent: listed, a new chat with it runs as it, its bar says its label", async ({ page }, info) => {
  await page.goto("/?list");
  const agents = page.locator(".ags");
  await expect(agents.locator(".agr .tt", { hasText: "Reviewer" })).toBeVisible();
  await page.goto("/new?profile=reviewer");
  await expect(page.locator(".hi")).toContainText("New chat with Reviewer");
  await expect(page.locator(".th .ds")).toContainText("Reviewer · reviewer · scratch");
  await expect(page.locator(".agr.on .tt")).toHaveText("Reviewer");
  await shot(page, info, "profile-new");
  await audit(page);

  const own = `prof-${info.project.name}`;
  await page.getByRole("button", { name: "folder: scratch ▾" }).click();
  await page.getByLabel("search folders").fill(own);
  await page.getByRole("radio", { name: new RegExp(`^${own}`) }).check();
  await page.getByLabel("message", { exact: true }).fill("hi");
  await page.getByRole("button", { name: "send" }).click();
  await page.waitForURL(/\/t\//);
  await expect(page.locator(".th .ds")).toContainText("Reviewer · fake-acp · ");
  await expect(page.locator(".th .ds")).toContainText(own);
  await expect(page.locator("#msgs .blk.me")).toHaveText(/hi/);
  await expect(page.locator(".agr.on .tt")).toHaveText("Reviewer");
  await shot(page, info, "profile-thread");
});
