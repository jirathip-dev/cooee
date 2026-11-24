//! cooee's desktop app (#715): one server's blocks screens, followed on its device stream and drawn by blocks-desktop.

mod net;

use std::{
    hash::{BuildHasher, RandomState},
    path::PathBuf,
    process::Child,
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, Sender, channel},
    },
    time::Duration,
};

use blocks::{Action, Answer, Block, Frame, Have, Hello, Mirror, Sync, Synced, Took};
use blocks_desktop::{Event, Kit, Stage};
use net::{Net, Reply};
use serde_json::Value;

const SESSIONS: &str = "/screens/sessions";
const PAIR: &str = "/screens/pair";

enum Note {
    Hello(u64, Hello),
    Frame(u64, Frame),
    Ended(u64),
    Synced(Vec<Have>, Synced),
    Got(String, Reply),
    Answered(Reply),
    Places(Vec<(String, String)>),
}

/// The screens of one server: the key shown, live on the stream or as its `GET` answered, and the way back.
pub struct Shell {
    net: Arc<Net>,
    tx: Sender<Note>,
    rx: Receiver<Note>,
    generation: u64,
    child: Option<Child>,
    sid: Option<String>,
    mirror: Mirror,
    key: String,
    back: Vec<String>,
    still: Option<Block>,
    stage: Stage,
    fresh: bool,
    toast: String,
    places: Vec<(String, String)>,
}

impl Drop for Shell {
    fn drop(&mut self) {
        self.stop();
    }
}

impl Shell {
    /// The server at `base` (`http://host:port`) with its cookies in `jar`, starting on the sessions screen.
    #[must_use]
    pub fn new(base: &str, jar: PathBuf) -> Self {
        let (tx, rx) = channel();
        let base = base.trim().trim_end_matches('/').to_owned();
        let mut shell = Self {
            net: Arc::new(Net {
                base,
                jar,
                one: Mutex::new(()),
            }),
            tx,
            rx,
            generation: 0,
            child: None,
            sid: None,
            mirror: Mirror::default(),
            key: String::new(),
            back: vec![],
            still: None,
            stage: Stage::default(),
            fresh: false,
            toast: String::new(),
            places: vec![],
        };
        shell.go(SESSIONS);
        shell.connect();
        shell
    }

    /// The key shown.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }

    /// The screen shown: the stream's copy of a live key, else the `GET` answer.
    #[must_use]
    pub fn screen(&self) -> Option<&Block> {
        self.mirror.screen(&self.key).or(self.still.as_ref())
    }

    /// What the last pass drew.
    #[must_use]
    pub fn drawn(&self) -> Option<&blocks_desktop::Drawn> {
        self.stage.drawn()
    }

    /// The plugins' screens the bar offers, each a label and its path.
    #[must_use]
    pub fn places(&self) -> &[(String, String)] {
        &self.places
    }

    /// The last error or toast.
    #[must_use]
    pub fn toast(&self) -> &str {
        &self.toast
    }

    /// The stream is open.
    #[must_use]
    pub fn connected(&self) -> bool {
        self.child.is_some()
    }

    fn stop(&mut self) {
        if let Some(mut c) = self.child.take() {
            c.kill().ok();
            c.wait().ok();
        }
        self.sid = None;
    }

    /// Opens the device stream again, dropping the one held.
    pub fn connect(&mut self) {
        self.stop();
        self.generation += 1;
        let path = format!("/stream?v={}", blocks::SCHEMA_VERSION);
        let (child, events) = match self.net.stream(&path) {
            Ok(opened) => opened,
            Err(e) => return self.toast = e,
        };
        self.child = Some(child);
        let (g, tx) = (self.generation, self.tx.clone());
        std::thread::spawn(move || {
            for (event, data) in events {
                let note = match event.as_str() {
                    "hello" => serde_json::from_str(&data).map(|h| Note::Hello(g, h)),
                    "message" => serde_json::from_str(&data).map(|f| Note::Frame(g, f)),
                    _ => continue,
                };
                if note.map(|n| tx.send(n)).is_ok_and(|sent| sent.is_err()) {
                    return;
                }
            }
            tx.send(Note::Ended(g)).ok();
        });
    }

