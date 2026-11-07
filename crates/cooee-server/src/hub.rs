//! The threads, live: each thread's chat, its numbered page events (kept in a ring for replay) and their
//! subscribers, fed by one shed `events` stream for every terminal: cooee's own, and the Workflow sessions other
//! tools start, known by their labels (#33).

use std::{
    collections::{BTreeMap, VecDeque},
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use cooee_core::{
    Agent, Body, Change, Chat, Item, Plugin, Shed, ShedError, Store, StoreError, Summary, Thread,
    Turn, Workflow,
    shed_proto::{self, ApprovalOption, Event, TerminalInfo},
};
use serde::Serialize;
use serde_json::json;
use tokio::sync::broadcast;

use crate::push::{Note, Push, note};

/// How many page events a thread keeps for a reconnecting page.
const RING: usize = 4096;

/// The label every cooee terminal carries.
const LABEL: &str = "cooee";

/// A working turn with no shed event for this long is surfaced as needs-you (herdsman#444), in milliseconds.
const STUCK: u64 = 30 * 60 * 1000;

/// How often the installed agents are checked again and stuck turns looked for.
const TICK: Duration = Duration::from_secs(30);

/// Why a thread action failed.
#[derive(Debug, thiserror::Error)]
pub enum HubError {
    /// No such thread.
    #[error("no thread {0}")]
    NoThread(String),
    /// The request can't be done (a folder outside the roots, a closed thread, an empty message).
    #[error("{0}")]
    Refused(String),
    /// shed failed.
    #[error(transparent)]
    Shed(#[from] ShedError),
    /// The store failed.
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// One page event: its number in the thread, its SSE event name and its JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Out {
    /// Counts up from 1 per thread since this server started.
    pub seq: u64,
    /// `item`, `text` or `state`.
    pub event: &'static str,
    /// The JSON data.
    pub data: String,
}

/// A thread in memory.
struct Live {
    meta: Thread,
    chat: Chat,
    /// The newest shed event applied.
    applied: u64,
    seq: u64,
    ring: VecDeque<Out>,
    tx: broadcast::Sender<Out>,
    /// The state the last published change left (a push goes on a change of state only).
    was: String,
    /// When its last shed event came (or it was loaded), in Unix milliseconds.
    moved: u64,
    /// Its working turn was surfaced as stuck; the next shed event clears it.
    stuck: bool,
    /// The first message of a start that answered at once ([`Hub::create_later`]), until shed takes it.
    pending: Option<String>,
}

/// A slice of a thread's items, with the turn in progress and the page event it is current to.
#[derive(Debug, Clone, Serialize)]
pub struct Slice {
    /// The items, oldest first.
    pub items: Vec<Item>,
    /// How many older items there are.
    pub skipped_before: usize,
    /// How many newer items there are.
    pub skipped_after: usize,
    /// The turn in progress.
    pub turn: Turn,
    /// Resume the event stream after this (`<boot>.<seq>`).
    pub cursor: String,
    /// The first message while its thread connects, not yet an item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending: Option<Pending>,
}

/// A thread's first message while it connects: its text and the start's id (the item it becomes has it).
#[derive(Debug, Clone, Serialize)]
pub struct Pending {
    /// The message.
    pub text: String,
    /// The start's `random_id`.
    pub random_id: String,
}

/// What a page subscription starts with: replayed events, or a reset (the page reloads its slice).
pub enum Resume {
    /// The events after the page's cursor.
    Replay(Vec<Out>),
    /// The cursor is from another run or too old.
    Reset,
}

/// The threads and the shed they run on.
pub struct Hub {
    shed: Shed,
    store: Store,
    push: Push,
    /// The server's default theme, whose words a push says.
    theme: cooee_web::Theme,
    roots: Mutex<Vec<PathBuf>>,
    /// The config file the roots are saved in.
    config: Option<PathBuf>,
    plugins: Mutex<Vec<Plugin>>,
    /// `shed agents`, as last checked: a page load never runs the CLI itself (herdsman#337).
    agents: Mutex<Result<Vec<Agent>, String>>,
    boot: String,
    threads: Mutex<BTreeMap<String, Arc<Mutex<Live>>>>,
    /// The stamp of the newest thread id handed out.
    last: Mutex<u64>,
}

pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

pub(crate) fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Settings › Folders' view of one folder.
#[derive(Debug, Serialize)]
pub struct Listing {
    /// The folder shown, resolved.
    pub path: PathBuf,
    /// The home folder, resolved: browsing stops there.
    pub home: PathBuf,
    /// The names of the folders inside it.
    pub dirs: Vec<String>,
    /// The roots.
    pub roots: Vec<PathBuf>,
}

/// The folders right inside `dir` (hidden ones if `hidden`; never symlinks), sorted.
fn dirs(dir: &Path, hidden: bool) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter(|e| hidden || !e.file_name().to_string_lossy().starts_with('.'))
        .map(|e| e.path())
        .collect();
    out.sort();
    out
}

/// `path` (default the home folder), if it is a folder inside the home folder, resolved but spelled from `$HOME`
/// (as `~` shortens it); and `$HOME`.
fn in_home(path: Option<&Path>) -> Result<(PathBuf, PathBuf), HubError> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    let real = home.canonicalize().unwrap_or_default();
    let path = path.unwrap_or(&home);
    match path.canonicalize() {
        Ok(p) if p.is_dir() && p.starts_with(&real) && real.has_root() => {
            let rest = p.strip_prefix(&real).unwrap_or(&p);
            let p = if rest.as_os_str().is_empty() {
                home.clone()
            } else {
                home.join(rest)
            };
            Ok((p, home))
        }
        _ => Err(HubError::Refused(format!(
            "{} is not a folder in your home folder",
            path.display()
        ))),
    }
}

