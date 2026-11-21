use cooee_core::{mcp, secrets};
use serde::Deserialize;

use super::{Hub, HubError, LABEL, mcp::Proposed};

/// An agent's `secret` (#163): secret `name` typed into shed terminal `terminal`, why, and whether to keep it.
#[derive(Debug, Deserialize)]
pub struct SecretAsk {
    /// Its name, a variable's (`CRUNCHBASE_KEY`).
    pub name: String,
    /// Why it is wanted.
    #[serde(default)]
    pub why: String,
    /// The terminal it is typed into.
    pub terminal: String,
    /// Kept in the keychain for the thread's persona, to be typed again without asking.
    #[serde(default)]
    pub save: bool,
}

pub(super) struct Asked {
    name: String,
    terminal: String,
    key: Option<String>,
}

fn entered(name: &str) -> String {
    format!("secret {name} entered")
}

impl Hub {
    /// An agent's `secret` in thread `id`: a saved one of its persona's is typed into the terminal at once; else a
    /// card asks the owner, whose value (with Face ID) is typed once and kept only when the agent asked to save it.
    /// The agent only ever reads `secret NAME entered`.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, a bad name, a save with no persona, or a terminal
    /// that is an agent's session or the owner's shell; [`HubError::Shed`] when shed fails.
    pub fn secret(&self, id: &str, token: &str, a: SecretAsk) -> Result<String, HubError> {
        let persona = self.token_of(id, token)?.profile;
        if !mcp::var(&a.name) {
            return Err(HubError::Refused(
                "name the secret like a variable: CRUNCHBASE_KEY".to_owned(),
            ));
        }
        if a.save && persona.is_empty() {
            return Err(HubError::Refused(
                "only a persona's thread can save a secret".to_owned(),
            ));
        }
        let t = self.shed.get(&a.terminal)?;
        if t.acp.is_some() || t.labels.contains_key(&format!("{LABEL}.owner")) {
            return Err(HubError::Refused(format!(
                "{} is an agent's session or the owner's shell: cooee types a secret only into a program's terminal",
                a.terminal
            )));
        }
        let key = secrets::persona(&persona, &a.name);
        let saved = if persona.is_empty() {
            None
        } else {
            secrets::get(&self.data, &key).map_err(|e| HubError::Refused(e.to_string()))?
        };
        if let Some(v) = saved {
            self.shed.type_line(&a.terminal, &v)?;
            return Ok(entered(&a.name));
        }
        let why: String = a.why.trim().chars().take(300).collect();
        let kept = if a.save {
            format!("saved in the keychain for {persona}")
        } else {
            "typed once, never stored".to_owned()
        };
        let tool = format!("secret {} for {}: {why} · {kept}", a.name, a.terminal);
        let said = format!(
            "Asked the owner for secret {0}. Once they drop it, cooee types it into {1} and tells you `{2}`; you never see the value.",
            a.name,
            a.terminal,
            entered(&a.name)
        );
        let asked = Asked {
            name: a.name.clone(),
            terminal: a.terminal,
            key: a.save.then_some(key),
        };
        self.raise(id, (tool, "secret", a.name), Proposed::Secret(asked))?;
        Ok(said)
    }

    pub(super) fn drop_secret(
        &self,
        (id, item): (&str, u64),
        a: &Asked,
        value: Option<&str>,
    ) -> Result<String, String> {
        let v = value.filter(|v| !v.is_empty()).ok_or("no value was sent")?;
        if v.contains(['\n', '\r']) {
            return Err("a secret is one line".to_owned());
        }
        if let Some(key) = &a.key {
            secrets::set(&self.data, key, v).map_err(|e| e.to_string())?;
        }
        self.shed
            .type_line(&a.terminal, v)
            .map_err(|e| e.to_string())?;
        let said = entered(&a.name);
        self.queue(id, &said, &format!("secret-{item}"), &[], None)
            .ok();
        Ok(said)
    }
}
