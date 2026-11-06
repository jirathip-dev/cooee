//! Test support: a fake shed server on a Unix socket (shed's JSON-RPC and events, with a scripted ACP agent per
//! terminal), a fake `shed` CLI beside it, and a tiny HTTP client for `cooee serve`.

#![allow(dead_code, reason = "each test file uses part of the support")]

use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    os::unix::{fs::PermissionsExt, net::UnixListener},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::Duration,
};

use cooee_core::shed_proto::{
    ApprovalOption, ErrorCode, Event, EventKind, Hello, Reported, Request, Response, RpcError,
    State, TerminalInfo, VERSION,
};
use serde_json::{Value, json};

/// What the scripted agent of a terminal is told.
#[derive(Debug, Clone, PartialEq)]
enum Input {
    Prompt(String),
    Approve(String),
    Cancel,
}

#[derive(Default)]
struct Shared {
    seq: u64,
    ring: Vec<Event>,
    subs: Vec<Sender<Event>>,
    reported: BTreeMap<String, Reported>,
    stopped: BTreeSet<String>,
    gone: BTreeSet<String>,
    agents: BTreeMap<String, Sender<Input>>,
}

/// A fake shed server in `dir` (`dir/shed.sock`, the CLI at `dir/shed`).
#[derive(Clone)]
pub struct FakeShed {
    pub dir: PathBuf,
    version: String,
    shared: Arc<Mutex<Shared>>,
    /// Every request (method, params) in order.
    pub requests: Arc<Mutex<Vec<(String, Value)>>>,
}

const CLI: &str = r#"#!/bin/sh
# The fake `shed` CLI: `agents` lists one agent; `run` records its arguments and prints the terminal.
printf '%s\n' "$*" >>"$(dirname "$0")/cli.log"
case "$*" in
*" agents") echo '[{"id":"fake-acp","version":"1.0.0","distribution":"npx","cmd":["fake-acp"]}]' ;;
*" run "*)
  while [ $# -gt 0 ]; do [ "$1" = --name ] && name=$2; shift; done
  printf '{"name":"%s","labels":{},"cwd":"/","cmd":[],"keep":false,"state":"running","exit":null,"pid":1,"acp":"fake-acp"}\n' "$name" ;;
*) echo "fake shed: $*" >&2; exit 2 ;;
esac
"#;

