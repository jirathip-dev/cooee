# Fleets: every setting, add and remove a repo, pause and resume, by manual edit or proposal (#184, R1)

This design covers managing herdsman's fleets from the native app (owner q1095): every fleet.toml setting, adding and
removing a repo, pausing and resuming, through both a manual edit and a proposal. The API is herdsman#786.

**The screens are #125 blocks.** Each screen is a fixture (`fixtures/*.json`) in the shape drover would answer it
over herdsman#786. screens-as-data's one renderer (`../screens-as-data/sd.js`) draws it over the orchestrator's chat (the
Conductor persona of #125), the same way drover's Queue is drawn, so these frames are the blocks themselves and not hand-made HTML. Fleets keeps its
place from #125 revision 1: a sheet under the Queue sheet's `⋯`. The look is #124's native v1 (`apple/design/v1/`) in
its three styles (q947).

**What this adds to the block set,** in `sd.js` and `sd.css`. #125's existing frames are unchanged, and all 21 still
pass its shoot.

- `diff`: a file's unified diff in hunks of kept, removed and added lines.
- `editor`: text with line numbers and each validation error inline under its line.
- On a row: `button` (Pause and Resume in the row), `handle` (≡ drag to reorder), `lifted` (the row being dragged) and
  `changed` (a mark on an edited value).
- On a form field: `doc` (its one-line doc), `was` (the value before your edit), `error`, and a picker's `open`
  menu.
- On a screen: `back` (a pushed screen inside the sheet) and `pending` (the unsaved-changes bar).
- Card kind `proposal`.
- Scene draws a figure that has a `coat` as its horse acting the state.

| page | frames |
|---|---|
| `fleets.html` | the Fleets list (and scrolled), Add a repo, Global settings (and scrolled), the list at 1440 |
| `settings.html` | a fleet's settings (and scrolled), a role, Edit as text, Remove, the settings at 1440 |
| `change.html` | Review changes, a stale edit, proposals in the orchestrator chat, Review at 1440 |

`?f=<frame>&mode=dark|light&style=modern|terminal|scene` shows one frame. Serve the repo root over http.
`node docs/design/fleets-config/shoot.mjs <chrome-headless-shell> [regexp]`, run from the repo root, regenerates
every PNG. It checks each one for sideways scroll, anything past the right edge, and cut text without an ellipsis.
**All 32 pass.** The 390 shots are 390 × 844 at 2× and the 1440 shots are 1440 × 900 at 1×. Scheme: farm. Data:
illustrative, taken from the real fleet.toml (its fleets, roles, accounts A → B → C, hosts and limits).

**R1 (q1247, amendment q1248).** "Orchestrator may" now shows herdsman's real allow groups (`design`, `clean`,
`hosts`, `reclaim`, `accounts`, `runners`, `pause`, `set`, from herdsman's `GROUPS` table, which herdsman#786 serves),
each with the verbs it covers and herdsman's default (all on but `pause` and `set`). Merge is in no group and is never
a switch. The proposer is called **orchestrator** everywhere, not Conductor. The four open questions are settled as R0
recommended: Fleets stays under the Queue sheet's `⋯`; a stale edit touching the same line keeps Apply off; a
proposal's Apply stays one tap on its card. Changed frames: `global-end`, `fleets` (the proposal's who), `stale`,
`proposal`.

## The PNGs and the decision each shows

### `fleets.html`

| PNG | shows | decision |
|---|---|---|
| `fleets-390-dark.png`, `-light.png` | **Fleets**, pushed from the Queue sheet's `⋯` (`‹ Queue`). A pending **proposal** is at the top, as the very card the orchestrator's chat shows. Under it, **each fleet is a row**: its state figure (driving = working, paused = idle, in amber), its lanes and repo, **[Pause] or [Resume] in the row**, and › to its settings. | Pause and Resume are one tap from the list. A proposal shows the same card here and in the chat |
| `fleets-end-390-dark.png` | Scrolled down: **+ Add a repo**, then **every fleet**: **Global settings** and **Edit fleet.toml as text**. | Global settings live beside the fleets, not in app Settings |
| `add-390-dark.png`, `-light.png` | **Add a repo**, a sheet with Cancel and Review. You pick the GitHub repo (from what your gh sign-in reaches), the fleet name, the folder on mini (cloned if missing) and the branch it merges to (never main). **Roles come from defaults** (impl, rev, then design, architect and verify), and the fleet **starts paused**. Review shows the `[fleet.<name>]` table it adds. | Adding a repo is a manual edit like any other: it goes through Review, then Apply |
| `global-390-dark.png`, `-light.png` | **Global settings**: **accounts** with #183's usage and sign-in (in use, capped and when it resets, + Sign in another account), **hosts** with their state, and **limits** as pickers, each with a one-line doc where it isn't obvious. | Accounts reuse #183's rows, not a second account screen |
| `global-end-390-dark.png` | Scrolled down: **notify** (web push, + Add a notifier) and **Orchestrator may**: herdsman's eight `[orchestrator] allow` groups as switches, each with the verbs it covers (Design and architect lanes, Clean, Hosts, Reclaim, Accounts, Runners on; Pause and resume, Set a role off, herdsman's default). No Merge switch: "Never merge: only you and drive do. Anything else is a card for you." | The switches are herdsman's own groups as herdsman#786 serves them, and merge is never one |
| `fleets-1440.png` | The Mac window: the sidebar, the orchestrator's chat, and the Fleets sheet as a centred panel, as #125's Queue is at 1440. | The same JSON at every size |
| `fleets-terminal-390-dark.png` | Fleets in **Terminal**: mono, the in-row buttons drawn bare. | Terminal style |
| `fleets-scene-390-light.png` | Fleets in **Scene**: each fleet is its horse. Driving ones are on the trail and paused ones are grazing. | Scene style: the state is the horse's pose |

