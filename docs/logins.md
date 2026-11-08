# Where logins live

cooee never reads or stores a token (#30, [architecture.md](architecture.md#sign-ins-m3-and-accounts)). An account
is a config folder; `[sign in]` runs the provider's own login with the folder in its variable (`CLAUDE_CONFIG_DIR`
for Claude Code), and the CLI keeps the login where it always does. The sign-ins card says "signed in" only when the
provider's status command, run for that folder at start, every 30 s and after each sign-in or sign-out, exits 0;
the time of the last sign-in kept in `.cooee.json` never counts as one.

## Claude Code, per OS

| OS | System default login (`~/.claude`) | An added account (`CLAUDE_CONFIG_DIR=<folder>`) |
|---|---|---|
| macOS | login Keychain, generic password `Claude Code-credentials` | login Keychain, `Claude Code-credentials-<hash>`: `<hash>` is the first 8 hex digits of the SHA-256 of the folder's path, exactly as the variable gives it |
| Linux | `~/.claude/.credentials.json` (mode 0600) | `<folder>/.credentials.json` (mode 0600) |

The plain item is for no `CLAUDE_CONFIG_DIR` at all; the variable set, even to `~/.claude`, means a hashed one.
On macOS, the item for a folder can be found without its secret:

```sh
security find-generic-password -s "Claude Code-credentials-$(printf %s "$CLAUDE_CONFIG_DIR" | shasum -a 256 | cut -c1-8)"
```

Removing a folder by hand leaves its Keychain item behind; sign out from the card instead, which runs the CLI's
logout before it removes the folder.

## A headless Mac

Every `claude` start (a lane, a thread's agent, the 30 s status check) reads the login Keychain. When the Keychain is
locked, or its daemon is stuck, they wait or fail: lanes stall even though cooee and herdsman store nothing. So a
headless Mac needs:

- the login Keychain unlocked after boot (log the user in once, or `security unlock-keychain` in that session);
- no lock timeout: in Keychain Access, the login Keychain's settings with "Lock after … of inactivity" and "Lock
  when sleeping" off (`security set-keychain-settings ~/Library/Keychains/login.keychain-db` with no flags does the
  same);
- the host awake: no system sleep (`sudo pmset -a sleep 0`, or `caffeinate -s` while it serves).

A stuck Keychain shows as `security find-generic-password` hanging; logging out and in again, or restarting the Mac,
clears it.

## Unattended lanes: a token file

A lane that must start without the Keychain can use a long-lived token instead: `claude setup-token` prints one; keep
it in a file only the owner can read (mode 0600, outside every repo and worktree) and export it as
`CLAUDE_CODE_OAUTH_TOKEN` in the lane's shell. That is the owner's choice and outside cooee: cooee neither writes
nor reads the file, and an account signed in from the card does not need it.
