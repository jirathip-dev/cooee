//! Installed plugins (docs/plugins/): a plugin is a git repo with a `cooee-plugin.toml` at its root. cooee keeps
//! it in `<data>/plugins/<name>/`, pins it in `<data>/plugins.lock` and writes its tabs as `[[plugins]]` entries in
//! `<data>/plugins/plugins.toml`, read beside the config's own. The curated index, `plugins/index.toml`, is built in.

use std::{
    collections::HashMap,
    fmt::Write as _,
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, Mutex, PoisonError},
    time::SystemTime,
};

use serde::{Deserialize, Serialize};

use crate::{Plugin, profiles::Profile};

/// A plugin's rows (drover#1571): what cooee reads and writes at its `[threads]` paths.
pub mod rows;

/// `cooee-plugin.toml`: what a plugin is, builds with and runs.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// Its name: lowercase letters, digits and `-`; its folder and lock entry.
    pub name: String,
    /// Its version, numbers and dots (`1.2.0`, a leading `v` allowed).
    pub version: String,
    /// The oldest cooee it accepts (none: any).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub min_cooee: String,
    /// Its PWA tabs (none: it has screens only).
    #[serde(default)]
    pub tabs: Vec<Tab>,
    /// Where the native app shows its screens (#125's places).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub places: Vec<Place>,
    /// The workflow `source` labels it owns (#229): the native app draws those sessions in its place, not as chats.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub claims: Vec<String>,
    /// What it may do: shown when it's attached, refused beyond.
    #[serde(default)]
    pub permissions: Permissions,
    /// The build command (`sh -c`, in the checkout); an attach-only plugin's never runs.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub build: String,
    /// The backend command (`sh -c`, in the checkout), with `PORT`, `COOEE_PLUGIN_TOKEN` and `COOEE` (cooee's URL).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub run: String,
    /// A backend at `http://127.0.0.1:<port>`: cooee uses it when it answers, else runs `run` there, if any.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub attach: String,
    /// The file (`~/` or absolute) holding the token the `attach` backend accepts; cooee calls it with that one.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub attach_token: String,
    /// The tools it needs on `PATH`.
    #[serde(default)]
    pub needs: Vec<String>,
    /// The agent profiles it brings, their paths in its checkout.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub profiles: Vec<Profile>,
    /// Where it serves its rows (drover#1571); none: it has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threads: Option<rows::Threads>,
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

/// Where a place goes: a chip on a persona's header strip, cards in its chats, or a row in the avatar menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlaceKind {
    /// A chip under a persona's chat header that opens `screen` as a sheet.
    HeaderStrip,
    /// Cards in a persona's chats.
    ChatCards,
    /// A row under the avatar menu's Plugins that opens `screen`.
    MenuRow,
}

/// One place of a plugin's screens (#125).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Place {
    /// Where it goes.
    #[serde(rename = "type")]
    pub kind: PlaceKind,
    /// Its id among the plugin's places.
    #[serde(default)]
    pub id: String,
    /// The persona it attaches to by default.
    #[serde(default)]
    pub persona: String,
    /// What the chip or row says.
    #[serde(default)]
    pub label: String,
    /// A menu row's second line.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub subtitle: String,
    /// Its screen's path on the backend, under `/screens/` (a `chat-cards` place's defaults to `/screens/cards`).
    #[serde(default)]
    pub screen: String,
}

/// `[permissions]`: what a plugin may do (#125). The app shows them at attach; cooee refuses the rest.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Permissions {
    /// The kinds of place it may have.
    pub places: Vec<PlaceKind>,
    /// It may notify.
    pub push: bool,
    /// The actions its screens and cards may send.
    pub actions: Vec<String>,
    /// Those the app always confirms first.
    pub destructive: Vec<String>,
    /// The hosts its backend, images and web blocks reach.
    pub network: Vec<String>,
}

