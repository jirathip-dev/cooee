use std::fmt;

use serde::{Deserialize, Serialize};

use super::{Agent, Role};

const ROLE_MAX: usize = 40;
const USE_MAX: usize = 120;

/// What an agent is for, as Chief's routing tool lists it: a few words and one "use me for …" line.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Purpose {
    /// A few words, at most 40 characters.
    pub role: String,
    /// One line, at most 120 characters: what to use it for.
    #[serde(rename = "use")]
    pub use_for: String,
}

/// How Chief may reach an agent: delegate (Chief asks and sums up) or hand-off (its own chat opens for the owner).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
#[expect(missing_docs, reason = "the enum's line names each variant in order")]
pub enum Mode {
    Delegate,
    HandOff,
}

/// An agent Chief may route to; only [`Table::of`] makes one, from agents that exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Routable {
    id: String,
    name: String,
    purpose: Option<Purpose>,
    mode: Mode,
}

impl Routable {
    /// Its id.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Its name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Its mode.
    #[must_use]
    pub const fn mode(&self) -> Mode {
        self.mode
    }

    /// Its line in the routing tool's description.
    #[must_use]
    pub fn line(&self) -> String {
        let (role, use_for) = self
            .purpose
            .as_ref()
            .map_or(("", ""), |p| (p.role.as_str(), p.use_for.as_str()));
        let mode = match self.mode {
            Mode::Delegate => "delegate",
            Mode::HandOff => "hand-off",
        };
        format!("- {} · {role} · {mode}: {use_for}", self.id)
    }
}

/// Why a route found no agent: the word asked and the ids there are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoAgent {
    /// What was asked for.
    pub asked: String,
    /// The ids it could have been.
    pub ids: Vec<String>,
}

impl fmt::Display for NoAgent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "NO_AGENT: no agent {}. The agents are {}.",
            self.asked,
            self.ids.join(", ")
        )
    }
}

/// The agents Chief routes to, read from the agents that exist.
#[derive(Debug, Clone, Default)]
pub struct Table(Vec<Routable>);

fn key(s: &str) -> String {
    s.replace(' ', "").to_lowercase()
}

impl Table {
    /// Every agent of `all` but Chief, a hidden one, a profile an owner agent stands on, an owner agent on none served.
    #[must_use]
    pub fn of(all: &[Agent]) -> Self {
        let taken = |id: &str| all.iter().any(|a| a.profile.as_deref() == Some(id));
        let hidden = |a: &Agent| a.lent.as_ref().is_some_and(|l| l.hidden.is_some());
        let routable = all
            .iter()
            .filter(|a| a.role != Role::Chief && !hidden(a) && !taken(&a.id))
            .filter_map(|a| crate::profiles::onto(a, all).ok())
            .map(|a| Routable {
                id: a.id.clone(),
                name: a.name.clone(),
                purpose: a.purpose.clone(),
                mode: a.mode(),
            });
        Self(routable.collect())
    }

    /// The agents that declare a purpose: the routing tool's list.
    pub fn declared(&self) -> impl Iterator<Item = &Routable> {
        self.0.iter().filter(|r| r.purpose.is_some())
    }

    /// The declared agent `id`, else [`NoAgent`] with the declared ids.
    #[expect(clippy::missing_errors_doc, reason = "the line names NoAgent")]
    pub fn pick(&self, id: &str) -> Result<&Routable, NoAgent> {
        let mut declared = self.declared();
        declared.find(|r| r.id == id).ok_or_else(|| NoAgent {
            asked: id.to_owned(),
            ids: self.declared().map(|r| r.id.clone()).collect(),
        })
    }

    /// What an `@name` reaches: one agent by id or spaceless name in any case, else [`NoAgent`] with every id.
    #[expect(clippy::missing_errors_doc, reason = "the line names NoAgent")]
    pub fn named(&self, word: &str) -> Result<&Routable, NoAgent> {
        let word = key(word);
        let mut hits = self
            .0
            .iter()
            .filter(|r| key(&r.id) == word || key(&r.name) == word);
        match (hits.next(), hits.next()) {
            (Some(r), None) => Ok(r),
            _ => Err(NoAgent {
                asked: format!("@{word}"),
                ids: self.0.iter().map(|r| r.id.clone()).collect(),
            }),
        }
    }
}

/// A leading `@word` of `text` and the rest, trimmed.
#[must_use]
pub fn lead(text: &str) -> Option<(&str, &str)> {
    let rest = text.trim_start().strip_prefix('@')?;
    let end = rest
        .find(|c: char| !(c.is_alphanumeric() || "-_/".contains(c)))
        .unwrap_or(rest.len());
    let (word, rest) = rest.split_at(end);
    (!word.is_empty()).then(|| (word, rest.trim()))
}

