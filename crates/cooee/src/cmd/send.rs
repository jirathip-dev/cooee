use std::{
    io::{BufRead, BufReader, ErrorKind, Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpStream},
    process::ExitCode,
    time::Duration,
};

use cooee_core::Config;
use serde_json::{Value, json};

use crate::{
    cli::{Failed, Flag, Invocation},
    devices::{devices_of, now},
};

pub(super) const USAGE: &str = "send <thread> <text> [--wait [--timeout <secs>]] [--json]";
pub(super) const FLAGS: &[Flag] = &[
    Flag::Switch("--wait"),
    Flag::Switch("--json"),
    Flag::Value("--timeout"),
];

pub(super) fn run(inv: &Invocation) -> Result<(), Failed> {
    let rest = inv.rest();
    let secs = inv.value("--timeout").map_or(Ok(600), str::parse::<u64>);
    let ([thread, text], Ok(secs)) = (rest.as_slice(), secs) else {
        return Err(Failed::Usage);
    };
    if thread.is_empty()
        || !thread
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(Failed::Usage);
    }
    let server = Server::of(&inv.config()?)?;
    let cursor = server.json("GET", &format!("/threads/{thread}/messages?before=0"), "")?;
    let cursor = cursor["cursor"].as_str().unwrap_or_default().to_owned();
    let random_id = format!("cli-{}-{}", now(), std::process::id());
    let body = json!({ "text": text, "random_id": random_id }).to_string();
    let sent = server.json("POST", &format!("/threads/{thread}/send"), &body)?;
    let (state, item) = if inv.on("--wait") {
        let sent = sent["id"].as_u64().unwrap_or_default();
        server.wait(thread, &cursor, sent, Duration::from_secs(secs))?
    } else {
        ("sent", Value::Null)
    };
    report(thread, state, &item, inv.on("--json"))?;
    match state {
        "approval" => Err(Failed::Exit(ExitCode::from(3))),
        "ended" => Err(Failed::Exit(ExitCode::FAILURE)),
        "timeout" => Err(Failed::Exit(ExitCode::from(124))),
        _ => Ok(()),
    }
}

fn report(thread: &str, state: &str, item: &Value, json: bool) -> Result<(), String> {
    let options: Vec<&Value> = item["options"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|o| &o["id"])
        .collect();
    let approval = (state == "approval")
        .then(|| json!({ "item": item["id"], "tool": item["tool"], "options": options }));
    let reply = (state == "done").then(|| &item["text"]);
    if json {
        let out = json!({ "thread": thread, "state": state, "reply": reply,
            "stop_reason": item.get("stop_reason"), "approval": approval });
        cooee_core::say!(
            "{}",
            serde_json::to_string(&out).map_err(|e| e.to_string())?
        );
        return Ok(());
    }
    let word = |v: &Value| v.as_str().unwrap_or_default().to_owned();
    match state {
        "done" => cooee_core::say!("{}", word(&item["text"])),
        "approval" => {
            let options: Vec<String> = options.into_iter().map(word).collect();
            cooee_core::say!(
                "waiting on an approval: {} ({})",
                word(&item["tool"]),
                options.join(", ")
            );
        }
        "ended" => cooee_core::note!("cooee: the session ended: {}", word(&item["text"])),
        "timeout" => cooee_core::note!("cooee: the turn did not end in time"),
        _ => {}
    }
    Ok(())
}

struct Server {
    addr: SocketAddr,
    token: String,
}

impl Server {
    fn of(config: &Config) -> Result<Self, String> {
        let ip = if config.bind.is_unspecified() {
            Ipv4Addr::LOCALHOST.into()
        } else {
            config.bind
        };
        Ok(Self {
            addr: SocketAddr::new(ip, config.port),
            token: token(config)?,
        })
    }

