//! Plugins (docs/plugins.md): a tab whose UI module, served by the plugin's own backend, runs in a sandboxed frame.
//! Its calls reach that backend through cooee with the plugin's token, never the device's cookie: one HTTP/1.0
//! request per call (so the answer is never chunked), an event stream read line by line. A call ends at its deadline
//! (connect, send and the whole answer) and a backend has at most [`IN_FLIGHT`] calls at once: one more fails at once.

use std::{
    collections::HashMap,
    io::{self, BufRead, BufReader, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    sync::{LazyLock, Mutex, PoisonError},
    time::{Duration, Instant},
};

use base64::{Engine, engine::general_purpose::STANDARD};
use cooee_core::{Plugin, Profile};
use sha2::{Digest, Sha256};

/// The largest answer cooee passes on.
const MAX_BODY: u64 = 16 << 20;

/// The most calls one backend has in flight; one more fails at once instead of taking a blocking thread.
pub const IN_FLIGHT: usize = 8;

/// The calls in flight, by backend URL.
static BUSY: LazyLock<Mutex<HashMap<String, usize>>> = LazyLock::new(Mutex::default);

/// One of a backend's [`IN_FLIGHT`] places, given back when dropped.
struct Permit(String);

impl Permit {
    fn take(url: &str) -> io::Result<Self> {
        let mut busy = BUSY.lock().unwrap_or_else(PoisonError::into_inner);
        let n = busy.entry(url.to_owned()).or_default();
        if *n >= IN_FLIGHT {
            return Err(io::Error::other(format!(
                "busy: {IN_FLIGHT} calls already in flight"
            )));
        }
        *n += 1;
        Ok(Self(url.to_owned()))
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        let mut busy = BUSY.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(n) = busy.get_mut(&self.0) {
            *n -= 1;
        }
    }
}

/// A backend's connection, read until its deadline (none: an event stream).
pub struct Conn {
    s: TcpStream,
    until: Option<Instant>,
}

impl Conn {
    /// The time left before the deadline.
    fn left(&self) -> io::Result<Option<Duration>> {
        let Some(until) = self.until else {
            return Ok(None);
        };
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "past the deadline"));
        }
        Ok(Some(left))
    }
}

impl Read for Conn {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.s.set_read_timeout(self.left()?)?;
        self.s.read(buf)
    }
}

impl Write for Conn {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.s.set_write_timeout(self.left()?)?;
        self.s.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.s.flush()
    }
}

/// Checks the plugins: unique names of lowercase letters, digits and `-`, an `http://` backend, a module path (or
/// none: screens only) and a token of at least 16 characters.
///
/// # Errors
/// The first plugin that breaks a rule, and why.
pub fn check(plugins: &[Plugin]) -> io::Result<()> {
    for (i, p) in plugins.iter().enumerate() {
        let bad = |why: &str| Err(io::Error::other(format!("plugin {:?}: {why}", p.name)));
        let name_ok = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-';
        if p.name.is_empty() || !p.name.bytes().all(name_ok) {
            return bad("`name` must be lowercase letters, digits and -");
        }
        if plugins[..i].iter().any(|q| q.name == p.name) {
            return bad("two plugins have this name");
        }
        if !p.url.starts_with("http://") {
            return bad("`url` must be http://host:port");
        }
        if !p.module.is_empty() && !p.module.starts_with('/') {
            return bad("`module` must be a path on the backend, like /tab.js");
        }
        if p.token.len() < 16 || !p.token.bytes().all(|b| b.is_ascii_graphic()) {
            return bad("`token` must be at least 16 characters, no spaces");
        }
    }
    Ok(())
}

/// A backend's answer: its status, its headers (lowercased names) and its body, still open.
pub struct Reply {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: BufReader<Conn>,
    permit: Permit,
}

impl Reply {
    /// The header `name` (lowercase).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    /// The whole body (at most 16 MiB).
    pub fn bytes(self) -> io::Result<Vec<u8>> {
        let mut out = Vec::new();
        self.body.take(MAX_BODY).read_to_end(&mut out)?;
        Ok(out)
    }
}

