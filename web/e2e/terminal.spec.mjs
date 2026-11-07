// The owner's terminal (crates/cooee-terminal) at 390 px, in cooee served by serve.sh over the fake shed: Face ID
// (Chromium's virtual authenticator) registers the device's passkey and opens the tab; a new shell takes typed text
// and Enter, its screen's update draws the output in the theme's green, and the key bar sits at the foot of the tab
// (on the keyboard, when one is open). An agent's terminal shows read-only, without the key bar. WebAuthn needs a domain, so the page is opened on localhost.
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

test("the terminal: Face ID opens it, a shell takes typing, the key bar sits at the foot", async ({ page, context, baseURL }, info) => {
  test.skip(info.project.name !== "390", "one device, one passkey: the 390 px audit");
  const local = baseURL.replace("127.0.0.1", "localhost");
  const cookies = await context.cookies(baseURL);
  await context.addCookies(cookies.map(({ name, value }) => ({ name, value, url: local })));
  const cdp = await context.newCDPSession(page);
  await cdp.send("WebAuthn.enable");
  await cdp.send("WebAuthn.addVirtualAuthenticator", {
    options: { protocol: "ctap2", transport: "internal", hasResidentKey: true, hasUserVerification: true, isUserVerified: true },
  });

  await page.goto(`${local}/p/terminal`);
  const tab = page.frameLocator("iframe.pf");
  // Locked: no passkey yet, so the backend's answer is 403 and cooee's own button shows over the tab.
  await expect(tab.getByRole("status")).toHaveText("Locked: unlock with Face ID to go on. Your shells keep running.");
  await page.getByRole("button", { name: "Unlock with Face ID" }).click();
  // Unlocked: no shell of the owner's yet, so the first agent's terminal shows, read-only, with no key bar.
  await expect(tab.getByRole("status")).toHaveText(/^lane-\S+ is an agent's terminal: read-only\.$/);
  await expect(tab.getByRole("toolbar", { name: "Keys" })).toBeHidden();

  await tab.getByRole("button", { name: "+ shell" }).click();
  await expect(tab.getByRole("combobox", { name: "Terminal" })).toHaveValue(/^owner-/);
  await tab.getByRole("log", { name: "Screen" }).click();
  await page.keyboard.type("echo ok-from-the-phone");
  await page.keyboard.press("Enter");
  const out = tab.locator(".scr span", { hasText: "ok-from-the-phone" });
  await expect(out).toBeVisible();
  const green = await tab.locator("html").evaluate((h) => getComputedStyle(h).getPropertyValue("--g").trim());
  await expect(out).toHaveCSS("color", await page.evaluate((c) => {
    const s = document.body.appendChild(document.createElement("span"));
    s.style.color = c;
    return getComputedStyle(s).color;
  }, green));

  const keys = tab.getByRole("toolbar", { name: "Keys" });
  for (const k of ["Esc", "Ctrl", "Tab", "up", "down", "left", "right", "control C", "paste", "history"])
    await expect(keys.getByRole("button", { name: k, exact: true })).toBeVisible();
  const bar = await keys.boundingBox();
  expect(Math.round(bar.y + bar.height)).toBe(page.viewportSize().height);
  await page.screenshot({ path: `../../ui-${info.project.name}/terminal.png` });
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
});