fn outside(folder: &Path) -> HubError {
    HubError::Refused(format!(
        "{} is not inside a configured root",
        folder.display()
    ))
}

fn refused(path: &Path, e: &io::Error) -> HubError {
    HubError::Refused(format!("{}: {e}", path.display()))
}

impl Live {
    fn new(meta: Thread, chat: Chat) -> Self {
        Self {
            applied: chat.seq(),
            was: chat.turn.state.clone(),
            meta,
            chat,
            seq: 0,
            ring: VecDeque::new(),
            tx: broadcast::channel(RING).0,
            moved: now(),
            stuck: false,
            pending: None,
        }
    }

    fn emit(&mut self, event: &'static str, data: String) {
        self.seq += 1;
        let out = Out {
            seq: self.seq,
            event,
            data,
        };
        if self.ring.len() == RING {
            self.ring.pop_front();
        }
        self.ring.push_back(out.clone());
        let _ = self.tx.send(out);
    }

    /// Stores and then publishes `changes`, pushing the ones the owner should hear of.
    fn publish(&mut self, hub: &Hub, changes: Vec<Change>) -> Result<(), StoreError> {
        let store = &hub.store;
        for change in changes {
            let watched = self.tx.receiver_count() > 0;
            if let Some(n) = note(
                &self.meta, &self.chat, &change, &self.was, watched, &hub.theme,
            ) {
                hub.push.send(&n);
            }
            if let Change::State(turn) = &change {
                self.was.clone_from(&turn.state);
            }
            match change {
                Change::Item(item) => {
                    store.append(&self.meta.id, &item)?;
                    self.emit("item", json!(item).to_string());
                }
                Change::Text(text) => self.emit("text", json!({ "text": text }).to_string()),
                Change::State(turn) => self.emit("state", json!(turn).to_string()),
            }
        }
        Ok(())
    }
}

impl Hub {
    /// Loads every stored thread, re-attaches the open ones to their shed terminals by name, and starts
    /// following shed's events. A push speaks `theme`'s words; `plugins` may post choice cards and get their answers;
    /// roots added or removed on the phone are saved in `config`.
    ///
    /// # Errors
    /// [`HubError::Store`] when the store can't be read.
    pub fn open(
        shed: Shed,
        data: &Path,
        (roots, config): (Vec<PathBuf>, Option<PathBuf>),
        theme: cooee_web::Theme,
        plugins: Vec<Plugin>,
    ) -> Result<Arc<Self>, HubError> {
        let store = Store::open(data)?;
        let mut threads = BTreeMap::new();
        for id in store.ids()? {
            let mut meta = store.load(&id)?;
            // Before #48 the system default login was stored by its label.
            if meta.account == "System default" {
                meta.account.clear();
            }
            let chat = Chat::from_versions(store.items(&id)?);
            threads.insert(id, Arc::new(Mutex::new(Live::new(meta, chat))));
        }
        let hub = Arc::new(Self {
            agents: Mutex::new(shed.agents().map_err(|e| e.to_string())),
            shed,
            store,
            push: Push::new(data),
            theme,
            roots: Mutex::new(roots),
            config,
            plugins: Mutex::new(plugins),
            boot: format!("{:x}", now()),
            threads: Mutex::new(threads),
            last: Mutex::new(0),
        });
        let follower = Arc::clone(&hub);
        thread::spawn(move || follower.follow());
        let ticker = Arc::clone(&hub);
        thread::spawn(move || {
            loop {
                thread::sleep(TICK);
                *lock(&ticker.agents) = ticker.shed.agents().map_err(|e| e.to_string());
                ticker.watch(now());
                let at = now();
                if let Ok(terminals) = ticker.shed.ls(BTreeMap::new()) {
                    ticker.prune(&terminals, at);
                }
            }
        });
        Ok(hub)
    }

    fn live(&self, id: &str) -> Result<Arc<Mutex<Live>>, HubError> {
        lock(&self.threads)
            .get(id)
            .cloned()
            .ok_or_else(|| HubError::NoThread(id.to_owned()))
    }

    fn all(&self) -> Vec<Arc<Mutex<Live>>> {
        lock(&self.threads).values().cloned().collect()
    }

    /// Checks each open thread's terminal: one that runs is attached (its state shown), one that is gone ends the
    /// thread. Returns the oldest applied shed event among the attached, to replay from.
    fn reattach(&self) -> Result<Option<u64>, ShedError> {
        self.shed.hello()?;
        let (at, terminals) = (now(), self.shed.ls(BTreeMap::new())?);
        self.prune(&terminals, at);
        for t in &terminals {
            if !self.all().iter().any(|l| lock(l).meta.terminal == t.name) {
                self.adopt(t);
            }
        }
        let mut since: Option<u64> = None;
        for live in self.all() {
            let mut live = lock(&live);
            if live.meta.closed || live.chat.turn.state == "ended" {
                continue;
            }
            let changes = match self.shed.get(&live.meta.terminal) {
                Ok(t) if t.state == shed_proto::State::Running => {
                    since = Some(since.map_or(live.applied, |s| s.min(live.applied)));
                    let r = t.reported.unwrap_or(shed_proto::Reported {
                        state: String::new(),
                        note: None,
                        state_source: None,
                    });
                    live.chat.turn.state = r.state;
                    live.chat.turn.note = r.note;
                    Vec::new()
                }
                Ok(_) => live.chat.ended(now(), 0, "The agent stopped.".to_owned()),
                Err(e) if e.no_such_terminal() => {
                    live.chat
                        .ended(now(), 0, "The agent is gone (shed restarted).".to_owned())
                }
                Err(e) => return Err(e),
            };
            if let Err(e) = live.publish(self, changes) {
                eprintln!("cooee: {e}");
            }
        }
        Ok(since)
    }

