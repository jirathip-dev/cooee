//! Theme packs (docs/themes.md): `cooee theme add|remove|list`. A pack is a folder or a folder of a git repo (a
//! shallow clone of that folder alone, in `<data>/themes/<name>/`); an installed one is copied into
//! `<config dir>/themes/<name>/` once it passes the theme gate, and pinned in `<data>/themes.lock` (its source and
//! commit). farm ships inside the build.

use std::{
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Mutex,
};

use cooee_core::{Config, plugins::Listed};
use serde::{Deserialize, Serialize};

use crate::hub::lock;

/// farm, the default theme, inside the build (#363): the index's farm at its commit installs from here, offline.
const FARM: [(&str, &[u8]); 4] = [
    (
        "theme.json",
        include_bytes!("../../../packs/farm/theme.json"),
    ),
    (
        "sprites.svg",
        include_bytes!("../../../packs/farm/sprites.svg"),
    ),
    ("scene.svg", include_bytes!("../../../packs/farm/scene.svg")),
    ("icon.png", include_bytes!("../../../packs/farm/icon.png")),
];

fn other(why: impl Into<String>) -> io::Error {
    io::Error::other(why.into())
}

/// One installed pack in `themes.lock`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Pinned {
    /// Its folder in `themes/`.
    pub name: String,
    /// Where it came from: the index, its git URL or folder, or `made here`.
    pub source: String,
    /// The commit it was copied from (none: not a git repo).
    #[serde(default)]
    pub commit: String,
    /// It came from the index.
    #[serde(default)]
    pub listed: bool,
}

/// What `add` would install: its name, source, the folder in the repo and the revision; `text` says so, with "not
/// reviewed" when it isn't in the index.
#[derive(Debug, Clone)]
pub struct Pack {
    /// Its folder in `themes/`.
    pub name: String,
    /// The git URL or local folder.
    pub source: String,
    /// The folder in the repo (none: its root).
    pub path: String,
    /// The revision (a git source).
    pub rev: String,
    /// It is in the index.
    pub listed: bool,
    /// The confirm, for the owner.
    pub text: String,
}

/// The theme packs of one instance.
pub struct Themes {
    dir: PathBuf,
    data: PathBuf,
    index: Vec<Listed>,
    ops: Mutex<()>,
}

fn git(dir: &Path, args: &[&str]) -> io::Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=cooee", "-c", "user.email=cooee@localhost"])
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .output()?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(other(format!("git {}: {}", args[0], err.trim())));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// Copies folder `from` into `to` (made), all but `.git`; a symlink is refused, never followed.
fn copy(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for e in fs::read_dir(from)? {
        let e = e?;
        if e.file_name() == ".git" {
            continue;
        }
        let t = e.file_type()?;
        if t.is_symlink() {
            return Err(other(format!(
                "{}: a symlink (a pack holds files only)",
                e.path().display()
            )));
        }
        if t.is_dir() {
            copy(&e.path(), &to.join(e.file_name()))?;
        } else {
            fs::copy(e.path(), to.join(e.file_name()))?;
        }
    }
    Ok(())
}

fn valid(name: &str) -> io::Result<()> {
    let ok = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-';
    if name.is_empty() || !name.bytes().all(ok) {
        return Err(other(format!(
            "{name:?}: a theme's name is lowercase letters, digits and -"
        )));
    }
    Ok(())
}

fn gate(dir: &Path) -> io::Result<()> {
    crate::look::check(dir).map_err(|fails| {
        other(format!(
            "it fails the theme gate:\n  {}",
            fails.join("\n  ")
        ))
    })
}

impl Themes {
    /// The instance's packs: its config folder, data folder and index (`plugin_index`, else the built-in one).
    ///
    /// # Errors
    /// When there is no config or data folder, or the index can't be read.
    pub fn new(config: &Config) -> io::Result<Self> {
        let dir = config
            .dir
            .clone()
            .ok_or_else(|| other("no config folder"))?;
        let data = config
            .data_dir()
            .ok_or_else(|| other("no data folder: set `data` or HOME"))?;
        let index = cooee_core::plugins::themes(config.plugin_index.as_deref())?;
        Ok(Self {
            dir,
            data,
            index,
            ops: Mutex::default(),
        })
    }

    /// The index's packs.
    #[must_use]
    pub fn index(&self) -> &[Listed] {
        &self.index
    }

    fn installed_dir(&self, name: &str) -> PathBuf {
        self.dir.join("themes").join(name)
    }

    fn lock_path(&self) -> PathBuf {
        self.data.join("themes.lock")
    }