impl FakeShed {
    /// Starts a fake shed of `version` in `dir`.
    pub fn start(dir: &Path, version: &str) -> Self {
        fs::create_dir_all(dir).unwrap();
        let cli = dir.join("shed");
        fs::write(&cli, CLI).unwrap();
        fs::set_permissions(&cli, fs::Permissions::from_mode(0o755)).unwrap();
        let socket = dir.join("shed.sock");
        let _ = fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).unwrap();
        let shed = Self {
            dir: dir.to_owned(),
            version: version.to_owned(),
            shared: Arc::default(),
            requests: Arc::default(),
        };
        let server = shed.clone();
        thread::spawn(move || {
            for conn in listener.incoming().flatten() {
                let server = server.clone();
                thread::spawn(move || server.serve(conn));
            }
        });
        shed
    }

    /// The fake CLI's argument lines.
    pub fn cli_log(&self) -> String {
        fs::read_to_string(self.dir.join("cli.log")).unwrap_or_default()
    }

    /// The `send` texts to `name`, in order.
    pub fn sent(&self, name: &str) -> Vec<String> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|(m, p)| m == "send" && p["name"] == name)
            .map(|(_, p)| p["text"].as_str().unwrap().to_owned())
            .collect()
    }

    fn emit(&self, name: &str, kind: EventKind) {
        let mut s = self.shared.lock().unwrap();
        s.seq += 1;
        let labels = BTreeMap::from([("cooee".to_owned(), "1".to_owned())]);
        let event = Event {
            seq: s.seq,
            name: name.to_owned(),
            labels,
            kind,
        };
        s.subs.retain(|tx| tx.send(event.clone()).is_ok());
        s.ring.push(event);
    }

    fn report(&self, name: &str, state: &str, note: Option<&str>) {
        let r = Reported {
            state: state.to_owned(),
            note: note.map(str::to_owned),
        };
        self.shared
            .lock()
            .unwrap()
            .reported
            .insert(name.to_owned(), r.clone());
        self.emit(name, EventKind::State(r));
    }

    fn info(name: &str, running: bool, reported: Option<Reported>) -> Value {
        json!(TerminalInfo {
            name: name.to_owned(),
            labels: BTreeMap::new(),
            cwd: PathBuf::from("/"),
            cmd: Vec::new(),
            keep: false,
            state: if running {
                State::Running
            } else {
                State::Exited
            },
            exit: (!running).then_some(143),
            pid: running.then_some(1),
            reported,
            idle_after: None,
            resources: None,
            acp: Some("fake-acp".to_owned()),
        })
    }

    /// A terminal by name: any name runs until it is killed or exits; a removed one is gone.
    fn terminal(&self, name: &str) -> Result<Value, RpcError> {
        let s = self.shared.lock().unwrap();
        if s.gone.contains(name) {
            return Err(RpcError::new(ErrorCode::NoSuchTerminal, name));
        }
        let running = !s.stopped.contains(name);
        Ok(Self::info(name, running, s.reported.get(name).cloned()))
    }

    fn serve(&self, conn: std::os::unix::net::UnixStream) {
        let mut writer = conn.try_clone().unwrap();
        let mut lines = BufReader::new(conn).lines();
        while let Some(Ok(line)) = lines.next() {
            let req: Request = serde_json::from_str(&line).unwrap();
            self.requests
                .lock()
                .unwrap()
                .push((req.method.clone(), req.params.clone()));
            let id = req.id.clone().unwrap_or_default();
            let name = req.params["name"].as_str().unwrap_or("").to_owned();
            let result = match req.method.as_str() {
                "hello" => {
                    let theirs: Hello = serde_json::from_value(req.params.clone()).unwrap();
                    let ours = Hello {
                        product: "shed".to_owned(),
                        version: self.version.clone(),
                    };
                    if theirs == ours {
                        Ok(json!(ours))
                    } else {
                        Err(RpcError::version(&theirs.label(), &ours.label()))
                    }
                }
                "get" => self.terminal(&name),
                "wait" => self
                    .terminal(&name)
                    .map(|t| json!({ "outcome": "matched", "terminal": t })),
                "state" => {
                    let note = req.params["note"].as_str();
                    self.report(&name, req.params["state"].as_str().unwrap(), note);
                    self.terminal(&name)
                }
                "send" => {
                    self.input(&name, req.params["text"].as_str().unwrap());
                    Ok(json!({}))
                }
                "kill" => {
                    self.shared.lock().unwrap().stopped.insert(name.clone());
                    self.emit(&name, EventKind::Killed);
                    self.terminal(&name)
                }
                "rm" => {
                    let t = self.terminal(&name);
                    self.shared.lock().unwrap().gone.insert(name);
                    t
                }
                "events" => {
                    let since = req.params["since"].as_u64();
                    let (tx, rx) = mpsc::channel();
                    let replay: Vec<Event> = {
                        let mut s = self.shared.lock().unwrap();
                        s.subs.push(tx);
                        s.ring
                            .iter()
                            .filter(|e| since.is_some_and(|n| e.seq > n))
                            .cloned()
                            .collect()
                    };
                    let ok = Response::ok(id, json!({}));
                    writeln!(writer, "{}", json!(ok)).unwrap();
                    for event in replay.into_iter().chain(rx) {
                        if writeln!(writer, "{}", json!(event)).is_err() {
                            return;
                        }
                    }
                    return;
                }
                other => Err(RpcError::new(ErrorCode::MethodNotFound, other)),
            };
            let response = match result {
                Ok(r) => Response::ok(id, r),
                Err(e) => Response::err(id, e),
            };
            if writeln!(writer, "{}", json!(response)).is_err() {
                return;
            }
        }
    }

    /// A framed input (`RS`, JSON, newline) for terminal `name`'s agent.
    pub fn input(&self, name: &str, text: &str) {
        let v: Value = serde_json::from_str(text.strip_prefix('\x1e').unwrap().trim_end()).unwrap();
        let input = match (&v["prompt"], &v["approve"]) {
            (Value::String(p), _) => Input::Prompt(p.clone()),
            (_, Value::String(o)) => Input::Approve(o.clone()),
            _ => Input::Cancel,
        };
        let mut s = self.shared.lock().unwrap();
        let tx = s.agents.entry(name.to_owned()).or_insert_with(|| {
            let (tx, rx) = mpsc::channel();
            let shed = self.clone();
            let name = name.to_owned();
            thread::spawn(move || shed.agent(&name, &rx));
            tx
        });
        tx.send(input).unwrap();
    }

    /// The scripted agent: a prompt's words pick the script.
    fn agent(&self, name: &str, rx: &Receiver<Input>) {
        while let Ok(input) = rx.recv() {
            let Input::Prompt(prompt) = input else {
                continue;
            };
            let turn = EventKind::Turn {
                prompt: prompt.clone(),
            };
            self.emit(name, turn);
            let Some((events, stop)) = self.script(name, &prompt, rx) else {
                continue;
            };
            for event in events {
                self.emit(name, event);
            }
            self.report(name, "done", Some(stop));
        }
    }

    /// One turn's script: the events that end it and its stop reason (`None` when the agent exits).
    fn script(
        &self,
        name: &str,
        prompt: &str,
        rx: &Receiver<Input>,
    ) -> Option<(Vec<EventKind>, &'static str)> {
        let delay = Duration::from_millis(
            env::var("FAKE_SHED_DELAY_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
        );
        let read = "Read src/lib.rs";
        Some(if prompt.contains("approve") {
            self.approval(name, rx)
        } else if prompt.contains("tool") {
            let events = [
                tool("c1", read, "pending"),
                tool("c1", read, "in_progress"),
                tool("c1", read, "completed"),
                text("Read "),
                text("it."),
                reply("Read it.", "end_turn"),
            ];
            (events.to_vec(), "end_turn")
        } else if prompt.contains("plan") {
            let entry = |c: &str, s: &str| json!({ "content": c, "priority": "high", "status": s });
            let entries = vec![
                entry("Read the code", "completed"),
                entry("Fix the bug", "in_progress"),
            ];
            let events = vec![EventKind::Plan { entries }, reply("Planned.", "end_turn")];
            (events, "end_turn")
        } else if prompt.contains("hang") {
            self.emit(name, tool("c3", "cargo test", "in_progress"));
            self.emit(name, text("w0 "));
            while rx.recv().is_ok_and(|i| i != Input::Cancel) {}
            (vec![reply("w0 ", "cancelled")], "cancelled")
        } else if let Some(n) = prompt.strip_prefix("slow ") {
            let n: usize = n.trim().parse().unwrap();
            let mut stop = "end_turn";
            for i in 0..n {
                thread::sleep(delay);
                if rx.try_recv() == Ok(Input::Cancel) {
                    stop = "cancelled";
                    break;
                }
                self.emit(name, text(&format!("w{i} ")));
            }
            // The reply waits for a stop, so a page can be read with the whole stream, however late it reconnects.
            if stop == "end_turn" {
                while rx.recv().is_ok_and(|i| i != Input::Cancel) {}
            }
            (vec![reply(&format!("streamed {n} words"), stop)], stop)
        } else if prompt.contains("exit") {
            self.shared.lock().unwrap().stopped.insert(name.to_owned());
            self.emit(name, EventKind::Exited { code: Some(1) });
            return None;
        } else {
            let events = vec![text("Hel"), text("lo"), reply("Hello", "end_turn")];
            (events, "end_turn")
        })
    }

    /// Asks before an edit and waits for `approve` (or a cancel).
    fn approval(&self, name: &str, rx: &Receiver<Input>) -> (Vec<EventKind>, &'static str) {
        let call = "Edit src/lib.rs";
        self.emit(name, tool("c2", call, "pending"));
        let option = |id: &str, name: &str, kind: &str| ApprovalOption {
            id: id.to_owned(),
            name: name.to_owned(),
            kind: kind.to_owned(),
        };
        let options = vec![
            option("allow", "Allow", "allow_once"),
            option("always", "Always allow", "allow_always"),
            option("reject", "Reject", "reject_once"),
        ];
        let tool_name = call.to_owned();
        self.emit(
            name,
            EventKind::ApprovalRequest {
                tool: tool_name,
                options,
            },
        );
        self.report(name, "blocked", Some(call));
        let Ok(Input::Approve(option)) = rx.recv() else {
            return (vec![reply("", "cancelled")], "cancelled");
        };
        let done = if option == "reject" {
            "failed"
        } else {
            "completed"
        };
        self.emit(name, EventKind::Approved { option });
        self.report(name, "working", None);
        (
            vec![tool("c2", call, done), reply("Edited.", "end_turn")],
            "end_turn",
        )
    }
}

