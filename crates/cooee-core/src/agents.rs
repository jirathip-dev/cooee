//! Agents (#359): the owner's are folders `<config>/agents/<id>/`: `agent.toml` (every structured key),
//! `INSTRUCTIONS.md` and `CHARACTER.md` (read as-is), `memory.md` ([`crate::memory`]) and `skills/`
//! ([`crate::skills`]). A plugin's `[[profiles]]` are lent as agents: read-only, with no folder. Files are read on
//! each call, so a hand edit needs no restart.

use std::{
    collections::BTreeMap,
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
    time::SystemTime,
};

use serde::{Deserialize, Serialize};

use crate::{
    browser, disk, mcp, memory, plugins, profiles::Profile, provider::Builtin, routines::Schedule,
    skills,
};

const DROVER: &str = "drover";

/// Variables, `K=V`.
pub type Vars = Vec<(String, String)>;

/// What an agent is (#159), apart from its name: drover attaches by it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// A plain agent.
    #[default]
    #[serde(rename = "")]
    Plain,
    /// One of herdsman's orchestrators: an installed drover is attached to it.
    Orchestrator,
    /// cooee's own persona (#664): only the folder `chief` has it.
    Chief,
}

/// How an agent's native chat reads (#242).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    /// Only the final reply, its steps folded into one row; told to answer short and plain.
    #[default]
    Chat,
    /// Every step, as it happens.
    Detailed,
}

/// What a setup chat's brief adds (#270).
pub const SETUP: &str = "This is a setup chat. Interview the owner, one question at a time, about your purpose, \
tone, boundaries and way of working; then write your Character and Instructions with your update_persona tool. \
Here each update is saved at once.";

/// What a new session is told above its folder's `routine.toml` (#362).
pub const ROUTINE: &str = "Your routine (routine.toml): if the owner has no routine of yours yet, offer it once and \
make it with create_routine when they agree.";

/// What a [`Style::Chat`] agent's new session is told after its brief.
pub const PLAIN: &str =
    "Answer short, plain and conversational: summarise, and give the details only when asked.";

/// What a call's utterance (#663) is told before it, unless its persona's `spoken` says otherwise.
pub const SPOKEN: &str = "Spoken reply: answer in one to three short plain sentences, no markdown, lists or code; \
it is read aloud.";

/// The consolidation routine's mark on its runs (#219), and its watermark's key with the persona's id after a `-`.
pub const CONSOLIDATE: &str = "consolidate";

/// What a consolidation run is told (#219).
pub const CONSOLIDATION: &str = "This is your consolidation run. Call history once with no arguments: it gives what \
was said since the last run. Then propose changes to your notes with remember, preferring to replace or merge a note \
over adding one, and a skill with skill propose only for something done at least twice. Each is a card the owner \
approves; nothing is saved before. Stop after this one pass.";

/// A persona's consolidation routine (#219): off unless `on`; at `when` (default nightly, 03:00) a run reads its new
/// history and proposes notes and skills, on `model` and `effort` (empty: its provider's cheapest).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Consolidate {
    /// Switched on.
    pub on: bool,
    /// A 5-field cron in the Mac's local time (none: nightly).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<Schedule>,
    /// Its model (empty: the provider's cheapest).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub model: String,
    /// Its effort (empty: the provider's lowest).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub effort: String,
}

impl Consolidate {
    /// When it runs: `when`, else nightly at 03:00.
    #[must_use]
    pub fn schedule(&self) -> Option<Schedule> {
        self.when
            .clone()
            .or_else(|| Schedule::parse("0 3 * * *").ok())
    }
}

/// The heartbeat's mark on its runs (#498), and its watermark's key with the persona's id after a `-`.
pub const HEARTBEAT: &str = "heartbeat";

