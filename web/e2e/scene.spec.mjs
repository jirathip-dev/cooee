// Scene (#34): Style's third value draws the thread list as the theme's scene: one sprite per session on a page per
// folder (swiped, a dot per page), a tap opens the thread in the style chosen before, refreshes never copy a sprite,
// the sprites switch is greyed out in it and Scene is offered only with sprites; and the audit at 390 in each theme.
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const sheet = (page) => page.locator("#sheet .sh:popover-open");
const pickOn = (context, url, pick) =>
  context.addCookies([{ name: "cooee-look", value: Object.entries(pick).map(([k, v]) => `${k}:${v}`).join("/"), url }]);
// The list as the server has it: each folder's thread ids, in order.
const listed = async (page) =>
  (await (await page.request.get("/threads")).json()).map((c) => ({ folder: c.folder, ids: c.rows.map((r) => r.id) }));
// The scene as drawn: each page's folder and its sprites' ids.
const drawn = (page) =>
  page.locator("#scene .sp").evaluateAll((ps) =>
    ps.map((p) => ({ folder: p.getAttribute("aria-label"), ids: [...p.querySelectorAll(".sb")].map((a) => a.dataset.id) })),
  );

test.beforeEach(async ({ page }) => { await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" }); });
// The device's clock by day (noon) or night (05:00), as now or before it, so the page never reads as an old copy.
async function clock(page, night) {
  const at = new Date();
  const h = at.getHours();
  if (night && h >= 7 && h < 19) at.setHours(5);
  if (!night && (h < 7 || h >= 19)) at.setHours(h >= 19 ? 12 : -12);
  await page.clock.setFixedTime(at);
}

// A thread in the root folder itself (the long fixtures are in its project folder), so there are two pages.
test.beforeAll(async ({ browser }) => {
  const page = await browser.newPage({ storageState: test.info().project.use.storageState });
  await page.goto("/#new");
  await page.locator("#new input[name=folder]").first().check();
  await page.getByLabel("first message").fill("a scene of two folders");
  await page.getByRole("button", { name: "start" }).click();
  await page.waitForURL(/\/t\//);
  await page.close();
});

test("Scene draws one sprite per session on a page per folder; a dot turns the page; a tap opens the thread", async ({ page, context, baseURL }) => {
  await pickOn(context, baseURL, { style: "terminal", list: "scene" });
  await page.goto("/");
  await expect(page.locator(".fc")).toHaveCount(0);
  // Other tests add threads meanwhile: the page drawn between two equal lists.
  let want;
  await expect(async () => {
    want = await listed(page);
    await page.reload();
    expect(await drawn(page)).toEqual(want);
    expect(await listed(page)).toEqual(want);
  }).toPass();
  expect(want.length).toBeGreaterThanOrEqual(2);
  // Each sprite acts out its state with the theme's art.
  const pose = await page.locator("#scene .sb .sf use").first().getAttribute("href");
  expect(pose).toMatch(/^\/look\/farm\/sprites\.svg#s-(edit|needs|done|fail|merged|idle)$/);
  await expect(page.locator("#scene .sky").first()).toHaveAttribute("src", /^\/look\/farm\/scene\.svg/);
  // A dot per page; the second turns to the second page and is marked.
  const dots = page.locator("#scene .sdt a");
  await expect(dots).toHaveCount(want.length);
  await expect(dots.first()).toHaveAttribute("aria-current", "page");
  await dots.nth(1).click();
  await expect(dots.nth(1)).toHaveAttribute("aria-current", "page");
  await expect(dots.first()).not.toHaveAttribute("aria-current", "page");
  const second = page.locator("#scene .sp").nth(1);
  await expect.poll(() => page.locator("#scene .spg").evaluate((e) => e.scrollLeft / e.clientWidth)).toBeCloseTo(1, 1);
  // A tap opens the thread, in Terminal, the style chosen before Scene.
  const id = want[1].ids[0];
  await second.locator(`.sb[data-id="${id}"]`).click();
  await expect(page).toHaveURL(new RegExp(`/t/${id}$`));
  await expect(page.locator("html")).toHaveAttribute("data-style", "terminal");
});

test("refreshed 5 times, Scene keeps one sprite per session", async ({ page, context, baseURL }) => {
  await pickOn(context, baseURL, { list: "scene" });
  await page.goto("/");
  for (let i = 0; i < 5; i++) {
    await page.getByRole("button", { name: "refresh" }).click();
    await expect(page.locator("#rf")).not.toHaveClass(/rfs/);
    const ids = (await drawn(page)).flatMap((p) => p.ids);
    expect(new Set(ids).size).toBe(ids.length);
    // Other tests add threads meanwhile: every sprite is a session the server lists, once.
    const all = (await listed(page)).flatMap((p) => p.ids);
    expect(ids.every((id) => all.includes(id))).toBe(true);
  }
  await expect(page.locator("#scene .sdt a")).toHaveCount((await drawn(page)).length);
});

test("in Scene the sprites switch is greyed out; ascii, off or a theme with no sprites hide Scene", async ({ page }, info) => {
  await page.goto("/");
  await page.getByRole("button", { name: "look: theme and colours" }).click();
  const scene = sheet(page).locator('[data-k=style][data-v=scene]');
  await scene.click();
  // The list is drawn again as the scene; the sheet says why the switch is off.
  await expect(page.locator("#scene")).toBeVisible();
  await page.getByRole("button", { name: "look: theme and colours" }).click();
  await expect(scene).toHaveAttribute("aria-checked", "true");
  for (const v of ["on", "ascii", "off"]) await expect(sheet(page).locator(`[data-k=sprites][data-v=${v}]`)).toBeDisabled();
  await expect(sheet(page).getByText("scene needs sprites")).toBeVisible();
  await page.screenshot({ path: `../../ui-${info.project.name}/scene-look.png` });
  // Modern again: the cards, the switch back.
  await sheet(page).locator('[data-k=style][data-v=modern]').click();
  await expect(page.locator("#scene")).toHaveCount(0);
  await expect(page.locator(".fc").first()).toBeVisible();
  await page.getByRole("button", { name: "look: theme and colours" }).click();
  await expect(sheet(page).locator("[data-k=sprites][data-v=ascii]")).toBeEnabled();
  await sheet(page).locator("[data-k=sprites][data-v=ascii]").click();
  await expect(scene).toBeHidden();
  await sheet(page).locator("[data-k=sprites][data-v=on]").click();
  await expect(scene).toBeVisible();
  await sheet(page).locator("[data-k=theme][data-v=terminal]").click();
  await expect(scene).toBeHidden();
  await sheet(page).locator("[data-k=theme][data-v=farm]").click();
  await page.keyboard.press("Escape");
});

for (const theme of ["farm", "kitchen", "garden"])
  for (const mode of ["dark", "light"])
    test(`Scene's audit at 390: ${theme}, ${mode}`, async ({ page, context, baseURL }, info) => {
      test.skip(info.project.name !== "390", "the audit is at 390");
      await clock(page, false);
      await pickOn(context, baseURL, { theme, mode, list: "scene" });
      await page.goto("/");
      await expect(page.locator("#scene .sb").first()).toBeVisible();
      await expect.poll(() => page.locator("#scene .sky").first().evaluate((i) => i.complete && i.naturalWidth > 0)).toBe(true);
      expect(await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)).toBeLessThanOrEqual(0);
      const { violations } = await new AxeBuilder({ page }).analyze();
      expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
      await page.screenshot({ path: `../../ui-390/scene-${theme}-${mode}.png` });
    });

test("farm's Scene at night: the strip's night from 19:00 on this device's clock", async ({ page, context, baseURL }, info) => {
  test.skip(info.project.name !== "390", "the audit is at 390");
  await clock(page, true);
  await pickOn(context, baseURL, { list: "scene" });
  await page.goto("/");
  await expect(page.locator("#scene")).toHaveClass(/night/);
  await expect(page.locator("#scene .sky").first()).toHaveAttribute("src", "/look/farm/scene.svg#night");
  await expect.poll(() => page.locator("#scene .sky").first().evaluate((i) => i.complete && i.naturalWidth > 0)).toBe(true);
  await page.screenshot({ path: "../../ui-390/scene-farm-night.png" });
});