impl Agent {
    /// Its mode: its own, else delegate for the owner's agent and hand-off for a plugin's.
    #[must_use]
    pub fn mode(&self) -> Mode {
        let lent = if self.lent.is_some() {
            Mode::HandOff
        } else {
            Mode::Delegate
        };
        self.mode.unwrap_or(lent)
    }

    pub(super) fn check_route(&self) -> Result<(), String> {
        if let Some(p) = &self.purpose {
            let line = |s: &str, max: usize| {
                !s.trim().is_empty() && !s.contains('\n') && s.chars().count() <= max
            };
            if !line(&p.role, ROLE_MAX) || !line(&p.use_for, USE_MAX) {
                return Err(format!(
                    "`purpose` needs role (one line, at most {ROLE_MAX} characters) and use (one line, at most {USE_MAX})"
                ));
            }
            if let Some(why) = [&p.role, &p.use_for]
                .into_iter()
                .find_map(|t| crate::memory::refusal(t))
            {
                return Err(format!("`purpose`: {why}"));
            }
        }
        let own = [
            ("mode", self.mode.is_some()),
            ("settings", self.settings.is_some()),
            ("env", !self.env.is_empty()),
            ("mcp", !self.mcp.is_empty()),
            ("policy", self.policy.is_some()),
            ("fallback", !self.fallback.is_empty()),
        ];
        match (&self.profile, own.iter().find(|(_, set)| *set)) {
            (Some(_), Some((k, _))) => Err(format!("`{k}` is the profile's beside `profile`")),
            (Some(p), None) if !p.contains('/') => Err("`profile` is `<plugin>/<name>`".to_owned()),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::Lent;

    fn agent(id: &str, purpose: bool, mode: Option<Mode>, lent: bool) -> Agent {
        Agent {
            id: id.to_owned(),
            name: id.to_uppercase(),
            agent: "a".to_owned(),
            purpose: purpose.then(|| Purpose {
                role: "R".to_owned(),
                use_for: "u".to_owned(),
            }),
            mode,
            lent: lent.then(Lent::default),
            ..Agent::default()
        }
    }

    #[test]
    fn the_table_picks_only_declared_agents_and_names_any_but_chief() {
        let chief = Agent {
            role: Role::Chief,
            ..agent("chief", true, None, false)
        };
        let all = [
            chief,
            agent("ada", true, None, false),
            agent("bo", false, None, false),
            agent("drover/default", true, None, true),
        ];
        let t = Table::of(&all);
        assert_eq!(t.pick("ada").unwrap().mode(), Mode::Delegate);
        assert_eq!(t.pick("drover/default").unwrap().mode(), Mode::HandOff);
        let no = t.pick("bo").unwrap_err();
        assert_eq!(no.ids, ["ada", "drover/default"]);
        assert!(t.pick("chief").is_err());
        assert_eq!(t.named("BO").unwrap().id(), "bo");
        assert!(t.named("chief").is_err());
        let own = Agent {
            profile: Some("drover/default".to_owned()),
            ..agent("orchestrator", false, None, false)
        };
        let t = Table::of(&[own, agent("drover/default", true, None, true)]);
        assert_eq!(t.named("drover/default").unwrap_err().ids, ["orchestrator"]);
    }

    #[test]
    fn a_leading_mention_is_split_from_its_text() {
        assert_eq!(lead(" @ada  hi there"), Some(("ada", "hi there")));
        assert_eq!(lead("@ada"), Some(("ada", "")));
        assert_eq!(lead("hi @ada"), None);
        assert_eq!(lead("@ hi"), None);
    }

    #[test]
    fn a_purpose_is_one_short_line_and_profile_keys_stay_the_profiles() {
        assert!(agent("a", true, None, false).check().is_ok());
        let mut a = agent("a", true, None, false);
        a.purpose = Some(Purpose {
            role: "x".repeat(41),
            use_for: "u".to_owned(),
        });
        assert!(a.check().unwrap_err().contains("purpose"));
        a.purpose = Some(Purpose {
            role: "R".to_owned(),
            use_for: "a\nb".to_owned(),
        });
        assert!(a.check().is_err());
        let on = Agent {
            profile: Some("drover/default".to_owned()),
            mode: Some(Mode::Delegate),
            ..agent("o", false, None, false)
        };
        assert!(on.check().unwrap_err().contains("`mode`"));
        let on = Agent {
            mode: None,
            agent: String::new(),
            ..on
        };
        assert!(on.check().is_ok());
    }
}
