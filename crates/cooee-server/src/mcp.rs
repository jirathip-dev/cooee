//! MCP for agents (#56). Each server a thread's profile picks reaches its agent as `cooee mcp proxy <server>
//! --thread <id>` (stdio), which asks cooee for its grant (the command or URL, the variables and headers resolved in
//! memory, the allowed tools) with the thread's token, starts or connects to the real server, answers `tools/list`
//! with the allowed tools only and refuses a call to any other. `cooee mcp serve --thread <id>`, on every thread, is
//! cooee's own server: `propose_server`, which raises a card in the thread (nothing runs until the owner approves it
//! with Face ID), and `remember`, a note in the thread's persona's memory. The token is in `COOEE_MCP` (`<cooee's URL> <token>`) and is good while the thread is
//! open.

use std::{
    collections::HashSet,
    env,
    fmt::Write as _,
    io::{self, BufRead, BufReader, Read, Write},
    net::TcpStream,
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        Arc, Mutex, PoisonError,
        mpsc::{self, Receiver, Sender},
    },
    time::Duration,
};

use cooee_core::mcp::Vars;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// What a thread's proxy runs: the server, resolved, and the tools it may offer (none: all).
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Grant {
    /// `stdio`, `http` or `sse`.
    pub kind: String,
    /// A stdio server's program and arguments.
    pub command: String,
    /// Its arguments.
    pub args: Vec<String>,
    /// An http server's URL.
    pub url: String,
    /// The variables, resolved.
    pub env: Vars,
    /// The headers, resolved.
    pub headers: Vars,
    /// The allowed tools; none: all.
    pub tools: Option<Vec<String>>,
}

/// The variable that carries cooee's URL and the thread's token.
pub const VAR: &str = "COOEE_MCP";

/// The token of `thread` under cooee's key.
#[must_use]
pub fn token(key: &str, thread: &str) -> String {
    cooee_core::sha256(format!("{key}:{thread}").as_bytes())
}

