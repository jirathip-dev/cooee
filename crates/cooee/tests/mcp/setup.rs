use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
};

use serde_json::{Value, json};

use crate::support::{Authenticator, Cooee, FakeShed, agent, scratch, version};

pub const SERVER: &str = r#"#!/bin/sh
: >"$(dirname "$0")/ran"
while IFS= read -r line; do
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([^,}]*\).*/\1/p')
  [ -n "$id" ] || continue
  case "$line" in
  *'"method":"initialize"'*) r='{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"fake","version":"1"}}' ;;
  *'"method":"tools/list"'*) r='{"tools":[{"name":"a","description":"tool a","inputSchema":{"type":"object"}},{"name":"b","description":"tool b","inputSchema":{"type":"object"}},{"name":"c","description":"tool c","inputSchema":{"type":"object"}}]}' ;;
  *'"method":"tools/call"'*)
    t=$(printf '%s' "$line" | sed -n 's/.*"name":"\([^"]*\)".*/\1/p')
    if [ -n "$TOKEN" ] && [ "$TOKEN" = "$WANT" ]; then k=matches; else k=differs; fi
    r="{\"content\":[{\"type\":\"text\",\"text\":\"$t ran; the token $k\"}]}" ;;
  *) r='{}' ;;
  esac
  printf '{"jsonrpc":"2.0","id":%s,"result":%s}\n' "$id" "$r"
done
"#;

pub struct Setup {
    pub dir: PathBuf,
    pub shed: FakeShed,
    pub cooee: Cooee,
    pub token: String,
}

pub fn setup(name: &str) -> Setup {
    let dir = scratch(name);
    let token = format!("tk-{}", "q7".repeat(16));
    let data = dir.join("data");
    fs::create_dir_all(data.join("secrets")).unwrap();
    fs::write(data.join("secrets/TOKEN"), format!("{token}\n")).unwrap();
    let server = dir.join("bin/fake-mcp");
    fs::create_dir_all(server.parent().unwrap()).unwrap();
    fs::write(&server, SERVER).unwrap();
    fs::set_permissions(&server, fs::Permissions::from_mode(0o755)).unwrap();
    let coder =
        "name = \"Coder\"\nagent = \"fake-acp\"\n\n[[mcp]]\nserver = \"fs\"\ntools = [\"a\"]\n";
    agent(&dir, "coder", coder, "");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let extra = format!(
        "[[mcp]]\nname = \"fs\"\ncommand = {:?}\nenv = {{ TOKEN = \"secret:TOKEN\", WANT = \"env:WANT_TOKEN\" }}\n",
        server.to_string_lossy()
    );
    let cooee = Cooee::start_env(&dir, &shed.dir, &extra, &[("WANT_TOKEN", &token)]);
    Setup {
        dir,
        shed,
        cooee,
        token,
    }
}

impl Setup {
    pub fn thread(&self, profile: &str) -> (String, Vec<Value>) {
        let random = format!("start-{}", self.shed.cli_log().len());
        let new = json!({ "agent": "fake-acp", "profile": profile, "folder": self.dir.join("root"),
                          "text": "hi", "random_id": random });
        let (status, body) = self.cooee.post("/threads", &new);
        assert_eq!(status, 200, "{body}");
        let id = serde_json::from_str::<Value>(&body).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let file = self.shed.dir.join(format!("mcp-cooee-{id}.json"));
        let servers: Value = serde_json::from_str(&fs::read_to_string(file).unwrap()).unwrap();
        (id, servers.as_array().unwrap().clone())
    }

