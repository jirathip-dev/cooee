//! A fake shed: an in-process stream of ACP-like events (`text` chunks, `tool_call` status changes, an
//! `approval_request`, the final `reply`) at a configurable rate. The server keeps the run's state; a render reads it.

use std::{sync::Mutex, time::Duration};
use tokio::sync::broadcast;

#[derive(Clone, Copy, PartialEq)]
pub enum Status { Pending, Running, Done }

#[derive(Clone)]
pub struct Tool { pub name: &'static str, pub status: Status }

#[derive(Clone)]
pub struct Approval { pub title: &'static str, pub options: Vec<&'static str>, pub chosen: Option<String> }

pub enum Ev { Text(String), ToolCall(usize, &'static str, Status), ApprovalRequest(&'static str, Vec<&'static str>), Reply }

#[derive(Default)]
pub struct Run {
    pub text: String,
    /// Where each token ends in `text`, so the reply can be cut into chunks.
    pub ends: Vec<usize>,
    pub tokens: usize,
    pub tools: Vec<Tool>,
    pub approval: Option<Approval>,
    pub done: bool,
    /// Bumped on every change: `text_v` for the reply text, `rows_v` for the tool and approval rows.
    pub text_v: u64,
    pub rows_v: u64,
    pub epoch: u64,
}

impl Run {
    fn apply(&mut self, ev: Ev) {
        match ev {
            Ev::Text(t) => { self.text.push_str(&t); self.ends.push(self.text.len()); self.tokens += 1; self.text_v += 1 }
            Ev::ToolCall(i, name, status) => {
                if i == self.tools.len() { self.tools.push(Tool { name, status }) } else { self.tools[i].status = status }
                self.rows_v += 1
            }
            Ev::ApprovalRequest(title, options) => { self.approval = Some(Approval { title, options, chosen: None }); self.rows_v += 1 }
            Ev::Reply => { self.done = true; self.rows_v += 1 }
        }
    }
}

pub struct Shed {
    pub run: Mutex<Run>,
    /// What the user sent from the composer.
    pub sent: Mutex<Vec<String>>,
    pub tx: broadcast::Sender<()>,
    pub tokens: usize,
    pub rate: f64,
}

impl Shed {
    pub fn new(tokens: usize, rate: f64) -> Self {
        Self { run: Mutex::default(), sent: Mutex::default(), tx: broadcast::channel(1024).0, tokens, rate }
    }

    fn apply(&self, ev: Ev) {
        self.run.lock().unwrap().apply(ev);
        let _ = self.tx.send(());
    }

    /// Starts a fresh run in the background; a run already going stops at its next event.
    pub fn start(&'static self) {
        let epoch = {
            let mut r = self.run.lock().unwrap();
            let (epoch, text_v, rows_v) = (r.epoch + 1, r.text_v + 1, r.rows_v + 1);
            *r = Run { epoch, text_v, rows_v, ..Run::default() };
            epoch
        };
        let _ = self.tx.send(());
        let n = self.tokens;
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_secs_f64(1.0 / self.rate));
            for i in 0..n {
                tick.tick().await;
                if self.run.lock().unwrap().epoch != epoch { return }
                // Each token is numbered, so a client can prove nothing was dropped or duplicated.
                self.apply(Ev::Text(format!("w{i} ")));
                let at = |f: usize| i == n * f / 10;
                if at(1) { self.apply(Ev::ToolCall(0, "read_file src/main.rs", Status::Pending)) }
                if at(2) { self.apply(Ev::ToolCall(0, "read_file src/main.rs", Status::Running)) }
                if at(3) { self.apply(Ev::ToolCall(0, "read_file src/main.rs", Status::Done)) }
                if at(4) { self.apply(Ev::ToolCall(1, "cargo test", Status::Running)) }
                if at(5) { self.apply(Ev::ApprovalRequest("Run `git push`?", vec!["Allow once", "Always", "Deny"])) }
                if at(7) { self.apply(Ev::ToolCall(1, "cargo test", Status::Done)) }
            }
            self.apply(Ev::Reply);
        });
    }

    /// The approval row's answer, from the browser.
    pub fn choose(&self, option: String) -> bool {
        let mut r = self.run.lock().unwrap();
        let Some(a) = r.approval.as_mut().filter(|a| a.chosen.is_none() && a.options.contains(&option.as_str())) else { return false };
        a.chosen = Some(option);
        r.rows_v += 1;
        drop(r);
        let _ = self.tx.send(());
        true
    }
}

/// Tokens per reply chunk: a full chunk is sealed and never sent again; only the last one streams.
pub const CHUNK: usize = 100;

impl Run {
    /// Chunk `c` of the reply, and whether it is sealed.
    pub fn chunk(&self, c: usize) -> (String, bool) {
        let at = |t: usize| if t == 0 { 0 } else { self.ends.get(t - 1).copied().unwrap_or(self.text.len()) };
        let (from, to) = (at((c * CHUNK).min(self.tokens)), at(((c + 1) * CHUNK).min(self.tokens)));
        (self.text[from..to].to_owned(), self.tokens >= (c + 1) * CHUNK || self.done)
    }
}
