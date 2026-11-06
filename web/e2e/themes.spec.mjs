// Theme packs (#38) from the phone: Settings › look › themes lists the installed packs; [remix] farm opens a chat with
// an agent in a new theme folder; its live preview (the list and the chat in the theme) reloads when a file is saved;
// [keep] passes the theme gate and installs it, and the look sheet picks it. Audited at 390 and 1440.
import { readFileSync, writeFileSync } from "node:fs";
import { dirname } from "node:path";
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const shot = (page, info, name) => page.screenshot({ path: `../../ui-${info.project.name}/${name}.png`, fullPage: true });
// serve.sh's config folder, where themes/ and themes-dev/ are.
const config = () => dirname(readFileSync(new URL("./.auth/code.config", import.meta.url), "utf8").trim());

// The preview's frames are the list and the chat, audited in their own specs.
async function audit(page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)).toBeLessThanOrEqual(0);
  const { violations } = await new AxeBuilder({ page }).exclude(".prf iframe").analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
}

test("remix farm with an agent, watch the preview reload on a save, keep it and pick it", async ({ page }, info) => {
  test.skip(info.project.name === "webkit", "the iPhone audits only");
  const name = `e2e-${info.project.name}`;
  await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" });
  await page.goto("/#settings/look");
  const farm = page.locator(".thm", { hasText: "farm" });
  await expect(farm.getByRole("button", { name: "remove" })).toBeVisible();
  await farm.locator("summary", { hasText: "remix" }).click();
  await farm.getByLabel("the new theme's name").fill(name);
  await farm.getByLabel("what it should look like").fill("farm at night");
  await shot(page, info, "themes-settings");
  await audit(page);
  await farm.getByRole("button", { name: "start with an agent" }).click();
  await page.waitForURL(/\/t\//);

  await page.locator(".tdv a", { hasText: `preview ${name}` }).click();
  await expect(page.locator(".prg")).toHaveText("passes the theme gate · reloads on each save");
  const list = page.frameLocator('iframe[title="thread list"]');
  await expect(list.locator(`html[data-theme="dev.${name}"]`)).toBeAttached();
  await expect(page.frameLocator('iframe[title="chat"]').locator("#msgs")).toBeAttached();
  await shot(page, info, "themes-preview");
  await audit(page);

  // A save (the agent's) reloads the preview: the page's own mark is gone, the new word shows.
  await page.evaluate(() => { window.before = true; });
  const file = `${config()}/themes-dev/${name}/theme.json`;
  const theme = JSON.parse(readFileSync(file, "utf8"));
  theme.label = `${name} night`;
  theme.words.loading = "saddling up at night";
  writeFileSync(file, JSON.stringify(theme));
  await expect.poll(() => page.evaluate(() => window.before), { timeout: 15_000 }).toBeUndefined();
  await expect(list.locator("#ld .lw")).toContainText("saddling up at night");

  await page.getByRole("button", { name: "keep" }).click();
  await page.waitForURL(/#settings\/look$/);
  await expect(page.locator(".thm", { hasText: `${name} night` })).toBeVisible();
  await page.getByRole("button", { name: "look: theme and colours" }).click();
  await page.locator(`#sh-look [data-k=theme][data-v="${name}"]`).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", name);
});
