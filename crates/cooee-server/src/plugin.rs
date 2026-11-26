use std::{
    collections::HashMap,
    io::{self, BufRead, BufReader, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    sync::{LazyLock, Mutex, PoisonError},
    time::{Duration, Instant},
};

use cooee_core::{Plugin, Profile};
use futures_util::{Stream, stream};
use tokio::io::{AsyncBufReadExt, AsyncReadExt};

mod feed;

pub use feed::{follow, share};

const MAX_BODY: u64 = 16 << 20;

/// The most calls one backend has in flight; one more fails at once instead of taking a blocking thread.
pub const IN_FLIGHT: usize = 8;

static BUSY: LazyLock<Mutex<HashMap<String, usize>>> = LazyLock::new(Mutex::default);

static KEPT: LazyLock<Mutex<HashMap<String, Vec<TcpStream>>>> = LazyLock::new(Mutex::default);

fn kept(url: &str) -> Option<TcpStream> {
    let mut kept = KEPT.lock().unwrap_or_else(PoisonError::into_inner);
    kept.get_mut(url)?.pop()
}

fn keep(url: &str, s: TcpStream) {
    let mut kept = KEPT.lock().unwrap_or_else(PoisonError::into_inner);
    let idle = kept.entry(url.to_owned()).or_default();
    if idle.len() < IN_FLIGHT {
        idle.push(s);
    }
}

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

/// A backend's connection, read and written until its deadline.
pub struct Conn {
    s: TcpStream,
    until: Instant,
}

impl Conn {
    fn left(&self) -> io::Result<Option<Duration>> {
        let left = self
            .until
            .saturating_duration_since(cooee_core::clock::instant());
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
        if plugins.iter().take(i).any(|q| q.name == p.name) {
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
    length: Option<u64>,
    reusable: bool,
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

    /// Whether the answer is an event stream.
    pub fn streams(&self) -> bool {
        self.header("content-type")
            .is_some_and(|t| t.starts_with("text/event-stream"))
    }

    /// The whole body (at most 16 MiB); a kept-alive connection read to its end goes back for the next call.
    pub fn bytes(self) -> io::Result<Vec<u8>> {
        let Self {
            mut body,
            length,
            reusable,
            permit,
            ..
        } = self;
        let mut out = Vec::new();
        let want = length.unwrap_or(MAX_BODY).min(MAX_BODY);
        (&mut body).take(want).read_to_end(&mut out)?;
        if reusable && length == u64::try_from(out.len()).ok() && body.buffer().is_empty() {
            keep(&permit.0, body.into_inner().s);
        }
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
    let until = cooee_core::clock::instant() + deadline;
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
    let connect = || {
        let addr = with_port
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| io::Error::other(format!("{authority}: no address")))?;
        TcpStream::connect_timeout(&addr, deadline.min(Duration::from_secs(5)))
    };
    let slash = if base.is_empty() { "" } else { "/" };
    let mut head = format!(
        "{method} {slash}{base}{target} HTTP/1.0\r\nHost: {authority}\r\nAuthorization: Bearer {}\r\nConnection: keep-alive\r\nContent-Length: {}\r\n",
        p.token,
        body.len()
    );
    for (k, v) in headers {
        head.extend([k, ": ", v, "\r\n"]);
    }
    head.push_str("\r\n");
    let send = |s: TcpStream| -> io::Result<(BufReader<Conn>, String)> {
        let mut s = Conn { s, until };
        s.write_all(head.as_bytes())?;
        s.write_all(body)?;
        let mut r = BufReader::new(s);
        let mut line = String::new();
        r.read_line(&mut line)?;
        Ok((r, line))
    };
    let (r, line) = match kept(&p.url).map(send) {
        Some(Ok((r, line))) if !line.is_empty() => (r, line),
        _ => send(connect()?)?,
    };
    read_reply(r, line, permit)
}

fn read_reply(mut r: BufReader<Conn>, mut line: String, permit: Permit) -> io::Result<Reply> {
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
    let mut reply = Reply {
        status,
        headers,
        body: r,
        length: Some(0),
        reusable: false,
        permit,
    };
    if !matches!(status, 204 | 304) {
        reply.length = reply.header("content-length").and_then(|n| n.parse().ok());
    }
    reply.reusable = reply
        .header("connection")
        .is_some_and(|c| c.eq_ignore_ascii_case("keep-alive"));
    if reply.header("transfer-encoding").is_some() {
        return Err(io::Error::other(
            "the backend answered HTTP/1.0 with a transfer-encoding",
        ));
    }
    Ok(reply)
}

/// One event of a backend's stream.
#[derive(Clone)]
pub struct Sse {
    pub id: Option<String>,
    pub event: String,
    pub data: String,
}

/// What a backend's stream said: an event, or a comment (its heartbeat).
pub enum Heard {
    Event(Sse),
    Beat,
}

/// How long a backend's stream may stay silent before cooee closes it.
pub const IDLE: Duration = Duration::from_secs(60);

/// `reply`'s event stream, read only as it is pulled (so a slow page holds back the backend, never a buffer); it ends
/// when the backend closes it or is silent for `IDLE`, and dropping it closes the connection. A stream has no
/// deadline and holds no place among its backend's calls.
///
/// # Errors
/// When the connection can't join the async runtime.
pub fn events(reply: Reply) -> io::Result<impl Stream<Item = Heard> + Send + 'static> {
    let Reply { body, permit, .. } = reply;
    drop(permit);
    let read = body.buffer().to_vec();
    let socket = body.into_inner().s;
    socket.set_nonblocking(true)?;
    let socket = tokio::net::TcpStream::from_std(socket)?;
    let lines =
        tokio::io::BufReader::new(AsyncReadExt::chain(io::Cursor::new(read), socket)).lines();
    Ok(stream::unfold(
        (lines, None::<String>),
        |(mut lines, mut id)| async move {
            let (mut event, mut data) = (String::new(), String::new());
            loop {
                let line = tokio::time::timeout(IDLE, lines.next_line()).await;
                let Ok(Ok(Some(line))) = line else {
                    return None;
                };
                let (k, v) = line.split_once(':').unwrap_or((&line, ""));
                let v = v.strip_prefix(' ').unwrap_or(v);
                match k {
                    "" if line.is_empty() && !data.is_empty() => {
                        let e = Sse {
                            id: id.clone(),
                            event,
                            data,
                        };
                        return Some((Heard::Event(e), (lines, id)));
                    }
                    "" if !line.is_empty() => return Some((Heard::Beat, (lines, id))),
                    "id" => id = Some(v.to_owned()),
                    "event" => v.clone_into(&mut event),
                    "data" if data.is_empty() => v.clone_into(&mut data),
                    "data" => {
                        data.push('\n');
                        data.push_str(v);
                    }
                    _ => {}
                }
            }
        },
    ))
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
    if let (Some(value), Some(o)) = (value, body.as_object_mut()) {
        o.insert("value".to_owned(), value.into());
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
/// answers 404 (it serves none: its manifest's are offered).
///
/// # Errors
/// When the backend can't be reached in 5 s, or answers anything else.
pub fn profiles(p: &Plugin) -> io::Result<Option<Vec<Profile>>> {
    let r = call(
        p,
        "GET",
        "/cooee/profiles",
        &[],
        &[],
        Duration::from_secs(5),
    )?;
    match r.status {
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
    }
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

#[cfg(test)]
mod tests {
    use std::{net::TcpListener, sync::mpsc, thread};

    use futures_util::StreamExt;

    use super::*;

    fn silent_backend() -> (Plugin, mpsc::Receiver<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let (closed, rx) = mpsc::channel();
        thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut head = [0; 1024];
            let _ = s.read(&mut head).unwrap();
            let sent =
                "HTTP/1.0 200 OK\r\nContent-Type: text/event-stream\r\n\r\n: open\n\ndata: one\n\n";
            s.write_all(sent.as_bytes()).unwrap();
            while s.read(&mut head).is_ok_and(|n| n > 0) {}
            closed.send(()).unwrap();
        });
        let p = Plugin {
            name: "a".to_owned(),
            label: "A".to_owned(),
            url: format!("http://127.0.0.1:{port}"),
            module: String::new(),
            token: "token-a-0123456789abcdef".to_owned(),
        };
        (p, rx)
    }

    #[tokio::test(start_paused = true)]
    async fn a_silent_backend_stream_is_closed_after_idle() {
        let (p, closed) = silent_backend();
        let reply = call(&p, "GET", "/events", &[], &[], Duration::from_secs(5)).unwrap();
        let mut heard = Box::pin(events(reply).unwrap());
        let started = tokio::time::sleep(Duration::ZERO).deadline();
        assert!(matches!(heard.next().await, Some(Heard::Beat)));
        let Some(Heard::Event(e)) = heard.next().await else {
            panic!("no event");
        };
        assert_eq!(e.data, "one");
        assert!(heard.next().await.is_none());
        assert_eq!(started.elapsed(), IDLE);
        closed.recv_timeout(Duration::from_secs(5)).unwrap();
    }

    fn trickle(head: bool) -> Plugin {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        thread::spawn(move || {
            for mut s in listener.incoming().flatten() {
                thread::spawn(move || {
                    let mut first = [0; 512];
                    s.read(&mut first).map(drop).ok();
                    if head {
                        s.write_all(b"HTTP/1.0 200 OK\r\n\r\n").ok();
                    }
                    for b in b"HTTP/1.0 200 OK\r\n\r\n".iter().cycle() {
                        if s.write_all(&[*b]).is_err() {
                            return;
                        }
                        cooee_core::clock::sleep(Duration::from_secs(1));
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
    fn calls_reuse_a_kept_alive_connection_and_a_closed_one_is_replaced() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let (opened, rx) = mpsc::channel();
        thread::spawn(move || {
            for (n, s) in listener.incoming().flatten().enumerate() {
                opened.send(()).unwrap();
                let mut r = BufReader::new(s);
                for i in 0..2 {
                    let mut line = String::new();
                    while r.read_line(&mut line).unwrap() > 2 {
                        line.clear();
                    }
                    let body = format!("{n}.{i}");
                    let sent = format!(
                        "HTTP/1.0 200 OK\r\nConnection: keep-alive\r\nContent-Length: {}\r\n\r\n{body}",
                        body.len()
                    );
                    r.get_mut().write_all(sent.as_bytes()).unwrap();
                }
            }
        });
        let p = Plugin {
            name: "kept".to_owned(),
            label: String::new(),
            url,
            module: String::new(),
            token: "token-0123456789abcdef".to_owned(),
        };
        let got: Vec<_> = (0..3)
            .map(|_| {
                let reply = call(&p, "GET", "/x", &[], &[], Duration::from_secs(5)).unwrap();
                String::from_utf8(reply.bytes().unwrap()).unwrap()
            })
            .collect();
        assert_eq!(got, ["0.0", "0.1", "1.0"]);
        assert_eq!(rx.try_iter().count(), 2);
    }

    #[test]
    fn a_trickling_backend_fails_at_the_deadline() {
        let deadline = Duration::from_millis(2500);
        for head in [false, true] {
            let p = trickle(head);
            let at = cooee_core::clock::instant();
            let got = call(&p, "GET", "/x", &[], &[], deadline).and_then(Reply::bytes);
            let took = at.elapsed();
            assert!(got.is_err(), "head {head}: it answered");
            assert!(
                took >= deadline
                    && (option_env!("CI").is_none()
                        || took < deadline + Duration::from_millis(500)),
                "head {head}: failed after {took:?}"
            );
        }
    }
}
