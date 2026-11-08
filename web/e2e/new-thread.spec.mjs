// A new chat's start (#48, on #86's new chat): send opens the chat at once with the theme's loading line, which the
// reply replaces; a failed connect shows inline with [retry]; the model and effort picked are the start's; nothing on
// screen names an agent id or says `System default`; Settings › providers is one flat list (serve.sh: a start in
// root/slow-<viewport> waits while the spec holds it, one in root/bad-<viewport> fails). Shots: ui-<viewport>/new-*.png.
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

// No sideways scroll, and no dev text (an agent id, `System default`).
function audit() {
  const bad = [];
  if (document.documentElement.scrollWidth > innerWidth) bad.push(`sideways scroll ${document.documentElement.scrollWidth}`);
  const dev = document.body.innerText.match(/\b[\w-]+-acp\b|system default/i);
  if (dev) bad.push(`dev text "${dev[0]}"`);
  return bad;
}
async function axe(page) {
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
}

test.beforeEach(async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
});

// A new chat with the first agent, in `where` (Use a folder…), its message typed.
async function start(page, where, text = "Say hello") {
  await page.goto("/new");
  await page.getByRole("button", { name: "folder: scratch ▾" }).click();
  await page.locator("#ufs label", { hasText: where }).click();
  await expect(page.getByRole("button", { name: `folder: ${where} ▾` })).toBeVisible();
  await page.getByLabel("message", { exact: true }).fill(text);
}

test("send opens the chat at once with the loading line, which the reply replaces", async ({ page }, info) => {
  const where = `slow-${info.project.name}`;
  gate(`hold-in-${where}`, true);
  try {
    await start(page, where);
    const t0 = Date.now();
    await page.getByRole("button", { name: "send" }).click();
    await page.waitForURL(/\/t\//);
    const who = await page.locator(".chat").getAttribute("data-who");
    const msgs = page.locator("#msgs");
    await expect(msgs.locator(".blk.me")).toHaveText("Say hello");
    const line = msgs.locator(".cnx");
    await expect(line).toHaveText(`saddling up · connecting ${who}`);
    console.log(`send → chat with your message: ${Date.now() - t0} ms (${info.project.name})`);
    await expect(page.locator("#st")).toHaveAttribute("data-st", "loading");
    await expect(page.locator(".tb .ds")).toContainText(`${who} · `);
    expect(await page.evaluate(audit)).toEqual([]);
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

test("the model and effort picked are the start's (#553 row 34), said in the thread's bar", async ({ page }, info) => {
  await page.goto("/new");
  await page.getByLabel("model", { exact: true }).selectOption("sonnet");
  await page.getByLabel("effort", { exact: true }).selectOption("high");
  await page.getByLabel("message", { exact: true }).fill(`Row 34 start ${info.project.name}`);
  await page.getByRole("button", { name: "send" }).click();
  await page.waitForURL(/\/t\//);
  await expect(page.locator(".tb .ds")).toContainText(" · sonnet · ");
  await expect(page.getByLabel("effort", { exact: true })).toHaveValue("high");
});

test("a connect that fails shows inline with retry; the message stays; retry sends it", async ({ page }, info) => {
  const where = `bad-${info.project.name}`;
  gate(`fails-in-${where}`, true);
  try {
    await start(page, where);
    await page.getByRole("button", { name: "send" }).click();
    await page.waitForURL(/\/t\//);
    const who = await page.locator(".chat").getAttribute("data-who");
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
  expect(await page.evaluate(audit)).toEqual([]);
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
  await shot(page, info, "providers");
  await axe(page);
  // ‹ goes back to Settings.
  await sheet.getByRole("button", { name: "back to settings" }).click();
  await expect(page.locator("#sh-settings")).toBeVisible();
});
