//! cooee's first-party plugin `terminal` (docs/plugins.md): the owner's shells on the Mac, from the phone. A shell is
//! a login shell (`$SHELL -l` in `~`) in a shed terminal labelled `cooee.owner=1`, so it outlives the page and cooee;
//! the other shed terminals (agents') are listed too, to watch but never type into. The screen comes from shed's
//! `screen.watch` (cell runs with colours) and goes to the tab as an event stream: no terminal emulator on the phone.
//! cooee lets a call through only from a device unlocked with its passkey (the gate in cooee-server) and names that
//! device in `Cooee-Device`; a call without one is refused. Each shell started is logged to `<data>/terminal.log`
//! with the time and device; keystrokes never are. `cooee serve` runs it when a `[[plugins]]` entry is named
//! `terminal`.

#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    io::{self, BufRead, BufReader, Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{Arc, Mutex, PoisonError},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use cooee_core::{
    Plugin, Shed, ShedError,
    shed_proto::{Method, ReadResult, RunParams, State},
};
use serde_json::{Value, json};

/// The plugin's name in `[[plugins]]`.
pub const NAME: &str = "terminal";

/// The label of the owner's shells.
pub const OWNER: &str = "cooee.owner";

/// The tab's UI module.
const TAB: &str = include_str!("tab.js");

/// The largest call body taken.
const MAX_BODY: u64 = 1 << 20;

/// A refusal: its status and sentence.
type Refused = (u16, String);

fn shed_error(e: &ShedError) -> Refused {
    if e.no_such_terminal() {
        (404, "That terminal is gone.".to_owned())
    } else {
        (502, format!("shed couldn't do that: {e}"))
    }
}

/// Unix milliseconds now.
fn millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis())
}

/// The text "insert text" types: never an Enter. Trailing line ends go, a `\` line continuation joins its lines, and
/// any other line end becomes `; `, so a block of commands stays on one line until the owner presses Enter.
#[must_use]
pub fn no_enter(text: &str) -> String {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let text = text.trim_end_matches('\n').replace("\\\n", "");
    text.replace('\n', "; ")
}

/// The plugin's state: shed, the log, where and what a shell starts, and the shell last used.
pub struct Terminal {
    shed: Shed,
    log: PathBuf,
    home: PathBuf,
    shell: String,
    last: Mutex<Option<String>>,
}

impl Terminal {
    /// The plugin over `shed`, logging to `log`; shells run `shell -l` in `home`.
    #[must_use]
    pub fn new(shed: Shed, log: PathBuf, home: PathBuf, shell: String) -> Self {
        Self {
            shed,
            log,
            home,
            shell,
            last: Mutex::default(),
        }
    }

    fn used(&self, name: &str) {
        *self.last.lock().unwrap_or_else(PoisonError::into_inner) = Some(name.to_owned());
    }

    /// The running terminals, the owner's shells first (newest first), then the agents'. Sign-in terminals are left
    /// out (cooee shows their screens redacted only); an owner's shell that exited is removed.
    ///
    /// # Errors
    /// When shed can't list them.
    pub fn list(&self) -> Result<Vec<Value>, Refused> {
        let mut out = Vec::new();
        for t in self.shed.ls(BTreeMap::new()).map_err(|e| shed_error(&e))? {
            let owner = t.labels.get(OWNER).is_some_and(|v| v == "1");
            if t.labels.contains_key("cooee-login") {
                continue;
            }
            if t.state != State::Running {
                if owner {
                    let _ = self.shed.close(&t.name);
                }
                continue;
            }
            let state = t.reported.map(|r| r.state);
            out.push(
                json!({ "name": t.name, "owner": owner, "cmd": t.cmd.join(" "), "state": state }),
            );
        }
        let name = |t: &Value| t["name"].as_str().unwrap_or_default().to_owned();
        out.sort_by(|a, b| {
            let mine = (a["owner"] != true).cmp(&(b["owner"] != true));
            // The newest shell first: its name holds its start time.
            mine.then_with(|| {
                if a["owner"] == true {
                    name(b).cmp(&name(a))
                } else {
                    name(a).cmp(&name(b))
                }
            })
        });
        Ok(out)
    }

    /// Starts a shell for `device`, logged; its name.
    ///
    /// # Errors
    /// When shed can't start it.
    pub fn start(&self, device: &str) -> Result<String, Refused> {
        let name = format!("owner-{:x}", millis());
        let run = RunParams {
            name: Some(name.clone()),
            labels: BTreeMap::from([(OWNER.to_owned(), "1".to_owned())]),
            cwd: self.home.clone(),
            keep: false,
            idle_after: None,
            cmd: vec![self.shell.clone(), "-l".to_owned()],
            acp: None,
            resume: None,
            mcp_servers: Vec::new(),
        };
        self.shed.run(&run).map_err(|e| shed_error(&e))?;
        let line = json!({ "at": millis(), "device": device, "event": "shell", "terminal": name });
        if let Err(e) = cooee_core::append_line(&self.log, &line.to_string()) {
            eprintln!("cooee-terminal: {}: {e}", self.log.display());
        }
        self.used(&name);
        Ok(name)
    }

