// Workflow (#33): the chips filter the one list (remembered for the device), the fake shed's lanes show their repo,
// issue, role and round, the finished one folds under "done today", and a running impl lane opens as a normal chat.
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const shot = (page, info, name) => page.screenshot({ path: `../../ui-${info.project.name}/${name}.png`, fullPage: true });
const axe = async (page) => {
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
};

test("Workflow: the chip filters the list and a running impl lane opens as a chat", async ({ page }, info) => {
  await page.goto("/");
  const chips = page.locator("#chips");
  await expect(chips.getByRole("radio", { name: "All" })).toHaveAttribute("aria-checked", "true");
  const lane = page.locator(".fc .tr", { hasText: "cooee #33" });
  await expect(lane).toContainText("impl · round 1");
  // Listed, perhaps under "older threads" once other tests have started threads in its folder.
  await expect(page.locator(".fc .tr", { hasText: "Long thread" }).first()).toBeAttached();

  await chips.getByRole("radio", { name: "Workflow" }).click();
  await expect(chips.getByRole("radio", { name: "Workflow" })).toHaveAttribute("aria-checked", "true");
  await expect(page.locator(".fc .tr", { hasText: "Long thread" })).toHaveCount(0);
  // Needs-you first, then working; the finished one folded.
  const open = page.locator(".fc > .tr .tt");
  await expect(open).toHaveText(["cooee #31", "cooee #33"]);
  const fold = page.locator(".fc details", { hasText: "1 done today" });
  await expect(fold.locator(".tr")).toBeHidden();
  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth);
  expect(overflow).toBeLessThanOrEqual(0);
  await shot(page, info, "workflow");
  await axe(page);
  // Remembered: the list loads under the same chip, and its refresh keeps it.
  await page.reload();
  await expect(chips.getByRole("radio", { name: "Workflow" })).toHaveAttribute("aria-checked", "true");
  await page.getByRole("button", { name: "refresh" }).click();
  await expect(page.locator(".fc .tr", { hasText: "Long thread" })).toHaveCount(0);
  await expect(page.locator(".fc > .tr .tt")).toHaveText(["cooee #31", "cooee #33"]);
  await fold.locator("summary").click();
  await expect(fold.locator(".tr")).toContainText("fix · round 1");

  await page.locator(".fc .tr", { hasText: "cooee #33" }).click();
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("cooee #33");
  await expect(page.locator(".th .ds")).toContainText("impl · round 1");
  await expect(page.getByRole("link", { name: "issue & PR" })).toHaveAttribute("href", "https://github.com/o/cooee/issues/33");
  await expect(page.locator("#msgs")).toContainText("w0");
  await expect(page.locator("#pill")).toBeVisible();
  await expect(page.locator("#pill .stop")).toBeVisible();
  await expect(page.getByRole("textbox", { name: "message" })).toBeEnabled();
  await expect(page.locator("#close")).toBeHidden();
  await shot(page, info, "workflow-chat");
  await axe(page);

  await page.goto("/");
  await chips.getByRole("radio", { name: "All" }).click();
  await expect(chips.getByRole("radio", { name: "All" })).toHaveAttribute("aria-checked", "true");
});
