//! Personas (#126): a named agent (who it is, its standing rules, its folder and default agent) with notes it keeps,
//! for the native app. The owner's are `<config>/personas/<id>.toml`; an owner profile with no persona of its name is
//! read as one that runs as that profile (its account, settings, env, MCP servers and policy), so nothing is lost.
//! Its notes are `<data>/personas/<id>/memory.jsonl`, its skills (#218) under `<data>/personas/<id>/skills/`.

use std::{
    fmt::Write as _,
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{plugins, profiles, skills};

/// drover's installed name (`plugins/index.toml`).
const DROVER: &str = "cooee-drover";

/// What a persona is (#159), apart from its name: drover attaches by it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// A plain persona.
    #[default]
    #[serde(rename = "")]
    Plain,
    /// One of herdsman's orchestrators: an installed drover is attached to it.
    Orchestrator,
}

/// How a persona's native chat reads (#242).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Style {
    /// Only the final reply, its steps folded into one row; told to answer short and plain.
    #[default]
    Chat,
    /// Every step, as it happens.
    Detailed,
}

/// What a [`Style::Chat`] persona's new session is told after its brief.
pub const PLAIN: &str =
    "Answer short, plain and conversational: summarise, and give the details only when asked.";

/// One persona.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Persona {
    /// Its id, its file's name (never in the file): lowercase letters, digits and `-`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub id: String,
    /// Its name, for display only.
    pub name: String,
    /// Its role (#159).
    pub role: Role,
    /// How its chat reads (#242): `chat` unless set, `detailed` for an orchestrator.
    pub style: Style,
    /// The repos or fleets it covers (#159); another persona may cover them too.
    pub fleets: Vec<String>,
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
    /// The machine it runs on (#148): one added to shed (`shed machine ls`), empty: this host.
    pub machine: String,
    /// The installed plugins attached to it (#129): their cards go only to its chats.
    pub plugins: Vec<String>,
    /// The personas it may ask with `ask_persona` (#156, G4).
    pub can_message: Vec<String>,
    /// Pinned to the native app's Home row (#139).
    pub pinned: bool,
    /// The owner profile it runs as (one read as a persona), never set over the API.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
}

/// One of its attached plugins' chips on a persona's header strip (#125's `header-strip`, q984).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Place {
    /// Its plugin.
    pub plugin: String,
    /// Its id among the plugin's places.
    pub id: String,
    /// What the chip says.
    pub label: String,
    /// Its screen's path on the plugin's backend (`/p/<plugin>` + it through cooee).
    pub screen: String,
}

/// The most characters a persona's notes hold in all (#216).
pub const CAP: usize = 2000;

/// One note in a persona's memory.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct Note {
    /// Its short id, shown in the brief; a note kept before #216 gets one on first read.
    #[serde(default)]
    pub id: String,
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
        match read.and_then(|t| from_file(id, &t).map_err(|e| e.message().to_owned())) {
            Ok(p) => out.push(p),
            Err(e) => eprintln!("cooee: {}: {e}", file.display()),
        }
    }
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
                role: role(&p.name, None),
                style: style(role(&p.name, None), None),
                profile: Some(p.name),
                ..Persona::default()
            });
        }
    }
    let lock = plugins::read_lock(data).unwrap_or_default();
    if lock.iter().any(|l| l.name == DROVER) {
        let orchestrators = out.iter_mut().filter(|p| p.role == Role::Orchestrator);
        for p in orchestrators.filter(|p| !p.plugins.iter().any(|n| n == DROVER)) {
            p.plugins.push(DROVER.to_owned());
        }
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out
}

/// Persona `id` from its file's `text`.
fn from_file(id: &str, text: &str) -> Result<Persona, toml::de::Error> {
    let table: toml::Table = toml::from_str(text)?;
    let set = table.contains_key("role");
    let styled = table.contains_key("style");
    let p: Persona = toml::Value::Table(table).try_into()?;
    let role = role(id, set.then_some(p.role));
    Ok(Persona {
        id: id.to_owned(),
        role,
        style: style(role, styled.then_some(p.style)),
        ..p
    })
}

/// The style a persona has, else (saved before #242) its role's: an orchestrator's chat is detailed.
fn style(role: Role, set: Option<Style>) -> Style {
    set.unwrap_or(if role == Role::Orchestrator {
        Style::Detailed
    } else {
        Style::Chat
    })
}

/// The role of the persona `id` (or read from profile `id`): the one it has, else (saved before #159) the conductor's
/// is an orchestrator.
fn role(id: &str, set: Option<Role>) -> Role {
    set.unwrap_or(if id == "conductor" {
        Role::Orchestrator
    } else {
        Role::Plain
    })
}

/// Persona `id`'s header strip: the `header-strip` places of the plugins attached to it, in its order and theirs.
///
/// # Errors
/// When there is no such persona.
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
                .filter(|p| p.kind == plugins::PlaceKind::HeaderStrip)
                .map(|p| Place {
                    plugin: l.name.clone(),
                    id: p.id,
                    label: p.label,
                    screen: p.screen,
                }),
        );
    }
    Ok(out)
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
    gone(fs::remove_dir_all(data.join("personas").join(id)))
}

