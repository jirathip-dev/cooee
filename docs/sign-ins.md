# Sign-ins (M3) and accounts

## Today

- **Sign-ins and accounts:** the provider's own login runs from the phone; several accounts per provider, each with usage (#18, #46, #191).

## How it works

shed owns the accounts ([accounts.md](accounts.md): #470 is reversed); cooee draws the sheet over shed's `account.*`
calls. Each provider is one row of the `providers` table in the config (its name, which is shed's provider of its
accounts, and the ACP agent it signs in). Settings' sign-ins row lists, per provider, each account `account.ls` has
(`default` first): signed in, signed out or at its limit, and who it is. `[+ account]` is `account.add` of a new
`account-<n>`, then its login; it is removed again (`account.rm`) unless the login ends signed in. `[sign out]` is
`account.rm` (shed keeps `default`). A thread on an account starts as `shed run --acp <agent> --account <name>`; on
the main login with no `--account`.

`[sign in]` is `account.login`: shed runs the provider's login in its terminal `login-<provider>-<name>`, whose
screen shed redacts. cooee resizes it to 500 columns so a long link stays on one line, reads its visible screen on
demand (`GET /logins/<provider>`), its links tappable, and types a code under it with `send` then `keys enter`. A
`wait` ends when the login exits (shed ends it after 10 minutes); then cooee removes the terminal, reads
`account.status` and shows how it ended. `[cancel]` removes the terminal at once. The screen is never stored or
logged. Where each OS keeps the tokens, and what a headless Mac needs, is in [logins.md](logins.md).

## API

`{id}` is a provider's name; `account` is an account's name as `account.ls` gives it (none: `default`).

| Call | Body | Answer |
|---|---|---|
| `POST /logins/{id}` | `{account?}`, or `{new: true}` for a new account | `null` |
| `POST /logins` | `{}`: the sign-ins card opened; reads every account's status again, not waiting | `null` |
| `GET /logins/{id}` | | `{state: "running" \| "ended" \| "none", screen, links: string[], why?, account?}`: `account` as `account.status` read it once the login ended |
| `POST /logins/{id}/send` | `{text}`: typed into the login's terminal | `null` |
| `POST /logins/{id}/cancel` | `{}` | `null` |
| `POST /logins/{id}/signout` | `{account}` | `null` |