fn text(t: &str) -> EventKind {
    EventKind::Text { text: t.to_owned() }
}

fn tool(id: &str, title: &str, status: &str) -> EventKind {
    EventKind::ToolCall {
        id: id.to_owned(),
        title: title.to_owned(),
        status: status.to_owned(),
    }
}

fn reply(t: &str, stop: &str) -> EventKind {
    EventKind::Reply {
        text: t.to_owned(),
        stop_reason: stop.to_owned(),
    }
}

/// A scratch folder, empty.
pub fn scratch(name: &str) -> PathBuf {
    let dir = env::temp_dir().join(format!("cooee-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// `cooee serve` on a free port with data in `dir/data`, the fake shed in `shed`, and the root `dir/root`; paired
/// (`cookie` goes with every request).
pub struct Cooee {
    pub child: Child,
    pub addr: String,
    pub config: PathBuf,
    pub cookie: String,
    /// Run as `--instance <name>` with `HOME` here and no `--config`, if set.
    pub instance: Option<(String, PathBuf)>,
}

impl Cooee {
    pub fn start(dir: &Path, shed: &Path) -> Self {
        Self::start_with(dir, shed, "")
    }

    /// [`Cooee::start`] with more config lines.
    pub fn start_with(dir: &Path, shed: &Path, extra: &str) -> Self {
        let data = format!("data = {:?}\n{extra}", dir.join("data").to_string_lossy());
        Self::launch(dir, shed, dir.join("config.toml"), &data, None)
    }

    /// `cooee serve --instance <name>` with `HOME` at `dir/home`: its config at the instance's default path, its
    /// data in the instance's default folder.
    pub fn start_instance(dir: &Path, shed: &Path, name: &str) -> Self {
        let home = dir.join("home");
        let config = home.join(".config/cooee").join(name).join("config.toml");
        fs::create_dir_all(config.parent().unwrap()).unwrap();
        Self::launch(dir, shed, config, "", Some((name.to_owned(), home)))
    }

    fn command(&self) -> Command {
        Self::command_for(&self.config, self.instance.as_ref())
    }

    fn command_for(config: &Path, instance: Option<&(String, PathBuf)>) -> Command {
        let mut c = Command::new(env!("CARGO_BIN_EXE_cooee"));
        match instance {
            Some((name, home)) => c
                .args(["--instance", name])
                .env("HOME", home)
                .env_remove("XDG_CONFIG_HOME")
                .env_remove("XDG_DATA_HOME"),
            None => c.arg("--config").arg(config),
        };
        c
    }

    fn launch(
        dir: &Path,
        shed: &Path,
        config: PathBuf,
        extra: &str,
        instance: Option<(String, PathBuf)>,
    ) -> Self {
        let root = dir.join("root");
        fs::create_dir_all(root.join("project")).unwrap();
        let q = |p: &Path| format!("{:?}", p.to_string_lossy());
        let toml = format!(
            "port = 0\nroots = [{}]\nshed = {}\nshed_dir = {}\n{extra}",
            q(&root),
            q(&shed.join("shed")),
            q(shed),
        );
        fs::write(&config, toml).unwrap();
        let mut child = Self::command_for(&config, instance.as_ref())
            .arg("serve")
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let addr = line
            .trim()
            .strip_prefix("cooee: listening on http://")
            .unwrap_or_else(|| panic!("{line}"))
            .to_owned();
        let mut cooee = Self {
            child,
            addr,
            config,
            cookie: String::new(),
            instance,
        };
        let token = cooee.pair().expect("pairing");
        cooee.cookie = format!("cooee={token}");
        cooee
    }

    /// Runs `cooee <args>` on our config or instance; its stdout, which must succeed.
    pub fn cli(&self, args: &[&str]) -> String {
        let out = self.command().args(args).output().unwrap();
        assert!(out.status.success(), "{out:?}");
        String::from_utf8(out.stdout).unwrap()
    }

    /// `cooee pair`, then its code entered as a browser would: the new device's token, if it paired.
    pub fn pair(&self) -> Option<String> {
        let out = self.cli(&["pair"]);
        let code = out
            .lines()
            .next()?
            .strip_prefix("code: ")?
            .split(' ')
            .next()?;
        let (head, _) = self.raw(
            "POST",
            "/pair",
            &[("Content-Type", "application/x-www-form-urlencoded")],
            &format!("code={code}"),
        );
        head.lines()
            .find_map(|l| l.strip_prefix("set-cookie: cooee="))
            .map(|c| c.split(';').next().unwrap().to_owned())
    }

    /// One request with exactly these headers (and `Host: <addr>` unless given): the status line and headers
    /// (lowercased names), and the body.
    pub fn raw(
        &self,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: &str,
    ) -> (String, String) {
        let mut stream = TcpStream::connect(&self.addr).unwrap();
        write!(stream, "{method} {path} HTTP/1.0\r\n").unwrap();
        if !headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("host")) {
            write!(stream, "Host: {}\r\n", self.addr).unwrap();
        }
        for (k, v) in headers {
            write!(stream, "{k}: {v}\r\n").unwrap();
        }
        write!(stream, "Content-Length: {}\r\n\r\n{body}", body.len()).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        let (head, body) = response.split_once("\r\n\r\n").unwrap();
        let head = head
            .lines()
            .map(|l| match l.split_once(": ") {
                Some((k, v)) => format!("{}: {v}", k.to_ascii_lowercase()),
                None => l.to_owned(),
            })
            .collect::<Vec<_>>()
            .join("\n");
        (head, body.to_owned())
    }

    fn request(&self, method: &str, path: &str, body: Option<&Value>) -> (u16, String) {
        let body = body.map(Value::to_string).unwrap_or_default();
        let headers = [
            ("Content-Type", "application/json"),
            ("Cookie", self.cookie.as_str()),
        ];
        let (head, body) = self.raw(method, path, &headers, &body);
        (head[9..12].parse().unwrap(), body)
    }

    /// GET `path` as text.
    pub fn get(&self, path: &str) -> (u16, String) {
        self.request("GET", path, None)
    }

    /// GET `path` as JSON, which must succeed.
    pub fn json(&self, path: &str) -> Value {
        let (status, body) = self.get(path);
        assert_eq!(status, 200, "{path}: {body}");
        serde_json::from_str(&body).unwrap()
    }

    /// POST `body` to `path`.
    pub fn post(&self, path: &str, body: &Value) -> (u16, String) {
        self.request("POST", path, Some(body))
    }

    /// Opens thread `id`'s event stream, after `cursor` (sent as `Last-Event-ID` when `header`).
    pub fn events(&self, id: &str, cursor: &str, header: bool) -> Events {
        let mut stream = TcpStream::connect(&self.addr).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(20)))
            .unwrap();
        let (path, extra) = if header {
            (
                format!("/threads/{id}/events"),
                format!("Last-Event-ID: {cursor}\r\n"),
            )
        } else {
            (
                format!("/threads/{id}/events?since={cursor}"),
                String::new(),
            )
        };
        write!(
            stream,
            "GET {path} HTTP/1.0\r\nHost: {}\r\nCookie: {}\r\n{extra}\r\n",
            self.addr, self.cookie
        )
        .unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        while reader.read_line(&mut line).unwrap() > 2 {
            line.clear();
        }
        Events(reader)
    }
}

