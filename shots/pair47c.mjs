import { chromium } from "@playwright/test";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
const out = process.env.OUT, P = process.env.P, lh = `http://localhost:${P}`;
const log = (...a) => console.log(...a);
await new Promise((r) => setTimeout(r, 3000));
const txt = fs.readFileSync(`${out}/cooee-pair.txt`, "utf8");
const code = txt.match(/^code {2}(\d{3}) (\d{3})/m).slice(1).join("");
const browser = await chromium.launch();
const c = await browser.newContext({ baseURL: lh, viewport: { width: 390, height: 844 }, storageState: { cookies: [], origins: [] } });
const p = await c.newPage();
const s = await c.newCDPSession(p); await s.send("WebAuthn.enable");
await s.send("WebAuthn.addVirtualAuthenticator", { options: { protocol: "ctap2", transport: "internal", hasResidentKey: true, hasUserVerification: true, isUserVerified: true, automaticPresenceSimulation: true } });
await p.goto(`/pair?code=${code}`); await p.waitForURL(/\/$/);
await p.getByRole("group", { name: "save a passkey" }).getByRole("button", { name: "save a passkey" }).click(); await p.getByText("✓ passkey saved").waitFor();
await new Promise((r) => setTimeout(r, 1500));
log("cooee pair output after the code was used:\n" + fs.readFileSync(`${out}/cooee-pair.txt`, "utf8").split("\n").filter((l) => !/[▀▄█]/.test(l)).join("\n"));
await p.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" });
for (const w of [390, 1440]) {
  await p.setViewportSize({ width: w, height: w === 390 ? 844 : 900 });
  await p.goto("/#settings/devices");
  const box = p.locator("#s-passkeys-box"); await box.getByRole("button", { name: /^remove the passkey/ }).first().waitFor();
  await box.scrollIntoViewIfNeeded();
  log(`${w} passkeys:`, JSON.stringify(await box.innerText()));
  await p.screenshot({ path: `${out}/shots/${w}-settings-passkeys.png` });
}
// the welcome now offers the passkey row
const u = await (await browser.newContext({ baseURL: lh, storageState: { cookies: [], origins: [] } })).newPage();
await u.goto("/pair"); log("welcome passkey row:", await u.getByRole("button", { name: "sign in with passkey" }).count());
await u.emulateMedia({ colorScheme: "light", reducedMotion: "reduce" });
await u.setViewportSize({ width: 1440, height: 900 }); await u.screenshot({ path: `${out}/shots/1440-pair-with-passkey-row.png`, fullPage: true });
await browser.close();
