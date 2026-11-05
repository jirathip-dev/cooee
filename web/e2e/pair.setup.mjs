// Pairs this browser with the code serve.sh got from `cooee pair`; the viewport projects reuse its cookie.
import fs from "node:fs";
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

test("an unpaired browser is sent to pairing; the code pairs it", async ({ page, request }) => {
  // Unpaired: pages go to /pair, reads and writes are refused.
  expect((await request.get("/threads/long-390/messages")).status()).toBe(401);
  expect((await request.post("/threads/long-390/send", { data: { text: "x", random_id: "r" } })).status()).toBe(401);
  await page.goto("/");
  await expect(page).toHaveURL(/\/pair$/);
  await page.screenshot({ path: "../../ui-390/pair.png", fullPage: true });
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);

  const code = fs.readFileSync(new URL("./.auth/code", import.meta.url), "utf8").trim();
  expect(code).toMatch(/^\d{6}$/);
  await page.getByLabel("pairing code").fill(code);
  await page.getByRole("button", { name: "pair" }).click();
  await expect(page).toHaveURL(/\/$/);
  await expect(page.locator(".fc .tr").first()).toBeVisible();
  await page.context().storageState({ path: new URL("./.auth/state.json", import.meta.url).pathname });
});