/// What a heartbeat run is told (#498), its notify rule and what is new after it.
pub const BEAT: &str = "This is your heartbeat run. Below is what is new in your threads and your routines' results \
since the last run; look at your attached connectors too where they bear on it. Decide in this one pass and act on \
nothing: anything you would do waits on a card the owner answers. End your reply with one last line: NOTIFY when \
what you found matches the owner's notify rule, else SILENT: and why.";

/// A persona's heartbeat (#498): off unless `on`; at `when` (default hourly) a run looks at what is new and stays
/// silent unless its notify rule matches, on `model` and `effort` (empty: its provider's cheapest), at most `per_day`
/// runs a local day (none: [`Heartbeat::PER_DAY`]).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Heartbeat {
    /// Switched on.
    pub on: bool,
    /// A 5-field cron in the Mac's local time (none: hourly).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<Schedule>,
    /// Its model (empty: the provider's cheapest).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub model: String,
    /// Its effort (empty: the provider's lowest).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub effort: String,
    /// At most this many runs a local day.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_day: Option<crate::routines::Cap>,
}

impl Heartbeat {
    /// The runs a local day when `per_day` is unset.
    pub const PER_DAY: crate::routines::Cap = crate::routines::Cap(4);

    /// When it runs: `when`, else hourly on the hour.
    #[must_use]
    pub fn schedule(&self) -> Option<Schedule> {
        self.when
            .clone()
            .or_else(|| Schedule::parse("0 * * * *").ok())
    }
}

/// The owner's notify rule for persona `home` (#498): `notify.md`, trimmed; empty until the owner writes one.
#[must_use]
pub fn notify(home: &Home) -> String {
    fs::read_to_string(home.path().join("notify.md"))
        .unwrap_or_default()
        .trim()
        .to_owned()
}

/// One agent: JSON on `/personas`; `agent.toml` is this without `id`, `character`, `instructions` and `version`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Agent {
    /// Its folder's name: lowercase letters, digits and `-`; a plugin's: `<plugin>/<name>`.
    pub id: String,
    /// Its name, for display only.
    pub name: String,
    /// Its role (#159).
    pub role: Role,
    /// How its chat reads (#242): `chat` unless set, `detailed` for an orchestrator.
    pub style: Style,
    /// The repos or fleets it covers (#159).
    pub fleets: Vec<String>,
    /// An image (`https://…`, `data:image/…`, a plugin's `/path`) or a theme's sprite id; none: its initials.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// `CHARACTER.md`: who it is, its tone.
    pub character: String,
    /// `INSTRUCTIONS.md`: its standing rules.
    pub instructions: String,
    /// An owned folder, or `scratch` (or empty): each chat's own.
    pub folder: String,
    /// The ACP agent (`shed agents`).
    pub agent: String,
    /// The default model (empty: the agent's).
    pub model: String,
    /// The default effort (empty: the agent's).
    pub effort: String,
    /// The machine it runs on (#148): one added to shed, empty: this host.
    pub machine: String,
    /// The installed plugins attached to it (#129).
    pub plugins: Vec<String>,
    /// The agents it may ask with `ask_persona` (#156).
    pub can_message: Vec<String>,
    /// Pinned to the native app's Home row (#139).
    pub pinned: bool,
    /// Its provider's account (none: the one the chat names).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    /// The ACP agent's settings file (its hooks are the agent's guard).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<PathBuf>,
    /// Variables for the agent: plain text (a `{thread}` in it is the thread's id), or `secret:<NAME>`.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// The MCP servers it picks from the registry (#56).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub mcp: Vec<mcp::Pick>,
    /// The sites its browser may open (#160).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sites: Vec<browser::Site>,
    /// shed's approval policy: `ask`, `allow` or `deny`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>,
    /// The agents its chats may continue on when this one hits a usage limit (#76), in the order offered.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fallback: Vec<Seat>,
    /// Its consolidation routine (#219); a save that leaves it out keeps it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub consolidate: Option<Consolidate>,
    /// Its heartbeat (#498); a save that leaves it out keeps it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heartbeat: Option<Heartbeat>,
    /// Its inbox (#220); set on the Mac, a save keeps it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mail: Option<crate::mail::Inbox>,
    /// The desk its `desk` tool drives (#172), by name; set on the Mac, a save keeps it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub desk: Option<String>,
    /// What a call's utterance is told before it (#663), in place of [`SPOKEN`]; empty: nothing. A save that leaves
    /// it out keeps it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spoken: Option<String>,
    /// Its files' [`disk::version`] when read; a save naming an older one is refused (#360), one naming none isn't.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub version: String,
    /// Set for a plugin's profile read as an agent: read-only, no folder.
    #[serde(skip)]
    pub lent: Option<Lent>,
}