### `settings.html`

| PNG | shows | decision |
|---|---|---|
| `fleet-390-dark.png`, `-light.png` | **A fleet's settings** (cooee), grouped. At the top: Driving · 3 lanes with [Pause]. Then **roles** (one row each: harness, model, effort, account order), **queue** (merges to, UI paths, checks, pre-merge), serial groups and more. The edited **rev** row carries the change mark, and the **pending bar** (`2 changes · unsaved · Discard · Review changes`) floats at the bottom. | Edits collect and nothing is written until you Review. One bar reaches Review from any depth |
| `fleet-end-390-dark.png` | Scrolled down: serial groups (+ Add a group), **limits for this fleet** ("unset uses Global settings"), the verify recipe, notifiers and **lessons** (`off`, `was weekly`), Edit as text, and **Remove cooee from herdsman…**. | Every fleet.toml key of a fleet has a place, and an inherited value says where it comes from |
| `role-390-dark.png`, `-light.png` | **A role** (rev). Harness, Model, Effort and Host are **native pickers** whose menus list only herdsman's allowed values (the Model menu is open: opus, sonnet, haiku 5.5). Effort shows `was model default`. Under them, **accounts in order**, with C being dragged above B ("A lane signs in with the first one not capped"). | Pickers come from herdsman's values, so you can't pick a value it would refuse. Account order is a drag |
| `role-terminal-390-dark.png` | The role in Terminal. | Terminal style |
| `text-390-dark.png`, `-light.png` | **Edit as text**: `[fleet.cooee]` in fleet.toml, with its comments, line numbers and **two errors inline** under their lines (`MAIN_TARGET: integration can't be main…`, `effort: one of none, low, medium, high, xhigh`). The banner says "2 errors · fix them to review", and Review is off. | Text edits are validated as you type, with herdsman's own words, and can't be applied invalid |
| `remove-390-dark.png` | **Remove**: the app's own action sheet. It names **what goes** (`[fleet.cooee]` and its 5 roles, with a backup) and **what stays** (the GitHub repo, `~/Projects/cooee`, its 3 live worktrees, its event history). "Live lanes finish first." The button says **Remove its config**. | Remove deletes config only, and the confirm says so |
| `fleet-1440.png` | The settings at 1440, light, with the pending bar inside the panel. | as above |
| `fleet-scene-390-light.png` | The settings in Scene. | Scene style |

### `change.html`

| PNG | shows | decision |
|---|---|---|
| `review-390-dark.png`, `-light.png` | **Review changes**, a sheet with Edit and Apply. It opens with **valid · herdsman checked them**, then **what changes** in words (`rev · effort: model default → high`, `lessons: weekly → off`), then **the diff** of `~/.config/herdsman/fleet.toml`, with its comment lines kept. "Apply writes fleet.toml once, keeps the old one as `fleet.toml.bak-…` and runs doctor." | One Review sheet for every manual edit: in words and as a diff, written once, with a backup |
| `stale-390-dark.png`, `-light.png` | **Review again**: fleet.toml changed while you edited, because you applied the orchestrator's c12. It shows **what changed meanwhile** (c12's diff), then **your changes on the new file**. Nothing was overwritten. Had a change touched the same line, Apply would stay off until you edit again (q1248). | A stale edit is never applied blind: you re-review against the new file |
| `proposal-390-dark.png`, `-light.png` | **Proposals in the orchestrator's chat**, which is where asks live (#125: asks are chat cards). One card is the orchestrator's (`synergy · serial_max 3 → 6`, `orchestrator · thread t55`), and one is a lane's (`berth · premerge adds the test gate`, `lane berth-impl-41 · from its PR #41`). Each card shows **who**, **why**, its diff (marked valid) and [Apply] / [Discard]. | A proposal is a card with its diff, who and why. Apply is one tap on the card (q1248) and goes through the same validated write and backup |
| `review-1440.png` | Review at 1440, as a panel. | as above |
| `review-terminal-390-dark.png`, `review-terminal-1440.png` | Review in Terminal: the diff ruled with dashes. | Terminal style |
| `proposal-scene-390-light.png` | The proposal cards in Scene. | Scene style |

## Open questions for the owner

Settled by q1248: Fleets stays under the Queue sheet's `⋯`; a stale edit touching the same line keeps Apply off; a
proposal's Apply is one tap on its card; the groups are herdsman's eight, never merge.

1. **The chat persona's name.** The proposer is now "orchestrator" in every #184 string, but the chat itself is
   still titled **Conductor** (#125's persona, drawn by the shared `sd.js`, as in `proposal-*` and the 1440 frames).
   Renaming the persona is #125's call, so this design leaves it. Rename it to Orchestrator there?
2. **Global settings' home.** Should Global settings (accounts, hosts) also be reachable from app Settings,
   beside #183's accounts?
