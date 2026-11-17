//! Config files as the owner keeps them (#360): versioned reads, [`Stale`] saves refused, TOML saves [`patch`]ed.

use std::{fmt::Write as _, fs, io, path::Path, time::UNIX_EPOCH};

use toml_edit::{DocumentMut, Item, Table};

/// A save refused: its files changed on disk since they were read.
#[derive(Debug, thiserror::Error)]
#[error("{0} changed on disk, reload")]
pub struct Stale(pub String);

/// The version of `files` as they are on disk: from each one's mtime, size and content hash, a missing one too.
#[must_use]
pub fn version(files: &[impl AsRef<Path>]) -> String {
    let mut all = String::new();
    for f in files {
        let meta = fs::metadata(f.as_ref()).ok();
        let since = meta
            .as_ref()
            .and_then(|m| m.modified().ok()?.duration_since(UNIX_EPOCH).ok());
        let hash = fs::read(f.as_ref())
            .map(|b| crate::sha256(&b))
            .unwrap_or_default();
        let (nanos, size) = (
            since.map_or(0, |d| d.as_nanos()),
            meta.map_or(0, |m| m.len()),
        );
        writeln!(all, "{nanos}:{size}:{hash}").ok();
    }
    crate::sha256(all.as_bytes()).chars().take(16).collect()
}

/// `text` with only the keys that differ between TOML `was` and `now` changed; every other line and comment stays.
///
/// # Errors
/// When `text`, `was` or `now` isn't TOML.
pub fn patch(text: &str, was: &str, now: &str) -> io::Result<String> {
    let bad = |e: String| io::Error::new(io::ErrorKind::InvalidData, e);
    let mut doc: DocumentMut = text.parse().map_err(|e| bad(format!("{e}")))?;
    let mut items: DocumentMut = now.parse().map_err(|e| bad(format!("{e}")))?;
    items.as_table_mut().iter_mut().for_each(|(_, i)| loose(i));
    let parse = |t: &str| toml::from_str::<toml::Table>(t).map_err(|e| bad(e.to_string()));
    let (was, now) = (parse(was)?, parse(now)?);
    keys(doc.as_table_mut(), &was, &now, items.as_table());
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
        let table = |v: Option<&toml::Value>| v.and_then(toml::Value::as_table).cloned();
        let tab = |v: Option<&toml::Value>| table(v).unwrap_or_default();
        let arr = |v: Option<&toml::Value>| v.and_then(toml::Value::as_array).cloned();
        match (doc.get_mut(k), item) {
            (Some(Item::Table(d)), Item::Table(i)) => keys(d, &tab(w), &tab(n), i),
            (Some(Item::ArrayOfTables(d)), Item::ArrayOfTables(i)) => {
                let (w, n) = (arr(w).unwrap_or_default(), arr(n).unwrap_or_default());
                for (at, item) in i.iter().enumerate() {
                    let row = |a: &[toml::Value]| tab(a.get(at));
                    match d.get_mut(at) {
                        Some(d) => keys(d, &row(&w), &row(&n), item),
                        None => d.push(item.clone()),
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
            _ => drop(doc.insert(k, item.clone())),
        }
    }
}
