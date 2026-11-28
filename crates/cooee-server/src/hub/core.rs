use std::{
    collections::{BTreeMap, HashMap},
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError, mpsc},
    time::{Duration, UNIX_EPOCH},
};

use cooee_core::{Body, Chat, Config, Plugin, Shed, ShedError, Store, StoreError, agents::AUTO};

use super::glance::summary;
use super::live::Live;
use super::{areas::Areas, home};
use crate::push::Push;

pub(super) const LAG: usize = 256;

pub(super) const LABEL: &str = "cooee";

pub(super) const STUCK: u64 = 30 * 60 * 1000;

pub(super) const CHANGE: u64 = 30 * 60 * 1000;

pub(super) const TICK: Duration = Duration::from_secs(30);
pub(super) const SERVED_EVERY: Duration = Duration::from_secs(5);

/// Why a thread action failed.
#[derive(Debug, thiserror::Error)]
#[expect(missing_docs, reason = "#737: moved docs")]
pub enum HubError {
    #[error("no thread {0}")]
    NoThread(String),
    #[error("{0}")]
    Refused(String),
    #[error("{0}")]
    Stale(String),
    #[error(transparent)]
    Shed(#[from] ShedError),
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// One page event: its number in the thread, its SSE event name and its JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Out {
    /// The hub-wide rev: counts up from 1 across every thread since this server started.
    pub rev: u64,
    /// `item`, `text` or `state`.
    pub event: &'static str,
    /// The JSON data.
    pub data: String,
}

/// What the hub opens from: an area takes what it needs.
pub struct Seed<'a> {
    /// The shed the threads run on.
    pub shed: Shed,
    /// The providers that read each agent's tool calls.
    pub providers: cooee_core::Providers,
    /// The data folder.
    pub data: &'a Path,
    /// The config: roots, its path, APNs and the approval expiry.
    pub config: &'a Config,
    /// The words a push speaks.
    pub theme: crate::look::Theme,
    /// The plugins that may post cards and get their answers.
    pub plugins: Vec<Plugin>,
}

/// The threads and the shed they run on.
pub struct Hub {
    pub(super) shed: Shed,
    pub(super) providers: cooee_core::Providers,
    pub(super) store: Store,
    pub(super) scratch: PathBuf,
    pub(super) files: PathBuf,
    pub(super) push: Push,
    pub(super) theme: crate::look::Theme,
    pub(super) config: Option<PathBuf>,
    pub(super) boot: String,
    pub(super) threads: Mutex<BTreeMap<String, Arc<Mutex<Live>>>>,
    pub(super) terminals: Mutex<HashMap<String, Arc<Mutex<Live>>>>,
    pub(super) last: Mutex<u64>,
    pub(super) data: PathBuf,
    pub(super) url: OnceLock<String>,
    pub(super) turns: mpsc::Sender<String>,
    pub(super) texts: mpsc::Sender<String>,
    pub(super) home: home::Home,
    pub(super) areas: Areas,
}

