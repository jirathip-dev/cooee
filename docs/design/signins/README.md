## Summary

Decision: Sign-ins lists every account under its shed machine. A machine that doesn't answer shows one "offline" row. Each row has the account's state and one short figure per usage window shed reports. The fullest window, if 80% or more, sits beside the name in amber. API-key accounts show this month's tokens and cost instead. Tapping a row opens the account: each window's % and reset time, the reading's age, this month's figures when shed has them, Sign in again, and Sign out with a confirm.
Screens: Sign-ins (grouped list), an account's detail (subscription and API-key versions), the Sign out confirm; each at 390 dark and light, and at 1440 as a split view.
Open questions: none.

# Settings › Sign-ins: usage, every machine, an account's detail (#793)

**The frames are blocks screens.** Each one is a fixture in `fixtures/`, shaped as the server would send it from
`/screens/settings/signins` and an account's own route. `si.js` draws them the way blocks' renderer lays out a row:
title, then badge, then subtitle under them, then value, then chevron. It uses `design-system/app.css`, plus `si.css`
for this page's layout. The design adds no block kind and no field. Everything uses the `section`, `list`, `row`,
`stat` and `button` blocks at the repo's blocks pin (cf626c6).

`signins.html?f=<fixture>` shows one screen at 390. `&side=<fixture>&sel=<row id>` shows the 1440 split, and
`&confirm=<button id>` adds the confirm. Serve the repo root over http. `node docs/design/signins/shoot.mjs
<chrome-headless-shell>`, run from the repo root, redraws every PNG. It fails a frame with sideways scroll, anything
past the right edge, or text cut without an ellipsis. **All 10 pass.** The 390 shots are 390 × 844 at 2×, and the 1440
shots are 1440 × 900 at 1×. Data is illustrative, and times count from 2026-11-23 14:00 UTC.
`cooee plugin check docs/design/signins/fixtures/*.json`, with a `cooee` built at this branch (blocks cf626c6),
passes all three fixtures. An older install (blocks d088209) refuses the `Time` inside a subtitle label.

## The PNGs and the decision each shows

| PNG | shows | decision |
|---|---|---|
| `signins-390-dark.png`, `-light.png` | **Sign-ins**, one `section` per shed machine, headed by the machine's name (`Mini · this machine`, `Air`, `boat-1`). Each row's title is `Provider · who`. Its subtitle is the state, toned: signed in, at its limit, signed out. Its `value` holds the usage windows in shed's order (`7d 36%`). | Grouped by machine, as cooee's machine list knows them. Provider names and window ids are shed's strings, with no list in cooee. |
| same | `Claude · work@acme.dev` has **`5h 84%` as an amber badge** beside its name, and `7d 41%` stays in the value. | The fullest window at 80% or more moves into the row's `badge`, toned `warn`. The rest stay in `value`. Under 80%, every window is in `value` and there is no badge. A row's `value` can't carry a tone, so the badge is how blocks shows the warning (see below). |
| same | `Claude · jk@hey.com` shows only `7d 36%`, and `Antigravity · jk@hey.com` shows **no figure**. | Only the windows shed reports are shown. A plan with no 5h window shows just its 7d. An account with no windows shows its state only. |
| same | `Anthropic API · ci` shows `1.2M tok · $18.40` in its value. | An API-key account's figure is this month's tokens and cost, from shed's session counts. Until shed reports them, the row shows its state only. |
| same | `Claude · air@acme.dev` (Air): **`at its limit · resets in 2h 14m`**, amber, with `7d 100%` as its badge. | A limited row says when it resets: `limited_until`, else the fullest window's `resets_at`. It is a blocks `Time` (`until`, label `… resets in {rel}`), so it counts down without a refetch. |
| same | `boat-1` has one row: **offline**, "its shed doesn't answer". It has no chevron. | A machine that doesn't answer is a row under its own header, not an error screen. The other machines still list. |
| `account-390-dark.png`, `-light.png` | **The account** (`Claude · work@acme.dev`, back to Sign-ins): State (badge) and Machine. Then **Usage**: one row per window, with its id as the title, its % as a badge (amber at 80% and up) and `resets in 1h 12m` / `resets in 3d 19h` as a counting `Time`. Then **Read by shed · 2 min ago** (a `Time` since `read_at`). Then **[Sign in again]** and **[Sign out]**. | The detail lists every window shed reports, with its reset and the reading's age. This month's tokens and cost are left out while shed has no figures for this account (the issue's note). |
| `account-api-390-dark.png` | **An API-key account** (`Anthropic API · ci`). It has no usage windows, so there's no Usage section. **This month** has two `stat` tiles, Tokens `1.2M` and Cost `$18.40`, with the footer "From shed's own session counts on Mini". Then the reading's age and the two buttons. | When shed has the month's figures, the detail shows them as tiles. A section with nothing from shed is left out, not shown empty. |
| `signout-390-dark.png`, `-light.png` | **Sign out's confirm**: "Sign out Claude · work@acme.dev on Mini?", "shed removes this sign-in from Mini. To use it there again, sign in again." It has **Sign out** (destructive) and Cancel. | Sign out is the button's own `confirm` (`destructive: true`). Its action calls shed's `account.rm` on that machine, the same call today's sign-out makes. Nothing happens without the confirm. |
| `signins-1440.png` | At 1440: the Settings sidebar, Sign-ins, and the selected account's detail beside it. The selected row is highlighted. | The same two screens make the split view. A row's `tap` opens its detail as a pushed sheet on a phone and in the next column on a wide screen. |
| `account-1440.png` | The split in light, with the API-key account selected. | as above |
| `signout-1440.png` | The confirm at 1440, centred over the split. | as above |

## Notes for impl

- **Routes.** `/screens/settings/signins` returns the grouped list. Each row's `tap.sheet` is
  `/screens/settings/signins/<machine>/<provider>/<account>`. Row ids are `<machine>/<provider>/<account>`, so two
  machines with the same account name don't clash. Ids come from shed's provider and account names, never from a
  list in cooee.
- **Machines.** These are the machines `cooee_machines` knows. Each machine's shed is read through shed-client. A
  machine whose shed doesn't answer within the screen's timeout becomes the `offline` row. The other sections still
  render.
- **Row figures.** Write each window as `<id> <used_pct rounded>%`, in shed's order, joined with ` · ` in `value`. If
  the fullest window is at 80% or more, it moves to `badge` with `tone: warn`. An account limited with no windows
  reported keeps its `at its limit · resets in …` subtitle (from `limited_until`), with no badge.
- **API-key month figures** are `<tokens> tok · $<cost>`, from shed's per-account counts. Until shed exposes them,
  leave the value out (the issue's note; a separate shed issue).
- **Sign in again** sends `signin.again` with `{machine, provider, account}`. It runs shed's `account.login` on that
  machine and opens the existing sign-in sheet (the login terminal, its links and the code field from
  `login/sign_in.rs`), pointed at that machine's shed. It isn't drawn here because that sheet is unchanged.
- **Sign out** sends `signin.out` with the same values, after the confirm. The row then reads `signed out` (or is
  gone, as shed's `account.rm` leaves it) on the next refresh.
- **Tests** (from the AC): rows for an account with 2 windows (badge plus value), 1 window, none, and an API-key
  account; machine grouping with one machine offline. Assert the blocks JSON the route returns.
