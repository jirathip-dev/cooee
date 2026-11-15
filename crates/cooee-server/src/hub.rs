//! The threads, live: each thread's chat, its numbered page events (kept in a ring for replay) and their
//! subscribers, fed by one shed `events` stream for every terminal: cooee's own, and the Workflow sessions other
//! tools start, known by their labels (#33).

use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError, atomic::AtomicBool, mpsc},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use cooee_core::{
    Agent, Body, Change, Chat, Item, Kind, Launch, Plugin, Profile, Queued, Shed, ShedError, Store,
    StoreError, Thread, Turn, Workflow,
    routines::Mark,
    shed_proto::{self, Event, TerminalInfo},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::broadcast;

use crate::push::{Push, note};

mod ask;
mod cards;
mod events;
mod folders;
mod mcp;
mod models;
mod plugins;
mod relaunch;
mod rooms;
mod send;
mod start;
#[cfg(test)]
mod tests;
mod threads;
use mcp::Proposed;
pub use rooms::Starter;

/// How long a thread keeps its page events for a reconnecting page, in milliseconds (#295).
const REPLAY: u64 = 5 * 60 * 1000;

/// How many page events a watching page may fall behind before it is cut off to resume from the replay.
const LAG: usize = 256;

/// How long a thread goes unused before its chat is unloaded, in milliseconds (#295).
const IDLE: u64 = 10 * 60 * 1000;

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
/// How often the installed plugins' backends are asked for their profiles again (#289).
const SERVED_EVERY: Duration = Duration::from_secs(5);

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
    /// The page events of the last [`REPLAY`], each with when it was sent.
    ring: VecDeque<(u64, Out)>,
    /// Made when a page first subscribes, dropped when the chat is unloaded.
    tx: Option<broadcast::Sender<Out>>,
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
    /// When it was last used, in Unix milliseconds.
    used: u64,
    /// Its row's view of the chat while the chat is unloaded (only its turn stays); `None` while loaded.
    unloaded: Option<Glance>,
    /// The messages handed to shed, by `random_id`, until it answers.
    sending: Vec<String>,
    /// The shed events that came while a message was on its way, applied after it.
    held: Vec<Event>,
}

/// What the thread list shows of a thread's chat.
#[derive(Clone)]
struct Glance {
    /// A member session's approval that waits, for its room's row: the room, the persona and the tool.
    ask: Option<(String, String, String)>,
    /// The approval that waits, else the newest item's last line.
    last: String,
    /// Who said the newest item: a room's member or its owner, `""` in a chat.
    speaker: String,
    /// When the newest item came.
    at: u64,
    /// How many items, not the owner's own, came after the read mark.
    unseen: usize,
    routine: String,
}

fn glance(live: &Live) -> Glance {
    let items = &live.chat.items;
    let ask = items.iter().find_map(|i| match &i.body {
        Body::Approval {
            tool,
            tapped: None,
            chosen: None,
            card,
            ..
        } if card.line.is_none() => Some((live.meta.room.clone(), i.speaker.clone(), tool.clone())),
        _ => None,
    });
    let last = items.last();
    let text = last.map_or("", |i| match &i.body {
        Body::Link { text, title, .. } if text.is_empty() => title,
        Body::You { text, .. }
        | Body::Agent { text, .. }
        | Body::Note { text }
        | Body::Link { text, .. }
        | Body::Ask { text, .. }
        | Body::Ended { text } => text,
        Body::Tool { title, .. } => title,
        Body::Approval { tool, .. } => tool,
        Body::Plan { .. } => "plan",
        Body::Routine { name, .. } => name,
    });
    let read = live.meta.read.unwrap_or(0);
    let from = items.partition_point(|i| i.id <= read);
    let waiting = items.iter().find_map(|i| match &i.body {
        Body::Approval { tool, .. } if waits(&i.body) => Some(format!("{tool} waiting")),
        _ => None,
    });
    Glance {
        ask,
        last: waiting
            .unwrap_or_else(|| text.lines().last().unwrap_or("").chars().take(80).collect()),
        at: last.map_or(0, |i| i.at),
        speaker: last.map(|i| i.speaker.clone()).unwrap_or_default(),
        unseen: items[from..]
            .iter()
            .filter(|i| !matches!(i.body, Body::You { .. }))
            .count(),
        routine: routine_of(live),
    }
}

/// What applying a shed event leaves to do once its thread is unlocked: its next queued message, its final reply,
/// its room and persona, and its asker's key.
struct After {
    next: Result<Option<Queued>, HubError>,
    reply: Option<Body>,
    seat: (String, String),
    asked: String,
}

