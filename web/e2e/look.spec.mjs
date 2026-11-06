// The look (#8, docs/design/look): a device's pick (its cookie, #49) drawn by the server from the first frame, the look
// sheet and Settings, System following the phone live, the four Style × Sprites combinations at 390, the scroll kept on
// a switch, and the look reaching a plugin tab.
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const shot = (page, info, name, fullPage = false) => page.screenshot({ path: `../../ui-${info.project.name}/${name}.png`, fullPage });
const bg = (page) => page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--bg").trim());
// The open sheet (the sheets are popovers in #sheet).
const sheet = (page) => page.locator("#sheet .sh:popover-open");
// A device's pick, as its `cooee-look` cookie keeps it (`theme:kitchen/style:terminal`).
const pickOn = (context, url, pick) =>
  context.addCookies([{ name: "cooee-look", value: Object.entries(pick).map(([k, v]) => `${k}:${v}`).join("/"), url }]);
const pickOf = async (context) =>
  Object.fromEntries((await context.cookies()).find((c) => c.name === "cooee-look")?.value.split("/").map((kv) => kv.split(":")) ?? []);
// The theme tiles' icons have loaded.
const icons = (page) => expect.poll(() => page.locator(".lth img").evaluateAll((l) => l.every((i) => i.complete && i.naturalWidth > 0))).toBe(true);
// A key in the look sheet: style, sprites, theme, mode (appearance), dark or light (colours), and its value.
const tap = (page, k, v) => sheet(page).locator(`[data-k="${k}"][data-v="${v}"]`).click();

async function audit(page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)).toBeLessThanOrEqual(0);
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
}

