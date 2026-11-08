// A fresh instance (serve.sh, $PORT + 3: no agents in shed, no roots): a new chat says what to run instead of
// showing a composer, a provider whose CLI is missing shows its install line, and each line copies.
import fs from "node:fs";
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

test.use({ storageState: { cookies: [], origins: [] } });

test("first run: no agents shows its fix line with a copy button, never an empty composer", async ({ page, context, baseURL }, info) => {
  test.skip(info.project.name !== "390", "the fresh instance has one pairing code");
  const url = baseURL.replace(/:(\d+)$/, (_, p) => `:${Number(p) + 3}`);
  await context.grantPermissions(["clipboard-read", "clipboard-write"], { origin: url });
  await page.goto(`${url}/`);
  const code = fs.readFileSync(new URL("./.auth/code.bare", import.meta.url), "utf8").trim();
  await page.getByLabel("pairing code").fill(code);
  await page.getByRole("button", { name: "pair" }).click();
  await page.waitForURL(/\/$/);
  await page.goto(`${url}/new`);

  // No agent to type to: what to run instead, no composer.
  await expect(page.locator("#new")).toHaveCount(0);
  const agents = page.locator(".hip .hn");
  await expect(agents).toContainText("no agents installed · on the Mac:");
  await expect(agents.locator("code")).toHaveText("shed agents add claude-acp");
  await page.screenshot({ path: "../../ui-390/first-run.png", fullPage: true });
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
  await agents.getByRole("button", { name: "copy shed agents add claude-acp" }).click();
  await expect(agents.getByRole("button")).toHaveText("copied");
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe("shed agents add claude-acp");

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
  await row.getByRole("button", { name: "copy install" }).click();
  await expect(row.getByRole("button")).toHaveText("copied");
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe("npm install -g @openai/codex");
  // `copied` is for a moment; the button reads [copy install] again.
  await expect(row.getByRole("button")).toHaveText("copy install");
});
