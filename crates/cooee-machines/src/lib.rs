//! cooee's first-party plugin `machines` (docs/plugins.md): the health of the machines shed runs on, from shed only
//! (`shed machine ls`, `shed [--on <m>] machine stats --since <t>`; shed does the SSH, cooee never does). One card per
//! machine: online or offline with when it was last seen, load per CPU, memory pressure, swap, free disk, shed's
//! terminals by state and 24 h sparklines that open a chart. A machine going offline or over a threshold sends one
//! push (tag `machine`) per state and puts a dot on the tab. `cooee serve` runs it when a `[[plugins]]` entry is
//! named `machines`.

#![forbid(unsafe_code)]

mod html;

use std::{
    collections::BTreeMap,
    io::{self, BufRead, BufReader, Read, Write},
    net::{SocketAddr, TcpListener, TcpStream},
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex, PoisonError},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use cooee_core::{
    Plugin,
    shed_proto::{MachineStats, Sample},
};
use serde_json::{Value, json};

pub use html::{ago, render};

/// Under this much free disk (10 GB, in MiB) a machine alerts.
pub const DISK_LOW_MB: u64 = 10 * 1024;
/// At this memory pressure (percent of memory not available) or more a machine alerts.
pub const MEMORY_CRITICAL_PCT: u64 = 90;
/// The history kept and drawn: 24 h, in seconds.
pub const DAY: u64 = 86_400;

const TAB: &str = include_str!("tab.js");

/// One machine's card: what shed last said about it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Card {
    /// Its name: this machine's host name, or the one `shed machine add` gave it.
    pub name: String,
    /// Whether it answered the last time.
    pub online: bool,
    /// When it last answered, unix seconds.
    pub last_seen: Option<u64>,
    /// Its last sample, kept while it is offline.
    pub now: Option<Sample>,
    /// Its samples of the last 24 h, oldest first.
    pub history: Vec<Sample>,
    /// Why shed couldn't be asked, if it couldn't.
    pub error: Option<String>,
}

/// What puts a machine in alert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    /// It didn't answer.
    Offline,
    /// Under [`DISK_LOW_MB`] free.
    DiskLow,
    /// At [`MEMORY_CRITICAL_PCT`] or more.
    MemoryCritical,
}

impl Card {
    /// What is wrong with it now.
    #[must_use]
    pub fn problems(&self) -> Vec<Problem> {
        match (&self.now, self.online) {
            (_, false) => vec![Problem::Offline],
            (None, true) => Vec::new(),
            (Some(s), true) => [
                (s.disk_free_mb < DISK_LOW_MB, Problem::DiskLow),
                (
                    s.mem_pressure_pct >= MEMORY_CRITICAL_PCT,
                    Problem::MemoryCritical,
                ),
            ]
            .into_iter()
            .filter_map(|(bad, p)| bad.then_some(p))
            .collect(),
        }
    }

    fn alert(&self, now: u64) -> String {
        let s = self.now.clone().unwrap_or_default();
        let what: Vec<String> = self
            .problems()
            .into_iter()
            .map(|p| match p {
                Problem::Offline => match self.last_seen {
                    Some(t) => format!("offline, last seen {}", html::ago(now, t)),
                    None => "offline, never seen".to_owned(),
                },
                Problem::DiskLow => format!("disk {} free", html::size(s.disk_free_mb)),
                Problem::MemoryCritical => {
                    format!("memory pressure critical ({}%)", s.mem_pressure_pct)
                }
            })
            .collect();
        format!("{}: {}", self.name, what.join("; "))
    }
}

/// The plugin's state: shed, the cards, and the problems each machine was last pushed for.
pub struct Machines {
    shed: PathBuf,
    dir: Option<PathBuf>,
    local: String,
    cards: Mutex<Vec<Card>>,
    pushed: Mutex<BTreeMap<String, Vec<Problem>>>,
    dot: Mutex<Option<bool>>,
}

/// cooee, as the plugin reaches it: its address and the plugin's token.
pub struct Cooee {
    /// cooee's address.
    pub addr: SocketAddr,
    /// The plugin's token.
    pub token: String,
}

