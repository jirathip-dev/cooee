use std::{
    sync::{Arc, Mutex, OnceLock, Weak},
    thread,
    time::Duration,
};

use cooee_core::{
    Body, Item, Via,
    agents::{
        self, Agent,
        route::{Mode, Routable, Table, lead},
    },
    chief,
    plugins::rows::Attach,
    profiles,
};

use super::{Hub, HubError, Me, Upload, ask::sent_as, chief::own_chat, live::Live, lock, now};

const WAIT: Duration = Duration::from_secs(300);

const CHIEF_SAID: &str = "[Chief, for the owner]:";

fn refused(why: impl Into<String>) -> HubError {
    HubError::Refused(why.into())
}

#[derive(Default)]
pub(super) struct State {
    pub(super) me: OnceLock<Weak<Hub>>,
}

/// Where a route went: a delegate's answer by name for Chief, or a hand-off agent's name and nothing of its words.
pub enum Routed {
    Answered { by: String, text: String },
    HandedOff { to: String },
}

impl Hub {
    pub(super) fn keep_me(self: &Arc<Self>) {
        self.areas.route.me.set(Arc::downgrade(self)).ok();
    }

    fn agents_now(&self) -> Vec<Agent> {
        agents::all(self.config_dir(), &self.data, |n| self.served(n))
    }

    pub(crate) fn table(&self) -> Table {
        Table::of(&self.agents_now())
    }

    pub(super) fn agent_of(&self, id: &str) -> Option<Agent> {
        let all = self.agents_now();
        let a = all.iter().find(|a| a.id == id)?;
        profiles::onto(a, &all).ok()
    }

    pub(crate) fn places(&self, id: &str) -> Result<Vec<agents::Place>, HubError> {
        let all = self.agents_now();
        let handed = all.iter().filter_map(|a| profiles::onto(a, &all).ok());
        let handed = handed.filter(|a| a.mode() == Mode::HandOff);
        let plugins = handed.flat_map(|a| a.lent.map(|l| l.plugin).into_iter().chain(a.plugins));
        let mut cards: Vec<String> = if id == chief::ID {
            plugins.collect()
        } else {
            Vec::new()
        };
        cards.sort();
        cards.dedup();
        agents::places(self.config_dir(), &self.data, id, &cards)
            .map_err(|e| refused(e.to_string()))
    }

    pub(crate) fn route(
        &self,
        me: &Me,
        agent: &str,
        brief: &str,
        wait: Duration,
    ) -> Result<Routed, HubError> {
        let table = self.table();
        let to = table.pick(agent).map_err(|e| refused(e.to_string()))?;
        let from = me.thread();
        if self.handed_this_turn(from)? {
            return Err(refused(
                "HANDED_OFF: this turn handed the owner to another agent; end it",
            ));
        }
        match to.mode() {
            Mode::Delegate => {
                let chief = self.persona(chief::ID).name;
                let asker = (chief::ID, chief.as_str());
                let text = self.exchange(from, asker, (to.id(), to.name()), brief, wait)?;
                let by = to.name().to_owned();
                Ok(Routed::Answered { by, text })
            }
            Mode::HandOff => {
                self.hand_off(from, to, brief, true)?;
                let to = to.name().to_owned();
                Ok(Routed::HandedOff { to })
            }
        }
    }

    fn handed_this_turn(&self, from: &str) -> Result<bool, HubError> {
        let live = self.live(from)?;
        let l = lock(&live);
        let turn = l.chat.items.iter().rev();
        let mut turn = turn.take_while(|i| !matches!(i.body, Body::You { .. }));
        Ok(turn.any(|i| matches!(i.body, Body::Link { out: true, .. })))
    }

