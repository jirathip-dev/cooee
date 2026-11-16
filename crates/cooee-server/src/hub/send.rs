//! Sending to a thread: a message, its files, the queue, and the prompt shed gets.

use std::{fs, sync::Mutex, time::Instant};

use cooee_core::{
    Body, Change, Item, Kind, Queued,
    routines::Mark,
    shed_proto::{self, TerminalInfo},
};

use super::{Hub, HubError, Live, Upload, lock, now, refused};

impl Hub {
    /// Sends the owner's message with its `files`: once per `random_id` (a retry of a stored send returns it
    /// again). The message is stored only once shed took it; the agent gets the files' paths under its text.
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
    ) -> Result<Item, HubError> {
        let live = self.live(id)?;
        if lock(&live).meta.kind == Kind::Room {
            return self.room_send(&live, text, random_id, files);
        }
        self.send_after(id, text, random_id, files, "")
    }

    /// [`Hub::send`] with `before` (a profile's instructions) ahead of the text for the agent; only the text is stored.
    pub(super) fn send_after(
        &self,
        id: &str,
        text: &str,
        random_id: &str,
        files: &[Upload],
        before: &str,
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
        let q = self.take(&l, text, random_id, files)?;
        drop(l);
        self.deliver(&live, q, before)
    }

    /// The stored message `random_id`, if it was sent.
    pub(super) fn sent(live: &Live, random_id: &str) -> Option<Item> {
        let mine = |i: &&Item| matches!(&i.body, Body::You { random_id: r, .. } if r == random_id);
        live.chat.items.iter().rev().find(mine).cloned()
    }

    /// A message to send or queue, its files stored.
    pub(super) fn take(
        &self,
        live: &Live,
        text: &str,
        random_id: &str,
        files: &[Upload],
    ) -> Result<Queued, HubError> {
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
            routine: None,
        })
    }

    /// Sends `q` to the agent, `before` ahead of it, then stores and publishes it; refused when the agent stopped,
    /// whose session would drop it (#253). The thread is unlocked while shed takes it.
    pub(super) fn deliver(
        &self,
        live: &Mutex<Live>,
        q: Queued,
        before: &str,
    ) -> Result<Item, HubError> {
        let start = Instant::now();
        let (terminal, machine, ended, handover, prompt) = {
            let mut l = lock(live);
            l.sending.push(q.random_id.clone());
            let handover = l.meta.handover.clone();
            // After Use a folder…, the hand-over goes once ahead of the next message.
            let before = if !before.trim().is_empty() {
                before
            } else if l.meta.asked_by.is_empty() {
                handover.as_str()
            } else {
                super::ask::OWNER_NOTE
            };
            let mut prompt = if before.trim().is_empty() {
                q.text.clone()
            } else {
                format!("{}\n\n{}", before.trim_end(), q.text)
            };
            if !q.files.is_empty() {
                prompt.push_str("\n\nAttached files:");
                for name in &q.files {
                    let path = self.files.join(&l.meta.id).join(name);
                    prompt.push_str("\n- ");
                    prompt.push_str(&path.to_string_lossy());
                }
            }
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
                eprintln!("cooee: {e}");
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

    /// Stores and publishes message `q`, which shed took (its prompt timed from `start`, with `handover` ahead).
    fn store_sent(
        &self,
        live: &mut Live,
        q: Queued,
        start: Instant,
        handover: &str,
    ) -> Result<Item, HubError> {
        if !handover.is_empty() && live.meta.handover == handover {
            live.meta.handover.clear();
            self.store.save(&live.meta)?;
        }
        live.sent = Some(start);
        let first = live.meta.random_id == q.random_id;
        let body = Body::You {
            text: q.text,
            random_id: q.random_id,
            files: q.files,
            routine: q.routine,
        };
        let item = live.chat.push(now(), 0, body);
        if first {
            live.pending = None;
        }
        live.publish(self, vec![Change::Item(item.clone())])?;
        Ok(item)
    }

    /// Hands `prompt` to `terminal`'s agent on `machine`; refused when the turn `ended` and the agent stopped.
    fn prompt(
        &self,
        terminal: &str,
        machine: &str,
        ended: bool,
        prompt: &str,
    ) -> Result<(), HubError> {
        let runs = |t: TerminalInfo| t.state == shed_proto::State::Running;
        if ended && !self.shed.get(terminal).is_ok_and(runs) {
            return Err(HubError::Refused(
                "the agent stopped, so it can't take the message".to_owned(),
            ));
        }
        if let Err(e) = self.shed.prompt(terminal, prompt) {
            if e.no_link() {
                self.reachable(machine)?;
            }
            return Err(e.into());
        }
        Ok(())
    }

    /// Queues the owner's message for when the turn ends (between turns it goes at once); once per `random_id`.
    /// Returns the queue.
    ///
    /// # Errors
    /// As [`Hub::send`].
    pub fn queue(
        &self,
        id: &str,
        text: &str,
        random_id: &str,
        files: &[Upload],
    ) -> Result<Vec<Queued>, HubError> {
        self.queue_as(id, (text, random_id), files, None)
    }

    /// [`Hub::queue`] by `routine` (#75) when given, not the owner.
    ///
    /// # Errors
    /// As [`Hub::send`].
    pub fn queue_as(
        &self,
        id: &str,
        (text, random_id): (&str, &str),
        files: &[Upload],
        routine: Option<Mark>,
    ) -> Result<Vec<Queued>, HubError> {
        let live = self.live(id)?;
        let mut l = lock(&live);
        let have = l.meta.queue.iter().any(|q| q.random_id == random_id);
        if have || Self::sent(&l, random_id).is_some() {
            return Ok(l.meta.queue.clone());
        }
        let q = Queued {
            routine,
            ..self.take(&l, text, random_id, files)?
        };
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
