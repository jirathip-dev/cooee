//! Personas (#126): a named agent (who it is, its standing rules, its folder and default agent) with notes it keeps,
//! for the native app. The owner's are `<config>/personas/<id>.toml`; an owner profile with no persona of its name is
//! read as one that runs as that profile (its account, settings, env, MCP servers and policy), so nothing is lost.
//! Its notes are `<data>/personas/<id>/memory.jsonl`.

use std::{
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{plugins, profiles};

/// drover's installed name (`plugins/index.toml`).
const DROVER: &str = "cooee-drover";

/// One persona.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Persona {
    /// Its id, its file's name (never in the file): lowercase letters, digits and `-`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub id: String,
    /// Its name.
    pub name: String,
    /// An image (as a profile's avatar); none: the client draws its initials.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// Character: who it is, its tone.
    pub character: String,
    /// Instructions: its standing rules.
    pub instructions: String,
    /// An owned folder, or `scratch` (or empty): each chat's own.
    pub folder: String,
    /// The default ACP agent.
    pub agent: String,
    /// The default model (empty: the agent's).
    pub model: String,
    /// The default effort (empty: the agent's).
    pub effort: String,
    /// The installed plugins attached to it (#129): their cards go only to its chats.
    pub plugins: Vec<String>,
    /// The owner profile it runs as (one read as a persona), never set over the API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}

/// One of its attached plugins' screens in a persona's header (#129, as #125 draws them).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Place {
    /// Its plugin.
    pub name: String,
    /// What the header shows.
    pub label: String,
    /// Its icon.
    pub icon: String,
    /// Its screen's URL.
    pub url: String,
}

/// One note in a persona's memory.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Note {
    /// The note.
    pub text: String,
    /// Who added it: `persona` or `owner`.
    pub by: String,
    /// When, in Unix milliseconds.
    pub at: u64,
}

fn refused(why: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, why)
}

/// Its notes' file.
fn memory(data: &Path, id: &str) -> PathBuf {
    data.join("personas").join(id).join("memory.jsonl")
}

