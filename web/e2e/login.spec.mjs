// Sign-ins against the fake shed (web/e2e/serve.sh), which runs a fake login program per viewport: the sheet starts
// it, shows its screen with the link tappable, types the code in, and shows the provider signed in.
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

test("sign-ins: the provider's own login, its link tappable, a code typed in, then signed in", async ({ page, context }, info) => {
  const name = `fake-${info.project.name}`, url = `https://login.example.com/device?client_id=cooee-${info.project.name}`;
  await page.goto("/");
  await page.getByRole("link", { name: "sign-ins" }).click();
  const row = page.locator(`.pv[data-provider="${name}"]`), state = row.locator(".fh span");
  await expect(state).toHaveText("signed out");
  await expect(row).toContainText("Not signed in");
  await row.getByRole("button", { name: "sign in" }).click();
  await expect(state).toHaveText("signing in");
  await expect(row.locator(".out")).toContainText("Paste code here");
  const link = row.locator(".out").getByRole("link");
  await expect(link).toHaveText(url);
  await page.screenshot({ path: `../../ui-${info.project.name}/signins.png`, fullPage: true });
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);

  // A tap opens the sign-in page.
  await context.route("https://login.example.com/**", (r) => r.fulfill({ body: "sign in here" }));
  const [popup] = await Promise.all([page.waitForEvent("popup"), link.click()]);
  expect(popup.url()).toBe(url);
  await popup.close();

  await row.getByLabel(`the code from the ${name} sign-in page`).fill("good-code");
  await row.getByRole("button", { name: "send" }).click();
  await expect(state).toHaveText("✓ signed in");
  await expect(row).toContainText("Signed in as owner@example.com");
  await expect(row.locator(".out")).toBeHidden();
  await expect(row.getByRole("button", { name: "sign in" })).toBeHidden();
});