    pub fn ls(&self) -> String {
        self.cooee.cli(&["mcp", "ls"])
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Agent {
    Claude,
    Codex,
}

pub struct Mcp {
    pub child: Child,
    pub stdin: ChildStdin,
    pub stdout: BufReader<ChildStdout>,
    pub agent: Agent,
    pub next: u64,
}

impl Mcp {
    pub fn spawn(entry: &Value, agent: Agent) -> Self {
        let mut c = Command::new(entry["command"].as_str().unwrap());
        c.args(
            entry["args"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| a.as_str().unwrap()),
        );
        for v in entry["env"].as_array().unwrap() {
            c.env(v["name"].as_str().unwrap(), v["value"].as_str().unwrap());
        }
        let mut child = c
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let (stdin, stdout) = (child.stdin.take().unwrap(), child.stdout.take().unwrap());
        Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
            agent,
            next: 0,
        }
    }

    pub fn start(entry: &Value, agent: Agent) -> Self {
        let mut m = Self::spawn(entry, agent);
        let (version, client) = match agent {
            Agent::Claude => ("2025-06-18", "claude-code"),
            Agent::Codex => ("2025-03-26", "codex-mcp-client"),
        };
        let hello = json!({ "protocolVersion": version, "capabilities": {}, "clientInfo": { "name": client, "version": "1" } });
        m.call("initialize", &hello).expect("initialize");
        writeln!(
            m.stdin,
            r#"{{"jsonrpc":"2.0","method":"notifications/initialized"}}"#
        )
        .unwrap();
        m
    }

    pub fn call(&mut self, method: &str, params: &Value) -> Result<Value, String> {
        let id = match self.agent {
            Agent::Claude => json!(self.next),
            Agent::Codex => json!(format!("codex-{}", self.next)),
        };
        self.next += 1;
        let msg = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        writeln!(self.stdin, "{msg}").unwrap();
        let answer = loop {
            let mut line = String::new();
            self.stdout.read_line(&mut line).unwrap();
            let v: Value =
                serde_json::from_str(&line).unwrap_or_else(|_| panic!("{method}: {line:?}"));
            if v.get("method").is_none() {
                break v;
            }
        };
        assert_eq!(answer["id"], id, "{answer}");
        match answer.get("error") {
            Some(e) => Err(e["message"].as_str().unwrap().to_owned()),
            None => Ok(answer["result"].clone()),
        }
    }

    pub fn tools(&mut self) -> Vec<String> {
        let params = match self.agent {
            Agent::Claude => json!({}),
            Agent::Codex => json!({ "cursor": null }),
        };
        let listed = self.call("tools/list", &params).unwrap();
        let names = listed["tools"].as_array().unwrap().iter();
        names
            .map(|t| t["name"].as_str().unwrap().to_owned())
            .collect()
    }

    pub fn run(&mut self, tool: &str, arguments: &Value) -> Result<String, String> {
        let r = self.call(
            "tools/call",
            &json!({ "name": tool, "arguments": arguments }),
        )?;
        Ok(r["content"][0]["text"].as_str().unwrap().to_owned())
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

pub fn named<'a>(servers: &'a [Value], name: &str) -> &'a Value {
    servers
        .iter()
        .find(|s| s["name"] == name)
        .unwrap_or_else(|| panic!("no {name} in {servers:?}"))
}

pub fn files(dir: &Path, out: &mut String) {
    for e in fs::read_dir(dir).into_iter().flatten().flatten() {
        if e.file_name() == "secrets" {
            continue;
        }
        if e.file_type().unwrap().is_dir() {
            files(&e.path(), out);
        } else if e.file_type().unwrap().is_file() {
            out.push_str(&String::from_utf8_lossy(&fs::read(e.path()).unwrap()));
        }
    }
}

pub const DROPPED: &str = "drop-v4lue-k9q2-unseen";

pub struct SecretDrop {
    pub s: Setup,
    pub id: String,
    pub agent: Mcp,
    pub phone: Authenticator,
}

pub fn drop_setup(name: &str) -> SecretDrop {
    let s = setup(name);
    s.shed.plain("login");
    let (id, servers) = s.thread("coder");
    let agent = Mcp::start(named(&servers, "cooee"), Agent::Claude);
    let phone = Authenticator::new(&s.cooee);
    phone.save(&s.cooee);
    SecretDrop {
        s,
        id,
        agent,
        phone,
    }
}

impl SecretDrop {
    pub fn card(&self) -> Value {
        let items = self.s.cooee.json(&format!("/threads/{}/messages", self.id));
        let cards = items["items"].as_array().unwrap().iter();
        let mut cards = cards.filter(|i| i["form"] == "secret");
        cards.next_back().expect("the secret card").clone()
    }

