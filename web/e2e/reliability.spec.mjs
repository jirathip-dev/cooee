// Reliability (#24, herdsman#553's "Reliability" rows) on the page: R5 the loading strip only after 300 ms with the
// page tappable, R16 one row per id however often it refreshes, R17 paths in text never images; and on WebKit (as
// on the iPhone) R4 the composer flush on a shrunk viewport with no page jump, R25 no flash when a sheet closes.
import { expect, test } from "@playwright/test";


// A new thread whose turn is done, open.
async function thread(page, text = "Say hello") {
  await page.goto("/#new");
  await page.getByLabel("first message").fill(text);
  await page.getByRole("button", { name: "start" }).click();
  await page.waitForURL(/\/t\//);
  await expect(page.locator("#st")).toHaveAttribute("data-st", "done");
}

test.describe("on every browser", () => {
  test.skip(({ browserName }) => browserName === "webkit", "the WebKit project runs the iPhone audits only");

  test("R5: a slow load shows the thin strip only after 300 ms, and the page stays tappable", async ({ page }) => {
    await page.clock.install();
    await thread(page);
    await page.clock.pauseAt(Date.now() + 1000);
    let release;
    const held = new Promise((ok) => { release = ok; });
    await page.route("**/messages?*", async (route) => { await held; await route.continue(); });
    await page.getByRole("button", { name: "refresh" }).click();
    await page.clock.runFor(290);
    await expect(page.locator("#ld")).not.toHaveClass(/run/);
    await page.clock.runFor(20);
    await expect(page.locator("#ld.run")).toBeVisible();
    // Nothing covers the page: the composer is what a tap on it hits, and it takes text.
    const hit = await page.locator("#box").evaluate((box) => {
      const r = box.getBoundingClientRect();
      return document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2) === box;
    });
    expect(hit).toBe(true);
    await page.locator("#box").fill("still here");
    release();
    await page.clock.runFor(100);
    await expect(page.locator("#ld")).toBeHidden();
    await page.unroute("**/messages?*");
  });

  test("R16: refreshed 5 times, each row is drawn once and the list keeps one card per thread", async ({ page }) => {
    await thread(page);
    const ids = () => page.locator("#msgs [data-id]").evaluateAll((r) => r.map((e) => e.dataset.id));
    const before = await ids();
    expect(new Set(before).size).toBe(before.length);
    for (let i = 0; i < 5; i++) {
      await page.getByRole("button", { name: "refresh" }).click();
      await expect(page.locator("#rf")).not.toHaveClass(/rfs/);
    }
    expect(await ids()).toEqual(before);
    await page.goto("/");
    const cards = () => page.locator(".fc .tr").evaluateAll((a) => a.map((e) => e.getAttribute("href")));
    const listed = await cards();
    for (let i = 0; i < 5; i++) {
      await page.getByRole("button", { name: "refresh" }).click();
      await expect(page.locator("#rf")).not.toHaveClass(/rfs/);
    }
    const again = await cards();
    expect(again).toEqual(listed);
    expect(new Set(again).size).toBe(again.length);
  });

  test("R17: paths and image words in a message stay text: no image, no file fetched", async ({ page }) => {
    const fetched = [];
    page.on("request", (r) => { if (/\.png($|\?)/.test(new URL(r.url()).pathname) && !r.url().includes("/icon-")) fetched.push(r.url()); });
    const text = "see /tmp/a.png and ../x.png ![x](../x.png) <img src=/tmp/b.png>";
    await thread(page, text);
    await expect(page.locator(".blk.me .md").first()).toHaveText(text);
    await expect(page.locator("#msgs img")).toHaveCount(0);
    expect(fetched).toEqual([]);
  });
});

test.describe("on WebKit, as on the iPhone", () => {
  test.skip(({ browserName }) => browserName !== "webkit", "WebKit only");

  test("R4: with the keyboard up (a shrunk viewport) the composer sits flush on it and the page never jumps", async ({ page }) => {
    await thread(page);
    const { width, height } = page.viewportSize();
    await page.locator("#box").focus();
    for (const shrunk of [height - 300, height - 340]) {
      await page.setViewportSize({ width, height: shrunk });
      await expect.poll(() => page.evaluate(() => innerHeight)).toBe(shrunk);
      const at = await page.evaluate(() => ({
        bottom: document.querySelector("#composer").getBoundingClientRect().bottom,
        view: visualViewport.height,
        top: document.scrollingElement.scrollTop,
        y: scrollY,
      }));
      expect(Math.abs(at.bottom - at.view)).toBeLessThanOrEqual(1);
      expect([at.top, at.y]).toEqual([0, 0]);
    }
    await page.setViewportSize({ width, height });
    expect(await page.evaluate(() => document.scrollingElement.scrollTop)).toBe(0);
  });

  test("R25: closing a sheet goes from sheet to page in one step: no frame flashes, the page never reloads", async ({ page }) => {
    await thread(page);
    await page.evaluate(() => { window.kept = true; });
    for (const name of ["settings", "look"]) {
      await page.locator(`[data-open=${name}]`).first().click();
      await expect(page.locator("#sheet")).toBeVisible();
      // Every frame from the tap: whether the sheet shows, and the page's background under it.
      await page.evaluate(() => {
        window.frames_ = [];
        const look = () => {
          const sheet = document.querySelector("#sheet");
          window.frames_.push({ sheet: !!sheet && !sheet.hidden, bg: getComputedStyle(document.body).backgroundColor });
          if (window.frames_.length < 60) requestAnimationFrame(look);
        };
        requestAnimationFrame(look);
      });
      await page.locator("#sheet [data-close]").first().click();
      await expect.poll(() => page.evaluate(() => window.frames_.length)).toBe(60);
      const frames = await page.evaluate(() => window.frames_);
      const shown = frames.map((f) => f.sheet);
      // Shown, then gone for good: never back, never a frame between.
      expect(shown.indexOf(false)).toBeGreaterThan(-1);
      expect(shown.slice(shown.indexOf(false)).every((s) => !s)).toBe(true);
      expect(new Set(frames.map((f) => f.bg)).size).toBe(1);
      expect(await page.evaluate(() => window.kept)).toBe(true);
    }
  });
});
