use std::{
    io::{Read, Write},
    net::TcpStream,
    time::Duration,
};

use cooee_core::mcp::Vars;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// What a thread's proxy does with a server: run a command on the Mac, or pipe the agent's lines to cooee.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Grant {
    /// A command server, run as the proxy's child.
    Run(Child),
    /// A URL server: cooee makes every call (`POST /mcp/agent/rpc`); the proxy holds no URL, header or token.
    Pipe,
}

/// A command server, resolved, and the tools it may offer (none: all).
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Child {
    /// Its program.
    pub command: String,
    /// Its arguments.
    pub args: Vec<String>,
    /// Its variables, resolved.
    pub env: Vars,
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

pub(super) fn ask(path: &str, body: &Value) -> Result<Value, String> {
    serde_json::from_str(&said(path, body)?).map_err(|e| e.to_string())
}

pub(super) fn said(path: &str, body: &Value) -> Result<String, String> {
    let var = cooee_core::env_var(VAR)
        .and_then(|v| v.into_string().ok())
        .ok_or_else(|| format!("{VAR} isn't set: run me as a thread's MCP server"))?;
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
        Ok(text.to_owned())
    } else {
        Err(text.trim().to_owned())
    }
}

pub(super) fn grant(server: &str, thread: &str) -> Result<Grant, String> {
    let g = ask(
        "/mcp/agent/grant",
        &json!({ "thread": thread, "server": server }),
    )?;
    serde_json::from_value(g).map_err(|e| e.to_string())
}
