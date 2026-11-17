use std::{io, sync::Mutex};

use cooee_core::{Body, Card, Change, Item, Plugin, agents, shed_proto::ApprovalOption};

use super::{CHANGE, CardSpec, Hub, HubError, Live, lock, now};

impl Hub {
    /// `by`'s answer to approval `item`: sent once (stored first) to shed, or to the plugin whose card it is (then
    /// chosen, with the plugin's line; a 409 leaves it closed with the plugin's why). A secret card's `value` goes
    /// to its plugin only and is never kept; a run card's `approve` sends its sha as the `value`. A second tap
    /// returns the item.
    ///
    /// # Errors
    /// [`HubError::Shed`] when shed fails; [`HubError::Refused`] when the plugin does.
    pub fn approve(
        &self,
        id: &str,
        item: u64,
        option: &str,
        value: Option<&str>,
        by: &str,
    ) -> Result<Option<Item>, HubError> {
        let live = self.live(id)?;
        let (terminal, tapped) = {
            let mut l = lock(&live);
            let Some(tapped) = l.chat.tap(item, option, by, now()) else {
                return Ok(l.chat.items.iter().find(|i| i.id == item).cloned());
            };
            l.publish(self, vec![Change::Item(tapped.clone())])?;
            (l.meta.terminal.clone(), tapped)
        };
        let sent = match &tapped.body {
            Body::Approval {
                plugin: Some(name),
                card,
                ..
            } => self
                .plugins()
                .into_iter()
                .find(|p| &p.name == name)
                .ok_or_else(|| io::Error::other("no longer configured"))
                .and_then(|p| {
                    let value = match card.form.as_deref() {
                        Some("run-card") => card.sha.as_deref().filter(|_| option == "approve"),
                        _ => value,
                    };
                    crate::plugin::answer(&p, id, item, option, by, value)
                })
                .map_err(|e| HubError::Refused(format!("plugin {name}: {e}"))),
            Body::Approval { card, .. }
                if matches!(
                    card.form.as_deref(),
                    Some("mcp" | "skill" | "memory" | "routine")
                ) =>
            {
                Ok(self.proposal_answer(id, item, option))
            }
            Body::Approval { card, .. } if card.form.as_deref() == Some("persona") => {
                Ok(self.persona_answer(id, item, option))
            }
            _ => return self.shed_approve(&live, &terminal, tapped, option),
        };
        let mut live = lock(&live);
        let done = live.chat.update(item, 0, |b| {
            if let Body::Approval {
                tapped,
                chosen,
                card,
                ..
            } = b
            {
                match &sent {
                    Ok(Ok(said)) => {
                        *chosen = Some(option.to_owned());
                        card.line.clone_from(said);
                    }
                    Ok(Err(why)) => card.line = Some(why.clone()),
                    Err(_) => *tapped = None,
                }
            }
        });
        live.publish(self, done.clone().map(Change::Item).into_iter().collect())?;
        sent.map(|_| done)
    }

    fn shed_approve(
        &self,
        live: &Mutex<Live>,
        terminal: &str,
        tapped: Item,
        option: &str,
    ) -> Result<Option<Item>, HubError> {
        if let Err(e) = self.shed.approve(terminal, option) {
            let mut live = lock(live);
            let untapped = live.chat.update(tapped.id, 0, |b| {
                if let Body::Approval { tapped, .. } = b {
                    *tapped = None;
                }
            });
            live.publish(self, untapped.map(Change::Item).into_iter().collect())?;
            return Err(e.into());
        }
        Ok(Some(tapped))
    }

