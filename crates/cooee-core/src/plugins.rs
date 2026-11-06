//! Installed plugins (docs/plugins.md): a plugin is a git repo with a `cooee-plugin.toml` at its root. cooee keeps
//! it in `<data>/plugins/<name>/`, pins it in `<data>/plugins.lock` and writes its tabs as `[[plugins]]` entries in
//! `<data>/plugins/plugins.toml`, read beside the config's own. The curated index, `plugins/index.toml`, is built in;
//! its `[[theme]]` entries are the theme packs (docs/themes.md).

use std::{
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::Plugin;

/// `cooee-plugin.toml`: what a plugin is, builds with and runs.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Its name: lowercase letters, digits and `-`; its folder and lock entry.
    pub name: String,
    /// Its version, as the owner sees it.
    pub version: String,
    /// Its tabs: one or more.
    pub tabs: Vec<Tab>,
    /// The build command (`sh -c`, in the checkout).
    pub build: String,
    /// The backend command (`sh -c`, in the checkout), with `PORT`, `COOEE_PLUGIN_TOKEN` and `COOEE` (cooee's URL).
    pub run: String,
    /// The tools it needs on `PATH`.
    #[serde(default)]
    pub needs: Vec<String>,
}

/// One tab of a plugin: its `[[plugins]]` entry's name, label and UI module.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Tab {
    /// Its URL, `/p/<name>`.
    pub name: String,
    /// Its label.
    pub label: String,
    /// Its module's path on the backend.
    pub module: String,
}

/// A plugin or theme pack in the index: reviewed, pinned to a commit.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Listed {
    /// Its manifest's name.
    pub name: String,
    /// Its git URL.
    pub url: String,
    /// The reviewed commit.
    pub commit: String,
    /// A theme pack's folder in the repo (none: its root).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
    /// One line on what it does.
    pub description: String,
    /// It lives with cooee: improved through an issue, not `[improve]`.
    #[serde(default)]
    pub first_party: bool,
}

/// One installed plugin in `plugins.lock`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize, Serialize)]
pub struct Locked {
    /// Its name.
    pub name: String,
    /// Its git URL.
    pub url: String,
    /// The commit running: its build is `<name>/builds/<commit>`.
    pub commit: String,
    /// Its version at that commit.
    pub version: String,
    /// The build before it, kept to roll back to.
    #[serde(default)]
    pub previous: Option<String>,
    /// It came from the index.
    #[serde(default)]
    pub listed: bool,
    /// Its tabs' entry names.
    pub tabs: Vec<String>,
    /// Its backend's port.
    pub port: u16,
    /// Whether cooee runs it.
    pub enabled: bool,
    /// Why it was turned off, when cooee did it.
    #[serde(default)]
    pub note: String,
    /// The thread improving it, on its checkout's `cooee-improve` branch.
    #[serde(default)]
    pub improving: Option<String>,
}

/// The built-in index.
pub const INDEX: &str = include_str!("../../../plugins/index.toml");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IndexFile {
    #[serde(default)]
    plugin: Vec<Listed>,
    #[serde(default)]
    theme: Vec<Listed>,
}

#[derive(Deserialize)]
struct Entries {
    #[serde(default)]
    plugins: Vec<Plugin>,
}

fn bad(e: impl std::fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
}

fn read_index(path: Option<&Path>) -> io::Result<IndexFile> {
    let text = match path {
        Some(p) => fs::read_to_string(p)?,
        None => INDEX.to_owned(),
    };
    toml::from_str(&text).map_err(bad)
}

/// The index's plugins: the index at `path`, else the built-in one.
///
/// # Errors
/// When the file can't be read or parsed.
pub fn index(path: Option<&Path>) -> io::Result<Vec<Listed>> {
    Ok(read_index(path)?.plugin)
}

/// The index's theme packs: the index at `path`, else the built-in one.
///
/// # Errors
/// When the file can't be read or parsed.
pub fn themes(path: Option<&Path>) -> io::Result<Vec<Listed>> {
    Ok(read_index(path)?.theme)
}

