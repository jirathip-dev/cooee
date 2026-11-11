import { execFileSync } from "node:child_process";
import fs from "node:fs";
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

test.use({ channel: "chromium" });

const shot = (page, info, name, fullPage = false) => page.screenshot({ path: `../../ui-${info.project.name}/${name}.png`, fullPage });
const config = () => fs.readFileSync(new URL("./.auth/code.config", import.meta.url), "utf8").trim();
const bin = () => fs.readFileSync(new URL("./.auth/code.bin", import.meta.url), "utf8").trim();
const cooee = (...args) => execFileSync(bin(), [...args, "--config", config()], { encoding: "utf8" });

async function audit(page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)).toBeLessThanOrEqual(0);
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
}

async function thread(page, text = "Say hello") {
  await page.goto("/new");
  await page.getByLabel("message", { exact: true }).fill(text);
  await page.getByRole("button", { name: "send" }).click();
  await page.waitForURL(/\/t\//);
  await expect(page.locator("#st")).toHaveAttribute("data-st", "done");
}

test.beforeEach(async ({ page }) => { await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" }); });

test("row 1: installable, no double-tap zoom, and offline it opens on its last copy with its time", async ({ page, context }, info) => {
  const manifest = await (await page.request.get("/manifest.webmanifest")).json();
  expect(manifest).toMatchObject({ display: "standalone", start_url: "/" });
  await page.clock.install();
  await page.goto("/");
  expect(await page.evaluate(() => getComputedStyle(document.documentElement).touchAction)).toBe("manipulation");
  await page.evaluate(() => navigator.serviceWorker.ready);
  await page.reload();
  await expect.poll(() => page.evaluate(() => !!navigator.serviceWorker.controller)).toBe(true);
  await thread(page);
  const url = page.url();
  await page.goto("/");
  await context.setOffline(true);
  await page.clock.fastForward("10:00");
  await page.reload();
  await expect(page.locator(".ags .agr").first()).toBeVisible();
  await expect(page.locator("#ld")).toBeVisible();
  await expect(page.locator("#ld .ago")).toHaveText(/^as of 1\dm ago$/);
  await shot(page, info, "parity-1-offline");
  await page.goto(url);
  await expect(page.locator(".blk.ai")).toContainText("Hello");
  await expect(page.locator("#ld .ago")).toHaveText(/^as of 1\dm ago$/);
  await context.setOffline(false);
});

test("row 2: the sync dot is green while the stream is open, amber when it drops, green again with no rows missing", async ({ page, context }, info) => {
  await thread(page);
  const dot = page.locator(".th .dt");
  await expect(dot).toHaveAttribute("data-sync", "on");
  const ids = () => page.locator("#msgs [data-id]").evaluateAll((r) => r.map((e) => Number(e.dataset.id)));
  const before = await ids();
  await context.setOffline(true);
  await expect(dot).toHaveAttribute("data-sync", "off");
  await shot(page, info, "parity-2-offline-dot");
  await context.setOffline(false);
  await expect(dot).toHaveAttribute("data-sync", "on");
  const after = await ids();
  expect(after.slice(0, before.length)).toEqual(before);
  expect(after.every((id, i) => i === 0 || id === after[i - 1] + 1)).toBe(true);
});

test("rows 3 and 5: ↻ spins while the thin strip shows and the page stays tappable; a pull and coming back reload too", async ({ page }, info) => {
  await thread(page);
  let release;
  const held = new Promise((ok) => { release = ok; });
  await page.route("**/messages?*", async (route) => { await held; await route.continue(); });
  const reloads = [];
  page.on("request", (r) => { if (r.url().includes("/messages?")) reloads.push(r.url()); });
  await page.getByRole("button", { name: "refresh" }).click();
  await expect(page.locator("#ld.run")).toBeVisible();
  await expect(page.locator("#rf")).toHaveClass(/rfs/);
  await expect(page.locator("#ld .lw")).not.toBeEmpty();
  await shot(page, info, "parity-3-strip");
  await page.locator(".th h1").click();
  await audit(page);
  release();
  await expect(page.locator("#ld")).toBeHidden();
  await expect(page.locator("#rf")).not.toHaveClass(/rfs/);
  await page.unroute("**/messages?*");

  const n = reloads.length;
  await page.locator("#msgs").evaluate((m) => { m.scrollTop = 0; });
  await page.evaluate(() => {
    const at = (y) => [new Touch({ identifier: 1, target: document.body, clientX: 100, clientY: y })];
    dispatchEvent(new TouchEvent("touchstart", { touches: at(100) }));
    dispatchEvent(new TouchEvent("touchmove", { touches: at(200) }));
  });
  await expect.poll(() => reloads.length).toBeGreaterThan(n);
});

test("row 5: the app coming back after 30 s away reloads; under 30 s it doesn't", async ({ page }) => {
  await page.clock.install();
  await thread(page);
  const reloads = [];
  page.on("request", (r) => { if (r.url().includes("/messages?")) reloads.push(r.url()); });
  const away = async (ms) => {
    await page.evaluate(() => { Object.defineProperty(document, "hidden", { value: true, configurable: true }); document.dispatchEvent(new Event("visibilitychange")); });
    await page.clock.fastForward(ms);
    await page.evaluate(() => { Object.defineProperty(document, "hidden", { value: false, configurable: true }); document.dispatchEvent(new Event("visibilitychange")); });
  };
  await away(5000);
  await page.waitForLoadState("networkidle");
  expect(reloads).toEqual([]);
  await away(31000);
  await expect.poll(() => reloads.length).toBeGreaterThan(0);
});

test("row 4: an action's toast is one short line on screen, then it clears", async ({ page }, info) => {
  await page.clock.install();
  await thread(page);
  await page.getByRole("button", { name: "refresh" }).click();
  await page.locator("#box").fill("/report");
  await page.getByRole("button", { name: "send" }).click();
  const toast = page.locator("#toast");
  await expect(toast).toHaveText("report: tap the part that's wrong");
  const box = await toast.boundingBox();
  expect(box.x).toBeGreaterThanOrEqual(0);
  expect(box.x + box.width).toBeLessThanOrEqual(page.viewportSize().width);
  expect(box.height).toBeLessThan(40);
  await shot(page, info, "parity-4-toast");
  await page.clock.runFor(3000);
  await expect(toast).toBeHidden();
});

async function onLocalhost(browser, info, baseURL, paired = true) {
  const state = JSON.parse(fs.readFileSync(new URL("./.auth/state.json", import.meta.url), "utf8"));
  const host = new URL(baseURL).port;
  const cookies = paired ? state.cookies.map((c) => ({ ...c, domain: "localhost" })) : [];
  const context = await browser.newContext({ viewport: viewport(info), baseURL: `http://localhost:${host}`, storageState: { cookies, origins: [] } });
  return [context, await context.newPage()];
}

test("row 6 (#47): a new device asks, a paired one approves with its passkey (a virtual authenticator) or declines", async ({ browser, baseURL }, info) => {
  test.setTimeout(90_000);
  const [paired, phone] = await onLocalhost(browser, info, baseURL);
  await phone.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" });
  const cdp = await paired.newCDPSession(phone);
  await cdp.send("WebAuthn.enable");
  await cdp.send("WebAuthn.addVirtualAuthenticator", {
    options: { protocol: "ctap2", transport: "internal", hasResidentKey: true, hasUserVerification: true, isUserVerified: true },
  });
  await phone.goto("/?paired");
  await expect(phone).toHaveURL(/\/$/);
  const offer = phone.locator("#pairing .pc").filter({ hasText: "Save a passkey?" });
  await offer.getByRole("button", { name: "save a passkey" }).click();
  await expect(phone.locator("#pairing")).toContainText("✓ passkey saved");

  const [fresh, ipad] = await onLocalhost(browser, info, baseURL, false);
  await ipad.goto("/pair");
  await ipad.getByRole("button", { name: "ask to pair" }).click();
  await expect(ipad.locator("#ask-word")).toHaveText(/^[a-z]+ [a-z]+$/);
  const word = (await ipad.locator("#ask-word").textContent()).trim();
  await expect(ipad.locator("#ask-wait")).toContainText("waiting for approval");
  await expect(ipad.locator("#ask-word")).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
  await shot(ipad, info, "pair-waiting", true);
  await audit(ipad);

  const card = phone.getByRole("group", { name: new RegExp(`asks to pair: ${word}$`) });
  await expect(card).toContainText("Pair a new device?");
  await shot(phone, info, "pair-approve-card");
  await audit(phone);
  await card.getByRole("button", { name: /^approve with/ }).click();
  await expect(card).toContainText("It's in settings › devices.");
  await expect(card).toHaveAttribute("data-k", "g");
  await ipad.waitForURL(/\/$/);
  await expect(ipad.locator(".ags")).toBeVisible();
  await card.getByRole("button", { name: "rename it" }).click();
  await card.getByRole("textbox", { name: "its name" }).fill("Kim's iPad");
  await card.getByRole("button", { name: "save" }).click();
  await expect(card.locator(".et")).toHaveText("✓ Kim's iPad paired");
  const named = await (await ipad.request.get("/devices")).json();
  expect(named.find((d) => d.me).label).toBe("Kim's iPad");

  const [, other] = await onLocalhost(browser, info, baseURL, false);
  await phone.clock.install();
  await other.goto("/pair");
  await other.getByRole("button", { name: "ask to pair" }).click();
  await expect(other.locator("#ask-word")).toHaveText(/^[a-z]+ [a-z]+$/);
  const word2 = (await other.locator("#ask-word").textContent()).trim();
  const card2 = phone.getByRole("group", { name: new RegExp(`asks to pair: ${word2}$`) }).last();
  await card2.getByRole("button", { name: "decline" }).click();
  await expect(card2).toContainText("Declined");
  await expect(card2).toContainText("for 10 s");
  await phone.clock.runFor(10500);
  await expect(other.locator("#ask-no")).toContainText("said no.");
  await expect(other.locator("#ask-no")).toContainText(`(${word2}) was declined`);
  await shot(other, info, "pair-declined", true);
  await audit(other);
  await other.locator("#ask-no").getByRole("link", { name: "use a code" }).click();
  await expect(other.getByLabel("digit 1")).toBeVisible();
  await expect(other.locator("#ask-no")).toBeHidden();

  await ipad.goto("/#settings/devices");
  await expect(ipad.locator("#s-passkeys-box .pkr").first()).toBeVisible();
  await expect(ipad.locator("#s-passkeys-box .pkr button").first()).not.toHaveCSS("box-shadow", "none");
  const mine = ipad.locator("#s-devices-box .lrw").filter({ hasText: "this device" });
  await expect(mine).toHaveCount(1);
  await shot(ipad, info, "parity-6-devices");
  ipad.on("dialog", (d) => d.accept());
  await mine.getByRole("button").click();
  await ipad.waitForURL(/\/pair$/);
  expect((await ipad.request.get("/devices")).status()).toBe(401);
  await paired.close();
  await fresh.close();
});
test("row 7: an unpaired browser gets the shell only: pairing, no thread data", async ({ browser }, info) => {
  const context = await browser.newContext({ viewport: viewport(info), storageState: { cookies: [], origins: [] } });
  const page = await context.newPage();
  await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" });
  await page.goto(`/t/long-${info.project.name}`);
  await expect(page).toHaveURL(/\/pair$/);
  await expect(page.locator("main")).toContainText("pair this device");
  await expect(page.locator("header")).not.toContainText("⚙");
  expect(await page.content()).not.toContain("Long thread");
  expect((await page.request.get(`/threads/long-${info.project.name}/messages`)).status()).toBe(401);
  await shot(page, info, "parity-7-shell", true);
  await audit(page);
  await context.close();
});
const viewport = (info) => (info.project.name === "390" ? { width: 390, height: 844 } : { width: 1440, height: 900 });

test("rows 8 and 13: Settings opens at a section from its link, has an index, and says truly that alerts are off", async ({ page, context }, info) => {
  await context.grantPermissions(["notifications"]);
  await page.goto("/#settings/alerts");
  const sheet = page.locator("#sheet .sh:popover-open");
  await expect(sheet.locator(".six a")).toHaveText(["sign-ins", "folders", "look", "alerts", "devices", "plugins", "report"]);
  await expect(sheet.locator("#s-alerts")).toBeInViewport();
  await expect(sheet.locator("#s-alerts-box")).toContainText(/alerts are off|can't show alerts/);
  if (await page.evaluate(() => "PushManager" in window)) {
    await expect(sheet.getByRole("button", { name: "turn on" })).toBeVisible();
    await expect(page.locator("header [data-open=settings]")).toHaveAttribute("data-dot", "");
  }
  await shot(page, info, "parity-13-settings");
  await audit(page);
  await sheet.locator(".six a", { hasText: "devices" }).click();
  await expect(page).toHaveURL(/#settings\/devices$/);
  await expect(sheet.locator("#s-devices")).toBeInViewport();
  await page.keyboard.press("Escape");
  await expect(page).toHaveURL(/\/$/);
});

test("row 10: a theme with moments plays one when a turn ends live; terminal plays none", async ({ page, context, baseURL }, info) => {
  await thread(page);
  const bar = page.locator(".th");
  const turn = async () => {
    await page.locator("#box").fill("Say hello");
    await page.getByRole("button", { name: "send" }).click();
  };
  await turn();
  await expect(bar).toHaveAttribute("data-mo", "done");
  await expect(bar.locator(".mot")).toHaveText("over the fence");
  await shot(page, info, "parity-10-moment");
  await context.addCookies([{ name: "cooee-look", value: "theme:terminal", url: baseURL }]);
  await page.reload();
  await expect(page.locator("#st")).toHaveAttribute("data-st", "done");
  const played = page.evaluate(() => new Promise((ok) => {
    new MutationObserver(() => ok(true)).observe(document.querySelector(".th"), { attributeFilter: ["data-mo"] });
    new MutationObserver((_, o) => { if (document.querySelector("#st").dataset.st === "done" && document.querySelectorAll(".blk.ai").length > 1) { o.disconnect(); setTimeout(() => ok(false), 100); } })
      .observe(document.querySelector("#st"), { attributes: true });
  }));
  await turn();
  expect(await played).toBe(false);
});

test("row 12: ascii figures replace the sprites and stay", async ({ page }, info) => {
  await page.goto("/");
  await page.getByRole("button", { name: "look: theme and colours" }).click();
  await page.locator('#sheet [data-k="figures"][data-v="ascii"]').click();
  await page.keyboard.press("Escape");
  await expect(page.locator(".tp .rsp")).toHaveCount(0);
  expect(await page.locator(".tp .rsa").count()).toBeGreaterThan(0);
  await expect(page.locator(".tp .rsa b").first()).toHaveText(/^\S{3}$/);
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-figures", "ascii");
  await shot(page, info, "parity-12-ascii");
  await audit(page);
});

test("row 14: report mode: tap an element, add a note, send; cooee reports lists it", async ({ page }, info) => {
  await page.goto("/#settings/report");
  await page.getByRole("button", { name: "report a problem" }).click();
  await expect(page.locator("#sheet")).toBeHidden();
  await page.locator(".bar h1").click();
  await expect(page.locator(".bar h1")).toHaveClass(/repo/);
  const note = `the title is off (${info.project.name})`;
  await page.getByLabel("what's wrong").fill(note);
  await shot(page, info, "parity-14-report");
  await page.locator(".repf").getByRole("button", { name: "send" }).click();
  await expect(page.locator("#toast")).toHaveText("report sent");
  await expect(page.locator(".repf")).toHaveCount(0);
  const listed = cooee("reports");
  expect(listed).toContain(note);
  expect(listed).toContain("header.bar > h1");
});