    /// Lists Workflow session `t` (#33) as a thread, stored so its rows outlive a restart; `None` if it isn't one.
    fn adopt(&self, t: &TerminalInfo) -> Option<Arc<Mutex<Live>>> {
        let workflow = Workflow::from_labels(&t.labels).filter(|_| t.acp.is_some())?;
        let safe = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
        let id: String = format!("w-{}", t.name)
            .chars()
            .map(|c| if safe(c) { c } else { '_' })
            .take(64)
            .collect();
        let title = format!("{} {}", workflow.group, workflow.item);
        let meta = Thread {
            id: id.clone(),
            title: Some(title.trim())
                .filter(|t| !t.is_empty())
                .unwrap_or(&t.name)
                .to_owned(),
            agent: t.acp.clone().unwrap_or_default(),
            folder: Some(t.cwd.clone()),
            terminal: t.name.clone(),
            workflow: Some(workflow),
            ..Thread::default()
        };
        if let Err(e) = self.store.save(&meta) {
            eprintln!("cooee: {e}");
            return None;
        }
        let live = Arc::new(Mutex::new(Live::new(meta, Chat::default())));
        lock(&self.threads).insert(id, Arc::clone(&live));
        Some(live)
    }

    /// Drops the Workflow sessions whose terminal is not among `terminals` (listed at `at`): the tool that started
    /// them cleaned them up (#33). One adopted since the listing stays.
    pub fn prune(&self, terminals: &[TerminalInfo], at: u64) {
        for live in self.all() {
            let id = {
                let live = lock(&live);
                let listed = terminals.iter().any(|t| t.name == live.meta.terminal);
                if live.meta.workflow.is_none() || listed || live.moved > at {
                    continue;
                }
                live.meta.id.clone()
            };
            lock(&self.threads).remove(&id);
            if let Err(e) = self.store.remove(&id) {
                eprintln!("cooee: {e}");
            }
        }
    }

    /// Follows shed's events for every terminal, for ever: re-attaches, subscribes, applies; on a break, waits a
    /// second and does it again from the last event seen.
    fn follow(&self) {
        let mut last: Option<u64> = None;
        let mut said = String::new();
        loop {
            let stream = self
                .reattach()
                .and_then(|since| self.shed.events(BTreeMap::new(), last.or(since)));
            match stream {
                Ok(events) => {
                    said.clear();
                    for event in events {
                        last = Some(event.seq);
                        self.apply(event);
                    }
                }
                Err(e) if e.to_string() != said => {
                    said = e.to_string();
                    eprintln!("cooee: {said}");
                }
                Err(_) => {}
            }
            thread::sleep(Duration::from_secs(1));
        }
    }

    /// Applies one shed event to the thread whose terminal it names; a new Workflow session's first is listed.
    pub fn apply(&self, event: Event) {
        let found = self
            .all()
            .into_iter()
            .find(|l| lock(l).meta.terminal == event.name)
            .or_else(|| {
                Workflow::from_labels(&event.labels)?;
                self.adopt(&self.shed.get(&event.name).ok()?)
            });
        let Some(live) = found else { return };
        let mut live = lock(&live);
        if event.seq <= live.applied {
            return;
        }
        live.applied = event.seq;
        live.moved = now();
        let mut changes = Vec::new();
        if live.stuck {
            // It moves again: back to working (a state event below says more).
            live.stuck = false;
            "working".clone_into(&mut live.chat.turn.state);
            live.chat.turn.note = None;
            changes.push(Change::State(live.chat.turn.clone()));
        }
        // A Workflow session's prompts come from its tool: shown as the owner's own are (#33).
        if let (Some(_), shed_proto::EventKind::Turn { prompt }) =
            (&live.meta.workflow, &event.kind)
        {
            let own = live
                .chat
                .items
                .last()
                .is_some_and(|i| matches!(&i.body, Body::You { text, .. } if text == prompt));
            if !own {
                let body = Body::You {
                    text: prompt.clone(),
                    random_id: String::new(),
                };
                changes.push(Change::Item(live.chat.push(now(), event.seq, body)));
            }
        }
        changes.extend(live.chat.apply(now(), event.seq, event.kind));
        if let Err(e) = live.publish(self, changes) {
            eprintln!("cooee: {e}");
        }
    }

