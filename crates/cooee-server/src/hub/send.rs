use std::{fs, sync::Mutex, time::Instant};

use cooee_core::{
    Body, Calls as _, Change, Item, Kind, Queued, Via, agents, routines::Mark, shed_proto,
};

use super::{Hub, HubError, live::Live, lock, now, refused};

/// A file sent with a message: its name on the phone and its bytes.
pub struct Upload {
    /// The name it had (only its safe characters are kept).
    pub name: String,
    /// Its bytes.
    pub data: Vec<u8>,
}

impl Hub {
    /// Sends the owner's message with its `files`, sent `via` voice or typed: once per `random_id` (a retry of a
    /// stored send returns it again). The message is stored only once shed took it; the agent gets the files' paths
    /// under its text, and a voice message the persona's spoken-reply hint above it.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a closed thread, an empty message or a stopped agent; [`HubError::Shed`] when shed
    /// fails.
    pub fn send(
        &self,
        id: &str,
        text: &str,
        random_id: &str,
        files: &[Upload],
        via: Option<Via>,
    ) -> Result<Item, HubError> {
        let live = self.live(id)?;
        if lock(&live).meta.kind == Kind::Room {
            return self.room_send(&live, text, random_id, files);
        }
        self.send_after(id, text, random_id, files, "", via)
    }

    pub(super) fn send_after(
        &self,
        id: &str,
        text: &str,
        random_id: &str,
        files: &[Upload],
        before: &str,
        via: Option<Via>,
    ) -> Result<Item, HubError> {
        let live = self.live(id)?;
        let l = lock(&live);
        if let Some(item) = Self::sent(&l, random_id) {
            return Ok(item);
        }
        if l.sending.iter().any(|r| r == random_id) {
            return Err(HubError::Refused(
                "that message is still on its way".to_owned(),
            ));
        }
        let q = Queued {
            via,
            ..self.take(&l, text, random_id, files)?
        };
        drop(l);
        self.deliver(&live, q, before)
    }

    pub(super) fn sent(live: &Live, random_id: &str) -> Option<Item> {
        let mine = |i: &&Item| matches!(&i.body, Body::You { random_id: r, .. } if r == random_id);
        live.chat.items.iter().rev().find(mine).cloned()
    }