    /// Refuses unless `name` is one of the owner's shells: an agent's terminal is read-only.
    fn owned(&self, name: &str) -> Result<(), Refused> {
        let t = self.shed.get(name).map_err(|e| shed_error(&e))?;
        if t.labels.get(OWNER).is_some_and(|v| v == "1") {
            self.used(name);
            Ok(())
        } else {
            Err((403, "Read-only: that is an agent's terminal.".to_owned()))
        }
    }

    /// `method` on one of the owner's shells, with `params` besides its name.
    fn act(&self, name: &str, method: Method, mut params: Value) -> Result<Value, Refused> {
        self.owned(name)?;
        params["terminal"] = name.into();
        let _: Value = self.shed.call(method, params).map_err(|e| shed_error(&e))?;
        Ok(json!({}))
    }

    /// Types `text` (with no Enter) into the shell used last, else the newest, else a new one; that shell's name.
    ///
    /// # Errors
    /// When shed can't list, start or type.
    pub fn insert(&self, device: &str, text: &str) -> Result<String, Refused> {
        let shells = self.list()?;
        let names: Vec<&str> = shells
            .iter()
            .filter(|t| t["owner"] == true)
            .filter_map(|t| t["name"].as_str())
            .collect();
        let last = self
            .last
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let name = match last.filter(|l| names.contains(&l.as_str())) {
            Some(l) => l,
            None => match names.first() {
                Some(n) => (*n).to_owned(),
                None => self.start(device)?,
            },
        };
        self.act(&name, Method::Input, json!({ "text": no_enter(text) }))?;
        Ok(name)
    }

    /// A call but the screen stream: its JSON answer.
    fn route(
        &self,
        method: &str,
        path: &str,
        device: &str,
        body: &Value,
    ) -> Result<String, Refused> {
        let name = || {
            let n = body["terminal"].as_str().unwrap_or_default();
            let ok = !n.is_empty()
                && n.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b));
            ok.then_some(n)
                .ok_or((400, "No such terminal name.".to_owned()))
        };
        let done = |v: Value| v.to_string();
        match (method, path) {
            ("GET", "/list") => self.list().map(|l| done(json!(l))),
            ("POST", "/seen") => Ok(done(json!({}))),
            ("POST", "/shell") => self.start(device).map(|n| done(json!({ "terminal": n }))),
            ("POST", "/insert") => {
                let text = body["text"].as_str().unwrap_or_default();
                self.insert(device, text)
                    .map(|n| done(json!({ "terminal": n })))
            }
            ("POST", "/input") => {
                let text = body["text"].as_str().unwrap_or_default();
                self.act(name()?, Method::Input, json!({ "text": text }))
                    .map(done)
            }
            ("POST", "/keys") => {
                let keys = body["keys"].clone();
                self.act(name()?, Method::TerminalKeys, json!({ "keys": keys }))
                    .map(done)
            }
            ("POST", "/resize") => {
                let size = |k: &str| body[k].as_u64().filter(|n| (1..=500).contains(n));
                let (Some(cols), Some(rows)) = (size("cols"), size("rows")) else {
                    return Err((400, "A size is 1 to 500 columns and rows.".to_owned()));
                };
                let size = json!({ "cols": cols, "rows": rows });
                self.act(name()?, Method::TerminalResize, size).map(done)
            }
            ("POST", "/close") => {
                let n = name()?;
                self.owned(n)?;
                self.shed.close(n).map_err(|e| shed_error(&e))?;
                Ok(done(json!({})))
            }
            ("POST", "/scrollback") => {
                let read = json!({ "name": name()?, "history": true });
                let r: ReadResult = self
                    .shed
                    .call(Method::Read, read)
                    .map_err(|e| shed_error(&e))?;
                Ok(done(json!({ "text": r.text })))
            }
            _ => Err((404, "No such path.".to_owned())),
        }
    }

    /// Streams terminal `name`'s screen to `s` as `screen` events (each a shed `ScreenUpdate`), a comment while
    /// quiet so a closed tab is noticed, and `closed` when its program's PTY closes.
    fn watch(&self, name: &str, s: &mut TcpStream) -> io::Result<()> {
        let updates = match self
            .shed
            .get(name)
            .and_then(|_| self.shed.watch(name, Duration::from_secs(15)))
        {
            Ok(u) => u,
            Err(e) => return respond(s, shed_error(&e).0, "text/plain", &shed_error(&e).1),
        };
        s.write_all(b"HTTP/1.0 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-store\r\n\r\n: open\n\n")?;
        for u in updates {
            match u {
                Some(u) => write!(s, "event: screen\ndata: {}\n\n", json!(u))?,
                None => s.write_all(b": quiet\n\n")?,
            }
            s.flush()?;
        }
        s.write_all(b"event: closed\ndata: {}\n\n")
    }
}

