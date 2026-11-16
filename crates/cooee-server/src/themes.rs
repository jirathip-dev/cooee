use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Stdio,
    sync::Mutex,
};

use cooee_core::{Config, swag::Swag};
use serde::{Deserialize, Serialize};

use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        Body,
        content::Json,
        path_param,
        response::{IntoResponse, Response},
        route,
    },
};

use crate::{
    http::{Data, plain},
    hub::lock,
    look::Looks,
};

fn other(why: impl Into<String>) -> io::Error {
    io::Error::other(why.into())
}

/// One installed pack in `themes.lock`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Pinned {
    /// Its folder in `themes/`.
    pub name: String,
    /// Where it came from: its git URL (swag's, for a first-party pack) or folder, or `made here`.
    pub source: String,
    /// The commit it was copied from (none: not a git repo).
    #[serde(default)]
    pub commit: String,
    /// A first-party pack, at swag's pin.
    #[serde(default)]
    pub listed: bool,
}

/// What `add` would install: its name, source, the folder in the repo and the revision; `text` says so, with "not
/// reviewed" when it isn't a first-party pack at swag's pin.
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
    /// A first-party pack at swag's pin.
    pub listed: bool,
    /// The confirm, for the owner.
    pub text: String,
}

/// The theme packs of one instance.
pub struct Themes {
    dir: PathBuf,
    data: PathBuf,
    swag: Swag,
    ops: Mutex<()>,
}

fn git(dir: &Path, args: &[&str]) -> io::Result<String> {
    let out = cooee_core::runner::command("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=cooee", "-c", "user.email=cooee@localhost"])
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .stdin(Stdio::null())
        .output()?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(other(format!(
            "git {}: {}",
            args.first().unwrap_or(&""),
            err.trim()
        )));
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
    crate::look::check(dir).map_err(|fails| {
        other(format!(
            "it fails the theme gate:\n  {}",
            fails.join("\n  ")
        ))
    })
}

impl Themes {
    /// The instance's packs: its config and data folders.
    ///
    /// # Errors
    /// When there is no config or data folder.
    pub fn new(config: &Config) -> io::Result<Self> {
        let dir = config
            .dir
            .clone()
            .ok_or_else(|| other("no config folder"))?;
        let data = config
            .data_dir()
            .ok_or_else(|| other("no data folder: set `data` or HOME"))?;
        Ok(Self {
            dir,
            data,
            swag: Swag::pinned(),
            ops: Mutex::default(),
        })
    }

    /// The first-party packs' names.
    #[must_use]
    pub fn first_party(&self) -> Vec<&str> {
        self.swag.names()
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

    fn pin(&self, name: &str, p: Option<Pinned>) -> io::Result<()> {
        let mut all = self.pinned()?;
        all.retain(|x| x.name != name);
        all.extend(p);
        all.sort_by(|a, b| a.name.cmp(&b.name));
        let text = serde_json::to_string_pretty(&all).map_err(|e| other(e.to_string()))? + "\n";
        cooee_core::write_private(&self.lock_path(), text.as_bytes())
    }

    /// What adding `source` would install: a first-party pack's name (at swag's pin, else at `rev` from swag), a local
    /// folder, or a git URL (at `rev`, else its default branch).
    ///
    /// # Errors
    /// When a name isn't a first-party pack's, or the name it would take isn't a theme's.
    pub fn plan(&self, source: &str, rev: Option<&str>) -> io::Result<Pack> {
        if rev.is_some_and(|r| r.starts_with('-')) || source.starts_with('-') {
            return Err(other("a source or --rev can't start with -"));
        }
        let first = self.swag.names();
        let pack = if first.contains(&source) {
            Pack {
                name: source.to_owned(),
                source: self.swag.url.clone(),
                path: format!("packs/{source}"),
                rev: rev.unwrap_or(&self.swag.commit).to_owned(),
                listed: rev.is_none_or(|r| r == self.swag.commit),
                text: String::new(),
            }
        } else if source.contains('/') || source.contains(':') {
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
        } else {
            return Err(other(format!(
                "{source} isn't a first-party theme ({}): add it by its git URL or folder",
                first.join(" ")
            )));
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
            text += "\nnot reviewed: this isn't a first-party theme at cooee's pin; look at its files before you add it";
        }
        Ok(Pack { text, ..pack })
    }

    /// Installs `pack`: copies it (a first-party pack from the build's checked copy, a git repo at its commit) into a
    /// staging folder, runs the theme gate on it, then moves it into `themes/` and pins it.
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
        fs::remove_dir_all(&stage).ok();
        let folder = Path::new(&pack.source);
        let commit = if pack.listed {
            self.vendored(&pack.name, &stage)?
        } else if folder.is_dir() {
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
            fs::remove_dir_all(&stage).ok();
        }
        done?;
        self.pin(
            &pack.name,
            Some(Pinned {
                name: pack.name.clone(),
                source: pack.source.clone(),
                commit,
                listed: pack.listed,
            }),
        )
    }

    fn vendored(&self, name: &str, stage: &Path) -> io::Result<String> {
        fs::create_dir_all(stage)?;
        for (file, bytes) in self.swag.files(name)? {
            fs::write(stage.join(file), bytes)?;
        }
        Ok(self.swag.commit.clone())
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
        let mut archive = cooee_core::runner::command("git")
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
        let untar = cooee_core::runner::command("tar")
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
}

pub(crate) fn routes(b: topcoat::router::RouterBuilder) -> topcoat::router::RouterBuilder {
    b.route(looks_json).route(look_art)
}

fn looks(cx: &Cx) -> impl Future<Output = Result<Looks, tokio::task::JoinError>> {
    let config = app_context::<Data>(cx).1.clone();
    tokio::task::spawn_blocking(move || Looks::load(config.as_deref(), None).0)
}

#[route(GET "/api/v1/looks")]
async fn looks_json(cx: &Cx) -> Result<Response> {
    Json(looks(cx).await?).into_response(cx)
}

path_param!(look);
path_param!(art);

const FRAMES: &[u8] = b"<style>.kf1,.f1{opacity:var(--o1,1)}.kf2,.f2{opacity:var(--o2,0)}</style>";
const CALM: &[u8] = b"<style>.h-busy,.h-held,.l-overcast,.l-rain{display:none}</style>";

fn with_rule(svg: &[u8], rule: &[u8]) -> Vec<u8> {
    let at = svg
        .windows(4)
        .position(|w| w == b"<svg")
        .and_then(|i| {
            svg.iter()
                .skip(i)
                .position(|&b| b == b'>')
                .map(|j| i + j + 1)
        })
        .unwrap_or(0);
    let (head, tail) = svg.split_at_checked(at).unwrap_or((&[], svg));
    [head, rule, tail].concat()
}

#[route(GET "/look/{look}/{art}")]
async fn look_art(cx: &Cx) -> Result<Response> {
    let (name, art) = (path_param::<Look>(cx).to_owned(), path_param::<Art>(cx));
    let looks = looks(cx).await?;
    let theme = looks.themes.get(&name);
    let svg = match art {
        "sprites.svg" => theme
            .and_then(|t| t.sprite_sheet.as_deref())
            .map(|b| with_rule(b, FRAMES)),
        "scene.svg" => theme
            .and_then(|t| t.scene_art.as_deref())
            .map(|b| with_rule(b, CALM)),
        _ => None,
    };
    match svg {
        Some(svg) => Ok(Response::builder()
            .header("Content-Type", "image/svg+xml")
            .body(Body::from(svg))?),
        None => plain(404, "no such theme art"),
    }
}
