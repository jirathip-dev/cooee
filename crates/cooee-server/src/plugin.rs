//! Plugins (docs/plugins.md): a tab whose UI module, served by the plugin's own backend, runs in a sandboxed frame.
//! Its calls reach that backend through cooee with the plugin's token, never the device's cookie: one HTTP/1.0
//! request per call (so the answer is never chunked), an event stream read line by line.

use std::{
    io::{self, BufRead, BufReader, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};

use base64::{Engine, engine::general_purpose::STANDARD};
use cooee_core::Plugin;
use sha2::{Digest, Sha256};

/// The largest answer cooee passes on.
const MAX_BODY: u64 = 16 << 20;

/// Checks the plugins: unique names of lowercase letters, digits and `-`, an `http://` backend, a module path and
/// a token of at least 16 characters.
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
        if !p.module.starts_with('/') {
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
    pub body: BufReader<TcpStream>,
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

/// One request to `p`'s backend, with its token: `target` is the path and query, `headers` go as given. A read
/// waits at most `timeout` (none: forever, for an event stream).
///
/// # Errors
/// When the backend can't be reached or its answer isn't HTTP.
pub fn call(
    p: &Plugin,
    method: &str,
    target: &str,
    headers: &[(&str, &str)],
    body: &[u8],
    timeout: Option<Duration>,
) -> io::Result<Reply> {
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
    let mut s = TcpStream::connect_timeout(&addr, Duration::from_secs(5))?;
    s.set_read_timeout(timeout)?;
    s.set_write_timeout(Some(Duration::from_secs(10)))?;
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

/// Reads `reply`'s event stream, giving each event to `f` until it returns false or the stream ends.
pub fn events(reply: Reply, mut f: impl FnMut(Sse) -> bool) {
    let mut e = Sse {
        id: None,
        event: String::new(),
        data: String::new(),
    };
    let mut lines = reply.body.lines();
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

/// Sends the owner's answer to choice card `item` in `thread`: `POST /cooee/choice`.
///
/// # Errors
/// When the backend can't be reached or doesn't answer 2xx.
pub fn answer(p: &Plugin, thread: &str, item: u64, option: &str) -> io::Result<()> {
    let body = serde_json::json!({ "thread": thread, "item": item, "option": option }).to_string();
    let json = [("Content-Type", "application/json")];
    let reply = call(
        p,
        "POST",
        "/cooee/choice",
        &json,
        body.as_bytes(),
        Some(Duration::from_secs(10)),
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
    let reply = call(p, "GET", &p.module, &[], &[], Some(Duration::from_secs(10)))?;
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
         body{{margin:0;padding:12px;background:var(--bg);color:var(--fg);font:15px/1.4 system-ui,sans-serif}}</style>\
         </head><body><script type=\"module\">{script}</script></body></html>"
    );
    let csp = format!(
        "sandbox allow-scripts; default-src 'none'; script-src 'sha256-{hash}'; style-src 'unsafe-inline'; \
         img-src data: blob:; base-uri 'none'; form-action 'none'; frame-ancestors 'self'"
    );
    Ok((page, csp))
}
