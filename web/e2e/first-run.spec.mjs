// A fresh instance (serve.sh, $PORT + 3: no agents in shed, no roots): the new-thread form says what to run instead
// of showing empty pickers, start waits, a provider whose CLI is missing shows its install line, and each line copies.
import fs from "node:fs";
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

test.use({ storageState: { cookies: [], origins: [] } });

test("first run: no agents and no roots show their fix lines with copy buttons, never empty pickers", async ({ page, context, baseURL }, info) => {
  test.skip(info.project.name !== "390", "the fresh instance has one pairing code");
  const url = baseURL.replace(/:(\d+)$/, (_, p) => `:${Number(p) + 3}`);
  await context.grantPermissions(["clipboard-read", "clipboard-write"], { origin: url });
  await page.goto(`${url}/`);
  const code = fs.readFileSync(new URL("./.auth/code.bare", import.meta.url), "utf8").trim();
  await page.getByLabel("digit 1").click();
  await page.keyboard.type(code);

  const form = page.locator("#new");
  await expect(form).toBeVisible();
  await expect(form.getByRole("radio")).toHaveCount(0);
  await expect(form.locator("select")).toHaveCount(0);
  const agents = form.locator(".ag .hn"), folders = form.locator(".hn").last();
  await expect(agents).toContainText("no agents installed · on the Mac:");
  await expect(agents.locator("code")).toHaveText("shed agents add claude-acp");
  await expect(folders).toContainText("no folders set · add");
  await expect(folders.locator("code")).toHaveText('roots = ["~/Projects"]');
  await expect(folders).toContainText("to config.toml");
  await expect(form.getByRole("button", { name: "start" })).toBeDisabled();
  await page.screenshot({ path: "../../ui-390/first-run.png", fullPage: true });
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);

  for (const [row, fix] of [[agents, "shed agents add claude-acp"], [folders, 'roots = ["~/Projects"]']]) {
    await row.getByRole("button", { name: `copy ${fix}` }).click();
    await expect(row.getByRole("button")).toHaveText("copied");
    expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(fix);
  }

  // A provider whose CLI isn't installed: the install line, no sign-in.
  await page.getByRole("button", { name: "settings" }).click();
  await page.locator("#sheet").getByRole("link", { name: "providers" }).click();
  const row = page.locator('.pv[data-provider="codex"]');
  await expect(row.locator(".fh span")).toHaveText("not installed");
  await expect(row).toContainText("not installed on this Mac");
  await expect(row.getByRole("button", { name: "+ account" })).toHaveCount(0);
  await expect(row.locator(".hn code")).toHaveText("npm install -g @openai/codex");
  await expect(row.getByRole("button", { name: "sign in" })).toBeHidden();
  await page.screenshot({ path: "../../ui-390/first-run-signins.png", fullPage: true });
  await row.getByRole("button", { name: "copy npm install -g @openai/codex" }).click();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe("npm install -g @openai/codex");
});
