// Pairing (#47, docs/design/pair): the unpaired page at 390 and 1440 in dark and light, the six boxes (typing moves
// on, a paste fills all six, the sixth sends it), a wrong and a used code, a QR link that pairs at once with the
// passkey offer after it, Settings › devices › pair a device (the QR), and the approve card's no-passkey, approved
// elsewhere and ran-out states. Codes are one at a time per server, so this runs in the 390 project only and shoots
// both widths itself. Shots: ui-<width>/pair-….png.
import { execFileSync } from "node:child_process";
import { createHash, generateKeyPairSync, randomBytes, sign } from "node:crypto";
import fs from "node:fs";
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const read = (name) => fs.readFileSync(new URL(`./.auth/${name}`, import.meta.url), "utf8").trim();
/// A new code from `cooee pair` (it drops the open one).
const newCode = () =>
  execFileSync(read("code.bin"), ["pair", "--config", read("code.config")], { encoding: "utf8" })
    .match(/^code {2}(\d{3}) (\d{3})/m)
    .slice(1)
    .join("");
const unpaired = { storageState: { cookies: [], origins: [] } };

async function audit(page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)).toBeLessThanOrEqual(0);
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
}
/// The page at 390 and 1440 (full height), audited at each.
async function shots(page, name, mode = "dark") {
  await page.emulateMedia({ colorScheme: mode, reducedMotion: "reduce" });
  await page.waitForLoadState("networkidle");
  for (const width of [390, 1440]) {
    await page.setViewportSize({ width, height: width === 390 ? 844 : 900 });
    await page.screenshot({ path: `../../ui-${width}/${name}.png`, fullPage: true });
    await audit(page);
  }
  await page.setViewportSize({ width: 390, height: 844 });
}

// Each test shoots and audits both widths, some in both schemes.
test.describe.configure({ mode: "serial", timeout: 90_000 });
test.beforeEach(({}, info) => test.skip(info.project.name !== "390", "one code at a time: the 390 project shoots both widths"));

test("the welcome: the figure, three steps and six boxes, nothing else; dark and light", async ({ browser }) => {
  const page = await (await browser.newContext(unpaired)).newPage();
  await page.goto("/pair");
  await expect(page.locator(".steps li")).toHaveCount(3);
  await expect(page.locator(".cb input")).toHaveCount(6);
  await expect(page.getByRole("button", { name: "pair", exact: true })).toBeDisabled();
  await expect(page.locator(".hint")).toHaveCount(0);
  await shots(page, "pair-dark", "dark");
  await shots(page, "pair-light", "light");
});

test("typing moves on, ⌫ goes back, a paste fills all six; a wrong code says so and clears the boxes", async ({ browser }) => {
  const page = await (await browser.newContext(unpaired)).newPage();
  const code = newCode();
  const wrong = code === "000000" ? "111111" : "000000";
  await page.goto("/pair");
  await page.getByLabel("digit 1").click();
  await page.keyboard.type("12");
  await expect(page.getByLabel("digit 3")).toBeFocused();
  await page.keyboard.press("Backspace");
  await expect(page.getByLabel("digit 2")).toBeFocused();
  await expect(page.getByLabel("digit 2")).toHaveValue("");
  // A paste into the first box fills all six, and the sixth sends it.
  await page.getByLabel("digit 1").focus();
  await page.evaluate((text) => {
    const data = new DataTransfer();
    data.setData("text", text);
    document.activeElement.dispatchEvent(new ClipboardEvent("paste", { clipboardData: data, bubbles: true, cancelable: true }));
  }, wrong);
  await page.waitForURL(/bad=wrong/);
  await expect(page.locator("#code > .pc:not([hidden])")).toContainText("That code isn't right.");
  await expect(page.locator("#code > .pc:not([hidden])")).toContainText("4 tries left this minute");
  await expect(page.getByLabel("digit 1")).toHaveValue("");
  await expect(page.getByLabel("digit 1")).toBeFocused();
  await shots(page, "pair-wrong");
});

test("a QR link pairs at once and the list offers a passkey; the used code then says it has run out", async ({ browser }) => {
  const code = newCode();
  const context = await browser.newContext(unpaired);
  const page = await context.newPage();
  await page.goto(`/pair?code=${code}`);
  await page.waitForURL(/\/$/);
  await expect(page.locator("main.list")).toBeVisible();
  await expect(page.locator("#pairing")).toContainText("Save a passkey?");
  await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" });
  await page.screenshot({ path: "../../ui-390/pair-offer.png" });
  await audit(page);
  await page.getByRole("button", { name: "not now" }).click();
  await expect(page.locator("#pairing .pc")).toHaveCount(0);

  const again = await (await browser.newContext(unpaired)).newPage();
  await again.goto(`/pair?code=${code}`);
  await again.waitForURL(/bad=expired/);
  await expect(again.locator("#code > .pc:not([hidden])")).toContainText("That code has run out.");
  await shots(again, "pair-expired");
  await again.getByRole("link", { name: "type a new code" }).click();
  await again.waitForURL(/\/pair$/);
  await expect(again.getByLabel("digit 1")).toBeVisible();
  await context.close();
});

