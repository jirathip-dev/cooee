// Settings › mcp servers (#56) in #48's flat-list pattern (serve.sh: `files`, stdio and on, three tools; `docs`, http
// and off): a row per server, `+ server`'s card with the exact command before Face ID, and an agent's servers with a
// checkbox per tool.
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const shot = (page, info, name) => page.screenshot({ path: `../../ui-${info.project.name}/${name}.png`, fullPage: true });

async function audit(page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)).toBeLessThanOrEqual(0);
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
}

test.beforeEach(async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" });
});

test("mcp servers: one row per server, + server's card shows the exact command, an agent's tools", async ({ page }, info) => {
  await page.goto("/");
  await expect(page.locator('#sh-settings a.sr[href="/mcp"]')).toHaveCount(1);
  await page.goto("/mcp");
  const files = page.locator('.acr[data-server="files"]');
  await expect(files.locator(".ptg")).toHaveText("stdio");
  await expect(files.locator(".sk")).toHaveText("files");
  await expect(files.getByRole("button", { name: "files is on: turn it off" })).toHaveText("on");
  await expect(files.locator(".st")).toHaveText("edit on Mac");
  await expect(files.getByRole("button", { name: "remove files" })).toBeVisible();
  const docs = page.locator('.acr[data-server="docs"]');
  await expect(docs.locator(".ptg")).toHaveText("http");
  await expect(docs.getByRole("button", { name: "docs is off: turn it on" })).toHaveText("off");
  await page.locator("summary", { hasText: "Reviewer" }).click();
  for (const t of ["read_file", "write_file", "search"]) await expect(page.getByLabel(t, { exact: true })).toBeChecked();
  await expect(page.getByLabel("files", { exact: true })).not.toBeChecked();
  await shot(page, info, "mcp-servers");
  await audit(page);

  await page.locator("summary", { hasText: "+ server" }).click();
  await page.getByLabel("name", { exact: true }).fill("git");
  await page.getByLabel("command or URL").fill("uvx mcp-server-git --repository ~/code");
  await page.getByRole("button", { name: "review" }).click();
  const card = page.getByRole("group", { name: "add MCP server git" });
  await expect(card.locator("pre")).toHaveText("uvx mcp-server-git --repository ~/code");
  await expect(card.getByRole("button", { name: "approve with Face ID" })).toBeVisible();
  await shot(page, info, "mcp-add-card");
  await audit(page);
});
