//! Chief (#664): cooee's own persona, the folder `agents/chief/` with role chief. What it sets on a persona with no
//! card ([`PERSONA_KEYS`]), and the changes that wait on the owner's Face ID ([`Carded`]), each made with its card.

use std::{
    collections::BTreeMap,
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    Config,
    agents::{self, Agent, Role},
    disk,
    mcp::{self, Pick, Server},
    skills,
};

/// Chief's folder name and id.
pub const ID: &str = "chief";

const CHARACTER: &str = include_str!("chief/CHARACTER.md");
const INSTRUCTIONS: &str = include_str!("chief/INSTRUCTIONS.md");

fn refused(why: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, why.into())
}

/// Chief, when there is one: the persona in folder `chief` whose role is chief.
#[must_use]
pub fn find(config: Option<&Path>, data: &Path) -> Option<Agent> {
    agents::find(config, data, ID)
        .ok()
        .filter(|a| a.role == Role::Chief)
}

/// Makes Chief's folder with `agent` and `machine` (`here`: this host), pinned to Home; an existing Chief is
/// returned as it is.
///
/// # Errors
/// A folder `chief` that is the owner's own persona (rename it), or one that can't be written.
pub fn create(config: &Path, data: &Path, agent: &str, machine: &str) -> io::Result<Agent> {
    if let Some(a) = find(Some(config), data) {
        return Ok(a);
    }
    let d = config.join("agents").join(ID);
    if d.exists() {
        return Err(refused(format!(
            "{} is a persona of yours: rename its folder, then run cooee setup again",
            d.display()
        )));
    }
    let a = Agent {
        name: "Chief".to_owned(),
        role: Role::Chief,
        agent: agent.to_owned(),
        machine: if machine == "here" { "" } else { machine }.to_owned(),
        pinned: true,
        character: CHARACTER.to_owned(),
        instructions: INSTRUCTIONS.to_owned(),
        ..Agent::default()
    };
    for (file, bytes) in agents::files(&a) {
        crate::write_private(&d.join(file), &bytes)?;
    }
    find(Some(config), data).ok_or_else(|| io::Error::other("Chief's folder was not read back"))
}

fn archive_dir(config: &Path) -> PathBuf {
    config.join("agents").join(".archive")
}

/// Moves persona `id`'s folder to `agents/.archive/<id>`, where no list reads it; its chats stay.
///
/// # Errors
/// `IS_CHIEF` for Chief, `NO_PERSONA` for no such persona, one archived under that id already, or a failed move.
pub fn archive(config: &Path, id: &str) -> io::Result<()> {
    if id == ID {
        return Err(refused("IS_CHIEF: Chief is never archived"));
    }
    let from = agents::home(config, id).map_err(|_| refused(format!("NO_PERSONA: {id}")))?;
    let to = archive_dir(config).join(id);
    if to.exists() {
        return Err(refused(format!("an archived {id} is there already")));
    }
    fs::create_dir_all(archive_dir(config))?;
    fs::rename(from.path(), to)
}

/// Brings archived persona `id` back.
///
/// # Errors
/// `NO_PERSONA` for none archived, a persona of that id made since, or a failed move.
pub fn restore(config: &Path, id: &str) -> io::Result<()> {
    let from = archive_dir(config).join(id);
    if !agents::is_id(id) || !from.join("agent.toml").is_file() {
        return Err(refused(format!("NO_PERSONA: no archived {id}")));
    }
    let to = config.join("agents").join(id);
    if to.exists() {
        return Err(refused(format!("{id} is taken by another persona now")));
    }
    fs::rename(from, to)
}

/// The archived personas' ids.
#[must_use]
pub fn archived(config: &Path) -> Vec<String> {
    let mut out: Vec<String> = fs::read_dir(archive_dir(config))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .filter(|n| agents::is_id(n))
        .collect();
    out.sort();
    out
}

/// The keys Chief sets on a persona with no card: none starts a program, names a secret or picks a machine.
pub const PERSONA_KEYS: &str =
    "name character instructions avatar agent model effort plugins can_message";

/// What Chief asks to change in a persona's access, each given replaced whole: carded.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Access {
    /// Its variables: plain text, or `secret:<NAME>`.
    pub env: Option<BTreeMap<String, String>>,
    /// Its MCP picks.
    pub mcp: Option<Vec<Pick>>,
    /// Its machine (empty: this host).
    pub machine: Option<String>,
}

