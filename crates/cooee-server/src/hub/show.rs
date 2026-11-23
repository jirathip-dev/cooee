use blocks::{Action, catalog::Catalog};
use cooee_core::{
    Change, Kind, Queued, Thread,
    show::{self, Shown, Unsent},
};
use serde_json::{Value, json};

use super::{Hub, HubError, live::Live, lock, now};

const DRAWN: &str = "Shown.";
const PLAIN: &str = "Shown as plain text. Do not send it again.";
const SPENT: &str =
    "NO_RETRY: this turn had its one retry; nothing more is read until the owner's next message.";

fn no(why: &impl ToString) -> HubError {
    HubError::Refused(why.to_string())
}

fn tool() -> Value {
    json!({
        "name": "show",
        "description": "Answer the owner with blocks (a form, a choice, a chart) in this chat, as your instructions describe them. A refusal lists what to fix: send the whole reply once more. A refusal starts with NO_CATALOG, NOT_HERE or NO_RETRY when there is nothing to fix.",
        "inputSchema": { "type": "object", "required": ["blocks"],
                         "properties": { "blocks": { "type": "array", "items": { "type": "object" } } } }
    })
}

impl Hub {
    fn catalog_of(&self, meta: &Thread) -> Result<&'static Catalog, HubError> {
        if meta.kind == Kind::Room || !meta.room.is_empty() {
            return Err(no(
                &"NOT_HERE: blocks are drawn only in a persona's own chat",
            ));
        }
        let persona = self.persona(&meta.profile);
        let catalog = persona.blocks.as_deref().and_then(show::catalog);
        catalog.ok_or_else(|| no(&"NO_CATALOG: this persona answers in text only"))
    }

    pub(crate) fn show_tools(&self, (id, token): (&str, &str)) -> Vec<Value> {
        let meta = self.token_of(id, token).ok();
        let listed = meta.is_some_and(|m| self.catalog_of(&m).is_ok());
        listed.then(tool).into_iter().collect()
    }

    /// Thread `id`'s persona answers with `reply`: drawn, refused once with what to fix, then plain; then not read.
    #[expect(clippy::missing_errors_doc, reason = "a refusal is the tool's answer")]
    pub fn show(&self, (id, token): (&str, &str), reply: &Value) -> Result<String, HubError> {
        let meta = self.token_of(id, token)?;
        let catalog = self.catalog_of(&meta)?;
        let live = self.live(id)?;
        let mut l = lock(&live);
        let (shown, changes) = l.chat.show(catalog, &reply.to_string(), now());
        l.publish(self, changes)?;
        match shown {
            Shown::Drawn => Ok(DRAWN.to_owned()),
            Shown::Again(say) => Err(HubError::Refused(say)),
            Shown::Plain => Ok(PLAIN.to_owned()),
            Shown::Spent => Err(no(&SPENT)),
        }
    }

    fn unanswer(&self, live: &mut Live, item: u64) -> Result<(), HubError> {
        let cleared = live.chat.answer(item, None).map(Change::Item);
        Ok(live.publish(self, cleared.into_iter().collect())?)
    }

    pub(super) fn unacted(&self, live: &mut Live, q: &Queued) -> Result<(), HubError> {
        q.acted.map_or(Ok(()), |item| self.unanswer(live, item))
    }

    /// The owner's answer `a` by device `by` to reply `item`, checked, then the persona's next turn through the queue.
    #[expect(clippy::missing_errors_doc, reason = "a refusal is the toast")]
    pub fn act(&self, id: &str, item: u64, a: &Action, by: &str) -> Result<(), HubError> {
        let live = self.live(id)?;
        let told = {
            let mut l = lock(&live);
            let catalog = self
                .catalog_of(&l.meta)
                .map_err(|_| no(&Unsent::NotListed))?;
            let (told, changes) = l
                .chat
                .act(catalog, item, a, (by, now()))
                .map_err(|e| no(&e))?;
            l.publish(self, changes)?;
            told
        };
        let Some(t) = told else {
            return Ok(());
        };
        let q = |l: &Live| {
            Ok(Queued {
                acted: Some(item),
                ..self.take(l, &t.text, &t.random_id, &[])?
            })
        };
        let queued = self.enqueue(id, &t.random_id, q);
        if queued.is_err() {
            self.unanswer(&mut lock(&live), item)?;
        }
        queued.map(drop)
    }
}
