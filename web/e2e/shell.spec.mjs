// The native shell (#118, docs/design/native-shell): no header; the tab bar (Chats · Workflow · drover's Queue and
// Asks · More, the other plugin tabs under More), the 88 px rail from 1000 px; each screen's large title scrolls under
// its nav bar, which then shows the inline title; More's grouped list; a chat swiped back from the left edge; press
// tints at touch-start. Shot at 390 and 1440 in light and dark, the frames of the approved PNGs.
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const shot = (page, info, name) => page.screenshot({ path: `../../ui-${info.project.name}/${name}.png` });
const audit = async (page) => {
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
};
const tabs = (page) => page.getByRole("navigation", { name: "tabs" }).getByRole("link");
// A finger on the chat: touches at x, as WebKit sends them.
const touch = (page, type, x) =>
  page.locator(".cfm .chat").evaluate((el, [type, x]) => {
    const e = Object.assign(new Event(type, { bubbles: true }), { touches: type === "touchend" ? [] : [{ clientX: x, clientY: 400 }] });
    el.dispatchEvent(e);
  }, [type, x]);

for (const mode of ["light", "dark"]) {
  test(`${mode}: no header; the tab bar, the large title and its collapse; More`, async ({ page }, info) => {
    const phone = info.project.name === "390";
    await page.emulateMedia({ colorScheme: mode, reducedMotion: "reduce" });
    await page.goto("/?list");
    await expect(page.locator("header.bar")).toHaveCount(0);
    await expect(page.getByRole("button", { name: "refresh" })).toHaveCount(0);
    // Workflow's dot while a lane waits on you (the fake shed's does), and More's (Machines' plugin dot).
    const rows = await (await page.request.get("/threads")).json();
    expect(rows.some((r) => r.workflow && r.state === "blocked")).toBe(true);
    await expect(tabs(page)).toHaveText(["Chats", "Workflow (needs you)", "Queue", "Asks", /^More/]);
    await expect(tabs(page).first()).toHaveAttribute("aria-current", "page");
    await expect(tabs(page).nth(2)).toHaveAttribute("href", "/p/hello");
    const lt = page.locator(".lt");
    await expect(lt).toHaveText("Chats");
    await expect(lt).toHaveCSS("font-size", "34px");
    await expect(lt).toHaveCSS("line-height", "41px");
    const bar = await page.locator(".tbar").boundingBox();
    if (phone) {
      // Over the bottom edge, 49 px (no inset in the test browser), full width.
      expect(bar).toMatchObject({ x: 0, y: 844 - 49, width: 390, height: 49 });
    } else {
      // The rail: 88 px on the left, then the 380 px list.
      expect(bar).toMatchObject({ x: 0, y: 0, width: 88, height: 900 });
      await shot(page, info, `chats-${mode}`);
      expect(await page.locator(".sdb").boundingBox()).toMatchObject({ x: 88, width: 380 });
    }
    const nb = page.locator(".sdb > .nb");
    await expect(nb).toHaveCSS("height", "44px");
    await expect(nb).not.toHaveClass(/\bsc\b/);
    await expect(nb.locator(".nt")).toHaveCSS("opacity", "0");
    await shot(page, info, `chats-${mode}`);
    await audit(page);

    // Scrolled under the nav bar: the inline title shows, the bar takes its blur and hairline.
    await page.locator(".sdb").evaluate((s) => {
      s.style.minHeight = "0";
      s.append(Object.assign(document.createElement("div"), { style: "height:2000px" }));
      s.scrollTop = 96;
    });
    await expect(nb).toHaveClass(/\bsc\b/);
    await expect(nb.locator(".nt")).toHaveCSS("opacity", "1");
    await expect(nb).toHaveCSS("backdrop-filter", /blur\(20px\)/);
    if (phone) await shot(page, info, `chats-scrolled-${mode}`);
    await page.locator(".sdb").evaluate((s) => { s.scrollTop = 0; });
    await expect(nb).not.toHaveClass(/\bsc\b/);

    // More: the look, alerts, the other plugin tabs, sign-ins and Settings; ◐ and ⚙ open from here.
    await tabs(page).filter({ hasText: "More" }).click();
    await expect(page).toHaveURL(/\/more$/);
    await expect(page.locator("h1.lt")).toHaveText("More");
    // Other specs may add a plugin meanwhile: these in this order, drover's two never.
    await expect(page.locator(".gs .sk")).toContainText(["Look", "Notifications", "Machines", "Terminal", "Sign-ins", "Settings"]);
    await expect(page.locator(".gs .sk").filter({ hasText: /^(Queue|Asks)$/ })).toHaveCount(0);
    // Group headers line up with the group's edge; Terminal has its own dark ›_ tile (more-390-*).
    const x = async (l) => (await l.boundingBox()).x;
    expect(await x(page.locator(".sdb>h2.ch").first())).toBe(await x(page.locator(".sdb>.gs").first()));
    const term = page.getByRole("link", { name: "Terminal" }).locator(".ms");
    await expect(term).toHaveText("›_");
    await expect(term).toHaveCSS("background-color", await page.locator("body").evaluate((b) => getComputedStyle(b).color));
    await expect(tabs(page).last()).toHaveAttribute("aria-current", "page");
    await shot(page, info, `more-${mode}`);
    await audit(page);
    await page.getByRole("button", { name: /^Look/ }).click();
    await expect(page.locator("#sh-look")).toBeVisible();
    await page.keyboard.press("Escape");
    await page.getByRole("button", { name: "Settings" }).click();
    await expect(page.locator("#sh-settings")).toBeVisible();
    await page.keyboard.press("Escape");
    // A plugin tab under More is pushed from it: on a phone `‹ More` and no tab bar, at 1440 More current in the rail.
    await page.getByRole("link", { name: "Terminal" }).click();
    await expect(page).toHaveURL(/\/p\/terminal$/);
    if (phone) {
      await expect(page.locator(".tbar")).toBeHidden();
      await page.getByRole("link", { name: "‹ More" }).click();
      await expect(page).toHaveURL(/\/more$/);
    } else {
      await expect(page.getByRole("link", { name: "‹ More" })).toBeHidden();
      await expect(tabs(page).last()).toHaveAttribute("aria-current", "page");
    }
    // drover's Queue is a tab of its own.
    await tabs(page).filter({ hasText: "Queue" }).click();
    await expect(page).toHaveURL(/\/p\/hello$/);
    await expect(tabs(page).nth(2)).toHaveAttribute("aria-current", "page");
    await expect(page.locator(".nb.sc .nt")).toHaveText("Queue");
    await expect(page.getByRole("link", { name: "‹ More" })).toHaveCount(0);
  });
}

