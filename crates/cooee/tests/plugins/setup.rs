use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    process::Command,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

use serde_json::{Value, json};

use crate::support::{Cooee, FakeShed, HANG, is_item, scratch, version};

pub struct Backend {
    pub port: u16,
    pub seen: Arc<Mutex<Vec<(String, String)>>>,
    pub choice: Arc<Mutex<Option<(u16, Value)>>>,
    pub hang: Arc<AtomicBool>,
}

impl Backend {
    pub fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let choice = Arc::new(Mutex::new(None));
        let hang = Arc::new(AtomicBool::new(false));
        let (log, next, held) = (Arc::clone(&seen), Arc::clone(&choice), Arc::clone(&hang));
        thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let (log, next, held) = (Arc::clone(&log), Arc::clone(&next), Arc::clone(&held));
                thread::spawn(move || answer(stream, &log, &next, &held));
            }
        });
        Self {
            port,
            seen,
            choice,
            hang,
        }
    }

    pub fn choices(&self) -> Vec<Value> {
        self.seen()
            .into_iter()
            .filter(|(h, _)| h.starts_with("post /cooee/choice "))
            .map(|(_, b)| serde_json::from_str(&b).unwrap())
            .collect()
    }

    pub fn seen(&self) -> Vec<(String, String)> {
        self.seen.lock().unwrap().clone()
    }
}

pub fn answer(
    stream: TcpStream,
    log: &Mutex<Vec<(String, String)>>,
    choice: &Mutex<Option<(u16, Value)>>,
    hang: &AtomicBool,
) {
    let mut r = BufReader::new(stream);
    let (mut head, mut line) = (String::new(), String::new());
    while r.read_line(&mut line).unwrap() > 2 {
        head.push_str(&line.to_ascii_lowercase());
        line.clear();
    }
    let len = head
        .lines()
        .find_map(|l| l.strip_prefix("content-length: "))
        .map_or(0, |n| n.trim().parse().unwrap());
    let mut body = vec![0; len];
    r.read_exact(&mut body).unwrap();
    let path = head.split(' ').nth(1).unwrap_or_default().to_owned();
    log.lock()
        .unwrap()
        .push((head, String::from_utf8(body).unwrap()));
    let until = Instant::now() + HANG;
    while matches!(path.as_str(), "/cooee/choice" | "/hang")
        && hang.load(Ordering::SeqCst)
        && Instant::now() < until
    {
        thread::sleep(Duration::from_millis(20));
    }
    if path == "/cooee/choice"
        && let Some((code, v)) = choice.lock().unwrap().take()
    {
        let mut s = r.into_inner();
        write!(
            s,
            "HTTP/1.0 {code} X\r\nContent-Type: application/json\r\n\r\n{v}"
        )
        .ok();
        return;
    }
    if path == "/beats" {
        let mut s = r.into_inner();
        write!(
            s,
            "HTTP/1.0 200 OK\r\nContent-Type: text/event-stream\r\n\r\n"
        )
        .ok();
        while s.write_all(b": beat\n\n").is_ok() {
            thread::sleep(Duration::from_millis(50));
        }
        log.lock()
            .unwrap()
            .push(("closed /beats".to_owned(), String::new()));
        return;
    }
    let (kind, text) = match path.as_str() {
        "/events" => (
            "text/event-stream",
            ": open\n\nid: 7\nevent: tick\ndata: one\ndata: two\n\n".to_owned(),
        ),
        _ => ("application/json", json!({ "path": path }).to_string()),
    };
    let mut s = r.into_inner();
    write!(s, "HTTP/1.0 200 OK\r\nContent-Type: {kind}\r\n\r\n{text}").ok();
}

pub const TOKEN_A: &str = "token-a-0123456789abcdef";

pub const TOKEN_B: &str = "token-b-0123456789abcdef";

pub struct Setup {
    pub dir: std::path::PathBuf,
    pub shed: FakeShed,
    pub cooee: Cooee,
    pub a: Backend,
    pub b: Backend,
}

pub fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    let shed = FakeShed::start(&dir.join("shed"), version());
    let (a, b) = (Backend::start(), Backend::start());
    let plugin = |name: &str, label: &str, port: u16, token: &str| {
        format!(
            "[[plugins]]\nname = \"{name}\"\nlabel = \"{label}\"\nurl = \"http://127.0.0.1:{port}\"\nmodule = \"/tab.js\"\ntoken = \"{token}\"\n"
        )
    };
    let extra = plugin("a", "Alpha", a.port, TOKEN_A) + &plugin("b", "Beta", b.port, TOKEN_B);
    let cooee = Cooee::start_with(&dir, &shed.dir, &extra);
    let chief = json!({ "name": "Chief", "agent": "fake-acp", "plugins": ["a", "b"] });
    let (code, body) = cooee.post("/personas", &chief);
    assert_eq!(code, 200, "{body}");
    Setup {
        dir,
        shed,
        cooee,
        a,
        b,
    }
}

pub fn status(head: &str) -> u16 {
    head[9..12].parse().unwrap()
}

impl Setup {
    pub fn as_backend(&self, path: &str, auth: &str, body: &Value) -> (u16, String) {
        let json = ("Content-Type", "application/json");
        let headers: &[(&str, &str)] = if auth.is_empty() {
            &[json]
        } else {
            &[json, ("Authorization", auth)]
        };
        let (head, body) = self.cooee.raw("POST", path, headers, &body.to_string());
        (status(&head), body)
    }
}

pub fn me(s: &Setup) -> String {
    let devices = s.cooee.json("/devices");
    let me = devices.as_array().unwrap().iter().find(|d| d["me"] == true);
    me.unwrap()["label"].as_str().unwrap().to_owned()
}

pub fn new_thread(s: &Setup, text: &str) -> String {
    thread_of(s, "chief", text)
}

pub fn thread_of(s: &Setup, persona: &str, text: &str) -> String {
    let random = format!("r-{persona}-{text}");
    let new = json!({ "agent": "fake-acp", "profile": persona, "folder": s.dir.join("root/project"), "text": text, "random_id": random });
    let (code, body) = s.cooee.post("/threads", &new);
    assert_eq!(code, 200, "{body}");
    let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_owned();
    let cursor = s.cooee.json(&format!("/threads/{id}/messages"))["cursor"].clone();
    let boot = cursor.as_str().unwrap().split('.').next().unwrap();
    let mut events = s.cooee.events(&id, &format!("{boot}.0"), false);
    events.until(|e| is_item(e, "agent"));
    id
}

impl Setup {
    pub fn card(&self, card: &Value) -> (u16, Value) {
        let (code, body) = self.as_backend("/plugin/card", &format!("Bearer {TOKEN_A}"), card);
        (code, serde_json::from_str(&body).unwrap_or(Value::Null))
    }

    pub fn tap(&self, thread: &str, answer: &Value) -> Value {
        let (code, body) = self
            .cooee
            .post(&format!("/threads/{thread}/approve"), answer);
        assert_eq!(code, 200, "{body}");
        serde_json::from_str(&body).unwrap()
    }
}

pub fn checked(s: &Setup, answer: &Value) -> (bool, String) {
    let file = s.dir.join("answer.json");
    std::fs::write(&file, answer.to_string()).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_cooee"))
        .args(["plugin", "check"])
        .arg(&file)
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}