    /// Surfaces each working turn with no shed event for 30 minutes before `now` as needs-you, once: the thread
    /// shows blocked with a note, and a push says so.
    pub fn watch(&self, now: u64) {
        for live in self.all() {
            let mut live = lock(&live);
            if live.stuck || live.chat.turn.state != "working" || now < live.moved + STUCK {
                continue;
            }
            live.stuck = true;
            "blocked".clone_into(&mut live.chat.turn.state);
            live.chat.turn.note = Some("no progress for 30 minutes".to_owned());
            let said = format!("{}: no progress for 30 minutes", live.meta.agent);
            self.push.send(&Note {
                title: live.meta.title.clone(),
                body: self
                    .theme
                    .push_body("needs", &said)
                    .chars()
                    .take(120)
                    .collect(),
                tag: live.meta.id.clone(),
                url: format!("/t/{}", live.meta.id),
                item: live.chat.items.last().map_or(0, |i| i.id),
            });
            let turn = live.chat.turn.clone();
            if let Err(e) = live.publish(self, vec![Change::State(turn)]) {
                eprintln!("cooee: {e}");
            }
        }
    }

    /// The installed ACP agents, as last checked.
    ///
    /// # Errors
    /// Why `shed agents` failed, when it did.
    pub fn agents(&self) -> Result<Vec<Agent>, String> {
        lock(&self.agents).clone()
    }

    /// The folders a thread may work in, with their depth: each root (0), then the folders inside it (1) and inside
    /// those (2), by name; never hidden ones or symlinks.
    #[must_use]
    pub fn folders(&self) -> Vec<(PathBuf, u8)> {
        let mut out = Vec::new();
        for root in lock(&self.roots).iter() {
            out.push((root.clone(), 0));
            for a in dirs(root, false) {
                let inside = dirs(&a, false);
                out.push((a, 1));
                out.extend(inside.into_iter().map(|b| (b, 2)));
            }
        }
        out
    }

    /// `folder`, resolved, if it is a root or inside one.
    fn allowed(&self, folder: &Path) -> Option<PathBuf> {
        let folder = folder.canonicalize().ok()?;
        let inside = |root: &PathBuf| root.canonicalize().is_ok_and(|r| folder.starts_with(r));
        lock(&self.roots).iter().any(inside).then_some(folder)
    }

    /// Settings › Folders: the folders inside `path` (default the home folder; hidden ones if `hidden`), and the roots.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a path outside the home folder or not a folder.
    pub fn browse(&self, path: Option<&Path>, hidden: bool) -> Result<Listing, HubError> {
        let (path, home) = in_home(path)?;
        let dirs = dirs(&path, hidden).into_iter().filter_map(|d| {
            let name = d.file_name()?.to_string_lossy();
            Some(name.into_owned())
        });
        Ok(Listing {
            dirs: dirs.collect(),
            path,
            home,
            roots: lock(&self.roots).clone(),
        })
    }

    /// Adds folder `path` (inside the home folder) to the roots, or removes it (as listed), and saves them in the
    /// config file. Returns the roots.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a path outside the home folder, no config file, or one that can't be written.
    pub fn set_root(&self, path: &Path, add: bool) -> Result<Vec<PathBuf>, HubError> {
        let mut roots = lock(&self.roots);
        let mut new = roots.clone();
        if add {
            let (path, _) = in_home(Some(path))?;
            if !new.contains(&path) {
                new.push(path);
            }
        } else {
            new.retain(|r| r != path);
        }
        let Some(config) = &self.config else {
            return Err(HubError::Refused(
                "there is no config file to save roots in".to_owned(),
            ));
        };
        cooee_core::save_roots(config, &new).map_err(|e| refused(config, &e))?;
        *roots = new;
        Ok(roots.clone())
    }

    /// Makes folder `name` inside `parent` (a root or inside one); returns its path.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a parent outside the roots, a name that isn't one plain folder name, or a folder that
    /// can't be made (it exists).
    pub fn new_folder(&self, parent: &Path, name: &str) -> Result<PathBuf, HubError> {
        let mut parts = Path::new(name).components();
        let plain = matches!(parts.next(), Some(std::path::Component::Normal(_)))
            && parts.next().is_none()
            && !name.contains('/')
            && !name.starts_with('.');
        if !plain {
            return Err(HubError::Refused(format!("{name:?} is not a folder name")));
        }
        if self.allowed(parent).is_none() {
            return Err(outside(parent));
        }
        let path = parent.join(name);
        fs::create_dir(&path).map_err(|e| refused(&path, &e))?;
        Ok(path)
    }

    /// Starts a thread: ACP agent `agent` in `folder` (inside a root) as a new shed terminal, with `account`'s
    /// config folder in `env` (none: the system default), then sends `text`. Returns the thread's id. A retry with
    /// the same `random_id` (after a timeout, or a crash) adopts the thread it started and never starts another.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a folder outside the roots or an empty message; [`HubError::Shed`] when the
    /// terminal can't start.
    pub fn create(
        &self,
        agent: &str,
        folder: &Path,
        text: &str,
        random_id: &str,
        (account, env): (&str, Option<(&str, &Path)>),
    ) -> Result<String, HubError> {
        let Some(folder) = self.allowed(folder) else {
            return Err(outside(folder));
        };
        self.start(agent, &folder, text, random_id, (account, env))
    }

    /// [`Hub::create`] in a folder of cooee's own (a plugin's checkout), not checked against the roots.
    ///
    /// # Errors
    /// As [`Hub::create`].
    pub fn start(
        &self,
        agent: &str,
        folder: &Path,
        text: &str,
        random_id: &str,
        (account, env): (&str, Option<(&str, &Path)>),
    ) -> Result<String, HubError> {
        self.shed.hello()?;
        let (id, fresh) = self.admit(agent, folder, text, random_id, account)?;
        self.connect(&id, (fresh, false), text, random_id, env)?;
        Ok(id)
    }

