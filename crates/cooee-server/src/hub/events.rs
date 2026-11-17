use std::{
    collections::BTreeMap,
    sync::{Mutex, atomic::Ordering},
    time::Duration,
};

use cooee_core::{
    Body, Change, Queued, ShedError, Workflow,
    shed_proto::{self, Event},
};
use serde_json::json;

use crate::push::Note;

use super::{After, Hub, HubError, Live, STUCK, lock, now};

impl Hub {
    pub(super) fn follow(&self) {
        let mut last: Option<u64> = None;
        let mut said = String::new();
        loop {
            let stream = self
                .reattach()
                .and_then(|since| self.shed.events(BTreeMap::new(), last.or(since)));
            match stream {
                Ok(events) => {
                    self.shed_seen(None);
                    said.clear();
                    for event in events {
                        last = Some(event.seq);
                        self.apply(event);
                    }
                }
                Err(e) if e.to_string() != said => {
                    said = e.to_string();
                    if !self.shed_seen(Some(&e)) {
                        cooee_core::note!("cooee: {said}");
                    }
                }
                Err(_) => {}
            }
            cooee_core::clock::sleep(Duration::from_secs(1));
        }
    }

    pub(super) fn shed_seen(&self, error: Option<&ShedError>) -> bool {
        let version = matches!(error, Some(ShedError::Version { .. }));
        if error.is_some() && !version
            || self.shed_version.swap(version, Ordering::SeqCst) == version
        {
            return version;
        }
        let body = error.map_or(
            "shed speaks this build's protocol again".to_owned(),
            ToString::to_string,
        );
        cooee_core::note!("cooee: {body}");
        self.push.send(&Note {
            title: "cooee".to_owned(),
            body,
            tag: "shed".to_owned(),
            url: "/".to_owned(),
            item: 0,
        });
        version
    }

    /// Applies one shed event to the thread whose terminal it names; a new Workflow session's first is listed.
    pub fn apply(&self, event: Event) {
        let found = lock(&self.terminals).get(&event.name).cloned();
        let found = found.or_else(|| {
            Workflow::from_labels(&event.labels)?;
            self.adopt(&self.shed.get(&event.name).ok()?)
        });
        let Some(found) = found else { return };
        let mut live = lock(&found);
        if !live.sending.is_empty() {
            live.held.push(event);
            return;
        }
        if let Err(e) = self.load(&mut live) {
            cooee_core::note!("cooee: {e}");
            return;
        }
        let after = self.applied(&mut live, event);
        drop(live);
        self.after(&found, after);
    }

    pub(super) fn applied(&self, live: &mut Live, event: Event) -> Option<After> {
        if event.seq <= live.applied {
            return None;
        }
        live.applied = event.seq;
        live.moved = now();
        let mut changes = Vec::new();
        if live.stuck {
            live.stuck = false;
            "working".clone_into(&mut live.chat.turn.state);
            live.chat.turn.note = None;
            changes.push(Change::State(live.chat.turn.clone()));
        }
        if let (Some(_), shed_proto::EventKind::Turn { prompt }) =
            (&live.meta.workflow, &event.kind)
        {
            let own = live
                .chat
                .items
                .last()
                .is_some_and(|i| matches!(&i.body, Body::You { text, .. } if text == prompt));
            if !own {
                let body = Body::You {
                    text: prompt.clone(),
                    random_id: String::new(),
                    files: Vec::new(),
                    routine: None,
                };
                changes.push(Change::Item(live.chat.push(now(), event.seq, body)));
            }
        }
        changes.extend(live.chat.apply(now(), event.seq, event.kind));
        let chunk = changes.iter().any(|c| match c {
            Change::Text(_) => true,
            Change::Item(i) => matches!(i.body, Body::Agent { .. }),
            Change::State(_) => false,
        });
        if let Some(sent) = live.sent.take_if(|_| chunk) {
            cooee_core::perf::first_token(&live.meta.id, &live.meta.agent, sent);
        }
        let reply = changes.iter().find_map(|c| match c {
            Change::Item(i) if matches!(&i.body, Body::Agent { stop_reason, .. } if stop_reason != "cancelled") => {
                Some(i.body.clone())
            }
            _ => None,
        });
        let seat = (live.meta.room.clone(), live.meta.profile.clone());
        let asked = format!("{}>{}", live.meta.asked_by, seat.1);
        let next = live
            .publish(self, changes)
            .map_err(HubError::from)
            .and_then(|()| self.drain(live));
        Some(After {
            next,
            reply,
            seat,
            asked,
        })
    }

    pub(super) fn after(&self, live: &Mutex<Live>, after: Option<After>) {
        let Some(After {
            next,
            reply,
            seat,
            asked,
        }) = after
        else {
            return;
        };
        let done = next.and_then(|q| self.send_next(live, q));
        let done = match reply {
            Some(body) if !seat.0.is_empty() => done.and(self.replied(&seat.0, &seat.1, body)),
            Some(Body::Agent { text, .. }) => {
                lock(&self.asks).remove(&asked).map(|tx| tx.send(text));
                done
            }
            _ => done,
        };
        if let Err(e) = done {
            cooee_core::note!("cooee: {e}");
        }
    }

    pub(super) fn drain(&self, live: &mut Live) -> Result<Option<Queued>, HubError> {
        let between = !matches!(
            live.chat.turn.state.as_str(),
            "working" | "blocked" | "ended"
        );
        if !between || live.meta.queue.is_empty() || live.meta.closed || !live.sending.is_empty() {
            return Ok(None);
        }
        let next = live.meta.queue.remove(0);
        self.queued(live)?;
        Ok(Some(next))
    }

    pub(super) fn send_next(
        &self,
        live: &Mutex<Live>,
        next: Option<Queued>,
    ) -> Result<(), HubError> {
        next.map_or(Ok(()), |q| self.deliver(live, q, "").map(drop))
    }

    pub(super) fn queued(&self, live: &mut Live) -> Result<(), HubError> {
        self.store.save(&live.meta)?;
        let data = json!(live.meta.queue).to_string();
        live.emit("queue", data);
        Ok(())
    }

    /// Surfaces each working turn with no shed event for 30 minutes before `now` as needs-you, once: the thread
    /// shows blocked with a note, and a push says so.
    pub fn watch(&self, now: u64) {
        self.deadlines(now);
        self.expire(now);
        for live in self.all() {
            let mut live = lock(&live);
            if live.stuck || live.chat.turn.state != "working" || now < live.moved + STUCK {
                continue;
            }
            live.stuck = true;
            "blocked".clone_into(&mut live.chat.turn.state);
            live.chat.turn.note = Some("no progress for 30 minutes".to_owned());
            let item = live.chat.items.last().map_or(0, |i| i.id);
            let said = (
                "needs",
                live.meta.agent.as_str(),
                "no progress for 30 minutes",
            );
            let n = Note::new(&live.meta, said, item, &self.theme);
            self.push.send(&self.routed(&live.meta, n));
            let turn = live.chat.turn.clone();
            if let Err(e) = live.publish(self, vec![Change::State(turn)]) {
                cooee_core::note!("cooee: {e}");
            }
        }
    }
}