/// The account that asks shed to pick the one with the most headroom and switch on a limit (#765).
pub const AUTO: &str = "auto";

/// The account that is the provider's own login.
pub const DEFAULT: &str = "default";

/// One agent and account of a fallback chain (#76).
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Seat {
    /// The ACP agent (`shed agents` id).
    pub agent: String,
    /// Its provider's account (none: the provider's default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
}

/// An agent and account a thread saw hit its usage limit (#76).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capped {
    /// The ACP agent.
    pub agent: String,
    /// Its account (empty: the system default).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub account: String,
    /// When it resets, as the harness said it (empty: not said).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub resets: String,
}

/// What only a plugin's agent has.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lent {
    /// The plugin.
    pub plugin: String,
    /// Why it isn't offered nor started, when it isn't.
    pub hidden: Option<String>,
    /// Threads its plugin lists as this agent's.
    pub threads: Vec<String>,
}

/// An owner agent's folder: only [`home`] makes one.
#[derive(Debug, Clone)]
pub struct Home(PathBuf);

impl Home {
    /// The folder.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }
}

/// One of its attached plugins' places in an agent's chats: a header-strip chip (q984) or its chat cards (#377).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Place {
    /// Where it goes.
    #[serde(rename = "type")]
    pub kind: plugins::PlaceKind,
    /// Its plugin.
    pub plugin: String,
    /// Its id among the plugin's places.
    pub id: String,
    /// What the chip says.
    pub label: String,
    /// Its screen's path on the plugin's backend.
    pub screen: String,
}

fn refused(why: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, why)
}

/// Whether `a` is an avatar: an image (`https://…`, `data:image/…`, or a plugin's `/path`) or a sprite id.
#[must_use]
pub fn avatar(a: &str) -> bool {
    a.starts_with("https://") || a.starts_with("data:image/") || a.starts_with('/') || is_id(a)
}

/// Whether `id` is lowercase letters, digits and `-`, not empty.
#[must_use]
pub fn is_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

/// An id made from `label`: its letters and digits, lowercased, other runs a `-`.
#[must_use]
pub fn slug(label: &str) -> String {
    let mut name = String::new();
    for c in label.trim().to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            name.push(c);
        } else if !name.is_empty() && !name.ends_with('-') {
            name.push('-');
        }
    }
    name.trim_end_matches('-').to_owned()
}

pub(crate) fn whole(p: &Path, base: &Path) -> PathBuf {
    match (p.strip_prefix("~"), crate::env_var("HOME")) {
        (Ok(rest), Some(home)) => PathBuf::from(home).join(rest),
        _ => base.join(p),
    }
}

fn dir(config: &Path, id: &str) -> PathBuf {
    config.join("agents").join(id)
}

/// Agent `id`'s folder.
///
/// # Errors
/// When `id` isn't an id or its folder has no `agent.toml` (a plugin's agent has none).
pub fn home(config: &Path, id: &str) -> io::Result<Home> {
    let d = dir(config, id);
    if is_id(id) && d.join("agent.toml").is_file() {
        Ok(Home(d))
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no agent {id}"),
        ))
    }
}