    pub fn drop_it(&self, card: &Value, passkey: &Value) -> (u16, String) {
        let answer = card["block"]["blocks"][0]["values"]["choice"].clone();
        self.s.cooee.post(
            &format!("/threads/{}/approve", self.id),
            &json!({ "item": card["id"], "option": answer, "value": DROPPED, "passkey": passkey }),
        )
    }

    pub fn events(&self) -> crate::support::Events {
        let messages = self.s.cooee.json(&format!("/threads/{}/messages", self.id));
        let boot = messages["cursor"]
            .as_str()
            .unwrap()
            .split('.')
            .next()
            .unwrap();
        self.s.cooee.events(&self.id, &format!("{boot}.0"), false)
    }

    pub fn told(&self) -> Vec<String> {
        self.s.shed.sent(&format!("cooee-{}", self.id))
    }

    pub fn leaks(&self, events: &[crate::support::Sse]) -> String {
        let mut seen = self
            .s
            .cooee
            .json(&format!("/threads/{}/messages", self.id))
            .to_string();
        seen.extend(events.iter().map(|e| e.data.to_string()));
        seen.extend(self.told());
        files(&self.s.dir, &mut seen);
        seen
    }
}

pub const CHROMIUM: &str = r#"#!/bin/sh
for a in "$@"; do
  case "$a" in --user-data-dir=*) d=${a#--user-data-dir=} ;; esac
done
printf '%s\n' "$d" >>"$(dirname "$0")/chromium.log"
p=$(cat "$(dirname "$0")/port" 2>/dev/null || echo 1)
printf '%s\n/devtools/browser/x\n' "$p" >"$d/DevToolsActivePort"
"#;

pub const PLAYWRIGHT: &str = r#"#!/bin/sh
printf '%s\n' "$*" >>"$(dirname "$0")/playwright.log"
while IFS= read -r line; do
  id=$(printf '%s' "$line" | sed -n 's/.*"id":\([^,}]*\).*/\1/p')
  [ -n "$id" ] || continue
  case "$line" in
  *'"method":"initialize"'*) r='{"protocolVersion":"2025-06-18","capabilities":{"tools":{}},"serverInfo":{"name":"pw","version":"1"}}' ;;
  *'"name":"browser_take_screenshot"'*) printf '%s\n' "$line" >>"$(dirname "$0")/shots.log"; r='{"content":[{"type":"text","text":"shot"},{"type":"image","data":"ZnJhbWUtcGl4ZWxz","mimeType":"image/jpeg"}]}' ;;
  *'"method":"tools/call"'*) printf '%s\n' "$line" >>"$(dirname "$0")/calls.log"; r='{"content":[{"type":"text","text":"done"}]}' ;;
  *) r='{}' ;;
  esac
  printf '{"jsonrpc":"2.0","id":%s,"result":%s}\n' "$id" "$r"
done
"#;

pub const CONTAINER: &str = r#"#!/bin/sh
d=$(dirname "$0")
case "$1" in
ls) printf '[{"configuration":{"id":"ann-desk","labels":{"cooee.provider":"container"}},"status":"running"},{"configuration":{"id":"hand-made","labels":{}},"status":"running"}]' ;;
exec) printf '%s\n' "$*" >>"$d/desk.log"; [ "$5" != import ] || printf 'desk-pixels' ;;
*) exit 1 ;;
esac
"#;

