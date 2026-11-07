// Sign-ins against the fake shed (web/e2e/serve.sh), which runs a fake login program per viewport: Settings ›
// providers (#48: one flat row per account) starts it for the system default login, shows its screen under the row
// with the link tappable, types the code in, and shows the account signed in; [+ account] is offered for it.
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

test("sign-ins: the provider's own login, its link tappable, a code typed in, then signed in", async ({ page, context }, info) => {
  const name = `fake-${info.project.name}`, url = `https://login.example.com/device?client_id=cooee-${info.project.name}`;
  await page.goto("/");
  await page.getByRole("button", { name: "settings" }).click();
  await page.locator("#sheet").getByRole("button", { name: "providers" }).click();
  const sheet = page.locator("#sh-providers");
  await expect(sheet).toBeVisible();
  const row = sheet.locator(`.acr[data-provider="${name}"][data-account=""]`), state = row.locator(".st");
  await expect(row.locator(".ptg")).toHaveText(name);
  await expect(row.locator(".sk")).toHaveText("main login");
  await expect(sheet.locator(`.acr.add[data-provider="${name}"]`).getByRole("button", { name: "+ account" })).toBeVisible();
  await expect(state).toHaveText("signed out");
  await row.getByRole("button", { name: "sign in" }).click();
  await expect(state).toHaveText("signing in");
  const panel = sheet.locator(`.lg[data-provider="${name}"]`);
  await expect(panel.locator(".out")).toContainText("Paste code here");
  // Under the row that started it.
  expect(await row.evaluate((r) => r.nextElementSibling?.classList.contains("lg"))).toBe(true);
  const link = panel.locator(".out").getByRole("link");
  await expect(link).toHaveText(url);
  await page.screenshot({ path: `../../ui-${info.project.name}/signins.png`, fullPage: true });
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);

  // A tap opens the sign-in page.
  await context.route("https://login.example.com/**", (r) => r.fulfill({ body: "sign in here" }));
  const [popup] = await Promise.all([page.waitForEvent("popup"), link.click()]);
  expect(popup.url()).toBe(url);
  await popup.close();

  await panel.getByLabel(`the code from the ${name} sign-in page`).fill("good-code");
  await panel.getByRole("button", { name: "send" }).click();
  // The page comes back with Settings › providers open, the account signed in.
  await expect(state).toHaveText("✓ signed in");
  await expect(sheet).toBeVisible();
  await expect(panel).toBeHidden();
  await expect(row.getByRole("button", { name: "sign in" })).toHaveCount(0);
  await expect(row.getByRole("button", { name: "sign out" })).toBeVisible();
  await expect(sheet).not.toContainText("default");
});
