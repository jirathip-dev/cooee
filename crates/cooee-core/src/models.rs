//! The models each agent offers (#193), as its ACP sessions report them, kept per agent and account. A pick is a
//! model's full id (pinned) or a short name (`opus`, `gpt-5`) that means the newest model of its family: the models
//! whose id holds the short name's words in order. A model's own short name is its id's alphabetic words
//! (`acme-large-4-5` → `acme-large`); a pinned id that is gone falls back to it.

use std::{fs, io, path::Path};

use serde::{Deserialize, Serialize};

/// One agent and account's models, as a session last reported them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offered {
    /// The ACP agent id.
    pub agent: String,
    /// The account (empty: the system default).
    pub account: String,
    /// Each model's id and name, in the agent's order.
    pub models: Vec<[String; 2]>,
    /// When a session last reported them (unix ms).
    pub seen: u64,
}

/// A pick as one of an agent's models.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// The model id it runs on.
    pub id: String,
    /// The family's short name a gone pinned id fell back to.
    pub fallback: Option<String>,
}

fn words(s: &str) -> Vec<String> {
    s.split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

/// Model `id`'s short name: its alphabetic words (`acme-5.1-coder` → `acme-coder`).
#[must_use]
pub fn family(id: &str) -> String {
    let alpha = words(id).into_iter();
    let alpha = alpha.filter(|w| w.chars().all(|c| c.is_ascii_alphabetic()));
    alpha.collect::<Vec<_>>().join("-")
}

fn newness(id: &str) -> (Vec<u64>, Vec<u64>) {
    let nums = words(id)
        .into_iter()
        .filter_map(|w| Some((w.parse().ok()?, w.len())));
    let (dates, versions): (Vec<_>, Vec<_>) = nums.partition(|(_, len)| *len >= 6);
    let only = |v: Vec<(u64, usize)>| v.into_iter().map(|(n, _)| n).collect();
    (only(versions), only(dates))
}

impl Offered {
    fn newest(&self, short: &str) -> Option<&str> {
        let want = words(short);
        let member = |id: &str| {
            let mut have = words(id).into_iter();
            !want.is_empty() && want.iter().all(|w| have.any(|h| &h == w))
        };
        let mut best: Option<(&str, _)> = None;
        for id in self
            .models
            .iter()
            .map(|m| m[0].as_str())
            .filter(|id| member(id))
        {
            if best.as_ref().is_none_or(|(_, n)| newness(id) > *n) {
                best = Some((id, newness(id)));
            }
        }
        best.map(|(id, _)| id)
    }

    /// `pick` as one of these models: an id as is, a short name as its family's newest, a gone id as its family's
    /// newest with that fallback said; none when nothing fits.
    #[must_use]
    pub fn resolve(&self, pick: &str) -> Option<Resolved> {
        let found = |id: &str, fallback| Resolved {
            id: id.to_owned(),
            fallback,
        };
        if self.models.iter().any(|m| m[0] == pick) {
            return Some(found(pick, None));
        }
        if let Some(id) = self.newest(pick) {
            return Some(found(id, None));
        }
        let short = family(pick);
        let id = self.newest(&short).filter(|_| short != pick)?;
        Some(found(id, Some(short)))
    }

    /// Each short name, with the id it resolves to now.
    #[must_use]
    pub fn short_names(&self) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = Vec::new();
        for m in &self.models {
            let short = family(&m[0]);
            if short.is_empty() || out.iter().any(|(s, _)| *s == short) {
                continue;
            }
            if let Some(id) = self.newest(&short) {
                out.push((short, id.to_owned()));
            }
        }
        out
    }

    /// What a model picker offers: the short names, then the ids.
    #[must_use]
    pub fn picks(&self) -> Vec<String> {
        let mut out: Vec<String> = self.short_names().into_iter().map(|(s, _)| s).collect();
        for m in &self.models {
            if !out.contains(&m[0]) {
                out.push(m[0].clone());
            }
        }
        out
    }
}

/// The lists kept in `path` (none when there is no file or it can't be read).
#[must_use]
pub fn load(path: &Path) -> Vec<Offered> {
    let text = fs::read_to_string(path).unwrap_or_default();
    serde_json::from_str(&text).unwrap_or_default()
}

/// Replaces the lists kept in `path`, atomically.
///
/// # Errors
/// When it can't be written.
pub fn save(path: &Path, lists: &[Offered]) -> io::Result<()> {
    let text = serde_json::to_string_pretty(lists).map_err(io::Error::other)? + "\n";
    crate::write_private(path, text.as_bytes())
}

#[cfg(test)]
mod tests;
