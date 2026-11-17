# Data shapes (#359)

`<config>` is the config file's folder (`~/.config/cooee/<instance>/`, or the folder of `--config`'s file).

## An agent's folder: `<config>/agents/<id>/`

`<id>` is the folder's name: lowercase letters, digits and `-`. A folder with another name, or without
`agent.toml`, is not an agent and is left alone (so `.git`, a `README.md` or a draft can sit beside the agents).

### `agent.toml`

Every key but `name` and `agent` is optional; an unknown key makes the file malformed.

```toml
name = "Kai"                      # shown; never the id
agent = "fake-acp"                # the ACP agent (`shed agents`)
role = "orchestrator"             # or absent: a plain agent
fleets = ["cooee"]
avatar = "https://example.com/kai.png"
model = ""                        # empty: the ACP agent's default
effort = ""
folder = "~/Projects/site"        # an owned folder; absent or "scratch": each chat's own
machine = ""                      # empty: this host
plugins = ["drover"]
can_message = ["mia"]
pinned = true

# The launch keys: edited by hand (and `[[mcp]]` by Settings › MCP servers); the API never sets them.
account = "work"
settings = "guard.json"           # the ACP agent's settings file
policy = "ask"                    # ask | allow | deny

[env]
REGION = "ap-southeast-1"
GH_TOKEN = "secret:GH_TOKEN"      # a secret is a reference, never inline (as today: the agent is hidden otherwise)

[[mcp]]
server = "github"
tools = ["list_issues"]
```

- `folder` and `settings`: `~/x` is under `$HOME`; a relative path is under the agent's folder. They are made whole
  when read; a save keeps the launch keys exactly as the file has them, so a relative `settings` stays relative.
- `id`, `character` and `instructions` are **not** keys: they are the folder's name and the two Markdown files. A
  file that sets one is malformed, so prose has one home.
- A malformed file (bad TOML, an unknown key, a broken rule) hides that agent only. The log line is
  `cooee: <path>/agent.toml: line <n>` for a parse error, or names the key for a broken rule; never a value or
  the file's text (AC 4).

### `INSTRUCTIONS.md`, `CHARACTER.md`

Plain Markdown, read as-is; a missing file is empty. A save writes both.

### `memory.md`

One note per line:

```markdown
- [3f] The owner prefers summaries under five lines <!-- by:persona at:1731283200000 -->
- [a01c] Deploys go out on Thursdays<br>never on a Friday <!-- by:owner at:1731369600000 -->
- a line the owner typed by hand
```

```text
line    = "- " [ "[" id "] " ] text [ " <!-- by:" who " at:" ms " -->" ]
id      = 2 to 16 of 0-9 a-f        (so "- [x] done" is text, not an id)
text    = the note; a line break inside it is written "<br>"
trailer = the last such comment on the line; absent: by "owner", at 0
```