/// A change that waits on the owner's card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// Remove MCP server `.0`.
    McpRemove(String),
    /// Turn MCP server `.0` on or off.
    McpSwitch(String, bool),
    /// Save secret `.0`, its value typed into the card.
    SecretSet(String),
    /// Remove secret `.0`.
    SecretRemove(String),
    /// Write `text` to `path`, unless the file changed since `version`.
    File {
        /// The file.
        path: PathBuf,
        /// Its [`disk::version`] when the card was made.
        version: String,
        /// Its new text, checked.
        text: String,
    },
    /// `cooee machine check <.0> --yes`.
    MachineSetup(String),
}

/// A change with the card that shows it: its title and detail are made from the value that is applied, and only its
/// constructors make one.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Carded {
    /// What is applied.
    pub change: Change,
    /// The card's title.
    pub title: String,
    /// The card's exact change.
    pub detail: String,
}

#[derive(Serialize)]
struct Launch<'a> {
    #[serde(skip_serializing_if = "str::is_empty")]
    machine: &'a str,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    env: &'a BTreeMap<String, String>,
    #[serde(skip_serializing_if = "<[Pick]>::is_empty")]
    mcp: &'a [Pick],
}

fn server<'a>(list: &'a [Server], name: &str) -> Result<&'a Server, String> {
    let s = list.iter().find(|s| s.name == name);
    s.ok_or_else(|| format!("NO_SERVER: no MCP server {name}"))
}

fn text<T: Serialize>(v: &T) -> Result<String, String> {
    serde_json::to_value(v)
        .map(|v| crate::toml_text(&v))
        .map_err(|e| e.to_string())
}

impl Carded {
    /// Removes MCP server `name` of `list`, picked by none of `personas`.
    ///
    /// # Errors
    /// `NO_SERVER`, or `IN_USE` naming a persona that picks it.
    pub fn mcp_remove(list: &[Server], personas: &[Agent], name: &str) -> Result<Self, String> {
        let s = server(list, name)?;
        let using = personas
            .iter()
            .find(|p| p.mcp.iter().any(|k| k.server == name));
        if let Some(p) = using {
            return Err(format!(
                "IN_USE: {} picks {name}: change its access first",
                p.id
            ));
        }
        Ok(Self {
            change: Change::McpRemove(name.to_owned()),
            title: format!("remove MCP server {name}"),
            detail: s.shown(),
        })
    }

    /// Turns MCP server `name` of `list` on or off.
    ///
    /// # Errors
    /// `NO_SERVER`, or one that is so already.
    pub fn mcp_switch(list: &[Server], name: &str, on: bool) -> Result<Self, String> {
        let s = server(list, name)?;
        let word = if on { "on" } else { "off" };
        if s.enabled == on {
            return Err(format!("MCP server {name} is {word} already"));
        }
        Ok(Self {
            change: Change::McpSwitch(name.to_owned(), on),
            title: format!("turn {word} MCP server {name}"),
            detail: format!(
                "[[mcp]] {name}\n-enabled = {}\n+enabled = {on}\n{}",
                !on,
                s.shown()
            ),
        })
    }

    /// Saves secret `name`; the owner types its value into the card.
    ///
    /// # Errors
    /// `BAD_NAME` for a name that is no variable's.
    pub fn secret_set(name: &str, why: &str) -> Result<Self, String> {
        if !mcp::var(name) {
            return Err("BAD_NAME: name the secret like a variable: FLIGHTS_KEY".to_owned());
        }
        let why: String = why.trim().chars().take(300).collect();
        Ok(Self {
            change: Change::SecretSet(name.to_owned()),
            title: format!("save secret {name} in the keychain: {why}"),
            detail: name.to_owned(),
        })
    }

    /// Removes secret `name`, one of `names`.
    ///
    /// # Errors
    /// `NO_SECRET` for one not saved.
    pub fn secret_remove(names: &[String], name: &str) -> Result<Self, String> {
        if !names.iter().any(|n| n == name) {
            return Err(format!("NO_SECRET: no secret {name}"));
        }
        Ok(Self {
            change: Change::SecretRemove(name.to_owned()),
            title: format!("remove secret {name}"),
            detail: name.to_owned(),
        })
    }

