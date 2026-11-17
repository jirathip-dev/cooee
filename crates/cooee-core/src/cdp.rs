//! A persona's Chromium over the `DevTools` protocol (#162): its current tab, and commands on it over a `WebSocket`.

use std::{
    io::{self, BufRead as _, BufReader, Read, Write as _},
    net::TcpStream,
    time::Duration,
};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// A tab of the browser.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct Tab {
    /// Its address.
    pub url: String,
    /// Its title.
    pub title: String,
    #[serde(rename = "webSocketDebuggerUrl", skip_serializing)]
    socket: String,
    #[serde(rename = "type", skip_serializing)]
    kind: String,
}

fn connect(host: &str) -> io::Result<TcpStream> {
    let s = TcpStream::connect(host)?;
    s.set_read_timeout(Some(Duration::from_secs(10)))?;
    Ok(s)
}

/// The current tab of the browser on CDP port `port`: the page it used last.
///
/// # Errors
/// When the browser doesn't answer or has no page open.
pub fn current(port: u16) -> io::Result<Tab> {
    let mut s = connect(&format!("127.0.0.1:{port}"))?;
    write!(
        s,
        "GET /json/list HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
    )?;
    let mut out = String::new();
    s.read_to_string(&mut out)?;
    let body = out.split_once("\r\n\r\n").map_or("", |(_, b)| b);
    let tabs: Vec<Tab> = serde_json::from_str(body).map_err(io::Error::other)?;
    tabs.into_iter()
        .find(|t| t.kind == "page")
        .ok_or_else(|| io::Error::other("the browser has no tab open"))
}

fn frame(payload: &[u8]) -> Vec<u8> {
    let mut f = vec![0x81];
    let n = payload.len();
    match (u8::try_from(n), u16::try_from(n)) {
        (Ok(b), _) if b < 126 => f.push(0x80 | b),
        (_, Ok(w)) => {
            f.push(0xfe);
            f.extend(w.to_be_bytes());
        }
        _ => {
            f.push(0xff);
            f.extend(u64::try_from(n).unwrap_or(u64::MAX).to_be_bytes());
        }
    }
    f.extend([0; 4]);
    f.extend(payload);
    f
}

fn message(r: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut msg = Vec::new();
    loop {
        let mut head = [0; 2];
        r.read_exact(&mut head)?;
        let [b0, b1] = head;
        let len = match b1 & 0x7f {
            126 => {
                let mut b = [0; 2];
                r.read_exact(&mut b)?;
                u64::from(u16::from_be_bytes(b))
            }
            127 => {
                let mut b = [0; 8];
                r.read_exact(&mut b)?;
                u64::from_be_bytes(b)
            }
            n => u64::from(n),
        };
        let mut body = Vec::new();
        r.by_ref().take(len).read_to_end(&mut body)?;
        if u64::try_from(body.len()).ok() != Some(len) {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        match b0 & 0x0f {
            8 => return Err(io::Error::other("the tab closed its WebSocket")),
            9 | 10 => continue,
            _ => msg.extend(body),
        }
        if b0 & 0x80 != 0 {
            return Ok(msg);
        }
    }
}

impl Tab {
    /// Runs each `(method, params)` on the tab in order and answers the last one's result.
    ///
    /// # Errors
    /// When the tab can't be reached or a command fails.
    pub fn run(&self, calls: &[(&str, Value)]) -> io::Result<Value> {
        let bad = || io::Error::other(format!("not a tab's WebSocket: {}", self.socket));
        let (host, path) = self
            .socket
            .strip_prefix("ws://")
            .and_then(|r| r.split_once('/'))
            .ok_or_else(bad)?;
        let mut s = connect(host)?;
        write!(
            s,
            "GET /{path} HTTP/1.1\r\nHost: {host}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: Y29vZWUtbGl2ZS12aWV3IQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
        )?;
        let mut r = BufReader::new(s.try_clone()?);
        let mut line = String::new();
        r.read_line(&mut line)?;
        if !line.contains(" 101 ") {
            return Err(io::Error::other(format!("no WebSocket: {}", line.trim())));
        }
        line.clear();
        while r.read_line(&mut line)? > 2 {
            line.clear();
        }
        let mut last = Value::Null;
        for (id, (method, params)) in (1_u64..).zip(calls) {
            let call = json!({ "id": id, "method": method, "params": params });
            s.write_all(&frame(call.to_string().as_bytes()))?;
            last = loop {
                let v: Value =
                    serde_json::from_slice(&message(&mut r)?).map_err(io::Error::other)?;
                if v.get("id") != Some(&json!(id)) {
                    continue;
                }
                if let Some(e) = v.get("error") {
                    return Err(io::Error::other(format!("{method}: {e}")));
                }
                break v.get("result").cloned().unwrap_or_default();
            };
        }
        Ok(last)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_are_masked_text_with_the_three_length_forms_and_read_back_whole() {
        for n in [0, 125, 126, 65_535, 65_536] {
            let payload = vec![b'x'; n];
            let f = frame(&payload);
            assert_eq!(f[0], 0x81);
            assert_eq!(f[1] & 0x80, 0x80);
            let unmasked: Vec<u8> = [&[0x81, f[1] & 0x7f][..], &f[2..]].concat();
            let header = f.len() - n - 4;
            let server: Vec<u8> = [&unmasked[..header], &unmasked[header + 4..]].concat();
            assert_eq!(message(&mut server.as_slice()).unwrap(), payload, "{n}");
        }
        let split = [&[0x01, 2][..], b"ab", &[0x89, 0], &[0x80, 1], b"c"].concat();
        assert_eq!(message(&mut split.as_slice()).unwrap(), b"abc");
        assert!(message(&mut [0x88, 0].as_slice()).is_err());
    }
}