fn parse(file: &Path) -> Result<Agent, String> {
    let text = fs::read_to_string(file).map_err(|e| e.kind().to_string())?;
    let line = |e: toml::de::Error| {
        let at = e.span().map_or(0, |s| s.start);
        format!(
            "line {}: not a valid agent.toml",
            text[..at].matches('\n').count() + 1
        )
    };
    let a: Agent = toml::from_str(&text).map_err(line)?;
    let table: toml::Table = toml::from_str(&text).map_err(line)?;
    if let Some(k) = ["id", "character", "instructions", "version"]
        .into_iter()
        .find(|k| table.contains_key(*k))
    {
        return Err(format!(
            "`{k}` isn't a key: the id is the folder's name, the prose its Markdown files"
        ));
    }
    a.check()?;
    Ok(Agent {
        style: style(a.role, table.contains_key("style").then_some(a.style)),
        ..a
    })
}

pub(crate) fn style(role: Role, set: Option<Style>) -> Style {
    set.unwrap_or(if role == Role::Orchestrator {
        Style::Detailed
    } else {
        Style::Chat
    })
}

const FILES: [&str; 3] = ["agent.toml", "INSTRUCTIONS.md", "CHARACTER.md"];

fn load(d: &Path, id: &str) -> Result<Agent, String> {
    let a = parse(&d.join("agent.toml"))?;
    if a.role == Role::Chief && id != crate::chief::ID {
        return Err("only cooee's own Chief, folder chief, has role chief".to_owned());
    }
    let md = |n: &str| fs::read_to_string(d.join(n)).unwrap_or_default();
    let folder = match a.folder.as_str() {
        "" | "scratch" => a.folder.clone(),
        f => whole(Path::new(f), d).display().to_string(),
    };
    Ok(Agent {
        id: id.to_owned(),
        character: md("CHARACTER.md"),
        instructions: md("INSTRUCTIONS.md"),
        settings: a.settings.as_deref().map(|s| whole(s, d)),
        folder,
        version: disk::version(&FILES.map(|f| d.join(f))),
        ..a
    })
}

