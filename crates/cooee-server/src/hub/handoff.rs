use cooee_core::{Body, Change, Item};

use super::{Hub, HubError, lock, now};

const ENDED: &str =
    "Handed off: end your turn now. When the owner hands back, their note is your next message.";

impl Hub {
    /// Thread `id`'s agent hands `what` (a log-in, a 2FA code, a CLI login) to the owner (#164, W5): a card in its
    /// chat, and until [`Hub::hand_back`] nothing the owner sends reaches it. Once per open handoff.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread or nothing to do.
    pub fn handoff(&self, (id, token): (&str, &str), what: &str) -> Result<String, HubError> {
        self.token_of(id, token)?;
        let text: String = what.trim().chars().take(300).collect();
        if text.is_empty() {
            return Err(HubError::Refused("say what the owner is to do".to_owned()));
        }
        let live = self.live(id)?;
        let mut l = lock(&live);
        if l.meta.handoff == 0 {
            let body = Body::Handoff {
                text,
                outcome: "waiting".to_owned(),
                note: String::new(),
            };
            let item = l.chat.push(now(), 0, body);
            l.meta.handoff = item.id;
            self.store.save(&l.meta)?;
            l.publish(self, vec![Change::Item(item)])?;
        }
        Ok(ENDED.to_owned())
    }

    /// The owner hands thread `id` back with `note` (#164): the card closes and the agent resumes with the note;
    /// once per `random_id`.
    ///
    /// # Errors
    /// [`HubError::Refused`] with `NO_HANDOFF` when none is open; as [`Hub::send`].
    pub fn hand_back(&self, id: &str, note: &str, random_id: &str) -> Result<Item, HubError> {
        let live = self.live(id)?;
        let mut l = lock(&live);
        if let Some(item) = Self::sent(&l, random_id) {
            return Ok(item);
        }
        let card = std::mem::take(&mut l.meta.handoff);
        let open = l.chat.items.iter_mut().find(|i| i.id == card);
        let Some(item) = open else {
            return Err(HubError::Refused(
                "NO_HANDOFF: the agent isn't waiting for you".to_owned(),
            ));
        };
        let mut before = String::new();
        if let Body::Handoff {
            text,
            outcome,
            note: n,
        } = &mut item.body
        {
            "handed_back".clone_into(outcome);
            note.trim().clone_into(n);
            before = format!("[The owner handed back: {text}; their note]:");
        }
        let item = item.clone();
        self.store.save(&l.meta)?;
        l.publish(self, vec![Change::Item(item)])?;
        let text = if note.trim().is_empty() {
            "Done."
        } else {
            note
        };
        let q = self.take(&l, text, random_id, &[])?;
        drop(l);
        self.deliver(&live, q, &before)
    }
}
