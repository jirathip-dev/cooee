# Verify recipe: cooee

cooee is a Rust server (`cooee serve`) that serves a web app. The verify instance is the same one the `ui` CI job
drives: `web/e2e/serve.sh` builds the checked-out head and runs `cooee serve` against a fake shed (seeded threads,
two fake providers, the example plugin as `hello` and `other`, the machines plugin), plus a fresh instance on
port + 3. No containers, no real agents, no logins.

Run every block from the worktree root, in one shell, so `OUT` and `P` stay set:

```sh
OUT="$PWD/.verify"            # the lane's own folder (git-ignored)
P="${HERDSMAN_PORT:?}"        # serve.sh also takes P+1 .. P+4
export CARGO_NET_GIT_FETCH_WITH_CLI=true   # the shed dependency is a private git repo: fetch it with git's own auth
mkdir -p "$OUT"
```

## Launch

One-time per worktree: Playwright's packages (the browsers are already on the host for the `ui` gate).

```sh
(cd web/e2e && npm ci) >"$OUT/npm-ci.log" 2>&1; echo "npm ci exit=$?"
```

Start it in its own process group, so teardown stops every child (fake shed, plugins, both `cooee serve`s):

```sh
PORT="$P" PAIR_CODE="$OUT/code" nohup perl -e 'setpgrp(0, 0); exec @ARGV' web/e2e/serve.sh >"$OUT/server.log" 2>&1 &
echo $! >"$OUT/server.pid"
```

Ready signal: `/` answers and the pairing code is written. A cold build takes minutes, a warm one ~15 s:

```sh
for i in $(seq 900); do curl -fs -o /dev/null "http://127.0.0.1:$P/" && test -s "$OUT/code" && break; sleep 1; done
echo "ready after ${i}s"
```

`server.log` holds the build and server output if it never gets ready.

Teardown (the pid is the group id):

```sh
kill -TERM -- "-$(cat "$OUT/server.pid")"
curl -s -o /dev/null "http://127.0.0.1:$P/"; echo "curl exit=$? (7: nothing listens)"
```

## Doctor

Read-only: the pair page serves, an unpaired `/` redirects to it, the code is six digits and the seeded 1,000-message
thread is on disk.

```sh
test "$(curl -s -o /dev/null -w '%{http_code}' "http://127.0.0.1:$P/pair")" = 200 &&
  test "$(curl -s -o /dev/null -w '%{redirect_url}' "http://127.0.0.1:$P/")" = "http://127.0.0.1:$P/pair" &&
  grep -Eq '^[0-9]{6}$' "$OUT/code" &&
  test "$(wc -l <"$(dirname "$(cat "$OUT/code.config")")/data/threads/long-390.jsonl")" -eq 1000
echo "doctor exit=$?"
```

## Drive

Every page but `/pair` needs a paired browser: type the code from `$OUT/code` into the six boxes (from `digit 1`; the sixth
sends it), keep the cookie. A code pairs once: the script saves the cookie to `$OUT/state.json` for reuse.

The script below pairs, then opens each screen at 390 and 1440 px and screenshots it once its ready element shows.
It runs from `web/e2e` so `@playwright/test` resolves. Add the PR's own steps after the screen it touches.

