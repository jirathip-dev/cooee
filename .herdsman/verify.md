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
fixture), the same build and the same UI tests (`apple/UITests/Live*UITests.swift`, one per screen, each from a fresh launch), once per look. It runs that job's `apple/e2e/run.sh`, with
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

### Launch, drive, teardown

`apple/e2e/run.sh`, as the job runs it, with the lane's port and `$OUT`: the server (ready once the pairing link is
written; a cold build takes minutes), the lane's own simulator, the app and its UI tests built for it (the app builds
before any code is minted: a code lives 10 min), then every step of `apple/e2e/steps/` in name order; per look it sets
the look with `simctl`, empties the keychain, pairs with a fresh link and runs the UI tests. Always, it exports each
look's screenshots into `$OUT/shots/ios/<look>/`, stops the server and deletes only the simulator it created (never
`simctl delete all`: other lanes' simulators share the host):

```bash
PORT="$P" E2E_DIR="$OUT" SHOTS="$OUT/shots/ios" apple/e2e/run.sh >"$OUT/e2e.log" 2>&1; echo "ios e2e exit=$?"
ls "$OUT/shots/ios/dark" "$OUT/shots/ios/light"
curl -s -o /dev/null "http://127.0.0.1:$P/"; echo "ios curl exit=$? (7: nothing listens)"
```

Each look's `manifest.json` maps an exported file to its screen: `suggestedHumanReadableName` starts with the screen
name below and the look (`home-dark_0_…png`).

### The Mac

An issue about the Mac app (a client only since #484) takes its 1440 screens from the `apple` job's `rendered-frames`
artifact (the `mac-*` design frames), in the same `apple` run the Doctor's fallback finds.

```bash
run=$(gh run list --workflow apple.yml --commit "$(git rev-parse HEAD)" --json databaseId -q '.[0].databaseId')
gh run download "${run:?no apple run at this head}" -n rendered-frames -D "$OUT/shots/mac"; echo "mac shots run $run exit=$?"
```

## Known noise

- iOS: one `GET /p/{name}/screens/{*rest}` 503 per run, after the test taps `Pause`: the fixture drover
  (`apple/e2e/screens.py`) answers 503 for 20 s on purpose, the offline case `queue-offline-*` shows.

## Evidence

- iOS: `$OUT/shots/ios/{dark,light}/` (the simulator, 390-class iPhone 17), one file per screen of the feature map's
  native rows; list each with its look in `verdict.md`, plus the 1440 Mac ones from `$OUT/shots/mac/` when the issue is
  about the Mac. Without Xcode, `verify-no-xcode` and the CI run id. Logs: `$OUT/serve.log`, `$OUT/e2e.log`. Record every `ios … exit=` line.

## Feature map

| Screen / feature | How to reach it |
|---|---|
| iOS: pairing (paired) | `pair-paired` |
| iOS: home | `home` (back from the first chat) |
| iOS: chat | `chat-streaming` (a `slow 60` reply, a message queued), `chat-cards`, `chat-cards-answered` (the `cards` plugin) |
| iOS: persona editor | `persona` (New chat → New persona…, then Edit a persona) |
| iOS: settings (avatar menu) | `avatar-menu` (Account; Look → Terminal / Modern) |
| iOS: Conductor and drover | `attach` (first look only), `conductor`, `events`, `queue`, `queue-live`, `queue-offline` |
| iOS: Workflow and Others | `conductor-workflow`, `workflow` (one issue unfolded), `workflow-round`, `home-no-lanes`, `others` (`LiveWorkflowUITests`) |
| iOS: the issue's own screen | add a `keep("<name>")` step to the `apple/UITests/Live*UITests.swift` test for that screen in the PR, after the screen it touches |
| Mac: first run | CI `rendered-frames`: `mac-connect` |
| iOS: a phone asks, `cooee pair approve` answers | `simulator-shots/ask/`: `pair-waiting`, `pair-approved` (`testAsksAndTheCLIApprovesIt`) |

iOS rows name the screenshot the live UI tests keep on that screen (`$OUT/shots/ios/<look>/`), on the path the test walks.
