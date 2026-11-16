# ADR: one folder per agent, with automatic migration (#359)

## Summary

Decision: One folder per agent in the config folder replaces persona and profile with one thing; notes become
Markdown bullets and routine definitions one settings file. The first start converts everything in one step, keeps
the old folders as dated backups, and leaves the old layout untouched if it can't finish.
Slices: the agent folder (read, write, start a chat); notes and skills inside it; routines split into settings and
run state; the migration with its dry run and the upgrade test. One PR, in that order.
Open questions: three, each with a default: personal setup, wire names, comments lost on app edits (see the end).

Files: [`callers.md`](callers.md) (written first), [`shapes.md`](shapes.md), [`boundaries.md`](boundaries.md),
[`migration.md`](migration.md).

## Question

How do persona and profile become one agent that lives in `<config>/agents/<id>/` (q1605, q1609), with its notes
and skills beside it and routine schedules in `<config>/routines.toml`, so that the tree is hand-editable and
git-able, an upgrade converts every install once without losing or deleting anything (q1617), and the line cap
holds?

## Constraints found in the code

1. **Two types, one precedence rule.** `recipe()` (cooee-server `lib.rs`) starts a persona as "the profile it runs
   as": the profile's `agent` and `folder` win over the persona's, the persona's instructions replace the profile's
   file. Whatever merges them must keep what starts today.
