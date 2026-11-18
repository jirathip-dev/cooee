use std::{
    collections::HashMap,
    sync::{Arc, Mutex, mpsc},
};

use cooee_core::{
    connectors::{Auth, HttpsUrl, Transport},
    mcp::Server,
};
use serde_json::{Value, json};

use super::lock;
use crate::{
    Hub,
    mcp::{error, only},
    net,
};

#[derive(Default)]
pub(super) struct Http {
    id: Option<String>,
    version: Option<String>,
    hello: Option<Value>,
}

type Waiting = Arc<Mutex<HashMap<String, mpsc::Sender<Value>>>>;

pub(super) enum Session {
    Http { url: String, state: Mutex<Http> },
    Sse { endpoint: String, waiting: Waiting },
}

fn origin(url: &str) -> &str {
    let after = url.find("://").map_or(0, |i| i + 3);
    let end = url
        .get(after..)
        .and_then(|r| r.find(['/', '?', '#']))
        .map_or(url.len(), |i| i + after);
    url.get(..end).unwrap_or(url)
}

fn answer_to(msg: &Value) -> Option<String> {
    let answer = msg.get("result").or_else(|| msg.get("error")).is_some();
    msg.get("id").filter(|_| answer).map(Value::to_string)
}

fn refused(msg: &Value, status: u16) -> Vec<Value> {
    let why = format!("the server answered HTTP {status}");
    msg.get("id")
        .map(|id| error(id, -32000, &why))
        .into_iter()
        .collect()
}

impl Session {
    pub(super) fn open(
        url: &HttpsUrl,
        transport: Transport,
        headers: &[String],
    ) -> Result<Self, String> {
        if transport == Transport::Http {
            return Ok(Self::Http {
                url: url.as_str().to_owned(),
                state: Mutex::default(),
            });
        }
        let mut all = vec!["Accept: text/event-stream".to_owned()];
        all.extend_from_slice(headers);
        let mut s = net::stream("GET", url.as_str(), &all, None)?;
        if !(200..300).contains(&s.status) {
            return Err(format!("{}: HTTP {}", url.as_str(), s.status));
        }
        let mut named = s.by_ref().skip_while(|l| l != "event: endpoint");
        let at = named.find_map(|l| l.strip_prefix("data:").map(|d| d.trim().to_owned()));
        let at = at.ok_or("the sse server named no endpoint")?;
        let home = origin(url.as_str());
        let endpoint = if at.starts_with('/') {
            format!("{home}{at}")
        } else {
            at
        };
        if origin(&endpoint) != home {
            return Err(format!("the sse endpoint {endpoint} isn't on {home}"));
        }
        let waiting: Waiting = Arc::default();
        let w = Arc::clone(&waiting);
        std::thread::spawn(move || {
            for l in s.by_ref() {
                let msg = l
                    .strip_prefix("data:")
                    .and_then(|d| serde_json::from_str(d).ok());
                if let Some((id, msg)) = msg.and_then(|m| Some((answer_to(&m)?, m)))
                    && let Some(tx) = lock(&w).remove(&id)
                {
                    tx.send(msg).ok();
                }
            }
            lock(&w).clear();
        });
        Ok(Self::Sse { endpoint, waiting })
    }

    pub(super) fn send(&self, headers: &[String], msg: &Value) -> Result<Vec<Value>, String> {
        match self {
            Self::Http { url, state } => Self::http((url, state), headers, msg),
            Self::Sse { endpoint, waiting } => {
                let rx = msg.get("id").map(|id| {
                    let (tx, rx) = mpsc::channel();
                    lock(waiting).insert(id.to_string(), tx);
                    rx
                });
                let mut all = vec!["Content-Type: application/json".to_owned()];
                all.extend_from_slice(headers);
                let body = msg.to_string();
                let r = net::fetch("POST", endpoint, &all, Some(body.as_bytes()))?;
                if !r.ok() {
                    return Ok(refused(msg, r.status));
                }
                rx.map(|rx| rx.recv().map_err(|_| "the sse stream closed".to_owned()))
                    .transpose()
                    .map(|a| a.into_iter().collect())
            }
        }
    }

