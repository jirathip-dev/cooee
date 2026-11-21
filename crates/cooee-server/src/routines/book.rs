use std::sync::{Arc, PoisonError};

use cooee_core::{
    Body,
    routines::{Ask, Book, Edit, Mode, Place, Routine, Trigger},
};

use super::Runner;
use crate::{http::refused, hub::HubError};

impl Runner {
    /// Every routine.
    ///
    /// # Errors
    /// [`HubError::Refused`] when the file can't be read.
    pub fn list(&self) -> Result<Vec<Routine>, HubError> {
        self.file.load().map(|b| b.routines).map_err(refused)
    }

    /// Routine `r`'s public URL, when it is a webhook one and `[hooks]` is set.
    #[must_use]
    pub fn url(&self, r: &Routine) -> Option<String> {
        let hooks = self
            .hooks
            .as_ref()
            .filter(|_| matches!(r.trigger, Trigger::Webhook(_)))?;
        Some(hooks.inbound(&r.id))
    }
    pub(super) fn change<T>(
        &self,
        f: impl FnOnce(&mut Book) -> Result<T, HubError>,
    ) -> Result<T, HubError> {
        let _held = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
        let mut book = self.file.load().map_err(refused)?;
        let done = f(&mut book)?;
        self.file.save(&book).map_err(refused)?;
        Ok(done)
    }

    /// `create_routine` by thread `id`'s agent (`token`: its MCP token): a schedule is stored, then its chat gets
    /// the `Created routine` line; a GitHub, webhook or script one only raises a card, and is stored when the owner
    /// approves it with Face ID. A Workflow session or a routine's own run can't make one. What the agent is told.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, such a thread, no `[hooks]` for a GitHub or webhook one, or a routine `ask`
    /// doesn't make.
    pub fn create(
        self: &Arc<Self>,
        (id, token): (&str, &str),
        ask: Ask,
        now: u64,
    ) -> Result<String, HubError> {
        let from = self.hub.token_of(id, token)?;
        if from.workflow.is_some() || from.routine.is_some() {
            return Err(refused(
                "NOT_HERE: a Workflow session or a routine's own run can't make routines",
            ));
        }
        let outside = ask.github.is_some() || ask.webhook.is_some() || ask.mail;
        if outside && self.hooks.is_none() {
            return Err(refused("NO_HOOKS: [hooks] isn't set up on this Mac"));
        }
        let inbox = self.hub.persona(&from.profile).mail;
        if ask.mail && !inbox.is_some_and(|m| m.on && !m.hook.is_empty()) {
            return Err(refused(
                "BAD_TRIGGER: mail arrives only at a persona's catch-all inbox, and it is off",
            ));
        }
        if outside || ask.script.is_some() {
            let list = self.list()?;
            let r = Routine::make(&list, ask.clone(), &from, (now, self.offset(now)))
                .map_err(refused)?;
            let kind = match r.script.as_ref().map(|s| s.mode) {
                Some(Mode::Post) => ", a script that posts its output",
                Some(Mode::Wake) => ", a script that wakes the agent when it fails",
                None => "",
            };
            let tool = format!(
                "save routine {}: {}, {}{kind}",
                r.name,
                self.when(&r, now),
                placed(&r.place)
            );
            let runner = Arc::downgrade(self);
            let save = Box::new(move || {
                let runner = runner.upgrade().ok_or("cooee is stopping")?;
                let now = crate::hub::now();
                let offset = runner.offset(now);
                let saved = runner.change(|b| {
                    let r =
                        Routine::make(&b.routines, ask, &from, (now, offset)).map_err(refused)?;
                    b.routines.push(r.clone());
                    Ok(r)
                });
                saved
                    .map(|r| format!("saved routine {}", r.name))
                    .map_err(|e| e.to_string())
            });
            let shown = r.script.map_or(r.prompt, |s| s.run);
            self.hub.propose_later(id, (tool, shown), save)?;
            return Ok(
                "Proposed. Nothing is saved until the owner approves the card in this chat."
                    .to_owned(),
            );
        }
        let r = self.make(id, ask, &from, now)?;
        Ok(format!(
            "Made routine {}. The owner sees it in this chat and can change it in Settings › routines; stop_routine ends it.",
            r.id
        ))
    }

    pub(crate) fn make(
        &self,
        id: &str,
        ask: Ask,
        from: &cooee_core::Thread,
        now: u64,
    ) -> Result<Routine, HubError> {
        let r = self.change(|b| {
            let r =
                Routine::make(&b.routines, ask, from, (now, self.offset(now))).map_err(refused)?;
            b.routines.push(r.clone());
            Ok(r)
        })?;
        let body = Body::Routine {
            routine: r.id.clone(),
            name: r.name.clone(),
            when: self.when(&r, now),
            place: placed(&r.place).to_owned(),
        };
        self.hub.say(id, body)?;
        Ok(r)
    }

    /// Settings › routines' edit of routine `id`.
    ///
    /// # Errors
    /// [`HubError::Refused`] for no such routine or an edit it refuses.
    pub fn edit(&self, id: &str, e: Edit, now: u64) -> Result<Routine, HubError> {
        self.change(|b| {
            let r = b.routines.iter_mut().find(|r| r.id == id);
            let r = r.ok_or_else(|| refused(format!("no routine {id}")))?;
            let was = r.clone();
            r.edit(e, now).map_err(refused)?;
            let resumed = was.paused.is_some() && r.paused.is_none();
            if resumed || was.prompt != r.prompt || was.trigger != r.trigger {
                self.fresh(r);
            }
            Ok(r.clone())
        })
    }

    /// Deletes routine `id`; the threads it ran in stay.
    ///
    /// # Errors
    /// [`HubError::Refused`] when the file can't be read or written.
    pub fn delete(&self, id: &str) -> Result<(), HubError> {
        self.change(|b| {
            for r in b.routines.iter().filter(|r| r.id == id) {
                self.forget(r);
            }
            b.routines.retain(|r| r.id != id);
            Ok(())
        })
    }
}

const fn placed(p: &Place) -> &'static str {
    match p {
        Place::Chat { .. } => "in this chat",
        Place::NewThread { .. } => "each run a new thread",
    }
}
