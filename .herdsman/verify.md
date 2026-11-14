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

## iOS

The native app (`apple/`) in the lane's own iPhone simulator, driven exactly as `apple.yml`'s `e2e` job does: the same
server (`apple/e2e/serve.sh`: fake agent, personas Coder and Conductor, the `cards` plugin on P+1, the `drover`
fixture), the same build and the same UI tests (`apple/UITests/Live*UITests.swift`, one per screen, each from a fresh launch), once per look. Every command below is that job's, with
`$OUT` for `$RUNNER_TEMP` and the lane's port for 4100. It takes P and P+1: run it after the web teardown, in the same
shell, with bash or zsh (it uses arrays).

### Doctor

An Xcode (not the Command Line Tools) and an iOS simulator runtime:

```bash
case "$(xcode-select -p)" in *.app/Contents/Developer) ;; *) false ;; esac &&
  xcrun simctl list runtimes | grep -q '^iOS '
echo "ios doctor exit=$?"
```

Not 0: write `verify-no-xcode` in `verdict.md` and take the screens from CI instead, the `simulator-shots` artifact of
the PR head's `apple` run (dark and light, same file names as the Drive below), naming the run id in `verdict.md`:

```bash
run=$(gh run list --workflow apple.yml --commit "$(git rev-parse HEAD)" --json databaseId -q '.[0].databaseId')
gh run download "${run:?no apple run at this head}" -n simulator-shots -D "$OUT/shots/ios"
echo "verify-no-xcode: apple run $run exit=$?"; ls "$OUT/shots/ios"
```

### Launch

The server in its own process group; ready once the pairing link is written (a cold build takes minutes):

```bash
PORT="$P" PAIR_LINK="$OUT/pair" nohup perl -e 'setpgrp(0, 0); exec @ARGV' apple/e2e/serve.sh >"$OUT/serve.log" 2>&1 &
echo $! >"$OUT/serve.pid"
for i in $(seq 900); do test -s "$OUT/pair" && break; sleep 1; done
test -s "$OUT/pair"; echo "ios serve ready after ${i}s exit=$?"
```

The lane's own simulator, its UDID kept in `$OUT/sim` for teardown, then the app and its UI tests built for it. The app
builds before any code is minted: a code lives 10 min.

```bash
sim=$(xcrun simctl create "verify-$(echo "${HERDSMAN_LANE:?}" | cut -d: -f2)" "iPhone 17"); echo "$sim" >"$OUT/sim"
xcrun simctl bootstatus "$sim" -b >"$OUT/boot.log" 2>&1; echo "ios boot exit=$?"
build=(xcodebuild -skipMacroValidation -project Cooee.xcodeproj -scheme Cooee -destination "id=$sim")
(cd apple && "${build[@]}" build-for-testing) >"$OUT/ios-build.log" 2>&1; echo "ios build exit=$?"
```

### Drive

Per look, like the job: set it with `simctl`, empty the keychain, pair with a fresh link, run the UI test; then export
its screenshots into `$OUT/shots/ios/<look>/`:

```bash
for look in dark light; do
  xcrun simctl ui "$sim" appearance "$look"
  xcrun simctl keychain "$sim" reset
  link=$(sh -c "$(cat "$OUT/pair.again")" | sed -n 's/^link  //p')
  rm -rf "${OUT:?}/e2e-$look.xcresult" "${OUT:?}/shots/ios/$look"
  (cd apple && TEST_RUNNER_COOEE_LOOK="$look" TEST_RUNNER_COOEE_PAIR_LINK="$link" "${build[@]}" \
    -resultBundlePath "$OUT/e2e-$look.xcresult" test-without-building) >"$OUT/ios-$look.log" 2>&1
  echo "ios drive $look exit=$?"
  xcrun xcresulttool export attachments --path "$OUT/e2e-$look.xcresult" --output-path "$OUT/shots/ios/$look"
  echo "ios export $look exit=$?"
done
ls "$OUT/shots/ios/dark" "$OUT/shots/ios/light"
```

Each look's `manifest.json` maps an exported file to its screen: `suggestedHumanReadableName` starts with the screen
name below and the look (`home-dark_0_…png`).

### Teardown

The server's process group, then only the simulator this run created (never `simctl delete all`: other lanes'
simulators share the host):

```bash
kill -TERM -- "-$(cat "$OUT/serve.pid")"; echo "ios kill exit=$?"
xcrun simctl shutdown "$(cat "$OUT/sim")"; xcrun simctl delete "$(cat "$OUT/sim")"; echo "ios sim delete exit=$?"
curl -s -o /dev/null "http://127.0.0.1:$P/"; echo "ios curl exit=$? (7: nothing listens)"
xcrun simctl list devices | grep -q "$(cat "$OUT/sim")"; echo "ios sim listed exit=$? (1: gone)"
```

### The Mac

An issue about the Mac app takes its 1440 screens from the `mac-setup` job's `mac-setup-shots` artifact, in the same
`apple` run the Doctor's fallback finds. The recipe does not run that job's `MacSetupUITests` here: it installs cooee
and shed as login items on the host it runs on.

```bash
run=$(gh run list --workflow apple.yml --commit "$(git rev-parse HEAD)" --json databaseId -q '.[0].databaseId')
gh run download "${run:?no apple run at this head}" -n mac-setup-shots -D "$OUT/shots/mac"; echo "mac shots run $run exit=$?"
```

## Evidence

- Screenshots: `$OUT/390-<screen>.png` and `$OUT/1440-<screen>.png` (full page, viewports 390×844 and 1440×900),
  one pair per screen the drive opened; list both widths in `verdict.md`.
- Transcript: `$OUT/server.log` (build + server), `$OUT/drive.log` (one `<width> <screen> ok` line per shot).
- Exit codes to record: `npm ci`, the ready loop (`ready after Ns`), `doctor`, `drive`, the teardown `kill` and the
  closing `curl` (7).
- iOS: `$OUT/shots/ios/{dark,light}/` (the simulator, 390-class iPhone 17), one file per screen of the feature map's
  native rows; list each with its look in `verdict.md`, plus the 1440 Mac ones from `$OUT/shots/mac/` when the issue is
  about the Mac. Without Xcode, `verify-no-xcode` and the CI run id. Logs: `$OUT/serve.log`, `$OUT/ios-build.log`,
  `$OUT/ios-<look>.log`. Record every `ios … exit=` line.

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
| iOS: pairing (paired) | `pair-paired` |
| iOS: home | `home` (back from the first chat) |
| iOS: chat | `chat-streaming` (a `slow 60` reply, a message queued), `chat-cards`, `chat-cards-answered` (the `cards` plugin) |
| iOS: persona editor | `persona` (New chat → New persona…, then Edit a persona) |
| iOS: settings (avatar menu) | `avatar-menu` (Account; Look → Terminal / Modern) |
| iOS: Conductor and drover | `attach` (first look only), `conductor`, `events`, `queue`, `queue-live`, `queue-offline` |
| iOS: the issue's own screen | add a `keep("<name>")` step to the `apple/UITests/Live*UITests.swift` test for that screen in the PR, after the screen it touches |
| Mac: first run, pairing approve | CI `mac-setup-shots`: `mac-setup/` (`welcome`, `folders`, `qr`, `approve`, `approved`), `phone-ask/` (`pair-waiting`, `pair-approved`) |

iOS rows name the screenshot the live UI tests keep on that screen (`$OUT/shots/ios/<look>/`), on the path the test walks.