pub fn desk_setup(dir: &Path) -> Setup {
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    fs::write(bin.join("container"), CONTAINER).unwrap();
    fs::set_permissions(bin.join("container"), fs::Permissions::from_mode(0o755)).unwrap();
    for (id, desk) in [
        ("ann", "desk = \"ann-desk\"\n"),
        ("bob", "desk = \"hand-made\"\n"),
        ("cy", ""),
    ] {
        agent(
            dir,
            id,
            &format!("name = \"{id}\"\nagent = \"fake-acp\"\n{desk}"),
            "",
        );
    }
    let secrets = dir.join("data/secrets");
    fs::create_dir_all(&secrets).unwrap();
    let key = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=";
    fs::write(secrets.join("FRAMES_KEY"), key).unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let extra = format!(
        "container = {:?}\n",
        bin.join("container").to_string_lossy()
    );
    Setup {
        cooee: Cooee::start_env(dir, &shed.dir, &extra, &[]),
        dir: dir.to_owned(),
        shed,
        token: String::new(),
    }
}

pub fn fake_page_list(pages: Arc<Mutex<Vec<String>>>) -> u16 {
    let cdp = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = cdp.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for mut c in cdp.incoming().map_while(Result::ok) {
            let head: Vec<String> = BufReader::new(&c)
                .lines()
                .map_while(Result::ok)
                .take_while(|l| !l.trim().is_empty())
                .collect();
            if !head
                .first()
                .is_some_and(|l| l.starts_with("GET /json/list "))
            {
                continue;
            }
            let list: Vec<Value> = pages
                .lock()
                .unwrap()
                .iter()
                .map(|u| json!({ "type": "page", "url": u }))
                .collect();
            let body = json!(list).to_string();
            write!(
                c,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .ok();
        }
    });
    port
}