test("Workflow lists the lanes' chats under its large title; a lane opens as its chat", async ({ page }, info) => {
  await page.goto("/");
  await tabs(page).filter({ hasText: "Workflow" }).click();
  await expect(page).toHaveURL(/\/w$/);
  await expect(page.locator("h1.lt")).toHaveText("Workflow");
  await expect(tabs(page).nth(1)).toHaveAttribute("aria-current", "page");
  const lane = page.getByRole("navigation", { name: "lanes" }).getByRole("link", { name: /cooee #33/ });
  await expect(lane).toBeVisible();
  await shot(page, info, "workflow");
  await audit(page);
  await lane.click();
  await expect(page.getByRole("heading", { level: 1 })).toHaveText("cooee #33");
});

test("a row tints at touch-start, before any tap", async ({ page }) => {
  await page.goto("/more");
  const row = page.getByRole("button", { name: "Settings" });
  const press = await page.evaluate(() => {
    const p = document.createElement("i");
    p.style.background = "var(--press)";
    document.body.append(p);
    return getComputedStyle(p).backgroundColor;
  });
  await expect(row).not.toHaveCSS("background-color", press);
  await row.hover();
  await page.mouse.down();
  await expect(row).toHaveCSS("background-color", press);
  await page.mouse.up();
});

test("a pushed chat: no tab bar; swiped from the left edge it follows the finger, and past a third it goes back", async ({ page }, info) => {
  test.skip(info.project.name !== "390", "swipe back is a phone's");
  await page.clock.install();
  for (const mode of ["light", "dark"]) {
    await page.emulateMedia({ colorScheme: mode, reducedMotion: "reduce" });
    await page.goto(`/t/long-390`);
    await expect(page.locator(".tbar")).toBeHidden();
    await expect(page.locator(".sdb")).toBeHidden();
    // A short drag springs back.
    await touch(page, "touchstart", 5);
    await touch(page, "touchmove", 60);
    await page.clock.runFor(500);
    await expect(page.locator(".cfm .chat")).toHaveCSS("transform", "matrix(1, 0, 0, 1, 55, 0)");
    await expect(page.locator(".app")).toHaveClass(/\bswp\b/);
    await expect(page.locator(".sdb")).toBeVisible();
    await touch(page, "touchend", 0);
    await page.clock.runFor(300);
    await expect(page.locator(".app")).not.toHaveClass(/\bswp\b/);
    await expect(page.locator(".cfm .chat")).toHaveCSS("transform", "none");
    await expect(page).toHaveURL(/\/t\/long-390$/);
    // Mid-gesture at 45 %: the list under it from -30 %, with its tab bar, under the dim.
    await touch(page, "touchstart", 5);
    await touch(page, "touchmove", 180);
    await expect(page.locator(".tbar")).toBeVisible();
    const sx = await page.locator(".app").evaluate((a) => a.style.getPropertyValue("--sx"));
    expect(Number.parseFloat(sx)).toBeCloseTo(-30 * (1 - 175 / 390), 1);
    await shot(page, info, `chat-swipe-${mode}`);
    // Let go past a third of the width: back to the chats.
    await touch(page, "touchend", 0);
    await page.clock.runFor(300);
    await page.waitForURL(/\/\?list/);
    await expect(page.locator(".tbar")).toBeVisible();
  }
});