impl Drop for Cooee {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// An open SSE stream.
pub struct Events(BufReader<TcpStream>);

/// One SSE event: its id, name and JSON data.
#[derive(Debug, Clone)]
pub struct Sse {
    pub id: String,
    pub event: String,
    pub data: Value,
}

impl Events {
    /// The next event (keep-alive comments skipped).
    pub fn next(&mut self) -> Sse {
        let mut sse = Sse {
            id: String::new(),
            event: String::new(),
            data: Value::Null,
        };
        loop {
            let mut line = String::new();
            assert!(self.0.read_line(&mut line).unwrap() > 0, "the stream ended");
            let line = line.trim_end_matches(['\r', '\n']);
            match line.split_once(": ") {
                Some(("id", v)) => v.clone_into(&mut sse.id),
                Some(("event", v)) => v.clone_into(&mut sse.event),
                Some(("data", v)) => sse.data = serde_json::from_str(v).unwrap(),
                _ if line.is_empty() && !sse.event.is_empty() => return sse,
                _ => {}
            }
        }
    }

    /// Events until one matches `f` (included).
    pub fn until(&mut self, f: impl Fn(&Sse) -> bool) -> Vec<Sse> {
        let mut out = Vec::new();
        loop {
            let e = self.next();
            let done = f(&e);
            out.push(e);
            if done {
                return out;
            }
        }
    }
}

/// Whether `e` is an `item` of `kind`.
pub fn is_item(e: &Sse, kind: &str) -> bool {
    e.event == "item" && e.data["kind"] == kind
}

/// The shed version this build speaks.
pub fn version() -> &'static str {
    VERSION
}
