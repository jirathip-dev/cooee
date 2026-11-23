use std::{fs, path::Path};

use cooee_core::{
    Launch,
    agents::Agent,
    provider::{Gap, Setup, Want},
};
use serde_json::{Map, Value};

use super::Logins;
use crate::hub::HubError;

impl Logins {
    /// ACP agent `agent`'s account name (empty: shed's own login) and the [`Launch`] that names it to shed.
    #[must_use]
    pub fn home(&self, agent: &str, account: Option<&str>) -> (String, Launch) {
        let signs_in = self.providers.for_agent(agent).is_some();
        let account = account
            .filter(|a| signs_in && !a.is_empty())
            .map(str::to_owned);
        let launch = Launch {
            account: account.clone(),
            ..Launch::default()
        };
        (account.unwrap_or_default(), launch)
    }

    /// How agent `agent`'s provider sets up a persona session for `want` (#503); none: no provider signs it in.
    #[must_use]
    pub fn session(&self, agent: &str, want: &Want) -> Option<Setup> {
        Some(self.providers.for_agent(agent)?.session(want))
    }

    /// What a persona session of `agent` can't be told, on its default login (#536: the agent picker's lines).
    #[must_use]
    pub fn gaps(&self, agent: &str) -> Vec<Gap> {
        let want = Want {
            brief: "",
            off: &[],
            ask_all: true,
            run: Path::new(""),
        };
        let setup = self.session(agent, &want);
        setup.unwrap_or_else(|| Setup::plain(&want)).gaps
    }

    /// # Errors [`HubError::Refused`] for settings its provider can't take, an unreadable settings file or a missing secret.
    pub fn agent(
        &self,
        a: &Agent,
        (account, meta): (Option<&str>, Option<Value>),
        data: &Path,
    ) -> Result<(String, Launch), HubError> {
        let refused = |why: String| HubError::Refused(why);
        let (account, mut launch) = self.home(&a.agent, a.account.as_deref().or(account));
        launch.meta = meta;
        if let Some(settings) = &a.settings {
            let provider = self.providers.for_agent(&a.agent);
            let key = provider.map(|p| p.spec().settings.as_str());
            let Some(key) = key.filter(|k| !k.is_empty()) else {
                return Err(refused(format!(
                    "this profile's guard can't run with {}: its ACP adapter takes no settings",
                    a.agent
                )));
            };
            launch.meta = Some(with_settings(launch.meta.take(), key, settings).map_err(refused)?);
        }
        let (plain, secret) = a.env(data).map_err(refused)?;
        launch.env.extend(plain.into_iter().chain(secret));
        launch.policy.clone_from(&a.policy);
        Ok((account, launch))
    }

    /// The efforts shed's record of `agent` offers (#553 row 34).
    #[must_use]
    pub fn efforts(&self, agent: &str) -> Vec<String> {
        let r = self.providers.record(agent);
        r.map(|r| r.efforts.clone()).unwrap_or_default()
    }

    /// What a background run of `agent` picks when it names none (#219): its provider's cheapest model and lowest
    /// effort (empty: the agent's own).
    #[must_use]
    pub fn cheapest(&self, agent: &str) -> (String, String) {
        let p = self.providers.for_agent(agent).map(|p| p.spec());
        p.map(|s| {
            let efforts = self.efforts(agent);
            (
                s.cheapest.clone(),
                efforts.first().cloned().unwrap_or_default(),
            )
        })
        .unwrap_or_default()
    }

    /// `agent`'s effort for shed's `run` (empty: its default).
    ///
    /// # Errors
    /// [`HubError::Refused`] for an effort shed's record of it doesn't offer.
    pub fn effort(&self, agent: &str, effort: &str) -> Result<Option<String>, HubError> {
        if effort.is_empty() {
            return Ok(None);
        }
        if !self.efforts(agent).iter().any(|e| e == effort) {
            return Err(HubError::Refused(format!(
                "{agent} offers no effort {effort:?}"
            )));
        }
        Ok(Some(effort.to_owned()))
    }
}

fn with_settings(meta: Option<Value>, key: &str, settings: &Path) -> Result<Value, String> {
    let file = if settings.is_dir() {
        settings.join("settings.json")
    } else {
        settings.to_path_buf()
    };
    let read = fs::read(&file).ok();
    let Some(Value::Object(read)) = read.and_then(|b| serde_json::from_slice(&b).ok()) else {
        return Err(format!("{}: no settings object", file.display()));
    };
    let mut meta = meta.unwrap_or_else(|| Value::Object(Map::new()));
    let mut at = &mut meta;
    for k in key.split('.') {
        if !at.is_object() {
            *at = Value::Object(Map::new());
        }
        let Value::Object(o) = at else {
            break;
        };
        at = o.entry(k).or_insert_with(|| Value::Object(Map::new()));
    }
    match at {
        Value::Object(o) => o.extend(read),
        other => *other = Value::Object(read),
    }
    Ok(meta)
}