    fn hand_off(
        &self,
        from: &str,
        to: &Routable,
        text: &str,
        by_chief: bool,
    ) -> Result<String, HubError> {
        let open = self.all().into_iter().find_map(|l| {
            let m = &lock(&l).meta;
            (m.handed_by == from && m.profile == to.id() && !m.closed).then(|| m.id.clone())
        });
        let rid = format!("hand-{from}-{}", now());
        let before = if by_chief { CHIEF_SAID } else { "" };
        let offline = |e: HubError| refused(format!("OFFLINE: {} can't start: {e}", to.name()));
        let x = match open {
            Some(x) if text.is_empty() => x,
            Some(x) => self
                .send_after(&x, text, &rid, &[], before, None)
                .map(|_| x)?,
            None if text.is_empty() => {
                return Err(refused(format!("say what to hand to {}", to.name())));
            }
            None => self
                .seat(to.id(), (text, &rid), |r| {
                    from.clone_into(&mut r.handed_by);
                    r.attach = self.attaches(to.id()).map(|_| Attach {
                        repo: "all".to_owned(),
                        adopt: String::new(),
                    });
                    format!("{}\n\n{before}", r.instructions.trim_end())
                        .trim()
                        .clone_into(&mut r.instructions);
                })
                .map_err(offline)?,
        };
        if by_chief {
            self.revise(
                &x,
                |i| sent_as(i, &rid),
                |i| {
                    chief::ID.clone_into(&mut i.speaker);
                },
            )?;
        }
        self.link(&x, from, "from Chief", false)?;
        self.link(from, &x, &format!("continued with {}", to.name()), true)?;
        Ok(x)
    }

    pub(super) fn mentioned(
        &self,
        live: &Mutex<Live>,
        text: &str,
        random_id: &str,
        files: &[Upload],
    ) -> Result<Option<Item>, HubError> {
        let Some((word, rest)) = lead(text) else {
            return Ok(None);
        };
        let l = lock(live);
        if !own_chat(&l.meta) || !self.is_chief(&l.meta) {
            return Ok(None);
        }
        if let Some(item) = Self::sent(&l, random_id) {
            return Ok(Some(item));
        }
        let table = self.table();
        let to = table.named(word).map_err(|e| refused(e.to_string()))?;
        if to.mode() == Mode::Delegate && rest.is_empty() {
            return Err(refused(format!("say what to ask {}", to.name())));
        }
        let q = self.take(&l, text, random_id, files)?;
        let from = l.meta.id.clone();
        drop(l);
        let you = Body::You {
            text: q.text,
            random_id: q.random_id,
            files: q.files,
            routine: None,
            via: None::<Via>,
        };
        let item = self.say(&from, you)?;
        match to.mode() {
            Mode::HandOff => {
                self.hand_off(&from, to, rest, false)?;
            }
            Mode::Delegate => {
                let hub = self.areas.route.me.get().and_then(Weak::upgrade);
                let hub = hub.ok_or_else(|| refused("the hub is stopping"))?;
                let (to, rest) = (to.clone(), rest.to_owned());
                thread::spawn(move || hub.answer_mention(&from, &to, &rest));
            }
        }
        Ok(Some(item))
    }

    fn answer_mention(&self, from: &str, to: &Routable, text: &str) {
        let asked = self.exchange(from, ("", "Owner"), (to.id(), to.name()), text, WAIT);
        let Ok(reply) = asked else {
            return;
        };
        let live = self.live(from);
        let said = live.and_then(|live| {
            let mut l = lock(&live);
            let body = Body::Agent {
                text: reply.clone(),
                stop_reason: "end_turn".to_owned(),
            };
            let item = l.chat.said(now(), 0, to.id().to_owned(), body);
            let told = format!(
                "[cooee] The owner asked {} directly: {text}\n[{} answered]: {reply}",
                to.name(),
                to.name()
            );
            let handover = format!("{}\n\n{told}", l.meta.handover);
            handover.trim().clone_into(&mut l.meta.handover);
            self.save(&l)?;
            l.publish(self, vec![cooee_core::Change::Item(item)])?;
            Ok(())
        });
        if let Err(e) = said {
            cooee_core::note!("cooee: {from}: @{}: {e}", to.id());
        }
    }
}
