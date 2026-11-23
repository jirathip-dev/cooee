use std::{collections::HashMap, sync::Mutex};

use cooee_core::{
    Body, Card, Change,
    agents::{self, Agent},
    shed_proto::ApprovalOption,
};
use serde::Deserialize;

use super::{Hub, HubError, lock, now};

/// What `update_persona` changes; nothing else, so it never names another persona.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Update {
    /// The new Character.
    pub character: Option<String>,
    /// The new Instructions.
    pub instructions: Option<String>,
}

#[derive(Default)]
pub(super) struct State {
    updates: Mutex<HashMap<(String, u64), (String, Update)>>,
}

impl Update {
    fn apply(&self, mut p: Agent) -> (Agent, Vec<&'static str>) {
        let mut changed = Vec::new();
        for (name, new, old) in [
            ("Character", &self.character, &mut p.character),
            ("Instructions", &self.instructions, &mut p.instructions),
        ] {
            if let Some(new) = new.as_ref().filter(|n| *n != old) {
                new.clone_into(old);
                changed.push(name);
            }
        }
        (p, changed)
    }
}

fn diff(old: &Agent, new: &Agent) -> String {
    let mut out = Vec::new();
    for (name, was, now) in [
        ("Character", &old.character, &new.character),
        ("Instructions", &old.instructions, &new.instructions),
    ] {
        if was != now {
            out.push(name.to_owned());
            out.extend(was.lines().map(|l| format!("-{l}")));
            out.extend(now.lines().map(|l| format!("+{l}")));
        }
    }
    out.join("\n")
}

impl Hub {
    fn save_persona(&self, persona: &str, u: &Update) -> Result<Vec<&'static str>, HubError> {
        let refused = |e: std::io::Error| HubError::Refused(e.to_string());
        let config = self.config_dir().ok_or_else(|| {
            HubError::Refused("there is no config folder for personas".to_owned())
        })?;
        let (p, changed) =
            u.apply(agents::find(Some(config), &self.data, persona).map_err(refused)?);
        if !changed.is_empty() {
            agents::save(config, &self.data, p).map_err(refused)?;
        }
        Ok(changed)
    }

    /// An agent's `update_persona` in thread `id`: a setup chat's is saved at once, with an `Updated …` line each;
    /// any other's is a card with the diff, saved only when the owner approves it.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, one with no persona, or an update changing nothing.
    pub fn update_persona(&self, id: &str, token: &str, u: Update) -> Result<String, HubError> {
        let meta = self.token_of(id, token)?;
        self.scan(
            id,
            "update its persona",
            u.character.iter().chain(&u.instructions),
        )?;
        let old = agents::find(self.config_dir(), &self.data, &meta.profile)
            .map_err(|_| HubError::Refused(format!("thread {id} has no persona")))?;
        let (new, changed) = u.apply(old.clone());
        if changed.is_empty() {
            return Err(HubError::Refused(
                "give a new character or instructions".to_owned(),
            ));
        }
        let live = self.live(id)?;
        if meta.setup {
            let changed = self.save_persona(&old.id, &u)?;
            let mut live = lock(&live);
            let items = changed.iter().map(|f| {
                let text = format!("Updated {f}");
                Change::Item(live.chat.push(now(), 0, Body::Note { text }))
            });
            let items = items.collect();
            live.publish(self, items)?;
            return Ok("Saved; it reaches your next session.".to_owned());
        }
        let option = |o: &str| ApprovalOption {
            id: o.to_owned(),
            name: o.to_owned(),
            kind: String::new(),
        };
        let body = Body::Approval {
            tool: format!("update {}'s {}", old.name, changed.join(" and ")),
            options: vec![option("approve"), option("deny")],
            tapped: None,
            chosen: None,
            plugin: None,
            card: Card {
                form: Some("persona".to_owned()),
                detail: Some(diff(&old, &new)),
                ..Card::default()
            },
        };
        let mut live = lock(&live);
        let item = live.chat.push(now(), 0, body);
        live.publish(self, vec![Change::Item(item.clone())])?;
        lock(&self.areas.setup.updates).insert((id.to_owned(), item.id), (old.id, u));
        Ok(
            "Asked. The owner sees a card with the diff; nothing is saved unless they approve it."
                .to_owned(),
        )
    }

    pub(super) fn persona_answer(
        &self,
        id: &str,
        item: u64,
        option: &str,
    ) -> Result<Option<String>, String> {
        let u = lock(&self.areas.setup.updates).remove(&(id.to_owned(), item));
        if option != "approve" {
            return Ok(Some("denied".to_owned()));
        }
        let (persona, u) = u.ok_or("this update ran out (cooee restarted): ask the agent again")?;
        let changed = self.save_persona(&persona, &u).map_err(|e| e.to_string())?;
        Ok(Some(format!("updated {}", changed.join(" and "))))
    }
}
