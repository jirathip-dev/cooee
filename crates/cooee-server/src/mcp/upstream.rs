use std::{
    fmt::Write as _,
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Stdio},
    sync::mpsc::Sender,
};

use cooee_core::mcp::Vars;
use serde_json::Value;

use super::{Grant, VAR, rpc::error};

pub(super) enum Upstream {
    Stdio(Child, ChildStdin),
    Http {
        url: String,
        headers: Vars,
        session: Option<String>,
        tx: Sender<Value>,
    },
}

pub(super) fn open(g: &Grant, tx: Sender<Value>) -> Result<Upstream, String> {
    match g.kind.as_str() {
        "stdio" => {
            let mut child = cooee_core::runner::command(&g.command)
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
    pub(super) fn send(&mut self, msg: &Value) -> Result<(), String> {
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
            writeln!(config, "header = {}", quoted(&format!("{k}: {v}"))).ok();
        }
        writeln!(config, "data-binary = {}", quoted(&msg.to_string())).ok();
        let mut child = cooee_core::runner::command("curl")
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
            tx.send(error(
                id,
                -32000,
                &format!("the server answered HTTP {status}"),
            ))
            .ok();
            return Ok(());
        }
        let events = body.lines().filter_map(|l| l.strip_prefix("data:"));
        for v in events.chain(
            [body]
                .into_iter()
                .filter(|b| b.trim_start().starts_with('{')),
        ) {
            if let Ok(v) = serde_json::from_str::<Value>(v) {
                tx.send(v).ok();
            }
        }
        Ok(())
    }
}