    fn spawn(&self, call: impl FnOnce(&Net) -> Note + Send + 'static) {
        let (net, tx) = (Arc::clone(&self.net), self.tx.clone());
        std::thread::spawn(move || tx.send(call(&net)).ok());
    }

    /// Shows `path`, keeping the key shown for the way back.
    pub fn open(&mut self, path: &str) {
        if path != self.key {
            self.back.push(self.key.clone());
        }
        self.go(path);
    }

    /// Goes back to the key shown before.
    pub fn back(&mut self) {
        if let Some(path) = self.back.pop() {
            self.go(&path);
        }
    }

    fn go(&mut self, path: &str) {
        let old = std::mem::replace(&mut self.key, path.to_owned());
        let close = if old == path { vec![] } else { vec![old] };
        for k in &close {
            self.mirror.close(k);
        }
        (self.still, self.stage, self.fresh) = (None, Stage::default(), true);
        self.mirror.open(path);
        if self.sid.is_some() {
            self.sync(close);
        } else {
            self.fetch();
        }
    }

    fn sync(&self, close: Vec<String>) {
        let Some(sid) = self.sid.clone() else { return };
        let open = self.mirror.have();
        self.spawn(move |net| {
            let body = serde_json::to_string(&Sync {
                open: open.clone(),
                close,
            });
            let reply = net.call(&format!("/stream/{sid}"), body.ok().as_deref());
            let synced = reply.ok().and_then(|(_, b)| serde_json::from_str(&b).ok());
            Note::Synced(open, synced.unwrap_or(Synced { keys: vec![] }))
        });
    }

    fn fetch(&self) {
        let key = self.key.clone();
        self.spawn(move |net| {
            let reply = net.call(&key, None);
            Note::Got(key, reply)
        });
    }

    /// Sends `a` from the key shown to its action route; an action with `text` and no `nonce` gets one.
    pub fn act(&mut self, mut a: Action) {
        let route = match self.key.strip_prefix("/p/") {
            Some(rest) => format!("/p/{}/action", rest.split('/').next().unwrap_or_default()),
            None if self.key.starts_with(PAIR) => format!("{PAIR}/action"),
            None => "/screens/action".to_owned(),
        };
        if !self.key.starts_with("/p/") {
            a.screen.clone_from(&self.key);
        }
        if a.values.contains_key("text") && !a.values.contains_key("nonce") {
            let s = RandomState::new();
            let nonce = format!("{:016x}{:016x}", s.hash_one(1), s.hash_one(2));
            a.values.insert("nonce".to_owned(), nonce.into());
        }
        let body = serde_json::to_string(&a).unwrap_or_default();
        self.spawn(move |net| Note::Answered(net.call(&route, Some(&body))));
    }

    /// Takes what came in, waiting up to `wait` for the first; whether anything did.
    pub fn take(&mut self, wait: Duration) -> bool {
        let Ok(first) = self.rx.recv_timeout(wait) else {
            return false;
        };
        self.note(first);
        while let Ok(n) = self.rx.try_recv() {
            self.note(n);
        }
        true
    }

    fn note(&mut self, n: Note) {
        let live = self.generation;
        match n {
            Note::Hello(g, h) if g == live => self.hello(&h),
            Note::Frame(g, f) if g == live => match self.mirror.frame(f) {
                Took::Gap => self.sync(vec![]),
                Took::Patched | Took::Replaced => self.fresh = true,
                Took::Old => {}
            },
            Note::Ended(g) if g == live => {
                let was = self.sid.is_some();
                self.stop();
                if was {
                    self.connect();
                }
            }
            Note::Synced(sent, s) => self.synced(&sent, s),
            Note::Got(key, reply) if key == self.key => self.got(reply),
            Note::Answered(reply) => self.answered(reply),
            Note::Places(p) => self.places = p,
            _ => {}
        }
    }