    pub(super) fn take(
        &self,
        live: &Live,
        text: &str,
        random_id: &str,
        files: &[Upload],
    ) -> Result<Queued, HubError> {
        handing_off(live)?;
        if live.meta.closed || (text.trim().is_empty() && files.is_empty()) {
            return Err(HubError::Refused(
                "the thread is closed or the message empty".to_owned(),
            ));
        }
        let dir = self.files.join(&live.meta.id);
        fs::create_dir_all(&dir).map_err(|e| refused(&dir, &e))?;
        let stamp = now();
        let mut names = Vec::new();
        for (i, f) in files.iter().enumerate() {
            let base = f.name.rsplit(['/', '\\']).next().unwrap_or_default();
            let safe: String = base
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || "._-".contains(c) {
                        c
                    } else {
                        '_'
                    }
                })
                .take(80)
                .collect();
            let name = format!("{stamp}-{i}-{}", safe.trim_start_matches('.'));
            let path = dir.join(&name);
            fs::write(&path, &f.data).map_err(|e| refused(&path, &e))?;
            names.push(name);
        }
        Ok(Queued {
            text: text.to_owned(),
            random_id: random_id.to_owned(),
            files: names,
            ..Queued::default()
        })
    }

    fn compose(&self, l: &Live, q: &Queued, before: &str) -> String {
        let spoken = (q.via == Some(Via::Voice)).then(|| {
            agents::find(self.config_dir(), &self.data, &l.meta.profile)
                .ok()
                .and_then(|a| a.spoken)
                .unwrap_or_else(|| agents::SPOKEN.to_owned())
        });
        let text = match spoken.filter(|s| !s.trim().is_empty()) {
            Some(s) => format!("{}\n\n{}", s.trim_end(), q.text),
            None => q.text.clone(),
        };
        let mut prompt = if before.trim().is_empty() {
            text
        } else {
            format!("{}\n\n{text}", before.trim_end())
        };
        if !q.files.is_empty() {
            prompt.push_str("\n\nAttached files:");
            for name in &q.files {
                let path = self.files.join(&l.meta.id).join(name);
                prompt.push_str("\n- ");
                prompt.push_str(&path.to_string_lossy());
            }
        }
        prompt
    }

    pub(super) fn deliver(
        &self,
        live: &Mutex<Live>,
        q: Queued,
        before: &str,
    ) -> Result<Item, HubError> {
        let start = cooee_core::clock::instant();
        let (terminal, machine, ended, handover, prompt) = {
            let mut l = lock(live);
            l.sending.push(q.random_id.clone());
            let handover = l.meta.handover.clone();
            let before = if !before.trim().is_empty() {
                before
            } else if l.meta.asked_by.is_empty() {
                handover.as_str()
            } else {
                super::ask::OWNER_NOTE
            };
            let prompt = self.compose(&l, &q, before);
            let ended = l.chat.turn.state == "ended";
            (
                l.meta.terminal.clone(),
                l.meta.machine.clone(),
                ended,
                handover,
                prompt,
            )
        };
        let sent = self.prompt(&terminal, &machine, ended, prompt.trim());
        let mut l = lock(live);
        l.sending.retain(|r| *r != q.random_id);
        let mismatch = match &sent {
            Ok(()) => self.shed_seen(None),
            Err(HubError::Shed(e)) => self.shed_seen(Some(e)),
            Err(_) => false,
        };
        if let (true, Err(e)) = (mismatch, &sent) {
            let item = l.chat.push(
                now(),
                0,
                Body::Note {
                    text: e.to_string(),
                },
            );
            if let Err(e) = l.publish(self, vec![Change::Item(item)]) {
                cooee_core::note!("cooee: {e}");
            }
        }
        let stored = sent.and_then(|()| self.store_sent(&mut l, q, start, &handover));
        let held = if l.sending.is_empty() {
            std::mem::take(&mut l.held)
        } else {
            Vec::new()
        };
        let after: Vec<_> = held.into_iter().map(|e| self.applied(&mut l, e)).collect();
        drop(l);
        for a in after {
            self.after(live, a);
        }
        stored
    }

    fn store_sent(
        &self,
        live: &mut Live,
        q: Queued,
        start: Instant,
        handover: &str,
    ) -> Result<Item, HubError> {
        if !handover.is_empty() && live.meta.handover == handover {
            live.meta.handover.clear();
            self.save(live)?;
        }
        live.sent = Some(start);
        if let Some(card) = q
            .denied
            .and_then(|c| live.chat.items.iter().find(|i| i.id == c))
        {
            return Ok(card.clone());
        }
        let first = live.meta.random_id == q.random_id;
        let body = Body::You {
            text: q.text,
            random_id: q.random_id,
            files: q.files,
            routine: q.routine,
            via: q.via,
        };
        let item = live.chat.push(now(), 0, body);
        if first {
            live.pending = None;
        }
        live.publish(self, vec![Change::Item(item.clone())])?;
        Ok(item)
    }

    fn prompt(
        &self,
        terminal: &str,
        machine: &str,
        ended: bool,
        prompt: &str,
    ) -> Result<(), HubError> {
        let got = ended.then(|| self.shed.get(terminal));
        match got {
            Some(Err(e)) if e.code() == Some(cooee_core::shed_proto::ErrorCode::NoSuchTerminal) => {
                return Err(e.into());
            }
            Some(t)
                if !t
                    .as_ref()
                    .is_ok_and(|t| t.state == shed_proto::State::Running) =>
            {
                return Err(HubError::Refused(
                    "the agent stopped, so it can't take the message".to_owned(),
                ));
            }
            _ => {}
        }
        if let Err(e) = self.shed.prompt(terminal, prompt) {
            if e.code() == Some(cooee_core::shed_proto::ErrorCode::NoLink) {
                self.reachable(machine)?;
            }
            return Err(e.into());
        }
        Ok(())
    }

    /// Queues the owner's message, sent `via` voice or typed, for when the turn ends (between turns it goes at once);
    /// once per `random_id`. Returns the queue.
    ///
    /// # Errors
    /// As [`Hub::send`].
    pub fn queue(
        &self,
        id: &str,
        text: &str,
        random_id: &str,
        files: &[Upload],
        via: Option<Via>,
    ) -> Result<Vec<Queued>, HubError> {
        self.queue_as(id, (text, random_id), files, (None, via))
    }

    /// [`Hub::queue`] by `routine` (#75) when given, not the owner, sent `via` voice or typed.
    ///
    /// # Errors
    /// As [`Hub::send`].
    pub fn queue_as(
        &self,
        id: &str,
        (text, random_id): (&str, &str),
        files: &[Upload],
        (routine, via): (Option<Mark>, Option<Via>),
    ) -> Result<Vec<Queued>, HubError> {
        self.enqueue(id, random_id, |l| {
            Ok(Queued {
                routine,
                via,
                ..self.take(l, text, random_id, files)?
            })
        })
    }

    pub(super) fn enqueue(
        &self,
        id: &str,
        random_id: &str,
        make: impl FnOnce(&Live) -> Result<Queued, HubError>,
    ) -> Result<Vec<Queued>, HubError> {
        let live = self.live(id)?;
        let mut l = lock(&live);
        let have = l.meta.queue.iter().any(|q| q.random_id == random_id);
        if have || Self::sent(&l, random_id).is_some() {
            return Ok(l.meta.queue.clone());
        }
        let q = make(&l)?;
        l.meta.queue.push(q);
        self.queued(&mut l)?;
        let next = self.drain(&mut l)?;
        drop(l);
        self.send_next(&live, next)?;
        Ok(lock(&live).meta.queue.clone())
    }

    /// Takes queued message `random_id` out of the queue; with `now`, sends it into the turn in progress (steers).
    /// Returns the queue.
    ///
    /// # Errors
    /// As [`Hub::send`]; [`HubError::Refused`] when it isn't queued.
    pub fn unqueue(&self, id: &str, random_id: &str, now: bool) -> Result<Vec<Queued>, HubError> {
        let live = self.live(id)?;
        let mut l = lock(&live);
        if now {
            handing_off(&l)?;
        }
        let Some(at) = l.meta.queue.iter().position(|q| q.random_id == random_id) else {
            return Err(HubError::Refused(
                "that message isn't queued any more".to_owned(),
            ));
        };
        let q = l.meta.queue.remove(at);
        self.queued(&mut l)?;
        drop(l);
        self.send_next(&live, now.then_some(q))?;
        Ok(lock(&live).meta.queue.clone())
    }
}

fn handing_off(live: &Live) -> Result<(), HubError> {
    if live.meta.handoff == 0 {
        return Ok(());
    }
    Err(HubError::Refused(
        "HANDOFF: the agent waits for you to hand back; your note goes with it".to_owned(),
    ))
}