/// One request to `p`'s backend, with its token: `target` is the path and query, `headers` go as given. The call,
/// its answer's body read included, ends `deadline` from now.
///
/// # Errors
/// When the backend is busy, can't be reached in time or its answer isn't HTTP.
pub fn call(
    p: &Plugin,
    method: &str,
    target: &str,
    headers: &[(&str, &str)],
    body: &[u8],
    deadline: Duration,
) -> io::Result<Reply> {
    let permit = Permit::take(&p.url)?;
    let until = Instant::now() + deadline;
    let rest = p.url.strip_prefix("http://").unwrap_or(&p.url);
    let (authority, base) = rest.split_once('/').unwrap_or((rest, ""));
    let base = base.trim_end_matches('/');
    let with_port = if authority
        .rsplit_once(':')
        .is_some_and(|(_, p)| !p.ends_with(']'))
    {
        authority.to_owned()
    } else {
        format!("{authority}:80")
    };
    let addr = with_port
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::other(format!("{authority}: no address")))?;
    let mut s = Conn {
        s: TcpStream::connect_timeout(&addr, deadline.min(Duration::from_secs(5)))?,
        until: Some(until),
    };
    let slash = if base.is_empty() { "" } else { "/" };
    let mut head = format!(
        "{method} {slash}{base}{target} HTTP/1.0\r\nHost: {authority}\r\nAuthorization: Bearer {}\r\nContent-Length: {}\r\n",
        p.token,
        body.len()
    );
    for (k, v) in headers {
        head.extend([k, ": ", v, "\r\n"]);
    }
    head.push_str("\r\n");
    s.write_all(head.as_bytes())?;
    s.write_all(body)?;
    let mut r = BufReader::new(s);
    let mut line = String::new();
    r.read_line(&mut line)?;
    let status = line
        .split(' ')
        .nth(1)
        .and_then(|c| c.parse().ok())
        .ok_or_else(|| io::Error::other("the backend's answer isn't HTTP"))?;
    let mut headers = Vec::new();
    loop {
        line.clear();
        if r.read_line(&mut line)? == 0 || line.trim_end().is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            headers.push((k.trim().to_ascii_lowercase(), v.trim().to_owned()));
        }
    }
    let reply = Reply {
        status,
        headers,
        body: r,
        permit,
    };
    if reply.header("transfer-encoding").is_some() {
        return Err(io::Error::other(
            "the backend answered HTTP/1.0 with a transfer-encoding",
        ));
    }
    Ok(reply)
}

/// One event of a backend's stream.
pub struct Sse {
    pub id: Option<String>,
    pub event: String,
    pub data: String,
}

/// Reads `reply`'s event stream, giving each event to `f` until it returns false or the stream ends; a stream has
/// no deadline and holds no place among its backend's calls.
pub fn events(reply: Reply, mut f: impl FnMut(Sse) -> bool) {
    let Reply {
        mut body, permit, ..
    } = reply;
    drop(permit);
    body.get_mut().until = None;
    let mut e = Sse {
        id: None,
        event: String::new(),
        data: String::new(),
    };
    let mut lines = body.lines();
    while let Some(Ok(line)) = lines.next() {
        let (k, v) = line.split_once(':').unwrap_or((&line, ""));
        let v = v.strip_prefix(' ').unwrap_or(v).to_owned();
        match k {
            "" if line.is_empty() && !e.data.is_empty() => {
                let id = e.id.clone();
                let done = std::mem::replace(
                    &mut e,
                    Sse {
                        id,
                        event: String::new(),
                        data: String::new(),
                    },
                );
                if !f(done) {
                    return;
                }
            }
            "id" => e.id = Some(v),
            "event" => e.event = v,
            "data" if e.data.is_empty() => e.data = v,
            "data" => {
                e.data.push('\n');
                e.data.push_str(&v);
            }
            _ => {}
        }
    }
}

/// Sends `by`'s answer to choice card `item` in `thread`: `POST /cooee/choice` `{thread, item, option, by}`, with a
/// secret card's `value` (never kept by cooee). A 2xx settles it, with the backend's `{"line": …}` when it sends one;
/// a 409 means it can't be answered any more: `Err` with its line, else `stale`.
///
/// # Errors
/// When the backend can't be reached or answers anything else.
pub fn answer(
    p: &Plugin,
    thread: &str,
    item: u64,
    option: &str,
    by: &str,
    value: Option<&str>,
) -> io::Result<Result<Option<String>, String>> {
    let mut body =
        serde_json::json!({ "thread": thread, "item": item, "option": option, "by": by });
    if let Some(value) = value {
        body["value"] = value.into();
    }
    let json = [("Content-Type", "application/json")];
    let reply = call(
        p,
        "POST",
        "/cooee/choice",
        &json,
        body.to_string().as_bytes(),
        Duration::from_secs(10),
    )?;
    let status = reply.status;
    let line = serde_json::from_slice::<serde_json::Value>(&reply.bytes().unwrap_or_default())
        .ok()
        .and_then(|v| Some(v.get("line")?.as_str()?.chars().take(2000).collect()));
    match status {
        200..=299 => Ok(Ok(line)),
        409 => Ok(Err(line.unwrap_or_else(|| "stale".to_owned()))),
        code => Err(io::Error::other(format!("the backend answered {code}"))),
    }
}

