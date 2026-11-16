//! A plugin's agent profiles: `[[profiles]]` in its `cooee-plugin.toml`, or its backend's list
//! (`GET /cooee/profiles`), their paths in its build; [`Profile::lend`] reads one as a read-only agent. A secret
//! never sits in a profile: an `env` value is plain text or `secret:<NAME>`, a saved secret ([`crate::secrets`]).

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::agents::{self, Agent, Lent, Style};

/// One agent profile.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// Its name: lowercase letters, digits and `-`; lent as agent `<plugin>/<name>` ([`Profile::lend`]).
    pub name: String,
    /// What the picker and the thread's header show.
    pub label: String,
    /// The ACP agent (`shed agents` id).
    pub agent: String,
    /// Its provider's account (none: the provider's default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account: Option<String>,
    /// The folder its threads work in (a root or inside one; none: the one picked).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<PathBuf>,
    /// A file sent before the first message.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instructions: Option<PathBuf>,
    /// An agent settings file (its hooks are the profile's guard), passed through the provider's `settings`
    /// variable; an agent without one can't start the profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<PathBuf>,
    /// Variables for the agent: plain text (a `{thread}` in it is the thread's id), or `secret:<NAME>`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// The MCP servers it picks from the registry (`[[mcp]]`: `server`, and `tools`, none for all; #56).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp: Vec<crate::mcp::Pick>,
    /// shed's approval policy: `ask` (shed's default), `allow` or `deny`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>,
    /// Not offered nor started (a plugin's, while its guard can't be proven); `why` says why.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub hidden: bool,
    /// Why it is hidden.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub why: String,
    /// Its avatar in the sidebar (#86): an image (`https://…`, `data:image/…`, or a plugin's `/path` on its backend),
    /// or a sprite of the theme's (`s-<id>` in its `sprites.svg`); none: its initials.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    /// Threads its plugin lists as this agent's (its side threads, the newest the ongoing chat).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub threads: Vec<String>,
}

impl Profile {
    /// Checks its name, then what [`Agent::check`] checks; an error names the key, never the value.
    ///
    /// # Errors
    /// What is wrong, for the owner.
    pub fn check(&self) -> Result<(), String> {
        if !agents::is_id(&self.name) {
            return Err("profile: `name` must be lowercase letters, digits and -".to_owned());
        }
        self.clone()
            .into_agent(String::new())
            .check()
            .map_err(|e| format!("profile {}: {e}", self.name))
    }

    pub(crate) fn rooted(mut self, base: &Path) -> Self {
        let paths = [&mut self.instructions, &mut self.settings, &mut self.folder];
        for p in paths.into_iter().flatten() {
            *p = agents::whole(p, base);
        }
        self
    }

    pub(crate) fn into_agent(self, instructions: String) -> Agent {
        Agent {
            name: self.label,
            agent: self.agent,
            avatar: self.avatar,
            folder: self
                .folder
                .map(|f| f.display().to_string())
                .unwrap_or_default(),
            instructions,
            account: self.account,
            settings: self.settings,
            env: self.env,
            mcp: self.mcp,
            policy: self.policy,
            ..Agent::default()
        }
    }

    /// Plugin `plugin`'s profile, its paths in its build `build`, lent as agent `<plugin>/<name>`: read-only, with no
    /// folder, notes or skills.
    ///
    /// # Errors
    /// A rule it breaks, or an instructions file that can't be read.
    pub fn lend(self, plugin: &str, build: &Path) -> Result<Agent, String> {
        self.check()?;
        let p = self.rooted(build);
        let text = match &p.instructions {
            Some(f) => fs::read_to_string(f)
                .map_err(|e| format!("profile {}: {}: {e}", p.name, f.display()))?,
            None => String::new(),
        };
        let lent = Lent {
            plugin: plugin.to_owned(),
            hidden: p.hidden.then(|| p.why.clone()),
            threads: p.threads.clone(),
        };
        Ok(Agent {
            id: format!("{plugin}/{}", p.name),
            style: Style::Detailed,
            lent: Some(lent),
            ..p.into_agent(text)
        })
    }
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    fn profile(text: &str) -> Profile {
        toml::from_str(&format!(
            "name = \"p\"\nlabel = \"P\"\nagent = \"a\"\n{text}"
        ))
        .unwrap()
    }

    #[test]
    fn a_secret_inline_or_a_bad_policy_is_refused_naming_the_key_only() {
        assert_eq!(
            profile("env = { A = \"x\", B = \"secret:B_1\" }\npolicy = \"allow\"").check(),
            Ok(())
        );
        let token = format!("sk{}", "a1".repeat(12));
        let e = profile(&format!("env = {{ TOKEN = \"{token}\" }}"))
            .check()
            .unwrap_err();
        assert!(e.contains("env TOKEN") && !e.contains(&token), "{e}");
        assert!(profile("env = { A = \"secret:../x\" }").check().is_err());
        assert!(profile("env = { \"A B\" = \"x\" }").check().is_err());
        assert!(profile("policy = \"yolo\"").check().is_err());
        assert!(profile("").check().is_ok());
        assert!(profile("env = { T = \"{thread}\" }").check().is_ok());
        for ok in [
            "https://x/a.png",
            "data:image/png;base64,AA",
            "/a.png",
            "idle",
        ] {
            assert!(profile(&format!("avatar = {ok:?}")).check().is_ok(), "{ok}");
        }
        for bad in ["javascript:alert(1)", "http://x/a.png", "", "Idle"] {
            assert!(
                profile(&format!("avatar = {bad:?}")).check().is_err(),
                "{bad}"
            );
        }
        assert!(
            toml::from_str::<Profile>("name = \"p\"\nlabel = \"P\"\nagent = \"a\"\nhooks = 1")
                .is_err()
        );
    }

    #[test]
    fn secrets_are_read_from_their_files_and_plain_values_kept() {
        let data = env::temp_dir().join(format!("cooee-profile-env-{}", std::process::id()));
        fs::create_dir_all(data.join("secrets")).unwrap();
        fs::write(data.join("secrets/S"), "s3cret\n").unwrap();
        let p = profile("env = { A = \"x\", B = \"secret:S\" }");
        let (plain, secret) = p.into_agent(String::new()).env(&data).unwrap();
        assert_eq!(plain, [("A".to_owned(), "x".to_owned())]);
        assert_eq!(secret, [("B".to_owned(), "s3cret".to_owned())]);
        let e = profile("env = { B = \"secret:NONE\" }")
            .into_agent(String::new())
            .env(&data)
            .unwrap_err();
        assert!(e.contains("secret NONE"), "{e}");
        fs::remove_dir_all(&data).unwrap();
    }
}
