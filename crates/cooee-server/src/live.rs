use std::{
    collections::HashMap,
    pin::pin,
    sync::{
        Arc, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use blocks::{Block, Have, Hello, KeyState, MediaType, Outdated, Put, Shelf, Sync, Synced, Why};
use cooee_core::{Device, clock};
use futures_util::future::{Either, select};
use tokio::{
    sync::{
        Mutex as Turn, Notify, broadcast,
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
pub(crate) const PAGES: usize = 10;

pub(crate) enum Who {
    Device(Device),
}

pub(crate) enum Wake {
    Home,
    Thread(String),
}

type Build = dyn Fn(&Hub, usize) -> Result<Block, HubError> + Send + std::marker::Sync;
type Seen = dyn Fn(&Hub) -> Result<(), HubError> + Send + std::marker::Sync;

pub(crate) struct Source {
    pub(crate) wake: Wake,
    pub(crate) build: Box<Build>,
    pub(crate) seen: Option<Box<Seen>>,
}

pub(crate) enum Sent {
    Frame(Arc<str>),
    Bye(Why),
}

struct Tap {
    sid: String,
    device: String,
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
    pages: usize,
    poke: Arc<Notify>,
}

#[derive(Default)]
struct State {
    streams: Vec<Open>,
    keys: HashMap<String, Held>,
}

impl State {
    fn shelf(&mut self, sid: &str, k: &str) -> Result<Arc<Turn<Shelf<()>>>, u16> {
        let Some(open) = self.streams.iter_mut().find(|o| o.tap.sid == sid) else {
            return Err(404);
        };
        if !open.keys.iter().any(|o| o == k) {
            if open.keys.len() >= KEYS {
                return Err(429);
            }
            open.keys.push(k.to_owned());
        }
        let held = self.keys.entry(k.to_owned()).or_insert_with(|| Held {
            shelf: Arc::default(),
            holders: vec![],
            task: None,
            idle: None,
            pages: 1,
            poke: Arc::default(),
        });
        held.idle = None;
        Ok(Arc::clone(&held.shelf))
    }

    fn place(&mut self, sid: &str, k: &str, task: Option<JoinHandle<()>>) -> bool {
        let placed = match (task, self.keys.get_mut(k)) {
            (Some(task), Some(held)) => {
                held.task.replace(task).inspect(JoinHandle::abort);
                true
            }
            (task, _) => {
                task.inspect(JoinHandle::abort);
                false
            }
        };
        if !placed {
            if let Some(open) = self.streams.iter_mut().find(|o| o.tap.sid == sid) {
                open.keys.retain(|o| o != k);
            }
            self.release(k, sid);
        }
        placed
    }

    fn hold(&mut self, tap: &Arc<Tap>, k: &str) -> bool {
        let open = self.streams.iter().any(|o| o.tap.sid == tap.sid);
        match self.keys.get_mut(k) {
            Some(held) if open => {
                held.holders.retain(|t| t.sid != tap.sid);
                held.holders.push(Arc::clone(tap));
                true
            }
            _ => {
                self.release(k, &tap.sid);
                false
            }
        }
    }

    fn end(&mut self, sid: &str) {
        let Some(at) = self.streams.iter().position(|o| o.tap.sid == sid) else {
            return;
        };
        let open = self.streams.remove(at);
        for k in &open.keys {
            self.release(k, sid);
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
        let Who::Device(device) = who;
        let n = self.next.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = mpsc::unbounded_channel();
        let tap = Arc::new(Tap {
            sid: format!("{}-{n}", self.epoch),
            device: device.id.clone(),
            tx,
            unsent: AtomicUsize::new(0),
            closed: AtomicBool::new(false),
        });
        let mut st = lock(&self.state);
        let mine: Vec<Arc<Tap>> = st
            .streams
            .iter()
            .filter(|o| o.tap.device == device.id)
            .map(|o| Arc::clone(&o.tap))
            .collect();
        for old in mine.iter().take((mine.len() + 1).saturating_sub(STREAMS)) {
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
        let Who::Device(device) = who;
        let tap = {
            let mut st = lock(&self.state);
            let open = st
                .streams
                .iter_mut()
                .find(|o| o.tap.sid == sid && o.tap.device == device.id)?;
            open.keys.retain(|k| !s.close.contains(k));
            let tap = Arc::clone(&open.tap);
            for k in &s.close {
                st.release(k, sid);
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
        let kept = match lock(&self.state).shelf(&tap.sid, &have.k) {
            Ok(kept) => kept,
            Err(code) => return gone(code),
        };
        let mut kept = kept.lock().await;
        let started = lock(&self.state)
            .keys
            .get(&have.k)
            .is_some_and(|h| h.task.is_some());
        let source = Arc::new(source);
        if !started {
            let k = have.k.clone();
            let task = match &source.wake {
                Wake::Home => Some(self.start(k, &source, &mut kept, self.hub.home()).await),
                Wake::Thread(id) => match self.hub.subscribe(id, "") {
                    Ok((_, wake)) => Some(self.start(k, &source, &mut kept, wake).await),
                    Err(_) => None,
                },
            };
            if !lock(&self.state).place(&tap.sid, &have.k, task) {
                return gone(404);
            }
        }
        if !lock(&self.state).hold(tap, &have.k) {
            return gone(404);
        }
        let state = kept.state(&(), &have.k, have.rev).unwrap_or_else(|e| {
            cooee_core::note!("cooee: stream {}: {e}", have.k);
            gone(500)
        });
        drop(kept);
        self.seen(&have.k, &source).await;
        state
    }

    async fn start<T: Clone + Send + 'static>(
        self: &Arc<Self>,
        k: String,
        source: &Arc<Source>,
        kept: &mut Shelf<()>,
        wake: broadcast::Receiver<T>,
    ) -> JoinHandle<()> {
        if let Some(block) = self.build(&k, source).await {
            put(kept, &k, &block);
        }
        tokio::spawn(Arc::clone(self).run(k, Arc::clone(source), wake))
    }

    async fn seen(&self, k: &str, source: &Arc<Source>) {
        if source.seen.is_none() {
            return;
        }
        let (hub, source) = (Arc::clone(&self.hub), Arc::clone(source));
        let seen = move || source.seen.as_ref().map_or(Ok(()), |seen| seen(&hub));
        match tokio::task::spawn_blocking(seen).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => cooee_core::note!("cooee: stream {k}: {e}"),
            Err(e) => cooee_core::note!("cooee: stream {k}: {e}"),
        }
    }

    pub(crate) fn more(&self, k: &str) -> bool {
        let mut st = lock(&self.state);
        let Some(held) = st
            .keys
            .get_mut(k)
            .filter(|h| h.task.is_some() && h.pages < PAGES)
        else {
            return false;
        };
        held.pages = held.pages.saturating_add(1);
        held.poke.notify_one();
        true
    }

    async fn build(&self, k: &str, source: &Arc<Source>) -> Option<Block> {
        let (hub, source) = (Arc::clone(&self.hub), Arc::clone(source));
        let pages = lock(&self.state).keys.get(k).map_or(1, |h| h.pages);
        match tokio::task::spawn_blocking(move || (source.build)(&hub, pages)).await {
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

    async fn run<T: Clone + Send + 'static>(
        self: Arc<Self>,
        k: String,
        source: Arc<Source>,
        mut wake: broadcast::Receiver<T>,
    ) {
        let Some(poke) = lock(&self.state).keys.get(&k).map(|h| Arc::clone(&h.poke)) else {
            return;
        };
        loop {
            let closed = {
                let (heard, poked) = (pin!(wake.recv()), pin!(poke.notified()));
                let first = select(heard, poked).await;
                matches!(first, Either::Left((Err(RecvError::Closed), _)))
            };
            if closed {
                return;
            }
            while !matches!(
                wake.try_recv(),
                Err(TryRecvError::Empty | TryRecvError::Closed)
            ) {}
            let start = clock::instant();
            if let Some(block) = self.build(&k, &source).await {
                let kept = lock(&self.state).keys.get(&k).map(|h| Arc::clone(&h.shelf));
                let Some(kept) = kept else { return };
                let mut kept = kept.lock().await;
                if let Some(frame) = put(&mut kept, &k, &block) {
                    {
                        let st = lock(&self.state);
                        for tap in st
                            .keys
                            .get(&k)
                            .map(|h| h.holders.as_slice())
                            .unwrap_or_default()
                        {
                            tap.push(&frame);
                        }
                    }
                    drop(kept);
                    self.seen(&k, &source).await;
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
