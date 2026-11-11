//! The threads, live: each thread's chat, its numbered page events (kept in a ring for replay) and their
//! subscribers, fed by one shed `events` stream for every terminal: cooee's own, and the Workflow sessions other
//! tools start, known by their labels (#33).

use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError, mpsc},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use cooee_core::{
    Agent, Body, Card, Change, Chat, Config, Item, Kind, Launch, Plugin, Queued, Shed, ShedError,
    Store, StoreError, Summary, Thread, Turn, Workflow,
    mcp::{self, COOEE, Pick, Server},
    personas,
    shed_proto::{self, ApprovalOption, Event, TerminalInfo},
    skills::{self, Skill},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::broadcast;

use crate::{
    mcp::Grant,
    push::{Note, Push, note},
};

mod ask;
mod models;
mod rooms;
pub use rooms::Starter;

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
/// How long the agent list is kept before the background refreshes it (#55), in milliseconds.
const AGENTS_FOR: u64 = 60_000;

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
    /// When its last prompt was sent, until the reply's first chunk came (#53).
    sent: Option<Instant>,
    /// A room's round was stopped while its working member's session was starting: cancel it once started (#155).
    stopped_starting: bool,
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
    /// The messages queued for the turns after this one.
    pub queue: Vec<Queued>,
    /// The oldest config card that waits (#553 row 32), wherever it is.
    pub waiting: Option<Item>,
}

/// A thread's first message while it connects: its text and the start's id (the item it becomes has it).
#[derive(Debug, Clone, Serialize)]
pub struct Pending {
    /// The message.
    pub text: String,
    /// The start's `random_id`.
    pub random_id: String,
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
    /// `config`, `secret`, `thread` or `run-card`; none: choices.
    #[serde(default)]
    pub form: Option<String>,
    /// A config card's diff, a secret card's name, or a run card's command.
    #[serde(default)]
    pub detail: Option<String>,
    /// A run card's command sha.
    #[serde(default)]
    pub sha: Option<String>,
    /// The conductor's own answer: shown answered, and the owner may change it for 30 minutes.
    #[serde(default)]
    pub answered: Option<String>,
    /// Who gave that answer (default: the plugin's label).
    #[serde(default)]
    pub by: Option<String>,
}

/// A file sent with a message: its name on the phone and its bytes.
pub struct Upload {
    /// The name it had (only its safe characters are kept).
    pub name: String,
    /// Its bytes.
    pub data: Vec<u8>,
}

/// What a page subscription starts with: replayed events, or a reset (the page reloads its slice).
pub enum Resume {
    /// The events after the page's cursor.
    Replay(Vec<Out>),
    /// The cursor is from another run or too old.
    Reset,
}

/// How a thread starts besides its agent, folder and first message: its account's label, its agent profile's id
/// (empty: Plain), what its agent runs with, and the instructions sent before the first message.
#[derive(Debug, Default)]
pub struct Recipe {
    /// The account's label, as the header shows it.
    pub account: String,
    /// The profile's id.
    pub profile: String,
    /// The account's folder, the profile's settings, env and policy.
    pub launch: Launch,
    /// Sent ahead of the first message, which is stored alone.
    pub instructions: String,
    /// The model picked (#553 row 34; empty: the agent's default), its words in `launch`.
    pub model: String,
    /// The effort picked, as `model`.
    pub effort: String,
    /// The MCP servers its profile picks (#56), each run through cooee's proxy.
    pub mcp: Vec<String>,
    /// The room it is a member's session of (#154; its persona is `profile`).
    pub room: String,
    /// The thread whose persona asked this one's (#156).
    pub asked_by: String,
    /// The machine it runs on (#148; empty: this host), one of [`cooee_machines::Machines::added`].
    pub machine: String,
}

