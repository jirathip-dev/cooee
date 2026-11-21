use std::{path::PathBuf, sync::atomic::Ordering};

use cooee_core::{Item, Kind, Queued, Thread, Turn};
use serde::Serialize;
use tokio::sync::broadcast;

use super::{Hub, HubError, LAG, Out, Resume, live::REV, lock, now, rooms, waits};

/// A slice of a thread's items, with the turn in progress and the page event it is current to.
#[derive(Debug, Clone, Serialize)]
pub struct Slice {
    /// The items, oldest first.
    pub items: Vec<Item>,
    /// How many older items there are.
    pub skipped_before: usize,
    /// How many newer items there are.
    pub skipped_after: usize,
    /// The turn in progress.
    pub turn: Turn,
    /// Resume the event stream after this (`<boot>.<rev>`).
    pub cursor: String,
    /// The first message while its thread connects, not yet an item.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pending: Option<Pending>,
    /// The messages queued for the turns after this one.
    pub queue: Vec<Queued>,
    /// The oldest config card that waits (#553 row 32), wherever it is.
    pub waiting: Option<Item>,
    /// A room's members' open asks (#178).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub asks: Vec<rooms::RoomAsk>,
}

/// A thread's first message while it connects: its text and the start's id (the item it becomes has it).
#[derive(Debug, Clone, Serialize)]
pub struct Pending {
    /// The message.
    pub text: String,
    /// The start's `random_id`.
    pub random_id: String,
}

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
        Ok(self.save(&live)?)
    }

    /// Archives the thread (`on`) or restores it.
    ///
    /// # Errors
    /// [`HubError::Store`] when it can't be saved.
    pub fn archive(&self, id: &str, on: bool) -> Result<(), HubError> {
        let live = self.live(id)?;
        let mut live = lock(&live);
        live.meta.archived = on;
        Ok(self.save(&live)?)
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

    /// Closes the thread: kills and removes its terminal, once. Its history stays; a Workflow session's is archived.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a live Workflow session a plugin claims; [`HubError::Shed`] when shed fails.
    pub fn close(&self, id: &str) -> Result<(), HubError> {
        let live = self.live(id)?;
        let terminal = {
            let l = lock(&live);
            if l.meta.closed {
                return Ok(());
            }
            if !self.closable(&l) {
                return Err(HubError::Refused(
                    "the tool that started this session closes it".to_owned(),
                ));
            }
            l.meta.terminal.clone()
        };
        self.shed.close(&terminal)?;
        let mut live = lock(&live);
        live.meta.closed = true;
        live.meta.archived |= live.meta.workflow.is_some();
        self.save(&live)?;
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
        Ok(self.save(&live)?)
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
            cursor: format!("{}.{}", self.boot, REV.load(Ordering::Relaxed)),
            pending: live.pending.clone().map(|text| Pending {
                text,
                random_id: live.meta.random_id.clone(),
            }),
            queue: live.meta.queue.clone(),
            waiting: items.iter().find(|i| waits(&i.body)).cloned(),
            asks: live.asks.open.clone(),
        })
    }

    /// Subscribes to thread `id`'s page events after `cursor` (`<boot>.<rev>`), in one step under the thread's
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
        let rev = cursor
            .strip_prefix(&self.boot)
            .and_then(|s| s.strip_prefix('.'))
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|s| *s <= REV.load(Ordering::Relaxed));
        let first = live.trimmed + 1;
        let resume = match rev {
            Some(s) if s + 1 >= first => Resume::Replay(
                live.ring
                    .iter()
                    .filter(|(_, o)| o.rev > s)
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
        format!("{}.{}", self.boot, out.rev)
    }
}
