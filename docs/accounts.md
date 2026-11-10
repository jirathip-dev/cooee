# Accounts: the credential helper contract

cooee owns its accounts: their folders, their sign-in (the sheet, `docs/architecture.md`) and this helper. A fleet
engine (herdsman) gets an account's login environment from `cooee accounts env` and lists them with
`cooee accounts ls --json`; it never reads cooee's folders. Every CLI keeps its own login: cooee never stores a
token, and a folder account's token is never read or printed.

Configure herdsman with `account_env = "cooee accounts env {provider} {name}"`. With it, cooee's account list wins
over herdsman's own `[accounts]`; herdsman falls back to its folder source when the helper fails. `--instance` and
`--config` pick the cooee instance as for any other command.

## Providers

One table, the config's `[[providers]]` `home` variable (`docs/architecture.md`, Configuration):

| provider | variable |
|---|---|
| `claude` | `CLAUDE_CONFIG_DIR` |
| `codex` | `CODEX_HOME` |
| `gemini` | `GEMINI_CLI_HOME` |

## `cooee accounts env <provider> <name> [--require-signed-in]`

Prints one JSON object of environment variables on one line and exits 0:

```console
$ cooee accounts env claude work
{"CLAUDE_CONFIG_DIR":"/home/user/.local/share/cooee/accounts/claude/work"}
```

- The name `""` is the system default login (`~/.claude`, …): `{}`.
- Signed in is the provider's own status command run for the folder (exit 0), never a file guess. It runs without
  an inherited `CLAUDE_CODE_OAUTH_TOKEN`, which would sign every folder in: a folder whose own Keychain login is gone
  reads as signed out. A signed-out account is still printed,
  with `cooee: claude/work is signed out` on stderr; `--require-signed-in` makes it a refusal instead.

| exit | meaning | stdout |
|---|---|---|
| 0 | the environment | one JSON object |
| 1 | refused: unknown provider (``cooee: no provider `x` ``), unknown account (``cooee: no account `x` of provider `y` ``), a token file looser than 0600, empty or unreadable | empty |
| 2 | usage | empty |
| 3 | signed out, with `--require-signed-in` | empty |

A refusal is one line on stderr starting `cooee: `.

## `cooee accounts add <provider> <name> --folder <path>` and `cooee accounts rm <provider> <name>`

`add` adopts an existing config folder (herdsman's `~/.config/herdsman/accounts/A`, …) as an account without a new
sign-in: `<data>/accounts/<provider>/<name>` becomes a link to it, and the account's folder is the folder's own path
(Claude Code keys its Keychain login by that path). A name is letters, digits and `-`. Adopting the same folder
again is a no-op; a name already taken by another folder is refused (exit 1). `rm` drops an account: an adopted
folder's link only, the folder stays. Both exit 0 when done, 1 refused, 2 usage.

## Token file (escape hatch)

For an unattended host that must not depend on the macOS login Keychain, a Claude account may be a hand-placed token
file instead of a folder: `<data>/accounts/claude/<name>.token`, mode 0600 or tighter, holding a token from
`claude setup-token`. cooee never writes it. The helper then prints its value, trimmed:

```console
$ cooee accounts env claude ci
{"CLAUDE_CODE_OAUTH_TOKEN":"…"}
```

A token file replaces a folder of the same name. Anything looser than 0600 is refused (exit 1). Folder logins stay
the default; only `claude` reads token files.

## `cooee accounts ls --json`

A JSON array, every provider's accounts, each provider's system default first, then its folders by name, then its
token files by name. It never prints a token.

```json
[
  {"provider": "claude", "name": "", "folder": null, "signed_in": true, "token_file": null, "token_age_days": null, "error": null,
   "usage": {"five_hour": null, "seven_day": {"used_pct": 83, "resets_at": 1794546000}, "read_at": 1794302253, "age_s": 40}},
  {"provider": "claude", "name": "work", "folder": "/home/user/.local/share/cooee/accounts/claude/work", "signed_in": false, "token_file": null, "token_age_days": null, "error": null, "usage": null},
  {"provider": "claude", "name": "ci", "folder": null, "signed_in": true, "token_file": "/home/user/.local/share/cooee/accounts/claude/ci.token", "token_age_days": 41, "error": null, "usage": null}
]
```

| field | |
|---|---|
| `provider`, `name` | what `accounts env` takes |
| `folder` | the config folder; `null` for the system default and token files |
| `signed_in` | a folder: its status command exited 0; a token file: it is a file no looser than 0600 |
| `token_file`, `token_age_days` | the token file and its age in whole days since it was last written (doctor warns near 300); else `null` |
| `error` | why a token file is refused; else `null` |
| `usage` | a signed-in folder account's 5 h and 7 d windows (`used_pct`, `resets_at` in s since the epoch; `null` when the reading has none), `read_at` and the reading's `age_s`; else `null` |

Usage is the provider's `usage` command (`[[providers]]`; Claude Code's `/usage`, `claude -p /usage` with `TZ=UTC`):
its `Current session` line is the 5 h window, `Current week (all models)` the 7 d one. A reading stands 5 minutes
in `.cooee.json` beside the folders; when the command fails the last reading is shown with its age. A window past its
reset time reads as reset, 0 %, not the old number. Settings › providers shows the same reading under a signed-in
account: `5h — · 7d 83% · read 5m`.

Without `--json`, `cooee accounts ls` prints one plain line per account. Tested by `crates/cooee/tests/accounts_env.rs`.