- A bullet without an id gets one on the first read, written back (as a note from before #216 does today).
- Any other line is not a note; it is not kept when cooee next rewrites the file. One note is one line.
- The cap stays 2,000 characters of text over all notes; ids and trailers don't count.

### `skills/<name>/SKILL.md`

Unchanged: front matter (`name`, `description`), then the body; at most 20 skills, descriptions of one line and at
most 120 characters.

## `<config>/routines.toml` and `<data>/routine-runs.json`

```toml
[[routine]]
id = "r-1731283200000"            # any id of lowercase letters, digits and `-`; never reused
name = "Monday renewals"
when = "0 9 * * 1"
prompt = "Which renewals are due {{due}}?"
paused = true                     # only when the owner paused it

[routine.place]
in = "chat"                       # or "new_thread" with [routine.place.who]: agent, account, profile, folder,
thread = "t-1"                    # model, effort, machine (as today's `Who`)
```

```json
{ "format": 1,
  "runs": { "r-1731283200000": { "made_in": "t-1", "made_at": 1731283200000, "last_due": 1731921600000,
                                 "last_run": { "at": 1731921601000, "key": "routine-r-…", "outcome": "ran", "thread": "t-1" },
                                 "chat_gone": "t-1" } } }
```

A routine with no entry in `runs` (one typed by hand) gets `made_at = now` at the next tick, so it first runs at its
next due time, not at once. An entry whose routine is gone is dropped at the next save.

## Rust types (`cooee-core`)

```rust
// agents.rs
/// One agent. JSON on the wire (`/personas`); `agent.toml` is this without `id`, `character` and `instructions`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Agent {
    pub id: String,                       // the folder's name; a plugin's: "<plugin>/<name>"
    pub name: String,
    pub role: Role,                       // Plain | Orchestrator, as today
    pub fleets: Vec<String>,
    pub avatar: Option<String>,
    pub character: String,                // CHARACTER.md
    pub instructions: String,             // INSTRUCTIONS.md
    pub folder: String,
    pub agent: String,
    pub model: String,
    pub effort: String,
    pub machine: String,
    pub plugins: Vec<String>,
    pub can_message: Vec<String>,
    pub pinned: bool,
    // Launch keys: `save` keeps the folder's; skipped in JSON when empty.
    pub account: Option<String>,
    pub settings: Option<PathBuf>,
    pub env: BTreeMap<String, String>,
    pub mcp: Vec<mcp::Pick>,
    pub policy: Option<String>,
    /// Set for a plugin's profile read as an agent: read-only, no folder.
    #[serde(skip)]
    pub lent: Option<Lent>,
}

/// What only a plugin's agent has.
pub struct Lent { pub plugin: String, pub hidden: Option<String> /* why */, pub threads: Vec<String> }

/// An owner agent's folder. Its field is private: only `agents::home` makes one.
pub struct Home(PathBuf);

pub fn list(config: Option<&Path>, data: &Path) -> Vec<Agent>;            // the owner's, by id
pub fn all(config: Option<&Path>, data: &Path, served: impl Fn(&str) -> Option<Vec<Profile>>) -> Vec<Agent>;
pub fn find(config: Option<&Path>, data: &Path, id: &str) -> io::Result<Agent>;
pub fn home(config: &Path, id: &str) -> io::Result<Home>;
pub fn save(config: &Path, data: &Path, a: Agent) -> io::Result<Agent>;   // new (id empty) or edit
pub fn copy(config: &Path, data: &Path, name: &str, base: Option<&Agent>, acp: &str) -> io::Result<Agent>;
pub fn set_mcp(config: &Path, data: &Path, id: &str, picks: Vec<mcp::Pick>) -> io::Result<()>;
pub fn remove(config: &Path, id: &str) -> io::Result<()>;                 // the whole folder; its chats stay
impl Agent {
    pub fn check(&self) -> Result<(), String>;                // name, agent, avatar, policy, env: names the key
    pub fn brief(&self, config: Option<&Path>) -> String;     // CHARACTER, INSTRUCTIONS, notes, skills' descriptions
    pub fn work_folder(&self) -> Option<PathBuf>;             // none: each chat's own
}

// memory.rs: Note {id, text, by, at} and Edit {text, replaces, forget} unchanged
pub fn notes(home: &Home) -> Vec<Note>;
pub fn remember(home: &Home, by: &str, e: &Edit, at: u64) -> io::Result<Option<Note>>;
pub fn forget(home: &Home) -> io::Result<()>;

// profiles.rs: only what a plugin's `[[profiles]]` (or its backend's list) parses into
pub struct Profile { /* today's keys, unchanged */ }
impl Profile { pub fn lend(self, plugin: &str, build: &Path) -> Result<Agent, String>; }

// routines.rs: `Routine` stays the joined value the runner and the web use; `Routines::new(config, data)`.
// migrate.rs: see migration.md.
```

`agent.toml` and `routines.toml` are written whole by the existing idiom (a JSON string is a TOML basic string),
atomically (`write_private`); no TOML-writing dependency is added.

## The wire (unchanged names)

`/personas…` carries `Agent` as JSON: today's persona keys, minus `profile`, plus the launch keys when set
(`account`, `settings`, `env`, `mcp`, `policy`; read-only: a `PUT` that omits or changes them keeps the folder's).
`Thread.profile`, `Who.profile`, `Row.persona*`, `POST /agents` and `POST /profiles/{id}/mcp` keep their names.
