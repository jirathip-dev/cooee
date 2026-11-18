use super::{Body, Item, Turn};

/// What applying an event changed, for the page.
#[derive(Debug, Clone, PartialEq)]
#[allow(
    clippy::large_enum_variant,
    reason = "a change lives only until it is published; boxing every item for it isn't worth it"
)]
pub enum Change {
    /// An item was added or replaced; it is stored before the page hears of it.
    Item(Item),
    /// A chunk of the reply in progress.
    Text(String),
    /// The state changed.
    State(Turn),
}

/// A thread's items and its turn in progress.
#[derive(Debug, Clone, Default)]
pub struct Chat {
    /// Every committed item, by id.
    pub items: Vec<Item>,
    /// The turn in progress.
    pub turn: Turn,
    pub(super) start: usize,
}

impl Chat {
    /// A chat from its stored item versions (oldest first; a later version of an id replaces the earlier).
    #[must_use]
    pub fn from_versions(versions: Vec<Item>) -> Self {
        let mut chat = Self::default();
        for item in versions {
            chat.put(item);
        }
        chat.start = chat.items.len();
        chat
    }

    /// The newest shed event seq the items reflect.
    #[must_use]
    pub fn seq(&self) -> u64 {
        self.items.iter().map(|i| i.seq).max().unwrap_or(0)
    }

    fn put(&mut self, item: Item) -> Item {
        match self.items.binary_search_by_key(&item.id, |i| i.id) {
            Ok(at) => {
                if let Some(i) = self.items.get_mut(at) {
                    i.clone_from(&item);
                }
            }
            Err(at) => self.items.insert(at, item.clone()),
        }
        item
    }

    /// Adds a new item with the next id.
    pub fn push(&mut self, at: u64, seq: u64, body: Body) -> Item {
        self.said(at, seq, String::new(), body)
    }

    /// [`Chat::push`] by `speaker` (a room's).
    pub fn said(&mut self, at: u64, seq: u64, speaker: String, body: Body) -> Item {
        let id = self.items.last().map_or(1, |i| i.id + 1);
        self.put(Item {
            id,
            at,
            seq,
            speaker,
            body,
        })
    }

    /// Replaces item `id`'s body; `None` when there is no such item.
    pub fn update(&mut self, id: u64, seq: u64, f: impl FnOnce(&mut Body)) -> Option<Item> {
        let at = self.items.binary_search_by_key(&id, |i| i.id).ok()?;
        let item = self.items.get_mut(at)?;
        f(&mut item.body);
        item.seq = item.seq.max(seq);
        Some(item.clone())
    }

    pub(super) fn find(&self, f: impl Fn(&Body) -> bool) -> Option<u64> {
        self.items
            .get(self.start..)
            .unwrap_or_default()
            .iter()
            .rev()
            .find(|i| f(&i.body))
            .map(|i| i.id)
    }

    pub(super) fn state(&mut self, state: &str, note: Option<String>) -> Change {
        self.turn.report(state, note);
        Change::State(self.turn.clone())
    }

    pub(super) fn end(&mut self, at: u64, seq: u64, status: Option<&str>) -> Vec<Change> {
        self.turn.text.clear();
        let open: Vec<u64> = self
            .items
            .get(self.start..)
            .unwrap_or_default()
            .iter()
            .filter(|i| matches!(&i.body, Body::Tool { ended: None, .. }))
            .map(|i| i.id)
            .collect();
        let changes = open
            .into_iter()
            .filter_map(|id| {
                self.update(id, seq, |b| {
                    if let Body::Tool {
                        status: s, ended, ..
                    } = b
                    {
                        if let Some(status) = status {
                            status.clone_into(s);
                        }
                        *ended = Some(at);
                    }
                })
            })
            .map(Change::Item)
            .collect();
        self.start = self.items.len();
        changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn r6_folding_a_long_log_is_within_budget() {
        let versions: Vec<Item> = (0..30)
            .flat_map(|v| {
                (1..=1000).map(move |id| Item {
                    id,
                    at: id,
                    seq: v * 1000 + id,
                    speaker: String::new(),
                    body: Body::Agent {
                        text: format!("reply {id} v{v}"),
                        stop_reason: "end_turn".to_owned(),
                    },
                })
            })
            .collect();
        let at = crate::clock::instant();
        let chat = Chat::from_versions(versions);
        let took = at.elapsed();
        assert_eq!((chat.items.len(), chat.seq()), (1000, 30_000));
        assert!(took < std::time::Duration::from_millis(300), "{took:?}");
    }

    #[test]
    fn a_repeated_id_is_one_row() {
        let you = |text: &str| Item {
            id: 7,
            at: 1,
            seq: 0,
            speaker: String::new(),
            body: Body::You {
                text: text.to_owned(),
                random_id: "r".to_owned(),
                files: Vec::new(),
                routine: None,
            },
        };
        let chat = Chat::from_versions((0..5).map(|i| you(&format!("v{i}"))).collect());
        assert_eq!(chat.items, [you("v4")]);
    }
}
