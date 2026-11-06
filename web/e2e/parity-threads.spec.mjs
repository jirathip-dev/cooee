// herdsman#553 parity, rows 15–29 (#43, threads and messages; row 24 is #48's): the list's unread dot, badge and live
// refresh, the title and rename, swipe to archive with undo and restore, markdown and folded tool runs, code blocks'
// [copy], attachments and the viewer, the outbox, the growing composer, the keyboard, stopping and the offline line,
// the queue and steering, drafts with files, the long-press menu, and older messages on scroll. Each row's shot is
// ui-<viewport>/parity-<row>-….png.
import AxeBuilder from "@axe-core/playwright";
import { expect, test } from "@playwright/test";

const shot = (page, info, name) => page.screenshot({ path: `../../ui-${info.project.name}/${name}.png` });
const icon = (n) => new URL(`../icons/icon-${n}.png`, import.meta.url).pathname;

async function audit(page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth - window.innerWidth)).toBeLessThanOrEqual(0);
  const { violations } = await new AxeBuilder({ page }).analyze();
  expect(violations.map((v) => `${v.id}: ${v.nodes.map((n) => n.target).join(" ")}`)).toEqual([]);
}

// A new thread whose turn is done, open; its id.
async function thread(page, text = "Say hello") {
  await page.goto("/#new");
  await page.getByLabel("first message").fill(text);
  await page.getByRole("button", { name: "start" }).click();
  await page.waitForURL(/\/t\//);
  await expect(page.locator("#st")).toHaveAttribute("data-st", "done");
  return new URL(page.url()).pathname.split("/").pop();
}

async function say(page, text) {
  await page.locator("#box").fill(text);
  await page.getByRole("button", { name: "send" }).click();
}

const box = (page) => page.getByRole("textbox", { name: "message", exact: true });

test.beforeEach(async ({ page }) => {
  await page.emulateMedia({ colorScheme: "dark", reducedMotion: "reduce" });
});

test("row 15: a new reply shows on the open list (dot and badge, no reload); opening the thread clears it", async ({ page }, info) => {
  await page.clock.install();
  const id = await thread(page);
  // Read: the mark goes after 3 s.
  await page.clock.runFor(3500);
  await page.goto("/");
  const row = page.locator(`.fc a.tr[data-id="${id}"]`);
  await expect(row).toHaveClass(/\brd\b/);
  await expect(row.locator(".bd")).toHaveCount(0);
  const before = await page.evaluate(() => performance.getEntriesByType("navigation").length);
  const sent = await page.request.post(`/threads/${id}/send`, { data: { text: "Say hello", random_id: `r15-${info.project.name}` } });
  expect(sent.ok()).toBe(true);
  await expect.poll(async () => (await (await page.request.get(`/threads/${id}/messages`)).json()).items.length).toBe(4);
  await page.clock.runFor(5000);
  await expect(row).not.toHaveClass(/\brd\b/);
  await expect(row.locator(".ud")).toHaveText("●");
  await expect(row.locator(".bd")).toHaveText("1");
  expect(await page.evaluate(() => performance.getEntriesByType("navigation").length)).toBe(before);
  await shot(page, info, "parity-15-unread");
  await audit(page);
  await row.click();
  await page.waitForURL(/\/t\//);
  await page.clock.runFor(3500);
  await page.goto("/");
  await expect(row).toHaveClass(/\brd\b/);
  await expect(row.locator(".bd")).toHaveCount(0);
});

test("row 16: a thread started by typing is titled by its first line; the title renames it", async ({ page }, info) => {
  const title = `hi ${info.project.name}`;
  const id = await thread(page, title);
  await page.goto("/");
  await expect(page.locator(`.fc a.tr[data-id="${id}"] .tt`)).toHaveText(title);
  await page.goto(`/t/${id}`);
  await page.locator(".th .ttl").click();
  const rt = page.locator("#rt");
  await expect(rt).toBeVisible();
  await rt.getByLabel("thread title").fill(`renamed ${info.project.name}`);
  await shot(page, info, "parity-16-rename");
  await audit(page);
  await rt.getByRole("button", { name: "rename" }).click();
  await expect(rt).toBeHidden();
  await expect(page.locator(".th .ttl")).toHaveText(`renamed ${info.project.name}`);
  await page.goto("/");
  await expect(page.locator(`.fc a.tr[data-id="${id}"] .tt`)).toHaveText(`renamed ${info.project.name}`);
});

test("row 17: a row swiped left is archived, undo brings it back, and the archived list restores it", async ({ page }, info) => {
  const id = await thread(page);
  await page.goto("/");
  const row = page.locator(`.fc a.tr[data-id="${id}"]`);
  const swipe = () =>
    row.evaluate((r) => {
      const y = r.getBoundingClientRect().top + 10;
      const at = (x) => [new Touch({ identifier: 1, target: r, clientX: x, clientY: y })];
      r.dispatchEvent(new TouchEvent("touchstart", { touches: at(300), bubbles: true }));
      r.dispatchEvent(new TouchEvent("touchmove", { touches: at(150), bubbles: true }));
      r.dispatchEvent(new TouchEvent("touchend", { touches: [], bubbles: true }));
    });
  await swipe();
  await expect(row).toHaveCount(0);
  const toast = page.locator("#toast");
  await expect(toast).toContainText("archived");
  await shot(page, info, "parity-17-undo");
  await toast.getByRole("button", { name: "undo" }).click();
  await expect(row).toHaveCount(1);

  await swipe();
  await expect(row).toHaveCount(0);
  const arc = page.locator("details.arc");
  await arc.locator("summary").click();
  const restore = arc.locator(`[data-restore="${id}"]`);
  await expect(restore).toBeVisible();
  await shot(page, info, "parity-17-archived");
  await audit(page);
  // Archived on the server: a reload lists it there too.
  await page.reload();
  await expect(row).toHaveCount(0);
  await page.locator("details.arc summary").click();
  await page.locator(`[data-restore="${id}"]`).click();
  await expect(row).toHaveCount(1);
  await expect(page.locator(`[data-restore="${id}"]`)).toHaveCount(0);
});

test("row 18: a reply's markdown renders (table, code, link, raw HTML as text); a run of commands folds and stays open", async ({ page }, info) => {
  await thread(page, "markdown please");
  const reply = page.locator(".blk.ai").last();
  await expect(reply.locator("h2")).toHaveText("Build report");
  await expect(reply.locator("table th")).toHaveText(["crate", "tests", "time"]);
  await expect(reply.locator("table tbody tr")).toHaveCount(2);
  await expect(reply.locator(".cb code")).toHaveText('fn main() {\n    println!("hello");\n}');
  await expect(reply.locator(".cb .cbh span")).toHaveText("rust");
  const link = reply.getByRole("link", { name: "the run" });
  await expect(link).toHaveAttribute("href", "https://example.com/run/1");
  await expect(link).toHaveAttribute("target", "_blank");
  await expect(link).toHaveAttribute("rel", "noopener");
  await expect(reply).toContainText("<script>alert(1)</script>");
  await expect(reply.locator("script")).toHaveCount(0);
  // The table scrolls inside the message, never widening the page.
  const fits = await reply.locator(".tbl").evaluate((t) => t.getBoundingClientRect().right <= innerWidth);
  expect(fits).toBe(true);
  await reply.scrollIntoViewIfNeeded();
  await shot(page, info, "parity-18-markdown");
  await audit(page);

  await say(page, "run commands");
  const fold = page.locator(".fold button").last();
  await expect(fold).toHaveText("▸ 3 commands");
  const runs = page.locator(".blk.run", { hasText: "cargo" });
  await expect(runs).toHaveCount(3);
  for (const r of await runs.all()) await expect(r).toBeHidden();
  await fold.click();
  await expect(fold).toHaveText("▾ 3 commands");
  for (const r of await runs.all()) await expect(r).toBeVisible();
  await shot(page, info, "parity-18-fold");
  // New rows (an update) leave it open.
  await say(page, "Say hello");
  await expect(page.locator(".blk.ai").last()).toContainText("Hello");
  for (const r of await runs.all()) await expect(r).toBeVisible();
  await expect(fold).toHaveText("▾ 3 commands");
});

test("row 19: a code block's [copy] puts its code on the clipboard", async ({ page, context }, info) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await thread(page, "markdown again");
  const copy = page.locator(".cb .cp").last();
  await copy.click();
  await expect(copy).toHaveText("copied");
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe('fn main() {\n    println!("hello");\n}');
  await copy.scrollIntoViewIfNeeded();
  await shot(page, info, "parity-19-copy");
});

test("row 20: two photos sent in one message show as thumbnails; one opens full size and back closes it", async ({ page }, info) => {
  const id = await thread(page);
  await page.setInputFiles("#files", [icon(192), icon(512)]);
  await expect(page.locator("#picked img")).toHaveCount(2);
  await shot(page, info, "parity-20-picked");
  await say(page, "two photos");
  const mine = page.locator(".blk.me", { hasText: "two photos" });
  await expect(mine.locator(".ct")).toHaveText(/✓$/);
  await expect(mine.locator("a.fi.im img")).toHaveCount(2);
  await expect(page.locator("#picked")).toBeHidden();
  // Stored on the server, served as images.
  const items = (await (await page.request.get(`/threads/${id}/messages`)).json()).items;
  const files = items.find((i) => i.text === "two photos").files;
  expect(files).toHaveLength(2);
  const got = await page.request.get(`/threads/${id}/files/${files[1]}`);
  expect(got.headers()["content-type"]).toBe("image/png");
  expect(got.headers()["content-security-policy"]).toBe("sandbox");
  expect((await page.request.get(`/threads/${id}/files/..%2F${id}.json`)).status()).toBe(404);
  await mine.locator("a.fi.im").first().click();
  const viewer = page.locator("#viewer");
  await expect(viewer).toBeVisible();
  await expect.poll(() => viewer.locator("img").evaluate((i) => i.naturalWidth)).toBe(192);
  await shot(page, info, "parity-20-viewer");
  await page.goBack();
  await expect(viewer).toBeHidden();
  expect(new URL(page.url()).pathname).toBe(`/t/${id}`);
});

test("row 21: a send says sending…, then ✓; offline it turns red with [retry], which resends it", async ({ page, context }, info) => {
  await thread(page);
  await context.setOffline(true);
  await say(page, "while offline");
  const row = page.locator(".blk.me", { hasText: "while offline" });
  await expect(row).toHaveClass(/failed/);
  await expect(row.locator(".ct")).toHaveText("not sent");
  await shot(page, info, "parity-21-failed");
  await context.setOffline(false);
  await row.getByRole("button", { name: /retry/ }).click();
  await expect(row).not.toHaveClass(/failed/);
  await expect(row.locator(".ct")).toHaveText(/✓$/);
});

test("row 22: the box grows with its text to 6 lines, then scrolls; send stays at its foot", async ({ page }, info) => {
  await thread(page);
  const b = box(page);
  const one = await b.evaluate((e) => e.clientHeight);
  await b.fill(["1", "2", "3", "4", "5", "6", "7", "8"].join("\n"));
  const lh = await b.evaluate((e) => Number.parseFloat(getComputedStyle(e).lineHeight));
  const [h, sh] = await b.evaluate((e) => [e.clientHeight, e.scrollHeight]);
  expect(h).toBeGreaterThan(one + 4 * lh);
  expect(h).toBeLessThan(one + 6 * lh);
  expect(sh).toBeGreaterThan(h);
  const send = await page.getByRole("button", { name: "send" }).boundingBox();
  const area = await b.boundingBox();
  expect(Math.abs(send.y + send.height - (area.y + area.height))).toBeLessThan(2);
  await shot(page, info, "parity-22-grown");
  await b.fill("");
  await b.dispatchEvent("input");
  expect(await b.evaluate((e) => e.clientHeight)).toBe(one);
});

test("row 23: the chat takes the visual viewport's height, so the composer sits on the keyboard", async ({ page }, info) => {
  await thread(page);
  const size = page.viewportSize();
  await page.setViewportSize({ width: size.width, height: size.height - 300 });
  await expect.poll(() => page.evaluate(() => document.documentElement.style.getPropertyValue("--vvh"))).toBe(`${size.height - 300}px`);
  const composer = await page.locator("#composer").boundingBox();
  expect(Math.round(composer.y + composer.height)).toBe(size.height - 300);
  expect(await page.evaluate(() => scrollY)).toBe(0);
  await shot(page, info, "parity-23-keyboard");
});

test("row 25: [stop] says stopping… until the turn stops; the bar says offline, then nothing once back", async ({ page, context }, info) => {
  await thread(page);
  await say(page, "hang on");
  const pill = page.locator("#pill");
  await expect(pill).toBeVisible();
  let release;
  const held = new Promise((ok) => { release = ok; });
  await page.route("**/cancel", async (route) => { await held; await route.continue(); });
  await pill.locator(".stop").click();
  await expect(pill.locator(".stop")).toHaveText("stopping…");
  await expect(pill.locator(".stop")).toBeDisabled();
  await shot(page, info, "parity-25-stopping");
  release();
  await expect(pill).toBeHidden();
  await expect(pill.locator(".stop")).toHaveText("stop");

  await context.setOffline(true);
  await expect(page.locator("#net")).toHaveText("offline · ");
  await shot(page, info, "parity-25-offline");
  await context.setOffline(false);
  await expect(page.locator(".th .dt")).toHaveAttribute("data-sync", "on");
  await expect(page.locator("#net")).toHaveText("");
});

test("row 26: a send during a turn is queued; [send now] steers it in; the next goes when the turn ends; [edit] takes one back", async ({ page }, info) => {
  await thread(page);
  await say(page, "hang on");
  await expect(page.locator("#pill")).toBeVisible();
  await say(page, "steer this");
  const queue = page.locator("#queue");
  await expect(queue.locator("li")).toHaveText([/queued · steer this/]);
  await shot(page, info, "parity-26-queued");
  await audit(page);
  await queue.getByRole("button", { name: "send now: steer this" }).click();
  await expect(queue).toBeHidden();
  await expect(page.locator(".blk.me", { hasText: "steer this" })).toHaveCount(1);

  await say(page, "fix the typo");
  await queue.getByRole("button", { name: "edit: fix the typo" }).click();
  await expect(queue).toBeHidden();
  await expect(box(page)).toHaveValue("fix the typo");
  await box(page).fill("");

  await say(page, "then say hello");
  await expect(queue.locator("li")).toHaveCount(1);
  await page.locator("#pill .stop").click();
  // The turn ends: the queued message goes, and its turn runs.
  await expect(queue).toBeHidden();
  await expect(page.locator(".blk.me").last()).toContainText("then say hello");
  await expect(page.locator(".blk.ai").last()).toContainText("Hello");
});

test("row 27: the draft keeps its text and picked files across a reload", async ({ page }, info) => {
  const id = await thread(page);
  await box(page).fill("half a thought");
  await page.setInputFiles("#files", [icon(192)]);
  await expect(page.locator("#picked img")).toHaveCount(1);
  // The page hiding saves the draft at once.
  await page.evaluate(() => {
    Object.defineProperty(document, "visibilityState", { value: "hidden", configurable: true });
    document.dispatchEvent(new Event("visibilitychange"));
  });
  await expect
    .poll(() =>
      page.evaluate(
        (id) =>
          new Promise((ok) => {
            const r = indexedDB.open("cooee", 1);
            r.onsuccess = () => {
              const g = r.result.transaction("drafts").objectStore("drafts").get(id);
              g.onsuccess = () => ok(g.result ? `${g.result.text}|${g.result.files?.length}` : "");
            };
          }),
        id,
      ),
    )
    .toBe("half a thought|1");
  await page.reload();
  await expect(box(page)).toHaveValue("half a thought");
  await expect(page.locator("#picked img")).toHaveCount(1);
  await shot(page, info, "parity-27-draft");
});

test("row 28: a long press on a reply opens copy, copy as markdown and quote; quote puts > lines in the box", async ({ page, context }, info) => {
  await context.grantPermissions(["clipboard-read", "clipboard-write"]);
  await page.clock.install();
  await thread(page);
  const reply = page.locator(".blk.ai").last();
  // A press held 500 ms.
  await reply.evaluate((r) => {
    const t = [new Touch({ identifier: 1, target: r, clientX: 60, clientY: r.getBoundingClientRect().top + 10 })];
    r.dispatchEvent(new TouchEvent("touchstart", { touches: t, bubbles: true }));
  });
  await page.clock.runFor(600);
  const menu = page.locator("#menu");
  await expect(menu).toBeVisible();
  await shot(page, info, "parity-28-menu");
  await audit(page);
  await menu.getByRole("menuitem", { name: "copy as markdown" }).click();
  await expect(menu).toBeHidden();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe("Hello");
  await reply.click({ button: "right" });
  await menu.getByRole("menuitem", { name: "quote" }).click();
  await expect(box(page)).toHaveValue("> Hello\n\n");
});

test("row 29: the thread bar stays put while scrolling to the top loads older messages", async ({ page }, info) => {
  await page.goto(`/t/long-${info.project.name}`);
  await expect(page.locator("#msgs .unread")).toBeVisible();
  const list = page.locator("#msgs");
  const minId = () => list.evaluate((l) => Math.min(...[...l.children].filter((r) => r.dataset.id).map((r) => +r.dataset.id)));
  const bar = await page.locator(".th").boundingBox();
  const first = await minId();
  await list.evaluate((l) => { l.scrollTop = 0; });
  await expect.poll(minId).toBeLessThan(first);
  expect(await page.locator(".th").boundingBox()).toEqual(bar);
  await shot(page, info, "parity-29-older");
});
