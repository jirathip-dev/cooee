# The callers, as they will read (#359)

Written before the types: each block is how the code that uses agents reads after the change.

## 1. The owner writes an agent by hand (AC 1)

```text
~/.config/cooee/cooee/agents/kai/agent.toml        name = "Kai"
                                                   agent = "fake-acp"
~/.config/cooee/cooee/agents/kai/INSTRUCTIONS.md   Answer in one paragraph.
```

No restart: the next `GET /personas` lists `kai`, and `POST /threads {"profile": "kai", …}` starts its chat.

## 2. Starting a thread: one path instead of persona-then-profile

```rust
// cooee-server lib.rs. `list` is agents::all(): the owner's folders, then each plugin's profiles read as agents.
fn recipe(logins: &Logins, (data, config, list): (&Path, Option<&Path>, &[agents::Agent]),
          (agent, account, id): (&str, Option<&str>, &str), folder: PathBuf, mut picked: (String, String),
) -> Result<(String, PathBuf, Recipe), HubError> {
    if id.is_empty() {                                    // Plain: the owner's own setup, as today
        let (account, launch) = logins.home(agent, account)?;
        return finish(agent.to_owned(), folder, Recipe { account, launch, ..Recipe::default() }, picked);
    }
    let a = list.iter().find(|a| a.id == id).ok_or_else(|| refused(format!("no agent {id}")))?;
    if let Some(why) = a.lent.as_ref().and_then(|l| l.hidden.as_ref()) {
        return Err(refused(format!("agent {id} is hidden: {why}")));
    }
    for (pick, default) in [(&mut picked.0, &a.model), (&mut picked.1, &a.effort)] {
        if pick.is_empty() { pick.clone_from(default); }
    }
    let (account, launch) = logins.agent(a, account, data)?;   // was Logins::profile: account, settings, env, policy
    let recipe = Recipe {
        account, launch,
        profile: a.id.clone(),                             // the thread's `profile` key keeps its name
        instructions: a.brief(config),                     // CHARACTER, INSTRUCTIONS, notes, skills' descriptions
        mcp: a.mcp.iter().map(|k| k.server.clone()).collect(),
        machine: a.machine.clone(),
        ..Recipe::default()
    };
    finish(a.agent.clone(), a.work_folder().unwrap_or(folder), recipe, picked)
}
```

`Persona.profile`, `personas::find` followed by `profiles::find`, and "the profile's agent wins" are gone: an agent
has one `agent`, one `folder`, one set of launch keys.

## 3. The API routes keep their paths; the body is the agent

```rust
#[route(GET "/personas")]
async fn persona_list(cx: &Cx) -> Result<Answer<Vec<agents::Agent>>> {
    on_agents(cx, |d, c| Ok(agents::list(Some(c), d))).await          // the owner's folders only, by id
}

#[route(PUT "/personas/{id}")]
async fn persona_put(cx: &Cx, Json(a): Json<agents::Agent>) -> Result<Answer<agents::Agent>> {
    let a = agents::Agent { id: thread_id(cx), ..a };
    persona_save(cx, a).await       // agents::save: the card keys from the body, the launch keys from the folder
}

// `+ new agent` (web form): a copy of `base` (any agent in the picker, a plugin's too) or Plain on `acp`.
agents::copy(&config, &data, &form.name, base.as_ref(), &acp)?;

// Settings › MCP servers: an agent's picks.
agents::set_mcp(&config, &data, &id, picks)?;
```

## 4. Notes and skills take the agent's folder, never the data folder

```rust
// cooee-server lib.rs
#[route(GET "/personas/{id}/memory")]
async fn memory(cx: &Cx) -> Result<Answer<Vec<memory::Note>>> {
    let id = thread_id(cx);
    on_agents(cx, move |_, c| Ok(memory::notes(&agents::home(c, &id)?))).await
}

// hub.rs: the agent's own `remember` tool
let home = agents::home(self.config_dir().ok_or_else(no_config)?, &meta.profile)
    .map_err(|_| HubError::Refused(format!("thread {id} has no agent folder")))?;   // a plugin's has none
let kept = memory::remember(&home, "persona", e, now()).map_err(refused)?;
let used = memory::used(&memory::notes(&home));

skills::list(&home);  skills::read(&home, name)?;  skills::save(&home, skill, replaces)?;
```

`Home` can only be made by `agents::home` (a slug id whose `agent.toml` reads), so a path outside
`<config>/agents/<id>/` can't be passed.

## 5. Routines: the runner is unchanged, the store opens two files

```rust
// cooee-server routines.rs
file: Routines::new(config_dir, data),     // <config>/routines.toml + <data>/routine-runs.json
// load() joins them into the same Vec<Routine>; save() writes each file only when its part changed.
```

## 6. The migration: one call at the start of `serve`, one command

```rust
// crates/cooee/src/main.rs
fn serve(config: &Config) -> Result<(), String> {
    migrate(config, false);            // before the hub opens; never stops the start
    …
}

/// `cooee config migrate [--dry-run]`, and `serve`'s first step.
fn migrate(config: &Config, dry_run: bool) {
    let (Some(dir), Some(data)) = (config.dir.as_deref(), config.data_dir()) else { return };
    let plan = cooee_core::migrate::plan(dir, &data, today());
    if dry_run {
        plan.lines().for_each(|l| println!("{l}"));              // the same lines a real run logs
    } else if let Err(e) = plan.apply(|line| eprintln!("cooee: migrate: {line}")) {
        eprintln!("cooee: migrate: {e}; the old files are untouched; `cooee config migrate` tries again");
    }
}
```

```text
$ cooee config migrate --dry-run
write  agents/kai/agent.toml            from personas/kai.toml, profiles/kai.toml
write  agents/kai/INSTRUCTIONS.md       from personas/kai.toml
write  agents/kai/CHARACTER.md          from personas/kai.toml
write  agents/kai/memory.md             from <data>/personas/kai/memory.jsonl (12 notes)
copy   agents/kai/skills/release/SKILL.md  from <data>/personas/kai/skills/release/SKILL.md
copy   agents/kai/guard.json            from profiles/guard.json
write  routines.toml                    from <data>/routines.json (2 routines)
write  <data>/routine-runs.json         from <data>/routines.json
warn   agents/kai: `agent` differs in personas/kai.toml and profiles/kai.toml; the profile's is kept
backup personas -> personas.bak-2026-11-14
backup profiles -> profiles.bak-2026-11-14
backup <data>/personas -> <data>/personas.bak-2026-11-14
backup <data>/routines.json -> <data>/routines.json.bak-2026-11-14
3 agents, 2 routines; nothing is deleted
```

A line names files and keys, never a value or a file's text.
