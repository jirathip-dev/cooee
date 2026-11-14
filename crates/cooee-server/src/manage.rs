//! The plugin manager (docs/plugins.md): `cooee plugin add|update|remove|list` and Settings › Plugins. A plugin's
//! checkout is `<data>/plugins/<name>/repo`; each commit it runs is built in a folder of its own, `builds/<commit>` (a
//! git worktree), and goes live only once it built and its backend served its first tab's module. The one before stays
//! to roll back to. `cooee serve` runs each enabled backend as its child (no launchd job per plugin), its output in
//! `plugin.log` beside the checkout, and restarts one that exits, or fails 3 health checks 10 s apart in a row, with
//! backoff: 5 crashes in 10 minutes turn it off, with an alert.

use std::{
    collections::{BTreeMap, VecDeque},
    fmt::Write as _,
    fs, io,
    io::Write as _,
    net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream},
    os::unix::process::CommandExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use cooee_core::{
    Config, Plugin,
    plugins::{self, Listed, Locked, Manifest},
};
use serde::Serialize;

use crate::{
    Hub,
    hub::{lock, now},
    push::Note,
};

/// How long a new build's backend has to serve its first tab's module.
const HEALTH: Duration = Duration::from_secs(30);
/// How often (ms) a running backend gets a health GET of its first tab's module.
const CHECK_EVERY: u64 = 10_000;
/// How long a health GET waits for the answer.
const CHECK_WAIT: Duration = Duration::from_secs(2);
/// The failed health checks in a row that make a running backend hung: it is killed and restarted like a crash.
const MISSES: u32 = 3;

/// A backend's launcher (its run command is `$1`): a watcher ends the backend's process group once its stdin, a
/// pipe only cooee holds open, closes, so a backend never outlives the cooee (or the `Child`) that started it.
const BACKEND: &str =
    "exec 3<&0 </dev/null; (cat <&3 >/dev/null; kill -TERM 0) & exec 3<&-; exec sh -c \"$1\"";

/// Where a first-party plugin is improved instead.
pub const ISSUES: &str = "https://github.com/jirathip-dev/cooee/issues/new";

fn other(why: impl Into<String>) -> io::Error {
    io::Error::other(why.into())
}

/// What `add` or `update` would run, for the owner to confirm: the URL, the commit, the manifest (its exact build
/// and run commands), whether the plugin is in the index, the tools it needs that are missing, and all of that as
/// the confirm's text.
#[derive(Debug, Clone, Serialize)]
pub struct Plan {
    /// Its git URL.
    pub url: String,
    /// The commit it would run.
    pub commit: String,
    /// Its manifest at that commit.
    pub manifest: Manifest,
    /// It is in the index.
    pub listed: bool,
    /// The tools it needs that aren't on `PATH`.
    pub missing: Vec<String>,
    /// The confirm, for the owner.
    pub text: String,
}

/// An installed plugin as `list` and Settings › Plugins show it: its lock entry, its state and whether it is
/// first-party.
#[derive(Debug, Serialize)]
pub struct Row {
    /// Its lock entry.
    #[serde(flatten)]
    pub locked: Locked,
    /// running, crashed, off, a job in progress, …
    pub state: String,
    /// It is improved through an issue.
    pub first_party: bool,
    /// Where its screens go (#125), shown with its permissions when it's attached to a persona.
    pub places: Vec<plugins::Place>,
    /// Its manifest's `[permissions]`.
    pub permissions: plugins::Permissions,
}

/// A supervised backend: its process (none between a crash and its restart), the build it runs, its crashes, the
/// path its health checks GET, when the next is due and the ones failed in a row.
struct Run {
    child: Option<Child>,
    commit: String,
    module: String,
    check: u64,
    misses: u32,
    backoff: Backoff,
    next: u64,
    state: String,
}

/// The crash policy: a restart waits 1 s, then 2, 4 and 8; the 5th crash within 10 minutes turns the plugin off.
#[derive(Debug, Default)]
pub struct Backoff {
    crashes: VecDeque<u64>,
}

impl Backoff {
    /// A crash at `now` (ms): how long to wait before the restart, or none when the plugin goes off.
    pub fn crash(&mut self, now: u64) -> Option<u64> {
        self.crashes
            .retain(|&t| now.saturating_sub(t) < 10 * 60 * 1000);
        self.crashes.push_back(now);
        (self.crashes.len() < 5).then(|| 1000 << (self.crashes.len() - 1))
    }
}

/// The installed plugins of one instance, and their backends while `cooee serve` runs.
pub struct Manager {
    data: PathBuf,
    index: Vec<Listed>,
    /// The config's own plugins, whose names an installed tab can't take.
    fixed: Vec<Plugin>,
    /// cooee's URL, a backend's `COOEE`.
    url: Mutex<String>,
    runs: Mutex<BTreeMap<String, Run>>,
    /// The phone's installs and updates in progress (`…`), or how they failed.
    jobs: Mutex<BTreeMap<String, String>>,
    /// One change to the lock at a time.
    ops: Mutex<()>,
    stopping: AtomicBool,
}

fn git(dir: &Path, args: &[&str]) -> io::Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
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

fn path(p: &Path) -> &str {
    p.to_str().unwrap_or_default()
}

