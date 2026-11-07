// herdsman#553 parity, rows 30–35 (#44): a plugin's (the conductor's) cards in a thread. Choice rows with the
// recommended one first, an "other" box and a ✓ line with who and when; a conductor's answer with [change]; a config
// card's diff, waiting bar and [apply]; a secret card's masked field; the model and effort chips; handoff chips and a
// pick-a-thread row. The example plugin `hello` answers each tap 200. Each row's shot is
// ui-<viewport>/parity-<row>-….png.
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const shot = (page, info, name) => page.screenshot({ path: `../../ui-${info.project.name}/${name}.png` });
// The plugin's own calls, with its token (serve.sh's `hello`).
const plugin = async (page, path, data) => {
  const r = await page.request.post(path, { headers: { authorization: "Bearer hello-token-0123456789" }, data });
  expect(r.status(), await r.text()).toBe(200);
  return r.json();
};
const card = (page, data) => plugin(page, "/plugin/card", data);
const at = /\d{1,2}:\d\d/;

async function audit(page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)).toBeLessThanOrEqual(0);
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
}

// A new thread whose turn is done, open; its id.
async function thread(page, text) {
  await page.goto("/new");
  await page.getByLabel("first message").fill(text);
  await page.getByRole("button", { name: "send" }).click();
  await page.waitForURL(/\/t\//);
  await expect(page.locator("#st")).toHaveAttribute("data-st", "done");
  return decodeURIComponent(new URL(page.url()).pathname.split("/")[2]);
}

test.beforeEach(async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" });
});

