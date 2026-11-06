// HTML-first (#49): with script off, the sheets and the new-thread form still open, as server HTML: the look sheet (a
// key posts the pick, and the page comes back drawn in it), Settings, and the new-thread folder picker.
import { expect, test } from "@playwright/test";

test.use({ javaScriptEnabled: false });

const shot = (page, info, name) => page.screenshot({ path: `../../ui-${info.project.name}/${name}.png` });

test("with no script the look sheet, Settings and the new-thread folder picker open", async ({ page }, info) => {
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

  await page.getByRole("button", { name: "settings" }).click();
  await expect(open).toHaveAttribute("id", "sh-settings");
  await expect(open.locator(".sv")).toHaveText(["terminal", "on", "farm", "system · dark now", "dark farm · light farm"]);
  await shot(page, info, "nojs-settings");
  await page.keyboard.press("Escape");
  await expect(open).toHaveCount(0);

  await page.goto("/#new");
  await expect(page.locator("#new")).toBeVisible();
  await expect(page.locator("#new input[name=folder]").first()).toBeChecked();
  await expect(page.getByRole("radio", { name: "~/root", exact: true })).toBeVisible();
  await shot(page, info, "nojs-new");
});