    fn hello(&mut self, h: &Hello) {
        self.sid = Some(h.sid.clone());
        if let Err(e) = self.mirror.hello(h) {
            self.toast = format!("{e:?}: update cooee or this app");
        }
        self.mirror.open(&self.key);
        self.sync(vec![]);
        self.spawn(|net| Note::Places(places(net)));
    }

    fn synced(&mut self, sent: &[Have], s: Synced) {
        let gone = s.keys.iter().any(|k| k.k == self.key && k.gone.is_some());
        self.mirror.synced(sent, s);
        self.fresh = true;
        if gone && self.still.is_none() {
            self.fetch();
        }
    }

    fn got(&mut self, reply: Reply) {
        match reply {
            Ok((200, body)) => match serde_json::from_str(&body) {
                Ok(b) => (self.still, self.fresh) = (Some(b), true),
                Err(e) => self.toast = format!("{}: {e}", self.key),
            },
            Ok((401, _)) if self.key != PAIR => self.go(PAIR),
            Ok((code, body)) => self.toast = format!("{code}: {body}"),
            Err(e) => self.toast = e,
        }
    }

    fn answered(&mut self, reply: Reply) {
        let (code, body) = match reply {
            Ok(r) => r,
            Err(e) => return self.toast = e,
        };
        let Ok(a) = serde_json::from_str::<Answer>(&body) else {
            return self.toast = format!("{code}: {body}");
        };
        self.toast = a.toast.map(|t| t.title).unwrap_or_default();
        if let Some(still) = self.still.as_mut().filter(|_| !a.ops.is_empty()) {
            self.fresh = blocks::apply(still, &a.ops).is_ok();
        }
        if let Some(to) = a.open {
            if self.sid.is_none() {
                self.connect();
            }
            self.open(&to);
        }
    }

    /// Draws the bar and the screen into `ui` at `unix_now`, then does what the user did.
    pub fn show(&mut self, ui: &mut egui::Ui, kit: &Kit, unix_now: f64) {
        if std::mem::take(&mut self.fresh) {
            let screen = self.mirror.screen(&self.key).or(self.still.as_ref());
            self.stage.set(screen);
        }
        let mut go = None;
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!self.back.is_empty(), egui::Button::new("←"))
                .clicked()
            {
                go = Some(None);
            }
            let tabs = [("Chats".to_owned(), SESSIONS.to_owned())];
            for (label, path) in tabs.iter().chain(&self.places) {
                if ui.selectable_label(self.key == *path, label).clicked() {
                    go = Some(Some(path.clone()));
                }
            }
            ui.label(&self.toast);
        });
        let events = egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .show(ui, |ui| self.stage.show(ui, kit, unix_now))
            .inner;
        match go {
            Some(Some(path)) => self.open(&path),
            Some(None) => self.back(),
            None => {}
        }
        for e in events {
            self.event(e);
        }
    }

    /// Does what the user did on the screen.
    pub fn event(&mut self, e: Event) {
        match e {
            Event::Act(a) => self.act(a),
            Event::Open(path) | Event::Sheet(path) => self.open(&path),
            Event::Thread(id) => self.open(&format!("/screens/chat/{id}")),
            Event::Refresh => self.go(&self.key.clone()),
            Event::Dropped(_) => {}
        }
    }
}

fn places(net: &Net) -> Vec<(String, String)> {
    let Ok((200, body)) = net.call("/plugins", None) else {
        return vec![];
    };
    let list: Value = serde_json::from_str(&body).unwrap_or_default();
    let field = |v: &Value, k: &str| {
        v.get(k)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let all = |v: &Value, k: &str| {
        v.get(k)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    let mut out: Vec<(String, String)> = vec![];
    for p in all(&list, "installed") {
        let name = field(&p, "name");
        for place in all(&p, "places") {
            let (screen, label) = (field(&place, "screen"), field(&place, "label"));
            let path = format!("/p/{name}{screen}");
            let cards = field(&place, "type") == "chat-cards";
            if !cards && !screen.is_empty() && !out.iter().any(|(_, p)| *p == path) {
                let label = if label.is_empty() {
                    name.clone()
                } else {
                    label
                };
                out.push((label, path));
            }
        }
    }
    out
}
