//! An agent's skills (#218): `skills/<name>/SKILL.md` in its folder, front matter (`name`, `description`) and a
//! body. The brief lists their descriptions; a body is read on demand.

use std::{fmt::Write as _, fs, io};

use serde::{Deserialize, Serialize};

use crate::agents::Home;

/// The most skills an agent keeps.
pub const MAX: usize = 20;
/// The longest description, in characters.
pub const DESCRIPTION: usize = 120;

/// One skill.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct Skill {
    /// Its name, its folder's: lowercase letters, digits and `-`.
    pub name: String,
    /// One line on when to use it.
    pub description: String,
    /// The procedure.
    pub body: String,
}

fn refused(why: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, why)
}

fn dir(home: &Home) -> std::path::PathBuf {
    home.path().join("skills")
}

pub(crate) fn checked(name: &str) -> io::Result<&str> {
    let ok = !name.is_empty()
        && !name.starts_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    ok.then_some(name).ok_or_else(|| {
        refused(format!(
            "{name:?} isn't a skill name: lowercase letters, digits and -"
        ))
    })
}

impl Skill {
    /// Its file's text.
    #[must_use]
    pub fn file(&self) -> String {
        format!(
            "---\nname: {}\ndescription: {}\n---\n{}\n",
            self.name,
            self.description,
            self.body.trim_end()
        )
    }

    pub(crate) fn parse(name: &str, text: &str) -> Self {
        let (head, body) = text
            .strip_prefix("---\n")
            .and_then(|t| t.split_once("\n---\n"))
            .unwrap_or(("", text));
        let description = head.lines().find_map(|l| l.strip_prefix("description:"));
        Self {
            name: name.to_owned(),
            description: description.unwrap_or_default().trim().to_owned(),
            body: body.trim_end().to_owned(),
        }
    }
}

/// The agent's skills, by name.
#[must_use]
pub fn list(home: &Home) -> Vec<Skill> {
    let mut out: Vec<Skill> = fs::read_dir(dir(home))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            let text = fs::read_to_string(e.path().join("SKILL.md")).ok()?;
            checked(&name).ok()?;
            Some(Skill::parse(&name, &text))
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// The agent's skill `name`.
///
/// # Errors
/// A name that isn't one, or no such skill.
pub fn read(home: &Home, name: &str) -> io::Result<Skill> {
    let file = dir(home).join(checked(name)?).join("SKILL.md");
    let text = fs::read_to_string(file).map_err(|_| refused(format!("no skill {name}")))?;
    Ok(Skill::parse(name, &text))
}

/// Checks `s` as the agent's new skill, or the edit of its skill `replaces`; returns the file it replaces
/// (empty for a new one).
///
/// # Errors
/// A bad name, an empty or too long description, a name taken, no skill `replaces`, or a 21st skill.
pub fn check(home: &Home, s: &Skill, replaces: Option<&str>) -> io::Result<String> {
    checked(&s.name)?;
    let d = s.description.trim();
    if d.is_empty() || d.contains('\n') || d.chars().count() > DESCRIPTION {
        return Err(refused(format!(
            "a skill's description is one line of at most {DESCRIPTION} characters"
        )));
    }
    let all = list(home);
    let old = match replaces {
        Some(r) => read(home, r)?.file(),
        None => String::new(),
    };
    if replaces != Some(s.name.as_str()) && all.iter().any(|o| o.name == s.name) {
        return Err(refused(format!("there is a skill {} already", s.name)));
    }
    if replaces.is_none() && all.len() >= MAX {
        return Err(refused(format!("a persona keeps at most {MAX} skills")));
    }
    Ok(old)
}

/// Saves `s` as the agent's new skill, or as the edit of its skill `replaces` (which may rename it).
///
/// # Errors
/// As [`check`], or a file that can't be written.
pub fn save(home: &Home, s: Skill, replaces: Option<&str>) -> io::Result<Skill> {
    check(home, &s, replaces)?;
    let s = Skill {
        description: s.description.trim().to_owned(),
        body: s.body.trim_end().to_owned(),
        ..s
    };
    let folder = dir(home).join(&s.name);
    fs::create_dir_all(&folder)?;
    crate::write_private(&folder.join("SKILL.md"), s.file().as_bytes())?;
    if let Some(old) = replaces.filter(|r| *r != s.name) {
        remove(home, old)?;
    }
    Ok(s)
}

/// Deletes the agent's skill `name`.
///
/// # Errors
/// A bad name, no such skill, or a folder that can't be removed.
pub fn remove(home: &Home, name: &str) -> io::Result<()> {
    read(home, name)?;
    fs::remove_dir_all(dir(home).join(name))
}

/// `old` to `new` as lines: the common head and tail as `  `, the rest `- ` then `+ `.
#[must_use]
pub fn diff(old: &str, new: &str) -> String {
    let (a, b): (Vec<&str>, Vec<&str>) = (old.lines().collect(), new.lines().collect());
    let head = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    let (a_rest, b_rest) = (
        a.get(head..).unwrap_or_default(),
        b.get(head..).unwrap_or_default(),
    );
    let tail = a_rest
        .iter()
        .rev()
        .zip(b_rest.iter().rev())
        .take_while(|(x, y)| x == y)
        .count();
    let (a_mid, a_tail) = a_rest
        .split_at_checked(a_rest.len() - tail)
        .unwrap_or_default();
    let b_mid = b_rest.get(..b_rest.len() - tail).unwrap_or_default();
    let mut out = String::new();
    let lines = a.iter().take(head).map(|l| ("  ", l));
    let lines = lines
        .chain(a_mid.iter().map(|l| ("- ", l)))
        .chain(b_mid.iter().map(|l| ("+ ", l)))
        .chain(a_tail.iter().map(|l| ("  ", l)));
    for (mark, l) in lines {
        writeln!(out, "{mark}{l}").ok();
    }
    out
}
