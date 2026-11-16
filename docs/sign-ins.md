# Sign-ins (M3) and accounts

## Today

- **Sign-ins and accounts:** the provider's own login runs from the phone; several accounts per provider, each with usage (#18, #46, #191).

## How it works

Each provider is one row of the `providers` table in the config: its name, the ACP agent it signs in, its CLI's own
login, logout and read-only status commands, and the variable that points its CLI at a config folder (`home`:
`CLAUDE_CONFIG_DIR`, `CODEX_HOME`, `GEMINI_CLI_HOME`). There is no provider-specific code beyond that table. The
status command signs an account in when it exits 0; its output, on one line and redacted, is the status line (a
JSON status, as `claude auth status` prints, becomes a plain line, its email or org and any `usage` shown apart). A
status command that runs past 10 s is killed and the account reads `unknown`: "can't tell: the status command timed
out". It keeps the signed-in state last known (a timeout never signs an account out) and the card offers no sign-in
for it; each timeout doubles that account's interval (20, 40, then at most 80 minutes, which opening the card doesn't
cut short) until a check answers. A login Keychain that doesn't answer is the usual cause.

A provider's accounts are the system default login (`~/.claude` and so on, the first row) and the ones added in
cooee: each a config folder `<data>/accounts/<provider>/<name>/` that the CLI gets through its variable, so the CLI
keeps its own login there or in its keychain. cooee never reads or stores a token. `.cooee.json` beside the folders
keeps only the provider's default account and when each login last succeeded. Settings' sign-ins row opens the
sign-ins card on the thread list: per provider, each account's state (signed in, signed out, needs sign-in again),
who it is, its last sign-in and usage, with `[sign in]`, `[sign out]` and `[make default]`; and `[+ account]`, which
runs the login in a new folder (`.new-<name>`) that is listed only once the login exits 0 and the status says signed
in, else removed. `[sign out]` (after a confirm) runs the logout for that folder, then removes it (the system
default's is never removed). A provider whose CLI is missing says "not installed on this Mac" with its install line.
The new-thread picker marks each agent by its default account, `✓` or `signed out` with a link to the sheet, and has
an account picker, the default preselected; the thread's agent starts as `shed run --acp <agent> … -- env
<variable>=<folder> <the agent's adapter>` (the system default: no `env`), and the thread's header names the account.
`cooee accounts env` and `cooee accounts ls --json` hand an account's environment to a fleet engine (herdsman):
the contract is in `docs/accounts.md`.

`[sign in]` runs the login command (for an added account, `env <variable>=<folder> <login>`) in a plain shed terminal (terminal mode, not ACP; over the socket: `run`, then
`resize` to 500 columns so a long link stays on one line), named `cooee-login-<provider>-<stamp>` and labelled
`cooee-login=1`. The page reads its visible screen every second (`GET /logins/<provider>`), its links tappable; a
code typed under it goes in with `send` then `keys enter`. A `wait` on the terminal ends when the command exits or
after 10 minutes; then the terminal is killed and removed, `[cancel]` does the same at once, and cooee removes any
`cooee-login` terminals a previous run left. The screen is only ever read on demand: never stored, logged or kept
in memory. Before it goes to the page, anything that looks like a token is replaced by `[redacted]`: a run of 20 or
more token characters with both a letter and a digit, and in a link the values of `token`, `secret`, `password` and
`key` parameters. The tokens themselves stay with the CLI: where each OS keeps them, and what a headless Mac needs,
is in [logins.md](logins.md), linked from the sign-ins card.

## API

`{id}` is a provider's name; `account` is an account's name (`""`: the system default).

| Call | Body | Answer |
|---|---|---|
| `POST /logins/{id}` | `{account?}`, or `{new: true}` for a new account | `null` |
| `POST /logins` | `{}`: the sign-ins card opened; checks the accounts' status again, not waiting | `null` |
| `GET /logins/{id}` | | `{state: "running" \| "ended" \| "none", screen, links: string[], why?, status?, age?}`: `status` as last checked (`{signed_in, unknown, line, …}`; `unknown`: timed out or not checked yet), `age` in seconds |
| `POST /logins/{id}/send` | `{text}`: typed into the login's terminal | `null` |
| `POST /logins/{id}/cancel` | `{}` | `null` |
| `POST /logins/{id}/signout` | `{account}` | `null` |