/// The threads and the shed they run on.
pub struct Hub {
    shed: Shed,
    store: Store,
    /// `<data>/scratch`: a thread started with no folder runs in its own folder here, named by its id (#48).
    scratch: PathBuf,
    /// `<data>/files/<thread>/`: the files sent with messages.
    files: PathBuf,
    push: Push,
    /// The server's default theme, whose words a push says.
    theme: cooee_web::Theme,
    roots: Mutex<Vec<PathBuf>>,
    /// The config file the roots are saved in.
    config: Option<PathBuf>,
    plugins: Mutex<Vec<Plugin>>,
    /// `shed agents`, as last checked, and when: a page load never runs the CLI itself (herdsman#337).
    agents: Mutex<(Result<Vec<Agent>, String>, u64)>,
    boot: String,
    threads: Mutex<BTreeMap<String, Arc<Mutex<Live>>>>,
    /// The stamp of the newest thread id handed out.
    last: Mutex<u64>,
    /// The data folder (the secrets' file fallback).
    data: PathBuf,
    /// Where a thread's MCP proxies reach cooee, once it listens.
    url: OnceLock<String>,
    /// The key each thread's MCP token is made with (`<data>/mcp.key`).
    mcp_key: String,
    /// The servers and skills agents proposed, by thread and card, until answered (a restart drops them).
    proposals: Mutex<HashMap<(String, u64), Proposed>>,
    /// Each server's tools as last listed, for the profile sheet; held while the registry is written.
    tools: Mutex<HashMap<String, cooee_web::Tools>>,
    /// The rooms with a member's turn to start, for the turn worker.
    turns: mpsc::Sender<String>,
    /// How a room's member session starts, given its persona.
    starter: OnceLock<Starter>,
    /// Each pending `ask_persona` (#156) by `<asking thread>><persona>`: where its reply goes.
    asks: Mutex<HashMap<String, mpsc::Sender<String>>>,
    /// Each agent and account's models, as last seen (#193; `<data>/models.json`).
    models: Mutex<Vec<cooee_core::models::Offered>>,
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
            sent: None,
            stopped_starting: false,
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
        for mut change in changes {
            if let Change::Item(i) = &mut change
                && matches!(i.body, Body::Approval { .. })
                && !self.meta.room.is_empty()
            {
                i.speaker.clone_from(&self.meta.profile);
                if let Some(c) = self.chat.items.iter_mut().find(|c| c.id == i.id) {
                    c.speaker.clone_from(&i.speaker);
                }
            }
            let watched = self.tx.receiver_count() > 0;
            if let Some(n) = note(
                &self.meta, &self.chat, &change, &self.was, watched, &hub.theme,
            ) {
                hub.push.send(&hub.routed(&self.meta, n));
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
        apns: Option<cooee_core::Apns>,
    ) -> Result<Arc<Self>, HubError> {
        let store = Store::open(data)?;
        let key = data.join("mcp.key");
        let mcp_key = if let Ok(k) = fs::read_to_string(&key) {
            k
        } else {
            let k = cooee_core::random(32).map(|r| cooee_core::sha256(&r));
            let k = k.and_then(|k| cooee_core::write_private(&key, k.as_bytes()).map(|()| k));
            k.map_err(|e| refused(&key, &e))?
        };
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
        let (turns, next) = mpsc::channel();
        let hub = Arc::new(Self {
            agents: Mutex::new((shed.agents().map_err(|e| e.to_string()), now())),
            shed,
            store,
            scratch: data.join("scratch"),
            files: data.join("files"),
            push: Push::new(data).with_apns(apns),
            theme,
            roots: Mutex::new(roots),
            config,
            plugins: Mutex::new(plugins),
            boot: format!("{:x}", now()),
            threads: Mutex::new(threads),
            last: Mutex::new(0),
            data: data.to_owned(),
            url: OnceLock::new(),
            mcp_key,
            proposals: Mutex::default(),
            tools: Mutex::default(),
            turns,
            starter: OnceLock::new(),
            asks: Mutex::default(),
            models: Mutex::new(cooee_core::models::load(&data.join("models.json"))),
        });
        let worker = Arc::clone(&hub);
        thread::spawn(move || {
            for room in next {
                if let Err(e) = worker.take_turn(&room) {
                    eprintln!("cooee: room {room}: {e}");
                }
            }
        });
        let follower = Arc::clone(&hub);
        thread::spawn(move || follower.follow());
        let ticker = Arc::clone(&hub);
        thread::spawn(move || {
            loop {
                thread::sleep(TICK);
                ticker.refresh(now(), false);
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
            if live.meta.closed || live.meta.kind == Kind::Room || live.chat.turn.state == "ended" {
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
                    files: Vec::new(),
                };
                changes.push(Change::Item(live.chat.push(now(), event.seq, body)));
            }
        }
        changes.extend(live.chat.apply(now(), event.seq, event.kind));
        let chunk = changes.iter().any(|c| match c {
            Change::Text(_) => true,
            Change::Item(i) => matches!(i.body, Body::Agent { .. }),
            Change::State(_) => false,
        });
        if let Some(sent) = live.sent.take_if(|_| chunk) {
            cooee_core::perf::first_token(&live.meta.id, &live.meta.agent, sent);
        }
        // A member's final reply goes to its room (#154); a cancelled one is a stopped round's, never the next's.
        let reply = changes.iter().find_map(|c| match c {
            Change::Item(i) if matches!(&i.body, Body::Agent { stop_reason, .. } if stop_reason != "cancelled") => {
                Some(i.body.clone())
            }
            _ => None,
        });
        let seat = (live.meta.room.clone(), live.meta.profile.clone());
        let asked = format!("{}>{}", live.meta.asked_by, seat.1);
        let done = live
            .publish(self, changes)
            .map_err(HubError::from)
            .and_then(|()| self.drain(&mut live));
        drop(live);
        let done = match reply {
            Some(body) if !seat.0.is_empty() => done.and(self.replied(&seat.0, &seat.1, body)),
            // An asked persona's reply goes to its asker's pending `ask_persona` (#156).
            Some(Body::Agent { text, .. }) => {
                lock(&self.asks).remove(&asked).map(|tx| tx.send(text));
                done
            }
            _ => done,
        };
        if let Err(e) = done {
            eprintln!("cooee: {e}");
        }
    }

    /// Between turns, sends the first queued message (the next one goes as that turn ends).
    fn drain(&self, live: &mut Live) -> Result<(), HubError> {
        let between = !matches!(
            live.chat.turn.state.as_str(),
            "working" | "blocked" | "ended"
        );
        if !between || live.meta.queue.is_empty() || live.meta.closed {
            return Ok(());
        }
        let next = live.meta.queue.remove(0);
        self.queued(live)?;
        self.deliver(live, next, "").map(drop)
    }

    /// Saves the queue and tells the pages.
    fn queued(&self, live: &mut Live) -> Result<(), HubError> {
        self.store.save(&live.meta)?;
        let data = json!(live.meta.queue).to_string();
        live.emit("queue", data);
        Ok(())
    }

    /// Surfaces each working turn with no shed event for 30 minutes before `now` as needs-you, once: the thread
    /// shows blocked with a note, and a push says so.
    pub fn watch(&self, now: u64) {
        self.deadlines(now);
        for live in self.all() {
            let mut live = lock(&live);
            if live.stuck || live.chat.turn.state != "working" || now < live.moved + STUCK {
                continue;
            }
            live.stuck = true;
            "blocked".clone_into(&mut live.chat.turn.state);
            live.chat.turn.note = Some("no progress for 30 minutes".to_owned());
            let item = live.chat.items.last().map_or(0, |i| i.id);
            let said = (
                "needs",
                live.meta.agent.as_str(),
                "no progress for 30 minutes",
            );
            let n = Note::new(&live.meta, said, item, &self.theme);
            self.push.send(&self.routed(&live.meta, n));
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
        lock(&self.agents).0.clone()
    }

    /// Runs `shed agents` again when the list is a minute old at `at`, or `now`; the list is never locked while the
    /// CLI runs. A failure keeps the list already had (logged), else it is the list's error.
    fn refresh(&self, at: u64, now: bool) {
        if !now && at.saturating_sub(lock(&self.agents).1) < AGENTS_FOR {
            return;
        }
        let fresh = self.shed.agents().map_err(|e| e.to_string());
        let mut agents = lock(&self.agents);
        agents.1 = at;
        match fresh {
            Err(e) if agents.0.is_ok() => eprintln!("cooee: kept the agent list: {e}"),
            fresh => agents.0 = fresh,
        }
    }

    /// Agent `id`'s adapter command, from the list as last checked; an id it lacks refreshes it once (#55). Empty
    /// when shed doesn't list it.
    fn adapter(&self, id: &str) -> Vec<String> {
        let find = || {
            let agents = self.agents().ok()?;
            agents.into_iter().find(|a| a.id == id).map(|a| a.cmd)
        };
        find()
            .or_else(|| {
                self.refresh(now(), true);
                find()
            })
            .unwrap_or_default()
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

    /// `folder`, resolved, if it is a root or inside one; none (empty) stays empty: the thread's scratch folder.
    fn place(&self, folder: &Path) -> Result<PathBuf, HubError> {
        if folder.as_os_str().is_empty() {
            return Ok(PathBuf::new());
        }
        self.allowed(folder).ok_or_else(|| outside(folder))
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

    /// Starts a thread: ACP agent `agent` in `folder` (inside a root; empty: its own scratch folder) as a new shed terminal, as `recipe` says, then
    /// sends `text`. Returns the thread's id. A retry with the same `random_id` (after a timeout, or a crash) adopts
    /// the thread it started and never starts another.
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
        recipe: &Recipe,
    ) -> Result<String, HubError> {
        let folder = self.place(folder)?;
        self.start(agent, &folder, text, random_id, recipe)
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
        recipe: &Recipe,
    ) -> Result<String, HubError> {
        let started = self.start_in(agent, folder, (text, random_id), recipe);
        // The adapter's shell removes it as it starts; a start that never got that far leaves nothing either.
        if let Some((file, _)) = &recipe.launch.secrets {
            let _ = fs::remove_file(file);
        }
        started
    }

    fn start_in(
        &self,
        agent: &str,
        folder: &Path,
        (text, random_id): (&str, &str),
        recipe: &Recipe,
    ) -> Result<String, HubError> {
        self.shed.hello()?;
        let (id, fresh) = self.admit(agent, folder, text, random_id, recipe)?;
        self.connect(&id, (fresh, false), text, random_id, recipe)?;
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
        recipe: Recipe,
    ) -> Result<String, HubError> {
        let folder = self.place(folder)?;
        let (id, fresh) = self.admit(agent, &folder, text, random_id, &recipe)?;
        if fresh {
            let live = self.live(&id)?;
            lock(&live).pending = Some(text.to_owned());
        }
        self.retry(&id, recipe, name)?;
        Ok(id)
    }

    /// Starts thread `id`'s agent again after its start failed (none failed, or one runs: nothing), and sends its
    /// first message; as [`Hub::create_later`].
    ///
    /// # Errors
    /// [`HubError::NoThread`]; [`HubError::Store`] when the state can't be published.
    pub fn retry(self: &Arc<Self>, id: &str, recipe: Recipe, name: &str) -> Result<(), HubError> {
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
            let done = hub.shed.hello().map_err(HubError::from);
            let done =
                done.and_then(|()| hub.connect(&id, (fresh, true), &text, &random_id, &recipe));
            // As in [`Hub::start`].
            if let Some((file, _)) = &recipe.launch.secrets {
                let _ = fs::remove_file(file);
            }
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
        recipe: &Recipe,
    ) -> Result<(String, bool), HubError> {
        if text.trim().is_empty() {
            return Err(HubError::Refused("the first message is empty".to_owned()));
        }
        self.offers(agent, &recipe.account, &recipe.model)?;
        let title: String = text
            .trim()
            .lines()
            .next()
            .unwrap_or("")
            .chars()
            .take(60)
            .collect();
        self.mint(random_id, |id| Thread {
            terminal: format!("cooee-{id}"),
            id,
            title,
            agent: agent.to_owned(),
            folder: Some(folder.to_owned()).filter(|f| !f.as_os_str().is_empty()),
            account: recipe.account.clone(),
            profile: recipe.profile.clone(),
            machine: recipe.machine.clone(),
            model: recipe.model.clone(),
            effort: recipe.effort.clone(),
            random_id: random_id.to_owned(),
            room: recipe.room.clone(),
            asked_by: recipe.asked_by.clone(),
            ..Thread::default()
        })
    }

    /// The thread start `random_id` opened, or a new one `make` makes of a new id, stored and listed; and whether
    /// it is new.
    fn mint(
        &self,
        random_id: &str,
        make: impl FnOnce(String) -> Thread,
    ) -> Result<(String, bool), HubError> {
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
        let meta = make(id.clone());
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
        recipe: &Recipe,
    ) -> Result<(), HubError> {
        let live = self.live(id)?;
        let (terminal, agent, folder) = {
            let l = lock(&live);
            let folder = l.meta.folder.clone();
            (l.meta.terminal.clone(), l.meta.agent.clone(), folder)
        };
        // No folder: its own scratch folder, made on its first start.
        let folder = if let Some(f) = folder {
            f
        } else {
            let dir = self.scratch.join(id);
            fs::create_dir_all(&dir).map_err(|e| refused(&dir, &e))?;
            dir
        };
        if fresh || (keep && self.shed.get(&terminal).is_err()) {
            // A member's session is labelled with its room and persona (#154).
            let room = Some(&recipe.room).filter(|r| !r.is_empty());
            let mut labels = BTreeMap::from([
                (LABEL.to_owned(), "1".to_owned()),
                (format!("{LABEL}.thread"), room.map_or(id, |r| r).to_owned()),
            ]);
            if room.is_some() {
                labels.insert(format!("{LABEL}.member"), recipe.profile.clone());
            }
            // A profile's `{thread}` is this thread's id.
            let mut launch = self.mcp_launch(id, recipe)?;
            for (_, v) in &mut launch.env {
                *v = v.replace("{thread}", id);
            }
            if let Err(e) = self.shed.run_acp(
                &terminal,
                (&agent, &self.adapter(&agent)),
                &folder,
                &labels,
                &launch,
            ) && self.shed.get(&terminal).is_err()
            {
                if !keep {
                    lock(&self.threads).remove(id);
                    self.store.remove(id)?;
                }
                return Err(e.into());
            }
        }
        if let Some(s) = self.shed.ready(&terminal)?
            && let Err(e) = self.settle(id, &terminal, &s)
        {
            eprintln!("cooee: {id}: model: {e}");
        }
        self.send_after(id, text, random_id, &[], &recipe.instructions)?;
        Ok(())
    }

    /// Sends the owner's message with its `files`: once per `random_id` (a retry of a stored send returns it
    /// again). The message is stored only once shed took it; the agent gets the files' paths under its text.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a closed thread, an empty message or a stopped agent; [`HubError::Shed`] when shed
    /// fails.
    pub fn send(
        &self,
        id: &str,
        text: &str,
        random_id: &str,
        files: &[Upload],
    ) -> Result<Item, HubError> {
        let live = self.live(id)?;
        if lock(&live).meta.kind == Kind::Room {
            return self.room_send(&live, text, random_id, files);
        }
        self.send_after(id, text, random_id, files, "")
    }

    /// "Use a folder…" (#86): thread `id`'s later turns run in `folder` (inside a root). shed fixes a session's folder
    /// when it starts, so its agent starts again there as `recipe` says, and the next message carries a hand-over:
    /// the profile's instructions, where it was and its last turns. The chat says so; the old terminal is closed.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a folder outside the roots, or a closed or Workflow thread; [`HubError::Shed`]
    /// when the agent can't start there (the thread stays where it was).
    pub fn attach(&self, id: &str, folder: &Path, recipe: &Recipe) -> Result<Item, HubError> {
        let folder = self.allowed(folder).ok_or_else(|| outside(folder))?;
        let live = self.live(id)?;
        let (agent, old) = {
            let l = lock(&live);
            if l.meta.closed || l.meta.workflow.is_some() {
                return Err(HubError::Refused(
                    "only an open chat of your own can move".to_owned(),
                ));
            }
            (l.meta.agent.clone(), l.meta.terminal.clone())
        };
        let terminal = format!("cooee-{id}-{:x}", now());
        let labels = BTreeMap::from([
            (LABEL.to_owned(), "1".to_owned()),
            (format!("{LABEL}.thread"), id.to_owned()),
        ]);
        let started = self
            .mcp_launch(id, recipe)
            .and_then(|launch| {
                let cmd = self.adapter(&agent);
                let run = (agent.as_str(), cmd.as_slice());
                Ok(self
                    .shed
                    .run_acp(&terminal, run, &folder, &labels, &launch)?)
            })
            .and_then(|_| Ok(self.shed.ready(&terminal)?));
        if let Some((file, _)) = &recipe.launch.secrets {
            let _ = fs::remove_file(file);
        }
        let session = started?;
        let mut live = lock(&live);
        let was = live
            .meta
            .folder
            .clone()
            .unwrap_or_else(|| self.scratch.join(id));
        let turns: Vec<String> = live
            .chat
            .items
            .iter()
            .filter_map(|i| match &i.body {
                Body::You { text, .. } => Some(format!("Owner: {text}")),
                Body::Agent { text, .. } => Some(format!("You: {text}")),
                _ => None,
            })
            .collect();
        let last = turns[turns.len().saturating_sub(6)..].join("\n\n");
        let handover = format!(
            "{}\n\nThis chat moved from {} to {}, where you now run; what it made before is still there. Its last turns:\n\n{last}",
            recipe.instructions.trim_end(),
            was.display(),
            folder.display()
        );
        handover.trim_start().clone_into(&mut live.meta.handover);
        live.meta.terminal.clone_from(&terminal);
        live.meta.folder = Some(folder.clone());
        self.store.save(&live.meta)?;
        let text = format!(
            "now in {} · later turns run here",
            cooee_web::short(&folder)
        );
        let item = live.chat.push(now(), 0, Body::Note { text });
        live.publish(self, vec![Change::Item(item.clone())])?;
        drop(live);
        if let Some(s) = session
            && let Err(e) = self.settle(id, &terminal, &s)
        {
            eprintln!("cooee: {id}: model: {e}");
        }
        if let Err(e) = self.shed.close(&old) {
            eprintln!("cooee: {e}");
        }
        Ok(item)
    }

    /// [`Hub::send`] with `before` (a profile's instructions) ahead of the text for the agent; only the text is stored.
    fn send_after(
        &self,
        id: &str,
        text: &str,
        random_id: &str,
        files: &[Upload],
        before: &str,
    ) -> Result<Item, HubError> {
        let live = self.live(id)?;
        let mut live = lock(&live);
        if let Some(item) = Self::sent(&live, random_id) {
            return Ok(item);
        }
        let q = self.take(&live, text, random_id, files)?;
        self.deliver(&mut live, q, before)
    }

    /// The stored message `random_id`, if it was sent.
    fn sent(live: &Live, random_id: &str) -> Option<Item> {
        let mine = |i: &&Item| matches!(&i.body, Body::You { random_id: r, .. } if r == random_id);
        live.chat.items.iter().rev().find(mine).cloned()
    }

    /// A message to send or queue, its files stored.
    fn take(
        &self,
        live: &Live,
        text: &str,
        random_id: &str,
        files: &[Upload],
    ) -> Result<Queued, HubError> {
        if live.meta.closed || (text.trim().is_empty() && files.is_empty()) {
            return Err(HubError::Refused(
                "the thread is closed or the message empty".to_owned(),
            ));
        }
        let dir = self.files.join(&live.meta.id);
        fs::create_dir_all(&dir).map_err(|e| refused(&dir, &e))?;
        let stamp = now();
        let mut names = Vec::new();
        for (i, f) in files.iter().enumerate() {
            let base = f.name.rsplit(['/', '\\']).next().unwrap_or_default();
            let safe: String = base
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || "._-".contains(c) {
                        c
                    } else {
                        '_'
                    }
                })
                .take(80)
                .collect();
            let name = format!("{stamp}-{i}-{}", safe.trim_start_matches('.'));
            let path = dir.join(&name);
            fs::write(&path, &f.data).map_err(|e| refused(&path, &e))?;
            names.push(name);
        }
        Ok(Queued {
            text: text.to_owned(),
            random_id: random_id.to_owned(),
            files: names,
        })
    }

    /// Sends `q` to the agent, `before` ahead of it, then stores and publishes it; refused when the agent stopped,
    /// whose session would drop it (#253).
    fn deliver(&self, live: &mut Live, q: Queued, before: &str) -> Result<Item, HubError> {
        let start = Instant::now();
        let runs = |t: TerminalInfo| t.state == shed_proto::State::Running;
        if live.chat.turn.state == "ended" && !self.shed.get(&live.meta.terminal).is_ok_and(runs) {
            return Err(HubError::Refused(
                "the agent stopped, so it can't take the message".to_owned(),
            ));
        }
        // After Use a folder…, the hand-over goes once ahead of the next message.
        let before = if before.trim().is_empty() {
            live.meta.handover.clone()
        } else {
            before.to_owned()
        };
        let mut prompt = if before.trim().is_empty() {
            q.text.clone()
        } else {
            format!("{}\n\n{}", before.trim_end(), q.text)
        };
        if !q.files.is_empty() {
            prompt.push_str("\n\nAttached files:");
            for name in &q.files {
                let path = self.files.join(&live.meta.id).join(name);
                prompt.push_str("\n- ");
                prompt.push_str(&path.to_string_lossy());
            }
        }
        self.shed.prompt(&live.meta.terminal, prompt.trim())?;
        if !live.meta.handover.is_empty() {
            live.meta.handover.clear();
            self.store.save(&live.meta)?;
        }
        live.sent = Some(start);
        let first = live.meta.random_id == q.random_id;
        let body = Body::You {
            text: q.text,
            random_id: q.random_id,
            files: q.files,
        };
        let item = live.chat.push(now(), 0, body);
        if first {
            live.pending = None;
        }
        live.publish(self, vec![Change::Item(item.clone())])?;
        Ok(item)
    }

    /// Queues the owner's message for when the turn ends (between turns it goes at once); once per `random_id`.
    /// Returns the queue.
    ///
    /// # Errors
    /// As [`Hub::send`].
    pub fn queue(
        &self,
        id: &str,
        text: &str,
        random_id: &str,
        files: &[Upload],
    ) -> Result<Vec<Queued>, HubError> {
        let live = self.live(id)?;
        let mut live = lock(&live);
        let have = live.meta.queue.iter().any(|q| q.random_id == random_id);
        if !have && Self::sent(&live, random_id).is_none() {
            let q = self.take(&live, text, random_id, files)?;
            live.meta.queue.push(q);
            self.queued(&mut live)?;
            self.drain(&mut live)?;
        }
        Ok(live.meta.queue.clone())
    }

    /// Takes queued message `random_id` out of the queue; with `now`, sends it into the turn in progress (steers).
    /// Returns the queue.
    ///
    /// # Errors
    /// As [`Hub::send`]; [`HubError::Refused`] when it isn't queued.
    pub fn unqueue(&self, id: &str, random_id: &str, now: bool) -> Result<Vec<Queued>, HubError> {
        let live = self.live(id)?;
        let mut live = lock(&live);
        let Some(at) = live
            .meta
            .queue
            .iter()
            .position(|q| q.random_id == random_id)
        else {
            return Err(HubError::Refused(
                "that message isn't queued any more".to_owned(),
            ));
        };
        let q = live.meta.queue.remove(at);
        self.queued(&mut live)?;
        if now {
            self.deliver(&mut live, q, "")?;
        }
        Ok(live.meta.queue.clone())
    }

    /// Renames the thread.
    ///
    /// # Errors
    /// [`HubError::Refused`] for an empty title.
    pub fn retitle(&self, id: &str, title: &str) -> Result<(), HubError> {
        let title: String = title.trim().chars().take(80).collect();
        if title.is_empty() {
            return Err(HubError::Refused("a title can't be empty".to_owned()));
        }
        let live = self.live(id)?;
        let mut live = lock(&live);
        live.meta.title = title;
        Ok(self.store.save(&live.meta)?)
    }

    /// Archives the thread (`on`) or restores it.
    ///
    /// # Errors
    /// [`HubError::Store`] when it can't be saved.
    pub fn archive(&self, id: &str, on: bool) -> Result<(), HubError> {
        let live = self.live(id)?;
        let mut live = lock(&live);
        live.meta.archived = on;
        Ok(self.store.save(&live.meta)?)
    }

    /// File `name` sent in thread `id`: its path, if it is one of that thread's.
    ///
    /// # Errors
    /// [`HubError::NoThread`] for no such thread or file.
    pub fn file(&self, id: &str, name: &str) -> Result<PathBuf, HubError> {
        self.live(id)?;
        let plain = !name.starts_with('.')
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c));
        let path = self.files.join(id).join(name);
        if plain && path.is_file() {
            Ok(path)
        } else {
            Err(HubError::NoThread(format!("{id}/{name}")))
        }
    }