```sh
cat >"$OUT/drive.mjs" <<'EOF'
import { chromium } from "@playwright/test";
import fs from "node:fs";
const out = process.env.OUT, base = `http://127.0.0.1:${process.env.P}`;
const browser = await chromium.launch();
const pair = await browser.newPage({ baseURL: base });
await pair.goto("/");
console.log("unpaired / ->", new URL(pair.url()).pathname);
await pair.getByLabel("digit 1").click();
await pair.keyboard.type(fs.readFileSync(`${out}/code`, "utf8").trim());
await pair.waitForURL(/\/(\?paired)?$/, { waitUntil: "commit" });
await pair.context().storageState({ path: `${out}/state.json` });
console.log("paired ->", new URL(pair.url()).pathname);
for (const w of [390, 1440]) {
  const page = await browser.newPage({ baseURL: base, viewport: { width: w, height: w === 390 ? 844 : 900 }, storageState: `${out}/state.json` });
  const shot = async (name, ready) => {
    await ready.waitFor();
    await page.screenshot({ path: `${out}/${w}-${name}.png`, fullPage: true });
    console.log(`${w} ${name} ok`);
  };
  await page.goto("/?list");
  await shot("agents", page.locator(".ags .agr").first());
  await page.goto("/new");
  await shot("new", page.getByLabel("message", { exact: true }));
  await page.goto(`/t/long-${w}`);
  await shot("chat", page.getByLabel("message", { exact: true }));
  await page.goto("/#settings/sign-ins");
  await shot("settings", page.locator("#sheet .sh:popover-open .six"));
  await page.goto("/p/hello");
  await shot("plugin-hello", page.locator("iframe.pf"));
  await page.goto("/p/machines");
  await shot("plugin-machines", page.frameLocator("iframe.pf").locator("article.on"));
  await page.close();
}
await browser.close();
EOF
(cd web/e2e && OUT="$OUT" P="$P" node --input-type=module - <"$OUT/drive.mjs") >"$OUT/drive.log" 2>&1
echo "drive exit=$?"; cat "$OUT/drive.log"
```

Selectors this UI answers to (from `web/e2e/*.spec.mjs`):

- header: `heading` level 1 `cooee`, link `new chat`, button `settings`, a plugin's tab link by its label
  (`Hello`, `Machines`; `Machines (needs you)` with its dot)
- agents sidebar (the home screen on a phone, `/?list`): `.ags .agr` rows, one per agent; a chat's `‹` is
  `back to agents`
- new chat (`/new`, `/new?<agent>`): nothing to pick; `message` field (exact), button `send` (lands on `/t/<id>`, run
  in its own scratch folder)
- Use a folder…: the composer's button `folder: scratch ▾` opens `#ufs` (`search folders`, a radio per folder); the
  thread bar's folder is `#bf`
- chat (`/t/<id>`): `message` field (exact), button `send`; seeded threads `long-390` and `long-1440`
  (1,000 messages, read up to #990)
- settings sheet: the open popover `#sheet .sh:popover-open`, its index `.six a` (sign-ins, folders, look, alerts, devices, plugins, report)
- sign-ins: `.pv[data-provider="fake-390"]` (or `fake-1440`), button `sign in`, the code field
  `the code from the fake-390 sign-in page`, `good-code` signs in
- plugin tab: `iframe.pf`; drive inside it with a frame locator

## Evidence

- Screenshots: `$OUT/390-<screen>.png` and `$OUT/1440-<screen>.png` (full page, viewports 390×844 and 1440×900),
  one pair per screen the drive opened; list both widths in `verdict.md`.
- Transcript: `$OUT/server.log` (build + server), `$OUT/drive.log` (one `<width> <screen> ok` line per shot).
- Exit codes to record: `npm ci`, the ready loop (`ready after Ns`), `doctor`, `drive`, the teardown `kill` and the
  closing `curl` (7).

## Feature map

| Screen / feature | How to reach it |
|---|---|
| Pairing | `/pair` (any page redirects there unpaired) |
| Agents | `/` (from 1000 px beside a chat), `/?list` on a phone |
| New chat | `/new`, or header `new chat` |
| Chat | `/t/long-390` or `/t/long-1440`; a new one via `/new` → `send` |
| Settings | `/#settings/<section>`: `sign-ins`, `folders`, `look`, `alerts`, `devices`, `plugins`, `report`; or header `settings` |
| Sign-ins (providers) | `/#settings/sign-ins` → `providers` |
| Folders | `/#settings/folders` → `roots` |
| Look (style, sprites, theme, colours) | `/#settings/look` |
| Devices | `/#settings/devices` |
| Plugin manager | `/#settings/plugins` → `#s-plugins-box` button `add a plugin` (its index lists `greet · the example plugin`) |
| Report mode | `/#settings/report` |
| Plugin tabs | `/p/hello`, `/p/other`, `/p/machines` |
| First run (no agents, no roots) | `http://127.0.0.1:$((P + 3))/`, paired with the code in `$OUT/code.bare` |
