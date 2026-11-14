import { expect, test } from "@playwright/test";

test.use({ javaScriptEnabled: false });

const shot = (page, info, name) => page.screenshot({ path: `../../ui-${info.project.name}/${name}.png` });

test("with no script the look sheet, Settings and Use a folder… open", async ({ page }, info) => {
  await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" });
  await page.goto("/");
  const open = page.locator("#sheet .sh:popover-open");
  await page.getByRole("button", { name: "look: theme and colours" }).click();
  await expect(open).toHaveAttribute("id", "sh-look");
  await expect(open.locator("[data-k=theme][data-v=farm]")).toHaveAttribute("aria-selected", "true");
  await shot(page, info, "nojs-look");
  await open.locator("[data-k=style][data-v=terminal]").click();
  await expect(page.locator("html")).toHaveAttribute("data-style", "terminal");
  await expect(page.locator("body")).toHaveCSS("font-family", /monospace/);
  await page.getByRole("button", { name: "look: theme and colours" }).click();
  await expect(open.getByRole("radiogroup", { name: "style" }).getByRole("radio")).toHaveText(["modern", "terminal", "scene"]);
  await open.locator("[data-k=figures][data-v=none]").click();
  await expect(page.locator("html")).toHaveAttribute("data-figures", "none");
  await page.getByRole("button", { name: "look: theme and colours" }).click();
  await expect(open.getByRole("radiogroup", { name: "style" }).getByRole("radio")).toHaveText(["modern", "terminal"]);
  await page.keyboard.press("Escape");

  await page.getByRole("button", { name: "settings" }).click();
  await expect(open).toHaveAttribute("id", "sh-settings");
  await expect(open.locator(".sv")).toHaveText(["terminal", "none", "farm", "system · dark now", "dark farm · light farm"]);
  await shot(page, info, "nojs-settings");
  await page.keyboard.press("Escape");
  await expect(open).toHaveCount(0);

  await page.goto("/new");
  await expect(page.locator("#new")).toBeVisible();
  await expect(page.locator("#new input[name=folder]").first()).toBeChecked();
  await page.getByRole("button", { name: "folder: scratch ▾" }).click();
  await expect(page.getByRole("radio", { name: /^~\/root( |$)/ })).toBeVisible();
  await shot(page, info, "nojs-new");
});