    /// `by`'s answer to approval `item`: sent once (stored first) to shed, or to the plugin whose card it is (then
    /// chosen, with the plugin's line; a 409 leaves it closed with the plugin's why). A secret card's `value` goes
    /// to its plugin only and is never kept; a run card's `approve` sends its sha as the `value`. A second tap
    /// returns the item.
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
                plugin: Some(name),
                card,
                ..
            } => self
                .plugins()
                .into_iter()
                .find(|p| &p.name == name)
                .ok_or_else(|| io::Error::other("no longer configured"))
                .and_then(|p| {
                    // A run card's `approve` carries the sha it showed, never the page's value.
                    let value = match card.form.as_deref() {
                        Some("run-card") => card.sha.as_deref().filter(|_| option == "approve"),
                        _ => value,
                    };
                    crate::plugin::answer(&p, id, item, option, by, value)
                })
                .map_err(|e| HubError::Refused(format!("plugin {name}: {e}"))),
            Body::Approval { card, .. }
                if matches!(card.form.as_deref(), Some("mcp" | "skill")) =>
            {
                Ok(self.proposal_answer(id, item, option))
            }
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
    /// or discard, a secret to save or dismiss, a herdsman run card's command to approve or deny (#83), or the
    /// other open threads to pick from. The owner's tap goes to the
    /// plugin; a card the conductor answered itself may be changed for 30 minutes.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a thread of no persona the plugin is attached to (#129), or without text, options, a
    /// config's diff, a secret's name or a run's command and sha.
    pub fn card(&self, plugin: &Plugin, c: CardSpec) -> Result<Item, HubError> {
        let (thread, owner) = (self.thread(&c.thread)?.0, self.plugin_of(&plugin.name));
        let personas = personas::list(self.config_dir(), &self.data);
        let persona = personas.iter().find(|p| p.id == thread.profile);
        if !persona.is_some_and(|p| p.plugins.contains(&owner)) {
            return Err(HubError::Refused(format!(
                "plugin {owner} is not attached to thread {}'s persona",
                c.thread
            )));
        }
        let plain = |o: &str| ApprovalOption {
            id: o.to_owned(),
            name: o.to_owned(),
            kind: String::new(),
        };
        let mut options: Vec<ApprovalOption> = match c.form.as_deref() {
            None => c.options.iter().map(|o| plain(o)).collect(),
            Some("config") => vec![plain("apply"), plain("discard")],
            Some("secret") => vec![plain("save"), plain("dismiss")],
            Some("run-card") => vec![plain("approve"), plain("deny")],
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
        let set = |v: &Option<String>| v.as_deref().is_some_and(|d| !d.trim().is_empty());
        if c.text.trim().is_empty()
            || options.is_empty()
            || (matches!(c.form.as_deref(), Some("config" | "secret" | "run-card"))
                && !set(&c.detail))
            || (c.form.as_deref() == Some("run-card") && !set(&c.sha))
        {
            return Err(HubError::Refused(
                "a card needs text and options (a thread to pick), a config card its diff, a secret card its name \
                 and a run card its command and sha"
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
                sha: c.sha,
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

    /// The owner's model and effort for thread `id` (#553 row 34): kept on the thread and said in it; they apply
    /// from the agent's next start.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a model its agent doesn't offer (#193); [`HubError::Store`] when the thread can't
    /// be saved.
    pub fn pick(&self, id: &str, model: &str, effort: &str) -> Result<Item, HubError> {
        let live = self.live(id)?;
        let mut live = lock(&live);
        self.offers(&live.meta.agent, &live.meta.account, model)?;
        model.clone_into(&mut live.meta.model);
        effort.clone_into(&mut live.meta.effort);
        self.store.save(&live.meta)?;
        let say = |what: &str, pick: &str| {
            format!("{what} {}", if pick.is_empty() { "default" } else { pick })
        };
        let text = format!(
            "{} · {} from the next start",
            say("model", model),
            say("effort", effort)
        );
        let item = live.chat.push(now(), 0, Body::Note { text });
        live.publish(self, vec![Change::Item(item.clone())])?;
        Ok(item)
    }

    /// Cancels the turn in progress; in a room, stop: the member taking its turn is cancelled and the rest dropped.
    ///
    /// # Errors
    /// [`HubError::Shed`] when shed fails.
    pub fn cancel(&self, id: &str) -> Result<(), HubError> {
        let live = self.live(id)?;
        let (kind, terminal) = {
            let l = lock(&live);
            (l.meta.kind.clone(), l.meta.terminal.clone())
        };
        if kind == Kind::Room {
            return self.stop_round(id, "stop", "");
        }
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
        let personas = personas::list(self.config_dir(), &self.data);
        let mut asks = Vec::new();
        let mut rows: Vec<Summary> = self
            .all()
            .iter()
            .filter_map(|live| {
                let live = lock(live);
                // A room's member sessions are the room's (#154).
                if !live.meta.room.is_empty() {
                    let ask = live.chat.items.iter().find_map(|i| match &i.body {
                        Body::Approval {
                            tool,
                            tapped: None,
                            chosen: None,
                            card,
                            ..
                        } if card.line.is_none() => {
                            Some((live.meta.room.clone(), i.speaker.clone(), tool.clone()))
                        }
                        _ => None,
                    });
                    asks.extend(ask);
                    return None;
                }
                let last = live.chat.items.last();
                let text = last.map_or("", |i| match &i.body {
                    Body::Link { text, title, .. } if text.is_empty() => title,
                    Body::You { text, .. }
                    | Body::Agent { text, .. }
                    | Body::Note { text }
                    | Body::Link { text, .. }
                    | Body::Ended { text } => text,
                    Body::Tool { title, .. } => title,
                    Body::Approval { tool, .. } => tool,
                    Body::Plan { .. } => "plan",
                });
                let read = live.meta.read.unwrap_or(0);
                let from = live.chat.items.partition_point(|i| i.id <= read);
                let unseen = live.chat.items[from..]
                    .iter()
                    .filter(|i| !matches!(i.body, Body::You { .. }))
                    .count();
                let waiting = live.chat.items.iter().find_map(|i| match &i.body {
                    Body::Approval { tool, .. } if waits(&i.body) => {
                        Some(format!("{tool} waiting"))
                    }
                    _ => None,
                });
                let persona = personas.iter().find(|p| p.id == live.meta.profile);
                Some(Summary {
                    id: live.meta.id.clone(),
                    title: live.meta.title.clone(),
                    agent: live.meta.agent.clone(),
                    account: live.meta.account.clone(),
                    profile: live.meta.profile.clone(),
                    persona: persona.map(|p| p.id.clone()).unwrap_or_default(),
                    persona_name: persona.map(|p| p.name.clone()).unwrap_or_default(),
                    persona_role: persona.map(|p| p.role).unwrap_or_default(),
                    folder: live.meta.folder.clone().unwrap_or_default(),
                    machine: live.meta.machine.clone(),
                    state: live.chat.turn.state.clone(),
                    last: waiting.unwrap_or_else(|| {
                        text.lines().last().unwrap_or("").chars().take(80).collect()
                    }),
                    at: last.map_or(0, |i| i.at),
                    unread: unseen > 0,
                    unseen,
                    archived: live.meta.archived,
                    workflow: live.meta.workflow.clone(),
                    members: live
                        .meta
                        .members
                        .iter()
                        .map(|m| m.persona.clone())
                        .collect(),
                })
            })
            .collect();
        for (room, persona, tool) in asks {
            let Some(row) = rows.iter_mut().find(|r| r.id == room) else {
                continue;
            };
            let name = personas.iter().find(|p| p.id == persona);
            let name = name.map_or(persona.as_str(), |p| p.name.as_str());
            row.last = format!("{name}: {tool} waiting");
            row.unseen += 1;
            row.unread = true;
        }
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
            queue: live.meta.queue.clone(),
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

    /// The installed plugin `[[plugins]]` entry `tab` is a tab of: its lock entry's name, else its own (a config's).
    #[must_use]
    pub fn plugin_of(&self, tab: &str) -> String {
        let lock = cooee_core::plugins::read_lock(&self.data).unwrap_or_default();
        let found = lock.into_iter().find(|l| l.tabs.iter().any(|t| t == tab));
        found.map_or_else(|| tab.to_owned(), |l| l.name)
    }

    /// The plugins a persona may be attached to: the installed ones (on or off) and the config's.
    #[must_use]
    pub fn installed(&self) -> Vec<String> {
        let lock = cooee_core::plugins::read_lock(&self.data).unwrap_or_default();
        let mut names: Vec<String> = lock.into_iter().map(|l| l.name).collect();
        names.extend(self.plugins().iter().map(|p| self.plugin_of(&p.name)));
        names
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

/// An agent's `propose_server` (#56): a server for the registry, and why.
#[derive(Debug, Deserialize)]
pub struct Proposal {
    /// Its name.
    pub name: String,
    /// `stdio`, `http` or `sse`.
    #[serde(rename = "type", default)]
    pub kind: String,
    /// A stdio server's program.
    #[serde(default)]
    pub command: String,
    /// Its arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// An http or sse server's URL.
    #[serde(default)]
    pub url: String,
    /// What the owner gets from it.
    #[serde(default)]
    pub why: String,
}

/// What an agent proposed, until the owner answers its card.
enum Proposed {
    /// A server for the registry (#56).
    Server(Server),
    /// A skill for the thread's persona (#218), new or the edit of `replaces`.
    Skill {
        /// The persona.
        persona: String,
        /// The skill.
        skill: Skill,
        /// The skill it edits.
        replaces: Option<String>,
    },
}

/// An agent's `skill` proposal (#218): the skill, and the one it edits.
#[derive(Debug, Deserialize)]
pub struct SkillProposal {
    /// The skill.
    #[serde(flatten)]
    pub skill: Skill,
    /// The name of the skill it edits; none: a new one.
    #[serde(default)]
    pub replaces: Option<String>,
}

impl Proposal {
    /// The server it would add: switched off, with no variables or headers (the Mac adds those).
    #[must_use]
    pub fn server(self) -> Server {
        Server {
            name: self.name,
            kind: self.kind,
            command: self.command,
            args: self.args,
            url: self.url,
            env: BTreeMap::new(),
            headers: BTreeMap::new(),
            enabled: false,
        }
    }
}

/// MCP servers (#56): the registry in the config file, a thread's proxies and their grants, and proposals.
impl Hub {
    /// Agents' MCP proxies reach cooee at `url` (`cooee serve`, once it listens).
    pub fn listening(&self, url: String) {
        let _ = self.url.set(url);
    }

    /// The registry, read from the config file now.
    ///
    /// # Errors
    /// [`HubError::Refused`] when the config can't be read.
    pub fn mcp(&self) -> Result<Vec<Server>, HubError> {
        let Some(c) = &self.config else {
            return Ok(Vec::new());
        };
        Config::load(c)
            .map(|c| c.mcp)
            .map_err(|e| HubError::Refused(e.to_string()))
    }

    /// Changes the registry with `f` (its sentence on success), checks and saves it.
    ///
    /// # Errors
    /// [`HubError::Refused`] with `f`'s or the check's reason, or no config file.
    pub fn edit_mcp(
        &self,
        f: impl FnOnce(&mut Vec<Server>) -> Result<String, String>,
    ) -> Result<String, HubError> {
        let mut tools = lock(&self.tools);
        let Some(c) = &self.config else {
            return Err(HubError::Refused(
                "there is no config file to save MCP servers in".to_owned(),
            ));
        };
        let mut list = self.mcp()?;
        let said = f(&mut list).map_err(HubError::Refused)?;
        mcp::check(&list).map_err(HubError::Refused)?;
        mcp::save(c, &list).map_err(|e| refused(c, &e))?;
        tools.clear();
        Ok(said)
    }

    /// The config's folder (its `profiles/`).
    fn config_dir(&self) -> Option<&Path> {
        self.config.as_deref().and_then(Path::parent)
    }

    /// Profile `id`'s picks (none for Plain or a profile gone).
    fn picks(&self, id: &str) -> Vec<Pick> {
        cooee_core::profiles::find(self.config_dir(), &self.data, id)
            .map_or_else(|_| Vec::new(), |p| p.mcp)
    }

    /// Saves profile `id`'s picks (an owner's profile only), each naming a server in the registry.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a plugin's profile, an unknown server or a file that can't be written.
    pub fn set_picks(&self, id: &str, picks: &[Pick]) -> Result<(), HubError> {
        let servers = self.mcp()?;
        if let Some(p) = picks
            .iter()
            .find(|p| !servers.iter().any(|s| s.name == p.server))
        {
            return Err(HubError::Refused(format!(
                "there is no MCP server {}",
                p.server
            )));
        }
        let file = self
            .config_dir()
            .and_then(|c| cooee_core::profiles::file(c, id));
        let file =
            file.ok_or_else(|| HubError::Refused(format!("{id} isn't one of your own profiles")))?;
        mcp::save_picks(&file, picks).map_err(|e| refused(&file, &e))
    }

    /// Server `name`'s tools (name, description), listed once and kept until the registry changes.
    ///
    /// # Errors
    /// Why they couldn't be listed (no such server, a reference that doesn't resolve, no answer).
    pub fn mcp_tools(&self, name: &str) -> cooee_web::Tools {
        if let Some(t) = lock(&self.tools).get(name) {
            return t.clone();
        }
        let s = self
            .mcp()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|s| s.name == name);
        let listed = s
            .ok_or_else(|| format!("no MCP server {name}"))
            .and_then(|s| self.grant_of(s, None))
            .and_then(|g| crate::mcp::tools(&g));
        lock(&self.tools).insert(name.to_owned(), listed.clone());
        listed
    }

    /// `s` resolved, with `tools` allowed.
    fn grant_of(&self, s: Server, tools: Option<Vec<String>>) -> Result<Grant, String> {
        let (env, headers) = s.resolved(&self.data)?;
        Ok(Grant {
            kind: s.kind,
            command: s.command,
            args: s.args,
            url: s.url,
            env,
            headers,
            tools,
        })
    }

    /// `recipe`'s launch for thread `id`, with its MCP servers: cooee's own, then each its profile picks (on, and
    /// resolving now), each a `cooee mcp proxy` holding the thread's token. Before cooee listens: none.
    fn mcp_launch(&self, id: &str, recipe: &Recipe) -> Result<Launch, HubError> {
        let mut launch = recipe.launch.clone();
        let (Some(url), Ok(exe)) = (self.url.get(), std::env::current_exe()) else {
            return Ok(launch);
        };
        let servers = self.mcp()?;
        for name in &recipe.mcp {
            let s = servers.iter().find(|s| &s.name == name && s.enabled);
            let s = s.ok_or_else(|| {
                HubError::Refused(format!(
                    "MCP server {name} is off or gone: settings › mcp servers"
                ))
            })?;
            s.resolved(&self.data).map_err(HubError::Refused)?;
        }
        let var = json!([{ "name": crate::mcp::VAR, "value": format!("{url} {}", crate::mcp::token(&self.mcp_key, id)) }]);
        let exe = exe.to_string_lossy();
        let entry = |name: &str, args: &[&str]| json!({ "name": name, "command": exe, "args": args, "env": var });
        let mut all = vec![entry(COOEE, &["mcp", "serve", "--thread", id])];
        all.extend(
            recipe
                .mcp
                .iter()
                .map(|s| entry(s, &["mcp", "proxy", s, "--thread", id])),
        );
        let file = self
            .data
            .join("secrets")
            .join(format!(".mcp-{id}-{:x}.json", now()));
        launch.mcp = Some((file, all));
        Ok(launch)
    }

    /// Thread `id`, if `token` is its MCP token and it is open.
    fn token_of(&self, id: &str, token: &str) -> Result<Thread, HubError> {
        let no = || {
            HubError::Refused(format!(
                "this MCP token isn't thread {id}'s, or the thread is closed"
            ))
        };
        if token != crate::mcp::token(&self.mcp_key, id) {
            return Err(no());
        }
        let meta = self.thread(id).map_err(|_| no())?.0;
        if meta.closed {
            return Err(no());
        }
        Ok(meta)
    }

    /// The grant of thread `id`'s proxy for `server` (`token`: the thread's): its pick from its profile, the server
    /// on, resolved now.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, a server it doesn't pick, one off, or a reference
    /// that doesn't resolve.
    pub fn grant(&self, id: &str, token: &str, server: &str) -> Result<Grant, HubError> {
        let meta = self.token_of(id, token)?;
        let pick = self
            .picks(&meta.profile)
            .into_iter()
            .find(|p| p.server == server);
        let pick = pick.ok_or_else(|| {
            HubError::Refused(format!(
                "thread {id}'s profile doesn't pick MCP server {server}"
            ))
        })?;
        let s = self
            .mcp()?
            .into_iter()
            .find(|s| s.name == server && s.enabled);
        let s =
            s.ok_or_else(|| HubError::Refused(format!("MCP server {server} is off or gone")))?;
        self.grant_of(s, pick.tools).map_err(HubError::Refused)
    }

    /// An agent's `remember` in thread `id` (#126, #216): an edit of its persona's notes, for its next sessions; the
    /// answer names the note's id and the characters used.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, one with no persona, or an edit refused.
    pub fn remember(&self, id: &str, token: &str, e: &personas::Edit) -> Result<String, HubError> {
        let meta = self.token_of(id, token)?;
        let found = personas::list(self.config_dir(), &self.data)
            .into_iter()
            .find(|p| p.id == meta.profile);
        let p = found.ok_or_else(|| HubError::Refused(format!("thread {id} has no persona")))?;
        let kept = personas::remember(&self.data, &p.id, "persona", e, now())
            .map_err(|e| HubError::Refused(e.to_string()))?;
        let used = personas::used(&personas::notes(&self.data, &p.id));
        let what = kept.map_or_else(|| "Forgotten".to_owned(), |n| format!("Kept as [{}]", n.id));
        Ok(format!(
            "{what}; {used} of {} characters used.",
            personas::CAP
        ))
    }

    /// An agent's `history` in thread `id` (#217): a search of its persona's threads (`query`, at most `limit`
    /// hits), or a page of one of them (`thread`, before item `before`).
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, one with no persona, or a thread of another persona.
    pub fn history(
        &self,
        (id, token): (&str, &str),
        (query, limit): (&str, usize),
        thread: &str,
        before: Option<u64>,
    ) -> Result<String, HubError> {
        let meta = self.token_of(id, token)?;
        let persona = personas::find(self.config_dir(), &self.data, &meta.profile)
            .map_err(|_| HubError::Refused(format!("thread {id} has no persona")))?;
        let refused = |e: cooee_core::StoreError| HubError::Refused(e.to_string());
        if thread.is_empty() && query.trim().is_empty() {
            return Err(HubError::Refused("give a query or a thread".to_owned()));
        }
        if thread.is_empty() {
            let mine: Vec<String> = self
                .all()
                .iter()
                .map(|l| lock(l).meta.clone())
                .filter(|m| m.profile == persona.id)
                .map(|m| m.id)
                .collect();
            return cooee_core::history::search(&self.store, &mine, query, limit).map_err(refused);
        }
        match self.thread(thread) {
            Ok((t, _)) if t.profile == persona.id => {
                cooee_core::history::read(&self.store, thread, before).map_err(refused)
            }
            _ => Err(HubError::Refused(format!(
                "thread {thread} isn't one of {}'s",
                persona.id
            ))),
        }
    }

    /// An agent's `propose_server` in thread `id`: a card with the exact command or URL; nothing runs, and the
    /// server is added (off) only when the owner approves it with Face ID.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, a server that breaks a rule or a name taken.
    pub fn propose(&self, id: &str, token: &str, p: Proposal) -> Result<Item, HubError> {
        self.token_of(id, token)?;
        let why: String = p.why.trim().chars().take(300).collect();
        let s = p.server();
        s.check().map_err(HubError::Refused)?;
        if self.mcp()?.iter().any(|o| o.name == s.name) {
            return Err(HubError::Refused(format!(
                "there is an MCP server {} already",
                s.name
            )));
        }
        let tool = format!("add MCP server {} ({}): {why}", s.name, s.kind);
        self.raise(id, (tool, "mcp", s.shown()), Proposed::Server(s))
    }

    /// An agent's `skill` proposal in thread `id` (#218): a card with the skill's file, or an edit's diff; nothing
    /// is saved until the owner approves it.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, one with no persona, or a skill [`skills::check`]
    /// refuses.
    pub fn propose_skill(
        &self,
        (id, token): (&str, &str),
        p: SkillProposal,
    ) -> Result<Item, HubError> {
        let persona = self.persona_of(id, token)?;
        let old = skills::check(&self.data, &persona, &p.skill, p.replaces.as_deref())
            .map_err(|e| HubError::Refused(e.to_string()))?;
        let verb = if p.replaces.is_some() { "edit" } else { "add" };
        let tool = format!(
            "{verb} {persona}'s skill {}: {}",
            p.skill.name, p.skill.description
        );
        let detail = skills::diff(&old, &p.skill.file());
        let pending = Proposed::Skill {
            persona,
            skill: p.skill,
            replaces: p.replaces,
        };
        self.raise(id, (tool, "skill", detail), pending)
    }

    /// An agent's `skill` read in thread `id` (#218): its persona's skill `name`'s body, at most 8,000 characters.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, one with no persona, or no such skill of its.
    pub fn read_skill(&self, (id, token): (&str, &str), name: &str) -> Result<String, HubError> {
        let persona = self.persona_of(id, token)?;
        let s = skills::read(&self.data, &persona, name)
            .map_err(|e| HubError::Refused(format!("{e} for {persona}")))?;
        Ok(s.body.chars().take(8000).collect())
    }

    /// The id of thread `id`'s persona, for its token.
    fn persona_of(&self, id: &str, token: &str) -> Result<String, HubError> {
        let meta = self.token_of(id, token)?;
        personas::find(self.config_dir(), &self.data, &meta.profile)
            .map(|p| p.id)
            .map_err(|_| HubError::Refused(format!("thread {id} has no persona")))
    }

    /// A proposal's card in thread `id`: what it does, its form and its detail; `p` waits for the owner's answer.
    fn raise(
        &self,
        id: &str,
        (tool, form, detail): (String, &str, String),
        p: Proposed,
    ) -> Result<Item, HubError> {
        let option = |o: &str| ApprovalOption {
            id: o.to_owned(),
            name: o.to_owned(),
            kind: String::new(),
        };
        let body = Body::Approval {
            tool,
            options: vec![option("approve"), option("decline")],
            tapped: None,
            chosen: None,
            plugin: None,
            card: Card {
                form: Some(form.to_owned()),
                detail: Some(detail),
                ..Card::default()
            },
        };
        let live = self.live(id)?;
        let mut live = lock(&live);
        let item = live.chat.push(now(), 0, body);
        live.publish(self, vec![Change::Item(item.clone())])?;
        lock(&self.proposals).insert((id.to_owned(), item.id), p);
        Ok(item)
    }

    /// Whether item `item` of thread `id` is an MCP proposal's card (approving it needs Face ID).
    #[must_use]
    pub fn is_proposal(&self, id: &str, item: u64) -> bool {
        let Ok(live) = self.live(id) else {
            return false;
        };
        let live = lock(&live);
        live.chat.items.iter().any(|i| {
            i.id == item && matches!(&i.body, Body::Approval { card, .. } if card.form.as_deref() == Some("mcp"))
        })
    }

    /// The owner's answer to a proposal: `approve` adds a server, off, or saves a skill.
    fn proposal_answer(&self, id: &str, item: u64, option: &str) -> Result<Option<String>, String> {
        let p = lock(&self.proposals).remove(&(id.to_owned(), item));
        if option != "approve" {
            return Ok(Some("declined".to_owned()));
        }
        match p.ok_or("this proposal ran out (cooee restarted): ask the agent again")? {
            Proposed::Server(s) => self.add_mcp(s).map(Some).map_err(|e| e.to_string()),
            Proposed::Skill {
                persona,
                skill,
                replaces,
            } => skills::save(&self.data, &persona, skill, replaces.as_deref())
                .map(|s| Some(format!("saved skill {}", s.name)))
                .map_err(|e| e.to_string()),
        }
    }

    /// Adds `s` to the registry, off (an agent's proposal, or the phone's `+ server`, both after Face ID).
    ///
    /// # Errors
    /// [`HubError::Refused`] for a name taken or a server that breaks a rule.
    pub fn add_mcp(&self, s: Server) -> Result<String, HubError> {
        self.edit_mcp(|list| {
            if list.iter().any(|o| o.name == s.name) {
                return Err(format!(
                    "there is an MCP server {} already: change it on the Mac",
                    s.name
                ));
            }
            let said = format!(
                "added {}, off: turn it on in settings › mcp servers",
                s.name
            );
            list.push(Server {
                enabled: false,
                ..s
            });
            Ok(said)
        })
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
        hub_on(dir, dir.join("no-shed"))
    }

    /// [`hub`] with `bin` as the shed CLI.
    fn hub_on(dir: &Path, bin: PathBuf) -> Arc<Hub> {
        let shed = Shed::new(bin, Some(dir.join("no-shed-dir")));
        let (looks, _) = cooee_web::Looks::load(None, None);
        let theme = looks.themes[&looks.default].clone();
        Hub::open(
            shed,
            &dir.join("data"),
            (Vec::new(), None),
            theme,
            Vec::new(),
            None,
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
            replay: false,
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

    /// #155 (G2): a room's turns on an owner message stop 10 minutes after it (a fake clock): its queued turns are
    /// dropped, the room told and its pages sent `room-stop`, once.
    #[test]
    fn a_rooms_turns_stop_at_the_deadline() {
        let dir = scratch("deadline");
        let t0 = now();
        let turn = |m: &str| json!({ "member": m, "reason": "round" });
        let meta = json!({ "format": 1, "thread": { "id": "room", "title": "room", "messages": [], "kind": "room",
            "members": [{ "persona": "ada" }, { "persona": "bo" }], "round": [turn("ada"), turn("bo")], "asked_at": t0 } });
        fs::write(dir.join("data/threads/room.json"), meta.to_string()).unwrap();
        fs::write(dir.join("data/threads/room.jsonl"), "").unwrap();
        let hub = hub(&dir);
        let round = |hub: &Hub| hub.thread("room").unwrap().0.round.len();
        hub.watch(t0 + rooms::DEADLINE - 1);
        assert_eq!(round(&hub), 2);
        let (_, mut rx) = hub.subscribe("room", "").unwrap();
        hub.watch(t0 + rooms::DEADLINE);
        assert_eq!(round(&hub), 0);
        let stop = rx.try_recv().unwrap();
        assert_eq!(
            (stop.event, stop.data),
            ("room-stop", json!({ "reason": "deadline" }).to_string())
        );
        let note = rx.try_recv().unwrap();
        assert!(note.data.contains("10 minutes passed"), "{}", note.data);
        hub.watch(t0 + 2 * rooms::DEADLINE);
        assert!(rx.try_recv().is_err());
    }

    /// #155: a member's session that started after its reply was already in is kept, never cancelled; one that
    /// started after [stop] is cancelled (the cancel reaches shed, here none: a shed error), once.
    #[test]
    fn only_a_stop_cancels_a_members_session_as_it_starts() {
        let dir = scratch("seated");
        let turn = |m: &str| json!({ "member": m, "reason": "round" });
        let room = |round: serde_json::Value| {
            json!({ "format": 1, "thread": { "id": "room", "title": "room", "messages": [],
            "kind": "room", "members": [{ "persona": "ada" }, { "persona": "bo" }], "round": round } })
        };
        let session = json!({ "format": 1, "thread": { "id": "s1", "title": "s1", "messages": [], "terminal": "cooee-s1" } });
        fs::write(dir.join("data/threads/s1.json"), session.to_string()).unwrap();
        fs::write(dir.join("data/threads/s1.jsonl"), "").unwrap();
        fs::write(dir.join("data/threads/room.jsonl"), "").unwrap();
        let seated = |hub: &Hub| hub.thread("room").unwrap().0.members[0].session.clone();

        // Ada replied before its start returned: its turn is gone, Bo's is next.
        fs::write(
            dir.join("data/threads/room.json"),
            room(json!([turn("bo")])).to_string(),
        )
        .unwrap();
        let hub = hub(&dir);
        hub.started("room", "ada", Ok("s1".to_owned())).unwrap();
        assert_eq!(seated(&hub), "s1");

        // [stop] while Ada's session starts: nothing to cancel then, so it is cancelled once started.
        fs::write(
            dir.join("data/threads/room.json"),
            room(json!([turn("ada"), turn("bo")])).to_string(),
        )
        .unwrap();
        let hub = self::hub(&dir);
        hub.cancel("room").unwrap();
        let cancelled = hub.started("room", "ada", Ok("s1".to_owned()));
        assert!(matches!(cancelled, Err(HubError::Shed(_))), "{cancelled:?}");
        hub.started("room", "ada", Ok("s1".to_owned())).unwrap();
    }

    /// #55: however often the agent list is read, `shed agents` runs once a minute (a fake clock); a failed run
    /// keeps the list.
    #[test]
    fn the_agent_list_is_refreshed_at_most_once_a_minute() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("agents");
        let (bin, runs) = (dir.join("shed"), dir.join("runs"));
        let cli = format!(
            "#!/bin/sh\necho run >>{runs:?}\n[ -e {:?} ] && exit 1\necho '[{{\"id\":\"a\",\"version\":\"1\"}}]'\n",
            dir.join("fails")
        );
        fs::write(&bin, cli).unwrap();
        fs::set_permissions(&bin, fs::Permissions::from_mode(0o755)).unwrap();
        let hub = hub_on(&dir, bin);
        let count = || fs::read_to_string(&runs).unwrap().lines().count();
        let t0 = now();
        for i in 0..10 {
            hub.refresh(t0 + i * 6_000, false);
            assert_eq!(hub.agents().unwrap()[0].id, "a");
        }
        assert_eq!(count(), 1);
        hub.refresh(t0 + AGENTS_FOR, false);
        assert_eq!(count(), 2);
        fs::write(dir.join("fails"), "").unwrap();
        hub.refresh(t0 + 2 * AGENTS_FOR, false);
        assert_eq!((count(), hub.agents().unwrap()[0].id.as_str()), (3, "a"));
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
