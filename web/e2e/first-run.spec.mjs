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
