use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use blocks::{Block, Have, Hello, KeyState, MediaType, Outdated, Put, Shelf, Sync, Synced, Why};
use cooee_core::{Device, clock};
use tokio::{
    sync::{
        Mutex as Turn, broadcast,
        broadcast::error::{RecvError, TryRecvError},
        mpsc,
    },
    task::JoinHandle,
};

use crate::{Hub, HubError};

const STREAMS: usize = 4;
const KEYS: usize = 16;
const UNSENT: usize = 1 << 20;
const PACE: Duration = Duration::from_millis(50);
const IDLE: Duration = Duration::from_secs(300);

pub(crate) enum Who {
    Device(Device),
    Asking { claim: String, data: PathBuf },
}

fn asker(claim: &str) -> String {
    format!("claim-{}", cooee_core::sha256(claim.as_bytes()))
}

impl Who {
    fn id(&self) -> String {
        match self {
            Self::Device(d) => d.id.clone(),
            Self::Asking { claim, .. } => asker(claim),
        }
    }

    fn scope(&self) -> String {
        match self {
            Self::Device(_) => String::new(),
            Self::Asking { claim, .. } => format!("{} ", asker(claim)),
        }
    }

    fn streams(&self) -> usize {
        match self {
            Self::Device(_) => STREAMS,
            Self::Asking { .. } => 1,
        }
    }
}

pub(crate) enum Wake {
    Home,
    Every(Duration),
}

enum Waker {
    Hub(broadcast::Receiver<()>),
    Tick(tokio::time::Interval),
}

impl Waker {
    async fn next(&mut self) -> bool {
        match self {
            Self::Hub(wake) => {
                if let Err(RecvError::Closed) = wake.recv().await {
                    return false;
                }
                while !matches!(
                    wake.try_recv(),
                    Err(TryRecvError::Empty | TryRecvError::Closed)
                ) {}
            }
            Self::Tick(every) => {
                every.tick().await;
            }
        }
        true
    }
}

type Build = dyn Fn(&Hub) -> Result<Block, HubError> + Send + std::marker::Sync;

pub(crate) struct Source {
    pub(crate) wake: Wake,
    pub(crate) build: Box<Build>,
}

pub(crate) enum Sent {
    Frame(Arc<str>),
    Bye(Why),
}

struct Tap {
    sid: String,
    device: String,
    scope: String,
    tx: mpsc::UnboundedSender<Sent>,
    unsent: AtomicUsize,
    closed: AtomicBool,
}

impl Tap {
    fn push(&self, frame: &Arc<str>) {
        if self.closed.load(Ordering::Relaxed) {
            return;
        }
        let unsent = self.unsent.fetch_add(frame.len(), Ordering::Relaxed);
        if unsent.saturating_add(frame.len()) > UNSENT {
            self.bye(Why::Evicted);
        } else {
            self.tx.send(Sent::Frame(Arc::clone(frame))).ok();
        }
    }

    fn bye(&self, why: Why) {
        if !self.closed.swap(true, Ordering::Relaxed) {
            self.tx.send(Sent::Bye(why)).ok();
        }
    }
}

pub(crate) struct Frames {
    rx: mpsc::UnboundedReceiver<Sent>,
    tap: Arc<Tap>,
    live: Arc<Live>,
}

impl Frames {
    pub(crate) async fn next(&mut self) -> Option<Sent> {
        let sent = self.rx.recv().await?;
        if let Sent::Frame(f) = &sent {
            self.tap.unsent.fetch_sub(f.len(), Ordering::Relaxed);
        }
        Some(sent)
    }
}

impl Drop for Frames {
    fn drop(&mut self) {
        let mut st = lock(&self.live.state);
        st.end(&self.tap.sid);
    }
}

struct Open {
    tap: Arc<Tap>,
    keys: Vec<String>,
}

struct Held {
    shelf: Arc<Turn<Shelf<()>>>,
    holders: Vec<Arc<Tap>>,
    task: Option<JoinHandle<()>>,
    idle: Option<Instant>,
}

