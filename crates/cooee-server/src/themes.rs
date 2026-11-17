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

    /// Replaces each old full clone, `<data>/themes/<name>.git`, once its new form is in place (a first-party pack's
    /// is the build, any other's its shallow clone); returns the bytes freed.
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
            let placed = if pin.listed {
                Ok(())
            } else {
                self.plan(&pin.source, Some(&pin.commit)).and_then(|p| {
                    let p = Pack {
                        name: name.to_owned(),
                        ..p
                    };
                    let stage = self.dir.join(".themes-stage").join(name);
                    let fetched = self.fetch(&p, &stage);
                    fs::remove_dir_all(&stage).ok();
                    fetched.map(drop)
                })
            };
            match placed {
                Ok(()) => {
                    freed += size(&old);
                    fs::remove_dir_all(&old)?;
                }
                Err(e) => cooee_core::note!("cooee: theme {name} keeps its old clone for now: {e}"),
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
        fs::remove_dir_all(crate::look::last_good(&self.dir, name)).ok();
        self.pin(name, None)
    }

    /// Saves pack `name` as `files` (path in the pack, bytes): written to a staging folder, through the theme gate,
    /// then moved over `themes/<name>/` and kept as its last good copy; `false` when it is the same already.
    ///
    /// # Errors
    /// When the name is a built-in or first-party pack's, or the pack fails the gate.
    pub fn make(&self, name: &str, files: &[(String, Vec<u8>)]) -> io::Result<bool> {
        let _one = lock(&self.ops);
        valid(name)?;
        if name == "plain" || name == "starter" || self.swag.names().contains(&name) {
            return Err(other(format!(
                "{name} is a built-in theme's name: pick another"
            )));
        }
        let to = self.installed_dir(name);
        let same = files
            .iter()
            .all(|(f, b)| fs::read(to.join(f)).is_ok_and(|on| on == *b));
        if same && read_all(&to).len() == files.len() {
            return Ok(false);
        }
        let stage = self.dir.join(".themes-stage").join(name);
        fs::remove_dir_all(&stage).ok();
        for (file, bytes) in files {
            let path = stage.join(file);
            fs::create_dir_all(path.parent().unwrap_or(&stage))?;
            fs::write(path, bytes)?;
        }
        let old = self.dir.join(".themes-stage").join(format!("{name}.old"));
        fs::remove_dir_all(&old).ok();
        let done = gate(&stage).and_then(|()| {
            fs::create_dir_all(self.dir.join("themes"))?;
            if to.exists() {
                fs::rename(&to, &old)?;
            }
            fs::rename(&stage, &to)
        });
        fs::remove_dir_all(&stage).ok();
        fs::remove_dir_all(&old).ok();
        done?;
        let good = crate::look::last_good(&self.dir, name);
        fs::remove_dir_all(&good).ok();
        copy(&to, &good)?;
        self.pin(
            name,
            Some(Pinned {
                name: name.to_owned(),
                source: "made here".to_owned(),
                ..Pinned::default()
            }),
        )?;
        Ok(true)
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

fn read_all(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .flat_map(|e| {
            if e.path().is_dir() {
                read_all(&e.path())
            } else {
                vec![e.path()]
            }
        })
        .collect()
}

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