fn respond(s: &mut TcpStream, status: u16, kind: &str, body: &str) -> io::Result<()> {
    let reason = match status {
        200 => "OK",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Error",
    };
    write!(
        s,
        "HTTP/1.0 {status} {reason}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\n\r\n{body}",
        body.len()
    )?;
    s.flush()
}

/// Answers cooee's calls on `listener`: each needs the plugin's `token` (else 401), and all but `GET /tab.js` the
/// unlocked device cooee names (else 403).
pub fn serve(listener: &TcpListener, token: &str, terminal: &Arc<Terminal>) {
    for stream in listener.incoming().flatten() {
        let (token, terminal) = (token.to_owned(), Arc::clone(terminal));
        thread::spawn(move || {
            if let Err(e) = answer(stream, &token, &terminal) {
                eprintln!("cooee-terminal: {e}");
            }
        });
    }
}

fn answer(stream: TcpStream, token: &str, terminal: &Terminal) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    let mut r = BufReader::new(stream);
    let (mut request, mut line) = (String::new(), String::new());
    let (mut auth, mut device, mut length) = (false, None, 0);
    r.read_line(&mut request)?;
    while r.read_line(&mut line)? > 0 && !line.trim_end().is_empty() {
        if let Some((k, v)) = line.split_once(':') {
            let (k, v) = (k.to_ascii_lowercase(), v.trim());
            match k.as_str() {
                "authorization" => auth = v.strip_prefix("Bearer ") == Some(token),
                "cooee-device" if !v.is_empty() => device = Some(v.to_owned()),
                "content-length" => length = v.parse().unwrap_or(0),
                _ => {}
            }
        }
        line.clear();
    }
    let mut body = Vec::new();
    r.by_ref()
        .take(length.min(MAX_BODY))
        .read_to_end(&mut body)?;
    let body: Value = serde_json::from_slice(&body).unwrap_or_default();
    let mut words = request.split(' ');
    let (method, target) = (words.next().unwrap_or(""), words.next().unwrap_or(""));
    let (path, query) = target.split_once('?').unwrap_or((target, ""));
    let mut s = r.into_inner();
    let watched = query
        .strip_prefix("t=")
        .filter(|_| method == "GET" && path == "/watch");
    match (auth, device, watched) {
        (false, ..) => respond(&mut s, 401, "text/plain", "no token"),
        (true, _, _) if method == "GET" && path == "/tab.js" => {
            respond(&mut s, 200, "text/javascript", TAB)
        }
        (true, None, _) => respond(&mut s, 403, "text/plain", "Unlock with the passkey first."),
        (true, Some(_), Some(name)) => terminal.watch(name, &mut s),
        (true, Some(device), None) => match terminal.route(method, path, &device, &body) {
            Ok(json) => respond(&mut s, 200, "application/json", &json),
            Err((status, why)) => respond(&mut s, status, "text/plain", &why),
        },
    }?;
    // Drain what is left so closing never resets the connection under the answer.
    let _ = s.set_read_timeout(Some(Duration::from_millis(100)));
    let _ = s.read(&mut [0; 1024]);
    Ok(())
}

/// Runs the plugin for `plugin` (its backend at its `url`) over `shed`, logging to `<data>/terminal.log`.
///
/// # Errors
/// When the plugin's address can't be bound.
pub fn start(plugin: &Plugin, shed: Shed, data: &std::path::Path) -> io::Result<()> {
    let rest = plugin.url.trim_start_matches("http://");
    let listener = TcpListener::bind(rest.split('/').next().unwrap_or(rest))?;
    let var = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty());
    let home = var("HOME").map_or_else(|| PathBuf::from("/"), PathBuf::from);
    let shell = var("SHELL").map_or_else(
        || "/bin/zsh".to_owned(),
        |s| s.to_string_lossy().into_owned(),
    );
    let terminal = Arc::new(Terminal::new(shed, data.join("terminal.log"), home, shell));
    let token = plugin.token.clone();
    thread::spawn(move || serve(&listener, &token, &terminal));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::no_enter;

    #[test]
    fn inserted_text_never_ends_a_line() {
        assert_eq!(no_enter("cooee doctor\n"), "cooee doctor");
        assert_eq!(no_enter("a \\\n  --b\r\n"), "a   --b");
        assert_eq!(no_enter("cd ~\nls\n\n"), "cd ~; ls");
        assert!(!no_enter("x\r\ny\rz\n").contains(['\n', '\r']));
    }
}
