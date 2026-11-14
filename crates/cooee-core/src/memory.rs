//! An agent's notes (#126, #216): `memory.md` in its folder, one `- [<id>] <text> <!-- by:<who> at:<ms> -->` line per
//! note, a line break inside a note written `<br>`. A bullet without an id gets one on the first read; any other line
//! is not a note.

use std::{
    fmt::Write as _,
    fs,
    hash::{DefaultHasher, Hash, Hasher},
    io,
};

use serde::{Deserialize, Serialize};

use crate::agents::Home;

/// The most characters an agent's notes hold in all (#216).
pub const CAP: usize = 2000;

/// One note.
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

fn refused(why: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, why)
}

/// The note on line `line`, if it is one (its id empty when it has none yet).
fn parse(line: &str) -> Option<Note> {
    let mut rest = line.strip_prefix("- ")?;
    let (mut by, mut at) = ("owner".to_owned(), 0);
    if let Some((text, trailer)) = rest.rsplit_once(" <!-- by:")
        && let Some((who, ms)) = trailer
            .strip_suffix(" -->")
            .and_then(|t| t.split_once(" at:"))
        && let Ok(ms) = ms.parse()
    {
        (rest, by, at) = (text, who.to_owned(), ms);
    }
    let hex = |id: &str| {
        (2..=16).contains(&id.len())
            && id
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    };
    let (id, text) = match rest.strip_prefix('[').and_then(|r| r.split_once("] ")) {
        Some((id, text)) if hex(id) => (id.to_owned(), text),
        _ => (String::new(), rest),
    };
    Some(Note {
        id,
        text: text.replace("<br>", "\n"),
        by,
        at,
    })
}

/// `notes` as `memory.md`'s text; any without an id get one.
pub(crate) fn render(mut notes: Vec<Note>) -> (Vec<Note>, String) {
    for i in 0..notes.len() {
        if notes[i].id.is_empty() {
            notes[i].id = fresh(&notes, &notes[i]);
        }
    }
    let mut text = String::new();
    for n in &notes {
        let body = n.text.replace("\r\n", "\n").replace('\n', "<br>");
        let _ = writeln!(text, "- [{}] {body} <!-- by:{} at:{} -->", n.id, n.by, n.at);
    }
    (notes, text)
}

/// The agent's notes, oldest first; any without an id get one, kept in the file.
#[must_use]
pub fn notes(home: &Home) -> Vec<Note> {
    let text = fs::read_to_string(home.path().join("memory.md")).unwrap_or_default();
    let notes: Vec<Note> = text.lines().filter_map(parse).collect();
    if notes.iter().all(|n| !n.id.is_empty()) {
        return notes;
    }
    let (notes, _) = render(notes);
    if let Err(e) = write(home, &notes) {
        eprintln!("cooee: {}: notes: {e}", home.path().display());
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

fn write(home: &Home, notes: &[Note]) -> io::Result<()> {
    let (_, text) = render(notes.to_vec());
    crate::write_private(&home.path().join("memory.md"), text.as_bytes())
}

/// Applies `e` `by` (`persona` or `owner`) at `at` to the agent's notes: the note added or replaced (an exact
/// duplicate is the note already there), none when it only forgets.
///
/// # Errors
/// An unknown note id, no text where one is needed, notes that would grow past [`CAP`] (the refusal names the use
/// and the ids), or a file that can't be written.
pub fn remember(home: &Home, by: &str, e: &Edit, at: u64) -> io::Result<Option<Note>> {
    let mut notes = notes(home);
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
    write(home, &notes)?;
    Ok(kept)
}

/// Clears the agent's notes, and only them: its skills stay.
///
/// # Errors
/// A file that can't be removed.
pub fn forget(home: &Home) -> io::Result<()> {
    match fs::remove_file(home.path().join("memory.md")) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_reads_back_as_the_note_it_was_written_from() {
        let n = Note {
            id: "3f".to_owned(),
            text: "two\nlines <!-- not a trailer".to_owned(),
            by: "persona".to_owned(),
            at: 1_731_283_200_000,
        };
        let (_, text) = render(vec![n.clone()]);
        assert_eq!(text.lines().count(), 1, "{text}");
        assert_eq!(parse(text.trim_end()), Some(n));
        let typed = parse("- [x] done").unwrap();
        assert_eq!(
            (
                typed.id.as_str(),
                typed.text.as_str(),
                typed.by.as_str(),
                typed.at
            ),
            ("", "[x] done", "owner", 0)
        );
        assert_eq!(parse("not a bullet"), None);
    }
}