// A thread that waits on you, as the design's shots show.
async function blocked(page) {
  await page.goto("/#new");
  await page.getByLabel("first message").fill("approve an edit");
  await page.getByRole("button", { name: "start" }).click();
  await page.waitForURL(/\/t\//);
  await expect(page.locator(".blk.ap")).toBeVisible();
}

test.beforeEach(async ({ page }) => { await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" }); });

test("a device's pick shows from the first frame, and stays on this device only", async ({ page, browser, context, baseURL }) => {
  await pickOn(context, baseURL, { theme: "kitchen", style: "terminal", sprites: "off" });
  await page.addInitScript(() => {
    // The first frame, before anything else paints.
    requestAnimationFrame(() => {
      const h = document.documentElement;
      window.first = { bg: getComputedStyle(h).getPropertyValue("--bg").trim(), style: h.dataset.style, sprites: h.dataset.sprites };
    });
  });
  await page.goto("/");
  await expect.poll(() => page.evaluate(() => window.first)).toEqual({ bg: "#1a1816", style: "terminal", sprites: "off" });
  // A reload draws it from the first frame too: no flash of the default look.
  await page.reload();
  await expect.poll(() => page.evaluate(() => window.first)).toEqual({ bg: "#1a1816", style: "terminal", sprites: "off" });
  await expect(page.locator("body")).toHaveCSS("font-family", /monospace/);
  // Kitchen's words, sprites off: a dot, then the word.
  await blocked(page);
  await expect.poll(() => page.evaluate(() => window.first)).toMatchObject({ bg: "#1a1816", style: "terminal" });
  await expect(page.locator("#st")).toHaveText("the bell");
  await expect(page.locator("#st .sd")).toBeVisible();
  await page.goto("/");
  await expect(page.locator(".tp [data-st=needs]").first()).toHaveText("the bell");
  await expect(page.locator(".tp .rsp")).toHaveCount(0);
  // Another device (a fresh context) keeps the server's default: farm, Modern, sprites on.
  const other = await browser.newPage();
  await other.emulateMedia({ colorScheme: "dark" });
  await other.goto(page.url());
  await expect(other.locator("html")).toHaveAttribute("data-style", "modern");
  expect(await bg(other)).toBe("#0f1a13");
  await other.close();
});

test("the look sheet: style, sprites, theme, appearance and colours apply at once and stay", async ({ page, context }, info) => {
  await blocked(page);
  await page.getByRole("button", { name: "look: theme and colours" }).click();
  await expect(sheet(page)).toBeVisible();
  await expect(page.getByRole("button", { name: "look: theme and colours" })).toHaveAttribute("aria-pressed", "true");
  await expect(sheet(page).locator('[data-k=theme][data-v=farm]')).toHaveAttribute("aria-selected", "true");
  await icons(page);
  await shot(page, info, "look-modern-dark");
  await audit(page);

  await tap(page, "style", "terminal");
  await expect(page.locator("html")).toHaveAttribute("data-style", "terminal");
  await shot(page, info, "look-terminal-dark");
  await tap(page, "style", "modern");

  await tap(page, "theme", "kitchen");
  await expect(page.locator("#st")).toHaveText("the bell");
  expect(await bg(page)).toBe("#1a1816");

  // Light: kitchen's light default; a tap in the dark list switches Appearance back to dark.
  await tap(page, "mode", "light");
  expect(await bg(page)).toBe("#f8f5f0");
  await expect(sheet(page).getByText("light colours · showing")).toBeVisible();
  await shot(page, info, "look-kitchen-light");
  await tap(page, "dark", "sea");
  await expect(sheet(page).getByRole("radio", { name: "dark" })).toHaveAttribute("aria-checked", "true");
  expect(await bg(page)).toBe("#2e3440");
  // The default row forgets the pick.
  await tap(page, "dark", "kitchen");
  expect(await bg(page)).toBe("#1a1816");

  await tap(page, "sprites", "off");
  await expect(page.locator("#st .sd")).toBeVisible();
  await page.reload();
  expect(await pickOf(context)).toEqual({ theme: "kitchen", mode: "dark", style: "modern", sprites: "off" });
  await expect(page.locator("#st")).toHaveText("the bell");
  await page.keyboard.press("Escape");
});

test("System follows the phone's mode live", async ({ page, context, baseURL }) => {
  await page.goto("/");
  expect(await bg(page)).toBe("#0f1a13");
  await page.emulateMedia({ colorScheme: "light" });
  await expect.poll(() => bg(page)).toBe("#f5efda");
  await expect(page.locator("html")).toHaveAttribute("data-mode", "light");
  // A forced mode ignores the phone.
  await pickOn(context, baseURL, { mode: "dark" });
  await page.reload();
  expect(await bg(page)).toBe("#0f1a13");
  await page.emulateMedia({ colorScheme: "dark" });
  await page.emulateMedia({ colorScheme: "light" });
  expect(await bg(page)).toBe("#0f1a13");
});

test("Settings: the look rows show the pick and open the look sheet", async ({ page }, info) => {
  await page.goto("/");
  await page.getByRole("button", { name: "settings" }).click();
  const rows = sheet(page).locator(".sr");
  await expect(rows.locator(".sk")).toHaveText(["providers", "roots", "style", "sprites", "theme", "appearance", "colours", "report a problem"]);
  await expect(rows.locator(".sv")).toHaveText(["modern", "on", "farm", "system · dark now", "dark farm · light farm"]);
  await shot(page, info, "settings-dark");
  await audit(page);
  await rows.filter({ hasText: "theme" }).click();
  await expect(sheet(page).getByText("dark colours · showing")).toBeVisible();
});

test("all four Style × Sprites combinations render at 390", async ({ page, context, baseURL }, info) => {
  test.skip(info.project.name !== "390", "the combinations are a phone audit");
  for (const style of ["modern", "terminal"]) {
    for (const sprites of ["on", "off"]) {
      await pickOn(context, baseURL, { style, sprites });
      await page.goto("/");
      await expect(page.locator("html")).toHaveAttribute("data-style", style);
      await expect(page.locator(".tp .rsp")).toHaveCount(sprites === "on" ? await page.locator(".tp [data-st]:not([data-st=''])").count() : 0);
      await audit(page);
      await shot(page, info, `combo-${style}-sprites-${sprites}-list`, true);
      await blocked(page);
      if (sprites === "on") {
        // The live line's sprite draws from the theme's sheet: it differs from the same box with its <use> hidden.
        const sprite = page.locator("#wait .rsp");
        await expect(sprite.locator("use")).toHaveAttribute("href", "/look/farm/sprites.svg#s-needs");
        const drawn = await sprite.screenshot();
        await sprite.locator("use").evaluate((u) => { u.style.visibility = "hidden"; });
        expect(drawn.equals(await sprite.screenshot())).toBe(false);
        await sprite.locator("use").evaluate((u) => { u.style.visibility = ""; });
      } else await expect(page.locator("#wait .sd")).toBeVisible();
      await audit(page);
      await shot(page, info, `combo-${style}-sprites-${sprites}-chat`);
    }
  }
});

test("switching style or sprites keeps the scroll position", async ({ page }, info) => {
  await page.goto(`/t/long-${info.project.name}`);
  await expect(page.locator("#msgs .unread")).toBeVisible();
  const list = page.locator("#msgs");
  // Mid-thread, with the top row cut through its middle.
  await list.evaluate((l) => {
    l.scrollTop = l.scrollHeight / 2;
    const r = [...l.children].find((r) => r.getBoundingClientRect().bottom > l.getBoundingClientRect().top);
    l.scrollTop += r.getBoundingClientRect().top - l.getBoundingClientRect().top + r.offsetHeight / 2;
  });
  // The first row in view, and how much of it is above the list's top edge (its share of the row's height): a
  // switch restyles every row, so the same row stays first in view, cut at the same share.
  const view = () => list.evaluate((l) => {
    const top = l.getBoundingClientRect().top, r = [...l.children].find((r) => r.getBoundingClientRect().bottom > top);
    const b = r.getBoundingClientRect();
    return { id: r.dataset.id, cut: Math.round(((top - b.top) / b.height) * 50) / 50 };
  });
  const before = await view();
  expect(before.cut).toBeGreaterThan(0);
  await page.getByRole("button", { name: "look: theme and colours" }).click();
  for (const [k, v] of [["style", "terminal"], ["sprites", "off"], ["style", "modern"], ["sprites", "on"]]) {
    const height = await list.evaluate((l) => l.scrollHeight);
    await tap(page, k, v);
    // The switch did move the rows: the thread's height changed.
    if (k === "style") expect(await list.evaluate((l) => l.scrollHeight)).not.toBe(height);
    expect(await view(), `${k} ${v}`).toEqual(before);
  }
});

test("a plugin tab gets the theme's variables, data URLs, style and sprites, and every change", async ({ page, request }) => {
  await page.goto("/");
  // A plugin tab as M4 will mount one: it asks for the look and keeps every answer.
  await page.evaluate(() => {
    const f = document.createElement("iframe");
    f.dataset.plugin = "drover";
    f.title = "drover";
    f.srcdoc = `<script>window.got = []; addEventListener("message", (e) => got.push(e.data)); parent.postMessage({ type: "cooee-look?" }, "*");<\/script>`;
    document.body.append(f);
  });
  const tab = page.frameLocator("iframe[data-plugin]");
  const got = () => page.frames().find((f) => f !== page.mainFrame()).evaluate(() => window.got);
  await expect.poll(async () => (await got())?.length).toBe(1);
  const [look] = await got();
  expect(look).toMatchObject({ type: "cooee-look", theme: "farm", mode: "dark", style: "modern", sprites: "on" });
  expect(look.vars["--bg"]).toBe("#0f1a13");
  expect(look.words.needs).toBe("at the gate");
  for (const [k, type] of [["sprites", "image/svg+xml"], ["icon", "image/png"], ["theme", "application/json"]]) {
    const r = await request.get(look.urls[k]);
    expect(r.ok(), k).toBe(true);
    expect(r.headers()["content-type"]).toBe(type);
  }
  expect(await (await request.get(look.urls.theme)).json()).toMatchObject({ label: "farm", words: { needs: "at the gate" } });
  expect(tab).toBeTruthy();
  await page.getByRole("button", { name: "look: theme and colours" }).click();
  await tap(page, "style", "terminal");
  await expect.poll(async () => (await got()).at(-1).style).toBe("terminal");
  await tap(page, "theme", "terminal");
  await expect.poll(async () => (await got()).at(-1)).toMatchObject({ theme: "terminal", sprites: "on", urls: { sprites: null } });
  expect((await got()).at(-1).vars["--bg"]).toBe("#0d1117");
});
