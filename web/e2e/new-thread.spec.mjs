// The new thread (#48, docs/design/new-thread): the composer is a chip row, a box and [send]; the folder chip starts
// at scratch (no folder: the thread's own) and the agent chip at the pick last used overall, then at the provider and
// account last used in the folder picked (serve.sh: root/project last used work@, root/other lab@);
// send opens the chat at once with the theme's loading line, which the reply replaces, and a failed connect shows
// inline with [retry]; Settings › providers is one flat list. Every control is on the design's one size scale, in
// each theme, checked as the design's shoot.mjs checks its mockup. Shots: ui-<viewport>/new-*.png.
import fs from "node:fs";
import path from "node:path";
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const shed = () => fs.readFileSync(new URL("./.auth/code.shed", import.meta.url), "utf8").trim();
const gate = (name, shut) => {
  const p = path.join(shed(), name);
  if (shut) fs.writeFileSync(p, "");
  else fs.rmSync(p, { force: true });
};
const shot = (page, info, name) => page.screenshot({ path: `../../ui-${info.project.name}/new-${name}.png` });
const LOOKS = { farm: "theme:farm/mode:dark", kitchen: "mode:light/theme:kitchen", garden: "mode:dark/theme:garden", terminal: "mode:light/style:terminal/theme:plain" };
const look = (context, baseURL, value) =>
  context.addCookies([{ name: "cooee-look", value, url: baseURL }]);

// The design's audit (docs/design/new-thread/shoot.mjs), on the page: no sideways scroll; at 390 the composer fits one
// screen; every chip, button and one-line input is --ctl tall with the --r-ctl radius and --fs text (inputs 16 px);
// every sheet and Settings row is --row; chips have no border; sheet rows line up with the title; the header's [+] at every width (no sidebar); and no dev text (an agent id,
// `System default`).
function audit(composer) {
  const bad = [], W = innerWidth, cs = getComputedStyle(document.documentElement);
  const px = (k) => parseFloat(cs.getPropertyValue(k)), CTL = px("--ctl"), RC = px("--r-ctl"), FS = px("--fs"), ROW = px("--row");
  const name = (e) => `${e.tagName.toLowerCase()}.${[...e.classList].join(".")} "${(e.textContent || e.placeholder || "").trim().slice(0, 24)}"`;
  const shown = (e) => e.getClientRects().length && getComputedStyle(e).visibility !== "hidden";
  const near = (a, b) => Math.abs(a - b) < 0.6;
  if (document.documentElement.scrollWidth > W) bad.push(`sideways scroll ${document.documentElement.scrollWidth} > ${W}`);
  if (composer && W < 1000 && document.documentElement.scrollHeight > innerHeight) bad.push(`composer scrolls: ${document.documentElement.scrollHeight} > ${innerHeight}`);
  const dev = document.body.innerText.match(/\b[\w-]+-acp\b|system default/i);
  if (dev) bad.push(`dev text "${dev[0]}"`);
  if (document.querySelector(".side")) bad.push("a sidebar");
  const plus = document.querySelector(".bar .np");
  if (plus && !shown(plus)) bad.push(`header [+] hidden at ${W}`);
  for (const e of document.querySelectorAll("body *")) {
    if (!shown(e)) continue;
    const r = e.getBoundingClientRect(), s = getComputedStyle(e);
    if (e.matches(".chip, :is(.nc,.acr,.lg) button:not(.sr,.gl,.stop), :is(.pop,.lg) :is(input:not([type=radio]), select), .nc textarea")) {
      if (!near(r.height, CTL)) bad.push(`control ${r.height}px, not --ctl ${CTL} · ${name(e)}`);
      if (!near(parseFloat(s.borderTopLeftRadius), RC)) bad.push(`radius ${s.borderTopLeftRadius}, not --r-ctl ${RC} · ${name(e)}`);
      const want = e.matches("input,textarea,select") ? 16 : FS;
      if (!near(parseFloat(s.fontSize), want)) bad.push(`text ${s.fontSize}, not ${want} · ${name(e)}`);
    }
    // A chip's edge is its one inset ring (none in Terminal, a bracket key), never a border on top of it.
    if (e.matches(".chip") && s.borderTopWidth !== "0px") bad.push(`chip border ${s.borderTopWidth} · ${name(e)}`);
    // A sheet's rows and headings line up with its title, flat (no fieldset inset, no depth indent).
    const t = e.matches(".pop :is(.pk, .ch)") && e.closest(".pop").querySelector("b[id^=sht-]");
    if (t && !near(r.left, t.getBoundingClientRect().left)) bad.push(`row at x=${r.left}, title at x=${t.getBoundingClientRect().left} · ${name(e)}`);
    if (e.matches(".pk, .acr") && !near(r.height, ROW)) bad.push(`row ${r.height}px, not --row ${ROW} · ${name(e)}`);
    if (e.matches(".pk .sk, .acr .sk") && !near(parseFloat(s.fontSize), FS)) bad.push(`row text ${s.fontSize}, not --fs ${FS} · ${name(e)}`);
  }
  return bad;
}
async function axe(page) {
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
}