2. **Personal setup (#320)** is kept out only on the profile path (`Logins::profile`); a persona with no profile
   starts with the owner's personal setup. One start path has to pick one behaviour.
3. **The API never sets a profile's keys**: the native app sends a persona without account, settings, env, MCP or
   policy, and `personas::save` keeps `profile` from the stored one.
4. **`cooee_core::Agent` is taken**: it is shed's installed ACP agent.
5. **No TOML writer**: `toml` is built with `parse` and `serde` only; files are written by hand ("a JSON string is
   a TOML basic string").
6. **Headroom is 230 lines** (27,270 of 27,500).
7. **Ids are already shared**: a thread's `profile` holds a persona's id or a profile's name, and a profile read as
   a persona has the profile's name as its id.
8. **A routine is one struct in one JSON file** holding both its definition and its run state; the runner and the
   web read that struct.
9. **The upgrade fixtures are flat**: one folder is both config and data; `personas/` exists in both trees.
10. **A parse error's message can quote a value** (`invalid type: string "…"`), and today it is logged.

## Options

| | Choice | For | Against |
|---|---|---|---|
| Merge A | New files, but `Persona` and `Profile` stay in Rust and are joined at read | Smallest diff | Two names for one thing and the precedence rule stay; the old API isn't deleted |
| Merge **B** | **One `agents::Agent`; a plugin's `Profile` is lent as one, read-only** | One start path; `personas.rs` and most of `profiles.rs` go | Renames across the server and the web picker |
| Launch keys A | The API may set them | One rule | A paired device could set `policy = "allow"`; not possible today |
| Launch keys **B** | **Hand-edited; `save` keeps the folder's** | As today's `profile` | A second, smaller entry (`copy`) for `+ new agent` |
| Notes A | Backslash escapes for line breaks | Exactly reversible | A hand-typed `\` is misread; less plain |
| Notes **B** | **`<br>` for a line break; trailer comment for who and when** | Plain Markdown, renders right | A literal `<br>` in a note reads back as a line break |
| Writing TOML A | `toml_edit`, or `toml`'s `display` feature | Keeps comments (`toml_edit`) | A new crate in the lock: an owner call |
| Writing TOML **B** | **Whole-file rewrite by the existing idiom** | No dependency | An app or agent edit drops hand-written comments in that file |
| Routines A | Two Rust types, joined by every caller | Types mirror the files | The runner, the routes and the web all change |
| Routines **B** | **`Routine` stays the joined value; its store reads and writes two files** | The runner is untouched | The store knows which key goes where |
| Atomicity A | Write `agents/` in place, then rename the old folders | Simplest | A crash leaves half the agents, and "`agents/` exists" then blocks the rest for good |
| Atomicity **B** | **Stage in `agents.migrating/`, commit by one rename, then the backups** | All or nothing; no journal | A crash between commit and backups leaves old names (harmless, reported) |
| Wire A | Rename `/personas` to `/agents`, `profile` to `agent` | One word everywhere | A thread-format bump and an app release, for no owner-visible gain here |
| Wire **B** | **Keep every wire name** | No app release; no thread rewritten | "persona" stays in routes and Swift |

## Decision

**Merge B, Launch keys B, Notes B, Writing B, Routines B, Atomicity B, Wire B.**

1. **An agent is its folder** `<config>/agents/<id>/`: `agent.toml` (every structured key of persona and profile),
   `INSTRUCTIONS.md`, `CHARACTER.md`, `memory.md`, `skills/` ([shapes](shapes.md)). The minimum is `agent.toml`
   with `name` and `agent`; missing Markdown files are empty. Files are read on each call, so a hand edit needs no
   restart.
2. **One Rust type, `agents::Agent`**, always written module-qualified (the crate root's `Agent` stays shed's). It
   is the API body of `/personas`, the web picker's row and `recipe()`'s input. A plugin's `[[profiles]]` still
   parse as `Profile`, which only `Profile::lend` turns into an `Agent` marked `lent`: read-only, no folder, no
   notes or skills, exactly as today.
3. **One start path** ([callers](callers.md) §2): Plain, or one agent's account, settings, env, policy, MCP picks,
   folder, model, effort, machine and brief. The persona-then-profile precedence is resolved once, by the
   migration, into the folder.
4. **Notes and skills take a `Home`**, the agent's folder, which only `agents::home` makes. `memory.md` is one
   `- [id] text <!-- by:… at:… -->` line per note; the 2,000-character cap, the ids and the skills' caps stay.
5. **Routines**: `<config>/routines.toml` holds id, name, schedule, prompt, place and the owner's pause;
   `<data>/routine-runs.json` holds each one's making, last due time, last run and a chat-gone pause. A routine
   typed by hand first runs at its next due time.
6. **The migration is a plan, then its apply** ([migration](migration.md)): `plan()` only reads and yields steps;
   `--dry-run` prints them; the real run logs the same lines. It stages under `agents.migrating/`, commits by one
   rename, then renames the old paths to `*.bak-<date>`. It deletes nothing and edits no old file. `agents/`
   existing is the only "done" marker. It runs first thing in `cooee serve` and as `cooee config migrate`; a
   failure is logged and never stops the start.
7. **Malformed means hidden, alone**: a bad `agent.toml` hides that agent; the log names the file and the line or
   key, never a value (stricter than today, because of constraint 10).
8. **The data folder** loses `personas/` and `routines.json`. Nothing else moves: plugin builds and their lock,
   secrets' files, keys, models and logs are not hand-edited settings.

## Consequences

- **Deleted:** `personas.rs`; `profiles::{list, find, file, create}`; `mcp::save_picks`; reading an owner profile
  as a persona; the `conductor`-is-an-orchestrator default; `Persona.profile`. **New:** `agents.rs`, `memory.rs`,
  `migrate.rs`; `skills.rs` and `routines.rs` change where they read. `docs/api.md` and `docs/architecture.md`
  follow.
- **Size:** about `src/ net 0`, budget `+100`, inside the 230 headroom: roughly −650 (personas, profiles' owner
  half, picks' writer, the two-step recipe) against +630 (agents ~300, memory ~115, migrate ~170, routines ~45).
  If another merge eats the headroom first, the impl lane trims, then asks under the cap rule.
- **Behaviour that changes, on purpose:**
  1. A persona that had no profile now starts without the owner's personal setup (question 1).
  2. An agent with no `account` uses the account the request names; a profile used to ignore it.
  3. Where a persona's name differed from its profile's label, the web sidebar now shows the persona's name.
  4. `GET /personas` no longer has `profile`; it gains the read-only launch keys. A body carrying `profile` is a
     400; the app only sends it back when the server sent it.
  5. A line in `memory.md` that isn't a bullet is dropped the next time cooee rewrites the file.
  6. A parse error's log line gives the file and line, not the parser's sentence.
- **Until the first `serve` of the new build**, other commands (`machine check`) see no agents: only
  `serve` and `config migrate` migrate.
- **The backups sit in the git-able tree** (`personas.bak-<date>`, `profiles.bak-<date>`): the owner deletes or
  ignores them; cooee never does.
- **Tests** ([migration](migration.md)): the upgrade test migrates every release's fixtures against a byte-exact
  expected tree and re-runs to prove "once"; the binary's tests cover the dry run's hashes, a hand-written folder
  and a malformed one. The existing persona, profile, skill and routine tests keep their assertions and write
  agent folders in their setup.
- **Out of scope:** renaming the wire or the Swift types; moving plugins, secrets or themes; a watch on the folder
  (it is read on each call); committing or pushing the tree for the owner.

## Open questions for the owner

1. **Personal setup.** After the merge every agent starts one way. **Default:** every agent folder starts without
   your personal setup (global instructions, personal plugins, your own connectors), as #320's note says for "a
   profile or persona"; a plain chat keeps it. Today a persona with no profile still gets it. Alternative: a
   `personal_setup = true` key, written by the migration for those personas, so nothing changes for them.
2. **Wire names.** **Default:** `/personas`, a thread's `profile` and the Swift `Persona` keep their names, so no
   app release or thread rewrite is needed; a rename is its own issue. OK?
3. **Comments.** **Default:** no new dependency, so when the app or an agent edits an agent or a routine, that
   `agent.toml` or `routines.toml` is rewritten whole and hand-written comments in it are lost (the Markdown files
   are not affected). Alternative: add `toml_edit` to keep them.

## `.drover/arch.toml`

No lines: this issue converts no hot file. (The agents themselves follow the same idea: one folder per entry,
collected by reading `agents/`, instead of two shared folders of files.)
