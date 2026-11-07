// Agent profiles (#40): the new-thread form's picker (serve.sh: the `reviewer` profile beside the config) starts on
// Plain; picking a profile hides the agent picker; its thread's header shows the profile's label; and a folder picked
// picks its last-used profile. Each viewport's threads go in its own ~/root/project/prof-<viewport> (serve.sh).
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const shot = (page, info, name) => page.screenshot({ path: `../../ui-${info.project.name}/${name}.png` });

async function audit(page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)).toBeLessThanOrEqual(0);
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
}

test.beforeEach(async ({ page }) => { await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" }); });

test("the profile picker: Plain first, a profile hides the agents, its thread says its label, a folder picks its last", async ({ page }, info) => {
  await page.goto("/#new");
  const form = page.locator("#new");
  const picker = form.getByRole("combobox", { name: "profile" });
  const agents = form.locator("fieldset.ag");
  const own = `prof-${info.project.name}`;
  const mine = form.getByRole("radio", { name: `~/root/project/${own}` });
  await page.getByLabel("search folders").fill(own);
  await mine.check();
  await expect(picker).toHaveValue("");
  await expect(agents).toBeVisible();
  await shot(page, info, "profile-plain");
  await audit(page);

  await picker.selectOption({ label: "Reviewer" });
  await expect(agents).toBeHidden();
  await shot(page, info, "profile-picked");
  await audit(page);

  await form.getByLabel("first message").fill("hi");
  await form.getByRole("button", { name: "start" }).click();
  await page.waitForURL(/\/t\//);
  await expect(page.locator(".th .ds")).toContainText("Reviewer · fake-acp · ");
  await expect(page.locator("#msgs .blk.me")).toHaveText(/hi/);
  await shot(page, info, "profile-thread");

  // Back on the form: its folder picks Reviewer, another folder Plain.
  await page.goto("/#new");
  await page.getByLabel("search folders").fill(own);
  await mine.check();
  await expect(picker).toHaveValue("reviewer");
  await page.getByLabel("search folders").fill("");
  await form.getByRole("radio", { name: "~/root/project", exact: true }).check();
  await expect(picker).toHaveValue("");
});