/// The owner's agents by id: each folder under `<config>/agents/` with an id for its name and an `agent.toml`. A
/// malformed one is left out and logged by its file and line or key, never its text.
#[must_use]
pub fn list(config: Option<&Path>, data: &Path) -> Vec<Agent> {
    let mut out = Vec::new();
    let root = config.map(|c| c.join("agents"));
    for d in root.iter().flat_map(fs::read_dir).flatten().flatten() {
        let (d, name) = (d.path(), d.file_name());
        let Some(id) = name.to_str().filter(|n| is_id(n)) else {
            continue;
        };
        if !d.join("agent.toml").is_file() {
            continue;
        }
        match load(&d, id) {
            Ok(a) => out.push(a),
            Err(e) => crate::note!("cooee: {}: {e}", d.join("agent.toml").display()),
        }
    }
    let lock = plugins::read_lock(data).unwrap_or_default();
    if lock.iter().any(|l| l.name == DROVER) {
        let orchestrators = out.iter_mut().filter(|a| a.role == Role::Orchestrator);
        for a in orchestrators.filter(|a| !a.plugins.iter().any(|n| n == DROVER)) {
            a.plugins.push(DROVER.to_owned());
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// What [`list`] reads, each path with its modified time and size: an equal stamp lists the same agents.
#[must_use]
pub fn stamp(config: Option<&Path>, data: &Path) -> Vec<(PathBuf, Option<(SystemTime, u64)>)> {
    let root = config.map(|c| c.join("agents"));
    let dirs = root.iter().flat_map(fs::read_dir).flatten().flatten();
    let files = dirs.flat_map(|d| FILES.map(|f| d.path().join(f)));
    let paths = root.clone().into_iter().chain(files);
    let seen = |p: PathBuf| {
        let m = fs::metadata(&p).ok();
        let m = m.and_then(|m| Some((m.modified().ok()?, m.len())));
        (p, m)
    };
    paths.chain([plugins::lock_path(data)]).map(seen).collect()
}

/// The agents to start from: the owner's, then each enabled installed plugin's profiles lent as agents
/// (`served(<plugin>)`'s list when its backend serves one, else its manifest's).
#[must_use]
pub fn all(
    config: Option<&Path>,
    data: &Path,
    served: impl Fn(&str) -> Option<Vec<Profile>>,
) -> Vec<Agent> {
    let mut out = list(config, data);
    for l in plugins::read_lock(data)
        .unwrap_or_default()
        .into_iter()
        .filter(|l| l.enabled)
    {
        let build = plugins::build_dir(data, &l.name, &l.commit);
        let profiles = if let Some(list) = served(&l.name) {
            list
        } else {
            let read = fs::read_to_string(build.join("cooee-plugin.toml"));
            let Ok(m) = read.and_then(|t| plugins::Manifest::parse(&t)) else {
                crate::note!("cooee: {}: no readable cooee-plugin.toml", build.display());
                continue;
            };
            m.profiles
        };
        for p in profiles {
            match p.lend(&l.name, &build) {
                Ok(a) => out.push(a),
                Err(e) => crate::note!("cooee: plugin {}: {e}", l.name),
            }
        }
    }
    out
}

/// The owner's agent `id`.
///
/// # Errors
/// When there is none.
pub fn find(config: Option<&Path>, data: &Path, id: &str) -> io::Result<Agent> {
    let found = list(config, data).into_iter().find(|a| a.id == id);
    found.ok_or_else(|| refused(format!("no persona {id}")))
}

fn card(a: &Agent) -> String {
    let card = Agent {
        id: String::new(),
        character: String::new(),
        instructions: String::new(),
        version: String::new(),
        ..a.clone()
    };
    crate::toml_text(&serde_json::to_value(card).unwrap_or_default())
}

pub(crate) fn files(a: &Agent) -> [(&'static str, Vec<u8>); 3] {
    [
        ("agent.toml", card(a).into_bytes()),
        ("INSTRUCTIONS.md", a.instructions.clone().into_bytes()),
        ("CHARACTER.md", a.character.clone().into_bytes()),
    ]
}

/// Saves `a`'s folder: a new one (`id` empty) gets an id made from its name, not taken, and no launch keys; an edit
/// keeps the launch keys, changes only the keys and files that differ, refused [`disk::Stale`] past `a.version`.
///
/// # Errors
/// No name or agent, a non-image avatar, a name taken, an unknown id, a stale version, or a file not written.
pub fn save(config: &Path, data: &Path, mut a: Agent) -> io::Result<Agent> {
    let (old, text) = if a.id.is_empty() {
        a.id = slug(&a.name);
        if a.id.is_empty() || dir(config, &a.id).exists() {
            return Err(refused(format!("{:?} needs a name of its own", a.name)));
        }
        (Agent::default(), String::new())
    } else {
        let d = home(config, &a.id)?.0;
        let files = FILES.map(|f| d.join(f));
        let [what, ..] = &files;
        if !a.version.is_empty() && a.version != disk::version(&files) {
            return Err(io::Error::other(disk::Stale(what.display().to_string())));
        }
        let old = parse(what).map_err(refused)?;
        let whole_folder = !matches!(old.folder.as_str(), "" | "scratch")
            && whole(Path::new(&old.folder), &d) == Path::new(&a.folder);
        if whole_folder {
            a.folder.clone_from(&old.folder);
        }
        (old, fs::read_to_string(what)?)
    };
    let set: toml::Table = toml::from_str(&text).map_err(io::Error::other)?;
    let was = Agent {
        style: style(a.role, set.contains_key("style").then_some(old.style)),
        ..old.clone()
    };
    a = Agent {
        account: old.account,
        settings: old.settings,
        env: old.env,
        mcp: old.mcp,
        sites: old.sites,
        policy: old.policy,
        fallback: old.fallback,
        mail: old.mail,
        desk: old.desk,
        consolidate: a.consolidate.or(old.consolidate),
        heartbeat: a.heartbeat.or(old.heartbeat),
        spoken: a.spoken.or(old.spoken),
        lent: None,
        ..a
    };
    if old.role == Role::Chief {
        a.role = Role::Chief;
    } else if a.role == Role::Chief {
        return Err(refused("only cooee's own Chief has that role".to_owned()));
    }
    a.check().map_err(|_| {
        refused("a persona needs a name, an agent, and an image or none as its avatar".to_owned())
    })?;
    let d = dir(config, &a.id);
    let toml = disk::patch(&text, &card(&was), &card(&a))?;
    let new = [&toml, &a.instructions, &a.character];
    for (file, text) in FILES.map(|f| d.join(f)).into_iter().zip(new) {
        if fs::read_to_string(&file).ok().as_ref() != Some(text) {
            crate::write_private(&file, text.as_bytes())?;
        }
    }
    find(Some(config), data, &a.id)
}

/// Replaces agent `id`'s `[[mcp]]` picks (#524: the phone's attach and detach), changing no other key.
///
/// # Errors
/// An unknown id, or an `agent.toml` that can't be read or written.
pub fn pick(config: &Path, data: &Path, id: &str, mcp: Vec<mcp::Pick>) -> io::Result<Agent> {
    let what = home(config, id)?.0.join("agent.toml");
    let text = fs::read_to_string(&what)?;
    let old = parse(&what).map_err(refused)?;
    let new = Agent { mcp, ..old.clone() };
    crate::write_private(
        &what,
        disk::patch(&text, &card(&old), &card(&new))?.as_bytes(),
    )?;
    find(Some(config), data, id)
}

/// Deletes agent `id`'s whole folder; its chats stay. Chief (#664) is never deleted.
///
/// # Errors
/// An unknown id, Chief, or a folder that can't be removed.
pub fn remove(config: &Path, id: &str) -> io::Result<()> {
    let h = home(config, id)?;
    if parse(&h.path().join("agent.toml")).is_ok_and(|a| a.role == Role::Chief) {
        return Err(refused(
            "Chief can't be deleted: rename it instead.".to_owned(),
        ));
    }
    fs::remove_dir_all(h.path())
}

/// Agent `id`'s chat places: the `header-strip` and `chat-cards` places of its plugins, in its order and theirs.
///
/// # Errors
/// When there is no such agent.
pub fn places(config: Option<&Path>, data: &Path, id: &str) -> io::Result<Vec<Place>> {
    let attached = find(config, data, id)?.plugins;
    let lock = plugins::read_lock(data).unwrap_or_default();
    let mut out = Vec::new();
    for l in attached
        .iter()
        .filter_map(|a| lock.iter().find(|l| &l.name == a))
    {
        let places = plugins::manifest(data, l)
            .map(|m| m.places)
            .unwrap_or_default();
        out.extend(
            places
                .into_iter()
                .filter(|p| p.kind != plugins::PlaceKind::MenuRow)
                .map(|p| Place {
                    kind: p.kind,
                    plugin: l.name.clone(),
                    id: p.id,
                    label: p.label,
                    screen: p.screen,
                }),
        );
    }
    Ok(out)
}

impl Agent {
    /// Checks its name, agent, avatar, policy and env (a value that looks like a token must be a `secret:`
    /// reference); an error names the key, never a value.
    ///
    /// # Errors
    /// What is wrong, for the owner.
    pub fn check(&self) -> Result<(), String> {
        if self.name.trim().is_empty() || self.agent.is_empty() {
            return Err("`name` and `agent` are needed".to_owned());
        }
        if self.avatar.as_deref().is_some_and(|a| !avatar(a)) {
            return Err("`avatar` isn't an image URL or a sprite id".to_owned());
        }
        if self
            .policy
            .as_deref()
            .is_some_and(|p| !["ask", "allow", "deny"].contains(&p))
        {
            return Err("`policy` isn't ask, allow or deny".to_owned());
        }
        for (k, v) in &self.env {
            let inline = match v.strip_prefix("secret:") {
                Some(name) => !mcp::var(name),
                None => crate::redact(v) != *v,
            };
            if !mcp::var(k) || inline {
                return Err(format!(
                    "env {k}: a variable name, and a secret as `secret:<NAME>`, never inline"
                ));
            }
        }
        Ok(())
    }

    /// Its env split into plain variables and secret ones, each secret read from [`crate::secrets::get`], and its
    /// persona's saved secrets (#163) among the secret ones.
    ///
    /// # Errors
    /// Names the secret that is missing or can't be read.
    pub fn env(&self, data: &Path) -> Result<(Vars, Vars), String> {
        let (mut plain, mut secret) = (Vec::new(), Vec::new());
        for (k, v) in &self.env {
            match v.strip_prefix("secret:") {
                Some(name) => {
                    let needs = format!("agent {} needs secret {name}", self.id);
                    let value = crate::secrets::get(data, name)
                        .map_err(|e| format!("{needs}: {e}"))?
                        .ok_or_else(|| format!("{needs}: it isn't saved"))?;
                    secret.push((k.clone(), value));
                }
                None => plain.push((k.clone(), v.clone())),
            }
        }
        let saved = crate::secrets::of_persona(data, &self.id)
            .map_err(|e| format!("agent {}'s saved secrets: {e}", self.id))?;
        secret.extend(saved);
        Ok((plain, secret))
    }

    /// The harness built-ins its sessions turn off (#503): every one cooee replaces; web once a pick provides it.
    #[must_use]
    pub fn off(&self, registry: &[mcp::Server]) -> Vec<Builtin> {
        let picked = |s: &&mcp::Server| s.enabled && self.mcp.iter().any(|p| p.server == s.name);
        let mut off = vec![
            Builtin::Browser,
            Builtin::Memory,
            Builtin::SubAgents,
            Builtin::Skills,
        ];
        let web = registry
            .iter()
            .filter(picked)
            .any(|s| s.provides.contains(&mcp::Provides::Web));
        off.extend(web.then_some(Builtin::Web));
        off
    }

    /// The folder its chats work in; none: each chat's own.
    #[must_use]
    pub fn work_folder(&self) -> Option<PathBuf> {
        (!matches!(self.folder.as_str(), "" | "scratch")).then(|| PathBuf::from(&self.folder))
    }

    /// What its agent gets ahead of a new session's first message: its Character, Instructions, notes, skills'
    /// descriptions (#218), never a skill's body, and, in [`Style::Chat`], [`PLAIN`].
    #[must_use]
    pub fn brief(&self, config: Option<&Path>) -> String {
        let home = config
            .filter(|_| self.lent.is_none())
            .and_then(|c| home(c, &self.id).ok());
        let mut notes = String::from("Your notes (edit them with your remember tool):");
        let mut skills = String::from("Your skills (read one with cooee's skill tool):");
        if let Some(h) = &home {
            for n in memory::notes(h) {
                write!(notes, "\n- [{}] {}", n.id, n.text).ok();
            }
            for s in skills::list(h).iter().take(skills::MAX) {
                write!(skills, "\n- {}: {}", s.name, s.description).ok();
            }
        }
        let routine = home
            .as_ref()
            .and_then(|h| fs::read_to_string(h.path().join("routine.toml")).ok())
            .map(|t| format!("{ROUTINE}\n```toml\n{}\n```", t.trim()))
            .unwrap_or_default();
        let plain = if self.style == Style::Chat { PLAIN } else { "" };
        let parts = [
            self.character.trim(),
            self.instructions.trim(),
            &notes,
            &skills,
            &routine,
            plain,
        ];
        let parts = parts
            .into_iter()
            .filter(|s| !s.is_empty() && !s.ends_with(':'));
        parts.collect::<Vec<_>>().join("\n\n")
    }
}