pub fn frames_setup(dir: &Path, port: u16) -> Setup {
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    fs::write(bin.join("port"), port.to_string()).unwrap();
    for (name, text) in [("chromium", CHROMIUM), ("playwright", PLAYWRIGHT)] {
        fs::write(bin.join(name), text).unwrap();
        fs::set_permissions(bin.join(name), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let toml = "name = \"ann\"\nagent = \"fake-acp\"\n\n[[mcp]]\nserver = \"browser\"\n\n[[sites]]\nsite = \"example.com\"\n\n[[sites]]\nsite = \"bank.com\"\nno_frames = true\n";
    agent(dir, "ann", toml, "");
    let secrets = dir.join("data/secrets");
    fs::create_dir_all(&secrets).unwrap();
    let key = "BwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwcHBwc=";
    fs::write(secrets.join("FRAMES_KEY"), key).unwrap();
    let shed = FakeShed::start(&dir.join("shed"), version());
    let extra = format!(
        "chromium = {:?}\n\n[[mcp]]\nname = \"browser\"\ncommand = {:?}\n",
        bin.join("chromium").to_string_lossy(),
        bin.join("playwright").to_string_lossy()
    );
    let cooee = Cooee::start_env(dir, &shed.dir, &extra, &[]);
    Setup {
        dir: dir.to_owned(),
        shed,
        cooee,
        token: String::new(),
    }
}

pub fn cdp_client(mut s: TcpStream, port: u16, got: &Mutex<Vec<Value>>) {
    let mut r = BufReader::new(s.try_clone().unwrap());
    let mut first = String::new();
    if r.read_line(&mut first).unwrap_or(0) == 0 {
        return;
    }
    let mut line = String::new();
    while r.read_line(&mut line).unwrap() > 2 {
        line.clear();
    }
    if first.starts_with("GET /json/list") {
        let tabs = json!([{ "type": "page", "url": "https://www.example.com/a", "title": "A",
                            "webSocketDebuggerUrl": format!("ws://127.0.0.1:{port}/devtools/page/1") }])
        .to_string();
        let head = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", tabs.len());
        s.write_all((head + &tabs).as_bytes()).unwrap();
        return;
    }
    s.write_all(
        b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\r\n",
    )
    .unwrap();
    let mut head = [0; 2];
    while r.read_exact(&mut head).is_ok() {
        let len = match head[1] & 0x7f {
            126 => {
                let mut b = [0; 2];
                r.read_exact(&mut b).unwrap();
                usize::from(u16::from_be_bytes(b))
            }
            n => usize::from(n),
        };
        let mut mask = [0; 4];
        r.read_exact(&mut mask).unwrap();
        let mut body = vec![0; len];
        r.read_exact(&mut body).unwrap();
        for (i, b) in body.iter_mut().enumerate() {
            *b ^= mask[i % 4];
        }
        let call: Value = serde_json::from_slice(&body).unwrap();
        let event = json!({ "method": "Page.loadEventFired" }).to_string();
        let result = match call["params"]["expression"].as_str() {
            Some(e) if e.contains("elementFromPoint") => {
                json!({ "result": { "value": "Sign in" } })
            }
            Some(_) => json!({ "result": { "value": "Email" } }),
            None => json!({ "data": "anBlZw==" }),
        };
        let reply = json!({ "id": call["id"], "result": result }).to_string();
        got.lock().unwrap().push(call);
        for m in [event, reply] {
            s.write_all(&[0x81, u8::try_from(m.len()).unwrap()])
                .unwrap();
            s.write_all(m.as_bytes()).unwrap();
        }
    }
}

pub fn fake_cdp() -> (u16, Arc<Mutex<Vec<Value>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let got: Arc<Mutex<Vec<Value>>> = Arc::default();
    let g = Arc::clone(&got);
    thread::spawn(move || {
        for s in listener.incoming().map_while(Result::ok) {
            let g = Arc::clone(&g);
            thread::spawn(move || cdp_client(s, port, &g));
        }
    });
    (port, got)
}

pub fn screen_setup(name: &str) -> (Setup, PathBuf, Arc<Mutex<Vec<Value>>>) {
    let dir = scratch(name);
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    for (name, text) in [("chromium", CHROMIUM), ("playwright", PLAYWRIGHT)] {
        fs::write(bin.join(name), text).unwrap();
        fs::set_permissions(bin.join(name), fs::Permissions::from_mode(0o755)).unwrap();
    }
    let (port, cdp) = fake_cdp();
    fs::write(bin.join("port"), port.to_string()).unwrap();
    let toml = "name = \"ann\"\nagent = \"fake-acp\"\n\n[[mcp]]\nserver = \"browser\"\n\n[[sites]]\nsite = \"example.com\"\n";
    agent(&dir, "ann", toml, "");
    let shed = FakeShed::start(&dir.join("shed"), version());
    let extra = format!(
        "chromium = {:?}\n\n[[mcp]]\nname = \"browser\"\ncommand = {:?}\n",
        bin.join("chromium").to_string_lossy(),
        bin.join("playwright").to_string_lossy()
    );
    let cooee = Cooee::start_env(&dir, &shed.dir, &extra, &[]);
    let s = Setup {
        dir: dir.clone(),
        shed,
        cooee,
        token: String::new(),
    };
    (s, bin, cdp)
}

pub fn assert_sent(cdp: &Mutex<Vec<Value>>) {
    let sent: Vec<(String, Value)> = cdp
        .lock()
        .unwrap()
        .iter()
        .map(|c| {
            let params = match c["params"]["expression"].as_str() {
                Some(e) => json!(e.rsplit_once(")(").unwrap().1),
                None => c["params"].clone(),
            };
            (c["method"].as_str().unwrap().to_owned(), params)
        })
        .collect();
    let ask = |at: &str| ("Runtime.evaluate".to_owned(), json!(at));
    let mouse = |kind: &str| {
        (
            "Input.dispatchMouseEvent".to_owned(),
            json!({ "type": kind, "x": 10.0, "y": 20.0, "button": "left", "clickCount": 1 }),
        )
    };
    assert_eq!(
        sent,
        [
            (
                "Page.captureScreenshot".to_owned(),
                json!({ "format": "jpeg", "quality": 60 })
            ),
            ask("document.elementFromPoint(10, 20))"),
            mouse("mousePressed"),
            mouse("mouseReleased"),
            ask("document.activeElement)"),
            ("Input.insertText".to_owned(), json!({ "text": "hunter2" })),
        ]
    );
}

pub fn heard(s: &TcpStream) -> (String, Value) {
    let mut r = BufReader::new(s.try_clone().unwrap());
    let mut head = String::new();
    while r.read_line(&mut head).unwrap() > 2 {}
    let len = head.lines().find_map(|l| {
        let l = l.to_ascii_lowercase();
        l.strip_prefix("content-length:")
            .map(|n| n.trim().parse().unwrap())
    });
    let mut body = vec![0; len.unwrap_or(0)];
    r.read_exact(&mut body).unwrap();
    (head, serde_json::from_slice(&body).unwrap_or_default())
}

pub fn answered(msg: &Value, head: &str, token: &str) -> Value {
    let tools = json!([{ "name": "a", "inputSchema": {} }, { "name": "b", "inputSchema": {} },
                       { "name": "c", "inputSchema": {} }]);
    let signed = head
        .to_ascii_lowercase()
        .contains(&format!("authorization: bearer {token}\r\n"));
    let k = if signed { "matches" } else { "differs" };
    let result = match msg["method"].as_str().unwrap_or_default() {
        "initialize" => json!({ "protocolVersion": "2025-06-18", "capabilities": { "tools": {} } }),
        "tools/list" => json!({ "tools": tools }),
        "tools/call" => {
            json!({ "content": [{ "type": "text", "text": format!("{} ran; the token {k}", msg["params"]["name"].as_str().unwrap()) }] })
        }
        _ => json!({}),
    };
    json!({ "jsonrpc": "2.0", "id": msg["id"], "result": result })
}

pub fn fake_web(token: String) -> (u16, Arc<Mutex<Vec<String>>>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let seen: Arc<Mutex<Vec<String>>> = Arc::default();
    let open: Arc<Mutex<Option<std::sync::mpsc::Sender<Value>>>> = Arc::default();
    let log = Arc::clone(&seen);
    thread::spawn(move || {
        for mut s in l.incoming().map(Result::unwrap) {
            let (log, open, token) = (Arc::clone(&log), Arc::clone(&open), token.clone());
            thread::spawn(move || {
                let (head, msg) = heard(&s);
                log.lock().unwrap().push(format!("{head}{msg}"));
                let path = head.split([' ', '?']).nth(1).unwrap().to_owned();
                let reply = |status: &str, more: &str, body: &str| {
                    format!(
                        "HTTP/1.1 {status}\r\n{more}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                };
                let answer = answered(&msg, &head, &token);
                let out = match (path.as_str(), msg["method"].as_str()) {
                    ("/sse", _) => {
                        let (tx, rx) = std::sync::mpsc::channel();
                        *open.lock().unwrap() = Some(tx);
                        write!(s, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\nevent: endpoint\ndata: /messages?s=1\n\n").unwrap();
                        for m in rx {
                            write!(s, "event: message\ndata: {m}\n\n").unwrap();
                        }
                        return;
                    }
                    ("/messages", _) => {
                        if msg.get("id").is_some() {
                            let tx = open.lock().unwrap().clone().unwrap();
                            tx.send(answer).unwrap();
                        }
                        reply("202 Accepted", "", "")
                    }
                    ("/moved", _) => reply(
                        "307 Temporary Redirect",
                        &format!("Location: http://127.0.0.1:{port}/mcp?redirected\r\n"),
                        "",
                    ),
                    (_, Some("tools/list")) => {
                        let note = json!({ "jsonrpc": "2.0", "method": "notifications/message", "params": {} });
                        let body = format!(
                            "event: message\ndata: {note}\n\nevent: message\ndata: {answer}\n\n"
                        );
                        reply("200 OK", "Content-Type: text/event-stream\r\n", &body)
                    }
                    (_, Some("initialize")) => reply(
                        "200 OK",
                        "Content-Type: application/json\r\nMcp-Session-Id: s-1\r\n",
                        &answer.to_string(),
                    ),
                    _ if msg.get("id").is_none() => reply("202 Accepted", "", ""),
                    _ => reply(
                        "200 OK",
                        "Content-Type: application/json\r\n",
                        &answer.to_string(),
                    ),
                };
                s.write_all(out.as_bytes()).unwrap();
            });
        }
    });
    (port, seen)
}