test.beforeEach(async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
});

for (const [theme, value] of Object.entries(LOOKS))
  test(`the composer is its chip row, box and send only, on the size scale (${theme})`, async ({ page, context, baseURL }, info) => {
    await look(context, baseURL, value);
    await page.goto("/new");
    const nc = page.locator("#nc");
    await expect(nc.locator("#chip-agent")).toBeVisible();
    // Only the chips, the box and send: no radios, account selects or folder list on the card.
    await expect(nc.locator(":is(button, textarea, input, select, a):visible")).toHaveCount(4);
    await expect(nc.getByRole("button", { name: /^agent: / })).toBeVisible();
    await expect(nc.getByRole("button", { name: "folder: scratch" })).toHaveText(/^scratch/);
    expect(await page.evaluate(audit, true)).toEqual([]);
    await shot(page, info, `composer-${theme}`);
    // The agent sheet: the agent profiles (#40), then each provider's accounts, a signed-out one muted with [sign in].
    await page.locator("#chip-agent").click();
    const agent = page.locator("#sh-agent");
    await expect(agent).toBeVisible();
    await expect(page.locator("#chip-agent")).toHaveAttribute("aria-expanded", "true");
    await expect(agent.locator(".ch")).toHaveText(["profiles", "fake-390"]);
    await expect(agent.locator(".pk .sk")).toHaveText(["Reviewer", "main login", "lab@example.com", "work@example.com"]);
    expect(await page.evaluate(audit, false)).toEqual([]);
    await shot(page, info, `agent-${theme}`);
    await page.keyboard.press("Escape");
    await page.locator("#chip-folder").click();
    const folder = page.locator("#sh-folder");
    await expect(folder).toBeVisible();
    await expect(folder.locator(".pk").first()).toContainText("scratch");
    await expect(folder.locator(".pk").first().locator("input")).toBeChecked();
    await expect(folder.locator(".ch").first()).toHaveText("recent");
    expect(await page.evaluate(audit, false)).toEqual([]);
    await shot(page, info, `folder-${theme}`);
    await axe(page);
  });

