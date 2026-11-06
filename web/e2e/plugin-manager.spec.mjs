// Settings › Plugins (#37) at 390: serve.sh's index lists the example plugin as `greet`; a URL gets the "not
// reviewed" warning. Adding shows the exact commit and commands to confirm; then it runs, its log shows the build,
// its tab is there with its backend reached, and [remove] takes the tab away. One install: the 390 project only.
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const shot = (page, info, name) => page.screenshot({ path: `../../ui-${info.project.name}/${name}.png` });

async function audit(page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)).toBeLessThanOrEqual(0);
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
}

test("Settings › Plugins: add from the index after a confirm, it runs with its tab and log, then remove", async ({ page }, info) => {
  test.skip(info.project.name !== "390", "one install, audited at 390");
  await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" });
  await page.goto("/#settings/plugins");
  const box = page.locator("#s-plugins-box");
  await box.getByRole("button", { name: "add a plugin" }).click();
  const listed = box.locator(".lrw", { hasText: "greet · the example plugin" });
  await expect(listed).toBeVisible();
  await expect(box.getByText(/^not reviewed: a plugin from a URL/)).toBeVisible();
  await box.getByLabel("git URL").fill("https://example.com/x.git");
  await shot(page, info, "plugins-add");
  await audit(page);

  let asked = "";
  page.once("dialog", (d) => { asked = d.message(); d.accept(); });
  await listed.getByRole("button", { name: "add greet" }).click();
  const row = box.locator(".lrw", { hasText: /^greet · 0\.1\.0 · [0-9a-f]{7} · index/ });
  await expect(row.locator(".ds")).toHaveText("running", { timeout: 30_000 });
  expect(asked).toMatch(/^Add this\?\n\ngreet 0\.1\.0 from .*\ncommit [0-9a-f]{40}\nbuild: node --check server\.mjs\nrun: TOKEN=\$COOEE_PLUGIN_TOKEN exec node server\.mjs\n/);
  expect(asked).not.toContain("not reviewed");
  await box.getByRole("button", { name: "close the plugin list" }).click();
  await expect(box.getByRole("radio", { name: "greet on" })).toHaveAttribute("aria-checked", "true");
  await box.getByRole("button", { name: "logs of greet" }).click();
  await expect(box.locator(".plg")).toContainText("== cooee: build");
  await shot(page, info, "plugins");
  await audit(page);

  // Its tab, its backend reached with its own token.
  await page.goto("/");
  await expect(page.getByRole("navigation", { name: "tabs" }).getByRole("link", { name: "Greet" })).toBeVisible();
  await page.goto("/p/greet");
  await expect(page.frameLocator("iframe.pf").getByText(/^whoami/)).toHaveText('whoami 200: {"token":"ok","cookie":null}');

  await page.goto("/#settings/plugins");
  page.once("dialog", (d) => d.accept());
  await box.getByRole("button", { name: "remove greet" }).click();
  await expect(box.getByText(/^greet · 0\.1\.0/)).toHaveCount(0);
  await page.goto("/");
  await expect(page.getByRole("navigation", { name: "tabs" }).getByRole("link", { name: "Greet" })).toHaveCount(0);
});