    fn open(&self, method: &str, path: &str, body: &str) -> Result<BufReader<TcpStream>, String> {
        let down =
            |e: std::io::Error| format!("cooee at {}: {e} (is `cooee serve` running?)", self.addr);
        let mut s = TcpStream::connect_timeout(&self.addr, Duration::from_secs(5)).map_err(down)?;
        s.set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(down)?;
        write!(
            s,
            "{method} {path} HTTP/1.0\r\nHost: {}\r\nCookie: cooee={}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
            self.addr,
            self.token,
            body.len()
        )
        .map_err(down)?;
        let mut r = BufReader::new(s);
        let (mut status, mut line) = (String::new(), String::from("-"));
        r.read_line(&mut status).map_err(down)?;
        while line.trim_end().len() > 0 {
            line.clear();
            if r.read_line(&mut line).map_err(down)? == 0 {
                break;
            }
        }
        if status.split(' ').nth(1).is_some_and(|c| c.starts_with('2')) {
            return Ok(r);
        }
        let mut why = String::new();
        r.read_to_string(&mut why).ok();
        Err(format!("{path}: {}", why.trim()))
    }

    fn json(&self, method: &str, path: &str, body: &str) -> Result<Value, String> {
        let mut text = String::new();
        self.open(method, path, body)?
            .read_to_string(&mut text)
            .map_err(|e| e.to_string())?;
        serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))
    }

    fn wait(
        &self,
        thread: &str,
        cursor: &str,
        sent: u64,
        limit: Duration,
    ) -> Result<(&'static str, Value), String> {
        let deadline = cooee_core::clock::instant() + limit;
        let mut r = self.open(
            "GET",
            &format!("/threads/{thread}/events?since={cursor}"),
            "",
        )?;
        let (mut event, mut line) = (String::new(), String::new());
        loop {
            let left = deadline.saturating_duration_since(cooee_core::clock::instant());
            if left.is_zero() {
                return Ok(("timeout", Value::Null));
            }
            r.get_ref()
                .set_read_timeout(Some(left))
                .map_err(|e| e.to_string())?;
            line.clear();
            match r.read_line(&mut line) {
                Ok(0) => return Err("the thread's event stream ended".to_owned()),
                Ok(_) => {}
                Err(e) if matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut) => {
                    return Ok(("timeout", Value::Null));
                }
                Err(e) => return Err(e.to_string()),
            }
            let line = line.trim_end();
            if let Some(name) = line.strip_prefix("event: ") {
                name.clone_into(&mut event);
            }
            if event == "reset" {
                return Err(
                    "the thread's event stream reset: read the thread in the app".to_owned(),
                );
            }
            let Some(item) = line.strip_prefix("data: ").filter(|_| event == "item") else {
                continue;
            };
            let item: Value = serde_json::from_str(item).map_err(|e| e.to_string())?;
            if item["id"].as_u64().is_none_or(|id| id <= sent) {
                continue;
            }
            let open = item.get("chosen").is_none() && item.get("tapped").is_none();
            match item["kind"].as_str() {
                Some("agent") => return Ok(("done", item)),
                Some("approval") if open => return Ok(("approval", item)),
                Some("ended") => return Ok(("ended", item)),
                _ => {}
            }
        }
    }
}

fn token(config: &Config) -> Result<String, String> {
    let devices = devices_of(config)?;
    let path = config
        .data_dir()
        .ok_or("no data folder: set `data` or HOME")?
        .join("cli-token");
    let e = |e: std::io::Error| format!("{}: {e}", path.display());
    let kept = std::fs::read_to_string(&path).unwrap_or_default();
    if !kept.is_empty() && devices.find(kept.trim(), now()).map_err(e)?.is_some() {
        return Ok(kept.trim().to_owned());
    }
    let code = devices.pair(now()).map_err(e)?;
    let token = devices
        .claim(&code, "", now())
        .map_err(e)?
        .ok_or("the CLI could not pair")?;
    if let Some(d) = devices.find(&token, now()).map_err(e)? {
        devices.rename(&d.id, "cooee CLI").map_err(e)?;
    }
    cooee_core::write_private(&path, token.as_bytes()).map_err(e)?;
    Ok(token)
}