test("settings › devices › pair a device shows the QR and its code", async ({ page }) => {
  await page.goto("/#settings/devices");
  await page.getByRole("button", { name: "pair a device" }).click();
  const panel = page.locator("#s-pair-box .pc");
  await expect(panel.locator(".bc")).toHaveText(/^\d{3} \d{3}$/);
  await expect(panel.getByRole("img", { name: /QR code/ })).toBeVisible();
  await expect(panel).toContainText("one use · expires in 9:5");
  await shots(page, "pair-qr");
});

/// A fake platform authenticator (P-256), signing as WebAuthn does for `base`: the bodies `POST /passkeys` and an
/// approve's `passkey` take.
function authenticator(base) {
  const { privateKey, publicKey } = generateKeyPairSync("ec", { namedCurve: "P-256" });
  const b64 = (b) => Buffer.from(b).toString("base64url");
  const id = b64(randomBytes(16));
  const auth = Buffer.concat([createHash("sha256").update(new URL(base).hostname).digest(), Buffer.from([5, 0, 0, 0, 1])]);
  const client = (type, challenge) => Buffer.from(JSON.stringify({ type, challenge, origin: base }));
  return {
    id,
    create: (challenge) => ({ id, key: b64(publicKey.export({ type: "spki", format: "der" })), client: b64(client("webauthn.create", challenge)), auth: b64(auth) }),
    get(challenge) {
      const c = client("webauthn.get", challenge);
      const sig = sign("sha256", Buffer.concat([auth, createHash("sha256").update(c).digest()]), privateKey);
      return { id, client: b64(c), auth: b64(auth), sig: b64(sig) };
    },
  };
}
const IPAD = { "user-agent": "Mozilla/5.0 (iPad; CPU OS 18_0 like Mac OS X) Version/18.0 Mobile/15E148 Safari/604.1" };

test("the approve card: no passkey on this device, approved elsewhere, and a request that ran out", async ({ page, browser, baseURL }) => {
  // Another paired device (B) holds the only passkey.
  const b = await browser.newContext(unpaired);
  const bPage = await b.newPage();
  await bPage.goto(`/pair?code=${newCode()}`);
  await bPage.waitForURL(/\/$/);
  const challenge = async () => (await (await b.request.post("/passkey/challenge", { data: {} })).json()).challenge;
  const key = authenticator(baseURL);
  expect((await b.request.post("/passkeys", { data: key.create(await challenge()) })).status()).toBe(200);
  const bLabel = (await (await b.request.get("/devices")).json()).find((d) => d.me).label;
  const c = await browser.newContext(unpaired);
  try {
    const { claim } = await (await c.request.post("/pair/request", { headers: IPAD, data: {} })).json();
    await page.goto("/");
    const card = page.locator("#pairing .apc");
    await expect(card).toContainText(`Approving needs a passkey. Save one on this device, or approve on ${bLabel}.`);
    await expect(card.getByRole("button", { name: "save a passkey" })).toBeVisible();
    await expect(card.getByRole("button", { name: /^approve/ })).toHaveCount(0);
    await shots(page, "approve-no-passkey");

    // B approves with its passkey, the iPad collects its cookie: the card here says so.
    const [ask] = await (await b.request.get("/pair/asks")).json();
    const answer = await b.request.post("/pair/answer", { data: { id: ask.id, yes: true, passkey: key.get(await challenge()) } });
    expect(answer.status()).toBe(200);
    expect((await c.request.post("/pair/claim", { data: { claim } })).status()).toBe(200);
    await expect(page.locator("#pairing .pc")).toContainText(`✓ iPad paired on ${bLabel}`);
    await shots(page, "approve-elsewhere");

    // A request nobody answers leaves the list after 5 minutes: the card says it ran out.
    const late = await browser.newContext(unpaired);
    const { claim: again } = await (await late.request.post("/pair/request", { headers: IPAD, data: {} })).json();
    await expect(page.locator("#pairing .apc")).toBeVisible();
    await page.clock.setFixedTime(Date.now() + 6 * 60_000);
    await late.request.post("/pair/cancel", { data: { claim: again } });
    await expect(page.locator("#pairing .pc").last()).toContainText("iPad's request expired");
    await expect(page.locator("#pairing .pc").last()).toContainText("Nobody approved it in 5 minutes. It can ask again.");
    await late.close();
  } finally {
    await b.request.post("/passkeys/remove", { data: { id: key.id } });
    const ds = await (await page.request.get("/devices")).json();
    for (const d of ds.filter((d) => !d.me && (d.label === "iPad" || d.label === bLabel)))
      await page.request.post("/devices/remove", { data: { id: d.id } });
    await b.close();
    await c.close();
  }
});