test("the agent chip starts at the account last used in the folder picked", async ({ page }, info) => {
  await page.goto("/new");
  const chip = page.locator("#chip-agent"), folder = page.locator("#chip-folder");
  const pick = async (p) => {
    await folder.click();
    await page.locator("#sh-folder label", { has: page.locator(".sk", { hasText: new RegExp(`^${p}$`) }) }).click();
    await expect(page.locator("#sh-folder")).toBeHidden();
  };
  await pick("~/root/other");
  await expect(folder).toHaveText(/^other/);
  await expect(chip).toHaveText(/^fake-390 · lab@example\.com/);
  await expect(page.getByLabel("first message")).toHaveAttribute("placeholder", "message fake-390");
  await pick("~/root/project");
  await expect(chip).toHaveText(/^fake-390 · work@example\.com/);
  // The folder sheet shows each recent folder's last pick beside it.
  await folder.click();
  const recent = page.locator("#sh-folder label", { has: page.locator(".sk", { hasText: /^~\/root\/other$/ }) });
  await expect(recent.locator(".sv")).toHaveText("fake-390 · lab@example.com");
  await shot(page, info, "folder-last-used");
  await page.keyboard.press("Escape");
  // Another pick in the sheet sets the chip; a folder with no history keeps the newest pick overall.
  await chip.click();
  await page.locator("#sh-agent label", { hasText: "lab@example.com" }).click();
  await expect(chip).toHaveText(/^fake-390 · lab@example\.com/);
  await pick("~/root");
  await expect(chip).toHaveText(/^fake-390 · work@example\.com/);
  // Back to scratch (no folder): the newest pick overall again.
  await pick("scratch");
  await expect(folder).toHaveText(/^scratch/);
  await expect(chip).toHaveText(/^fake-390 · work@example\.com/);
});

