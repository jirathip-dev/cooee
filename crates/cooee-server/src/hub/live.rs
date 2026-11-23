use std::{
    collections::VecDeque,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use cooee_core::{Body, Change, Chat, Queued, StoreError, Thread, shed_proto::Event};
use serde_json::json;
use tokio::sync::broadcast;

use super::glance::{Glance, glance};
use super::{Hub, HubError, Out, now, rooms};
use crate::push::note;

pub(super) static REV: AtomicU64 = AtomicU64::new(0);

pub(super) const REPLAY: u64 = 5 * 60 * 1000;

const TEXT_TICK: u64 = 40;

pub(super) const IDLE: u64 = 10 * 60 * 1000;

pub(super) struct Live {
    pub(super) meta: Thread,
    pub(super) chat: Chat,
    pub(super) applied: u64,
    pub(super) ring: VecDeque<(u64, Out)>,
    pub(super) trimmed: u64,
    pub(super) unsent: String,
    pub(super) text_at: u64,
    pub(super) flushing: bool,
    pub(super) tx: Option<broadcast::Sender<Out>>,
    pub(super) was: String,
    pub(super) moved: u64,
    pub(super) stuck: bool,
    pub(super) pending: Option<String>,
    pub(super) sent: Option<Instant>,
    pub(super) stopped_starting: bool,
    pub(super) asks: rooms::OpenAsks,
    pub(super) used: u64,
    pub(super) unloaded: Option<Glance>,
    pub(super) sending: Vec<String>,
    pub(super) held: Vec<Event>,
}

pub(super) struct After {
    pub(super) next: Result<Option<Queued>, HubError>,
    pub(super) reply: Option<Body>,
    pub(super) seat: (String, String),
    pub(super) asked: String,
}

impl Live {
    pub(super) fn new(meta: Thread, chat: Chat) -> Self {
        Self {
            applied: chat.seq(),
            was: chat.turn.state.clone(),
            meta,
            chat,
            ring: VecDeque::new(),
            trimmed: 0,
            unsent: String::new(),
            text_at: 0,
            flushing: false,
            tx: None,
            moved: now(),
            stuck: false,
            pending: None,
            sent: None,
            stopped_starting: false,
            asks: rooms::OpenAsks::default(),
            used: now(),
            unloaded: None,
            sending: Vec::new(),
            held: Vec::new(),
        }
    }

    pub(super) fn trim(&mut self, now: u64) {
        while let Some((_, out)) = self.ring.pop_front_if(|(at, _)| *at + REPLAY < now) {
            self.trimmed = out.rev;
        }
    }

    pub(super) fn unload(&mut self, now: u64) {
        let turn = &self.chat.turn;
        let busy = matches!(turn.state.as_str(), "working" | "connecting" | "blocked")
            || !turn.text.is_empty()
            || self.pending.is_some()
            || self.sent.is_some()
            || self.stopped_starting
            || !self.sending.is_empty()
            || !self.held.is_empty()
            || self.tx.as_ref().is_some_and(|t| t.receiver_count() > 0);
        if self.unloaded.is_some() || busy || now < self.used + IDLE {
            return;
        }
        self.unloaded = Some(glance(self));
        let turn = std::mem::take(&mut self.chat.turn);
        self.chat = Chat::default();
        self.chat.turn = turn;
        self.ring.shrink_to_fit();
        self.tx = None;
    }

    pub(super) fn text(&mut self, at: u64, text: &str) -> bool {
        self.unsent.push_str(text);
        if at >= self.text_at + TEXT_TICK {
            self.flush(at);
        }
        let schedule = !self.unsent.is_empty() && !self.flushing;
        self.flushing |= schedule;
        schedule
    }

    pub(super) fn tick(&mut self, at: u64) -> Option<u64> {
        let due = self.text_at + TEXT_TICK;
        if !self.unsent.is_empty() && at < due {
            return Some(due);
        }
        self.flush(at);
        self.flushing = false;
        None
    }

    pub(super) fn flush(&mut self, at: u64) {
        if !self.unsent.is_empty() {
            self.text_at = at;
            let text = std::mem::take(&mut self.unsent);
            self.emit("text", json!({ "text": text }).to_string());
        }
    }

    pub(super) fn emit(&mut self, event: &'static str, data: String) {
        if event != "text" && !self.unsent.is_empty() {
            self.flush(now());
        }
        let out = Out {
            rev: REV.fetch_add(1, Ordering::Relaxed) + 1,
            event,
            data,
        };
        let at = now();
        self.trim(at);
        self.ring.push_back((at, out.clone()));
        if let Some(tx) = &self.tx {
            tx.send(out).ok();
        }
    }

    pub(super) fn publish(&mut self, hub: &Hub, changes: Vec<Change>) -> Result<(), StoreError> {
        let store = &hub.store;
        for mut change in changes {
            if let Change::Item(i) = &mut change
                && matches!(i.body, Body::Approval { .. })
                && !self.meta.room.is_empty()
            {
                i.speaker.clone_from(&self.meta.profile);
                if let Some(c) = self.chat.items.iter_mut().find(|c| c.id == i.id) {
                    c.speaker.clone_from(&i.speaker);
                }
                hub.room_ask(&self.meta, i.clone());
            }
            let watched = self.tx.as_ref().is_some_and(|t| t.receiver_count() > 0);
            let beat = self.meta.routine.as_ref();
            if beat.is_some_and(|r| r.id == cooee_core::agents::HEARTBEAT) {
                if let Change::State(t) = &change
                    && t.state != self.was
                    && matches!(t.state.as_str(), "done" | "error")
                {
                    hub.beat_end(&self.meta, &self.chat, t);
                }
            } else if let Some(n) = note(
                &self.meta, &self.chat, &change, &self.was, watched, &hub.theme,
            ) {
                hub.push.send(&hub.routed(&self.meta, n));
            }
            if let Change::State(turn) = &change {
                self.was.clone_from(&turn.state);
            }
            match change {
                Change::Item(item) => {
                    if acknowledged(&item.body) {
                        store.append(&self.meta.id, &item)?;
                    } else {
                        store.queue(&self.meta.id, &item)?;
                    }
                    self.emit("item", serde_json::to_string(&item).unwrap_or_default());
                    if matches!(item.body, Body::Agent { .. }) {
                        self.ring.retain(|(_, o)| o.event != "text");
                    }
                }
                Change::Text(text) => {
                    if self.text(now(), &text) {
                        hub.texts.send(self.meta.id.clone()).ok();
                    }
                }
                Change::State(turn) => self.emit("state", json!(turn).to_string()),
            }
        }
        hub.relist(self);
        Ok(())
    }
}

fn acknowledged(body: &Body) -> bool {
    match body {
        Body::You { .. } | Body::Agent { .. } => true,
        Body::Approval { tapped, chosen, .. } => tapped.is_some() || chosen.is_some(),
        _ => false,
    }
}
