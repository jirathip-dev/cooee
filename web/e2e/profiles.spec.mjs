// Agent profiles (#40) in the composer (#48): the agent sheet (serve.sh: the `reviewer` profile beside the config) lists
// the profiles, then Plain's accounts; picking one sets the chip to its label; its thread's header shows the label; and
// a folder picked picks its last-used profile. Each viewport's threads go in its own ~/root/project/prof-<viewport>
// (serve.sh).
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const shot = (page, info, name) => page.screenshot({ path: `../../ui-${info.project.name}/${name}.png` });

async function audit(page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)).toBeLessThanOrEqual(0);
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
}

test.beforeEach(async ({ page }) => { await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" }); });

test("the agent sheet: profiles, then Plain's accounts; a profile's thread says its label; a folder picks its last", async ({ page }, info) => {
  const own = `prof-${info.project.name}`;
  const chip = page.locator("#chip-agent"), sheet = page.locator("#sh-agent");
  const folder = async (q, name) => {
    await page.locator("#chip-folder").click();
    await page.getByLabel("search folders").fill(q);
    await page.locator("#sh-folder label", { has: page.locator(".sk", { hasText: new RegExp(`^${name}$`) }) }).click();
    await expect(page.locator("#sh-folder")).toBeHidden();
  };
  await page.goto("/new");
  await folder(own, `~/root/project/${own}`);
  await chip.click();
  await expect(sheet.locator(".ch").first()).toHaveText("profiles");
  await expect(sheet.locator(".pk .sk").first()).toHaveText("Reviewer");
  await shot(page, info, "profile-sheet");
  await audit(page);

  await sheet.locator("label", { hasText: "Reviewer" }).click();
  await expect(sheet).toBeHidden();
  await expect(chip).toHaveText(/^Reviewer/);
  await shot(page, info, "profile-picked");
  await audit(page);

  await page.getByLabel("first message").fill("hi");
  await page.getByRole("button", { name: "send" }).click();
  await page.waitForURL(/\/t\//);
  await expect(page.locator(".th .ds")).toContainText("Reviewer · ");
  await expect(page.locator("#msgs .blk.me")).toHaveText(/hi/);
  await shot(page, info, "profile-thread");

  // Back on the composer: its folder picks Reviewer, another folder a Plain account.
  await page.goto("/new");
  await folder(own, `~/root/project/${own}`);
  await expect(chip).toHaveText(/^Reviewer/);
  await folder("", "~/root/project");
  await expect(chip).not.toHaveText(/^Reviewer/);
});