    /// [`Hub::create`] that answers at once (#48): the thread is listed `connecting`, its first message pending,
    /// and the agent starts behind it. A start that fails leaves it `unconnected` with why (`name`, the provider's
    /// name, said for the agent's id), kept; [`Hub::retry`], or the same start again, starts it again.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a folder outside the roots or an empty message.
    pub fn create_later(
        self: &Arc<Self>,
        (agent, name): (&str, &str),
        folder: &Path,
        (text, random_id): (&str, &str),
        (account, env): (&str, Option<(String, PathBuf)>),
    ) -> Result<String, HubError> {
        let Some(folder) = self.allowed(folder) else {
            return Err(outside(folder));
        };
        let (id, fresh) = self.admit(agent, &folder, text, random_id, account)?;
        if fresh {
            let live = self.live(&id)?;
            lock(&live).pending = Some(text.to_owned());
        }
        self.retry(&id, env, name)?;
        Ok(id)
    }

    /// Starts thread `id`'s agent again after its start failed (none failed, or one runs: nothing), and sends its
    /// first message; as [`Hub::create_later`].
    ///
    /// # Errors
    /// [`HubError::NoThread`]; [`HubError::Store`] when the state can't be published.
    pub fn retry(
        self: &Arc<Self>,
        id: &str,
        env: Option<(String, PathBuf)>,
        name: &str,
    ) -> Result<(), HubError> {
        let live = self.live(id)?;
        let (text, random_id, agent, fresh) = {
            let mut l = lock(&live);
            let Some(text) = l
                .pending
                .clone()
                .filter(|_| l.chat.turn.state != "connecting")
            else {
                return Ok(());
            };
            // Never tried: its terminal starts; else it starts again only if it isn't running.
            let fresh = l.chat.turn.state.is_empty();
            "connecting".clone_into(&mut l.chat.turn.state);
            l.chat.turn.note = None;
            let turn = l.chat.turn.clone();
            l.publish(self, vec![Change::State(turn)])?;
            (text, l.meta.random_id.clone(), l.meta.agent.clone(), fresh)
        };
        let (hub, id, name) = (Arc::clone(self), id.to_owned(), name.to_owned());
        thread::spawn(move || {
            let env = env.as_ref().map(|(k, dir)| (k.as_str(), dir.as_path()));
            let done = hub.shed.hello().map_err(HubError::from);
            let done = done.and_then(|()| hub.connect(&id, (fresh, true), &text, &random_id, env));
            let mut l = lock(&live);
            match done {
                // shed's events say more.
                Ok(()) if l.chat.turn.state == "connecting" => {
                    "working".clone_into(&mut l.chat.turn.state);
                }
                Ok(()) => return,
                Err(e) => {
                    "unconnected".clone_into(&mut l.chat.turn.state);
                    l.chat.turn.note = Some(format!(
                        "{name} stopped: {}",
                        e.to_string().replace(&agent, &name)
                    ));
                }
            }
            let turn = l.chat.turn.clone();
            if let Err(e) = l.publish(&hub, vec![Change::State(turn)]) {
                eprintln!("cooee: {e}");
            }
        });
        Ok(())
    }

    /// The thread start `random_id` opened (stored and listed before shed starts it: an event, an ask, that comes
    /// before the start returns is applied, and a crash after the start is adopted on restart, herdsman#446, #509),
    /// or a new one; and whether it is new.
    fn admit(
        &self,
        agent: &str,
        folder: &Path,
        text: &str,
        random_id: &str,
        account: &str,
    ) -> Result<(String, bool), HubError> {
        if text.trim().is_empty() {
            return Err(HubError::Refused("the first message is empty".to_owned()));
        }
        // Taken under the lock and counted up, so two threads started in one millisecond differ.
        let mut last = lock(&self.last);
        let started = self.all().into_iter().find_map(|l| {
            let l = lock(&l);
            let same = !random_id.is_empty() && l.meta.random_id == random_id;
            same.then(|| l.meta.id.clone())
        });
        if let Some(id) = started {
            return Ok((id, false));
        }
        *last = (*last + 1).max(now());
        while lock(&self.threads).contains_key(&format!("t{:x}", *last)) {
            *last += 1;
        }
        let id = format!("t{:x}", *last);
        let title: String = text
            .trim()
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(60)
            .collect();
        let meta = Thread {
            id: id.clone(),
            title,
            agent: agent.to_owned(),
            folder: Some(folder.to_owned()),
            account: account.to_owned(),
            terminal: format!("cooee-{id}"),
            random_id: random_id.to_owned(),
            ..Thread::default()
        };
        self.store.save(&meta)?;
        let live = Live::new(meta, Chat::default());
        lock(&self.threads).insert(id.clone(), Arc::new(Mutex::new(live)));
        Ok((id, true))
    }

    /// Starts thread `id`'s terminal when `fresh` (or, `keep`, when it doesn't run), waits for its session and
    /// sends `text`. A start that failed but came up anyway (a slow CLI) is kept, never started twice
    /// (herdsman#477); one that didn't come up is removed unless `keep`.
    fn connect(
        &self,
        id: &str,
        (fresh, keep): (bool, bool),
        text: &str,
        random_id: &str,
        env: Option<(&str, &Path)>,
    ) -> Result<(), HubError> {
        let live = self.live(id)?;
        let (terminal, agent, folder) = {
            let l = lock(&live);
            let folder = l.meta.folder.clone().unwrap_or_default();
            (l.meta.terminal.clone(), l.meta.agent.clone(), folder)
        };
        if fresh || (keep && self.shed.get(&terminal).is_err()) {
            let labels = BTreeMap::from([
                (LABEL.to_owned(), "1".to_owned()),
                (format!("{LABEL}.thread"), id.to_owned()),
            ]);
            if let Err(e) = self.shed.run_acp(&terminal, &agent, &folder, &labels, env)
                && self.shed.get(&terminal).is_err()
            {
                if !keep {
                    lock(&self.threads).remove(id);
                    self.store.remove(id)?;
                }
                return Err(e.into());
            }
        }
        self.shed.ready(&terminal)?;
        self.send(id, text, random_id)?;
        Ok(())
    }

