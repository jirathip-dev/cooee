use std::{io, path::Path};

use cooee_core::{
    Item,
    agents::{self, Home},
    library::{self, Source, Update},
    skills::{self, Skill},
};
use serde::Deserialize;

use super::proposals::Proposed;
use super::{Hub, HubError};

/// An agent's `skill` proposal (#218): the skill, and the one it edits.
#[derive(Debug, Deserialize)]
pub struct SkillProposal {
    /// The skill.
    #[serde(flatten)]
    pub skill: Skill,
    /// The name of the skill it edits; none: a new one.
    #[serde(default)]
    pub replaces: Option<String>,
}

impl Hub {
    /// An agent's `skill` proposal in thread `id` (#218): a card with the skill's file, or an edit's diff; nothing is saved until the owner approves it.
    #[expect(clippy::missing_errors_doc, reason = "#738: moved docs")]
    pub fn propose_skill(
        &self,
        (id, token): (&str, &str),
        p: SkillProposal,
    ) -> Result<Item, HubError> {
        let home = self.home_of(id, token)?;
        let persona = self.token_of(id, token)?.profile;
        let old = skills::check(&home, &p.skill, p.replaces.as_deref())
            .map_err(|e| HubError::Refused(e.to_string()))?;
        let verb = if p.replaces.is_some() { "edit" } else { "add" };
        let tool = format!(
            "{verb} {persona}'s skill {}: {}",
            p.skill.name, p.skill.description
        );
        let detail = skills::diff(&old, &p.skill.file());
        let pending = Proposed::Skill {
            persona,
            skill: p.skill,
            replaces: p.replaces,
        };
        self.raise(id, (tool, "skill", detail), pending)
    }

    /// Brings thread `id`'s persona's library skills up to `sources` (#496), fetching in `work`: an unedited one is replaced, an edited one kept and its update raised as a `skill` diff card in the thread. Says what it did.
    #[expect(clippy::missing_errors_doc, reason = "#738: moved docs")]
    pub fn update_skills(
        &self,
        id: &str,
        sources: &[Source],
        work: &Path,
    ) -> Result<Vec<String>, HubError> {
        let thread = self.list().into_iter().find(|t| t.id == id);
        let persona = thread
            .ok_or_else(|| HubError::NoThread(id.to_owned()))?
            .persona;
        let home = self
            .config_dir()
            .ok_or_else(|| io::Error::other("no config folder"))
            .and_then(|c| agents::home(c, &persona))
            .map_err(|_| HubError::Refused(format!("thread {id} has no persona")))?;
        let done =
            library::update(&home, sources, work).map_err(|e| HubError::Refused(e.to_string()))?;
        let mut said = Vec::new();
        for (name, u) in done {
            said.push(match u {
                Update::Current => format!("{name}: up to date"),
                Update::Updated => format!("{name}: updated"),
                Update::Kept { new, old } => {
                    let tool = format!("update {persona}'s skill {name}: you edited it, so it stays until you approve");
                    let detail = skills::diff(&old, &new.file());
                    let pending = Proposed::Skill {
                        persona: persona.clone(),
                        skill: new,
                        replaces: Some(name.clone()),
                    };
                    self.raise(id, (tool, "skill", detail), pending)?;
                    format!("{name}: you edited it, so it is kept; the card shows the update")
                }
            });
        }
        Ok(said)
    }

    /// An agent's `skill` read in thread `id` (#218): its persona's skill `name`'s body, at most 8,000 characters.
    #[expect(clippy::missing_errors_doc, reason = "#738: moved docs")]
    pub fn read_skill(&self, (id, token): (&str, &str), name: &str) -> Result<String, HubError> {
        let home = self.home_of(id, token)?;
        let s = skills::read(&home, name).map_err(|e| {
            let persona = home
                .path()
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            HubError::Refused(format!("{e} for {persona}"))
        })?;
        Ok(s.body.chars().take(8000).collect())
    }

    pub(super) fn home_of(&self, id: &str, token: &str) -> Result<Home, HubError> {
        let meta = self.token_of(id, token)?;
        let config = self
            .config_dir()
            .ok_or_else(|| HubError::Refused("no config folder".to_owned()))?;
        agents::home(config, &meta.profile)
            .map_err(|_| HubError::Refused(format!("thread {id} has no persona")))
    }
}
