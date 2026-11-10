//! The models each agent offers (#193), kept per agent and account from each session's start and a daily probe; a
//! thread's pick is resolved against them and set on its session. A pinned model that is gone falls back to its
//! family's short name, said once on the thread (and its persona's notes).

use std::{collections::BTreeMap, fs};

use cooee_core::{
    Body, Change, Launch,
    models::{self, Offered},
    personas,
    shed_proto::AcpSession,
};

use super::{Hub, HubError, lock, now};

/// How old a list gets before the daily probe asks its agent again, in milliseconds.
const DAY: u64 = 24 * 60 * 60 * 1000;

impl Hub {
    /// Every agent and account's models, as last seen.
    #[must_use]
    pub fn models(&self) -> Vec<Offered> {
        lock(&self.models).clone()
    }

    /// `agent`'s models on `account`: none until a session reported them.
    #[must_use]
    pub fn offered(&self, agent: &str, account: &str) -> Option<Offered> {
        let lists = lock(&self.models);
        let mut found = lists.iter().filter(|o| o.agent == agent);
        found.find(|o| o.account == account).cloned()
    }

    /// Refuses `pick` for `agent` on `account` when its models are known and none fits it.
    pub(super) fn offers(&self, agent: &str, account: &str, pick: &str) -> Result<(), HubError> {
        match self.offered(agent, account) {
            Some(o) if !pick.is_empty() && o.resolve(pick).is_none() => Err(HubError::Refused(
                format!("{agent} offers no model {pick:?}"),
            )),
            _ => Ok(()),
        }
    }

    /// Keeps `models` (none: kept as they were) as `agent`'s on `account`, seen now.
    fn seen(&self, agent: &str, account: &str, models: &[[String; 2]]) {
        if models.is_empty() {
            return;
        }
        let mut lists = lock(&self.models);
        lists.retain(|o| (o.agent.as_str(), o.account.as_str()) != (agent, account));
        lists.push(Offered {
            agent: agent.to_owned(),
            account: account.to_owned(),
            models: models.to_vec(),
            seen: now(),
        });
        if let Err(e) = models::save(&self.data.join("models.json"), &lists) {
            eprintln!("cooee: models.json: {e}");
        }
    }

    /// Thread `id`'s session `s` came up on `terminal`: its models are kept and the thread's pick set on it.
    pub(super) fn settle(&self, id: &str, terminal: &str, s: &AcpSession) -> Result<(), HubError> {
        let live = self.live(id)?;
        let mut live = lock(&live);
        let meta = &live.meta;
        let (agent, account, pick) = (meta.agent.clone(), meta.account.clone(), meta.model.clone());
        self.seen(&agent, &account, &s.models);
        let Some(r) = self
            .offered(&agent, &account)
            .and_then(|o| o.resolve(&pick))
        else {
            return Ok(());
        };
        if let Some(short) = &r.fallback {
            let text = format!("model {pick} is gone: now {short} ({})", r.id);
            short.clone_into(&mut live.meta.model);
            self.store.save(&live.meta)?;
            let item = live.chat.push(now(), 0, Body::Note { text: text.clone() });
            live.publish(self, vec![Change::Item(item)])?;
            let persona = personas::list(self.config_dir(), &self.data).into_iter();
            let persona = persona.filter(|p| p.id == live.meta.profile && p.model == pick);
            for p in persona {
                if !personas::notes(&self.data, &p.id)
                    .iter()
                    .any(|n| n.text == text)
                {
                    let noted = personas::remember(&self.data, &p.id, "owner", &text, now());
                    noted.map_err(|e| HubError::Refused(e.to_string()))?;
                }
            }
        }
        drop(live);
        if s.model.as_deref() != Some(r.id.as_str()) {
            self.shed.set_model(terminal, &r.id)?;
        }
        Ok(())
    }

    /// The agents and accounts whose models were last seen a day or more before `at`.
    #[must_use]
    pub fn stale(&self, at: u64) -> Vec<(String, String)> {
        let lists = lock(&self.models);
        let old = lists.iter().filter(|o| at.saturating_sub(o.seen) >= DAY);
        old.map(|o| (o.agent.clone(), o.account.clone())).collect()
    }

    /// The daily refresh: starts `agent` on `account` (`launch`) only to keep the models its session reports.
    ///
    /// # Errors
    /// [`HubError::Shed`] when it doesn't start, or isn't closed again.
    pub fn probe(&self, agent: &str, account: &str, launch: &Launch) -> Result<(), HubError> {
        let terminal = format!("cooee-models-{:x}", now());
        let dir = self.scratch.join("models");
        fs::create_dir_all(&dir).map_err(|e| super::refused(&dir, &e))?;
        let run = (agent, self.adapter(agent));
        let started = self
            .shed
            .run_acp(&terminal, (run.0, &run.1), &dir, &BTreeMap::new(), launch);
        let session = started.and_then(|_| self.shed.ready(&terminal));
        let closed = self.shed.close(&terminal);
        if let Some(s) = session? {
            self.seen(agent, account, &s.models);
        }
        Ok(closed?)
    }
}
