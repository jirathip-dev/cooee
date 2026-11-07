// A fresh instance (serve.sh, $PORT + 3: no agents in shed, no roots): the composer says what to run instead of
// showing empty pickers, send waits, a provider whose CLI is missing is one line with [copy install], and each copies.
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
  await page.getByLabel("pairing code").fill(code);
  await page.getByRole("button", { name: "pair" }).click();
  await page.waitForURL(/\/$/);
  await page.goto(`${url}/new`);

  const form = page.locator("#nc");
  await expect(form).toBeVisible();
  // Only scratch (no folder: the thread's own), picked; no agent or folder to pick.
  await expect(form.locator("input[type=radio]")).toHaveCount(1);
  await expect(form.locator("input[name=folder][value='']")).toBeChecked();
  const agents = form.locator(".hn").first(), folders = form.locator(".hn").last();
  await expect(agents).toContainText("no agents installed · on the Mac:");
  await expect(agents.locator("code")).toHaveText("shed agents add claude-acp");
  await expect(folders).toContainText("no folders set · add");
  await expect(folders.locator("code")).toHaveText('roots = ["~/Projects"]');
  await expect(folders).toContainText("to config.toml");
  await expect(form.getByRole("button", { name: "send" })).toBeDisabled();
  await page.screenshot({ path: "../../ui-390/first-run.png", fullPage: true });
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);

  for (const [row, fix] of [[agents, "shed agents add claude-acp"], [folders, 'roots = ["~/Projects"]']]) {
    await row.getByRole("button", { name: `copy ${fix}` }).click();
    await expect(row.getByRole("button")).toHaveText("copied");
    expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(fix);
  }

  // A provider whose CLI isn't installed: one line, `[codex] not installed [copy install]`, no sign-in.
  await page.getByRole("button", { name: "settings" }).click();
  await page.locator("#sheet").getByRole("button", { name: "providers" }).click();
  const sheet = page.locator("#sh-providers");
  await expect(sheet.locator(".acr")).toHaveCount(1);
  const row = sheet.locator(".acr.ni");
  await expect(row.locator(".ptg")).toHaveText("codex");
  await expect(row.locator(".sk")).toHaveText("not installed");
  await expect(sheet.getByRole("button", { name: /sign in|\+ account/ })).toHaveCount(0);
  await page.screenshot({ path: "../../ui-390/first-run-signins.png", fullPage: true });
  await row.getByRole("button", { name: "copy npm install -g @openai/codex" }).click();
  await expect(row.getByRole("button")).toHaveText("copied");
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe("npm install -g @openai/codex");
});
