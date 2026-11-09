// Scene (#34, #86): Style's third value draws the agents sidebar as the theme's scene: one page, a sprite per agent
// acting out its ongoing chat, a tap opens that chat in the style chosen before, the current agent ringed with its
// chats as chips; refreshes never copy a sprite; Scene is offered only with Figures Sprite (ASCII, None or the plain
// theme hide it); and the audit at 390 in each theme, farm's at 1440 too, every pose a vector figure (#115).
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const sheet = (page) => page.locator("#sheet .sh:popover-open");
const pickOn = (context, url, pick) =>
  context.addCookies([{ name: "cooee-look", value: Object.entries(pick).map(([k, v]) => `${k}:${v}`).join("/"), url }]);
const hrefs = (l) => l.evaluateAll((as) => as.map((a) => a.getAttribute("href")));
// Every Scene pose's symbol, from the sheet the page uses: a 132 × 100 figure, never a 16 px row sprite blown up (#115).
async function poses(page, theme) {
  const svg = await (await page.request.get(`/look/${theme}/sprites.svg`)).text();
  for (const k of ["edit", "idle", "done", "needs", "merged", "fail"]) {
    const use = `/look/${theme}/sprites.svg#s-scene-${k}`;
    const sym = new RegExp(`<symbol id="s-scene-${k}" viewBox="0 0 132 100">(.*?)</symbol>`).exec(svg)?.[1];
    expect(sym, use).toBeTruthy();
    expect(sym, use).not.toMatch(/class="row /);
    if (theme === "farm") expect(sym, use).not.toContain("crispEdges");
  }
  for (const href of await page.locator("#scene .sb .sf use").evaluateAll((us) => us.map((u) => u.getAttribute("href"))))
    expect(href).toMatch(new RegExp(`^/look/${theme}/sprites\\.svg#s-scene-(edit|needs|done|fail|merged|idle)$`));
}

test.beforeEach(async ({ page }) => { await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" }); });
// The device's clock by day (noon) or night (05:00), as now or before it, so the page never reads as an old copy.
async function clock(page, night) {
  const at = new Date();
  const h = at.getHours();
  if (night && h >= 7 && h < 19) at.setHours(5);
  if (!night && (h < 7 || h >= 19)) at.setHours(h >= 19 ? 12 : -12);
  await page.clock.setFixedTime(at);
}

test("Scene draws a sprite per agent on one page; a tap opens its chat, ringed, with its chats as chips", async ({ page, context, baseURL }) => {
  // The agents as Modern lists them, then as the scene: the same agents (other tests open and close chats meanwhile,
  // so where each goes may move).
  await page.goto("/?list");
  const rows = await page.locator(".ags .agr .tt").allTextContents();
  expect(rows.length).toBeGreaterThanOrEqual(1);
  await pickOn(context, baseURL, { style: "terminal", scene: "on" });
  await page.goto("/?list");
  await expect(page.locator(".ags .agr")).toHaveCount(0);
  await expect(page.locator("#scene .sp")).toHaveCount(1);
  // Other tests add agents meanwhile: every agent listed is a sprite, and the bar counts the sprites.
  const names = await page.locator("#scene .sb .sn b").allTextContents();
  expect(names).toEqual(expect.arrayContaining(rows));
  await expect(page.locator("#scene .fh .ds")).toHaveText(`${names.length} agents`);
  // Each sprite acts out its state with the theme's art.
  const pose = await page.locator("#scene .sb .sf use").first().getAttribute("href");
  expect(pose).toMatch(/^\/look\/farm\/sprites\.svg#s-scene-(edit|needs|done|fail|merged|idle)$/);
  await expect(page.locator("#scene .sky")).toHaveAttribute("src", /^\/look\/farm\/scene\.svg/);
  // The list's current agent is the first (a phone's ‹ names its own): ringed, its chats as chips (the approved 390).
  await expect(page.locator("#scene .sb").first()).toHaveAttribute("aria-current", "true");
  await expect(page.locator(".scc .chip").last()).toHaveText("+ new chat");
  // A tap opens the first agent's chat (or a new one), in Terminal, the style chosen before Scene.
  const first = page.locator("#scene .sb").first();
  const name = await first.locator(".sn b").textContent();
  const to = await first.getAttribute("href");
  await first.click();
  await expect(page).toHaveURL(new RegExp(`${to.replace("?", "\\?")}$`));
  await expect(page.locator("html")).toHaveAttribute("data-style", "terminal");
  // From 1000 px the scene stays beside it: that agent ringed, its chats as chips ending in + new chat.
  if (test.info().project.name !== "1440") return;
  const ringed = page.locator("#scene .sb[aria-current]");
  await expect(ringed).toHaveCount(1);
  await expect(ringed.locator(".sn b")).toHaveText(name);
  const chips = page.getByRole("navigation", { name: `${name}'s chats` }).locator(".chip");
  if (to.startsWith("/t/")) await expect(chips.first()).toHaveText(name);
  await expect(chips.last()).toHaveText("+ new chat");
});

test("refreshed 5 times, Scene keeps one sprite per agent", async ({ page, context, baseURL }) => {
  await pickOn(context, baseURL, { scene: "on" });
  await page.goto("/?list");
  const want = (await hrefs(page.locator("#scene .sb"))).length;
  for (let i = 0; i < 5; i++) {
    await page.getByRole("button", { name: "refresh" }).click();
    await expect(page.locator("#rf")).not.toHaveClass(/rfs/);
    await expect(page.locator("#scene")).toHaveCount(1);
    const keys = await hrefs(page.locator("#scene .sb"));
    expect(new Set(keys).size).toBe(keys.length);
    // Other tests add agents meanwhile, never take one away.
    expect(keys.length).toBeGreaterThanOrEqual(want);
  }
});

test("Scene is offered only with Figures Sprite; ASCII, None or the plain theme hide it", async ({ page }, info) => {
  await page.goto("/?list");
  await page.getByRole("button", { name: "look: theme and colours" }).click();
  const scene = sheet(page).locator('[data-k=style][data-v=scene]');
  await scene.click();
  // The sidebar is drawn again as the scene.
  await expect(page.locator("#scene")).toBeVisible();
  await page.getByRole("button", { name: "look: theme and colours" }).click();
  await expect(scene).toHaveAttribute("aria-checked", "true");
  await page.screenshot({ path: `../../ui-${info.project.name}/scene-look.png` });
  // Modern again: the agents' rows.
  await sheet(page).locator('[data-k=style][data-v=modern]').click();
  await expect(page.locator("#scene")).toHaveCount(0);
  await expect(page.locator(".agr").first()).toBeVisible();
  await page.getByRole("button", { name: "look: theme and colours" }).click();
  for (const f of ["ascii", "none"]) {
    await sheet(page).locator(`[data-k=figures][data-v=${f}]`).click();
    await expect(scene).toBeHidden();
  }
  await sheet(page).locator("[data-k=figures][data-v=sprite]").click();
  await expect(scene).toBeVisible();
  await sheet(page).locator("[data-k=theme][data-v=plain]").click();
  await expect(scene).toBeHidden();
  await sheet(page).locator("[data-k=theme][data-v=farm]").click();
  await page.keyboard.press("Escape");
});

for (const theme of ["farm", "kitchen", "garden"])
  for (const mode of ["dark", "light"])
    test(`Scene's audit: ${theme}, ${mode}`, async ({ page, context, baseURL }, info) => {
      // farm's vector horses are shot at 1440 too (#115).
      test.skip(info.project.name !== "390" && theme !== "farm", "the audit is at 390");
      await clock(page, false);
      await pickOn(context, baseURL, { theme, mode, scene: "on" });
      await page.goto("/?list");
      await expect(page.locator("#scene .sb").first()).toBeVisible();
      await expect.poll(() => page.locator("#scene .sky").evaluate((i) => i.complete && i.naturalWidth > 0)).toBe(true);
      expect(await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)).toBeLessThanOrEqual(0);
      const { violations } = await new AxeBuilder({ page }).analyze();
      expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
      if (theme !== "garden") await poses(page, theme);
      await page.screenshot({ path: `../../ui-${info.project.name}/scene-${theme}-${mode}.png` });
    });

test("Scene beside a chat at 1440: the sidebar is the scene, the chat keeps its style", async ({ page, context, baseURL }, info) => {
  test.skip(info.project.name !== "1440", "the side by side is from 1000 px");
  await clock(page, false);
  await pickOn(context, baseURL, { theme: "kitchen", mode: "dark", scene: "on" });
  await page.goto("/?list");
  await page.locator("#scene .sb").first().click();
  await expect(page.locator("#scene")).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("data-style", "modern");
  await expect.poll(() => page.locator("#scene .sky").evaluate((i) => i.complete && i.naturalWidth > 0)).toBe(true);
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
  await page.screenshot({ path: "../../ui-1440/scene-kitchen-chat.png" });
});

test("farm's Scene at night: the strip's night from 19:00 on this device's clock", async ({ page, context, baseURL }, info) => {
  await clock(page, true);
  await pickOn(context, baseURL, { scene: "on" });
  await page.goto("/?list");
  await expect(page.locator("#scene")).toHaveClass(/night/);
  await expect(page.locator("#scene .sky")).toHaveAttribute("src", "/look/farm/scene.svg#night");
  await expect.poll(() => page.locator("#scene .sky").evaluate((i) => i.complete && i.naturalWidth > 0)).toBe(true);
  await poses(page, "farm");
  await page.screenshot({ path: `../../ui-${info.project.name}/scene-farm-night.png` });
});