/// `r`, a missing file being no error.
fn gone(r: io::Result<()>) -> io::Result<()> {
    match r {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

/// A `remember` (#216): drop the notes in `forget`, then add `text`, or put it in place of note `replaces`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Edit {
    /// The note's text.
    pub text: Option<String>,
    /// The id of the note it replaces.
    pub replaces: Option<String>,
    /// The ids of the notes to drop.
    pub forget: Vec<String>,
}

/// Persona `id`'s notes, oldest first; any without an id get one, kept in the file.
#[must_use]
pub fn notes(data: &Path, id: &str) -> Vec<Note> {
    let text = fs::read_to_string(memory(data, id)).unwrap_or_default();
    let mut notes: Vec<Note> = text
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    if notes.iter().any(|n| n.id.is_empty()) {
        for i in 0..notes.len() {
            if notes[i].id.is_empty() {
                notes[i].id = fresh(&notes, &notes[i]);
            }
        }
        if let Err(e) = write(data, id, &notes) {
            eprintln!("cooee: persona {id}: notes: {e}");
        }
    }
    notes
}

fn fresh(notes: &[Note], n: &Note) -> String {
    let mut h = DefaultHasher::new();
    (n.text.as_str(), n.at).hash(&mut h);
    let hex = format!("{:016x}", h.finish());
    let id = (2..hex.len())
        .map(|k| &hex[..k])
        .find(|c| notes.iter().all(|o| o.id != *c));
    id.unwrap_or(&hex).to_owned()
}

/// The characters `notes` use against [`CAP`].
#[must_use]
pub fn used(notes: &[Note]) -> usize {
    notes.iter().map(|n| n.text.chars().count()).sum()
}

fn write(data: &Path, id: &str, notes: &[Note]) -> io::Result<()> {
    let mut lines = String::new();
    for n in notes {
        lines.push_str(&serde_json::to_string(n)?);
        lines.push('\n');
    }
    crate::write_private(&memory(data, id), lines.as_bytes())
}

/// Applies `e` `by` (`persona` or `owner`) at `at` to persona `id`'s notes: the note added or replaced (an exact
/// duplicate is the note already there), none when it only forgets.
///
/// # Errors
/// An unknown note id, no text where one is needed, notes that would grow past [`CAP`] (the refusal names the use
/// and the ids), or a file that can't be written.
pub fn remember(data: &Path, id: &str, by: &str, e: &Edit, at: u64) -> io::Result<Option<Note>> {
    let mut notes = notes(data, id);
    let before = used(&notes);
    let ids: Vec<&str> = notes.iter().map(|n| n.id.as_str()).take(100).collect();
    let ids = ids.join(", ");
    let mut named = e.forget.iter().chain(&e.replaces);
    if let Some(gone) = named.find(|f| notes.iter().all(|n| n.id != **f)) {
        return Err(refused(format!("no note {gone}; the notes are: {ids}")));
    }
    notes.retain(|n| !e.forget.contains(&n.id));
    let (text, by) = (e.text.as_deref().unwrap_or_default().trim(), by.to_owned());
    let kept = match (text, &e.replaces) {
        ("", None) if !e.forget.is_empty() => None,
        ("", _) => return Err(refused("a note needs text".to_owned())),
        (text, Some(r)) => {
            let n = notes.iter_mut().find(|n| n.id == *r);
            let n = n.ok_or_else(|| refused(format!("note {r} is forgotten too")))?;
            (n.text, n.by, n.at) = (text.to_owned(), by, at);
            Some(n.clone())
        }
        (text, None) => {
            let duplicate = notes.iter().find(|n| n.text == text).cloned();
            duplicate.or_else(|| {
                let mut n = Note {
                    id: String::new(),
                    text: text.to_owned(),
                    by,
                    at,
                };
                n.id = fresh(&notes, &n);
                notes.push(n.clone());
                Some(n)
            })
        }
    };
    let after = used(&notes);
    if after > CAP && after > before {
        return Err(refused(format!(
            "memory full: {before} of {CAP} characters used, this makes {after}; merge or forget notes first ({ids})"
        )));
    }
    write(data, id, &notes)?;
    Ok(kept)
}

/// Clears persona `id`'s notes, and only them: its skills stay.
///
/// # Errors
/// A file that can't be removed.
pub fn forget(data: &Path, id: &str) -> io::Result<()> {
    gone(fs::remove_file(memory(data, id)))
}

impl Persona {
    /// What its agent gets ahead of a new session's first message: its Character, Instructions, notes, skills'
    /// descriptions (#218), by name, never a skill's body, and, in [`Style::Chat`], [`PLAIN`].
    #[must_use]
    pub fn brief(&self, data: &Path) -> String {
        let mut notes = String::from("Your notes (edit them with your remember tool):");
        for n in self::notes(data, &self.id) {
            let _ = write!(notes, "\n- [{}] {}", n.id, n.text);
        }
        let mut skills = String::from("Your skills (read one with cooee's skill tool):");
        for s in skills::list(data, &self.id).iter().take(skills::MAX) {
            let _ = write!(skills, "\n- {}: {}", s.name, s.description);
        }
        let plain = if self.style == Style::Chat { PLAIN } else { "" };
        let parts = [
            self.character.trim(),
            self.instructions.trim(),
            &notes,
            &skills,
            plain,
        ];
        let parts = parts
            .into_iter()
            .filter(|s| !s.is_empty() && !s.ends_with(':'));
        parts.collect::<Vec<_>>().join("\n\n")
    }
}