/// A plugin in the index: reviewed, pinned to a commit.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Listed {
    /// Its manifest's name.
    pub name: String,
    /// Its git URL.
    pub url: String,
    /// The reviewed commit.
    pub commit: String,
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

/// This cooee's version.
pub const COOEE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The oldest plugin version this cooee accepts.
pub const MIN_PLUGIN: &str = "0.1.0";

fn numbers(v: &str) -> Option<Vec<u64>> {
    let mut n: Vec<u64> = v
        .strip_prefix('v')
        .unwrap_or(v)
        .split('.')
        .map(|part| {
            let digits = part.split(|c: char| !c.is_ascii_digit()).next()?;
            digits.parse().ok()
        })
        .collect::<Option<_>>()?;
    while n.last() == Some(&0) {
        n.pop();
    }
    Some(n)
}

fn older(a: &str, than: &str) -> bool {
    numbers(a) < numbers(than)
}

/// The built-in index.
pub const INDEX: &str = include_str!("../../../plugins/index.toml");

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct IndexFile {
    #[serde(default)]
    plugin: Vec<Listed>,
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

impl Manifest {
    /// Parses and checks a manifest: a name of lowercase letters, digits and `-`, a tab or a place, each place
    /// permitted, with a label and a `/screens/` path where it opens one, and destructive actions among its actions.
    ///
    /// # Errors
    /// When it isn't valid TOML for a manifest or breaks a rule.
    pub fn parse(text: &str) -> io::Result<Self> {
        let mut m: Self =
            toml::from_str(text).map_err(|e| bad(format!("cooee-plugin.toml: {e}")))?;
        for p in &mut m.places {
            if p.kind == PlaceKind::ChatCards && p.screen.is_empty() {
                "/screens/cards".clone_into(&mut p.screen);
            }
        }
        for (key, v) in [("version", &m.version), ("min_cooee", &m.min_cooee)] {
            if numbers(v).is_none() && (key == "version" || !v.is_empty()) {
                return Err(bad(format!(
                    "cooee-plugin.toml: `{key}` must be numbers and dots, like 1.2.0"
                )));
            }
        }
        let ok = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-';
        if m.name.is_empty() || !m.name.bytes().all(ok) {
            return Err(bad(
                "cooee-plugin.toml: `name` must be lowercase letters, digits and -",
            ));
        }
        if m.tabs.is_empty() && m.places.is_empty() {
            return Err(bad("cooee-plugin.toml: a plugin needs a tab or a place"));
        }
        m.check_permissions()?;
        if !m.attach.is_empty() && m.attach_port().is_none() {
            return Err(bad(
                "cooee-plugin.toml: `attach` must be http://127.0.0.1:<port>",
            ));
        }
        if m.run.is_empty() && m.attach.is_empty() {
            return Err(bad("cooee-plugin.toml: a plugin needs `run` or `attach`"));
        }
        let token_ok = m.attach_token.starts_with('/') || m.attach_token.starts_with("~/");
        if !m.attach_token.is_empty() && (m.attach.is_empty() || !token_ok) {
            return Err(bad(
                "cooee-plugin.toml: `attach_token` needs `attach` and a path, ~/ or absolute",
            ));
        }
        for p in &m.profiles {
            p.check()
                .map_err(|e| bad(format!("cooee-plugin.toml: {e}")))?;
        }
        Ok(m)
    }

    fn check_permissions(&self) -> io::Result<()> {
        let perms = &self.permissions;
        for p in &self.places {
            if !perms.places.contains(&p.kind) {
                return Err(bad(format!(
                    "cooee-plugin.toml: {:?} is not in [permissions] places",
                    p.kind
                )));
            }
            let opens = p.kind != PlaceKind::ChatCards;
            if opens && (p.label.is_empty() || !p.screen.starts_with("/screens/")) {
                return Err(bad(
                    "cooee-plugin.toml: a place needs a label and a screen under /screens/",
                ));
            }
        }
        if let Some(a) = perms
            .destructive
            .iter()
            .find(|a| !perms.actions.contains(a))
        {
            return Err(bad(format!(
                "cooee-plugin.toml: destructive action {a} is not in [permissions] actions"
            )));
        }
        Ok(())
    }

    /// The cooee versions it accepts, as the owner reads them.
    #[must_use]
    pub fn accepts(&self) -> String {
        if self.min_cooee.is_empty() {
            "any cooee".to_owned()
        } else {
            format!("cooee {} or newer", self.min_cooee)
        }
    }

    /// Whether it and this cooee accept each other.
    ///
    /// # Errors
    /// One plain line naming the version needed, when either is too old for the other.
    pub fn fits(&self) -> Result<(), String> {
        if older(&self.version, MIN_PLUGIN) {
            return Err(format!(
                "{} {} is too old for cooee {COOEE_VERSION}: it needs {} {MIN_PLUGIN} or newer",
                self.name, self.version, self.name
            ));
        }
        if older(COOEE_VERSION, &self.min_cooee) {
            return Err(format!(
                "{} needs cooee {} or newer; this is cooee {COOEE_VERSION}",
                self.name, self.min_cooee
            ));
        }
        Ok(())
    }

    /// Whether it only attaches: no `run`, so cooee never builds or starts it.
    #[must_use]
    pub fn attach_only(&self) -> bool {
        self.run.is_empty()
    }

    /// The port of its `attach` backend, when it names one.
    #[must_use]
    pub fn attach_port(&self) -> Option<u16> {
        let port = self.attach.strip_prefix("http://127.0.0.1:")?;
        port.trim_end_matches('/').parse().ok().filter(|&p| p > 0)
    }

    /// Its `attach_token` file, `~/` under `$HOME`.
    #[must_use]
    pub fn attach_token_path(&self) -> Option<PathBuf> {
        match self.attach_token.strip_prefix("~/") {
            Some(rest) => Some(Path::new(&crate::env_var("HOME")?).join(rest)),
            None => (!self.attach_token.is_empty()).then(|| PathBuf::from(&self.attach_token)),
        }
    }
}

/// `<data>/plugins.lock`.
#[must_use]
pub fn lock_path(data: &Path) -> PathBuf {
    data.join("plugins.lock")
}

/// Plugin `name`'s build at `commit`: `<data>/plugins/<name>/builds/<commit>`.
#[must_use]
pub fn build_dir(data: &Path, name: &str, commit: &str) -> PathBuf {
    data.join("plugins").join(name).join("builds").join(commit)
}

/// Installed plugin `l`'s manifest, from its running build.
///
/// # Errors
/// When the build's manifest can't be read or parsed.
pub fn manifest(data: &Path, l: &Locked) -> io::Result<Manifest> {
    let file = build_dir(data, &l.name, &l.commit).join("cooee-plugin.toml");
    Manifest::parse(&fs::read_to_string(file)?)
}

type Stamp = Option<(SystemTime, u64)>;

type Installed = Arc<[(Locked, Manifest)]>;

static READ: LazyLock<Mutex<HashMap<PathBuf, (Stamp, Installed)>>> = LazyLock::new(Mutex::default);

/// The lock's plugins whose manifest reads, each with it; read again only once the lock file changes.
///
/// # Errors
/// When the lock can't be read or parsed.
pub fn installed(data: &Path) -> io::Result<Installed> {
    let stamp = match fs::metadata(lock_path(data)) {
        Ok(m) => Some((m.modified()?, m.len())),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };
    let mut read = READ.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some((at, all)) = read.get(data)
        && *at == stamp
    {
        return Ok(Arc::clone(all));
    }
    let all: Installed = read_lock(data)?
        .into_iter()
        .filter_map(|l| manifest(data, &l).ok().map(|m| (l, m)))
        .collect();
    read.insert(data.to_owned(), (stamp, Arc::clone(&all)));
    Ok(all)
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
    let q = |s: &str| serde_json::to_string(s).unwrap_or_default();
    let mut text = String::from("# Written by cooee plugin add/update/remove.\n");
    for p in entries {
        write!(
            text,
            "\n[[plugins]]\nname = {}\nlabel = {}\nurl = {}\nmodule = {}\ntoken = {}\n",
            q(&p.name),
            q(&p.label),
            q(&p.url),
            q(&p.module),
            q(&p.token)
        )
        .ok();
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
                .any(|l| l.name == "drover" && l.commit.len() == 40)
        );
    }

    #[test]
    fn a_manifest_needs_a_name_a_tab_or_a_permitted_place() {
        let ok = "name = \"x\"\nversion = \"1\"\nbuild = \"true\"\nrun = \"true\"\n\
                  [[tabs]]\nname = \"x\"\nlabel = \"X\"\nmodule = \"/tab.js\"\n";
        assert_eq!(Manifest::parse(ok).unwrap().tabs[0].label, "X");
        assert!(Manifest::parse(&ok.replace("\"x\"\nversion", "\"X\"\nversion")).is_err());
        let bare = ok.split("[[tabs]]").next().unwrap();
        assert!(Manifest::parse(bare).is_err());
        let place = "[[places]]\ntype = \"menu-row\"\nlabel = \"Hi\"\nscreen = \"/screens/hi\"\n";
        let perms = "[permissions]\nplaces = [\"menu-row\"]\nactions = [\"wave\"]\n";
        let m = Manifest::parse(&format!("{bare}{place}{perms}")).unwrap();
        assert_eq!(m.places[0].kind, PlaceKind::MenuRow);
        assert_eq!(m.permissions.actions, ["wave"]);
        assert!(Manifest::parse(&format!("{bare}{place}")).is_err());
        let off = place.replace("/screens/hi", "/hi");
        assert!(Manifest::parse(&format!("{bare}{off}{perms}")).is_err());
        let wild = perms.replace("actions", "destructive = [\"rm\"]\nactions");
        assert!(Manifest::parse(&format!("{bare}{place}{wild}")).is_err());
        let with =
            |p: &str| format!("{ok}[[profiles]]\nname = {p:?}\nlabel = \"C\"\nagent = \"a\"\n");
        assert_eq!(Manifest::parse(&with("c")).unwrap().profiles[0].label, "C");
        assert!(Manifest::parse(&with("C!")).is_err());
        let at = |a: &str| Manifest::parse(&format!("attach = {a:?}\n{ok}"));
        assert_eq!(
            at("http://127.0.0.1:7777").unwrap().attach_port(),
            Some(7777)
        );
        assert!(at("http://example.com:7777").is_err() && at("http://127.0.0.1:0").is_err());
        assert_eq!(Manifest::parse(ok).unwrap().attach_port(), None);
        let tok =
            |t: &str| format!("attach = \"http://127.0.0.1:7777\"\nattach_token = {t:?}\n{ok}");
        let home = crate::env_var("HOME").unwrap();
        assert_eq!(
            Manifest::parse(&tok("~/.config/herdsman/cooee-token"))
                .unwrap()
                .attach_token_path(),
            Some(Path::new(&home).join(".config/herdsman/cooee-token"))
        );
        assert!(Manifest::parse(&tok("relative")).is_err());
        assert!(Manifest::parse(&format!("attach_token = \"/t\"\n{ok}")).is_err());
        let no_run = ok.replace("build = \"true\"\nrun = \"true\"\n", "");
        let only = Manifest::parse(&tok("/t").replace(ok, &no_run)).unwrap();
        assert!(only.attach_only() && only.attach_port() == Some(7777));
        assert_eq!(only.attach_token_path(), Some(PathBuf::from("/t")));
        assert!(!Manifest::parse(ok).unwrap().attach_only());
        let neither = Manifest::parse(&no_run).unwrap_err().to_string();
        assert!(neither.contains("needs `run` or `attach`"), "{neither}");
    }

    #[test]
    fn entries_round_trip() {
        let data = crate::scratch::owned(
            std::env::temp_dir().join(format!("cooee-entries-{}", std::process::id())),
        );
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