#[derive(Default)]
struct State {
    streams: Vec<Open>,
    keys: HashMap<String, Held>,
}

impl State {
    fn end(&mut self, sid: &str) {
        let Some(at) = self.streams.iter().position(|o| o.tap.sid == sid) else {
            return;
        };
        let open = self.streams.remove(at);
        for k in &open.keys {
            self.release(&format!("{}{k}", open.tap.scope), sid);
        }
    }

    fn release(&mut self, k: &str, sid: &str) {
        let Some(held) = self.keys.get_mut(k) else {
            return;
        };
        held.holders.retain(|t| t.sid != sid);
        if held.holders.is_empty() {
            if let Some(task) = held.task.take() {
                task.abort();
            }
            held.idle = Some(clock::instant());
        }
    }
}

pub(crate) struct Live {
    hub: Arc<Hub>,
    epoch: String,
    next: AtomicU64,
    state: Mutex<State>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Live {
    pub(crate) fn new(hub: Arc<Hub>) -> Self {
        Self {
            hub,
            epoch: format!("{:x}", crate::http::now()),
            next: AtomicU64::new(0),
            state: Mutex::default(),
        }
    }

    pub(crate) fn attach(
        self: &Arc<Self>,
        who: &Who,
        accept: &str,
    ) -> Result<(Hello, Frames), Outdated> {
        let media = MediaType::pick(accept)?;
        let device = who.id();
        let n = self.next.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::unbounded_channel();
        let tap = Arc::new(Tap {
            sid: format!("{}-{n}", self.epoch),
            device: device.clone(),
            scope: who.scope(),
            tx,
            unsent: AtomicUsize::new(0),
            closed: AtomicBool::new(false),
        });
        let mut st = lock(&self.state);
        let mine: Vec<Arc<Tap>> = st
            .streams
            .iter()
            .filter(|o| o.tap.device == device)
            .map(|o| Arc::clone(&o.tap))
            .collect();
        for old in mine
            .iter()
            .take((mine.len() + 1).saturating_sub(who.streams()))
        {
            old.bye(Why::Evicted);
            st.end(&old.sid);
        }
        st.streams.push(Open {
            tap: Arc::clone(&tap),
            keys: vec![],
        });
        let hello = Hello {
            v: media.v,
            sid: tap.sid.clone(),
            epoch: self.epoch.clone(),
            keys: KEYS,
        };
        let live = Arc::clone(self);
        Ok((hello, Frames { rx, tap, live }))
    }

    pub(crate) async fn sync(self: &Arc<Self>, sid: &str, who: &Who, s: Sync) -> Option<Synced> {
        let device = who.id();
        let tap = {
            let mut st = lock(&self.state);
            let open = st
                .streams
                .iter_mut()
                .find(|o| o.tap.sid == sid && o.tap.device == device)?;
            open.keys.retain(|k| !s.close.contains(k));
            let tap = Arc::clone(&open.tap);
            for k in &s.close {
                st.release(&format!("{}{k}", tap.scope), sid);
            }
            st.keys
                .retain(|_, h| h.task.is_some() || h.idle.is_none_or(|i| i.elapsed() < IDLE));
            tap
        };
        let mut synced = Synced::default();
        for have in s.open {
            synced.keys.push(self.open(&tap, who, have).await);
        }
        Some(synced)
    }

    async fn open(self: &Arc<Self>, tap: &Arc<Tap>, who: &Who, have: Have) -> KeyState {
        let gone = |code| KeyState {
            k: have.k.clone(),
            rev: None,
            ops: None,
            screen: None,
            gone: Some(code),
        };
        let Some(source) = crate::screens::source(&have.k, who) else {
            return gone(404);
        };
        let at = format!("{}{}", tap.scope, have.k);
        let kept = {
            let mut st = lock(&self.state);
            let Some(open) = st.streams.iter_mut().find(|o| o.tap.sid == tap.sid) else {
                return gone(404);
            };
            if !open.keys.contains(&have.k) {
                if open.keys.len() >= KEYS {
                    return gone(429);
                }
                open.keys.push(have.k.clone());
            }
            let held = st.keys.entry(at.clone()).or_insert_with(|| Held {
                shelf: Arc::default(),
                holders: vec![],
                task: None,
                idle: None,
            });
            held.idle = None;
            Arc::clone(&held.shelf)
        };
        let mut kept = kept.lock().await;
        let started = lock(&self.state)
            .keys
            .get(&at)
            .is_some_and(|h| h.task.is_some());
        if !started {
            let wake = match source.wake {
                Wake::Home => Waker::Hub(self.hub.home()),
                Wake::Every(every) => Waker::Tick(tokio::time::interval(every)),
            };
            let source = Arc::new(source);
            if let Some(block) = self.build(&have.k, &source).await {
                put(&mut kept, &have.k, &block);
            }
            let task =
                tokio::spawn(Arc::clone(self).run((at.clone(), have.k.clone()), source, wake));
            if let Some(held) = lock(&self.state).keys.get_mut(&at) {
                held.task.replace(task).inspect(JoinHandle::abort);
            }
        }
        {
            let mut st = lock(&self.state);
            let open = st.streams.iter().any(|o| o.tap.sid == tap.sid);
            match st.keys.get_mut(&at) {
                Some(held) if open => {
                    held.holders.retain(|t| t.sid != tap.sid);
                    held.holders.push(Arc::clone(tap));
                }
                _ => {
                    st.release(&at, &tap.sid);
                    return gone(404);
                }
            }
        }
        kept.state(&(), &have.k, have.rev).unwrap_or_else(|e| {
            cooee_core::note!("cooee: stream {}: {e}", have.k);
            gone(500)
        })
    }

    pub(crate) fn hand_over(&self, claim: &str) {
        let id = asker(claim);
        let mut st = lock(&self.state);
        let taps: Vec<Arc<Tap>> = st
            .streams
            .iter()
            .filter(|o| o.tap.device == id)
            .map(|o| Arc::clone(&o.tap))
            .collect();
        for tap in taps {
            tap.bye(Why::Handover);
            st.end(&tap.sid);
        }
    }

    async fn build(&self, k: &str, source: &Arc<Source>) -> Option<Block> {
        let (hub, source) = (Arc::clone(&self.hub), Arc::clone(source));
        match tokio::task::spawn_blocking(move || (source.build)(&hub)).await {
            Ok(Ok(block)) => Some(block),
            Ok(Err(e)) => {
                cooee_core::note!("cooee: stream {k}: {e}");
                None
            }
            Err(e) => {
                cooee_core::note!("cooee: stream {k}: {e}");
                None
            }
        }
    }

    async fn run(self: Arc<Self>, (at, k): (String, String), source: Arc<Source>, mut wake: Waker) {
        while wake.next().await {
            let start = clock::instant();
            if let Some(block) = self.build(&k, &source).await {
                let kept = lock(&self.state)
                    .keys
                    .get(&at)
                    .map(|h| Arc::clone(&h.shelf));
                let Some(kept) = kept else { return };
                let mut kept = kept.lock().await;
                if let Some(frame) = put(&mut kept, &k, &block) {
                    let st = lock(&self.state);
                    for tap in st
                        .keys
                        .get(&at)
                        .map(|h| h.holders.as_slice())
                        .unwrap_or_default()
                    {
                        tap.push(&frame);
                    }
                }
            }
            tokio::time::sleep(PACE.max(start.elapsed().saturating_mul(10))).await;
        }
    }
}

fn put(shelf: &mut Shelf<()>, k: &str, block: &Block) -> Option<Arc<str>> {
    match shelf.put(&(), k, block) {
        Ok(Put::Changed(d)) => Some(d.frame),
        Ok(Put::Same | Put::First) => None,
        Err(e) => {
            cooee_core::note!("cooee: stream {k}: {e}");
            None
        }
    }
}
