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
- Signed in is the provider's own status command run for the folder (exit 0). A signed-out account is still printed,
  with `cooee: claude/work is signed out` on stderr; `--require-signed-in` makes it a refusal instead.

| exit | meaning | stdout |
|---|---|---|
| 0 | the environment | one JSON object |
| 1 | refused: unknown provider (``cooee: no provider `x` ``), unknown account (``cooee: no account `x` of provider `y` ``), a token file looser than 0600, empty or unreadable | empty |
| 2 | usage | empty |
| 3 | signed out, with `--require-signed-in` | empty |

A refusal is one line on stderr starting `cooee: `.

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
  {"provider": "claude", "name": "", "folder": null, "signed_in": true, "token_file": null, "token_age_days": null, "error": null},
  {"provider": "claude", "name": "work", "folder": "/home/user/.local/share/cooee/accounts/claude/work", "signed_in": false, "token_file": null, "token_age_days": null, "error": null},
  {"provider": "claude", "name": "ci", "folder": null, "signed_in": true, "token_file": "/home/user/.local/share/cooee/accounts/claude/ci.token", "token_age_days": 41, "error": null}
]
```

| field | |
|---|---|
| `provider`, `name` | what `accounts env` takes |
| `folder` | the config folder; `null` for the system default and token files |
| `signed_in` | a folder: its status command exited 0; a token file: it is a file no looser than 0600 |
| `token_file`, `token_age_days` | the token file and its age in whole days since it was last written (doctor warns near 300); else `null` |
| `error` | why a token file is refused; else `null` |

Without `--json`, `cooee accounts ls` prints one plain line per account. Tested by `crates/cooee/tests/accounts_env.rs`.
