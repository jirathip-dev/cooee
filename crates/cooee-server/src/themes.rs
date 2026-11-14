use std::{
    fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Mutex,
};

use cooee_core::{Config, plugins::Listed};
use serde::{Deserialize, Serialize};

use crate::hub::lock;

const STARTER: [(&str, &str); 3] = [
    (
        "theme.json",
        include_str!("../../../packs/starter/theme.json"),
    ),
    (
        "sprites.svg",
        include_str!("../../../packs/starter/sprites.svg"),
    ),
    (
        "scene.svg",
        include_str!("../../../packs/starter/scene.svg"),
    ),
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
    cooee_web::check(dir).map_err(|fails| {
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

    /// Theme `name`'s folder in `themes-dev/`.
    #[must_use]
    pub fn dev_dir(&self, name: &str) -> PathBuf {
        self.dir.join("themes-dev").join(name)
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

    /// Installs `pack`: copies it (at its commit, for a git repo) into a staging folder, runs the theme gate on it,
    /// then moves it into `themes/` and pins it.
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
        let commit = if folder.is_dir() {
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

    fn fetch(&self, pack: &Pack, stage: &Path) -> io::Result<String> {
        let repo = self.data.join("themes").join(format!("{}.git", pack.name));
        if repo.exists() {
            git(&repo, &["remote", "set-url", "origin", &pack.source])?;
            git(&repo, &["fetch", "--quiet", "origin"])?;
        } else {
            fs::create_dir_all(repo.parent().unwrap_or(&repo))?;
            let at = repo.to_string_lossy();
            git(
                &self.data,
                &["clone", "--quiet", "--mirror", "--", &pack.source, &at],
            )?;
        }
        let commit = git(
            &repo,
            &["rev-parse", "--verify", &format!("{}^{{commit}}", pack.rev)],
        )
        .map_err(|_| other(format!("{} has no commit {}", pack.source, pack.rev)))?;
        fs::create_dir_all(stage)?;
        let mut archive = Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args([
                "archive",
                "--format=tar",
                &format!("{commit}:{}", pack.path),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let untar = Command::new("tar")
            .arg("-x")
            .arg("-C")
            .arg(stage)
            .stdin(archive.stdout.take().ok_or_else(|| other("git archive"))?)
            .status()?;
        if !archive.wait()?.success() || !untar.success() {
            return Err(other(format!(
                "{} has no folder {:?} at {commit}",
                pack.source, pack.path
            )));
        }
        Ok(commit)
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

    /// `[new theme]` (`from` none: the starter pack) or `[remix]` of installed theme `from`: theme `name`'s folder in
    /// `themes-dev/`, for an agent to build.
    ///
    /// # Errors
    /// When the name is taken or `from` isn't installed.
    pub fn start(&self, name: &str, from: Option<&str>) -> io::Result<PathBuf> {
        let _one = lock(&self.ops);
        valid(name)?;
        let dev = self.dev_dir(name);
        if dev.exists() || self.installed_dir(name).exists() {
            return Err(other(format!("a theme named {name} is there already")));
        }
        if let Some(from) = from {
            valid(from)?;
            let src = self.installed_dir(from);
            if !src.is_dir() {
                return Err(other(format!("{from} isn't installed")));
            }
            copy(&src, &dev)?;
        } else {
            fs::create_dir_all(&dev)?;
            for (file, text) in STARTER {
                fs::write(dev.join(file), text)?;
            }
        }
        Ok(dev)
    }

    /// `[keep]`: theme `name` from `themes-dev/` into `themes/`, only once it passes the theme gate.
    ///
    /// # Errors
    /// The gate's failures, or when the name is taken by now.
    pub fn keep(&self, name: &str) -> io::Result<()> {
        let _one = lock(&self.ops);
        valid(name)?;
        let (dev, to) = (self.dev_dir(name), self.installed_dir(name));
        if !dev.is_dir() {
            return Err(other(format!("no theme {name} is being built")));
        }
        gate(&dev)?;
        if to.exists() {
            return Err(other(format!("a theme named {name} is installed already")));
        }
        fs::create_dir_all(to.parent().unwrap_or(&to))?;
        fs::rename(dev, to)?;
        let made = Pinned {
            name: name.to_owned(),
            source: "made here".to_owned(),
            ..Pinned::default()
        };
        self.pin(name, Some(made))
    }

    /// `[publish]`: installed theme `name` as a git repo of its own (its changes committed); returns the folder and
    /// commit, to push and to propose for the index by PR.
    ///
    /// # Errors
    /// When it isn't installed or git fails.
    pub fn publish(&self, name: &str) -> io::Result<(PathBuf, String)> {
        let _one = lock(&self.ops);
        valid(name)?;
        let dir = self.installed_dir(name);
        if !dir.is_dir() {
            return Err(other(format!("{name} isn't installed")));
        }
        if !dir.join(".git").exists() {
            git(&dir, &["init", "--quiet", "-b", "main"])?;
        }
        git(&dir, &["add", "-A"])?;
        if !git(&dir, &["status", "--porcelain"])?.is_empty() {
            git(&dir, &["commit", "--quiet", "-m", &format!("theme {name}")])?;
        }
        let commit = git(&dir, &["rev-parse", "HEAD"])?;
        let mut p = self.pinned()?.into_iter().find(|p| p.name == name);
        let p = p.get_or_insert_with(|| Pinned {
            name: name.to_owned(),
            source: "made here".to_owned(),
            ..Pinned::default()
        });
        p.commit.clone_from(&commit);
        self.pin(name, Some(p.clone()))?;
        Ok((dir, commit))
    }
}

/// A fingerprint of the files in `dir` (names, sizes and change times): the preview reloads when it changes.
#[must_use]
pub fn stamp(dir: &Path) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    fn walk(dir: &Path, h: &mut DefaultHasher) {
        let Ok(list) = fs::read_dir(dir) else { return };
        let mut list: Vec<_> = list.flatten().collect();
        list.sort_by_key(fs::DirEntry::file_name);
        for e in list {
            e.file_name().hash(h);
            match e.metadata() {
                Ok(m) if m.is_dir() => walk(&e.path(), h),
                Ok(m) => (m.len(), m.modified().ok()).hash(h),
                Err(_) => {}
            }
        }
    }
    let mut h = DefaultHasher::new();
    walk(dir, &mut h);
    h.finish()
}