test("with no folder picked the thread runs in its own scratch folder, listed as scratch", async ({ page }, info) => {
  await page.goto("/new");
  await expect(page.locator("#chip-folder")).toHaveText(/^scratch/);
  const text = `Scratch start ${info.project.name}`;
  await page.getByLabel("first message").fill(text);
  await page.getByRole("button", { name: "send" }).click();
  await page.waitForURL(/\/t\//);
  await expect(page.locator(".tb .ds")).toContainText(" · scratch · ");
  await expect(page.locator("#msgs .blk.ai").first()).toContainText("Hello");
  await page.goto("/");
  const card = page.locator(".fc", { has: page.locator(".fh b", { hasText: /^scratch$/ }) });
  await expect(card.locator(".tr", { hasText: text })).toBeAttached();
});

test("send opens the chat at once with the loading line, which the reply replaces", async ({ page }, info) => {
  const where = `slow-${info.project.name}`;
  gate(`hold-in-${where}`, true);
  try {
    await page.goto("/new");
    await page.locator("#chip-folder").click();
    await page.locator("#sh-folder label", { hasText: where }).click();
    const who = (await page.locator("#chip-agent").textContent()).replace("▾", "").trim();
    await page.getByLabel("first message").fill("Say hello");
    const t0 = Date.now();
    await page.getByRole("button", { name: "send" }).click();
    await page.waitForURL(/\/t\//);
    const msgs = page.locator("#msgs");
    await expect(msgs.locator(".blk.me")).toHaveText("Say hello");
    const line = msgs.locator(".cnx");
    await expect(line).toHaveText(`saddling up · connecting ${who}`);
    console.log(`send → chat with your message: ${Date.now() - t0} ms (${info.project.name})`);
    await expect(page.locator("#st")).toHaveAttribute("data-st", "loading");
    await expect(page.locator("#st")).toHaveText("saddling up");
    await expect(page.locator(".tb .ds")).toContainText(`${who} · `);
    expect(await page.evaluate(audit, false)).toEqual([]);
    // The chat's composer is on the same scale as /new's, so nothing jumps on send; the bar's word is amber, as the line's.
    const got = await page.evaluate(() => {
      const cs = getComputedStyle(document.documentElement), v = (k) => cs.getPropertyValue(k).trim();
      const box = [...document.querySelectorAll("#composer :is(textarea, button[type=submit])")].map((e) => {
        const s = getComputedStyle(e);
        return [e.getBoundingClientRect().height, s.borderTopLeftRadius];
      });
      const color = (q) => getComputedStyle(document.querySelector(q)).color;
      return { box, want: [parseFloat(v("--ctl")), v("--r-ctl")], st: color("#st"), word: color(".cnx .thw") };
    });
    expect(got.box).toEqual([got.want, got.want]);
    expect(got.st).toBe(got.word);
    await shot(page, info, "chat-connecting");
    await axe(page);
    gate(`hold-in-${where}`, false);
    await expect(msgs.locator(".blk.ai").first()).toContainText("Hello");
    await expect(line).toHaveCount(0);
    await expect(msgs.locator(".blk.me")).toHaveCount(1);
  } finally {
    gate(`hold-in-${where}`, false);
  }
});

test("the agent sheet picks the start's model and effort (#553 row 34), said in the thread's bar", async ({ page }, info) => {
  await page.goto("/new");
  await page.locator("#chip-agent").click();
  const sheet = page.locator("#sh-agent");
  await sheet.getByLabel("model", { exact: true }).selectOption("sonnet");
  await sheet.getByLabel("effort", { exact: true }).selectOption("high");
  await page.keyboard.press("Escape");
  await page.getByLabel("first message").fill(`Row 34 start ${info.project.name}`);
  await page.getByRole("button", { name: "send" }).click();
  await page.waitForURL(/\/t\//);
  await expect(page.locator(".tb .ds")).toContainText(" · sonnet · ");
  await expect(page.getByLabel("effort", { exact: true })).toHaveValue("high");
});

test("a connect that fails shows inline with retry; the message stays; retry sends it", async ({ page }, info) => {
  const where = `bad-${info.project.name}`;
  gate(`fails-in-${where}`, true);
  try {
    await page.goto("/new");
    await page.locator("#chip-folder").click();
    await page.locator("#sh-folder label", { hasText: where }).click();
    const who = (await page.locator("#chip-agent").textContent()).replace("▾", "").trim();
    await page.getByLabel("first message").fill("Say hello");
    await page.getByRole("button", { name: "send" }).click();
    await page.waitForURL(/\/t\//);
    const msgs = page.locator("#msgs"), card = msgs.locator(".cfl");
    await expect(card.locator(".cnx")).toHaveText(`refused · couldn't connect ${who}`);
    await expect(card.locator(".ds")).toContainText("no ACP session");
    await expect(card).not.toContainText("-acp");
    await expect(msgs.locator(".blk.me")).toHaveText("Say hello");
    await expect(msgs.locator(".blk.me.failed")).toHaveCount(0);
    await expect(page.getByLabel("message", { exact: true })).toBeEnabled();
    await shot(page, info, "chat-failed");
    await axe(page);
    gate(`fails-in-${where}`, false);
    for (const f of fs.readdirSync(shed())) if (f.startsWith("ready-timeout-")) gate(f, false);
    await card.getByRole("button", { name: "retry" }).click();
    await expect(msgs.locator(".blk.ai").first()).toContainText("Hello");
    await expect(card).toHaveCount(0);
    await expect(msgs.locator(".blk.me")).toHaveCount(1);
  } finally {
    gate(`fails-in-${where}`, false);
  }
});

test("Settings › providers is one flat list: a row per account, [+ account] per provider", async ({ page }, info) => {
  await page.goto("/");
  await page.getByRole("button", { name: "settings" }).click();
  await page.locator("#sheet").getByRole("button", { name: "providers" }).click();
  const sheet = page.locator("#sh-providers");
  await expect(sheet.locator(".hd b")).toHaveText("providers");
  const rows = sheet.locator('.acr[data-provider="fake-390"]:not(.add)');
  await expect(rows.locator(".sk")).toHaveText(["main login", "lab@example.com", "work@example.com"]);
  for (const r of await rows.all()) {
    await expect(r.locator(".ptg")).toHaveText("fake-390");
    await expect(r.getByRole("button", { name: /^(sign in|sign out)$/ })).toHaveCount(1);
  }
  await expect(sheet.locator(".acr.add .ptg")).toHaveText(["fake-390", "fake-1440"]);
  await expect(sheet).not.toContainText("default");
  expect(await page.evaluate(audit, false)).toEqual([]);
  await shot(page, info, "providers");
  await axe(page);
  // ‹ goes back to Settings.
  await sheet.getByRole("button", { name: "back to settings" }).click();
  await expect(page.locator("#sh-settings")).toBeVisible();
  await expect(page.locator("#sh-settings")).not.toContainText("default for new threads");
});