/// The name of the routine that started thread `live`, else of the one that sent its last `you` item (#75).
fn routine_of(live: &Live) -> String {
    let you = live.chat.items.iter().rev().find_map(|i| match &i.body {
        Body::You { routine, .. } => Some(routine),
        _ => None,
    });
    let mark = live.meta.routine.as_ref().or_else(|| you?.as_ref());
    mark.map(|m| m.name.clone()).unwrap_or_default()
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
    /// The routine whose run starts it (#75).
    pub routine: Option<Mark>,
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
    /// Whether shed was last seen on another protocol (#397): its alerts go once per state.
    shed_version: AtomicBool,
    /// The server's default theme, whose words a push says.
    theme: crate::look::Theme,
    roots: Mutex<Vec<PathBuf>>,
    /// The config file the roots are saved in.
    config: Option<PathBuf>,
    plugins: Mutex<Vec<Plugin>>,
    /// `shed agents`, as last checked, and when: a page load never runs the CLI itself (herdsman#337).
    agents: Mutex<(Result<Vec<Agent>, String>, u64)>,
    /// Each installed plugin's served profiles as its backend last answered: a page never asks a backend (#289).
    served: Mutex<HashMap<String, Option<Vec<Profile>>>>,
    boot: String,
    threads: Mutex<BTreeMap<String, Arc<Mutex<Live>>>>,
    /// The same threads by terminal: a shed event locks only its own thread.
    terminals: Mutex<HashMap<String, Arc<Mutex<Live>>>>,
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
    /// Held while the MCP registry is written.
    registry: Mutex<()>,
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

/// Terminal `name` on `machine` (empty: this host): shed names another machine's `<machine>/<name>` (#149).
fn on_machine(machine: &str, name: &str) -> String {
    if machine.is_empty() {
        name.to_owned()
    } else {
        format!("{machine}/{name}")
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
            tx: None,
            moved: now(),
            stuck: false,
            pending: None,
            sent: None,
            stopped_starting: false,
            used: now(),
            unloaded: None,
            sending: Vec::new(),
            held: Vec::new(),
        }
    }

    /// Drops the page events older than [`REPLAY`] before `now`.
    fn trim(&mut self, now: u64) {
        while self.ring.front().is_some_and(|(at, _)| at + REPLAY < now) {
            self.ring.pop_front();
        }
    }

    /// Unloads the chat (keeping its turn and its row's glance) if unused for [`IDLE`] before `now`, with no page
    /// watching and no turn under way.
    fn unload(&mut self, now: u64) {
        let turn = &self.chat.turn;
        let busy = matches!(turn.state.as_str(), "working" | "connecting" | "blocked")
            || !turn.text.is_empty()
            || self.pending.is_some()
            || self.sent.is_some()
            || self.stopped_starting
            || !self.sending.is_empty()
            || !self.held.is_empty()
            || self.tx.as_ref().is_some_and(|t| t.receiver_count() > 0);
        if self.unloaded.is_some() || busy || now < self.used + IDLE {
            return;
        }
        self.unloaded = Some(glance(self));
        let turn = std::mem::take(&mut self.chat.turn);
        self.chat = Chat::default();
        self.chat.turn = turn;
        self.ring.shrink_to_fit();
        self.tx = None;
    }

    fn emit(&mut self, event: &'static str, data: String) {
        self.seq += 1;
        let out = Out {
            seq: self.seq,
            event,
            data,
        };
        let at = now();
        self.trim(at);
        self.ring.push_back((at, out.clone()));
        if let Some(tx) = &self.tx {
            let _ = tx.send(out);
        }
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
            let watched = self.tx.as_ref().is_some_and(|t| t.receiver_count() > 0);
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
        theme: crate::look::Theme,
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
        let (mut threads, mut terminals) = (BTreeMap::new(), HashMap::new());
        for id in store.ids()? {
            let mut meta = store.load(&id)?;
            // Before #48 the system default login was stored by its label.
            if meta.account == "System default" {
                meta.account.clear();
            }
            let terminal = meta.terminal.clone();
            let live = Arc::new(Mutex::new(Live::new(
                meta,
                Chat::from_versions(store.items(&id)?),
            )));
            terminals.insert(terminal, Arc::clone(&live));
            threads.insert(id, live);
        }
        let (turns, next) = mpsc::channel();
        let hub = Arc::new(Self {
            agents: Mutex::new((shed.agents().map_err(|e| e.to_string()), now())),
            served: Mutex::default(),
            shed,
            store,
            scratch: data.join("scratch"),
            files: data.join("files"),
            push: Push::new(data).with_apns(apns),
            shed_version: AtomicBool::new(false),
            theme,
            roots: Mutex::new(roots),
            config,
            plugins: Mutex::new(plugins),
            boot: format!("{:x}", now()),
            threads: Mutex::new(threads),
            terminals: Mutex::new(terminals),
            last: Mutex::new(0),
            data: data.to_owned(),
            url: OnceLock::new(),
            mcp_key,
            proposals: Mutex::default(),
            registry: Mutex::default(),
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
        let asker = Arc::clone(&hub);
        thread::spawn(move || {
            loop {
                let began = Instant::now();
                asker.refresh_served();
                thread::sleep(SERVED_EVERY.saturating_sub(began.elapsed()));
            }
        });
        let ticker = Arc::clone(&hub);
        thread::spawn(move || {
            loop {
                thread::sleep(TICK);
                ticker.refresh(now(), false);
                ticker.watch(now());
                ticker.unload(now());
                let at = now();
                if let Ok(terminals) = ticker.shed.ls(BTreeMap::new()) {
                    ticker.prune(&terminals, at);
                }
            }
        });
        Ok(hub)
    }

    /// Thread `id`, its chat loaded.
    fn live(&self, id: &str) -> Result<Arc<Mutex<Live>>, HubError> {
        let live = lock(&self.threads)
            .get(id)
            .cloned()
            .ok_or_else(|| HubError::NoThread(id.to_owned()))?;
        self.load(&mut lock(&live))?;
        Ok(live)
    }

    /// Marks `live` used, reading its chat back from the store if it was unloaded.
    fn load(&self, live: &mut Live) -> Result<(), StoreError> {
        live.used = now();
        if live.unloaded.is_some() {
            let mut chat = Chat::from_versions(self.store.items(&live.meta.id)?);
            chat.turn = std::mem::take(&mut live.chat.turn);
            live.chat = chat;
            live.unloaded = None;
        }
        Ok(())
    }

    /// Trims every thread's page events to [`REPLAY`] before `now` and unloads the idle threads' chats (#295).
    fn unload(&self, now: u64) {
        for live in self.all() {
            let mut live = lock(&live);
            live.trim(now);
            live.unload(now);
        }
    }

    fn all(&self) -> Vec<Arc<Mutex<Live>>> {
        lock(&self.threads).values().cloned().collect()
    }

    fn insert(&self, id: String, terminal: String, live: &Arc<Mutex<Live>>) {
        lock(&self.terminals).insert(terminal, Arc::clone(live));
        lock(&self.threads).insert(id, Arc::clone(live));
    }

    fn remove(&self, id: &str) {
        if let Some(live) = lock(&self.threads).remove(id) {
            lock(&self.terminals).retain(|_, l| !Arc::ptr_eq(l, &live));
        }
    }

    /// Checks each open thread's terminal: one that runs is attached (its state shown), one that is gone ends the
    /// thread. Returns the oldest applied shed event among the attached, to replay from.
    fn reattach(&self) -> Result<Option<u64>, ShedError> {
        self.shed.hello()?;
        let (at, terminals) = (now(), self.shed.ls(BTreeMap::new())?);
        self.prune(&terminals, at);
        for t in &terminals {
            if !lock(&self.terminals).contains_key(&t.name) {
                self.adopt(t);
            }
        }
        let mut since: Option<u64> = None;
        for live in self.all() {
            let terminal = {
                let l = lock(&live);
                let done =
                    l.meta.closed || l.meta.kind == Kind::Room || l.chat.turn.state == "ended";
                if done {
                    continue;
                }
                l.meta.terminal.clone()
            };
            let got = self.shed.get(&terminal);
            let mut live = lock(&live);
            if let Err(e) = self.load(&mut live) {
                eprintln!("cooee: {e}");
                continue;
            }
            let changes = match got {
                Ok(t) if t.state == shed_proto::State::Running => {
                    since = Some(since.map_or(live.applied, |s| s.min(live.applied)));
                    let r = t.reported.unwrap_or_default();
                    live.chat.turn.state = r.state;
                    live.chat.turn.note = r.note;
                    Vec::new()
                }
                Ok(_) => live.chat.ended(now(), 0, "The agent stopped.".to_owned()),
                Err(e) if e.no_such_terminal() => {
                    live.chat
                        .ended(now(), 0, "The agent is gone (shed restarted).".to_owned())
                }
                Err(e) if e.no_link() => Vec::new(),
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
        self.insert(id, t.name.clone(), &live);
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
            self.remove(&id);
            if let Err(e) = self.store.remove(&id) {
                eprintln!("cooee: {e}");
            }
        }
    }

    /// The push subscriptions and key.
    #[must_use]
    pub fn push(&self) -> &Push {
        &self.push
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

/// `~/x` for a folder under `$HOME` (as given, or resolved: a folder a chat works in is stored resolved); none
/// (empty): `scratch`, a chat's own folder.
fn short(folder: &Path) -> String {
    if folder.as_os_str().is_empty() {
        return "scratch".to_owned();
    }
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let homes = home
        .iter()
        .flat_map(|h| [h.clone(), h.canonicalize().unwrap_or_default()]);
    let homes = homes.filter(|h| h.has_root());
    let rest = homes
        .into_iter()
        .find_map(|h| folder.strip_prefix(h).ok().map(Path::to_path_buf));
    match rest {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
        Some(rest) => format!("~/{}", rest.display()),
        None => folder.display().to_string(),
    }
}