    fn file(
        path: &Path,
        title: String,
        f: impl FnOnce(&str) -> Result<String, String>,
    ) -> Result<Self, String> {
        let old = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let version = disk::version(&[path]);
        let new = f(&old)?;
        if new == old {
            return Err("nothing to change".to_owned());
        }
        Ok(Self {
            detail: skills::diff(&old, &new),
            change: Change::File {
                path: path.to_owned(),
                version,
                text: new,
            },
            title,
        })
    }

    /// Sets persona `old`'s variables, MCP picks (of `servers`) and machine, each given; the card is
    /// `agent.toml`'s diff.
    ///
    /// # Errors
    /// `IS_CHIEF`, an MCP pick of no server, or variables [`Agent::check`] refuses.
    pub fn access(
        config: &Path,
        old: &Agent,
        ask: Access,
        servers: &[Server],
    ) -> Result<Self, String> {
        let new = Agent {
            env: ask.env.unwrap_or_else(|| old.env.clone()),
            mcp: ask.mcp.unwrap_or_else(|| old.mcp.clone()),
            machine: ask.machine.unwrap_or_else(|| old.machine.clone()),
            ..old.clone()
        };
        if let Some(p) = new.mcp.iter().find(|p| server(servers, &p.server).is_err()) {
            return Err(format!("NO_SERVER: no MCP server {}", p.server));
        }
        new.check()?;
        let launch = |a: &'_ Agent| {
            text(&Launch {
                machine: &a.machine,
                env: &a.env,
                mcp: &a.mcp,
            })
        };
        let (was, now) = (launch(old)?, launch(&new)?);
        let path = agents::home(config, &old.id)
            .map_err(|e| e.to_string())?
            .path()
            .join("agent.toml");
        let title = format!("change {}'s variables, MCP servers or machine", old.id);
        Self::file(&path, title, |t| {
            disk::patch(t, &was, &now).map_err(|e| e.to_string())
        })
    }

    /// Sets up `machine`: `failed` are its checks that fail now, each fixed by `cooee machine check --yes`.
    ///
    /// # Errors
    /// When nothing fails.
    pub fn machine_setup(machine: &str, failed: &[String]) -> Result<Self, String> {
        if failed.is_empty() {
            return Err(format!("{machine} is ready: nothing to set up"));
        }
        Ok(Self {
            change: Change::MachineSetup(machine.to_owned()),
            title: format!("set up machine {machine}"),
            detail: format!(
                "cooee machine check {machine} --yes clones each missing folder and copies the rules there:\n{}",
                failed.join("\n")
            ),
        })
    }

    /// Sets the keys of `set` and removes those of `unset` in config file `path`; it applies at cooee's next start.
    ///
    /// # Errors
    /// `BAD_KEY` for `mcp` or `plugins`, `BAD_CONFIG` for a file cooee would not load.
    pub fn config(
        path: &Path,
        set: &serde_json::Map<String, serde_json::Value>,
        unset: &[String],
    ) -> Result<Self, String> {
        let set: toml::Table =
            serde_json::from_value(set.clone().into()).map_err(|e| format!("BAD_CONFIG: {e}"))?;
        let keys: Vec<&str> = set
            .keys()
            .map(String::as_str)
            .chain(unset.iter().map(String::as_str))
            .collect();
        if let Some(k) = keys.iter().find(|k| ["mcp", "plugins"].contains(k)) {
            return Err(format!("BAD_KEY: {k} has its own tool, or holds tokens"));
        }
        let title = format!(
            "change config.toml: {} (applies at cooee's next start)",
            keys.join(", ")
        );
        Self::file(path, title, |t| {
            let all: toml::Table = toml::from_str(t).map_err(|e| format!("BAD_CONFIG: {e}"))?;
            let was: toml::Table = all
                .into_iter()
                .filter(|(k, _)| keys.contains(&k.as_str()))
                .collect();
            let mut now = was.clone();
            now.extend(set.clone());
            now.retain(|k, _| !unset.iter().any(|u| u == k));
            let new = disk::patch(t, &text(&was)?, &text(&now)?).map_err(|e| e.to_string())?;
            toml::from_str::<Config>(&new).map_err(|e| format!("BAD_CONFIG: {}", e.message()))?;
            Ok(new)
        })
    }
}