/// `sh -c cmd` in `dir`, its own process group, its output appended to `log`.
fn sh(cmd: &str, dir: &Path, log: &Path) -> io::Result<Command> {
    let out = fs::OpenOptions::new().create(true).append(true).open(log)?;
    let mut c = Command::new("sh");
    c.arg("-c")
        .arg(cmd)
        .current_dir(dir)
        .stdin(Stdio::null())
        .stdout(out.try_clone()?)
        .stderr(out)
        .process_group(0);
    Ok(c)
}

fn note(log: &Path, line: &str) {
    let f = fs::OpenOptions::new().create(true).append(true).open(log);
    let _ = f.and_then(|mut f| writeln!(f, "== cooee: {line}"));
}

/// Stops a backend and everything it started (its process group).
fn stop(child: &mut Child) {
    cooee_core::group::signal(child, "-TERM");
    let _ = child.kill();
    let _ = child.wait();
}

fn free_port() -> io::Result<u16> {
    Ok(TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?
        .local_addr()?
        .port())
}

fn up(port: u16) -> bool {
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok()
}

/// The token in `m`'s `attach_token` file, when it names one that can be read.
fn attach_token(m: &Manifest) -> Option<String> {
    let text = fs::read_to_string(m.attach_token_path()?).ok()?;
    Some(text.trim().to_owned()).filter(|t| !t.is_empty())
}

