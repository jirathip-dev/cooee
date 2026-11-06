//! The threads, live: each thread's chat, its numbered page events (kept in a ring for replay) and their
//! subscribers, fed by one shed `events` stream for every cooee terminal.

use std::{
    collections::{BTreeMap, VecDeque},
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use cooee_core::{
    Agent, Body, Card, Change, Chat, Item, Plugin, Shed, ShedError, Store, StoreError, Summary,
    Thread, Turn,
    shed_proto::{self, ApprovalOption, Event},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::broadcast;

use crate::push::{Note, Push, note};

/// How many page events a thread keeps for a reconnecting page.
const RING: usize = 4096;

/// The label every cooee terminal carries.
const LABEL: &str = "cooee";

/// A working turn with no shed event for this long is surfaced as needs-you (herdsman#444), in milliseconds.
const STUCK: u64 = 30 * 60 * 1000;

/// How long a conductor's answer may be changed (#553 row 31), in milliseconds.
const CHANGE: u64 = 30 * 60 * 1000;

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
    /// The oldest config card that waits (#553 row 32), wherever it is.
    pub waiting: Option<Item>,
}

/// A plugin's card (`POST /plugin/card`, docs/plugins.md).
#[derive(Debug, Deserialize)]
pub struct CardSpec {
    /// The thread it goes in.
    pub thread: String,
    /// The question, a config change's name or why a secret is wanted.
    pub text: String,
    /// The choices (none for a form's card).
    #[serde(default)]
    pub options: Vec<String>,
    /// The option shown first, marked recommended.
    #[serde(default)]
    pub recommend: Option<String>,
    /// `config`, `secret` or `thread`; none: choices.
    #[serde(default)]
    pub form: Option<String>,
    /// A config card's diff, or a secret card's name.
    #[serde(default)]
    pub detail: Option<String>,
    /// The conductor's own answer: shown answered, and the owner may change it for 30 minutes.
    #[serde(default)]
    pub answered: Option<String>,
    /// Who gave that answer (default: the plugin's label).
    #[serde(default)]
    pub by: Option<String>,
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
            let meta = store.load(&id)?;
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

    /// Follows shed's events for every cooee terminal, for ever: re-attaches, subscribes, applies; on a break,
    /// waits a second and does it again from the last event seen.
    fn follow(&self) {
        let mut last: Option<u64> = None;
        let mut said = String::new();
        loop {
            let labels = BTreeMap::from([(LABEL.to_owned(), "1".to_owned())]);
            let stream = self
                .reattach()
                .and_then(|since| self.shed.events(labels, last.or(since)));
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

    /// Applies one shed event to the thread whose terminal it names.
    pub fn apply(&self, event: Event) {
        let found = self
            .all()
            .into_iter()
            .find(|l| lock(l).meta.terminal == event.name);
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
        if text.trim().is_empty() {
            return Err(HubError::Refused("the first message is empty".to_owned()));
        }
        let (id, terminal, adopted) = {
            // Taken under the lock and counted up, so two threads started in one millisecond differ.
            let mut last = lock(&self.last);
            let started = self.all().into_iter().find_map(|l| {
                let l = lock(&l);
                let same = !random_id.is_empty() && l.meta.random_id == random_id;
                same.then(|| (l.meta.id.clone(), l.meta.terminal.clone()))
            });
            if let Some((id, terminal)) = started {
                (id, terminal, true)
            } else {
                {
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
                    // Stored and listed before shed starts it: an event (an ask) that comes before the start
                    // returns is applied, and a crash after the start is adopted on restart (herdsman#446, #509).
                    self.store.save(&meta)?;
                    let terminal = meta.terminal.clone();
                    let live = Live::new(meta, Chat::default());
                    lock(&self.threads).insert(id.clone(), Arc::new(Mutex::new(live)));
                    (id, terminal, false)
                }
            }
        };
        if !adopted {
            let labels = BTreeMap::from([
                (LABEL.to_owned(), "1".to_owned()),
                (format!("{LABEL}.thread"), id.clone()),
            ]);
            // A start that failed but came up anyway (a slow CLI) is kept, never started twice (herdsman#477).
            if let Err(e) = self.shed.run_acp(&terminal, agent, folder, &labels, env)
                && self.shed.get(&terminal).is_err()
            {
                lock(&self.threads).remove(&id);
                self.store.remove(&id)?;
                return Err(e.into());
            }
        }
        self.shed.ready(&terminal)?;
        self.send(&id, text, random_id)?;
        Ok(id)
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
        live.publish(self, vec![Change::Item(item.clone())])?;
        Ok(item)
    }

    /// `by`'s answer to approval `item`: sent once (stored first) to shed, or to the plugin whose card it is (then
    /// chosen, with the plugin's line; a 409 leaves it closed with the plugin's why). A secret card's `value` goes
    /// to its plugin only and is never kept. A second tap returns the item.
    ///
    /// # Errors
    /// [`HubError::Shed`] when shed fails; [`HubError::Refused`] when the plugin does.
    pub fn approve(
        &self,
        id: &str,
        item: u64,
        option: &str,
        value: Option<&str>,
        by: &str,
    ) -> Result<Option<Item>, HubError> {
        let live = self.live(id)?;
        let mut live = lock(&live);
        let Some(tapped) = live.chat.tap(item, option, by, now()) else {
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
                .and_then(|p| crate::plugin::answer(&p, id, item, option, by, value))
                .map_err(|e| HubError::Refused(format!("plugin {name}: {e}"))),
            _ => return self.shed_approve(&mut live, tapped, option),
        };
        let done = live.chat.update(item, 0, |b| {
            if let Body::Approval {
                tapped,
                chosen,
                card,
                ..
            } = b
            {
                match &sent {
                    Ok(Ok(said)) => {
                        *chosen = Some(option.to_owned());
                        card.line.clone_from(said);
                    }
                    Ok(Err(why)) => card.line = Some(why.clone()),
                    Err(_) => *tapped = None,
                }
            }
        });
        live.publish(self, done.clone().map(Change::Item).into_iter().collect())?;
        sent.map(|_| done)
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

    /// Plugin `plugin`'s card (#553 rows 30–33, 35): choices (the recommended one first), a config diff to apply
    /// or discard, a secret to save or dismiss, or the other open threads to pick from. The owner's tap goes to the
    /// plugin; a card the conductor answered itself may be changed for 30 minutes.
    ///
    /// # Errors
    /// [`HubError::Refused`] without text, options, a config's diff or a secret's name.
    pub fn card(&self, plugin: &Plugin, c: CardSpec) -> Result<Item, HubError> {
        let plain = |o: &str| ApprovalOption {
            id: o.to_owned(),
            name: o.to_owned(),
            kind: String::new(),
        };
        let mut options: Vec<ApprovalOption> = match c.form.as_deref() {
            None => c.options.iter().map(|o| plain(o)).collect(),
            Some("config") => vec![plain("apply"), plain("discard")],
            Some("secret") => vec![plain("save"), plain("dismiss")],
            Some("thread") => self
                .list()
                .into_iter()
                .filter(|t| t.id != c.thread && t.state != "ended")
                .take(6)
                .map(|t| ApprovalOption {
                    id: t.id,
                    name: t.title,
                    kind: String::new(),
                })
                .collect(),
            Some(f) => return Err(HubError::Refused(format!("no card form {f:?}"))),
        };
        let detail = c.detail.as_deref().is_some_and(|d| !d.trim().is_empty());
        if c.text.trim().is_empty()
            || options.is_empty()
            || (matches!(c.form.as_deref(), Some("config" | "secret")) && !detail)
        {
            return Err(HubError::Refused(
                "a card needs text and options (a thread to pick), a config card its diff and a secret card its name"
                    .to_owned(),
            ));
        }
        if let Some(at) = options
            .iter()
            .position(|o| c.recommend.as_ref() == Some(&o.id))
        {
            let mut first = options.remove(at);
            "recommended".clone_into(&mut first.kind);
            options.insert(0, first);
        }
        let live = self.live(&c.thread)?;
        let mut live = lock(&live);
        let at = now();
        let answered = c.answered.is_some();
        let body = Body::Approval {
            tool: c.text,
            options,
            tapped: None,
            chosen: c.answered,
            plugin: Some(plugin.name.clone()),
            card: Card {
                by: answered.then(|| c.by.unwrap_or_else(|| plugin.label.clone())),
                answered: answered.then_some(at),
                form: c.form,
                detail: c.detail,
                line: None,
                until: answered.then_some(at + CHANGE),
            },
        };
        let item = live.chat.push(at, 0, body);
        live.publish(self, vec![Change::Item(item.clone())])?;
        Ok(item)
    }

    /// A line in thread `id` linked to thread `other` (#553 row 35): a handoff's `continued in` (`out`), or a
    /// line `from` it.
    ///
    /// # Errors
    /// [`HubError::NoThread`] when either thread is unknown.
    pub fn link(&self, id: &str, other: &str, text: &str, out: bool) -> Result<Item, HubError> {
        let title = self.thread(other)?.0.title;
        let live = self.live(id)?;
        let mut live = lock(&live);
        let body = Body::Link {
            text: text.to_owned(),
            thread: other.to_owned(),
            title,
            out,
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
                    Body::Link { text, title, .. } if text.is_empty() => title,
                    Body::Link { text, .. } => text,
                    Body::Plan { .. } => "plan",
                });
                let waiting = live.chat.items.iter().find_map(|i| match &i.body {
                    Body::Approval { tool, .. } if waits(&i.body) => {
                        Some(format!("{tool} waiting"))
                    }
                    _ => None,
                });
                Summary {
                    id: live.meta.id.clone(),
                    title: live.meta.title.clone(),
                    agent: live.meta.agent.clone(),
                    folder: live.meta.folder.clone().unwrap_or_default(),
                    state: live.chat.turn.state.clone(),
                    last: waiting.unwrap_or_else(|| {
                        text.lines().last().unwrap_or("").chars().take(80).collect()
                    }),
                    at: last.map_or(0, |i| i.at),
                    unread: last.is_some_and(|i| live.meta.read.is_none_or(|r| r < i.id)),
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
            waiting: items.iter().find(|i| waits(&i.body)).cloned(),
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

/// Whether `body` is a config card that waits for the owner (#553 row 32).
fn waits(body: &Body) -> bool {
    matches!(body, Body::Approval { chosen: None, card, .. } if card.form.as_deref() == Some("config") && card.line.is_none())
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
}
