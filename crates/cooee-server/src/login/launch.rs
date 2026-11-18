use std::path::Path;

use cooee_core::{
    Launch,
    agents::Agent,
    provider::{Setup, Want},
};

use super::Logins;
use crate::hub::HubError;

impl Logins {
    /// What a thread of ACP agent `agent` runs with: its account's name (empty for the system default, or an agent
    /// no provider signs in), and its config folder in the provider's variable (none for the system default). `account` none:
    /// the system default.
    ///
    /// # Errors
    /// [`HubError::Refused`] for an account the agent's provider doesn't have.
    pub fn home(&self, agent: &str, account: Option<&str>) -> Result<(String, Launch), HubError> {
        let Some(p) = self.providers.for_agent(agent) else {
            return Ok((String::new(), Launch::default()));
        };
        let a = self.account(&p.spec().name, account.unwrap_or_default())?;
        let env = p.env(a.dir.as_deref()).into_iter();
        let env = env.map(|(var, dir)| (var.to_owned(), dir.display().to_string()));
        let launch = Launch {
            env: env.collect(),
            ..Launch::default()
        };
        Ok((a.name, launch))
    }

    /// How agent `agent`'s provider sets up a persona session for `want` (#503); none: no provider signs it in.
    #[must_use]
    pub fn session(&self, agent: &str, want: &Want) -> Option<Setup> {
        Some(self.providers.for_agent(agent)?.session(want))
    }

    /// What a thread of agent `a` runs with: its account's label (`a`'s, else `account`) and [`Launch`] (the account's
    /// folder, the settings file in the provider's `settings` variable (in place of the account's folder when it is
    /// the same variable), the env, its secrets read from `data` through a file there, and the policy). An agent whose ACP agent can't run it as written is refused: settings
    /// with no `settings` variable (its guard can't run).
    ///
    /// # Errors
    /// [`HubError::Refused`] as above, for an unknown account or a missing secret.
    pub fn agent(
        &self,
        a: &Agent,
        account: Option<&str>,
        data: &Path,
    ) -> Result<(String, Launch), HubError> {
        let refused = |why: String| HubError::Refused(why);
        let (account, mut launch) = self.home(&a.agent, a.account.as_deref().or(account))?;
        let provider = self.providers.for_agent(&a.agent);
        if let Some(settings) = &a.settings {
            let var = provider
                .map(|v| v.spec().settings.as_str())
                .filter(|v| !v.is_empty());
            let Some(var) = var else {
                return Err(refused(format!(
                    "this profile's guard can't run with {}: its ACP adapter has no settings variable",
                    a.agent
                )));
            };
            launch.env.retain(|(k, _)| k != var);
            launch
                .env
                .push((var.to_owned(), settings.display().to_string()));
        }
        let (plain, secret) = a.env(data).map_err(refused)?;
        launch.env.extend(plain);
        if !secret.is_empty() {
            let name = cooee_core::random(16).map_err(|e| refused(e.to_string()))?;
            let name = cooee_core::sha256(&name);
            launch.secrets = Some((data.join("secrets").join(format!(".run-{name}")), secret));
        }
        launch.policy.clone_from(&a.policy);
        Ok((account, launch))
    }

    /// The efforts `agent`'s provider offers (#553 row 34).
    #[must_use]
    pub fn efforts(&self, agent: &str) -> Vec<String> {
        let p = self.providers.for_agent(agent);
        p.map(|p| p.spec().efforts.clone()).unwrap_or_default()
    }

    /// What a background run of `agent` picks when it names none (#219): its provider's cheapest model and lowest
    /// effort (empty: the agent's own).
    #[must_use]
    pub fn cheapest(&self, agent: &str) -> (String, String) {
        let p = self.providers.for_agent(agent).map(|p| p.spec());
        p.map(|s| {
            (
                s.cheapest.clone(),
                s.efforts.first().cloned().unwrap_or_default(),
            )
        })
        .unwrap_or_default()
    }

    /// The words that start `agent` on `effort` (empty: its default).
    ///
    /// # Errors
    /// [`HubError::Refused`] for an effort its provider doesn't offer.
    pub fn picks(&self, agent: &str, effort: &str) -> Result<Vec<String>, HubError> {
        let words = match self.providers.for_agent(agent) {
            Some(p) => p.picks(effort),
            None => effort.is_empty().then(Vec::new),
        };
        words.ok_or_else(|| HubError::Refused(format!("{agent} offers no effort {effort:?}")))
    }
}