    /// Sends the owner's message: once per `random_id` (a retry of a stored send returns it again). The message
    /// is stored only once shed took it.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a closed thread or an empty message; [`HubError::Shed`] when shed fails.
    pub fn send(&self, id: &str, text: &str, random_id: &str) -> Result<Item, HubError> {
        let live = self.live(id)?;
        let mut live = lock(&live);
        let sent = live
            .chat
            .items
            .iter()
            .rev()
            .find(|i| matches!(&i.body, Body::You { random_id: r, .. } if r == random_id));
        if let Some(item) = sent {
            return Ok(item.clone());
        }
        if live.meta.closed || text.trim().is_empty() {
            return Err(HubError::Refused(
                "the thread is closed or the message empty".to_owned(),
            ));
        }
        self.shed.prompt(&live.meta.terminal, text)?;
        let body = Body::You {
            text: text.to_owned(),
            random_id: random_id.to_owned(),
        };
        let item = live.chat.push(now(), 0, body);
        if live.meta.random_id == random_id {
            live.pending = None;
        }
        live.publish(self, vec![Change::Item(item.clone())])?;
        Ok(item)
    }

    /// The owner's answer to approval `item`: sent once (stored first) to shed, or to the plugin whose choice card
    /// it is (then chosen); a second tap returns the item.
    ///
    /// # Errors
    /// [`HubError::Shed`] when shed fails; [`HubError::Refused`] when the plugin does.
    pub fn approve(&self, id: &str, item: u64, option: &str) -> Result<Option<Item>, HubError> {
        let live = self.live(id)?;
        let mut live = lock(&live);
        let Some(tapped) = live.chat.tap(item, option) else {
            return Ok(live.chat.items.iter().find(|i| i.id == item).cloned());
        };
        live.publish(self, vec![Change::Item(tapped.clone())])?;
        let sent = match &tapped.body {
            Body::Approval {
                plugin: Some(name), ..
            } => self
                .plugins()
                .into_iter()
                .find(|p| &p.name == name)
                .ok_or_else(|| io::Error::other("no longer configured"))
                .and_then(|p| crate::plugin::answer(&p, id, item, option))
                .map_err(|e| HubError::Refused(format!("plugin {name}: {e}"))),
            _ => return self.shed_approve(&mut live, tapped, option),
        };
        let done = live.chat.update(item, 0, |b| {
            if let Body::Approval { tapped, chosen, .. } = b {
                if sent.is_ok() {
                    *chosen = Some(option.to_owned());
                } else {
                    *tapped = None;
                }
            }
        });
        live.publish(self, done.clone().map(Change::Item).into_iter().collect())?;
        sent.map(|()| done)
    }

    fn shed_approve(
        &self,
        live: &mut Live,
        tapped: Item,
        option: &str,
    ) -> Result<Option<Item>, HubError> {
        if let Err(e) = self.shed.approve(&live.meta.terminal, option) {
            // Not delivered: the row can be tapped again.
            let untapped = live.chat.update(tapped.id, 0, |b| {
                if let Body::Approval { tapped, .. } = b {
                    *tapped = None;
                }
            });
            live.publish(self, untapped.map(Change::Item).into_iter().collect())?;
            return Err(e.into());
        }
        Ok(Some(tapped))
    }

    /// Plugin `plugin`'s choice card in thread `id`: `text` and a button per option. The owner's tap goes to the
    /// plugin.
    ///
    /// # Errors
    /// [`HubError::Refused`] without text or options.
    pub fn card(
        &self,
        id: &str,
        plugin: &str,
        text: &str,
        options: &[String],
    ) -> Result<Item, HubError> {
        if text.trim().is_empty() || options.is_empty() {
            return Err(HubError::Refused(
                "a choice card needs text and options".to_owned(),
            ));
        }
        let live = self.live(id)?;
        let mut live = lock(&live);
        let options = options.iter().map(|o| ApprovalOption {
            id: o.clone(),
            name: o.clone(),
            kind: String::new(),
        });
        let body = Body::Approval {
            tool: text.to_owned(),
            options: options.collect(),
            tapped: None,
            chosen: None,
            plugin: Some(plugin.to_owned()),
        };
        let item = live.chat.push(now(), 0, body);
        live.publish(self, vec![Change::Item(item.clone())])?;
        Ok(item)
    }

    /// Cancels the turn in progress.
    ///
    /// # Errors
    /// [`HubError::Shed`] when shed fails.
    pub fn cancel(&self, id: &str) -> Result<(), HubError> {
        let live = self.live(id)?;
        let terminal = lock(&live).meta.terminal.clone();
        Ok(self.shed.cancel(&terminal)?)
    }

