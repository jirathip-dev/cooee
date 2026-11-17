//! Config files as the owner keeps them (#360): a read hands out a [`version`] of its files, a save naming an
//! older one is refused with [`Stale`], and a TOML save through [`patch`] changes only the keys it touches.

use std::{fmt::Write as _, fs, io, path::Path, time::UNIX_EPOCH};

use toml_edit::{DocumentMut, Item, Table};

/// A save refused: its files changed on disk since they were read.
#[derive(Debug, thiserror::Error)]
#[error("{0} changed on disk, reload")]
pub struct Stale(pub String);

/// Whether `e` is a [`Stale`] refusal.
#[must_use]
pub fn stale(e: &io::Error) -> bool {
    e.get_ref()
        .is_some_and(<dyn std::error::Error + Send + Sync>::is::<Stale>)
}

/// The version of `files` as they are on disk: from each one's mtime, size and content hash, a missing one too.
#[must_use]
pub fn version(files: &[&Path]) -> String {
    let mut all = String::new();
    for f in files {
        let meta = fs::metadata(f).ok();
        let modified = meta.as_ref().and_then(|m| m.modified().ok());
        let since = modified.and_then(|t| t.duration_since(UNIX_EPOCH).ok());
        let size = meta.map_or(0, |m| m.len());
        let hash = fs::read(f).map(|b| crate::sha256(&b)).unwrap_or_default();
        let nanos = since.map_or(0, |d| d.as_nanos());
        writeln!(all, "{nanos}:{size}:{hash}").ok();
    }
    crate::sha256(all.as_bytes()).chars().take(16).collect()
}

/// Refuses with [`Stale`], naming `what`, when the version `seen` (empty: none named) isn't `files`' now.
///
/// # Errors
/// The [`Stale`] refusal.
pub fn check(files: &[&Path], seen: &str, what: &Path) -> io::Result<()> {
    if seen.is_empty() || seen == version(files) {
        Ok(())
    } else {
        Err(io::Error::other(Stale(what.display().to_string())))
    }
}

/// `text` with only the keys that differ between TOML `was` and `now` changed (set, added or removed), each
/// table and array of tables patched key by key; every other line, comment and blank line stays.
///
/// # Errors
/// When `text`, `was` or `now` isn't TOML.
pub fn patch(text: &str, was: &str, now: &str) -> io::Result<String> {
    let bad = |e: String| io::Error::new(io::ErrorKind::InvalidData, e);
    let mut doc: DocumentMut = text.parse().map_err(|e| bad(format!("{e}")))?;
    let mut items: DocumentMut = now.parse().map_err(|e| bad(format!("{e}")))?;
    for (_, item) in items.as_table_mut().iter_mut() {
        loose(item);
    }
    let parse = |t: &str| toml::from_str::<toml::Table>(t).map_err(|e| bad(e.to_string()));
    keys(
        doc.as_table_mut(),
        &parse(was)?,
        &parse(now)?,
        items.as_table(),
    );
    Ok(doc.to_string())
}

fn loose(item: &mut Item) {
    let tables: Vec<&mut Table> = match item {
        Item::Table(t) => vec![t],
        Item::ArrayOfTables(a) => a.iter_mut().collect(),
        _ => Vec::new(),
    };
    for t in tables {
        t.set_position(None);
        for (_, i) in t.iter_mut() {
            loose(i);
        }
    }
}

fn keys(doc: &mut Table, was: &toml::Table, now: &toml::Table, items: &Table) {
    for k in was.keys().filter(|k| !now.contains_key(*k)) {
        doc.remove(k);
    }
    for (k, item) in items {
        let (w, n) = (was.get(k), now.get(k));
        if w == n {
            continue;
        }
        let tables = |v: Option<&toml::Value>| v.and_then(toml::Value::as_table).cloned();
        let arrays = |v: Option<&toml::Value>| v.and_then(toml::Value::as_array).cloned();
        match (doc.get_mut(k), item) {
            (Some(Item::Table(d)), Item::Table(i)) => {
                keys(
                    d,
                    &tables(w).unwrap_or_default(),
                    &tables(n).unwrap_or_default(),
                    i,
                );
            }
            (Some(Item::ArrayOfTables(d)), Item::ArrayOfTables(i)) => {
                let (w, n) = (arrays(w).unwrap_or_default(), arrays(n).unwrap_or_default());
                for (at, t) in i.iter().enumerate() {
                    let row = |a: &[toml::Value]| tables(a.get(at)).unwrap_or_default();
                    match d.get_mut(at) {
                        Some(d) => keys(d, &row(&w), &row(&n), t),
                        None => d.push(t.clone()),
                    }
                }
                while d.len() > i.len() {
                    d.remove(d.len() - 1);
                }
            }
            (Some(Item::Value(d)), Item::Value(i)) => {
                let decor = d.decor().clone();
                *d = i.clone();
                *d.decor_mut() = decor;
            }
            _ => {
                doc.insert(k, item.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_patch_changes_only_the_keys_that_differ() {
        let text = "# mine\nname = 'A' # kept\n\nmodel = \"x\" # why\n\n[env]\n# a var\nA = \"1\"\n\n\
                    [[r]]\nid = \"1\"\n# note\nn = 1\n\n[[r]]\nid = \"2\"\nn = 2\n";
        let was = "name = \"A\"\nmodel = \"x\"\n[env]\nA = \"1\"\n[[r]]\nid = \"1\"\nn = 1\n[[r]]\nid = \"2\"\nn = 2\n";
        let now = "name = \"A\"\nmodel = \"y\"\npinned = true\n[env]\nA = \"1\"\nB = \"2\"\n[[r]]\nid = \"1\"\nn = 3\n";
        let out = patch(text, was, now).unwrap();
        assert_eq!(
            out,
            "# mine\nname = 'A' # kept\n\nmodel = \"y\" # why\npinned = true\n\n[env]\n# a var\nA = \"1\"\nB = \"2\"\n\n\
             [[r]]\nid = \"1\"\n# note\nn = 3\n"
        );
        assert_eq!(patch(&out, now, now).unwrap(), out);
    }
}