/// The agent profiles `p`'s backend serves: `GET /cooee/profiles` answers `{"profiles": [...]}`. `None` when it
/// answers 404 (it serves none: its manifest's are offered); any other answer offers none.
pub fn profiles(p: &Plugin) -> Option<Vec<Profile>> {
    let got = call(
        p,
        "GET",
        "/cooee/profiles",
        &[],
        &[],
        Duration::from_secs(5),
    );
    let list = got.and_then(|r| match r.status {
        404 => Ok(None),
        200 => {
            #[derive(serde::Deserialize)]
            struct Served {
                profiles: Vec<Profile>,
            }
            Ok(Some(
                serde_json::from_slice::<Served>(&r.bytes()?)?.profiles,
            ))
        }
        code => Err(io::Error::other(format!("the backend answered {code}"))),
    });
    list.unwrap_or_else(|e| {
        eprintln!("cooee: plugin {}: its profiles: {e}", p.name);
        Some(Vec::new())
    })
}

/// Tells `p`'s backend thread `thread` started as its profile `profile`: `POST /cooee/thread-started`
/// `{thread, profile}`.
///
/// # Errors
/// When the backend can't be reached or answers anything but a 2xx.
pub fn started(p: &Plugin, thread: &str, profile: &str) -> io::Result<()> {
    let body = serde_json::json!({ "thread": thread, "profile": profile }).to_string();
    let json = [("Content-Type", "application/json")];
    let reply = call(
        p,
        "POST",
        "/cooee/thread-started",
        &json,
        body.as_bytes(),
        Duration::from_secs(10),
    )?;
    match reply.status {
        200..=299 => Ok(()),
        code => Err(io::Error::other(format!("the backend answered {code}"))),
    }
}

/// The frame page for `p`: its UI module (fetched from the backend) after `api`, in one inline module script, and
/// the page's Content-Security-Policy: a sandbox (an opaque origin, even opened on its own), only that script, no
/// connections; the module reaches cooee through the parent page only.
///
/// # Errors
/// When the module can't be fetched or would end its script element.
pub fn frame(p: &Plugin, api: &str) -> io::Result<(String, String)> {
    let reply = call(p, "GET", &p.module, &[], &[], Duration::from_secs(10))?;
    if reply.status != 200 {
        return Err(io::Error::other(format!(
            "the module answered {}",
            reply.status
        )));
    }
    let module = String::from_utf8(reply.bytes()?).map_err(io::Error::other)?;
    if module.to_ascii_lowercase().contains("</script") {
        return Err(io::Error::other("the module contains </script"));
    }
    let script = format!("{api}\n{module}");
    let hash = STANDARD.encode(Sha256::digest(script.as_bytes()));
    let title = p
        .label
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let page = format!(
        "<!DOCTYPE html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>{title}</title>\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><style>:root{{color-scheme:light dark}}\
         body{{margin:0;padding:12px;background:var(--bg);color:var(--fg);font:var(--fs,15px)/1.4 var(--font,system-ui)}}</style>\
         </head><body><script type=\"module\">{script}</script></body></html>"
    );
    let csp = format!(
        "sandbox allow-scripts; default-src 'none'; script-src 'sha256-{hash}'; style-src 'unsafe-inline'; \
         img-src data: blob:; base-uri 'none'; form-action 'none'; frame-ancestors 'self'"
    );
    Ok((page, csp))
}

#[cfg(test)]
mod tests {
    use std::{net::TcpListener, thread};

    use super::*;

    /// A backend that answers one byte a second: its head at once when `head`, else that too a byte a second.
    fn trickle(head: bool) -> Plugin {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        thread::spawn(move || {
            for mut s in listener.incoming().flatten() {
                thread::spawn(move || {
                    let mut first = [0; 512];
                    let _ = s.read(&mut first);
                    if head {
                        let _ = s.write_all(b"HTTP/1.0 200 OK\r\n\r\n");
                    }
                    for b in b"HTTP/1.0 200 OK\r\n\r\n".iter().cycle() {
                        if s.write_all(&[*b]).is_err() {
                            return;
                        }
                        thread::sleep(Duration::from_secs(1));
                    }
                });
            }
        });
        Plugin {
            name: "slow".to_owned(),
            label: String::new(),
            url,
            module: "/tab.js".to_owned(),
            token: "token-0123456789abcdef".to_owned(),
        }
    }

    #[test]
    fn a_trickling_backend_fails_at_the_deadline() {
        let deadline = Duration::from_millis(2500);
        for head in [false, true] {
            let p = trickle(head);
            let at = Instant::now();
            let got = call(&p, "GET", "/x", &[], &[], deadline).and_then(Reply::bytes);
            let took = at.elapsed();
            assert!(got.is_err(), "head {head}: it answered");
            assert!(
                took >= deadline && took < deadline + Duration::from_millis(500),
                "head {head}: failed after {took:?}"
            );
        }
    }
}