    /// Closes the thread: kills and removes its terminal. Its history stays.
    ///
    /// # Errors
    /// [`HubError::Shed`] when shed fails.
    pub fn close(&self, id: &str) -> Result<(), HubError> {
        let live = self.live(id)?;
        let mut live = lock(&live);
        if live.meta.workflow.is_some() {
            return Err(HubError::Refused(
                "the tool that started this session closes it".to_owned(),
            ));
        }
        self.shed.close(&live.meta.terminal)?;
        live.meta.closed = true;
        self.store.save(&live.meta)?;
        if live.chat.turn.state != "ended" {
            let changes = live
                .chat
                .ended(now(), 0, "The thread was closed.".to_owned());
            live.publish(self, changes)?;
        }
        Ok(())
    }

    /// Marks the thread read up to item `item`.
    ///
    /// # Errors
    /// [`HubError::Store`] when the thread can't be saved.
    pub fn read(&self, id: &str, item: u64) -> Result<(), HubError> {
        let live = self.live(id)?;
        let mut live = lock(&live);
        if live.meta.read.is_some_and(|r| r >= item) {
            return Ok(());
        }
        live.meta.read = Some(item);
        Ok(self.store.save(&live.meta)?)
    }

    /// The thread list, newest first.
    #[must_use]
    pub fn list(&self) -> Vec<Summary> {
        let mut rows: Vec<Summary> = self
            .all()
            .iter()
            .map(|live| {
                let live = lock(live);
                let last = live.chat.items.last();
                let text = last.map_or("", |i| match &i.body {
                    Body::You { text, .. } | Body::Agent { text, .. } | Body::Ended { text } => {
                        text
                    }
                    Body::Tool { title, .. } => title,
                    Body::Approval { tool, .. } => tool,
                    Body::Plan { .. } => "plan",
                });
                Summary {
                    id: live.meta.id.clone(),
                    title: live.meta.title.clone(),
                    agent: live.meta.agent.clone(),
                    account: live.meta.account.clone(),
                    folder: live.meta.folder.clone().unwrap_or_default(),
                    state: live.chat.turn.state.clone(),
                    last: text.lines().last().unwrap_or("").chars().take(80).collect(),
                    at: last.map_or(0, |i| i.at),
                    unread: last.is_some_and(|i| live.meta.read.is_none_or(|r| r < i.id)),
                    workflow: live.meta.workflow.clone(),
                }
            })
            .collect();
        rows.sort_by_key(|r| std::cmp::Reverse(r.at));
        rows
    }

    /// The thread's meta.
    ///
    /// # Errors
    /// [`HubError::NoThread`].
    pub fn thread(&self, id: &str) -> Result<(Thread, Turn), HubError> {
        let live = self.live(id)?;
        let live = lock(&live);
        Ok((live.meta.clone(), live.chat.turn.clone()))
    }

    /// Up to `before` items before item `around` and `after` after it (with it); without `around`, the newest
    /// `before` items. With the turn in progress and the cursor they are current to.
    ///
    /// # Errors
    /// [`HubError::NoThread`].
    pub fn slice(
        &self,
        id: &str,
        around: Option<u64>,
        before: usize,
        after: usize,
    ) -> Result<Slice, HubError> {
        let live = self.live(id)?;
        let live = lock(&live);
        let items = &live.chat.items;
        let (from, to) = match around.map(|a| items.partition_point(|i| i.id < a)) {
            Some(at) => (at.saturating_sub(before), (at + after + 1).min(items.len())),
            None => (items.len().saturating_sub(before), items.len()),
        };
        Ok(Slice {
            items: items[from..to.max(from)].to_vec(),
            skipped_before: from,
            skipped_after: items.len() - to.max(from),
            turn: live.chat.turn.clone(),
            cursor: format!("{}.{}", self.boot, live.seq),
            pending: live.pending.clone().map(|text| Pending {
                text,
                random_id: live.meta.random_id.clone(),
            }),
        })
    }

    /// Subscribes to thread `id`'s page events after `cursor` (`<boot>.<seq>`), in one step under the thread's
    /// lock, so nothing is missed or doubled.
    ///
    /// # Errors
    /// [`HubError::NoThread`].
    pub fn subscribe(
        &self,
        id: &str,
        cursor: &str,
    ) -> Result<(Resume, broadcast::Receiver<Out>), HubError> {
        let live = self.live(id)?;
        let live = lock(&live);
        let seq = cursor
            .strip_prefix(&self.boot)
            .and_then(|s| s.strip_prefix('.'))
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|s| *s <= live.seq);
        let first = live.ring.front().map_or(live.seq + 1, |o| o.seq);
        let resume = match seq {
            Some(s) if s + 1 >= first => {
                Resume::Replay(live.ring.iter().filter(|o| o.seq > s).cloned().collect())
            }
            _ => Resume::Reset,
        };
        Ok((resume, live.tx.subscribe()))
    }

    /// The plugins: the config's own and the installed ones' tabs.
    #[must_use]
    pub fn plugins(&self) -> Vec<Plugin> {
        lock(&self.plugins).clone()
    }

    /// Replaces the plugins (the supervisor, as plugins are installed or removed).
    pub fn set_plugins(&self, plugins: Vec<Plugin>) {
        *lock(&self.plugins) = plugins;
    }

    /// The push subscriptions and key.
    #[must_use]
    pub fn push(&self) -> &Push {
        &self.push
    }

    /// The SSE id of a page event.
    #[must_use]
    pub fn event_id(&self, out: &Out) -> String {
        format!("{}.{}", self.boot, out.seq)
    }
}

