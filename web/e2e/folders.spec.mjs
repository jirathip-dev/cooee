// Folders (#31): Settings › Folders browses the home folder (serve.sh: the temp folder) by folder names only, hidden
// ones behind a toggle, and adds or removes a root; the new-thread picker's search finds a sub-folder 2 levels in,
// and [+ new folder] makes one inside a root and picks it. Each viewport uses its own folders.
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const shot = (page, info, name) => page.screenshot({ path: `../../ui-${info.project.name}/${name}.png` });

async function audit(page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)).toBeLessThanOrEqual(0);
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
}

test.beforeEach(async ({ page }) => { await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" }); });

test("Settings › Folders: browse from home, add a root and remove it", async ({ page }, info) => {
  const folder = info.project.name === "390" ? "schemes" : "shed";
  await page.goto("/");
  await page.getByRole("button", { name: "settings" }).click();
  await page.locator("#sheet").getByRole("button", { name: "roots" }).click();
  const sheet = page.locator("#sheet .sh:popover-open");
  await expect(sheet.locator(".hd b")).toHaveText("folders");
  await expect(sheet.locator(".lrw .sk").first()).toHaveText("~/root");
  await expect(sheet.locator("[data-go]", { hasText: folder })).toBeVisible();
  // Folder names only: no config.toml, no hidden folder until the toggle.
  await expect(sheet.getByText("config.toml")).toHaveCount(0);
  await expect(sheet.getByText(".hidden-folder")).toHaveCount(0);
  await sheet.getByRole("radio", { name: "on" }).click();
  await expect(sheet.getByText(".hidden-folder")).toBeVisible();
  await sheet.getByRole("radio", { name: "off" }).click();
  await expect(sheet.getByText(".hidden-folder")).toHaveCount(0);
  await shot(page, info, "folders");
  await audit(page);

  await sheet.locator("[data-go]", { hasText: folder }).click();
  await expect(sheet.locator(".lrw b.sk")).toHaveText(`~/${folder}`);
  await sheet.getByRole("button", { name: "add as root" }).click();
  await expect(sheet.locator(".lrw .sk", { hasText: `~/${folder}` })).toHaveCount(2);
  await shot(page, info, "folders-added");
  // Closing reloads the page: the picker lists the new root.
  await sheet.getByRole("button", { name: "close" }).click();
  await page.waitForLoadState("load");
  await page.goto("/#new");
  await expect(page.getByRole("radio", { name: `~/${folder}`, exact: true })).toBeAttached();

  await page.getByRole("button", { name: "settings" }).click();
  await page.locator("#sheet").getByRole("button", { name: "roots" }).click();
  page.once("dialog", (d) => d.accept());
  await sheet.locator(".lrw", { hasText: `~/${folder}` }).first().getByRole("button", { name: "remove" }).click();
  await expect(sheet.locator(".lrw .sk", { hasText: `~/${folder}` })).toHaveCount(1);
});

test("new-thread picker: search finds a sub-folder, [+ new folder] makes one in a root and picks it", async ({ page }, info) => {
  const name = `new-${info.project.name}`;
  await page.goto("/#new");
  const deep = page.getByRole("radio", { name: "~/root/project/deep-src" });
  await expect(deep).toBeHidden();
  await page.getByLabel("search folders").fill("deep");
  await expect(deep).toBeVisible();
  await expect(page.getByRole("radio", { name: "~/root", exact: true })).toBeHidden();
  await shot(page, info, "picker-search");
  await page.getByLabel("search folders").fill("");
  await page.getByRole("radio", { name: "~/root", exact: true }).check();
  page.once("dialog", (d) => d.accept(name));
  await page.getByRole("button", { name: "+ new folder" }).click();
  await expect(page.getByRole("radio", { name: `~/root/${name}` })).toBeChecked();
  await expect(page.locator("#new-err")).toBeHidden();
  await shot(page, info, "picker-new-folder");
  await audit(page);
});