/// The personas by id: `<config>/personas/*.toml`, and each owner profile with none of its name read as one. One that
/// can't be read is left out and logged by its file, never its text.
#[must_use]
pub fn list(config: Option<&Path>, data: &Path) -> Vec<Persona> {
    let mut out = Vec::new();
    let dir = config.map(|c| c.join("personas"));
    for file in dir.iter().flat_map(fs::read_dir).flatten().flatten() {
        let file = file.path();
        let id = file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if file.extension().is_none_or(|e| e != "toml") || profiles::slug(id) != id {
            continue;
        }
        let read = fs::read_to_string(&file).map_err(|e| e.to_string());
        match read.and_then(|t| toml::from_str::<Persona>(&t).map_err(|e| e.message().to_owned())) {
            Ok(p) => out.push(Persona {
                id: id.to_owned(),
                ..p
            }),
            Err(e) => eprintln!("cooee: {}: {e}", file.display()),
        }
    }
    // Migrated (#129): an installed drover goes with the conductor profile's persona.
    let lock = plugins::read_lock(data).unwrap_or_default();
    let drover: Vec<String> = lock
        .into_iter()
        .map(|l| l.name)
        .filter(|n| n == DROVER)
        .collect();
    for p in profiles::list(config, data, |_| None) {
        if p.plugin.is_none() && out.iter().all(|o| o.id != p.name) {
            let text = p.instructions.and_then(|f| fs::read_to_string(f).ok());
            out.push(Persona {
                id: p.name.clone(),
                name: p.label,
                avatar: p.avatar,
                instructions: text.unwrap_or_default(),
                folder: p
                    .folder
                    .map(|f| f.display().to_string())
                    .unwrap_or_default(),
                agent: p.agent,
                plugins: if p.name == "conductor" {
                    drover.clone()
                } else {
                    Vec::new()
                },
                profile: Some(p.name),
                ..Persona::default()
            });
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// Persona `id`.
///
/// # Errors
/// When there is none.
pub fn find(config: Option<&Path>, data: &Path, id: &str) -> io::Result<Persona> {
    let all = if id.is_empty() {
        Vec::new()
    } else {
        list(config, data)
    };
    let p = all.into_iter().find(|p| p.id == id);
    p.ok_or_else(|| refused(format!("no persona {id}")))
}

/// Saves `p` as `<config>/personas/<id>.toml`: a new one (`id` empty) gets an id made from its name, not taken; an
/// edit keeps the profile it runs as. Returns it.
///
/// # Errors
/// No name or agent, an avatar that isn't an image, a name taken, an unknown id, or a file that can't be written.
pub fn save(config: &Path, data: &Path, mut p: Persona) -> io::Result<Persona> {
    if p.id.is_empty() {
        p.id = profiles::slug(&p.name);
        if p.id.is_empty() || find(Some(config), data, &p.id).is_ok() {
            return Err(refused(format!("{:?} needs a name of its own", p.name)));
        }
        p.profile = None;
    } else {
        p.profile = find(Some(config), data, &p.id)?.profile;
    }
    if p.name.trim().is_empty()
        || p.agent.is_empty()
        || p.avatar.as_deref().is_some_and(|a| !profiles::avatar(a))
    {
        return Err(refused(
            "a persona needs a name, an agent, and an image or none as its avatar".to_owned(),
        ));
    }
    // TOML's basic strings and arrays read JSON's; every field is a string or a list of them.
    let fields = serde_json::to_value(Persona {
        id: String::new(),
        ..p.clone()
    })?;
    let mut text = String::new();
    for (k, v) in fields.as_object().into_iter().flatten() {
        let _ = writeln!(text, "{k} = {v}");
    }
    fs::create_dir_all(config.join("personas"))?;
    fs::write(config.join("personas").join(format!("{}.toml", p.id)), text)?;
    Ok(p)
}

/// Deletes persona `id`: its file, the owner profile it runs as and its notes; its chats stay.
///
/// # Errors
/// An unknown id, or a file that can't be removed.
pub fn remove(config: &Path, data: &Path, id: &str) -> io::Result<()> {
    let profile = find(Some(config), data, id)?
        .profile
        .and_then(|n| profiles::file(config, &n));
    for f in profile
        .into_iter()
        .chain([config.join("personas").join(format!("{id}.toml"))])
    {
        gone(fs::remove_file(f))?;
    }
    forget(data, id)
}

/// `r`, a missing file being no error.
fn gone(r: io::Result<()>) -> io::Result<()> {
    match r {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// Persona `id`'s notes, oldest first.
#[must_use]
pub fn notes(data: &Path, id: &str) -> Vec<Note> {
    let text = fs::read_to_string(memory(data, id)).unwrap_or_default();
    text.lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

/// Adds a note `by` (`persona` or `owner`) at `at` to persona `id`'s memory.
///
/// # Errors
/// An empty note, or a file that can't be written.
pub fn remember(data: &Path, id: &str, by: &str, text: &str, at: u64) -> io::Result<Note> {
    let (text, by) = (text.trim().to_owned(), by.to_owned());
    if text.is_empty() {
        return Err(refused("a note needs text".to_owned()));
    }
    let file = memory(data, id);
    fs::create_dir_all(file.parent().unwrap_or(data))?;
    let note = Note { text, by, at };
    crate::append_line(&file, &serde_json::to_string(&note)?)?;
    Ok(note)
}

/// Clears persona `id`'s notes, and only them.
///
/// # Errors
/// A folder that can't be removed.
pub fn forget(data: &Path, id: &str) -> io::Result<()> {
    gone(fs::remove_dir_all(data.join("personas").join(id)))
}

impl Persona {
    /// What its agent gets ahead of a new session's first message: its Character, Instructions and notes.
    #[must_use]
    pub fn brief(&self, data: &Path) -> String {
        let mut notes = String::from("Your notes (keep one with cooee's remember tool):");
        for n in self::notes(data, &self.id) {
            let _ = write!(notes, "\n- {} ({})", n.text, n.by);
        }
        let parts = [self.character.trim(), self.instructions.trim(), &notes];
        let parts = parts
            .into_iter()
            .filter(|s| !s.is_empty() && !s.ends_with(':'));
        parts.collect::<Vec<_>>().join("\n\n")
    }
}
