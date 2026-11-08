// Folders (#31): Settings › Folders browses the home folder (serve.sh: the temp folder) by folder names only, hidden
// ones behind a toggle, and adds or removes a root; Use a folder…'s search (#86) finds a sub-folder 2 levels in, and
// [+ new folder] makes one inside a root and picks it. Each viewport uses its own folders.
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
  const reloaded = page.waitForEvent("load");
  await sheet.getByRole("button", { name: "close" }).click();
  await reloaded;
  await page.goto("/new");
  await expect(page.locator(`#ufs input[name=folder][value$="/${folder}"]`)).toHaveCount(1);

  await page.getByRole("button", { name: "settings" }).click();
  await page.locator("#sheet").getByRole("button", { name: "roots" }).click();
  page.once("dialog", (d) => d.accept());
  await sheet.locator(".lrw", { hasText: `~/${folder}` }).first().getByRole("button", { name: "remove" }).click();
  await expect(sheet.locator(".lrw .sk", { hasText: `~/${folder}` })).toHaveCount(1);
});

test("Use a folder…: search finds a sub-folder, [+ new folder] makes one in a root and picks it", async ({ page }, info) => {
  const name = `new-${info.project.name}`;
  await page.goto("/new");
  await page.getByRole("button", { name: "folder: scratch ▾" }).click();
  const panel = page.locator("#ufs");
  await expect(panel).toBeVisible();
  await expect(panel.getByRole("radio", { name: /^scratch/ })).toBeChecked();
  // Search first, then scratch; a root heads its group and its folders show by name.
  await expect(panel.locator(".fl > :not(legend)").first()).toHaveAttribute("id", "fq");
  await expect(panel.locator('label[title="~/root"]')).toHaveText("~/root");
  await expect(panel.locator('label[title="~/root/project"]')).toHaveText(/^project/);
  const deep = panel.getByRole("radio", { name: /^deep-src/ });
  const root = panel.getByRole("radio", { name: /^~\/root( |$)/ });
  await expect(deep).toBeHidden();
  await page.getByLabel("search folders").fill("deep");
  await expect(deep).toBeVisible();
  await expect(root).toBeHidden();
  await shot(page, info, "picker-search");
  await page.getByLabel("search folders").fill("");
  await root.check();
  await expect(page.getByRole("button", { name: "folder: root ▾" })).toBeVisible();
  await page.getByRole("button", { name: "folder: root ▾" }).click();
  page.once("dialog", (d) => d.accept(name));
  await panel.getByRole("button", { name: "+ new folder" }).click();
  await expect(page.getByRole("button", { name: `folder: ${name} ▾` })).toBeVisible();
  await expect(panel.locator(`input[value$="/root/${name}"]`)).toBeChecked();
  await page.getByRole("button", { name: `folder: ${name} ▾` }).click();
  await shot(page, info, "picker-new-folder");
  await audit(page);
});

test("Use a folder… in a chat: its later turns run in the folder, the chat says so", async ({ page }, info) => {
  await page.goto("/new");
  await page.getByLabel("message", { exact: true }).fill("Say hello");
  await page.getByRole("button", { name: "send" }).click();
  await page.waitForURL(/\/t\//);
  await expect(page.locator("#st")).toHaveAttribute("data-st", "done");
  await expect(page.locator(".th .ds")).toContainText("scratch");
  // Beside the agents the messages keep the design's centred column, at most 100ch wide.
  const [inner, ch100] = await page.locator("#msgs").evaluate((el) => {
    const s = getComputedStyle(el), p = document.createElement("div");
    p.style.width = "100ch";
    el.append(p);
    const w = p.getBoundingClientRect().width;
    p.remove();
    return [el.clientWidth - parseFloat(s.paddingLeft) - parseFloat(s.paddingRight), w];
  });
  expect(inner).toBeLessThanOrEqual(ch100 + 1);
  await page.getByRole("button", { name: "folder: scratch ▾" }).click();
  await page.getByLabel("search folders").fill("project");
  await page.locator("#ufs").getByRole("radio", { name: /^project( |$)/ }).check();
  await expect(page.locator("#msgs .qs").last()).toHaveText(/^now in ~\/root\/project · later turns run here · \d\d?:\d\d/);
  await expect(page.getByRole("button", { name: "folder: project ▾" })).toBeVisible();
  // The thread bar names the folder at once, as the chip does.
  await expect(page.locator("#bf")).toHaveText("~/root/project");
  await shot(page, info, "chat-folder-attached");
  // A reload draws the folder; scratch is no way back.
  await page.reload();
  await expect(page.locator(".th .ds")).toContainText("~/root/project");
  await expect(page.locator('#ufs input[value=""]')).toBeDisabled();
  await audit(page);
});
