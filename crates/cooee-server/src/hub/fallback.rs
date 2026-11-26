use std::iter;

use cooee_core::{
    Body, Card, Change, Item, Thread,
    agents::{self, AUTO, Capped, Seat},
    shed_proto::ApprovalOption,
};

use super::{Hub, HubError, Recipe, live::Live, lock, now};

pub(super) const FALLBACK: &str = "fallback";

const WAIT: &str = "wait";

const GO_ON: &str = "Go on with my last message: the agent before you hit its usage limit.";

impl Hub {
    pub(super) fn capped(&self, live: &mut Live) -> Option<Item> {
        let resets = live.chat.turn.note.clone().unwrap_or_default();
        let (agent, account) = (live.meta.agent.clone(), live.meta.account.clone());
        let hit = if account == AUTO {
            format!("every {agent} account is limited{}", until(&resets))
        } else {
            format!("{} capped{}", seat(&agent, &account), until(&resets))
        };
        let others = self.chain(&live.meta);
        if others.is_empty() {
            return (account == AUTO).then(|| live.chat.push(now(), 0, Body::Note { text: hit }));
        }
        live.meta
            .capped
            .retain(|c| (&c.agent, &c.account) != (&agent, &account));
        live.meta.capped.push(Capped {
            agent,
            account,
            resets,
        });
        let (text, options) = fallback_choices(&hit, &others, &live.meta.capped);
        if let Err(e) = self.save(live) {
            cooee_core::note!("cooee: {e}");
        }
        let body = Body::Approval {
            tool: text,
            options,
            tapped: None,
            chosen: None,
            plugin: None,
            card: Card {
                form: Some(FALLBACK.to_owned()),
                ..Card::default()
            },
        };
        Some(live.chat.push(now(), 0, body))
    }

    fn chain(&self, m: &Thread) -> Vec<Seat> {
        if !m.room.is_empty() || m.workflow.is_some() || m.profile.is_empty() {
            return Vec::new();
        }
        let personas = agents::all(self.config_dir(), &self.data, |n| self.served(n));
        let Some(p) = personas.into_iter().find(|a| a.id == m.profile) else {
            return Vec::new();
        };
        let head = Seat {
            agent: p.agent,
            account: p.account,
        };
        let mut others: Vec<Seat> = Vec::new();
        for s in iter::once(head).chain(p.fallback) {
            if s.agent != m.agent && others.iter().all(|o| o.agent != s.agent) {
                others.push(s);
            }
        }
        others
    }

    pub(super) fn uncapped(&self, live: &mut Live) {
        let m = &mut live.meta;
        let before = m.capped.len();
        m.capped
            .retain(|c| (&c.agent, &c.account) != (&m.agent, &m.account));
        if m.capped.len() != before
            && let Err(e) = self.save(live)
        {
            cooee_core::note!("cooee: {e}");
        }
    }

    /// The agent and account option `option` of thread `id`'s open fallback card `item` names (#76); none for its
    /// `wait`, an answered card or any other item.
    #[must_use]
    pub fn fallback_seat(&self, id: &str, item: u64, option: &str) -> Option<Seat> {
        let live = self.live(id).ok()?;
        let live = lock(&live);
        let card = live.chat.items.iter().find(|i| i.id == item)?;
        let Body::Approval {
            options,
            tapped: None,
            chosen: None,
            plugin: None,
            card,
            ..
        } = &card.body
        else {
            return None;
        };
        if card.form.as_deref() != Some(FALLBACK)
            || option == WAIT
            || !options.iter().any(|o| o.id == option)
        {
            return None;
        }
        let (agent, account) = option.split_once(" · ").unwrap_or((option, ""));
        Some(Seat {
            agent: agent.to_owned(),
            account: Some(account.to_owned()).filter(|a| !a.is_empty()),
        })
    }

    /// `by` picks `option` on thread `id`'s fallback card `item` (#76): the thread switches to it, set up by
    /// `recipe`, with a handoff of the thread so far, and the turn goes on there. A second tap returns the card.
    ///
    /// # Errors
    /// As [`Hub::switch`], the card left open; as [`Hub::send`].
    pub fn fall_back(
        &self,
        (id, item): (&str, u64),
        option: &str,
        recipe: &Recipe,
        by: &str,
    ) -> Result<Option<Item>, HubError> {
        let live = self.live(id)?;
        {
            let mut l = lock(&live);
            let Some(tapped) = l.chat.tap(item, option, by, now()) else {
                return Ok(l.chat.items.iter().find(|i| i.id == item).cloned());
            };
            l.publish(self, vec![Change::Item(tapped)])?;
        }
        let agent = option.split(" · ").next().unwrap_or(option);
        let switched = self.switch(id, agent, recipe);
        let mut l = lock(&live);
        let done = l.chat.update(item, 0, |b| {
            if let Body::Approval { tapped, chosen, .. } = b {
                if switched.is_ok() {
                    *chosen = Some(option.to_owned());
                } else {
                    *tapped = None;
                }
            }
        });
        l.publish(self, done.clone().map(Change::Item).into_iter().collect())?;
        switched?;
        let q = self.take(&l, GO_ON, &format!("fallback-{item}"), &[])?;
        drop(l);
        self.deliver(&live, q, "")?;
        Ok(done)
    }
}

pub(super) fn waited(option: &str) -> Result<Option<String>, String> {
    if option == WAIT {
        Ok(Some("waiting for its reset".to_owned()))
    } else {
        Err(format!("no {option} in this chat's chain"))
    }
}

fn fallback_choices(hit: &str, others: &[Seat], all: &[Capped]) -> (String, Vec<ApprovalOption>) {
    let capped = |s: &Seat| {
        all.iter()
            .find(|c| c.agent == s.agent && s.account.as_ref().is_none_or(|a| *a == c.account))
    };
    let text = match all.first() {
        Some(c) if others.iter().all(|s| capped(s).is_some()) => format!(
            "{hit}: every agent in this chat's chain is capped; {} resets first{}",
            seat(&c.agent, &c.account),
            Some(&c.resets)
                .filter(|r| !r.is_empty())
                .map(|r| format!(", {r}"))
                .unwrap_or_default()
        ),
        _ => format!("{hit}, continue on"),
    };
    let mut options: Vec<ApprovalOption> = others
        .iter()
        .map(|s| {
            let id = seat(&s.agent, s.account.as_deref().unwrap_or_default());
            let name = capped(s).map_or_else(
                || id.clone(),
                |c| format!("{id} (capped{})", until(&c.resets)),
            );
            ApprovalOption {
                id,
                name,
                kind: String::new(),
            }
        })
        .collect();
    options.push(ApprovalOption {
        id: WAIT.to_owned(),
        name: WAIT.to_owned(),
        kind: String::new(),
    });
    (text, options)
}

fn seat(agent: &str, account: &str) -> String {
    if account.is_empty() {
        agent.to_owned()
    } else {
        format!("{agent} · {account}")
    }
}

fn until(resets: &str) -> String {
    if resets.is_empty() {
        String::new()
    } else {
        format!(" until {resets}")
    }
}
