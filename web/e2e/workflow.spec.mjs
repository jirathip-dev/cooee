import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const shot = (page, info, name) => page.screenshot({ path: `../../ui-${info.project.name}/${name}.png`, fullPage: true });
const axe = async (page) => {
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
};

test("Workflow: lanes are never among the agents, and a running impl lane opens as a chat", async ({ page }, info) => {
  await page.goto("/?list");
  await expect(page.locator(".ags .agr").first()).toBeVisible();
  await expect(page.locator(".ags", { hasText: "cooee #33" })).toHaveCount(0);
  const rows = await (await page.request.get("/threads")).json();
  const lane = rows.find((r) => r.workflow?.item === "#33" && r.workflow.role === "impl");
  expect(lane.workflow).toMatchObject({ group: "cooee", round: "1" });

  await page.goto(`/t/${lane.id}`);
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("cooee #33");
  await expect(page.locator(".th .ds")).toContainText("impl · round 1");
  await expect(page.getByRole("link", { name: "issue & PR" })).toHaveAttribute("href", "https://github.com/o/cooee/issues/33");
  await expect(page.locator("#msgs")).toContainText("w0");
  await expect(page.locator("#pill")).toBeVisible();
  await expect(page.locator("#pill .stop")).toBeVisible();
  await expect(page.getByRole("textbox", { name: "message" })).toBeEnabled();
  await expect(page.locator("#close")).toBeHidden();
  await expect(page.locator(".fo")).toHaveCount(0);
  await shot(page, info, "workflow-chat");
  await axe(page);
});
