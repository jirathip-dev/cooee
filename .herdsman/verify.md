# Verify recipe: cooee

cooee is a Rust server (`cooee serve`); its client is the native app (`apple/`). No containers, no real agents,
no logins.

Run every block from the worktree root, in one shell, so `OUT` and `P` stay set:

```sh
OUT="$PWD/.verify"            # the lane's own folder (git-ignored)
P="${HERDSMAN_PORT:?}"        # serve.sh also takes P+1
export CARGO_NET_GIT_FETCH_WITH_CLI=true   # the shed dependency is a private git repo: fetch it with git's own auth
mkdir -p "$OUT"
```

## iOS

The native app (`apple/`) in the lane's own iPhone simulator, driven exactly as `apple.yml`'s `e2e` job does: the same
server (`apple/e2e/serve.sh`: fake agent, personas Coder and Conductor, the `cards` plugin on P+1, the `drover`
fixture), the same build and the same UI test (`LiveUITests`), once per look. Every command below is that job's, with
`$OUT` for `$RUNNER_TEMP` and the lane's port for 4100. It takes P and P+1: run it in the same shell, with bash or
zsh (it uses arrays).

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

- iOS: `$OUT/shots/ios/{dark,light}/` (the simulator, 390-class iPhone 17), one file per screen of the feature map's
  native rows; list each with its look in `verdict.md`, plus the 1440 Mac ones from `$OUT/shots/mac/` when the issue is
  about the Mac. Without Xcode, `verify-no-xcode` and the CI run id. Logs: `$OUT/serve.log`, `$OUT/ios-build.log`,
  `$OUT/ios-<look>.log`. Record every `ios … exit=` line.

## Feature map

| Screen / feature | How to reach it |
|---|---|
| iOS: pairing (paired) | `pair-paired` |
| iOS: home | `home` (back from the first chat) |
| iOS: chat | `chat-streaming` (a `slow 60` reply, a message queued), `chat-cards`, `chat-cards-answered` (the `cards` plugin) |
| iOS: persona editor | `persona` (New chat → New persona…, then Edit a persona) |
| iOS: settings (avatar menu) | `avatar-menu` (Account; Look → Terminal / Modern) |
| iOS: Conductor and drover | `attach` (first look only), `conductor`, `events`, `queue`, `queue-live`, `queue-offline` |
| iOS: the issue's own screen | add a `keep("<name>")` step to `apple/UITests/LiveUITests.swift` in the PR, after the screen it touches |
| Mac: first run, pairing approve | CI `mac-setup-shots`: `mac-setup/` (`welcome`, `folders`, `qr`, `approve`, `approved`), `phone-ask/` (`pair-waiting`, `pair-approved`) |

iOS rows name the screenshot `LiveUITests` keeps on that screen (`$OUT/shots/ios/<look>/`), on the path the test walks.
