// The machines plugin (crates/cooee-machines), run in cooee by serve.sh over the fake shed: a card for this machine
// with its numbers, terminals and 24 h sparklines, a tap opens a chart; the Air offline with when it was last seen,
// and the tab's dot while it is.
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

test("the machines tab: a card per machine, a chart on tap, the Air offline and the tab's dot", async ({ page }, info) => {
  await page.goto("/p/machines");
  const tab = page.frameLocator("iframe.pf");
  const here = tab.locator("article.on");
  const air = tab.getByRole("article", { name: "air", exact: true });
  await expect(here.locator(".seen")).toHaveText("online");
  await expect(here.locator("summary").filter({ hasText: "disk free" })).toContainText("GB");
  await expect(here.locator(".tm")).toHaveText("terminals: 1 blocked · 4 exited · 2 working");
  await expect(here.locator("svg.sp polyline").first()).toHaveAttribute("points", /^(\S+ ){1439}\S+$/);
  await expect(air.locator(".seen")).toHaveText(/^offline · last seen 4\d min ago$/);
  // The dot: the plugin has told cooee by now; the header is drawn with the page.
  await expect.poll(async () => { await page.reload(); return page.getByRole("link", { name: "Machines (needs you)" }).count(); }).toBe(1);

  const load = here.locator("details", { hasText: "load/cpu" });
  await load.locator("summary").click();
  await expect(load.getByRole("img", { name: /^load\/cpu, last 24 h: low 0\.\d\d · high 0\.\d\d$/ })).toBeVisible();
  await page.screenshot({ path: `../../ui-${info.project.name}/machines.png` });
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
});
