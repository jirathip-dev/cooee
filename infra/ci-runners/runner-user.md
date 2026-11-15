# A runner-only macOS user for `mac-setup` (#368)

apple.yml's `mac-setup` job sets the Mac app up like a new user's first run: it registers the
`com.jirathip.cooee.serve` / `shed` login items, writes `~/.config/cooee/cooee` and
`~/.local/share/cooee/cooee`, binds :4747 and drives the iPhone simulator. On the owner's
account that would stop and overwrite the live cooee. `runner-user.sh` gives it a macOS user of
its own, `cooeeci`, so all of that lands in `~cooeeci` instead.

## Set it up (once, on the mini, as the owner)

```sh
infra/ci-runners/runner-user.sh --dry-run   # prints every step, changes nothing
sudo infra/ci-runners/runner-user.sh
```

It needs `gh` signed in as the owner (with admin rights on jirathip-dev/cooee) and Xcode at
`/Applications/Xcode.app` with its licence and first launch already accepted. Running it again
changes nothing: each step checks first and prints `ok: …` when it is already done.

What it creates:

- `cooeeci`, a standard (not admin) user. Its password is generated and kept only in the System
  keychain as `cooee-ci-runner`. The user is added to `_developer` so Xcode UI tests can run
  without a prompt.
- Fast user switching turned on, so `cooeeci` can stay logged in behind the owner's session.
- rustup and cargo in `~cooeeci/.cargo`, with no toolchain installed: the job's
  `dtolnay/rust-toolchain` step installs the one it pins. Xcode is shared and `xcode-select` is
  left alone.
- actions-runner (the latest release, checked against its sha256) in
  `~cooeeci/actions-runner`, registered on jirathip-dev/cooee as `mini-cooee-setup` with
  the labels `self-hosted`, `macOS`, `ARM64` and `cooee-mac-setup` (not `cooee-mac`). `gh` fetches
  the registration token and pipes it to the runner, so it is never printed.
- The LaunchAgent `com.jirathip.cooee.ci-runner` in `~cooeeci/Library/LaunchAgents`, with
  no `SessionCreate`. It runs the runner in `cooeeci`'s own GUI session and restarts it if it
  exits. Its log is `~cooeeci/Library/Logs/cooee-ci-runner.log`.

It never reads or writes the owner's `~/.config/cooee`, `~/.local/share/cooee` or login items,
and it leaves the owner's automatic login alone, so the owner stays the console user.

## Log `cooeeci` in after each boot

The login items and the simulator need a real GUI session, and macOS has no command that
starts a second one in the background. After the mini boots (or after the script's first
run), log `cooeeci` in once:

1. Copy its password: `sudo security find-generic-password -s cooee-ci-runner -w /Library/Keychains/System.keychain`
2. Use the fast user switching menu in Control Center to choose `cooeeci` and paste the password.
3. Once its desktop appears, switch back to the owner from the same menu.

The `cooeeci` session keeps running in the background until the next reboot or log-out, and
the runner starts with it.

## Check it

```sh
gh api repos/jirathip-dev/cooee/actions/runners -q '.runners[] | select(.name == "mini-cooee-setup") | .status + " " + ([.labels[].name] | join(","))'
```

It prints `online self-hosted,macOS,ARM64,cooee-mac-setup` while `cooeeci` is logged in. If it
says `offline`, check that `cooeeci` is logged in (`sudo launchctl print gui/$(id -u cooeeci)`
succeeds) and read the log above.

## Memory

On the 32 GB mini, an idle background session (WindowServer share, Finder, Dock and the runner)
takes roughly 0.5–1 GB. While `mac-setup` runs, its Xcode builds, the Mac app and the iPhone
simulator take about 6–10 GB more, then give it back. Measure it with:

```sh
ps -u cooeeci -o rss= | awk '{ s += $1 } END { printf "%.0f MB\n", s / 1024 }'
```

## Remove it all

```sh
sudo infra/ci-runners/runner-user.sh --remove
```

This ends the `cooeeci` session, unregisters `mini-cooee-setup` from GitHub (with a removal
token `gh` fetches), deletes the `cooeeci` user and its home folder (the runner, rustup, the
LaunchAgent, its cooee files) and deletes the `cooee-ci-runner` password from the System
keychain. Add `--dry-run` to see the steps first. Fast user switching stays on, in case it was
already on before the script ran; turn it off in System Settings → Control Center if you don't
want it.