pub(crate) fn now() -> u64 {
    cooee_core::clock::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

pub(crate) fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(super) fn on_machine(machine: &str, name: &str) -> String {
    if machine.is_empty() {
        name.to_owned()
    } else {
        format!("{machine}/{name}")
    }
}

pub(super) fn outside(folder: &Path) -> HubError {
    HubError::Refused(format!(
        "{} is not inside a configured root",
        folder.display()
    ))
}

pub(super) fn refused(path: &Path, e: &io::Error) -> HubError {
    HubError::Refused(format!("{}: {e}", path.display()))
}

impl Hub {
    /// Loads the stored threads, re-attaches the open ones to shed and starts the hub's workers.
    #[expect(clippy::missing_errors_doc, reason = "#737: moved docs")]
    pub fn open(seed: Seed<'_>) -> Result<Arc<Self>, HubError> {
        let store = Store::open(seed.data)?;
        let areas = Areas::open(&seed)?;
        let Seed {
            shed,
            providers,
            data,
            config,
            theme,
            ..
        } = seed;
        let (mut threads, mut terminals) = (BTreeMap::new(), HashMap::new());
        for id in store.ids()? {
            let mut meta = store.load(&id)?;
            let default = ["", "System default"].contains(&meta.account.as_str());
            if default {
                let signs_in = providers.for_agent(&meta.agent).is_some();
                if signs_in { AUTO } else { "" }.clone_into(&mut meta.account);
            }
            let terminal = meta.terminal.clone();
            let (items, unseen) = store.glance(&id, meta.read.unwrap_or(0))?;
            let mut live = Live::new(meta, Chat::default());
            live.unloaded = Some(summary(&live.meta, &items, unseen));
            let live = Arc::new(Mutex::new(live));
            terminals.insert(terminal, Arc::clone(&live));
            threads.insert(id, live);
        }
        let (turns, next) = mpsc::channel();
        let (texts, unsent) = mpsc::channel();
        let hub = Arc::new(Self {
            home: home::Home::new(&threads),
            shed,
            providers,
            store,
            scratch: data.join("scratch"),
            files: data.join("files"),
            push: Push::new(data).with_apns(config.apns.clone()),
            theme,
            config: config.path.clone(),
            boot: format!("{:x}", now()),
            threads: Mutex::new(threads),
            terminals: Mutex::new(terminals),
            last: Mutex::new(0),
            data: data.to_owned(),
            url: OnceLock::new(),
            turns,
            texts,
            areas,
        });
        hub.start_workers(next, unsent);
        hub.keep_me();
        Ok(hub)
    }

    pub(super) fn live(&self, id: &str) -> Result<Arc<Mutex<Live>>, HubError> {
        let live = self.entry(id)?;
        self.load(&mut lock(&live))?;
        Ok(live)
    }

    pub(super) fn entry(&self, id: &str) -> Result<Arc<Mutex<Live>>, HubError> {
        lock(&self.threads)
            .get(id)
            .cloned()
            .ok_or_else(|| HubError::NoThread(id.to_owned()))
    }

    pub(super) fn load(&self, live: &mut Live) -> Result<(), StoreError> {
        live.used = now();
        if live.unloaded.is_some() {
            live.fill(self.store.items(&live.meta.id)?);
        }
        Ok(())
    }

    pub(super) fn warm(&self, live: &Mutex<Live>) -> Result<(), StoreError> {
        let id = Some(lock(live)).and_then(|l| l.unloaded.is_some().then(|| l.meta.id.clone()));
        let items = id.map(|id| self.store.items(&id)).transpose()?;
        let mut live = lock(live);
        if let Some(items) = items.filter(|_| live.unloaded.is_some()) {
            live.fill(items);
        }
        Ok(())
    }

    pub(super) fn unload(&self, now: u64) {
        for live in self.all() {
            let mut live = lock(&live);
            live.trim(now);
            live.unload(now);
        }
    }

    pub(super) fn flush_text(&self, id: String) {
        let Some(live) = lock(&self.threads).get(&id).cloned() else {
            return;
        };
        let due = lock(&live).tick(now());
        if let Some(due) = due {
            cooee_core::clock::sleep(Duration::from_millis(due.saturating_sub(now())));
            self.texts.send(id).ok();
        }
    }

    pub(super) fn all(&self) -> Vec<Arc<Mutex<Live>>> {
        lock(&self.threads).values().cloned().collect()
    }

    pub(super) fn insert(&self, id: String, terminal: String, live: &Arc<Mutex<Live>>) {
        lock(&self.terminals).insert(terminal, Arc::clone(live));
        lock(&self.threads).insert(id, Arc::clone(live));
        self.relist(&lock(live));
    }

    pub(super) fn remove(&self, id: &str) {
        if let Some(live) = lock(&self.threads).remove(id) {
            lock(&self.terminals).retain(|_, l| !Arc::ptr_eq(l, &live));
        }
        self.home.forget(id);
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

pub(super) fn waits(body: &Body) -> bool {
    matches!(body, Body::Approval { chosen: None, card, .. } if card.form.as_deref() == Some("config") && card.line.is_none())
}