    fn http(
        h: (&str, &Mutex<Http>),
        headers: &[String],
        msg: &Value,
    ) -> Result<Vec<Value>, String> {
        let first = Self::post(h, headers, msg)?;
        let again = lock(h.1).hello.clone().filter(|_| first.0 == 404);
        let (status, got) = match again {
            Some(hello) => {
                lock(h.1).id = None;
                Self::post(h, headers, &hello)?;
                let ready = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
                Self::post(h, headers, &ready)?;
                Self::post(h, headers, msg)?
            }
            None => first,
        };
        if !(200..300).contains(&status) {
            return Ok(refused(msg, status));
        }
        if msg.get("method").and_then(Value::as_str) == Some("initialize") {
            let settled = got
                .iter()
                .find_map(|m| m.pointer("/result/protocolVersion"));
            let mut h = lock(h.1);
            h.version = settled.and_then(Value::as_str).map(str::to_owned);
            h.hello = Some(msg.clone());
        }
        Ok(got)
    }

    fn post(
        (url, h): (&str, &Mutex<Http>),
        headers: &[String],
        msg: &Value,
    ) -> Result<(u16, Vec<Value>), String> {
        let (id, version) = {
            let h = lock(h);
            (h.id.clone(), h.version.clone())
        };
        let mut all = vec![
            "Content-Type: application/json".to_owned(),
            "Accept: application/json, text/event-stream".to_owned(),
        ];
        all.extend(id.map(|s| format!("Mcp-Session-Id: {s}")));
        all.extend(version.map(|v| format!("MCP-Protocol-Version: {v}")));
        all.extend_from_slice(headers);
        let body = msg.to_string();
        let mut s = net::stream("POST", url, &all, Some(body.as_bytes()))?;
        if let Some(sid) = s.header("mcp-session-id") {
            lock(h).id = Some(sid.to_owned());
        }
        let events = s
            .header("content-type")
            .is_some_and(|t| t.starts_with("text/event-stream"));
        let status = s.status;
        if !events {
            let text: String = s.by_ref().collect();
            let got = match serde_json::from_str(&text) {
                Ok(Value::Array(all)) => all,
                Ok(one) => vec![one],
                Err(_) => Vec::new(),
            };
            return Ok((status, got));
        }
        let mine = msg.get("id").map(Value::to_string);
        let mut got = Vec::new();
        for l in s.by_ref() {
            let Some(m) = l
                .strip_prefix("data:")
                .and_then(|d| serde_json::from_str(d).ok())
            else {
                continue;
            };
            let done = mine.is_some() && answer_to(&m) == mine;
            got.push(m);
            if done {
                break;
            }
        }
        Ok((status, got))
    }
}

impl super::Connectors {
    fn headers(&self, s: &Server, account: Option<&str>, now: u64) -> Result<Vec<String>, String> {
        let (_, _, auth) = super::target(s)?;
        let (_, refs) = s.resolved(&self.data)?;
        let mut all: Vec<String> = refs.iter().map(|(k, v)| format!("{k}: {v}")).collect();
        if !matches!(auth, Auth::None) {
            let account =
                account.ok_or_else(|| format!("{}: the pick names no account", s.name))?;
            let (k, v) = self.credential(s, account, now)?;
            all.push(format!("{k}: {v}"));
        }
        Ok(all)
    }

    pub(crate) fn rpc(
        &self,
        hub: &Hub,
        (thread, token): (&str, &str),
        server: &str,
        msg: &Value,
        now: u64,
    ) -> Result<Vec<Value>, String> {
        let (_, pick, s) = hub
            .granted(thread, token, server)
            .map_err(|e| e.to_string())?;
        let method = msg.get("method").and_then(Value::as_str);
        if method == Some("tools/call") {
            let tool = msg.pointer("/params/name").and_then(Value::as_str);
            let args = msg
                .pointer("/params/arguments")
                .cloned()
                .unwrap_or_default();
            if let Err(e) = hub.call((thread, token), (server, tool.unwrap_or_default()), &args) {
                let id = msg.get("id").cloned().unwrap_or_default();
                return Ok(vec![error(&id, -32602, &e.to_string())]);
            }
        }
        let (url, transport, _) = super::target(&s)?;
        let headers = self.headers(&s, pick.account.as_deref(), now)?;
        let key = (thread.to_owned(), server.to_owned());
        let held = lock(&self.sessions).get(&key).cloned();
        let session = if let Some(h) = held {
            h
        } else {
            let fresh = Arc::new(Session::open(&url, transport, &headers)?);
            lock(&self.sessions).insert(key.clone(), Arc::clone(&fresh));
            fresh
        };
        let got = session.send(&headers, msg).inspect_err(|_| {
            lock(&self.sessions).remove(&key);
        })?;
        Ok(match method {
            Some("tools/list") => got
                .into_iter()
                .map(|m| only(m, pick.tools.as_deref()))
                .collect(),
            _ => got,
        })
    }
}