fn on_path(tool: &str) -> bool {
    Command::new("sh")
        .args(["-c", "command -v \"$1\"", "sh", tool])
        .stdout(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// A plugin's tabs as `[[plugins]]` entries on `port` with `token`; one with screens only gets one of its name and
/// no module (no tab).
fn entries(m: &Manifest, port: u16, token: &str) -> Vec<Plugin> {
    let tab = |t: &plugins::Tab| Plugin {
        name: t.name.clone(),
        label: t.label.clone(),
        url: format!("http://127.0.0.1:{port}"),
        module: t.module.clone(),
        token: token.to_owned(),
    };
    let screens = plugins::Tab {
        name: m.name.clone(),
        label: m.name.clone(),
        module: String::new(),
    };
    let tabs = if m.tabs.is_empty() {
        std::slice::from_ref(&screens)
    } else {
        &m.tabs
    };
    tabs.iter().map(tab).collect()
}

/// What a build's backend must serve to be healthy: its first tab's module, else its first place's screen.
fn first_path(m: &Manifest) -> String {
    let tab = m.tabs.first().map(|t| t.module.clone());
    let screen = m
        .places
        .iter()
        .map(|p| p.screen.clone())
        .find(|s| !s.is_empty());
    tab.or(screen).unwrap_or_default()
}

impl Manager {
    /// The instance's plugins: its data folder, its index (`plugin_index`, else the built-in one).
    ///
    /// # Errors
    /// When there is no data folder or the index can't be read.
    pub fn new(config: &Config) -> io::Result<Arc<Self>> {
        let data = config
            .data_dir()
            .ok_or_else(|| other("no data folder: set `data` or HOME"))?;
        Ok(Arc::new(Self {
            data,
            index: plugins::index(config.plugin_index.as_deref())?,
            fixed: config.plugins.clone(),
            url: Mutex::new(format!("http://127.0.0.1:{}", config.port)),
            runs: Mutex::default(),
            jobs: Mutex::default(),
            ops: Mutex::default(),
            stopping: AtomicBool::new(false),
        }))
    }

    /// The index.
    #[must_use]
    pub fn index(&self) -> &[Listed] {
        &self.index
    }

    fn dir(&self, name: &str) -> PathBuf {
        self.data.join("plugins").join(name)
    }

    fn repo(&self, name: &str) -> PathBuf {
        self.dir(name).join("repo")
    }

    fn build_dir(&self, name: &str, commit: &str) -> PathBuf {
        plugins::build_dir(&self.data, name, commit)
    }

    /// The plugin's log: its builds' and its backend's output.
    #[must_use]
    pub fn log(&self, name: &str) -> PathBuf {
        self.dir(name).join("plugin.log")
    }

    /// The installed plugins.
    ///
    /// # Errors
    /// When the lock can't be read.
    pub fn installed(&self) -> io::Result<Vec<Locked>> {
        plugins::read_lock(&self.data)
    }

    fn locked(&self, name: &str) -> io::Result<Locked> {
        let found = self.installed()?.into_iter().find(|l| l.name == name);
        found.ok_or_else(|| other(format!("{name} isn't installed")))
    }

    /// Changes plugin `name`'s lock entry.
    fn change(&self, name: &str, f: impl FnOnce(&mut Locked)) -> io::Result<()> {
        let mut all = self.installed()?;
        let l = all.iter_mut().find(|l| l.name == name);
        f(l.ok_or_else(|| other(format!("{name} isn't installed")))?);
        plugins::write_lock(&self.data, &all)
    }

    fn token(&self, l: &Locked) -> io::Result<String> {
        let entries = plugins::read_entries(&self.data)?;
        let found = entries.into_iter().find(|p| l.tabs.contains(&p.name));
        found
            .map(|p| p.token)
            .ok_or_else(|| other(format!("{} has no token", l.name)))
    }

    /// The tabs: the config's own plugins, then the installed ones'.
    #[must_use]
    pub fn tabs(&self) -> Vec<Plugin> {
        let mut all = self.fixed.clone();
        match plugins::read_entries(&self.data) {
            Ok(entries) => all.extend(entries),
            Err(e) => eprintln!("cooee: {e}"),
        }
        all
    }

    /// The checkout for `url`: plugin `name`'s, fetched, or else a new clone (`.clone-<stamp>`).
    fn checkout(&self, url: &str, name: Option<&str>) -> io::Result<PathBuf> {
        if let Some(repo) = name.map(|n| self.repo(n)).filter(|r| r.exists()) {
            git(&repo, &["remote", "set-url", "origin", url])?;
            git(&repo, &["fetch", "--quiet", "origin"])?;
            return Ok(repo);
        }
        let root = self.data.join("plugins");
        fs::create_dir_all(&root)?;
        let tmp = root.join(format!(".clone-{}", now()));
        git(&root, &["clone", "--quiet", "--", url, path(&tmp)])?;
        Ok(tmp)
    }

    /// What adding `source` would run: a name in the index (at its pinned commit, or `rev`) or a git URL (at its
    /// default branch, or `rev`). Clones it into its folder, but runs nothing.
    ///
    /// # Errors
    /// When it can't be fetched, has no such commit or no valid manifest, or a name isn't in the index.
    pub fn plan(&self, source: &str, rev: Option<&str>) -> io::Result<Plan> {
        if rev.is_some_and(|r| r.starts_with('-')) {
            return Err(other("a --rev can't start with -"));
        }
        let listed = self.index.iter().find(|l| l.name == source);
        let (url, rev) = match listed {
            Some(l) => (l.url.as_str(), rev.unwrap_or(&l.commit)),
            None if source.contains(':') || source.contains('/') => {
                (source, rev.unwrap_or("origin/HEAD"))
            }
            None => {
                return Err(other(format!(
                    "{source} isn't in the index: add it by its git URL"
                )));
            }
        };
        let repo = self.checkout(url, listed.map(|l| l.name.as_str()))?;
        let listed = listed.map(|l| l.name.as_str());
        let plan = Self::plan_at(&repo, url, rev, listed);
        if repo
            .file_name()
            .is_some_and(|n| n.to_string_lossy().starts_with(".clone-"))
        {
            // A new clone becomes its plugin's checkout, unless that is kept from before: then that one is fetched.
            let to = plan.as_ref().ok().map(|p| self.repo(&p.manifest.name));
            if let Some(to) = to.filter(|to| !to.exists()) {
                fs::create_dir_all(to.parent().unwrap_or(&to))?;
                fs::rename(&repo, &to)?;
            } else {
                fs::remove_dir_all(&repo)?;
                if let Ok(p) = plan {
                    let repo = self.checkout(url, Some(&p.manifest.name))?;
                    return Self::plan_at(&repo, url, rev, listed);
                }
            }
        }
        plan
    }

    /// The plan for `repo` at `rev`; `listed`: the index's name for it.
    fn plan_at(repo: &Path, url: &str, rev: &str, listed: Option<&str>) -> io::Result<Plan> {
        let commit = [rev.to_owned(), format!("origin/{rev}")]
            .iter()
            .find_map(|r| {
                git(
                    repo,
                    &[
                        "rev-parse",
                        "--verify",
                        "--quiet",
                        &format!("{r}^{{commit}}"),
                    ],
                )
                .ok()
            })
            .ok_or_else(|| other(format!("{url} has no commit {rev}")))?;
        let manifest = git(repo, &["show", &format!("{commit}:cooee-plugin.toml")])
            .map_err(|_| other(format!("{url} has no cooee-plugin.toml at {commit}")))?;
        let m = Manifest::parse(&manifest)?;
        if listed.is_some_and(|l| l != m.name) {
            return Err(other(format!(
                "{url}: its manifest names {}, not the index's name",
                m.name
            )));
        }
        let missing: Vec<String> = m.needs.iter().filter(|t| !on_path(t)).cloned().collect();
        let tabs: Vec<&str> = m.tabs.iter().map(|t| t.label.as_str()).collect();
        let mut text = format!(
            "{} {} from {url}\ncommit {commit}\nbuild: {}\nrun: {}\n  (its backend runs as you on this Mac)\ntabs: {}",
            m.name,
            m.version,
            m.build,
            m.run,
            tabs.join(", ")
        );
        if !m.places.is_empty() {
            let actions = m.permissions.actions.join(", ");
            let _ = write!(
                text,
                "\nscreens: {} places; actions: {actions}",
                m.places.len()
            );
        }
        if !m.profiles.is_empty() {
            let labels: Vec<String> = m
                .profiles
                .iter()
                .map(|p| format!("{} ({})", p.label, p.agent))
                .collect();
            let _ = write!(text, "\nagent profiles: {}", labels.join(", "));
        }
        if !m.needs.is_empty() {
            let _ = write!(text, "\nneeds: {}", m.needs.join(", "));
        }
        if !missing.is_empty() {
            let _ = write!(text, "\nmissing: {} (install it first)", missing.join(", "));
        }
        if listed.is_none() {
            text += "\nnot reviewed: this plugin isn't in cooee's index; read its code before you add it";
        }
        Ok(Plan {
            url: url.to_owned(),
            commit,
            manifest: m,
            missing,
            text,
            listed: listed.is_some(),
        })
    }

    /// Builds plugin `name` at `commit` in a new folder and checks its backend serves its first tab's module with
    /// `token`; a failed build's folder goes. Returns the manifest at that commit.
    fn build(&self, name: &str, commit: &str, token: &str) -> io::Result<Manifest> {
        let (repo, dir, log) = (
            self.repo(name),
            self.build_dir(name, commit),
            self.log(name),
        );
        let gone = |dir: &Path| {
            let _ = git(&repo, &["worktree", "remove", "--force", path(dir)]);
            let _ = fs::remove_dir_all(dir);
            git(&repo, &["worktree", "prune"])
        };
        gone(&dir)?;
        git(
            &repo,
            &[
                "worktree",
                "add",
                "--quiet",
                "--detach",
                "--force",
                path(&dir),
                commit,
            ],
        )?;
        let built =
            Manifest::parse(&fs::read_to_string(dir.join("cooee-plugin.toml"))?).and_then(|m| {
                note(&log, &format!("build {commit}: {}", m.build));
                let status = sh(&m.build, &dir, &log)?.status()?;
                if !status.success() {
                    return Err(other(format!("the build failed ({status}): see its log")));
                }
                self.health(&dir, &m, token, &log)?;
                Ok(m)
            });
        if built.is_err() {
            gone(&dir)?;
        }
        built
    }

    fn backend(
        &self,
        dir: &Path,
        m: &Manifest,
        port: u16,
        token: &str,
        log: &Path,
    ) -> io::Result<Command> {
        let mut c = sh(BACKEND, dir, log)?;
        c.args(["sh", &m.run])
            .stdin(Stdio::piped())
            .env("PORT", port.to_string())
            .env("COOEE_PLUGIN_TOKEN", token)
            .env("COOEE", lock(&self.url).as_str());
        Ok(c)
    }

    /// Runs the build's backend on a spare port until it serves its first tab's module (200), then stops it; an
    /// `attach` backend that is up is asked instead, and nothing runs.
    fn health(&self, dir: &Path, m: &Manifest, token: &str, log: &Path) -> io::Result<()> {
        let attached = m.attach_port().filter(|&p| up(p));
        let port = attached.map_or_else(free_port, Ok)?;
        let mut child = match attached {
            Some(_) => None,
            None => Some(self.backend(dir, m, port, token, log)?.spawn()?),
        };
        let p = Plugin {
            name: m.name.clone(),
            label: String::new(),
            url: format!("http://127.0.0.1:{port}"),
            module: first_path(m),
            token: token.to_owned(),
        };
        let started = Instant::now();
        let answer = loop {
            if let Some(Ok(Some(s))) = child.as_mut().map(Child::try_wait) {
                break Err(other(format!("its backend exited ({s}): see its log")));
            }
            let reply = crate::plugin::call(&p, "GET", &p.module, &[], &[], HEALTH);
            match reply.map(|r| r.status) {
                Ok(200) => break Ok(()),
                got if started.elapsed() > HEALTH => {
                    let got = got.map_or_else(|e| e.to_string(), |s| format!("answered {s}"));
                    break Err(other(format!(
                        "its backend didn't serve {}: {got}",
                        p.module
                    )));
                }
                _ => thread::sleep(Duration::from_millis(100)),
            }
        };
        if let Some(c) = &mut child {
            stop(c);
        }
        answer
    }

    /// Adds the plugin `plan` describes: builds it, checks it, writes its tabs' entries with a fresh token and
    /// records it in the lock; `cooee serve` starts its backend.
    ///
    /// # Errors
    /// When it is installed already, needs a missing tool, a tab's name is taken, or it doesn't build or answer.
    pub fn add(&self, plan: &Plan) -> io::Result<()> {
        let _one = lock(&self.ops);
        let m = &plan.manifest;
        if self.installed()?.iter().any(|l| l.name == m.name) {
            return Err(other(format!(
                "{} is installed already: update it instead",
                m.name
            )));
        }
        if !plan.missing.is_empty() {
            return Err(other(format!(
                "{} needs {} first",
                m.name,
                plan.missing.join(", ")
            )));
        }
        let used = self.tabs();
        if let Some(t) = m
            .tabs
            .iter()
            .find(|t| used.iter().any(|p| p.name == t.name))
        {
            return Err(other(format!("a tab named {} is there already", t.name)));
        }
        let token = match attach_token(m) {
            Some(t) => t,
            None => cooee_core::sha256(&cooee_core::random(32)?),
        };
        let m = self.build(&m.name, &plan.commit, &token)?;
        let port = m.attach_port().map_or_else(free_port, Ok)?;
        let mut all = plugins::read_entries(&self.data)?;
        all.extend(entries(&m, port, &token));
        crate::plugin::check(&[self.fixed.as_slice(), &all].concat())?;
        plugins::write_entries(&self.data, &all)?;
        let mut lock = self.installed()?;
        lock.push(Locked {
            name: m.name.clone(),
            url: plan.url.clone(),
            commit: plan.commit.clone(),
            version: m.version.clone(),
            listed: plan.listed,
            tabs: entries(&m, port, &token)
                .into_iter()
                .map(|p| p.name)
                .collect(),
            port,
            enabled: true,
            ..Locked::default()
        });
        plugins::write_lock(&self.data, &lock)
    }

    /// What updating plugin `name` would run: the index's pinned commit, or for a plugin added by URL the latest on
    /// its default branch.
    ///
    /// # Errors
    /// When it isn't installed or can't be fetched.
    pub fn plan_update(&self, name: &str) -> io::Result<Plan> {
        let l = self.locked(name)?;
        let pin = self.index.iter().find(|x| x.name == name && l.listed);
        let repo = self.checkout(&l.url, Some(name))?;
        let rev = pin.map_or("origin/HEAD", |x| &x.commit);
        Self::plan_at(&repo, &l.url, rev, pin.map(|x| x.name.as_str()))
    }

    /// Builds `plan`'s commit in a new folder and switches to it once it built and answered; otherwise the old one
    /// keeps running.
    ///
    /// # Errors
    /// When it is at that commit already, or the new build fails (the old one stays).
    pub fn update(&self, plan: &Plan) -> io::Result<()> {
        let _one = lock(&self.ops);
        let name = &plan.manifest.name;
        let l = self.locked(name)?;
        if l.commit == plan.commit {
            return Err(other(format!("{name} is at {} already", &l.commit[..7])));
        }
        let m = self.build(name, &plan.commit, &self.token(&l)?)?;
        self.switch(&l, &plan.commit, &m)
    }

    /// Makes `commit`'s build (built already) the one that runs, keeping the one before to roll back to, and
    /// removes any older builds.
    fn switch(&self, l: &Locked, commit: &str, m: &Manifest) -> io::Result<()> {
        let token = self.token(l)?;
        let mut all = plugins::read_entries(&self.data)?;
        all.retain(|p| !l.tabs.contains(&p.name));
        all.extend(entries(m, l.port, &token));
        crate::plugin::check(&[self.fixed.as_slice(), &all].concat())?;
        plugins::write_entries(&self.data, &all)?;
        self.change(&l.name, |l| {
            l.previous = Some(std::mem::replace(&mut l.commit, commit.to_owned()));
            l.version.clone_from(&m.version);
            l.tabs = entries(m, l.port, "").into_iter().map(|p| p.name).collect();
        })?;
        let keep = [commit, l.commit.as_str()];
        for e in fs::read_dir(self.dir(&l.name).join("builds"))?.flatten() {
            if !keep.iter().any(|k| e.file_name() == *k) {
                let _ = git(
                    &self.repo(&l.name),
                    &["worktree", "remove", "--force", path(&e.path())],
                );
            }
        }
        Ok(())
    }

    /// Goes back to the build before the one running (which stays, to go forward again).
    ///
    /// # Errors
    /// When there is no build before.
    pub fn rollback(&self, name: &str) -> io::Result<()> {
        let _one = lock(&self.ops);
        let l = self.locked(name)?;
        let prev = l
            .previous
            .clone()
            .ok_or_else(|| other(format!("{name} has no build to roll back to")))?;
        let manifest = fs::read_to_string(self.build_dir(name, &prev).join("cooee-plugin.toml"))?;
        self.switch(&l, &prev, &Manifest::parse(&manifest)?)
    }

    /// Removes plugin `name`: `cooee serve` stops it, its entries go; its folder stays unless `purge`.
    ///
    /// # Errors
    /// When it isn't installed or the files can't be written.
    pub fn remove(&self, name: &str, purge: bool) -> io::Result<()> {
        let _one = lock(&self.ops);
        let l = self.locked(name)?;
        let mut all = plugins::read_entries(&self.data)?;
        all.retain(|p| !l.tabs.contains(&p.name));
        plugins::write_entries(&self.data, &all)?;
        let mut lock = self.installed()?;
        lock.retain(|x| x.name != name);
        plugins::write_lock(&self.data, &lock)?;
        if purge {
            fs::remove_dir_all(self.dir(name))?;
        }
        Ok(())
    }

    /// Turns plugin `name` on or off.
    ///
    /// # Errors
    /// When it isn't installed.
    pub fn enable(&self, name: &str, on: bool) -> io::Result<()> {
        let _one = lock(&self.ops);
        self.change(name, |l| {
            l.enabled = on;
            l.note.clear();
        })?;
        if let Some(mut c) = lock(&self.runs).remove(name).and_then(|r| r.child) {
            stop(&mut c);
        }
        Ok(())
    }

    /// Puts plugin `name`'s checkout on the `cooee-improve` branch (from the commit running, unless thread
    /// `improving` is on it already) for an agent to work in; returns the checkout and the manifest.
    ///
    /// # Errors
    /// For a first-party plugin (improved through an issue), or when the branch can't be made.
    pub fn improve(&self, name: &str) -> io::Result<(PathBuf, Manifest)> {
        let l = self.locked(name)?;
        if self.index.iter().any(|x| x.name == name && x.first_party) {
            return Err(other(format!(
                "{name} is first-party: improve it via an issue, {ISSUES}"
            )));
        }
        let repo = self.repo(name);
        if l.improving.is_none() {
            git(
                &repo,
                &["checkout", "--quiet", "-B", "cooee-improve", &l.commit],
            )?;
        }
        let manifest =
            fs::read_to_string(self.build_dir(name, &l.commit).join("cooee-plugin.toml"))?;
        Ok((repo, Manifest::parse(&manifest)?))
    }

    /// Records the thread improving plugin `name` (none: done).
    ///
    /// # Errors
    /// When it isn't installed.
    pub fn improving(&self, name: &str, thread: Option<String>) -> io::Result<()> {
        let _one = lock(&self.ops);
        self.change(name, |l| l.improving = thread)
    }

    /// The owner's approval of the improvement: commits the branch's changes, builds that commit and swaps it in;
    /// a failed build leaves the old one running.
    ///
    /// # Errors
    /// When nothing changed, or the build fails.
    pub fn approve(&self, name: &str) -> io::Result<()> {
        let _one = lock(&self.ops);
        let l = self.locked(name)?;
        let repo = self.repo(name);
        if !git(&repo, &["status", "--porcelain"])?.is_empty() {
            git(&repo, &["add", "-A"])?;
            let msg = format!("improve {name}");
            let commit = [
                "-c",
                "user.name=cooee",
                "-c",
                "user.email=cooee@localhost",
                "commit",
                "--quiet",
                "-m",
                &msg,
            ];
            git(&repo, &commit)?;
        }
        let commit = git(&repo, &["rev-parse", "HEAD"])?;
        if commit == l.commit {
            return Err(other(format!(
                "nothing changed on {name}'s cooee-improve branch yet"
            )));
        }
        let m = self.build(name, &commit, &self.token(&l)?)?;
        self.switch(&l, &commit, &m)?;
        self.change(name, |l| l.improving = None)
    }

    /// The installed plugins with their state.
    ///
    /// # Errors
    /// When the lock can't be read.
    pub fn list(&self) -> io::Result<Vec<Row>> {
        let rows = self.installed()?.into_iter().map(|l| {
            let state = if let Some(job) = lock(&self.jobs).get(&l.name) {
                job.clone()
            } else if !l.enabled {
                if l.note.is_empty() {
                    "off".to_owned()
                } else {
                    l.note.clone()
                }
            } else if let Some(run) = lock(&self.runs).get(&l.name) {
                run.state.clone()
            } else {
                (if up(l.port) { "running" } else { "not running" }).to_owned()
            };
            let first_party = self.index.iter().any(|x| x.name == l.name && x.first_party);
            let m = plugins::manifest(&self.data, &l).ok();
            let (places, permissions) = m.map(|m| (m.places, m.permissions)).unwrap_or_default();
            Row {
                locked: l,
                state,
                first_party,
                places,
                permissions,
            }
        });
        Ok(rows.collect())
    }

    /// The phone's installs and updates: in progress (`…`) or failed, by plugin.
    #[must_use]
    pub fn jobs(&self) -> BTreeMap<String, String> {
        lock(&self.jobs).clone()
    }

    /// Runs `f` for plugin `name` on a thread of its own, its progress `what…` in [`Manager::jobs`] (and its
    /// failure, after).
    ///
    /// # Errors
    /// When the plugin has a job running already.
    pub fn job(
        self: &Arc<Self>,
        name: &str,
        what: &str,
        f: impl FnOnce(&Self) -> io::Result<()> + Send + 'static,
    ) -> io::Result<()> {
        let mut jobs = lock(&self.jobs);
        if jobs.get(name).is_some_and(|j| j.ends_with('…')) {
            return Err(other(format!("{name} is busy: {}", jobs[name])));
        }
        jobs.insert(name.to_owned(), format!("{what}…"));
        let (m, name) = (Arc::clone(self), name.to_owned());
        thread::spawn(move || {
            let done = f(&m);
            let mut jobs = lock(&m.jobs);
            match done {
                Ok(()) => jobs.remove(&name),
                Err(e) => jobs.insert(name, format!("failed: {e}")),
            };
        });
        Ok(())
    }

    /// Starts plugin `l`'s backend from its build; returns it and the path its health checks GET: none when its
    /// `attach` backend is up.
    fn spawn(&self, l: &Locked) -> io::Result<Option<(Child, String)>> {
        let dir = self.build_dir(&l.name, &l.commit);
        let m = Manifest::parse(&fs::read_to_string(dir.join("cooee-plugin.toml"))?)?;
        if m.attach_port().is_some_and(up) {
            return Ok(None);
        }
        let log = self.log(&l.name);
        note(&log, &format!("start {} on port {}", l.commit, l.port));
        let child = self
            .backend(&dir, &m, l.port, &self.token(l)?, &log)?
            .spawn()?;
        Ok(Some((child, first_path(&m))))
    }

    /// The health checks due at `now` (ms), run without holding the backends: each running plugin's name and
    /// whether its backend served its first tab's module (200) within [`CHECK_WAIT`].
    fn checks(&self, installed: &[Locked], now: u64) -> Vec<(String, bool)> {
        let due: Vec<_> = lock(&self.runs)
            .iter()
            .filter(|(_, r)| r.child.is_some() && now >= r.check)
            .map(|(n, r)| (n.clone(), r.module.clone()))
            .collect();
        let check = |(name, module): (String, String)| {
            let l = installed.iter().find(|l| l.name == name)?;
            let p = Plugin {
                name,
                label: String::new(),
                url: format!("http://127.0.0.1:{}", l.port),
                module,
                token: self.token(l).ok()?,
            };
            let reply = crate::plugin::call(&p, "GET", &p.module, &[], &[], CHECK_WAIT);
            Some((p.name, reply.is_ok_and(|r| r.status == 200)))
        };
        due.into_iter().filter_map(check).collect()
    }

    /// One pass of supervision at `now` (ms): stops the backends removed, turned off or switched to another build;
    /// starts the enabled ones not running once their backoff is over; notes the ones that exited or are hung (failed
    /// 3 health checks in a row, one every 10 s) and kills the hung ones. Returns the
    /// alerts: a plugin turned off after its 5th crash in 10 minutes.
    pub fn tick(&self, now: u64) -> Vec<String> {
        if self.stopping.load(Ordering::SeqCst) {
            return Vec::new();
        }
        let installed = self.installed().unwrap_or_default();
        let checks = self.checks(&installed, now);
        let mut runs = lock(&self.runs);
        runs.retain(|name, run| {
            let keep = installed
                .iter()
                .any(|l| &l.name == name && l.enabled && l.commit == run.commit);
            if let Some(c) = run.child.as_mut().filter(|_| !keep) {
                stop(c);
            }
            keep
        });
        let mut alerts = Vec::new();
        for l in installed.iter().filter(|l| l.enabled) {
            let run = runs.entry(l.name.clone()).or_insert_with(|| Run {
                child: None,
                commit: l.commit.clone(),
                module: String::new(),
                check: 0,
                misses: 0,
                backoff: Backoff::default(),
                next: 0,
                state: "starting".to_owned(),
            });
            let mut crashed = match run.child.as_mut().map(Child::try_wait) {
                Some(Ok(None)) | None => None,
                Some(exit) => Some(
                    exit.ok()
                        .flatten()
                        .map_or("it exited".to_owned(), |s| s.to_string()),
                ),
            };
            if let Some(c) = run.child.as_mut().filter(|_| crashed.is_none())
                && let Some(&(_, ok)) = checks.iter().find(|(n, _)| n == &l.name)
            {
                run.check = now + CHECK_EVERY;
                run.misses = if ok { 0 } else { run.misses + 1 };
                if run.misses >= MISSES {
                    stop(c);
                    crashed = Some(format!("hung, {MISSES} health checks in a row failed"));
                }
            }
            if crashed.is_none() && run.child.is_none() && now >= run.next {
                match self.spawn(l) {
                    Ok(Some((c, module))) => {
                        run.child = Some(c);
                        run.module = module;
                        run.check = now + CHECK_EVERY;
                        run.misses = 0;
                        "running".clone_into(&mut run.state);
                    }
                    Ok(None) => "attached".clone_into(&mut run.state),
                    Err(e) => crashed = Some(e.to_string()),
                }
            }
            let Some(how) = crashed else { continue };
            run.child = None;
            note(&self.log(&l.name), &format!("the backend stopped: {how}"));
            if let Some(wait) = run.backoff.crash(now) {
                run.next = now + wait;
                run.state = format!("crashed ({how}), restarting");
                continue;
            }
            let why = "off after 5 crashes in 10 minutes".to_owned();
            let _one = lock(&self.ops);
            let _ = self.change(&l.name, |l| {
                l.enabled = false;
                l.note.clone_from(&why);
            });
            alerts.push(format!(
                "plugin {}: {why}; its log is in Settings › Plugins",
                l.name
            ));
        }
        alerts
    }

    /// Supervises the backends for `hub` on a thread of its own, every half second: its tabs follow the installed
    /// plugins, and its push sends the alerts. Backends get `url` as cooee's.
    pub fn supervise(self: &Arc<Self>, hub: Arc<Hub>, url: String) {
        *lock(&self.url) = url;
        let m = Arc::clone(self);
        thread::spawn(move || {
            loop {
                hub.set_plugins(m.tabs());
                for body in m.tick(now()) {
                    let tag = "plugins".to_owned();
                    let url = "/#settings/plugins".to_owned();
                    let title = "cooee".to_owned();
                    hub.push().send(&Note {
                        title,
                        body,
                        tag,
                        url,
                        item: 0,
                    });
                }
                thread::sleep(Duration::from_millis(500));
            }
        });
    }

    /// Stops every backend, for good (`cooee serve` is ending).
    pub fn stop_all(&self) {
        self.stopping.store(true, Ordering::SeqCst);
        for run in lock(&self.runs).values_mut() {
            if let Some(c) = &mut run.child {
                stop(c);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: u64 = 60 * 1000;

    #[test]
    fn backoff_doubles_and_the_fifth_crash_in_ten_minutes_turns_it_off() {
        let mut b = Backoff::default();
        let waits: Vec<_> = [0, 1, 2, 3, 4].map(|m| b.crash(m * MIN)).into();
        assert_eq!(
            waits,
            [Some(1000), Some(2000), Some(4000), Some(8000), None]
        );
        // Crashes 11 minutes apart never add up to 5.
        let mut b = Backoff::default();
        assert!((0..20).all(|i| b.crash(i * 11 * MIN) == Some(1000)));
    }

    /// Plugin `x` installed in a fresh data folder `cooee-<test>-<pid>`, its backend `run`, on `port`: the folder,
    /// its build and the manager.
    fn installed(test: &str, run: &str, port: u16) -> (PathBuf, PathBuf, Arc<Manager>) {
        let data = std::env::temp_dir().join(format!("cooee-{test}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&data);
        let build = data.join("plugins/x/builds/c0");
        fs::create_dir_all(&build).unwrap();
        let manifest = format!(
            "name = \"x\"\nversion = \"1\"\nbuild = \"true\"\nrun = \"{run}\"\n\
             [[tabs]]\nname = \"x\"\nlabel = \"X\"\nmodule = \"/tab.js\"\n"
        );
        fs::write(build.join("cooee-plugin.toml"), manifest).unwrap();
        let tab = Plugin {
            name: "x".into(),
            label: "X".into(),
            url: format!("http://127.0.0.1:{port}"),
            module: "/tab.js".into(),
            token: "0123456789abcdef".into(),
        };
        plugins::write_entries(&data, &[tab]).unwrap();
        let x = Locked {
            name: "x".into(),
            commit: "c0".into(),
            tabs: vec!["x".into()],
            port,
            enabled: true,
            ..Locked::default()
        };
        plugins::write_lock(&data, &[x]).unwrap();
        let config = Config {
            data: Some(data.clone()),
            ..Config::default()
        };
        (data, build, Manager::new(&config).unwrap())
    }

    fn runs(build: &Path) -> usize {
        fs::read_to_string(build.join("runs"))
            .unwrap_or_default()
            .lines()
            .count()
    }

    /// A backend that stays alive on a port where nothing answers is killed after 3 failed checks and restarted.
    #[test]
    fn the_supervisor_restarts_a_hung_backend_after_three_failed_checks() {
        let silent = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = silent.local_addr().unwrap().port();
        let (data, build, m) = installed("hung", "echo ran >> runs; exec sleep 600", port);
        m.tick(0);
        thread::sleep(Duration::from_millis(200));
        for at in [1, 2] {
            m.tick(at * CHECK_EVERY);
            assert_eq!(lock(&m.runs)["x"].state, "running");
        }
        m.tick(3 * CHECK_EVERY);
        assert!(lock(&m.runs)["x"].state.contains("hung"));
        m.tick(3 * CHECK_EVERY + 1000);
        thread::sleep(Duration::from_millis(200));
        assert_eq!(runs(&build), 2);
        assert_eq!(lock(&m.runs)["x"].state, "running");
        let log = fs::read_to_string(data.join("plugins/x/plugin.log")).unwrap();
        assert!(
            log.contains("the backend stopped: hung, 3 health checks in a row failed"),
            "{log}"
        );
        assert_eq!(log.matches("== cooee: start c0").count(), 2, "{log}");
        m.stop_all();
        fs::remove_dir_all(&data).unwrap();
    }

    /// A backend that answers its health checks is never restarted by them.
    #[test]
    fn the_supervisor_never_restarts_a_healthy_backend() {
        let ok = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = ok.local_addr().unwrap().port();
        let served = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let count = Arc::clone(&served);
        thread::spawn(move || {
            for mut s in ok.incoming().flatten() {
                let mut buf = [0; 1024];
                let _ = std::io::Read::read(&mut s, &mut buf);
                let _ = s.write_all(b"HTTP/1.0 200 OK\r\nContent-Length: 0\r\n\r\n");
                count.fetch_add(1, Ordering::SeqCst);
            }
        });
        let (data, build, m) = installed("healthy", "echo ran >> runs; exec sleep 600", port);
        for at in 0..=60 {
            m.tick(at * CHECK_EVERY);
        }
        thread::sleep(Duration::from_millis(200));
        assert_eq!(served.load(Ordering::SeqCst), 60);
        assert_eq!(runs(&build), 1);
        assert_eq!(lock(&m.runs)["x"].state, "running");
        let log = fs::read_to_string(data.join("plugins/x/plugin.log")).unwrap();
        assert!(!log.contains("stopped"), "{log}");
        m.stop_all();
        fs::remove_dir_all(&data).unwrap();
    }

    #[test]
    fn the_supervisor_restarts_a_crashing_backend_then_turns_it_off_with_an_alert() {
        let (data, build, m) = installed("supervise", "echo ran >> runs; exit 3", 1);
        let runs = || runs(&build);
        // The fake clock: each pass is a second after the last restart's wait, so nothing waits for real but
        // the backend's own exit.
        let mut at = 0;
        let mut alerts = Vec::new();
        for _ in 0..200 {
            alerts = m.tick(at);
            if !alerts.is_empty() {
                break;
            }
            let next = lock(&m.runs).get("x").map_or(at, |r| r.next);
            at = next.max(at) + 1;
            thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(runs(), 5, "{alerts:?}");
        assert_eq!(
            alerts,
            ["plugin x: off after 5 crashes in 10 minutes; its log is in Settings › Plugins"]
        );
        let x = &m.installed().unwrap()[0];
        assert!(!x.enabled && x.note.contains("5 crashes"), "{x:?}");
        assert!(m.list().unwrap()[0].state.contains("5 crashes"));
        // Off, it stays off.
        m.tick(at + 60 * MIN);
        assert_eq!(runs(), 5);
        let log = fs::read_to_string(data.join("plugins/x/plugin.log")).unwrap();
        assert_eq!(log.matches("the backend stopped").count(), 5, "{log}");
        fs::remove_dir_all(&data).unwrap();
    }
}
