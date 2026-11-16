use std::path::PathBuf;

use cooee_core::{Kind, Summary, Thread, Turn, personas};
use tokio::sync::broadcast;

use super::{Hub, HubError, LAG, Out, Pending, Resume, Slice, glance, lock, now, rooms, waits};

impl Hub {
    /// Renames the thread.
    ///
    /// # Errors
    /// [`HubError::Refused`] for an empty title.
    pub fn retitle(&self, id: &str, title: &str) -> Result<(), HubError> {
        let title: String = title.trim().chars().take(80).collect();
        if title.is_empty() {
            return Err(HubError::Refused("a title can't be empty".to_owned()));
        }
        let live = self.live(id)?;
        let mut live = lock(&live);
        live.meta.title = title;
        Ok(self.store.save(&live.meta)?)
    }

    /// Archives the thread (`on`) or restores it.
    ///
    /// # Errors
    /// [`HubError::Store`] when it can't be saved.
    pub fn archive(&self, id: &str, on: bool) -> Result<(), HubError> {
        let live = self.live(id)?;
        let mut live = lock(&live);
        live.meta.archived = on;
        Ok(self.store.save(&live.meta)?)
    }

    /// File `name` sent in thread `id`: its path, if it is one of that thread's.
    ///
    /// # Errors
    /// [`HubError::NoThread`] for no such thread or file.
    pub fn file(&self, id: &str, name: &str) -> Result<PathBuf, HubError> {
        self.live(id)?;
        let plain = !name.starts_with('.')
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c));
        let path = self.files.join(id).join(name);
        if plain && path.is_file() {
            Ok(path)
        } else {
            Err(HubError::NoThread(format!("{id}/{name}")))
        }
    }

    /// Cancels the turn in progress; in a room, stop: the member taking its turn is cancelled and the rest dropped.
    ///
    /// # Errors
    /// [`HubError::Shed`] when shed fails.
    pub fn cancel(&self, id: &str) -> Result<(), HubError> {
        let live = self.live(id)?;
        let (kind, terminal) = {
            let l = lock(&live);
            (l.meta.kind.clone(), l.meta.terminal.clone())
        };
        if kind == Kind::Room {
            return self.stop_round(id, "stop", "");
        }
        Ok(self.shed.cancel(&terminal)?)
    }

    /// Closes the thread: kills and removes its terminal. Its history stays.
    ///
    /// # Errors
    /// [`HubError::Shed`] when shed fails.
    pub fn close(&self, id: &str) -> Result<(), HubError> {
        let live = self.live(id)?;
        let terminal = {
            let l = lock(&live);
            if l.meta.workflow.is_some() {
                return Err(HubError::Refused(
                    "the tool that started this session closes it".to_owned(),
                ));
            }
            l.meta.terminal.clone()
        };
        self.shed.close(&terminal)?;
        let mut live = lock(&live);
        live.meta.closed = true;
        self.store.save(&live.meta)?;
        if live.chat.turn.state != "ended" {
            let changes = live
                .chat
                .ended(now(), 0, "The thread was closed.".to_owned());
            live.publish(self, changes)?;
        }
        Ok(())
    }

    /// Marks the thread read up to item `item`.
    ///
    /// # Errors
    /// [`HubError::Store`] when the thread can't be saved.
    pub fn read(&self, id: &str, item: u64) -> Result<(), HubError> {
        let live = self.live(id)?;
        let mut live = lock(&live);
        if live.meta.read.is_some_and(|r| r >= item) {
            return Ok(());
        }
        live.meta.read = Some(item);
        Ok(self.store.save(&live.meta)?)
    }

    /// The thread list, newest first.
    #[must_use]
    pub fn list(&self) -> Vec<Summary> {
        let personas = personas::list(self.config_dir(), &self.data);
        let mut asks = Vec::new();
        let mut rows: Vec<Summary> = self
            .all()
            .iter()
            .filter_map(|live| {
                let live = lock(live);
                let g = live.unloaded.clone().unwrap_or_else(|| glance(&live));
                if !live.meta.room.is_empty() {
                    asks.extend(g.ask);
                    return None;
                }
                let persona = personas.iter().find(|p| p.id == live.meta.profile);
                Some(Summary {
                    id: live.meta.id.clone(),
                    title: live.meta.title.clone(),
                    agent: live.meta.agent.clone(),
                    account: live.meta.account.clone(),
                    profile: live.meta.profile.clone(),
                    persona: persona.map(|p| p.id.clone()).unwrap_or_default(),
                    persona_name: persona.map(|p| p.name.clone()).unwrap_or_default(),
                    persona_role: persona.map(|p| p.role).unwrap_or_default(),
                    folder: live.meta.folder.clone().unwrap_or_default(),
                    machine: live.meta.machine.clone(),
                    state: live.chat.turn.state.clone(),
                    last: rooms::said(&personas, &g.speaker, g.last),
                    at: g.at,
                    unread: g.unseen > 0,
                    unseen: g.unseen,
                    archived: live.meta.archived,
                    setup: live.meta.setup,
                    workflow: live.meta.workflow.clone(),
                    routine: g.routine,
                    members: rooms::seated(&live.meta),
                    asked_by: live.meta.asked_by.clone(),
                })
            })
            .collect();
        for (room, persona, tool) in asks {
            let Some(row) = rows.iter_mut().find(|r| r.id == room) else {
                continue;
            };
            let name = personas.iter().find(|p| p.id == persona);
            let name = name.map_or(persona.as_str(), |p| p.name.as_str());
            row.last = format!("{name}: {tool} waiting");
            row.unseen += 1;
            row.unread = true;
        }
        rows.sort_by_key(|r| std::cmp::Reverse(r.at));
        rows
    }

    /// The thread's meta.
    ///
    /// # Errors
    /// [`HubError::NoThread`].
    pub fn thread(&self, id: &str) -> Result<(Thread, Turn), HubError> {
        let live = self.live(id)?;
        let live = lock(&live);
        Ok((live.meta.clone(), live.chat.turn.clone()))
    }

    /// Up to `before` items before item `around` and `after` after it (with it); without `around`, the newest
    /// `before` items. With the turn in progress and the cursor they are current to.
    ///
    /// # Errors
    /// [`HubError::NoThread`].
    pub fn slice(
        &self,
        id: &str,
        around: Option<u64>,
        before: usize,
        after: usize,
    ) -> Result<Slice, HubError> {
        let live = self.live(id)?;
        let live = lock(&live);
        let items = &live.chat.items;
        let (from, to) = match around.map(|a| items.partition_point(|i| i.id < a)) {
            Some(at) => (at.saturating_sub(before), (at + after + 1).min(items.len())),
            None => (items.len().saturating_sub(before), items.len()),
        };
        Ok(Slice {
            items: items.get(from..to.max(from)).unwrap_or_default().to_vec(),
            skipped_before: from,
            skipped_after: items.len() - to.max(from),
            turn: live.chat.turn.clone(),
            cursor: format!("{}.{}", self.boot, live.seq),
            pending: live.pending.clone().map(|text| Pending {
                text,
                random_id: live.meta.random_id.clone(),
            }),
            queue: live.meta.queue.clone(),
            waiting: items.iter().find(|i| waits(&i.body)).cloned(),
            asks: live.asks.open.clone(),
        })
    }

    /// Subscribes to thread `id`'s page events after `cursor` (`<boot>.<seq>`), in one step under the thread's
    /// lock, so nothing is missed or doubled.
    ///
    /// # Errors
    /// [`HubError::NoThread`].
    pub fn subscribe(
        &self,
        id: &str,
        cursor: &str,
    ) -> Result<(Resume, broadcast::Receiver<Out>), HubError> {
        let live = self.live(id)?;
        let mut live = lock(&live);
        let seq = cursor
            .strip_prefix(&self.boot)
            .and_then(|s| s.strip_prefix('.'))
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|s| *s <= live.seq);
        let first = live.ring.front().map_or(live.seq + 1, |(_, o)| o.seq);
        let resume = match seq {
            Some(s) if s + 1 >= first => Resume::Replay(
                live.ring
                    .iter()
                    .filter(|(_, o)| o.seq > s)
                    .map(|(_, o)| o.clone())
                    .collect(),
            ),
            _ => Resume::Reset,
        };
        let tx = live.tx.get_or_insert_with(|| broadcast::channel(LAG).0);
        Ok((resume, tx.subscribe()))
    }

    /// The SSE id of a page event.
    #[must_use]
    pub fn event_id(&self, out: &Out) -> String {
        format!("{}.{}", self.boot, out.seq)
    }
}
