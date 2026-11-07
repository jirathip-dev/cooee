// Pairs this browser with the code serve.sh got from `cooee pair`; the viewport projects reuse its cookie.
import fs from "node:fs";
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

// How many colours the screen shows: a bare background is one.
async function colours(page) {
  const png = (await page.screenshot()).toString("base64");
  return page.evaluate(async (png) => {
    const img = new Image();
    img.src = `data:image/png;base64,${png}`;
    await img.decode();
    const c = new OffscreenCanvas(img.width, img.height).getContext("2d");
    c.drawImage(img, 0, 0);
    const px = new Uint32Array(c.getImageData(0, 0, img.width, img.height).data.buffer);
    return new Set(px).size;
  }, png);
}

test("an unpaired browser is sent to pairing; the code pairs it", async ({ page, request }) => {
  // Unpaired: pages go to /pair, reads and writes are refused.
  expect((await request.get("/threads/long-390/messages")).status()).toBe(401);
  expect((await request.post("/threads/long-390/send", { data: { text: "x", random_id: "r" } })).status()).toBe(401);
  await page.goto("/");
  await expect(page).toHaveURL(/\/pair$/);
  await page.screenshot({ path: "../../ui-390/pair.png", fullPage: true });
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
  // No service worker installs while pairing, to claim the page as the pair redirects.
  expect(await page.evaluate(async () => (await navigator.serviceWorker.getRegistrations()).length)).toBe(0);

  const code = fs.readFileSync(new URL("./.auth/code", import.meta.url), "utf8").trim();
  expect(code).toMatch(/^\d{6}$/);
  // The six boxes: each digit moves on and the sixth sends it (#47).
  await page.getByLabel("digit 1").click();
  await page.keyboard.type(code);
  await page.waitForURL(/\/(\?paired)?$/, { waitUntil: "commit" });
  // No black frame: from its first paint the list is drawn, not a bare background, with no reload.
  await page.screenshot({ path: "../../ui-390/paired.png" });
  expect(await colours(page)).toBeGreaterThan(8);
  await expect(page.locator(".fc .tr").first()).toBeVisible();
  await page.context().storageState({ path: new URL("./.auth/state.json", import.meta.url).pathname });
});
