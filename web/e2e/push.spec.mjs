// Notifications in the service worker (sw.js), with pushes delivered through Chromium's DevTools protocol in the
// shape the server sends (crates/cooee-server/src/push.rs): one per thread (tag), none while that thread is open, and
// reading the thread clears it.
import { expect, test } from "@playwright/test";

// The full Chromium (new headless): the headless shell has no notifications.
test.use({ channel: "chromium" });

/// The service worker's registration id, for `ServiceWorker.deliverPushMessage`.
async function registration(page, cdp) {
  await page.evaluate(() => navigator.serviceWorker.ready);
  const found = new Promise((ok) =>
    cdp.on("ServiceWorker.workerRegistrationUpdated", ({ registrations }) => {
      const r = registrations.find((r) => !r.isDeleted && r.scopeURL.startsWith(new URL(page.url()).origin));
      if (r) ok(r.registrationId);
    }));
  await cdp.send("ServiceWorker.enable");
  return found;
}

const shown = (page, tag) =>
  page.evaluate(async (tag) => (await (await navigator.serviceWorker.ready).getNotifications({ tag })).map((n) => n.body), tag);

test("two pushes for a thread show one notification; none while it is open; reading it clears it", async ({ page, context }) => {
  await context.grantPermissions(["notifications"]);
  await page.goto("/new");
  await page.getByLabel("first message").fill("Say hello");
  await page.getByRole("button", { name: "send" }).click();
  await page.waitForURL(/\/t\//);
  await expect(page.locator("#st")).toHaveAttribute("data-st", "done");
  const id = new URL(page.url()).pathname.split("/")[2];
  const last = (await (await page.request.get(`/threads/${id}/messages`)).json()).items.at(-1).id;

  await page.goto("/");
  const cdp = await context.newCDPSession(page);
  const registrationId = await registration(page, cdp);
  const origin = new URL(page.url()).origin;
  const push = (body) => cdp.send("ServiceWorker.deliverPushMessage", {
    origin, registrationId, data: JSON.stringify({ title: "Say hello", body, tag: id, url: `/t/${id}`, item: last }),
  });

  await push("needs you: Edit src/lib.rs");
  await push("done: Hello");
  await expect.poll(() => shown(page, id)).toEqual(["done: Hello"]);

  // Opening the thread reads it up to its last item: the notification goes.
  await page.goto(`/t/${id}`);
  await expect(page.locator(".blk.ai")).toContainText("Hello");
  await expect.poll(() => shown(page, id)).toEqual([]);

  // While the thread is open in view, a push for it shows nothing.
  const cdp2 = await context.newCDPSession(page);
  const again = await registration(page, cdp2);
  await cdp2.send("ServiceWorker.deliverPushMessage", {
    origin, registrationId: again, data: JSON.stringify({ title: "Say hello", body: "done: again", tag: id, url: `/t/${id}`, item: last }),
  });
  await page.waitForTimeout(500);
  expect(await shown(page, id)).toEqual([]);
});
