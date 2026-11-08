import { chromium } from "@playwright/test";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
const out = process.env.OUT, P = process.env.P;
const ip = `http://127.0.0.1:${P}`, lh = `http://localhost:${P}`;
const rd = (n) => fs.readFileSync(`${out}/${n}`, "utf8").trim();
const newCode = () => execFileSync(rd("code.bin"), ["pair", "--config", rd("code.config")], { encoding: "utf8" }).match(/^code {2}(\d{3}) (\d{3})/m).slice(1).join("");
const log = (...a) => console.log(...a);
const IPHONE = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1";
const IPAD = "Mozilla/5.0 (iPad; CPU OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1";
const browser = await chromium.launch();
const unpaired = { storageState: { cookies: [], origins: [] } };
async function ctx(base, extra = {}) { const c = await browser.newContext({ baseURL: base, viewport: { width: 390, height: 844 }, ...unpaired, ...extra }); return c; }
async function shots(page, name, mode = "dark", widths = [390, 1440]) {
  await page.emulateMedia({ colorScheme: mode, reducedMotion: "reduce" });
  for (const w of widths) {
    await page.setViewportSize({ width: w, height: w === 390 ? 844 : 900 });
    await page.waitForTimeout(150);
    await page.screenshot({ path: `${out}/shots/${w}-${name}.png`, fullPage: true });
  }
  await page.setViewportSize({ width: 390, height: 844 });
  log(`shot ${name} (${mode})`);
}
async function authn(page) {
  const s = await page.context().newCDPSession(page);
  await s.send("WebAuthn.enable");
  const { authenticatorId } = await s.send("WebAuthn.addVirtualAuthenticator", { options: { protocol: "ctap2", transport: "internal", hasResidentKey: true, hasUserVerification: true, isUserVerified: true, automaticPresenceSimulation: true } });
  return { s, authenticatorId };
}
const step = async (name, fn) => { try { await fn(); log(`OK   ${name}`); } catch (e) { log(`FAIL ${name}: ${e.message.split("\n")[0]}`); } };

