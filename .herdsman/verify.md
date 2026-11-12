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

## Native (iOS simulator)

The native app (`apple/`) in the lane's own iOS simulator against `apple/e2e/serve.sh`, the server the `apple.yml` `e2e`
job uses (the fake agent, personas Coder and Conductor, the `cards` plugin on P+1, the `drover` fixture). It takes P and
P+1, so run it after the web teardown, in the same shell (`OUT`, `P`). Needs Xcode and the iOS simulator runtime (the
Mini has both); a cold build takes minutes.

The app pairs only by typing an address, so the e2e job's own UI test (`LiveUITests`) pairs it and walks the screens,
once per look. `simctl io` shoots what a plain launch shows: the pair screen, before the run and again on the relaunch
after it (this unsigned build keeps no pairing across launches). The test's own screenshot of every e2e screen is
exported next to them; `attach` only in the first look, since the drover stays attached.

Server, in its own process group; ready once the pairing link is written:

```sh
mkdir -p "$OUT/shots"
PORT="$P" PAIR_LINK="$OUT/pair" nohup perl -e 'setpgrp(0, 0); exec @ARGV' apple/e2e/serve.sh >"$OUT/native-server.log" 2>&1 &
echo $! >"$OUT/native-server.pid"
for i in $(seq 900); do test -s "$OUT/pair" && break; sleep 1; done
test -s "$OUT/pair"; echo "native serve ready after ${i}s exit=$?"
```

The lane's own simulator (the CI job's device), its UDID in `$OUT/sim`; a rerun reuses it:

```sh
test -s "$OUT/sim" || xcrun simctl create "cooee-verify-$(printf %s "${HERDSMAN_LANE:-$P}" | tr -c 'A-Za-z0-9' -)" "iPhone 17" >"$OUT/sim"
SIM=$(cat "$OUT/sim")
xcrun simctl bootstatus "$SIM" -b >"$OUT/sim-boot.log" 2>&1; echo "native sim $SIM exit=$?"
```

Build the app and its UI tests for that simulator, unsigned, then install the app:

```sh
XB=(xcodebuild -skipMacroValidation -project Cooee.xcodeproj -scheme Cooee -destination "id=$SIM"
  -derivedDataPath "$OUT/dd" CODE_SIGNING_ALLOWED=NO)
(cd apple && "${XB[@]}" build-for-testing) >"$OUT/native-build.log" 2>&1; echo "native build exit=$?"
APP="$OUT/dd/Build/Products/Debug-iphonesimulator/Cooee.app"
BUNDLE=$(/usr/libexec/PlistBuddy -c 'Print CFBundleIdentifier' "$APP/Info.plist")
xcrun simctl install "$SIM" "$APP"; echo "native install $BUNDLE exit=$?"
```

Pair, launch and shoot, dark then light like the CI job. Each look empties the keychain and pairs with a fresh link
(a code pairs once and lives 10 min):

```sh
for look in dark light; do
  xcrun simctl ui "$SIM" appearance "$look"
  xcrun simctl keychain "$SIM" reset
  xcrun simctl launch --terminate-running-process "$SIM" "$BUNDLE" >/dev/null && sleep 3 &&
    xcrun simctl io "$SIM" screenshot "$OUT/shots/sim-pair-$look.png" >/dev/null 2>&1
  echo "native shot pair $look exit=$?"
  xcrun simctl terminate "$SIM" "$BUNDLE"
  link=$(sh -c "$(cat "$OUT/pair.again")" | sed -n 's/^link  //p')
  rm -rf "${OUT:?}/e2e-$look.xcresult" "${OUT:?}/shots/$look"
  (cd apple && TEST_RUNNER_COOEE_LOOK="$look" TEST_RUNNER_COOEE_PAIR_LINK="$link" "${XB[@]}" \
    -only-testing:CooeeAppUITests/LiveUITests/testPairsStreamsAndShowsHome \
    -resultBundlePath "$OUT/e2e-$look.xcresult" test-without-building) >"$OUT/native-drive-$look.log" 2>&1
  echo "native drive $look exit=$?"
  xcrun simctl launch --terminate-running-process "$SIM" "$BUNDLE" >/dev/null && sleep 3 &&
    xcrun simctl io "$SIM" screenshot "$OUT/shots/sim-relaunch-$look.png" >/dev/null 2>&1
  echo "native shot relaunch $look exit=$?"
  xcrun xcresulttool export attachments --path "$OUT/e2e-$look.xcresult" --output-path "$OUT/shots/$look" >/dev/null &&
    python3 -c 'import json, os, sys
d = sys.argv[1]
for t in json.load(open(f"{d}/manifest.json")):
    for a in t["attachments"]:
        name = a["suggestedHumanReadableName"].rsplit("_", 3)[0]
        os.rename(os.path.join(d, a["exportedFileName"]), os.path.join(d, "e2e-" + name + ".png"))' "$OUT/shots/$look"
  echo "native export $look exit=$?"
done
ls "$OUT/shots" "$OUT/shots/dark" "$OUT/shots/light"
```

Native doctor, read-only: the app is installed in this simulator and its process runs (the last launch above):

```sh
xcrun simctl get_app_container "$SIM" "$BUNDLE" >/dev/null &&
  xcrun simctl spawn "$SIM" launchctl list | grep -q "UIKitApplication:$BUNDLE"
echo "native doctor exit=$?"
```

Teardown: the server's process group, then only this lane's simulator, and proof nothing is left:

```sh
kill -TERM -- "-$(cat "$OUT/native-server.pid")"; echo "native kill exit=$?"
xcrun simctl shutdown "$SIM"; xcrun simctl delete "$SIM"; echo "native sim delete exit=$?"
curl -s -o /dev/null "http://127.0.0.1:$P/"; echo "native curl exit=$? (7: nothing listens)"
xcrun simctl list devices | grep -q "$SIM"; echo "native sim listed exit=$? (1: gone)"
```

Never `simctl delete all` or `erase all`: other lanes' simulators share this host.

## Evidence

- Screenshots: `$OUT/390-<screen>.png` and `$OUT/1440-<screen>.png` (full page, viewports 390×844 and 1440×900),
  one pair per screen the drive opened; list both widths in `verdict.md`.
- Transcript: `$OUT/server.log` (build + server), `$OUT/drive.log` (one `<width> <screen> ok` line per shot).
- Exit codes to record: `npm ci`, the ready loop (`ready after Ns`), `doctor`, `drive`, the teardown `kill` and the
  closing `curl` (7).
- Native: `$OUT/shots/sim-{pair,relaunch}-{dark,light}.png` (`simctl io`) and `$OUT/shots/{dark,light}/e2e-<screen>.png`
  (the UI test's own, one per e2e screen); list them in `verdict.md`. Logs: `$OUT/native-server.log`,
  `$OUT/native-build.log`, `$OUT/native-drive-<look>.log`. Record every `native … exit=` line.

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
