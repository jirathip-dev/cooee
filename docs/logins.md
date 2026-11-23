# Where logins live

**#470 is reversed: shed owns the accounts** ([accounts.md](accounts.md)). cooee never reads or stores a token or an
account folder. An account is a config folder shed keeps; `[sign in]` is shed's `account.login`, which runs the
provider's own login with the folder in its variable (`CLAUDE_CONFIG_DIR` for Claude Code) in a terminal whose screen
shed redacts, and the CLI keeps the login where it always does. The sign-ins card shows what shed last read
(`account.ls`); a login's end, and opening the card, read it again (`account.status`).

## Claude Code, per OS

| OS | System default login (`~/.claude`) | An added account (`CLAUDE_CONFIG_DIR=<folder>`) |
|---|---|---|
| macOS | login Keychain, generic password `Claude Code-credentials` | login Keychain, `Claude Code-credentials-<hash>`: `<hash>` is the first 8 hex digits of the SHA-256 of the folder's path, exactly as the variable gives it |
| Linux | `~/.claude/.credentials.json` (mode 0600) | `<folder>/.credentials.json` (mode 0600) |

The plain item is for no `CLAUDE_CONFIG_DIR` at all; the variable set, even to `~/.claude`, means a hashed one. That
is why shed adopts an older cooee's folders at their own path: the item stays the same.

Removing a folder by hand leaves its Keychain item behind; sign out from the card instead (`account.rm`, which runs
the CLI's logout before it removes the account).

## A headless Mac

Every `claude` start (a lane, a thread's agent, a status read) reads the login Keychain. When the Keychain is locked,
or its daemon is stuck, they wait or fail. So a headless Mac needs:

- the login Keychain unlocked after boot (log the user in once, or `security unlock-keychain` in that session);
- no lock timeout: in Keychain Access, the login Keychain's settings with "Lock after … of inactivity" and "Lock
  when sleeping" off (`security set-keychain-settings ~/Library/Keychains/login.keychain-db` with no flags does the
  same);
- the host awake: no system sleep (`sudo pmset -a sleep 0`, or `caffeinate -s` while it serves).

A stuck Keychain shows as `security find-generic-password` hanging; logging out and in again, or restarting the Mac,
clears it. For runs that must start without the Keychain, shed keeps a setup token beside the account
(`shed account login <provider> <name> --token`).