test("row 30: choice rows, the recommended first, an other box, and the ✓ line says who and when", async ({ page }, info) => {
  const id = await thread(page, `Row 30 ${info.project.name}`);
  await card(page, { thread: id, text: "Which issue first?", options: ["#43", "#44"], recommend: "#44" });
  const row = page.locator(".blk.ap", { hasText: "Which issue first?" });
  await expect(row.locator(".chc button").first()).toHaveText("#44");
  await expect(row.locator(".chc button.rec")).toHaveText("#44");
  await expect(row.getByLabel("other")).toBeVisible();
  await audit(page);
  await shot(page, info, "parity-30-choice");
  await row.locator(".chc button.rec").click();
  await expect(row.locator(".ans")).toHaveText(new RegExp(`^✓ #44 · .+ · ${at.source}`));
  await expect(row.locator(".chc")).toHaveCount(0);
  await shot(page, info, "parity-30-answered");

  await card(page, { thread: id, text: "Merge when?", options: ["now", "tonight"] });
  const other = page.locator(".blk.ap", { hasText: "Merge when?" });
  await other.getByLabel("other").fill("after #43 lands");
  await other.getByRole("button", { name: "send" }).click();
  await expect(other.locator(".ans")).toHaveText(/^✓ after #43 lands · /);
});

test("row 31: a conductor's answer keeps [change]; the owner's pick replaces it", async ({ page }, info) => {
  const id = await thread(page, `Row 31 ${info.project.name}`);
  await card(page, {
    thread: id,
    text: "answered q12 for #44: split the PR, because the issue says 3 PRs",
    options: ["split", "keep"],
    answered: "split",
    by: "conductor-t6",
  });
  const row = page.locator(".blk.ap", { hasText: "answered q12" });
  await expect(row.locator(".ans")).toHaveText(new RegExp(`^✓ split · conductor-t6 · ${at.source}`));
  await audit(page);
  await shot(page, info, "parity-31-change");
  await row.getByRole("button", { name: "change" }).click();
  await row.getByRole("button", { name: "keep" }).click();
  await expect(row.locator(".ans")).toHaveText(/^✓ keep · /);
  await expect(row.locator(".ans")).not.toContainText("conductor-t6");
  await expect(row.getByRole("button", { name: "change" })).toHaveCount(0);
  await shot(page, info, "parity-31-changed");
});

test("row 32: a config card shows its diff, the bar says it waits, [apply] settles it", async ({ page }, info) => {
  const id = await thread(page, `Row 32 ${info.project.name}`);
  const diff = " [fleet.cooee]\n-max_impl = 2\n+max_impl = 3";
  await card(page, { thread: id, text: "config c1", form: "config", detail: diff });
  const row = page.locator(".blk.ap.config");
  await expect(row.locator(".diff .r")).toHaveText("-max_impl = 2");
  await expect(row.locator(".diff .g")).toHaveText("+max_impl = 3");
  await expect(page.locator("#cw")).toHaveText("config c1 waiting");
  await audit(page);
  await shot(page, info, "parity-32-config");
  // Opened again, the bar is there from the first slice.
  await page.reload();
  await expect(page.locator("#cw")).toHaveText("config c1 waiting");
  await page.locator("#cw").click();
  await expect(row).toBeInViewport();
  await row.getByRole("button", { name: "apply" }).click();
  await expect(row.locator(".ans")).toHaveText(/^config c1 applied · /);
  await expect(page.locator("#cw")).toBeHidden();
  await shot(page, info, "parity-32-applied");
});

test("row 33: a secret card's field is masked; saved, the line names only the secret", async ({ page }, info) => {
  const id = await thread(page, `Row 33 ${info.project.name}`);
  await card(page, { thread: id, text: "the bot token, for alerts", form: "secret", detail: "BOT_TOKEN" });
  const row = page.locator(".blk.ap.secret");
  const field = row.getByLabel("value of BOT_TOKEN");
  await expect(field).toHaveAttribute("type", "password");
  await field.fill("not-a-real-token-123");
  await audit(page);
  await shot(page, info, "parity-33-secret");
  await row.getByRole("button", { name: "save" }).click();
  await expect(row.locator(".ans")).toHaveText(/^secret BOT_TOKEN set ✓ · /);
  await expect(page.locator("main")).not.toContainText("not-a-real-token-123");
  await shot(page, info, "parity-33-set");
});

test("row 34: the composer's model and effort chips; a pick is kept and said in the thread", async ({ page }, info) => {
  const id = await thread(page, `Row 34 ${info.project.name}`);
  await expect(page.getByLabel("model", { exact: true })).toHaveValue("");
  await page.getByLabel("effort", { exact: true }).selectOption("low");
  await expect(page.locator(".qs", { hasText: "effort low" })).toHaveText("model default · effort low from the next start");
  await audit(page);
  await shot(page, info, "parity-34-chip");
  await page.goto(`/t/${id}`);
  await expect(page.getByLabel("effort", { exact: true })).toHaveValue("low");
});

test("row 35: handoff chips open the other thread; a line flows back; a pick-a-thread row", async ({ page }, info) => {
  const target = await thread(page, `Row 35 build ${info.project.name}`);
  const source = await thread(page, `Row 35 plan ${info.project.name}`);
  await plugin(page, "/plugin/handoff", { from: source, to: target, text: "take the build" });
  await plugin(page, "/plugin/line", { thread: source, from: target, text: "filed #61: build cache" });
  await card(page, { thread: source, text: "Continue in which thread?", form: "thread" });
  const pick = page.locator(".blk.ap.thread");
  await expect(pick.getByRole("button", { name: `Row 35 build ${info.project.name}` })).toBeVisible();
  const flow = page.locator(".ln", { hasText: "filed #61" });
  await expect(flow.locator(".chip")).toHaveText(`from Row 35 build ${info.project.name}`);
  await audit(page);
  await shot(page, info, "parity-35-handoff");
  await page.locator(".ln .chip", { hasText: "continued in" }).click();
  await page.waitForURL(`/t/${target}`);
  const back = page.locator(".ln", { hasText: "take the build" });
  await expect(back.locator(".chip")).toHaveText(`from Row 35 plan ${info.project.name}`);
  await shot(page, info, "parity-35-target");
});

test("#84: the header shows the id; a whole-word ref opens that thread and back returns; code, partial and unknown stay plain", async ({ page }, info) => {
  const target = await thread(page, `Refs target ${info.project.name}`);
  const source = await thread(page, `Refs source ${info.project.name}`);
  await expect(page.locator(".th .tn small")).toHaveText(source);
  await plugin(page, "/plugin/line", { thread: source, from: target, text: `see ${target}, not \`${target}\` x${target} t0ffff` });
  await card(page, { thread: source, text: `Ship ${target}?`, options: [`take ${target}`, "no"] });
  const line = page.locator(".ln", { hasText: "see" });
  await expect(line.locator("a.ref")).toHaveCount(1);
  await expect(line.locator("a.ref")).toHaveText(target);
  await expect(line).toContainText(`\`${target}\` x${target} t0ffff`);
  await expect(page.locator(".blk.ap .md a.ref")).toHaveText(target);
  await expect(page.locator(".blk.ap .chc a.ref")).toHaveText(target);
  await expect(page.locator(".blk.ap .chc").getByRole("button", { name: `take ${target}` })).toBeVisible();
  await audit(page);
  await shot(page, info, "refs-header");
  await line.locator("a.ref").click();
  await page.waitForURL(`/t/${target}`);
  await expect(page.locator(".th .tn small")).toHaveText(target);
  await shot(page, info, "refs-tapped");
  await page.locator(".th .bk").click();
  await page.waitForURL(`/t/${source}`);
});