impl From<HubError> for io::Error {
    fn from(e: HubError) -> Self {
        io::Error::other(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use cooee_core::shed_proto::{EventKind, Reported};

    use super::*;

    /// A hub over `dir`'s data with no shed (its follower retries in the background, harmlessly).
    fn hub(dir: &Path) -> Arc<Hub> {
        let shed = Shed::new(dir.join("no-shed"), Some(dir.join("no-shed-dir")));
        let (looks, _) = cooee_web::Looks::load(None, None);
        let theme = looks.themes[&looks.default].clone();
        Hub::open(
            shed,
            &dir.join("data"),
            (Vec::new(), None),
            theme,
            Vec::new(),
        )
        .unwrap()
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cooee-hub-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("data/threads")).unwrap();
        dir
    }

    /// Thread `id` on terminal `cooee-<id>`, with `lines` as its items file.
    fn seed(dir: &Path, id: &str, lines: &str) {
        let threads = dir.join("data/threads");
        let meta = json!({ "format": 1, "thread": { "id": id, "title": id, "messages": [], "terminal": format!("cooee-{id}") } });
        fs::write(threads.join(format!("{id}.json")), meta.to_string()).unwrap();
        fs::write(threads.join(format!("{id}.jsonl")), lines).unwrap();
    }

    fn event(id: &str, seq: u64, kind: EventKind) -> Event {
        Event {
            seq,
            name: format!("cooee-{id}"),
            labels: BTreeMap::new(),
            kind,
        }
    }

    /// R6 (herdsman#447): a thread of 1,000 messages written as 30,000 versions serves its requests (a slice, the
    /// list, the page's thread) in under 300 ms: none is quadratic in the log. (The fold at load: cooee-core `chat`.)
    #[test]
    fn r6_requests_over_a_long_log_are_within_budget() {
        let dir = scratch("budget");
        let mut lines = String::new();
        for v in 0..30 {
            for id in 1..=1000 {
                let line = json!({ "id": id, "at": id, "seq": v * 1000 + id, "kind": "agent", "text": format!("reply {id} v{v}"), "stop_reason": "end_turn" });
                lines += &line.to_string();
                lines.push('\n');
            }
        }
        seed(&dir, "long", &lines);
        let hub = hub(&dir);
        let at = Instant::now();
        let slice = hub.slice("long", Some(500), 50, 50).unwrap();
        let newest = hub.slice("long", None, 500, 0).unwrap();
        let rows = hub.list();
        hub.thread("long").unwrap();
        let took = at.elapsed();
        assert_eq!((slice.items.len(), slice.skipped_before), (101, 449));
        assert_eq!(
            slice.items[0].body,
            Body::Agent {
                text: "reply 450 v29".into(),
                stop_reason: "end_turn".into()
            }
        );
        assert_eq!((newest.items.len(), rows.len()), (500, 1));
        assert!(took < Duration::from_millis(300), "{took:?}");
    }

    /// R15 (herdsman#444, #445, #493, #534): a working turn with no shed event for 30 minutes (a fake clock) is
    /// surfaced as needs-you, once; its next event puts it back to working.
    #[test]
    fn r15_a_turn_idle_30_minutes_is_surfaced_as_needs_you() {
        let dir = scratch("stuck");
        seed(&dir, "idle", "");
        let hub = hub(&dir);
        let working = Reported {
            state: "working".to_owned(),
            note: None,
            state_source: None,
        };
        hub.apply(event("idle", 1, EventKind::State(working)));
        let state = |hub: &Hub| hub.thread("idle").unwrap().1;
        let t0 = now();
        hub.watch(t0 + STUCK - 60_000);
        assert_eq!(state(&hub).state, "working");
        let (_, rx) = hub.subscribe("idle", "").unwrap();
        hub.watch(t0 + STUCK + 1);
        let turn = state(&hub);
        assert_eq!(
            (turn.state.as_str(), turn.note.as_deref()),
            ("blocked", Some("no progress for 30 minutes"))
        );
        // Once: a second look sends nothing more.
        hub.watch(t0 + 2 * STUCK);
        assert_eq!(rx.len(), 1);
        // On the list too.
        assert_eq!(hub.list()[0].state, "blocked");
        hub.apply(event("idle", 2, EventKind::Text { text: "w".into() }));
        assert_eq!(state(&hub).state, "working");
    }

    /// #33: a Workflow session whose terminal shed no longer lists is dropped (its files too); one that moved since
    /// the listing and the owner's own chats stay.
    #[test]
    fn prune_drops_only_the_workflow_sessions_shed_no_longer_lists() {
        let dir = scratch("prune");
        seed(&dir, "mine", "");
        let threads = dir.join("data/threads");
        for id in ["gone", "new"] {
            let meta = json!({ "format": 1, "thread": { "id": id, "title": id, "messages": [], "terminal": format!("cooee-{id}"), "workflow": { "source": "tool" } } });
            fs::write(threads.join(format!("{id}.json")), meta.to_string()).unwrap();
        }
        let hub = hub(&dir);
        let at = now();
        std::thread::sleep(Duration::from_millis(2));
        hub.apply(event("new", 1, EventKind::Text { text: "w".into() }));
        hub.prune(&[], at);
        let ids: Vec<String> = hub.list().into_iter().map(|r| r.id).collect();
        assert_eq!(ids.len(), 2, "{ids:?}");
        assert!(ids.contains(&"mine".to_owned()) && ids.contains(&"new".to_owned()));
        assert!(!threads.join("gone.json").exists());
    }
}
