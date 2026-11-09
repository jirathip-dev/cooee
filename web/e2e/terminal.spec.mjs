// The owner's terminal (crates/cooee-terminal) at 390 px, in cooee served by serve.sh over the fake shed: Face ID
// (Chromium's virtual authenticator) registers the device's passkey and opens the tab; a new shell takes typed text
// and Enter, its screen's update draws the output in the theme's green, and the key bar sits at the foot of the tab
// (on the keyboard, when one is open). An agent's terminal shows read-only, without the key bar. A chat's code block
// types into a shell from its [▸ terminal] (#29), Face ID first, never Enter. WebAuthn needs a domain, so the page is opened on localhost.
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

  // #29: a reply's code block has [▸ terminal] beside [copy]. The owner's tap opens the Terminal tab; locked, it asks
  // for Face ID first, then types the block's exact text into a shell and never presses Enter.
  const sent = { insert: [], keys: [] };
  page.on("request", (r) => {
    const call = r.url().match(/\/p\/terminal\/api\/(insert|keys)$/)?.[1];
    if (call) sent[call].push(r.postDataJSON());
  });
  await page.goto(`${local}/new`);
  await page.getByLabel("message", { exact: true }).fill("echo ```sh\necho one-from-chat\necho two-from-chat\n```");
  await page.getByRole("button", { name: "send" }).click();
  await page.waitForURL(/\/t\//);
  const block = page.locator(".blk.ai .cb").last();
  await expect(block.getByRole("button", { name: "copy the sh code" })).toBeVisible();
  const term = block.getByRole("button", { name: "type the code into the terminal" });
  await expect(term).toHaveText("▸ terminal");
  await page.screenshot({ path: `../../ui-${info.project.name}/terminal-button.png` });
  const audit = await new AxeBuilder({ page }).analyze();
  expect(audit.violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
  // A script's click (an agent's markup can't make the owner's tap) keeps nothing and opens nothing.
  await term.dispatchEvent("click");
  expect(await page.evaluate(() => sessionStorage.getItem("cooee-terminal-insert"))).toBeNull();
  const refused = page.waitForResponse((r) => r.url().endsWith("/p/terminal/api/insert"));
  await term.click();
  await page.waitForURL(/\/p\/terminal$/);
  // Locked: no passkey yet, so the backend's answer is 403, nothing is typed and cooee's own button shows.
  expect((await refused).status()).toBe(403);
  await page.getByRole("button", { name: "Unlock with Face ID" }).click();
  await page.waitForURL(/\/p\/terminal#owner-/);
  const tab = page.frameLocator("iframe.pf");
  const shell = new URL(page.url()).hash.slice(1);
  await expect(tab.getByRole("combobox", { name: "Terminal" })).toHaveValue(shell);
  expect(sent.insert).toEqual(Array(2).fill({ text: "echo one-from-chat\necho two-from-chat" }));
  expect(sent.keys).toEqual([]);
  // Typed but not run: no output until the owner presses Enter themself, then both commands' (the fake shed's shell
  // has no echo, so only output shows).
  const ran = tab.locator(".scr div", { hasText: /^(one|two)-from-chat\s*$/ });
  await expect(tab.locator(".scr")).toBeVisible();
  await expect(ran).toHaveCount(0);
  await tab.getByRole("log", { name: "Screen" }).click();
  await page.keyboard.press("Enter");
  await expect(ran).toHaveCount(2);
  expect(sent.keys).toEqual([{ terminal: shell, keys: ["enter"] }]);

  // An agent's terminal shows read-only, with no key bar.
  const agent = await tab.locator("optgroup[label^=Agents] option").first().getAttribute("value");
  await tab.getByRole("combobox", { name: "Terminal" }).selectOption(agent);
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