fn ask(path: &str, body: &Value) -> Result<Value, String> {
    let var =
        env::var(VAR).map_err(|_| format!("{VAR} isn't set: run me as a thread's MCP server"))?;
    let (url, token) = var
        .split_once(' ')
        .ok_or(format!("{VAR} is `<url> <token>`"))?;
    let host = url
        .strip_prefix("http://")
        .ok_or("cooee's URL is http://host:port")?;
    let body = body.to_string();
    let mut s = TcpStream::connect(host).map_err(|e| format!("cooee at {url}: {e}"))?;
    s.set_read_timeout(Some(Duration::from_secs(630)))
        .map_err(|e| e.to_string())?;
    let head = format!(
        "POST {path} HTTP/1.0\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {token}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    s.write_all((head + &body).as_bytes())
        .map_err(|e| e.to_string())?;
    let mut out = String::new();
    s.read_to_string(&mut out).map_err(|e| e.to_string())?;
    let (head, text) = out.split_once("\r\n\r\n").unwrap_or((&out, ""));
    if head.split(' ').nth(1).is_some_and(|c| c.starts_with('2')) {
        serde_json::from_str(text).map_err(|e| e.to_string())
    } else {
        Err(text.trim().to_owned())
    }
}

fn grant(server: &str, thread: &str) -> Result<Grant, String> {
    let g = ask(
        "/mcp/agent/grant",
        &json!({ "thread": thread, "server": server }),
    )?;
    serde_json::from_value(g).map_err(|e| e.to_string())
}

fn reply(id: &Value, result: &Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error(id: &Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

fn put(out: &Mutex<io::Stdout>, msg: &Value) {
    let mut out = out.lock().unwrap_or_else(PoisonError::into_inner);
    let _ = writeln!(out, "{msg}").and_then(|()| out.flush());
}

enum Upstream {
    Stdio(Child, ChildStdin),
    Http {
        url: String,
        headers: Vars,
        session: Option<String>,
        tx: Sender<Value>,
    },
}

fn open(g: &Grant, tx: Sender<Value>) -> Result<Upstream, String> {
    match g.kind.as_str() {
        "stdio" => {
            let mut child = Command::new(&g.command)
                .args(&g.args)
                .envs(g.env.iter().map(|(k, v)| (k, v)))
                .env_remove(VAR)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .map_err(|e| format!("{}: {e}", g.command))?;
            let (stdin, stdout) = (child.stdin.take(), child.stdout.take());
            let (Some(stdin), Some(stdout)) = (stdin, stdout) else {
                return Err("no pipes".to_owned());
            };
            std::thread::spawn(move || {
                for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                    if let Ok(v) = serde_json::from_str(&line)
                        && tx.send(v).is_err()
                    {
                        return;
                    }
                }
            });
            Ok(Upstream::Stdio(child, stdin))
        }
        "http" => Ok(Upstream::Http {
            url: g.url.clone(),
            headers: g.headers.clone(),
            session: None,
            tx,
        }),
        k => Err(format!("{k} servers aren't proxied yet: use stdio or http")),
    }
}

fn quoted(v: &str) -> String {
    format!("\"{}\"", v.replace('\\', r"\\").replace('"', "\\\""))
}

impl Upstream {
    fn send(&mut self, msg: &Value) -> Result<(), String> {
        let (url, headers, session, tx) = match self {
            Self::Stdio(_, stdin) => return writeln!(stdin, "{msg}").map_err(|e| e.to_string()),
            Self::Http {
                url,
                headers,
                session,
                tx,
            } => (url, headers, session, tx),
        };
        let mut config = format!(
            "url = {}\nrequest = \"POST\"\nsilent\ninclude\n",
            quoted(url)
        );
        let fixed = [
            ("Content-Type", "application/json"),
            ("Accept", "application/json, text/event-stream"),
        ];
        let sid = session.iter().map(|s| ("Mcp-Session-Id", s.as_str()));
        for (k, v) in fixed
            .into_iter()
            .chain(sid)
            .chain(headers.iter().map(|(k, v)| (k.as_str(), v.as_str())))
        {
            let _ = writeln!(config, "header = {}", quoted(&format!("{k}: {v}")));
        }
        let _ = writeln!(config, "data-binary = {}", quoted(&msg.to_string()));
        let mut child = Command::new("curl")
            .args(["-K", "-"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| format!("curl: {e}"))?;
        child
            .stdin
            .take()
            .map(|mut i| i.write_all(config.as_bytes()));
        let out = child.wait_with_output().map_err(|e| e.to_string())?;
        let out = String::from_utf8_lossy(&out.stdout).replace("\r\n", "\n");
        let (head, body) = out.split_once("\n\n").unwrap_or((&out, ""));
        let status = head.split(' ').nth(1).unwrap_or_default().to_owned();
        for line in head.lines() {
            if let Some((k, v)) = line.split_once(':')
                && k.eq_ignore_ascii_case("mcp-session-id")
            {
                *session = Some(v.trim().to_owned());
            }
        }
        let id = msg.get("id").filter(|_| !status.starts_with('2'));
        if let Some(id) = id {
            let _ = tx.send(error(
                id,
                -32000,
                &format!("the server answered HTTP {status}"),
            ));
            return Ok(());
        }
        let events = body.lines().filter_map(|l| l.strip_prefix("data:"));
        for v in events.chain(
            [body]
                .into_iter()
                .filter(|b| b.trim_start().starts_with('{')),
        ) {
            if let Ok(v) = serde_json::from_str::<Value>(v) {
                let _ = tx.send(v);
            }
        }
        Ok(())
    }
}

fn only(mut msg: Value, tools: Option<&[String]>) -> Value {
    if let (Some(allow), Some(list)) = (
        tools,
        msg.pointer_mut("/result/tools")
            .and_then(Value::as_array_mut),
    ) {
        list.retain(|t| {
            t["name"]
                .as_str()
                .is_some_and(|n| allow.iter().any(|a| a == n))
        });
    }
    msg
}

/// `cooee mcp proxy <server> --thread <id>`: forwards the agent's stdio to the server and back, with only the
/// thread's allowed tools; each `tools/call` checks the grant again, so a closed thread's token stops working.
///
/// # Errors
/// When the grant is refused or the server can't start.
pub fn proxy(server: &str, thread: &str) -> Result<(), String> {
    let g = grant(server, thread)?;
    let (tx, rx): (Sender<Value>, Receiver<Value>) = mpsc::channel();
    let mut up = open(&g, tx)?;
    let out = Arc::new(Mutex::new(io::stdout()));
    let lists: Arc<Mutex<HashSet<String>>> = Arc::default();
    let (o, l) = (Arc::clone(&out), Arc::clone(&lists));
    let tools = g.tools.clone();
    std::thread::spawn(move || {
        for msg in rx {
            let id = msg.get("id").map(Value::to_string).unwrap_or_default();
            let listed = msg.get("method").is_none()
                && l.lock().unwrap_or_else(PoisonError::into_inner).remove(&id);
            put(
                &o,
                &if listed {
                    only(msg, tools.as_deref())
                } else {
                    msg
                },
            );
        }
    });
    for line in io::stdin().lock().lines().map_while(Result::ok) {
        let Ok(msg) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let id = msg.get("id").cloned().unwrap_or(Value::Null);
        match msg["method"].as_str() {
            Some("tools/list") => {
                lists
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .insert(id.to_string());
            }
            Some("tools/call") => {
                let name = msg
                    .pointer("/params/name")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                let why = match grant(server, thread) {
                    Err(e) => Some(e),
                    Ok(g) if !g.tools.as_ref().is_none_or(|t| t.iter().any(|t| t == name)) => {
                        Some(format!(
                            "tool {name} isn't allowed in this thread (its profile's tools for {server})"
                        ))
                    }
                    Ok(_) => None,
                };
                if let Some(why) = why {
                    put(&out, &error(&id, -32602, &why));
                    continue;
                }
            }
            _ => {}
        }
        up.send(&msg)?;
    }
    if let Upstream::Stdio(mut child, _) = up {
        let _ = child.kill();
        let _ = child.wait();
    }
    Ok(())
}

/// Lists `g`'s tools (name, description) for the profile sheet: initialize, then `tools/list`, 10 s at most.
///
/// # Errors
/// When the server can't start or doesn't answer.
pub fn tools(g: &Grant) -> Result<Vec<(String, String)>, String> {
    let (tx, rx) = mpsc::channel();
    let mut up = open(g, tx)?;
    let hello = json!({ "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "cooee", "version": env!("CARGO_PKG_VERSION") } });
    let call = |up: &mut Upstream, id: u64, method: &str, params: Value| -> Result<Value, String> {
        up.send(&json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))?;
        loop {
            let msg = rx
                .recv_timeout(Duration::from_secs(10))
                .map_err(|_| format!("no answer to {method}"))?;
            if msg["id"] == id {
                return msg
                    .get("result")
                    .cloned()
                    .ok_or_else(|| msg["error"]["message"].to_string());
            }
        }
    };
    let listed = call(&mut up, 1, "initialize", hello).and_then(|_| {
        up.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))?;
        call(&mut up, 2, "tools/list", json!({}))
    });
    if let Upstream::Stdio(mut child, _) = up {
        let _ = child.kill();
        let _ = child.wait();
    }
    let list = listed?["tools"].as_array().cloned().unwrap_or_default();
    let text = |t: &Value, k: &str| t[k].as_str().unwrap_or_default().to_owned();
    Ok(list
        .iter()
        .map(|t| (text(t, "name"), text(t, "description")))
        .collect())
}

/// `cooee mcp serve --thread <id>`: cooee's own MCP server: `propose_server`, and `remember` (#126),
/// `ask_persona` (#156) and `history` (#217) for a persona's.
///
/// # Errors
/// Never past a broken stdin; each refused proposal is the tool's error.
pub fn serve(thread: &str) -> Result<(), String> {
    let out = Mutex::new(io::stdout());
    let schema = json!({
        "type": "object",
        "required": ["name", "type", "why"],
        "properties": {
            "name": { "type": "string", "description": "lowercase letters, digits, - and _" },
            "type": { "type": "string", "enum": ["stdio", "http", "sse"] },
            "command": { "type": "string", "description": "a stdio server's program" },
            "args": { "type": "array", "items": { "type": "string" } },
            "url": { "type": "string", "description": "an http or sse server's URL" },
            "why": { "type": "string", "description": "what the owner gets from it" }
        }
    });
    let tool = json!({
        "name": "propose_server",
        "description": "Propose an MCP server for cooee's registry. The owner sees the exact command or URL and approves it with Face ID; it is added switched off, and nothing runs before that.",
        "inputSchema": schema
    });
    let remember = json!({
        "name": "remember",
        "description": "Keep a note in your persona's memory: it reaches you with your instructions in every new session. The owner can read and clear it.",
        "inputSchema": { "type": "object", "required": ["text"], "properties": { "text": { "type": "string" } } }
    });
    let ask_persona = json!({
        "name": "ask_persona",
        "description": "Ask another persona (one your persona's can_message names) and wait for its reply. The owner sees the exchange as a linked thread. A refusal starts with NOT_ALLOWED, DEPTH, TIMEOUT or OFFLINE.",
        "inputSchema": { "type": "object", "required": ["persona", "message"], "properties": {
            "persona": { "type": "string", "description": "its id" },
            "message": { "type": "string" },
            "timeout": { "type": "integer", "description": "seconds to wait, at most 600 (default 300)" }
        } }
    });
    let history = json!({
        "name": "history",
        "description": "Search your persona's past threads (query: words; at most 5 hits, each a thread ref, date and snippet), or read one (thread: its ref; paged with before).",
        "inputSchema": { "type": "object", "properties": {
            "query": { "type": "string" },
            "thread": { "type": "string" },
            "before": { "type": "integer", "description": "the older: before= a read ended with" },
            "limit": { "type": "integer", "description": "fewer hits, at most 5" }
        } }
    });
    for line in io::stdin().lock().lines().map_while(Result::ok) {
        let Ok(msg) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let Some(id) = msg.get("id") else {
            continue;
        };
        let answer = match msg["method"].as_str().unwrap_or_default() {
            "initialize" => reply(
                id,
                &json!({
                    "protocolVersion": msg.pointer("/params/protocolVersion").cloned().unwrap_or(json!("2025-06-18")),
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "cooee", "version": env!("CARGO_PKG_VERSION") }
                }),
            ),
            "ping" => reply(id, &json!({})),
            "tools/list" => reply(
                id,
                &json!({ "tools": [tool, remember, ask_persona, history] }),
            ),
            "tools/call" => {
                let args = msg
                    .pointer("/params/arguments")
                    .cloned()
                    .unwrap_or_default();
                let call = match msg.pointer("/params/name").and_then(Value::as_str) {
                    Some("propose_server") => Some((
                        "/mcp/agent/propose",
                        "Proposed. The owner sees a card with the exact command or URL; once approved with Face ID it is added switched off. Nothing has run.",
                    )),
                    Some("remember") => Some((
                        "/mcp/agent/remember",
                        "Noted: it reaches you in your next sessions.",
                    )),
                    Some("ask_persona") => Some(("/mcp/agent/ask", "")),
                    Some("history") => Some(("/mcp/agent/history", "")),
                    _ => None,
                };
                match call.map(|(path, said)| {
                    let got = ask(path, &json!({ "thread": thread, "server": args }));
                    got.map(|v| v.as_str().unwrap_or(said).to_owned())
                }) {
                    Some(said) => {
                        let (text, failed) = said.map_or_else(|e| (e, true), |s| (s, false));
                        reply(
                            id,
                            &json!({ "content": [{ "type": "text", "text": text }], "isError": failed }),
                        )
                    }
                    None => error(id, -32602, "cooee has no such tool"),
                }
            }
            m => error(id, -32601, &format!("no method {m}")),
        };
        put(&out, &answer);
    }
    Ok(())
}
