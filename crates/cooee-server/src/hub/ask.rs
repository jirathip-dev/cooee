use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

use cooee_core::{Body, Change, Item, personas};

use super::{Hub, HubError, lock, now};

const DEPTH: usize = 2;

pub(super) const OWNER_NOTE: &str = "[The owner's note; the asker gets it with your reply]:";

fn no<T>(code: &str, why: &str) -> Result<T, HubError> {
    Err(HubError::Refused(format!("{code}: {why}")))
}

impl Hub {
    /// Thread `id`'s persona asks persona `target` `message` and waits up to `wait` for its reply, in the exchange
    /// thread linked to `id` (started on the first ask, reused after). The reply carries the owner's notes.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token or a closed thread, and with a code: `NOT_ALLOWED` when the persona's
    /// `can_message` doesn't name `target`, `DEPTH` past G4's depth, `OFFLINE` when `target`'s session can't take
    /// it, `TIMEOUT` when no reply came in time.
    pub fn ask_persona(
        &self,
        (id, token): (&str, &str),
        target: &str,
        message: &str,
        wait: Duration,
    ) -> Result<String, HubError> {
        let meta = self.token_of(id, token)?;
        let found = |p: &str| personas::find(self.config_dir(), &self.data, p).ok();
        let name = found(target).map_or_else(|| target.to_owned(), |p| p.name);
        let card = |thread: &str, outcome: &str| Body::Ask {
            persona: target.to_owned(),
            name: name.clone(),
            text: message.to_owned(),
            thread: thread.to_owned(),
            outcome: outcome.to_owned(),
            secs: 0,
            note: String::new(),
        };
        let asker = found(&meta.profile).filter(|p| p.can_message.iter().any(|t| t == target));
        let Some(asker) = asker else {
            self.say(id, card("", "not_allowed"))?;
            return no("NOT_ALLOWED", &format!("can_message doesn't name {target}"));
        };
        let (mut depth, mut up) = (0, meta.asked_by);
        while !up.is_empty() {
            depth += 1;
            up = self.thread(&up).map(|t| t.0.asked_by).unwrap_or_default();
        }
        if depth >= DEPTH {
            self.say(id, card("", "depth"))?;
            return no("DEPTH", "an asked persona may ask one more, no further");
        }
        let key = format!("{id}>{target}");
        let (tx, rx) = mpsc::channel();
        lock(&self.asks).insert(key.clone(), tx);
        let open = self.all().into_iter().find_map(|l| {
            let m = &lock(&l).meta;
            (m.asked_by == id && m.profile == target && !m.closed).then(|| m.id.clone())
        });
        let (rid, by) = (format!("ask-{key}-{}", now()), format!("[{}]:", asker.name));
        let start = Instant::now();
        let sent = match open {
            Some(x) => self.send_after(&x, message, &rid, &[], &by).map(|_| x),
            None => self.seat(target, (message, &rid), |r| {
                id.clone_into(&mut r.asked_by);
                format!("{}\n\n{by}", r.instructions.trim_end())
                    .trim_start()
                    .clone_into(&mut r.instructions);
            }),
        };
        let x = match sent {
            Ok(x) => x,
            Err(e) => {
                lock(&self.asks).remove(&key);
                self.say(id, card("", "offline"))?;
                return no("OFFLINE", &format!("{target} can't take it: {e}"));
            }
        };
        let question = self.revise(
            &x,
            |i| sent_as(i, &rid),
            |i| meta.profile.clone_into(&mut i.speaker),
        )?;
        let ask = self.say(id, card(&x, "asking"))?.id;
        let got = rx.recv_timeout(wait);
        lock(&self.asks).remove(&key);
        let secs = start.elapsed().as_secs();
        let Ok(reply) = got else {
            self.answered(id, ask, "timeout", secs, "")?;
            return no("TIMEOUT", &format!("no reply from {target} in time"));
        };
        let notes = self.notes(&x, question)?;
        self.answered(id, ask, "replied", secs, &notes)?;
        let with = if notes.is_empty() {
            ""
        } else {
            " with your note"
        };
        let back = format!("returned to {}{with} · {secs}s", asker.name);
        self.say(&x, Body::Note { text: back })?;
        Ok(if notes.is_empty() {
            reply
        } else {
            format!("{reply}\n\n[The owner's note to {name}: {notes}]")
        })
    }

    fn answered(
        &self,
        id: &str,
        item: u64,
        outcome: &str,
        secs: u64,
        note: &str,
    ) -> Result<u64, HubError> {
        self.revise(
            id,
            |i| i.id == item,
            |i| {
                if let Body::Ask {
                    outcome: o,
                    secs: s,
                    note: n,
                    ..
                } = &mut i.body
                {
                    outcome.clone_into(o);
                    *s = secs;
                    note.clone_into(n);
                }
            },
        )
    }

    fn notes(&self, x: &str, question: u64) -> Result<String, HubError> {
        let live = self.live(x)?;
        let live = lock(&live);
        let owner = live
            .chat
            .items
            .iter()
            .filter(|i| i.id > question && i.speaker.is_empty());
        let notes: Vec<&str> = owner
            .filter_map(|i| match &i.body {
                Body::You { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect();
        Ok(notes.join("\n"))
    }

    fn revise(
        &self,
        id: &str,
        pick: impl Fn(&Item) -> bool,
        f: impl FnOnce(&mut Item),
    ) -> Result<u64, HubError> {
        let live = self.live(id)?;
        let mut live = lock(&live);
        let Some(item) = live.chat.items.iter_mut().rev().find(|i| pick(i)) else {
            return Ok(0);
        };
        f(item);
        let item = item.clone();
        live.publish(self, vec![Change::Item(item.clone())])?;
        Ok(item.id)
    }
}

fn sent_as(item: &Item, random_id: &str) -> bool {
    matches!(&item.body, Body::You { random_id: r, .. } if r == random_id)
}
