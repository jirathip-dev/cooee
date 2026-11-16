//! An SSE stream from `cooee serve` and its events.

use std::{
    io::{BufRead, BufReader},
    net::TcpStream,
};

use serde_json::Value;

/// An open SSE stream.
pub struct Events(pub(super) BufReader<TcpStream>);

/// One SSE event: its id, name and JSON data.
#[derive(Debug, Clone)]
pub struct Sse {
    pub id: String,
    pub event: String,
    pub data: Value,
}

impl Events {
    /// The next event (keep-alive comments skipped).
    pub fn next(&mut self) -> Sse {
        let mut sse = Sse {
            id: String::new(),
            event: String::new(),
            data: Value::Null,
        };
        loop {
            let mut line = String::new();
            assert!(self.0.read_line(&mut line).unwrap() > 0, "the stream ended");
            let line = line.trim_end_matches(['\r', '\n']);
            match line.split_once(": ") {
                Some(("id", v)) => v.clone_into(&mut sse.id),
                Some(("event", v)) => v.clone_into(&mut sse.event),
                Some(("data", v)) => sse.data = serde_json::from_str(v).unwrap(),
                _ if line.is_empty() && !sse.event.is_empty() => return sse,
                _ => {}
            }
        }
    }

    /// Events until one matches `f` (included).
    pub fn until(&mut self, f: impl Fn(&Sse) -> bool) -> Vec<Sse> {
        let mut out = Vec::new();
        loop {
            let e = self.next();
            let done = f(&e);
            out.push(e);
            if done {
                return out;
            }
        }
    }
}

/// Whether `e` is an `item` of `kind`.
pub fn is_item(e: &Sse, kind: &str) -> bool {
    e.event == "item" && e.data["kind"] == kind
}