impl Cooee {
    /// POSTs `body` to `path` with the token; the status.
    ///
    /// # Errors
    /// When cooee can't be reached or doesn't answer HTTP.
    pub fn post(&self, path: &str, body: &Value) -> io::Result<u16> {
        let body = body.to_string();
        let mut s = TcpStream::connect_timeout(&self.addr, Duration::from_secs(5))?;
        s.set_read_timeout(Some(Duration::from_secs(10)))?;
        write!(
            s,
            "POST {path} HTTP/1.0\r\nHost: {}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\n\
             Content-Length: {}\r\n\r\n{body}",
            self.addr,
            self.token,
            body.len()
        )?;
        let mut line = String::new();
        BufReader::new(s).read_line(&mut line)?;
        line.split(' ')
            .nth(1)
            .and_then(|c| c.parse().ok())
            .ok_or_else(|| io::Error::other("cooee's answer isn't HTTP"))
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Unix seconds now.
#[must_use]
pub fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

impl Machines {
    /// The plugin for the `shed` binary `shed` (with `--dir dir`), this machine named `local`.
    #[must_use]
    pub fn new(shed: PathBuf, dir: Option<PathBuf>, local: String) -> Self {
        Self {
            shed,
            dir,
            local,
            cards: Mutex::default(),
            pushed: Mutex::default(),
            dot: Mutex::default(),
        }
    }

    fn shed(&self, args: &[&str]) -> Result<Value, String> {
        let mut cmd = Command::new(&self.shed);
        if let Some(dir) = &self.dir {
            cmd.arg("--dir").arg(dir);
        }
        let out = cmd
            .arg("--json")
            .args(args)
            .output()
            .map_err(|e| format!("{}: {e}", self.shed.display()))?;
        serde_json::from_slice(&out.stdout)
            .map_err(|_| String::from_utf8_lossy(&out.stderr).trim().to_owned())
    }

    /// The cards as last polled.
    #[must_use]
    pub fn cards(&self) -> Vec<Card> {
        lock(&self.cards).clone()
    }

    /// The machines added to shed (`shed machine ls`), by name: the ones `--on` reaches; none when shed can't say.
    #[must_use]
    pub fn added(&self) -> Vec<String> {
        let added = self.shed(&["machine", "ls"]).unwrap_or_default();
        let added = added.as_array().into_iter().flatten();
        added
            .filter_map(|m| Some(m["name"].as_str()?.to_owned()))
            .collect()
    }

    /// Asks shed about this machine and each added one, keeping 24 h of samples per machine (only the new ones
    /// are asked for); the new cards.
    pub fn poll(&self, now: u64) -> Vec<Card> {
        let old = self.cards();
        let cards: Vec<Card> = std::iter::once(None)
            .chain(self.added().into_iter().map(Some))
            .map(|on| {
                let name = on.clone().unwrap_or_else(|| self.local.clone());
                let was = old.iter().find(|c| c.name == name).cloned();
                let mut card = was.unwrap_or(Card {
                    name,
                    ..Card::default()
                });
                card.history.retain(|s| s.at + DAY > now);
                let after = card
                    .history
                    .last()
                    .map_or(now.saturating_sub(DAY), |s| s.at);
                let since = after.to_string();
                let mut args = vec!["machine", "stats", "--since", &since];
                if let Some(on) = &on {
                    args.splice(0..0, ["--on", on.as_str()]);
                }
                let answer = self.shed(&args);
                let stats = answer
                    .as_ref()
                    .ok()
                    .filter(|v| v["online"] != false)
                    .and_then(|v| serde_json::from_value::<MachineStats>(v.clone()).ok());
                card.online = stats.is_some();
                card.error = answer.as_ref().err().filter(|e| !e.is_empty()).cloned();
                if let Some(s) = stats {
                    let new = s.history.into_iter().filter(|s| s.at > after);
                    card.history.extend(new);
                    card.last_seen = Some(s.now.at);
                    card.now = Some(s.now);
                } else {
                    let seen = answer.ok().and_then(|v| v["last_seen"].as_u64());
                    card.last_seen = seen.or(card.last_seen);
                }
                card
            })
            .collect();
        lock(&self.cards).clone_from(&cards);
        cards
    }

    /// Polls, then tells cooee: one push (tag `machine`) when a machine's problems change to a new non-empty set,
    /// and the tab's dot on while any machine has one. A push that fails is tried again next time.
    pub fn tick(&self, now: u64, cooee: &Cooee) {
        let cards = self.poll(now);
        let mut pushed = lock(&self.pushed);
        for card in &cards {
            let problems = card.problems();
            if pushed.get(&card.name) == Some(&problems) {
                continue;
            }
            if !problems.is_empty() {
                let push = json!({ "text": card.alert(now), "path": card.name, "tag": "machine" });
                match cooee.post("/plugin/push", &push) {
                    Ok(200..=299) => {}
                    r => {
                        eprintln!("cooee-machines: push for {}: {r:?}", card.name);
                        continue;
                    }
                }
            }
            pushed.insert(card.name.clone(), problems);
        }
        let dot = cards.iter().any(|c| !c.problems().is_empty());
        let mut was = lock(&self.dot);
        if *was != Some(dot)
            && matches!(
                cooee.post("/plugin/dot", &json!({ "on": dot })),
                Ok(200..=299)
            )
        {
            *was = Some(dot);
        }
    }
}

/// Answers cooee's calls on `listener`, each with the plugin's `token` or refused (401): `GET /tab.js` (the module)
/// and `GET /cards` (the cards as HTML).
pub fn serve(listener: &TcpListener, token: &str, machines: &Arc<Machines>) {
    for stream in listener.incoming().flatten() {
        let (token, machines) = (token.to_owned(), Arc::clone(machines));
        thread::spawn(move || {
            if let Err(e) = answer(stream, &token, &machines) {
                eprintln!("cooee-machines: {e}");
            }
        });
    }
}

fn answer(stream: TcpStream, token: &str, machines: &Machines) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    let mut r = BufReader::new(stream);
    let (mut request, mut line, mut auth) = (String::new(), String::new(), false);
    r.read_line(&mut request)?;
    while r.read_line(&mut line)? > 0 && !line.trim_end().is_empty() {
        if let Some((k, v)) = line.split_once(':') {
            auth |= k.eq_ignore_ascii_case("authorization")
                && v.trim().strip_prefix("Bearer ") == Some(token);
        }
        line.clear();
    }
    let mut words = request.split(' ');
    let (method, path) = (words.next(), words.next());
    let (status, kind, body) = match (auth, method, path) {
        (false, ..) => ("401 Unauthorized", "text/plain", "no token".to_owned()),
        (true, Some("GET"), Some("/tab.js")) => ("200 OK", "text/javascript", TAB.to_owned()),
        (true, Some("GET"), Some("/cards")) => (
            "200 OK",
            "text/html; charset=utf-8",
            render(&machines.cards(), unix_now()),
        ),
        _ => ("404 Not Found", "text/plain", "no such path".to_owned()),
    };
    let mut s = r.into_inner();
    write!(
        s,
        "HTTP/1.0 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )?;
    s.flush()?;
    let _ = s.set_read_timeout(Some(Duration::from_millis(100)));
    let _ = s.read(&mut [0; 1024]);
    Ok(())
}

/// This machine's short host name, or `this machine`.
#[must_use]
pub fn local_name() -> String {
    let host = Command::new("hostname").arg("-s").output();
    let local = host.map_or_else(
        |_| String::new(),
        |o| String::from_utf8_lossy(&o.stdout).trim().to_owned(),
    );
    if local.is_empty() {
        "this machine".to_owned()
    } else {
        local
    }
}

/// Runs the plugin for `plugin` (its backend at its `url`), reaching shed with `shed`/`dir` and cooee at `cooee`:
/// polls once a minute.
///
/// # Errors
/// When the plugin's address can't be bound.
pub fn start(
    plugin: &Plugin,
    shed: PathBuf,
    dir: Option<PathBuf>,
    cooee: SocketAddr,
) -> io::Result<()> {
    let rest = plugin.url.trim_start_matches("http://");
    let listener = TcpListener::bind(rest.split('/').next().unwrap_or(rest))?;
    let machines = Arc::new(Machines::new(shed, dir, local_name()));
    let (m, token) = (Arc::clone(&machines), plugin.token.clone());
    thread::spawn(move || serve(&listener, &token, &m));
    let cooee = Cooee {
        addr: cooee,
        token: plugin.token.clone(),
    };
    thread::spawn(move || {
        loop {
            machines.tick(unix_now(), &cooee);
            thread::sleep(Duration::from_secs(60));
        }
    });
    Ok(())
}