    /// Plugin `plugin`'s card (#553 rows 30–33, 35): choices (the recommended one first), a config diff to apply
    /// or discard, a secret to save or dismiss, a herdsman run card's command to approve or deny (#83), or the
    /// other open threads to pick from. The owner's tap goes to the
    /// plugin; a card the conductor answered itself may be changed for 30 minutes.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a thread of no persona the plugin is attached to (#129), or without text, options, a
    /// config's diff, a secret's name or a run's command and sha.
    pub fn card(&self, plugin: &Plugin, c: CardSpec) -> Result<Item, HubError> {
        let (thread, owner) = (self.thread(&c.thread)?.0, self.plugin_of(&plugin.name));
        let personas = agents::list(self.config_dir(), &self.data);
        let persona = personas.iter().find(|p| p.id == thread.profile);
        if !persona.is_some_and(|p| p.plugins.contains(&owner)) {
            return Err(HubError::Refused(format!(
                "plugin {owner} is not attached to thread {}'s persona",
                c.thread
            )));
        }
        let plain = |o: &str| ApprovalOption {
            id: o.to_owned(),
            name: o.to_owned(),
            kind: String::new(),
        };
        let mut options: Vec<ApprovalOption> = match c.form.as_deref() {
            None => c.options.iter().map(|o| plain(o)).collect(),
            Some("config") => vec![plain("apply"), plain("discard")],
            Some("secret") => vec![plain("save"), plain("dismiss")],
            Some("run-card") => vec![plain("approve"), plain("deny")],
            Some("thread") => self
                .list()
                .into_iter()
                .filter(|t| t.id != c.thread && t.state != "ended")
                .take(6)
                .map(|t| ApprovalOption {
                    id: t.id,
                    name: t.title,
                    kind: String::new(),
                })
                .collect(),
            Some(f) => return Err(HubError::Refused(format!("no card form {f:?}"))),
        };
        let set = |v: &Option<String>| v.as_deref().is_some_and(|d| !d.trim().is_empty());
        if c.text.trim().is_empty()
            || options.is_empty()
            || (matches!(c.form.as_deref(), Some("config" | "secret" | "run-card"))
                && !set(&c.detail))
            || (c.form.as_deref() == Some("run-card") && !set(&c.sha))
        {
            return Err(HubError::Refused(
                "a card needs text and options (a thread to pick), a config card its diff, a secret card its name \
                 and a run card its command and sha"
                    .to_owned(),
            ));
        }
        if let Some(at) = options
            .iter()
            .position(|o| c.recommend.as_ref() == Some(&o.id))
        {
            let mut first = options.remove(at);
            "recommended".clone_into(&mut first.kind);
            options.insert(0, first);
        }
        let live = self.live(&c.thread)?;
        let mut live = lock(&live);
        let at = now();
        let answered = c.answered.is_some();
        let body = Body::Approval {
            tool: c.text,
            options,
            tapped: None,
            chosen: c.answered,
            plugin: Some(plugin.name.clone()),
            card: Card {
                by: answered.then(|| c.by.unwrap_or_else(|| plugin.label.clone())),
                answered: answered.then_some(at),
                form: c.form,
                detail: c.detail,
                sha: c.sha,
                line: None,
                until: answered.then_some(at + CHANGE),
            },
        };
        let item = live.chat.push(at, 0, body);
        live.publish(self, vec![Change::Item(item.clone())])?;
        Ok(item)
    }

    /// Adds `body` to thread `id` as cooee's own line (#75: a made routine's).
    ///
    /// # Errors
    /// [`HubError::NoThread`]; [`HubError::Store`] when it can't be stored.
    pub fn say(&self, id: &str, body: Body) -> Result<Item, HubError> {
        let live = self.live(id)?;
        let mut live = lock(&live);
        let item = live.chat.push(now(), 0, body);
        live.publish(self, vec![Change::Item(item.clone())])?;
        Ok(item)
    }

    /// A line in thread `id` linked to thread `other` (#553 row 35): a handoff's `continued in` (`out`), or a
    /// line `from` it.
    ///
    /// # Errors
    /// [`HubError::NoThread`] when either thread is unknown.
    pub fn link(&self, id: &str, other: &str, text: &str, out: bool) -> Result<Item, HubError> {
        let title = self.thread(other)?.0.title;
        let live = self.live(id)?;
        let mut live = lock(&live);
        let body = Body::Link {
            text: text.to_owned(),
            thread: other.to_owned(),
            title,
            out,
        };
        let item = live.chat.push(now(), 0, body);
        live.publish(self, vec![Change::Item(item.clone())])?;
        Ok(item)
    }

    /// The owner's model and effort for thread `id` (#553 row 34): kept on the thread and said in it; they apply
    /// from the agent's next start.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a model its agent doesn't offer (#193); [`HubError::Store`] when the thread can't
    /// be saved.
    pub fn pick(&self, id: &str, model: &str, effort: &str) -> Result<Item, HubError> {
        let live = self.live(id)?;
        let mut live = lock(&live);
        self.offers(&live.meta.agent, &live.meta.account, model)?;
        model.clone_into(&mut live.meta.model);
        effort.clone_into(&mut live.meta.effort);
        self.store.save(&live.meta)?;
        let say = |what: &str, pick: &str| {
            format!("{what} {}", if pick.is_empty() { "default" } else { pick })
        };
        let text = format!(
            "{} · {} from the next start",
            say("model", model),
            say("effort", effort)
        );
        let item = live.chat.push(now(), 0, Body::Note { text });
        live.publish(self, vec![Change::Item(item.clone())])?;
        Ok(item)
    }
}