    /// The installed packs, as the lock pins them.
    ///
    /// # Errors
    /// When the lock can't be read.
    pub fn pinned(&self) -> io::Result<Vec<Pinned>> {
        match fs::read(self.lock_path()) {
            Ok(b) => serde_json::from_slice(&b).map_err(|e| other(e.to_string())),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }

    /// Pins `p` (`None`: unpins `name`).
    fn pin(&self, name: &str, p: Option<Pinned>) -> io::Result<()> {
        let mut all = self.pinned()?;
        all.retain(|x| x.name != name);
        all.extend(p);
        all.sort_by(|a, b| a.name.cmp(&b.name));
        let text = serde_json::to_string_pretty(&all).map_err(|e| other(e.to_string()))? + "\n";
        cooee_core::write_private(&self.lock_path(), text.as_bytes())
    }

    /// What adding `source` would install: a name in the index (at its pinned commit), a local folder, or a git URL
    /// (at `rev`, else its default branch).
    ///
    /// # Errors
    /// When a name isn't in the index, or the name it would take isn't a theme's.
    pub fn plan(&self, source: &str, rev: Option<&str>) -> io::Result<Pack> {
        if rev.is_some_and(|r| r.starts_with('-')) || source.starts_with('-') {
            return Err(other("a source or --rev can't start with -"));
        }
        let listed = self.index.iter().find(|l| l.name == source);
        let pack = match listed {
            Some(l) => Pack {
                name: l.name.clone(),
                source: l.url.clone(),
                path: l.path.clone(),
                rev: rev.unwrap_or(&l.commit).to_owned(),
                listed: true,
                text: String::new(),
            },
            None if source.contains('/') || source.contains(':') => {
                // A folder by its full path, so its pin says where it was.
                let source = fs::canonicalize(source)
                    .map_or_else(|_| source.to_owned(), |p| p.to_string_lossy().into_owned());
                let name = source
                    .trim_end_matches('/')
                    .rsplit(['/', ':'])
                    .next()
                    .unwrap_or_default();
                Pack {
                    name: name.trim_end_matches(".git").to_owned(),
                    source: source.clone(),
                    path: String::new(),
                    rev: rev.unwrap_or("HEAD").to_owned(),
                    listed: false,
                    text: String::new(),
                }
            }
            None => {
                return Err(other(format!(
                    "{source} isn't in the index: add it by its git URL or folder"
                )));
            }
        };
        valid(&pack.name)?;
        let at = if pack.path.is_empty() {
            String::new()
        } else {
            format!(" ({})", pack.path)
        };
        let mut text = format!(
            "theme {} from {}{at} at {}",
            pack.name, pack.source, pack.rev
        );
        if !pack.listed {
            text += "\nnot reviewed: this theme isn't in cooee's index; look at its files before you add it";
        }
        Ok(Pack { text, ..pack })
    }

    /// `pack` is the built-in index's farm at its commit, which the build carries.
    fn bundled(pack: &Pack) -> bool {
        let farm = cooee_core::plugins::themes(None).unwrap_or_default();
        farm.iter().any(|l| {
            l.name == "farm"
                && pack.listed
                && pack.name == l.name
                && (&pack.source, &pack.path, &pack.rev) == (&l.url, &l.path, &l.commit)
        })
    }

    /// Installs `pack`: copies it (farm from the build, a git repo's folder at its commit) into a staging folder,
    /// runs the theme gate on it, then moves it into `themes/` and pins it.
    ///
    /// # Errors
    /// When it is installed already, can't be fetched, or fails the gate.
    pub fn add(&self, pack: &Pack) -> io::Result<()> {
        let _one = lock(&self.ops);
        let to = self.installed_dir(&pack.name);
        if to.exists() {
            return Err(other(format!(
                "{} is installed already: remove it first",
                pack.name
            )));
        }
        let stage = self.dir.join(".themes-stage").join(&pack.name);
        let _ = fs::remove_dir_all(&stage);
        let folder = Path::new(&pack.source);
        let commit = if Self::bundled(pack) {
            fs::create_dir_all(&stage)?;
            for (file, bytes) in FARM {
                fs::write(stage.join(file), bytes)?;
            }
            pack.rev.clone()
        } else if !pack.listed && folder.is_dir() {
            copy(&folder.join(&pack.path), &stage)?;
            git(folder, &["rev-parse", "HEAD"]).unwrap_or_default()
        } else {
            self.fetch(pack, &stage)?
        };
        let done = gate(&stage).and_then(|()| {
            fs::create_dir_all(to.parent().unwrap_or(&to))?;
            fs::rename(&stage, &to)
        });
        if done.is_err() {
            let _ = fs::remove_dir_all(&stage);
        }
        done?;
        self.pin(
            &pack.name,
            Some(Pinned {
                name: pack.name.clone(),
                source: if pack.listed {
                    "index".to_owned()
                } else {
                    pack.source.clone()
                },
                commit,
                listed: pack.listed,
            }),
        )
    }

    /// Fetches `pack`'s folder at its revision into its clone, `<data>/themes/<name>/`: shallow (`--depth 1`), only
    /// that folder checked out; copies it into `stage` and returns the commit.
    fn fetch(&self, pack: &Pack, stage: &Path) -> io::Result<String> {
        let repo = self.data.join("themes").join(&pack.name);
        fs::create_dir_all(&repo)?;
        let only = format!("/{}/", pack.path.trim_matches('/')).replace("//", "/*");
        git(&repo, &["init", "--quiet"])?;
        git(&repo, &["config", "remote.origin.url", &pack.source])?;
        git(&repo, &["sparse-checkout", "set", "--no-cone", &only])?;
        git(
            &repo,
            &[
                "fetch",
                "--quiet",
                "--depth",
                "1",
                "--filter=blob:none",
                "origin",
                &pack.rev,
            ],
        )
        .map_err(|e| other(format!("{} has no commit {}: {e}", pack.source, pack.rev)))?;
        git(
            &repo,
            &["checkout", "--quiet", "--force", "--detach", "FETCH_HEAD"],
        )?;
        let commit = git(&repo, &["rev-parse", "HEAD"])?;
        let folder = repo.join(&pack.path);
        if !folder.is_dir() {
            return Err(other(format!(
                "{} has no folder {:?} at {commit}",
                pack.source, pack.path
            )));
        }
        copy(&folder, stage)?;
        Ok(commit)
    }

    /// Replaces each old full clone, `<data>/themes/<name>.git`, with the new form, once that is in place: farm's is
    /// the build, any other pinned pack's its shallow clone; returns the bytes freed.
    ///
    /// # Errors
    /// When the lock can't be read; a pack that can't be fetched now (offline) keeps its old clone till next time.
    pub fn migrate(&self) -> io::Result<u64> {
        let pinned = self.pinned()?;
        let mut freed = 0;
        let Ok(dirs) = fs::read_dir(self.data.join("themes")) else {
            return Ok(0);
        };
        for old in dirs.flatten().map(|e| e.path()) {
            let Some(name) = old
                .file_name()
                .and_then(|n| n.to_str()?.strip_suffix(".git"))
            else {
                continue;
            };
            let Some(pin) = pinned.iter().find(|p| p.name == name) else {
                continue;
            };
            let pack = if pin.listed {
                self.plan(name, None)
            } else {
                self.plan(&pin.source, Some(&pin.commit))
            };
            let placed = pack.and_then(|p| {
                if Self::bundled(&p) {
                    return Ok(());
                }
                let stage = self.dir.join(".themes-stage").join(name);
                let fetched = self.fetch(&p, &stage);
                let _ = fs::remove_dir_all(&stage);
                fetched.map(drop)
            });
            match placed {
                Ok(()) => {
                    freed += size(&old);
                    fs::remove_dir_all(&old)?;
                }
                Err(e) => eprintln!("cooee: theme {name} keeps its old clone for now: {e}"),
            }
        }
        Ok(freed)
    }

    /// Removes installed theme `name`: its folder and its pin.
    ///
    /// # Errors
    /// When it isn't installed.
    pub fn remove(&self, name: &str) -> io::Result<()> {
        let _one = lock(&self.ops);
        valid(name)?;
        let dir = self.installed_dir(name);
        if !dir.is_dir() {
            return Err(other(format!("{name} isn't installed")));
        }
        fs::remove_dir_all(dir)?;
        self.pin(name, None)
    }

    /// The installed packs by folder, with their pin (an own folder has none).
    ///
    /// # Errors
    /// When `themes/` or the lock can't be read.
    pub fn list(&self) -> io::Result<Vec<Pinned>> {
        let pinned = self.pinned()?;
        let mut rows = Vec::new();
        let dirs = match fs::read_dir(self.dir.join("themes")) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(rows),
            dirs => dirs?,
        };
        for e in dirs.flatten().filter(|e| e.path().is_dir()) {
            let name = e.file_name().to_string_lossy().into_owned();
            let p = pinned.iter().find(|p| p.name == name).cloned();
            rows.push(p.unwrap_or(Pinned {
                name,
                source: "own folder".to_owned(),
                ..Pinned::default()
            }));
        }
        rows.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(rows)
    }
}

/// The bytes of the files under `dir`.
fn size(dir: &Path) -> u64 {
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| match e.metadata() {
            Ok(m) if m.is_dir() => size(&e.path()),
            Ok(m) => m.len(),
            Err(_) => 0,
        })
        .sum()
}
