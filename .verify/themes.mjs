import { chromium } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";
const out = process.env.OUT, base = `http://127.0.0.1:${process.env.P}`;
const cfg = path.dirname(fs.readFileSync(`${out}/code.config`, "utf8").trim());
const browser = await chromium.launch();
const log = (...a) => console.log(...a);
for (const w of [390, 1440]) {
  const page = await browser.newPage({ baseURL: base, viewport: { width: w, height: w === 390 ? 844 : 900 }, storageState: `${out}/state.json` });
  await page.emulateMedia({ colorScheme: "dark" });
  const shot = async (name) => { await page.screenshot({ path: `${out}/${w}-${name}.png`, fullPage: true }); log(`${w} ${name} ok`); };
  const sheet = page.locator("#sheet .sh:popover-open");
  await page.goto("/#settings/look");
  await sheet.locator("#s-themes").waitFor();
  log(w, "themes rows:", JSON.stringify(await sheet.locator(".thm .thn").allInnerTexts()));
  log(w, "buttons on farm:", JSON.stringify(await sheet.locator(".thm", { hasText: "farm" }).locator("button, summary").allInnerTexts()));
  await sheet.locator("#s-themes").scrollIntoViewIfNeeded();
  await shot("look-themes");
  // + theme
  await sheet.locator("summary", { hasText: "+ theme" }).click();
  await shot("plus-theme");
  log(w, "+theme text:", JSON.stringify(await sheet.locator("details.thx[open]").innerText()));
  if (w === 390) {
    // remove kitchen
    await sheet.locator(".thm", { hasText: "kitchen" }).getByRole("button", { name: "remove" }).click();
    await page.waitForURL(/#settings\/look$/);
    await sheet.locator("#s-themes").waitFor();
    log(w, "after remove:", JSON.stringify(await sheet.locator(".thm .thn").allInnerTexts()));
    // add a folder by its path through the URL field
    await sheet.locator("summary", { hasText: "+ theme" }).click();
    await sheet.getByLabel("a theme's git URL").fill(`${out}/kitchen2`);
    const [resp] = await Promise.all([page.waitForResponse((r) => r.url().endsWith("/themes/add")), sheet.getByRole("button", { name: "add · not reviewed" }).click()]);
    log(w, "add status:", resp.status(), "->", page.url());
    await page.waitForLoadState();
    if (resp.status() >= 400) { log(w, "add body:", (await page.locator("body").innerText()).slice(0, 300)); await shot("add-error"); await page.goto("/#settings/look"); }
    await sheet.locator("#s-themes").waitFor();
    log(w, "after add:", JSON.stringify(await sheet.locator(".thm .thn").allInnerTexts()));
  }
  // remix farm
  const name = `night-${w}`;
  const farm = sheet.locator(".thm", { hasText: "farm" });
  await farm.locator("summary", { hasText: "remix" }).click();
  await farm.getByLabel("the new theme's name").fill(name);
  await farm.getByLabel("what it should look like").fill("farm at night");
  await shot("remix-form");
  await farm.getByRole("button", { name: "start with an agent" }).click();
  await page.waitForURL(/\/t\//);
  await page.getByLabel("message", { exact: true }).waitFor();
  await page.waitForTimeout(1500);
  log(w, "chat url:", page.url());
  log(w, "dev folder:", JSON.stringify(fs.readdirSync(`${cfg}/themes-dev/${name}`)));
  log(w, "farm copied same:", fs.readFileSync(`${cfg}/themes-dev/${name}/sprites.svg`, "utf8") === fs.readFileSync(`${cfg}/themes/farm/sprites.svg`, "utf8"));
  log(w, "chat first lines:", JSON.stringify((await page.locator("#msgs").innerText()).slice(0, 300)));
  await shot("remix-chat");
  const chatUrl = page.url();
  await page.locator(".tdv a", { hasText: `preview ${name}` }).click();
  await page.locator(".prg").waitFor();
  log(w, "gate:", await page.locator(".prg").innerText());
  await page.frameLocator('iframe[title="thread list"]').locator(`html[data-theme="dev.${name}"]`).waitFor({ state: "attached" });
  await page.waitForTimeout(800);
  log(w, "scene img:", await page.locator('img[alt="scene"]').count());
  await shot("preview");
  // break it: drop a sprite state, as an agent's save would
  const sprites = `${cfg}/themes-dev/${name}/sprites.svg`;
  const good = fs.readFileSync(sprites, "utf8");
  let reload = page.waitForEvent("load", { timeout: 15000 });
  fs.writeFileSync(sprites, good.replace('id="s-needs"', 'id="s-nope"'));
  await reload; await page.locator(".prg").waitFor();
  log(w, "after bad save:", await page.locator(".prg").innerText(), JSON.stringify(await page.locator(".prx li").allInnerTexts()));
  await shot("preview-fails");
  await page.getByRole("button", { name: "keep" }).click();
  await page.waitForLoadState();
  log(w, "keep (failing) ->", page.url());
  await page.waitForTimeout(1500);
  log(w, "chat tail:", JSON.stringify((await page.locator("#msgs").innerText()).slice(-400)));
  log(w, "installed?", fs.existsSync(`${cfg}/themes/${name}`));
  await shot("keep-blocked");
  // fix: restore and change a word
  const tj = `${cfg}/themes-dev/${name}/theme.json`;
  const t = JSON.parse(fs.readFileSync(tj, "utf8"));
  t.label = `${name} night`; t.words.loading = "saddling up by moonlight";
  fs.writeFileSync(sprites, good); fs.writeFileSync(tj, JSON.stringify(t));
  await page.goto(`/themes/dev/${name}?thread=${chatUrl.split("/t/")[1]}`);
  await page.locator(".prg").waitFor();
  log(w, "after fix:", await page.locator(".prg").innerText());
  log(w, "frame loading word:", await page.frameLocator('iframe[title="thread list"]').locator("#ld .lw").innerText().catch((e) => "ERR " + e.message));
  await page.getByRole("button", { name: "keep" }).click();
  await page.waitForURL(/#settings\/look$/);
  await sheet.locator("#s-themes").waitFor();
  log(w, "after keep:", JSON.stringify(await sheet.locator(".thm .thn").allInnerTexts()), "installed:", fs.existsSync(`${cfg}/themes/${name}`), "dev gone:", !fs.existsSync(`${cfg}/themes-dev/${name}`));
  await sheet.locator(".thm", { hasText: `${name} night` }).scrollIntoViewIfNeeded();
  await shot("kept");
  // use it
  await page.getByRole("button", { name: "look: theme and colours" }).click();
  await page.locator(`#sh-look [data-k=theme][data-v="${name}"]`).click();
  await page.waitForTimeout(500);
  log(w, "html theme:", await page.locator("html").getAttribute("data-theme"));
  await page.goto("/");
  await page.locator(".fc .tr").first().waitFor();
  log(w, "html theme on /:", await page.locator("html").getAttribute("data-theme"));
  await shot("threads-in-new-theme");
  if (w === 390) {
    // new theme from the starter
    await page.goto("/#settings/look");
    await sheet.locator("summary", { hasText: "new theme" }).click();
    const nf = sheet.locator("details.thx[open] form");
    await nf.getByLabel("the new theme's name").fill("fresh");
    await nf.getByRole("button", { name: "start with an agent" }).click();
    await page.waitForURL(/\/t\//);
    log(w, "new folder:", JSON.stringify(fs.readdirSync(`${cfg}/themes-dev/fresh`)));
    await page.locator(".tdv a", { hasText: "preview fresh" }).click();
    await page.locator(".prg").waitFor();
    log(w, "starter gate:", await page.locator(".prg").innerText());
    await page.waitForTimeout(800);
    await shot("new-theme-preview");
    // remix to a name already being built
    await page.goto("/#settings/look");
    const f2 = sheet.locator(".thm", { hasText: "farm" });
    await f2.locator("summary", { hasText: "remix" }).click();
    await f2.getByLabel("the new theme's name").fill("fresh");
    const [r2] = await Promise.all([page.waitForResponse((r) => r.url().endsWith("/themes/remix")), f2.getByRole("button", { name: "start with an agent" }).click()]);
    await page.waitForLoadState();
    log(w, "dup remix:", r2.status(), JSON.stringify((await page.locator("body").innerText()).slice(0, 200)));
    await shot("dup-remix");
    // publish a kept theme
    await page.goto("/#settings/look");
    const k = sheet.locator(".thm", { hasText: `${name} night` });
    const [r3] = await Promise.all([page.waitForResponse((r) => r.url().endsWith("/themes/publish")), k.getByRole("button", { name: "publish" }).click()]);
    await page.waitForLoadState();
    log(w, "publish:", r3.status(), JSON.stringify((await page.locator("body").innerText()).slice(0, 300)));
    await shot("publish");
  }
  await page.close();
}
await browser.close();