// A. Unpaired shell
await step("A unpaired welcome: shell only", async () => {
  const page = await (await ctx(ip)).newPage();
  await page.goto("/t/long-390");
  log("  unpaired /t/long-390 ->", new URL(page.url()).pathname);
  const r = await page.request.get("/threads"); log("  unpaired GET /threads ->", r.status());
  const txt = await page.locator("body").innerText();
  log("  body mentions 'Long thread':", txt.includes("Long thread"));
  log("  steps:", await page.locator(".steps li").count(), "boxes:", await page.locator(".cbx input").count(), "hint:", await page.locator(".hint").count());
  log("  header buttons:", JSON.stringify(await page.locator("header").getByRole("button").allInnerTexts()), "links:", JSON.stringify(await page.locator("header").getByRole("link").allInnerTexts()));
  log("  pair disabled:", await page.getByRole("button", { name: "pair", exact: true }).isDisabled());
  log("  passkey row:", await page.getByRole("button", { name: "sign in with passkey" }).count());
  await shots(page, "pair-dark", "dark"); await shots(page, "pair-light", "light");
  // typing state
  await page.emulateMedia({ colorScheme: "dark" });
  await page.getByLabel("digit 1").click(); await page.keyboard.type("482");
  log("  after 482 focused digit 4:", await page.getByLabel("digit 4").evaluate((e) => e === document.activeElement));
  await shots(page, "pair-typing", "light");
});
// B. iOS hint
await step("B iOS Safari hint", async () => {
  const page = await (await ctx(ip, { userAgent: IPHONE })).newPage();
  await page.goto("/pair");
  log("  hint count:", await page.locator(".hint").count(), "text:", JSON.stringify((await page.locator(".hint").allInnerTexts()).join("|")));
  await shots(page, "pair-ios-hint", "dark", [390]);
  const p2 = await (await ctx(ip, { userAgent: IPHONE })).newPage();
  await p2.addInitScript(() => Object.defineProperty(navigator, "standalone", { get: () => true }));
  await p2.goto("/pair");
  log("  standalone hint visible:", await p2.locator(".hint").isVisible().catch(() => false), "count:", await p2.locator(".hint").count());
});
// C. Wrong code
await step("C wrong code", async () => {
  const page = await (await ctx(ip)).newPage();
  const code = newCode(); const wrong = code === "000000" ? "111111" : "000000";
  await page.goto("/pair"); await page.getByLabel("digit 1").click(); await page.keyboard.type(wrong);
  await page.waitForURL(/bad=wrong/);
  log("  card:", JSON.stringify(await page.locator("#code > .pc:not([hidden])").innerText()));
  log("  digit1 empty+focused:", await page.getByLabel("digit 1").inputValue(), await page.getByLabel("digit 1").evaluate((e) => e === document.activeElement));
  await shots(page, "pair-wrong");
});
// G. QR link pairs at once + passkey offer + save passkey (device A, localhost)
const A = await ctx(lh); const pa = await A.newPage(); const aa = await authn(pa);
let qrCode;
await step("G QR link -> paired, passkey offer, save", async () => {
  qrCode = newCode();
  await pa.goto(`/pair?code=${qrCode}`);
  await pa.waitForURL(/\/$/);
  log("  landed:", new URL(pa.url()).pathname + new URL(pa.url()).search);
  await pa.locator("#pairing").getByText("Save a passkey?").waitFor();
  log("  offer:", JSON.stringify(await pa.locator("#pairing").innerText()));
  await shots(pa, "pair-offer", "dark"); await shots(pa, "pair-offer-light", "light", [390]);
  await pa.getByRole("button", { name: "save a passkey" }).click();
  await pa.locator("#pairing").getByText("✓ passkey saved").waitFor();
  const { credentials } = await aa.s.send("WebAuthn.getCredentials", { authenticatorId: aa.authenticatorId });
  log("  authenticator credentials:", credentials.length, "server passkeys:", JSON.stringify(await (await pa.request.get("/passkeys")).json()));
  await shots(pa, "pair-passkey-saved", "dark", [390]);
});
// H. Used code
await step("H used QR code -> run out", async () => {
  const page = await (await ctx(lh)).newPage();
  await page.goto(`/pair?code=${qrCode}`); await page.waitForURL(/bad=/);
  log("  url:", page.url()); log("  card:", JSON.stringify(await page.locator("#code > .pc:not([hidden])").innerText()));
  await shots(page, "pair-expired");
});
// I. settings devices: pair a device QR, passkeys list
await step("I settings › devices: QR + passkeys", async () => {
  await pa.goto("/#settings/devices");
  await pa.getByRole("button", { name: "pair a device" }).click();
  const panel = pa.locator("#s-pair-box .pc"); await panel.waitFor();
  log("  panel:", JSON.stringify(await panel.innerText()));
  const alt = await panel.getByRole("img").getAttribute("alt"); log("  qr alt:", alt);
  const r = await pa.request.post("/devices/pair", { data: {}, headers: { origin: "https://mac.tailnet.example" } });
  const j = await r.json(); log("  /devices/pair link:", j.link, "host:", j.host, "svg bytes:", j.svg.length);
  await pa.getByRole("button", { name: "pair a device" }).count();
  await shots(pa, "settings-pair-qr");
  await pa.locator("#s-passkeys-box").scrollIntoViewIfNeeded();
  log("  passkeys box:", JSON.stringify(await pa.locator("#s-passkeys-box").innerText()));
});
// J. Ask to pair -> waiting -> approve on A with passkey -> C pairs
await step("J ask to pair, approve with passkey", async () => {
  await pa.goto("/"); 
  const C = await ctx(lh, { userAgent: IPAD }); const pc = await C.newPage();
  await pc.goto("/pair"); await pc.getByRole("button", { name: "ask to pair" }).click();
  await pc.locator("#ask-wait").waitFor();
  const word = (await pc.locator("#ask-word").innerText()).trim();
  log("  waiting:", JSON.stringify(await pc.locator("#ask-wait").innerText()));
  await shots(pc, "pair-waiting");
  const card = pa.getByRole("group", { name: new RegExp(`asks to pair: ${word}`) });
  await card.waitFor({ timeout: 10000 });
  log("  card on A:", JSON.stringify(await card.innerText()));
  await shots(pa, "approve-card");
  await card.getByRole("button", { name: /^approve with/ }).click();
  await pa.getByText(/paired$/).first().waitFor({ timeout: 10000 }).catch(() => {});
  log("  A after approve:", JSON.stringify(await pa.locator("#pairing").innerText()));
  await shots(pa, "approve-approved", "dark", [390]);
  await pc.waitForURL(/\/(\?paired)?$/, { timeout: 15000 });
  log("  C landed:", new URL(pc.url()).pathname, "list visible:", await pc.locator("main.cfm").isVisible());
  await shots(pc, "pair-approved-newdevice", "dark", [390]);
  await C.close();
});
// K. Decline
await step("K decline -> declined state on new device", async () => {
  await pa.goto("/");
  const D = await ctx(lh, { userAgent: IPAD }); const pd = await D.newPage();
  await pd.goto("/pair"); await pd.getByRole("button", { name: "ask to pair" }).click(); await pd.locator("#ask-wait").waitFor();
  const word = (await pd.locator("#ask-word").innerText()).trim();
  const card = pa.getByRole("group", { name: new RegExp(`asks to pair: ${word}`) }); await card.waitFor({ timeout: 10000 });
  await card.getByRole("button", { name: "decline" }).click();
  await pa.getByText(/^Declined/).waitFor();
  log("  A declined card:", JSON.stringify(await pa.locator("#pairing").innerText()));
  await shots(pa, "approve-declined", "dark", [390]);
  await pd.locator("#ask-no").waitFor({ state: "visible", timeout: 20000 });
  log("  D:", JSON.stringify(await pd.locator("#ask-no").innerText()));
  await shots(pd, "pair-declined");
  await D.close();
});
// L. Passkey sign-back-in after losing the cookie
await step("L passkey sign-in restores pairing", async () => {
  await A.clearCookies();
  await pa.goto("/"); log("  after clearing cookies / ->", new URL(pa.url()).pathname);
  const btn = pa.getByRole("button", { name: "sign in with passkey" });
  log("  passkey row shown:", await btn.count());
  await btn.click(); await pa.waitForURL(/\/$/, { timeout: 10000 });
  log("  after passkey ->", new URL(pa.url()).pathname, "GET /devices:", (await pa.request.get("/devices")).status());
  // no passkey on this device
  const E = await ctx(lh); const pe = await E.newPage(); await authn(pe);
  await pe.goto("/pair"); await pe.getByRole("button", { name: "sign in with passkey" }).click();
  await pe.locator("#pk-err").waitFor({ state: "visible" });
  log("  no-passkey error:", JSON.stringify(await pe.locator("#pk-err").innerText()));
  await shots(pe, "pair-no-passkey", "dark", [390]);
  await E.close();
});
// O. Passkey removable
await step("O passkey remove in settings", async () => {
  pa.on("dialog", (d) => d.accept());
  await pa.goto("/#settings/devices");
  const box = pa.locator("#s-passkeys-box"); await box.getByRole("button", { name: /^remove the passkey/ }).first().waitFor();
  await shots(pa, "settings-devices-passkeys");
  await box.getByRole("button", { name: /^remove the passkey/ }).first().click();
  await box.getByText("none yet").waitFor();
  log("  after remove:", JSON.stringify(await box.innerText()));
});
// M. Offline
await step("M offline", async () => {
  const page = await (await ctx(ip)).newPage(); await page.goto("/pair");
  await page.route("**/pair", (r) => r.request().method() === "POST" ? r.abort() : r.continue());
  await page.getByLabel("digit 1").click(); await page.keyboard.type("123456");
  await page.locator("#offline").waitFor({ state: "visible" });
  log("  offline:", JSON.stringify(await page.locator("#offline").innerText()));
  await shots(page, "pair-offline");
});
// N. Rate limit (5 tries per minute per IP)
await step("N rate limit", async () => {
  const code = newCode(); const wrong = code === "000000" ? "111111" : "000000";
  const page = await (await ctx(ip)).newPage();
  for (let i = 1; i <= 6; i++) {
    await page.goto("/pair"); await page.getByLabel("digit 1").click(); await page.keyboard.type(wrong);
    await page.waitForURL(/bad=/);
    log(`  try ${i}: ${new URL(page.url()).search}`);
    if (/limit/.test(page.url())) break;
  }
  log("  boxes disabled:", await page.getByLabel("digit 1").isDisabled());
  log("  card:", JSON.stringify(await page.locator("#code .pc:not([hidden])").first().innerText()));
  await shots(page, "pair-limit");
  // the right code is refused while limited
  const r = await page.request.post("/pair", { form: { code }, maxRedirects: 0 }); log("  right code while limited ->", r.status(), r.headers().location);
});
await browser.close();
