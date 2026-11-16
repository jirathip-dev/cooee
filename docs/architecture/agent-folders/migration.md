# The migration (#359, q1617)

## The plan is data

```rust
// cooee-core migrate.rs: the only module that knows the old paths
pub fn plan(config: &Path, data: &Path, today: &str) -> Plan;      // reads only

pub struct Plan { state: State, steps: Vec<Step> }

pub enum State { Fresh, Done, LeftOver, Old }

pub enum Step {
    /// A file of the new layout: its bytes, and the old files they were made from.
    Write { to: PathBuf, from: Vec<PathBuf>, bytes: Vec<u8>, note: String },
    /// A file copied byte for byte (a skill, a settings file).
    Copy { to: PathBuf, from: PathBuf },
    /// Something the owner should know: names files and keys, never a value.
    Warn(String),
    /// An old folder or file renamed as its backup.
    Backup { from: PathBuf, to: PathBuf },
}

impl Plan {
    pub fn lines(&self) -> impl Iterator<Item = String>;                 // `--dry-run` prints these
    pub fn apply(self, log: impl FnMut(&str)) -> io::Result<()>;         // logs the same lines as it goes
}
```

`--dry-run` and the real run share `plan()` and `lines()`, so what the dry run prints is what the run logs. `plan`
opens nothing for writing; the AC 3 test hashes both trees before and after.

## Statechart

```text
 `cooee serve` (first thing) or `cooee config migrate`
        │
        ▼
 agents/ exists? ──yes──► an old path still has its old name? ──no──► Done: nothing, no output
        │ no                         └─yes──► LeftOver: one `left over` line per path; nothing is touched
        ▼
 an old path exists? ──no──► Fresh: nothing is written (the first save makes agents/)
        │ yes
        ▼
       Old ──(--dry-run)──► print the plan; stop
        │
        ▼
     Staged      remove a stale agents.migrating/ (an interrupted run's), then write every agent folder under
        │        agents.migrating/, routines.toml and <data>/routine-runs.json (each atomic, same bytes on a re-run)
        │        ── a failure: agents.migrating/ removed, the error logged, serve starts; the next start is Old again
        ▼
   Committed     rename agents.migrating → agents: the one step that makes it done
        │
        ▼
   Backed up     rename personas, profiles, <data>/personas, <data>/routines.json → *.bak-<date>; each logged
        │        ── a crash here: the next start is LeftOver, which harms nothing (no code reads the old names)
        ▼
      Done
```

- **Once:** `agents/` existing is the only marker (the issue's rule). An install with routines but no agent gets an
  empty `agents/`.
- **Old paths:** `<config>/personas/`, `<config>/profiles/`, `<data>/personas/`, `<data>/routines.json`.
- **Nothing is deleted or edited:** old files are read, then their folder is renamed. `<date>` is today's UTC date;
  a taken name gets `-2`, `-3`.
- **LeftOver** also covers an owner who made `agents/` by hand before upgrading: the line says the old folder was
  not migrated and that moving `agents/` away and running `cooee config migrate` does it.
- `serve` never fails on the migration: an upgrade must not stop the service.

## What becomes what

The agents are exactly the ones the old build lists: each `personas/<id>.toml`, plus each owner profile with no
persona of its name. A persona "runs as" the profile its `profile` key names.

| New | From | A persona and the profile it runs as both have it |
|---|---|---|
| `name` | the persona's `name`; a profile alone: its `label` | the persona's; `warn` if the label differs |
| `agent` | the persona's or the profile's | **the profile's**: it is the one that starts today |
| `folder` | the persona's or the profile's, made whole | **the profile's** when set: likewise |
| `account`, `policy`, `[env]`, `[[mcp]]` | the profile | — |
| `settings` | the profile's; a file inside `profiles/` is copied into the agent's folder and named relatively | — |
| `avatar` | the persona's, else the profile's | the persona's |
| `INSTRUCTIONS.md` | the persona's `instructions`; a profile alone: its instructions file's text | the persona's: the profile's file is not read today; `warn` |
| `CHARACTER.md` | the persona's `character` | — |
| `role` | the persona's; unset on `conductor`: written as `orchestrator` | — |
| `fleets`, `model`, `effort`, `machine`, `plugins`, `can_message`, `pinned` | the persona | — |
| `memory.md` | `<data>/personas/<id>/memory.jsonl`: id, text, by, at; a note without an id gets one | — |
| `skills/` | `<data>/personas/<id>/skills/`, copied byte for byte | — |
| `routines.toml` | each routine's id, name, schedule, prompt, place, owner pause | — |
| `routine-runs.json` | each routine's `made_in`, `made_at`, `last_due`, `last_run`, a `ChatGone` pause | — |

Skipped with a `warn`, and still in the backup: a persona or profile file that doesn't parse (its notes and skills
still move to `agents/<id>/`, which lists once the owner adds an `agent.toml`); an owner profile marked `hidden`
(it must not become startable); a second profile of a name already taken; a file name that isn't an id. A persona
that runs as a plugin's profile gets a copy of that profile's launch keys, with a `warn`.

Ids don't change, so no thread or routine is rewritten: a thread's `profile` names the same agent after as before.

## Fixtures and tests (scope 6, AC 2 to 4)

```text
crates/cooee-core/tests/fixtures/upgrade/
├── 0.1.0/                    never edited: config.toml + threads/ (no agents: the plan is empty or routines only)
└── 0.1.0-personas/           the last layout before agents/, using every key and every case in the table above
    ├── config/               config.toml, personas/, profiles/ (with an instructions file and a settings file)
    ├── data/                 threads/, personas/<id>/{memory.jsonl, skills/}, routines.json
    └── expected/             agents/…, routines.toml, routine-runs.json: the migrated tree, byte for byte
```

A release folder is its own config and data folder unless it has `config/` and `data/`; the two existing tests
find their files through that one helper. A later layout change adds a folder; these are never edited.

| Test | Asserts |
|---|---|
| `upgrade_compat.rs`: every release migrates | on a copy: every old file is present under its backup name with the same hash; `agents/` and the routines files equal `expected/` where there is one; every value in an old TOML or JSONL file is found in the new agent, note or routine (the table's three "the profile's/persona's wins" cells aside); a second `plan` is `Done` with no steps and the tree's hashes don't move |
| `cooee/tests/agents.rs`: dry run (AC 3) | `cooee config migrate --dry-run` on the fixture: stdout is the plan's lines; the hash of every file in both trees is unchanged; no `agents/` |
| `cooee/tests/agents.rs`: by hand (AC 1) | a folder with `agent.toml` (`name`, `agent`) and `INSTRUCTIONS.md` is in `GET /personas`, and its chat's first prompt carries the instructions |
| `cooee/tests/agents.rs`: malformed (AC 4) | of two folders, one with a bad `agent.toml` holding a marker string: the other is listed; stderr names the bad file and never the marker |
| the existing persona, profile, skill, MCP, room, routine and machine tests | unchanged behaviour, their setup writing agent folders through one `support` helper |
