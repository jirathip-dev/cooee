# Module boundaries and statecharts (#359)

## Who calls whom

```text
 phone / web ──► cooee-server  lib.rs routes (/personas…, POST /agents, /profiles/{id}/mcp), recipe()
 agent ─MCP──►   cooee-server  hub.rs (remember, skill, history, ask), hub/rooms.rs, hub/models.rs
                    │  login.rs  Logins::agent(&Agent)  (was Logins::profile)
                    ▼
                 cooee-core  agents.rs ──► memory.rs, skills.rs   (all three: only under <config>/agents/<id>/)
                    │            └──► profiles.rs (a plugin's shape, lent as an Agent), plugins.rs (lock, manifest)
                    │         routines.rs ──► <config>/routines.toml, <data>/routine-runs.json
                    ▼
                 <config>/agents/<id>/…

 cooee (binary)  serve / `config migrate` ──► cooee-core migrate.rs ──► agents, memory, routines (their writers)
                                                   └──► the old paths: read, then renamed
 cooee-web       renders agents::Agent (the picker, the MCP page); no file access
```

| Module | May call | Never |
|---|---|---|
| `cooee-core::agents` | `memory`, `skills`, `profiles`, `plugins` (lock, manifests), `mcp::Pick`, `secrets` (an env's references) | `<data>/personas`, threads, sockets |
| `cooee-core::memory`, `skills` | the files under the `Home` they are given | building a path from a data folder or an id |
| `cooee-core::profiles` | nothing but serde | the owner's files: a `Profile` is only ever a plugin's |
| `cooee-core::routines` | its two files, `cron` | the hub, the clock |
| `cooee-core::migrate` | `agents`, `memory`, `routines`, `profiles` (the old profile file is that shape) | being called from the server or the web; deleting anything |
| `cooee-server` | `agents`, `memory`, `skills`, `routines` through their functions | joining `agents/…` paths itself; `migrate` |
| `cooee` (binary) | `migrate::plan` and `Plan::apply`, once, before the hub opens | reading the old layout itself |
| `cooee-web` | — (renders `Agent`) | the server |

Enforcers: `scripts/dep-direction.sh` (crate direction, unchanged); `Home`'s private field (a data path can't reach
`memory` or `skills`); `Agent`'s `deny_unknown_fields` (the API and the file refuse stray keys); the upgrade test
(only `migrate.rs` can make the old fixtures readable); rev checks that `"personas"` and `"profiles"` as path
segments appear in `migrate.rs` alone.

Deleted, not kept beside the new: `personas.rs`; `profiles::{list, find, file, create}`; `mcp::save_picks`; the
"an owner profile with no persona is read as one" rule; the "`conductor` is an orchestrator when `role` is unset"
rule (the migration writes the role out); `Persona.profile`.

## Statechart: a folder under `agents/`

```text
                         name is a slug and agent.toml exists?
                 no ─────────────┴───────────── yes
                 │                               │
          Not an agent                     read agent.toml
     (left alone, not logged)        ┌───────────┴────────────┐
                                  parses, rules hold      bad TOML, unknown key,
                                     │                    no name/agent, inline secret
                                     ▼                           ▼
                                  Listed ◄── owner fixes ──── Hidden (this agent only; one log line:
                                     │        the file         the file and line or key, never its text)
              save (API) / copy / set_mcp: agent.toml (and the two .md) rewritten whole
              remember: memory.md rewritten      skill save: skills/<name>/SKILL.md
                                     │
                               DELETE /personas/{id}
                                     ▼
                                   gone (the whole folder; its chats stay)
```

- The files are read on each call, as today: a hand edit or a `git pull` shows without a restart.
- A plugin's agent is never in this chart: it has no folder, so `agents::home` refuses it and it has no notes or
  skills, as today.

The migration's statechart is in [`migration.md`](migration.md).
