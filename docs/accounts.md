# Accounts: shed's

**#470 is reversed.** cooee used to own its accounts: folders under its data folder, a login runner, status checks
every 30 s, a `/usage` reader and `cooee accounts env` for a fleet engine. Since #764 (shed's accounts ADR,
shed#256) shed owns all of it, and cooee reads and changes accounts only through shed's `account.*` calls:

| call | what cooee uses it for |
|---|---|
| `account.ls` | the sign-ins sheet's rows: per provider `default` first, each account's state, who it is and its usage |
| `account.add` | `[+ account]` (a new `account-<n>`), and handing an older cooee's folders to shed (below) |
| `account.login` | `[sign in]`: the provider's login in a terminal shed redacts; cooee shows its screen and types the code |
| `account.status` | how a login ended, and the card's refresh |
| `account.rm` | `[sign out]`: shed runs the logout and removes the account (never `default`) |

A thread names its account to shed (`shed run --acp <agent> --account <name>`, `RunParams::account`); shed sets the
account's folder itself, so no run of cooee's sets a config-folder variable in `env`. The main login is no account
at all: the run gets shed's own environment.

`cooee accounts` is gone: a fleet engine asks shed (`shed account ls`, `shed run --account`).

## Upgrading

At start cooee hands every account an older cooee kept to shed, in place: each `<data>/accounts/<provider>/<name>`
folder (a link's own target, else the folder) by `account.add`'s `folder`, and each `<name>.token` by its
`token_file`. The folder keeps its path, so its login (Claude Code's Keychain item is named for the path) is the same
one and nobody signs in again. Adding the same folder twice is one account, so every start does it again harmlessly;
cooee never removes the folders. A folder shed refuses (a name outside `[a-z0-9-]+`, or `ACCOUNT_SETUP`: it holds
a setup such as `settings.json`) is logged on stderr and left alone.