impl Manifest {
    /// Parses and checks a manifest: a name of lowercase letters, digits and `-`, and at least one tab.
    ///
    /// # Errors
    /// When it isn't valid TOML for a manifest or breaks a rule.
    pub fn parse(text: &str) -> io::Result<Self> {
        let m: Self = toml::from_str(text).map_err(|e| bad(format!("cooee-plugin.toml: {e}")))?;
        let ok = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-';
        if m.name.is_empty() || !m.name.bytes().all(ok) {
            return Err(bad(
                "cooee-plugin.toml: `name` must be lowercase letters, digits and -",
            ));
        }
        if m.tabs.is_empty() {
            return Err(bad("cooee-plugin.toml: a plugin needs at least one tab"));
        }
        Ok(m)
    }
}

/// `<data>/plugins.lock`.
#[must_use]
pub fn lock_path(data: &Path) -> PathBuf {
    data.join("plugins.lock")
}

/// `<data>/plugins/plugins.toml`, the installed plugins' `[[plugins]]` entries.
#[must_use]
pub fn entries_path(data: &Path) -> PathBuf {
    data.join("plugins/plugins.toml")
}

/// The lock's plugins (none when there is no lock).
///
/// # Errors
/// When it can't be read or parsed.
pub fn read_lock(data: &Path) -> io::Result<Vec<Locked>> {
    match fs::read(lock_path(data)) {
        Ok(b) => serde_json::from_slice(&b).map_err(bad),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

/// Replaces the lock, atomically.
///
/// # Errors
/// When it can't be written.
pub fn write_lock(data: &Path, lock: &[Locked]) -> io::Result<()> {
    let text = serde_json::to_string_pretty(lock).map_err(bad)? + "\n";
    crate::write_private(&lock_path(data), text.as_bytes())
}

/// The installed plugins' entries (none when there are none).
///
/// # Errors
/// When the file can't be read or parsed.
pub fn read_entries(data: &Path) -> io::Result<Vec<Plugin>> {
    match fs::read_to_string(entries_path(data)) {
        Ok(t) => Ok(toml::from_str::<Entries>(&t).map_err(bad)?.plugins),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

/// Replaces the entries file (mode 0600: it holds the tokens), atomically.
///
/// # Errors
/// When it can't be written.
pub fn write_entries(data: &Path, entries: &[Plugin]) -> io::Result<()> {
    // A JSON string is a TOML basic string too.
    let q = |s: &str| serde_json::to_string(s).unwrap_or_default();
    let mut text = String::from("# Written by cooee plugin add/update/remove.\n");
    for p in entries {
        let _ = write!(
            text,
            "\n[[plugins]]\nname = {}\nlabel = {}\nurl = {}\nmodule = {}\ntoken = {}\n",
            q(&p.name),
            q(&p.label),
            q(&p.url),
            q(&p.module),
            q(&p.token)
        );
    }
    crate::write_private(&entries_path(data), text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_built_in_index_parses() {
        let index = index(None).unwrap();
        assert!(
            index
                .iter()
                .any(|l| l.name == "cooee-drover" && l.commit.len() == 40)
        );
        let names: Vec<_> = themes(None).unwrap().into_iter().map(|t| t.name).collect();
        assert_eq!(names, ["farm", "kitchen", "garden"]);
    }

    #[test]
    fn a_manifest_needs_a_name_and_a_tab() {
        let ok = "name = \"x\"\nversion = \"1\"\nbuild = \"true\"\nrun = \"true\"\n\
                  [[tabs]]\nname = \"x\"\nlabel = \"X\"\nmodule = \"/tab.js\"\n";
        assert_eq!(Manifest::parse(ok).unwrap().tabs[0].label, "X");
        assert!(Manifest::parse(&ok.replace("\"x\"\nversion", "\"X\"\nversion")).is_err());
        assert!(Manifest::parse(ok.split("[[tabs]]").next().unwrap()).is_err());
    }

    #[test]
    fn entries_round_trip() {
        let data = std::env::temp_dir().join(format!("cooee-entries-{}", std::process::id()));
        let p = Plugin {
            name: "a".into(),
            label: "A \"q\"".into(),
            url: "http://127.0.0.1:1".into(),
            module: "/t.js".into(),
            token: "0123456789abcdef".into(),
        };
        write_entries(&data, std::slice::from_ref(&p)).unwrap();
        assert_eq!(read_entries(&data).unwrap(), [p]);
        fs::remove_dir_all(&data).unwrap();
    }
}
