use blocks::{Action, catalog::Catalog, model::Refusal};

use super::{Body, Change, Chat, Item};
use crate::show::{self, Checked, Reply, Shown, Told, Unsent};

/// How far a turn's `show` calls got: the state of its one retry.
#[derive(Debug, Clone, Default)]
pub(super) enum Tries {
    #[default]
    Fresh,
    Asked(Box<Refusal>),
    Spent,
}

impl Chat {
    fn keep(&mut self, blocks: Checked, at: u64) -> Vec<Change> {
        let body = Body::Blocks(Reply { blocks, sent: None });
        vec![Change::Item(self.push(at, 0, body))]
    }

    pub(super) fn unasked(&mut self, at: u64) -> Option<Change> {
        let Tries::Asked(no) = std::mem::take(&mut self.tries) else {
            return None;
        };
        let next = self.items.last().map_or(1, |i| i.id + 1);
        self.keep(show::plain(&no, next), at).pop()
    }

    /// A persona's `show` call with reply `text` under `catalog`: drawn, refused once with what to fix, then plain.
    pub fn show(&mut self, catalog: &Catalog, text: &str, at: u64) -> (Shown, Vec<Change>) {
        if matches!(self.tries, Tries::Spent) {
            return (Shown::Spent, Vec::new());
        }
        let next = self.items.last().map_or(1, |i| i.id + 1);
        match (
            std::mem::take(&mut self.tries),
            show::read(catalog, next, text),
        ) {
            (was, Ok(blocks)) => {
                if matches!(was, Tries::Asked(_)) {
                    self.tries = Tries::Spent;
                }
                (Shown::Drawn, self.keep(blocks, at))
            }
            (Tries::Asked(_), Err(no)) => {
                self.tries = Tries::Spent;
                (Shown::Plain, self.keep(show::plain(&no, next), at))
            }
            (_, Err(no)) => {
                let say = no.say();
                self.tries = Tries::Asked(Box::new(no));
                (Shown::Again(say), Vec::new())
            }
        }
    }

    /// The owner's answer `a` by `by` to reply `item` under `catalog`: its line set and the persona's next turn, or
    /// nothing for the same answer again.
    ///
    /// # Errors
    /// What [`show::told`] refuses; [`Unsent::Gone`] when `item` is no reply of blocks.
    pub fn act(
        &mut self,
        catalog: &Catalog,
        item: u64,
        a: &Action,
        (by, at): (&str, u64),
    ) -> Result<(Option<Told>, Vec<Change>), Unsent> {
        let found = self.items.iter().find(|i| i.id == item);
        let Some(Body::Blocks(Reply { blocks, sent })) = found.map(|i| &i.body) else {
            return Err(Unsent::Gone);
        };
        let Some((s, told)) = show::told(catalog, (item, blocks, sent.as_ref()), a, (by, at))?
        else {
            return Ok((None, Vec::new()));
        };
        let changed = self.answer(item, Some(s));
        Ok((Some(told), changed.map(Change::Item).into_iter().collect()))
    }

    /// Sets or clears reply `item`'s answer.
    pub fn answer(&mut self, item: u64, to: Option<show::Sent>) -> Option<Item> {
        self.update(item, 0, |b| {
            if let Body::Blocks(r) = b {
                r.sent = to;
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BAD: &str = r#"{"blocks": [{"type": "button", "label": "Go", "action": "pick"}]}"#;
    const GOOD: &str = r#"{"blocks": [{"type": "prose", "markdown": "Hello"}]}"#;

    fn kinds(chat: &Chat) -> Vec<String> {
        let blocks = chat.items.iter().filter_map(|i| match &i.body {
            Body::Blocks(r) => Some(serde_json::to_value(&r.blocks).unwrap()),
            _ => None,
        });
        let all: Vec<serde_json::Value> =
            blocks.flat_map(|b| b.as_array().unwrap().clone()).collect();
        all.iter()
            .map(|b| b["type"].as_str().unwrap().to_owned())
            .collect()
    }

    #[test]
    fn a_reply_refused_twice_is_plain_after_one_retry_and_a_third_is_not_read() {
        let mut chat = Chat::default();
        let catalog = show::catalog("show").unwrap();
        let outcomes = [BAD, BAD, GOOD].map(|text| chat.show(catalog, text, 1).0);
        assert!(
            matches!(outcomes, [Shown::Again(_), Shown::Plain, Shown::Spent]),
            "{outcomes:?}"
        );
        assert_eq!(kinds(&chat), ["prose"]);
    }

    #[test]
    fn a_turn_that_ends_after_one_refusal_keeps_its_plain_text_and_the_next_turn_starts_fresh() {
        let mut chat = Chat::default();
        let catalog = show::catalog("show").unwrap();
        assert!(matches!(chat.show(catalog, BAD, 1).0, Shown::Again(_)));
        chat.end(2, 0, None);
        assert_eq!(kinds(&chat), ["prose"]);
        assert_eq!(chat.show(catalog, GOOD, 3).0, Shown::Drawn);
        assert_eq!(kinds(&chat), ["prose", "prose"]);
    }
}
