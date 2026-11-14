//! Agent profiles: a named recipe for a thread's agent (its agent, account, folder, instructions, settings, env, MCP
//! servers and shed's approval policy), picked when the thread starts and kept for its life. Plain (no profile) is
//! the agent with the owner's own setup. The owner's are `<config>/profiles/*.toml`; a plugin brings its own as
//! `[[profiles]]` in its `cooee-plugin.toml`, or its backend serves them (`GET /cooee/profiles`), their paths in its
//! build. A secret never sits in a profile: an `env`
//! value is plain text or `secret:<NAME>`, a saved secret ([`crate::secrets`]).

use std::{
    collections::BTreeMap,
    env, fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{
    mcp::var,
    plugins::{self, Manifest},
};

/// Variables, `K=V`.
pub type Vars = Vec<(String, String)>;

/// One agent profile.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// Its id: lowercase letters, digits and `-` (a plugin's: `<plugin>/<name>`, see [`Profile::id`]).
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
    /// The plugin it came with (none: the owner's own).
    #[serde(skip)]
    pub plugin: Option<String>,
}

impl Profile {
    /// Its id in the picker and the thread: its name, `<plugin>/<name>` for a plugin's.
    #[must_use]
    pub fn id(&self) -> String {
        match &self.plugin {
            Some(p) => format!("{p}/{}", self.name),
            None => self.name.clone(),
        }
    }

    /// Checks its name, policy and env (a value that looks like a token must be a `secret:` reference); an error
    /// names the key, never the value.
    ///
    /// # Errors
    /// What is wrong, for the owner.
    pub fn check(&self) -> Result<(), String> {
        let ok = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-';
        if self.name.is_empty() || !self.name.bytes().all(ok) {
            return Err(format!(
                "profile {:?}: `name` must be lowercase letters, digits and -",
                self.name
            ));
        }
        if let Some(p) = self
            .policy
            .as_deref()
            .filter(|p| !["ask", "allow", "deny"].contains(p))
        {
            return Err(format!(
                "profile {}: policy {p:?} isn't ask, allow or deny",
                self.name
            ));
        }
        if let Some(a) = self.avatar.as_deref().filter(|a| !avatar(a)) {
            return Err(format!(
                "profile {}: avatar {a:?} isn't an image URL or a sprite id",
                self.name
            ));
        }
        for (k, v) in &self.env {
            let inline = match v.strip_prefix("secret:") {
                Some(name) => !var(name),
                None => crate::redact(v) != *v,
            };
            if !var(k) || inline {
                return Err(format!(
                    "profile {}: env {k}: a variable name, and a secret as `secret:<NAME>`, never inline",
                    self.name
                ));
            }
        }
        Ok(())
    }

    fn rooted(mut self, base: &Path) -> Self {
        let home = env::var_os("HOME").map(PathBuf::from);
        let whole = |p: &mut PathBuf, base: &Path| {
            *p = match (p.strip_prefix("~"), &home) {
                (Ok(rest), Some(home)) => home.join(rest),
                _ => base.join(&*p),
            };
        };
        let paths = [&mut self.instructions, &mut self.settings, &mut self.folder];
        for p in paths.into_iter().flatten() {
            whole(p, base);
        }
        self
    }

    /// Its env split into plain variables and secret ones, each secret read from [`crate::secrets::get`].
    ///
    /// # Errors
    /// Names the secret that is missing or can't be read.
    pub fn env(&self, data: &Path) -> Result<(Vars, Vars), String> {
        let (mut plain, mut secret) = (Vec::new(), Vec::new());
        for (k, v) in &self.env {
            match v.strip_prefix("secret:") {
                Some(name) => {
                    let needs = format!("profile {} needs secret {name}", self.name);
                    let value = crate::secrets::get(data, name)
                        .map_err(|e| format!("{needs}: {e}"))?
                        .ok_or_else(|| format!("{needs}: it isn't saved"))?;
                    secret.push((k.clone(), value));
                }
                None => plain.push((k.clone(), v.clone())),
            }
        }
        Ok((plain, secret))
    }
}

/// Whether `a` is an avatar: an image (`https://…`, `data:image/…`, or a plugin's `/path`) or a sprite id.
#[must_use]
pub fn avatar(a: &str) -> bool {
    let ok = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-';
    a.starts_with("https://")
        || a.starts_with("data:image/")
        || a.starts_with('/')
        || (!a.is_empty() && a.bytes().all(ok))
}

/// An id made from `label`: its letters and digits, lowercased, other runs a `-`.
#[must_use]
pub fn slug(label: &str) -> String {
    let mut name = String::new();
    for c in label.trim().to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            name.push(c);
        } else if !name.is_empty() && !name.ends_with('-') {
            name.push('-');
        }
    }
    name.trim_end_matches('-').to_owned()
}

/// The profiles to pick from: the owner's (`<config>/profiles/*.toml`, by file name), then each enabled installed
/// plugin's: `served(<plugin>)`'s list when its backend serves one (none: its manifest's). One that can't be read or
/// breaks a rule is left out and logged by its file, never its text.
#[must_use]
pub fn list(
    config: Option<&Path>,
    data: &Path,
    served: impl Fn(&str) -> Option<Vec<Profile>>,
) -> Vec<Profile> {
    let mut out = Vec::new();
    let dir = config.map(|c| c.join("profiles"));
    let mut files: Vec<PathBuf> = dir
        .iter()
        .flat_map(fs::read_dir)
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "toml"))
        .collect();
    files.sort();
    for file in files {
        let read = fs::read_to_string(&file).map_err(|e| e.to_string());
        let parsed =
            read.and_then(|t| toml::from_str::<Profile>(&t).map_err(|e| e.message().to_owned()));
        match parsed.and_then(|p| p.check().map(|()| p)) {
            Ok(p) => out.push(p.rooted(file.parent().unwrap_or(&file))),
            Err(e) => eprintln!("cooee: {}: {e}", file.display()),
        }
    }
    for l in plugins::read_lock(data)
        .unwrap_or_default()
        .into_iter()
        .filter(|l| l.enabled)
    {
        let build = plugins::build_dir(data, &l.name, &l.commit);
        let profiles = if let Some(list) = served(&l.name) {
            list
        } else {
            let read = fs::read_to_string(build.join("cooee-plugin.toml"));
            let Ok(m) = read.and_then(|t| Manifest::parse(&t)) else {
                eprintln!("cooee: {}: no readable cooee-plugin.toml", build.display());
                continue;
            };
            m.profiles
        };
        for p in profiles {
            match p.check() {
                Ok(()) => out.push(Profile {
                    plugin: Some(l.name.clone()),
                    ..p.rooted(&build)
                }),
                Err(e) => eprintln!("cooee: plugin {}: {e}", l.name),
            }
        }
    }
    out
}

/// The file of the owner's profile `name` under `<config>/profiles/`.
#[must_use]
pub fn file(config: &Path, name: &str) -> Option<PathBuf> {
    let files = fs::read_dir(config.join("profiles")).ok()?.flatten();
    files.map(|e| e.path()).find(|f| {
        let p = fs::read_to_string(f)
            .ok()
            .and_then(|t| toml::from_str::<Profile>(&t).ok());
        f.extension().is_some_and(|e| e == "toml") && p.is_some_and(|p| p.name == name)
    })
}

/// The profile with id `id` (see [`Profile::id`]), plugins' from their manifests.
///
/// # Errors
/// When there is none.
pub fn find(config: Option<&Path>, data: &Path, id: &str) -> io::Result<Profile> {
    let found = list(config, data, |_| None)
        .into_iter()
        .find(|p| p.id() == id);
    found.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("no profile {id}")))
}

/// `+ new agent` (#86): a profile named `label` saved as `<config>/profiles/<name>.toml`, `name` made from the label;
/// a copy of profile `base` (empty: Plain on ACP agent `agent`). Returns it.
///
/// # Errors
/// A label with no letter or digit, a name taken, an unknown `base`, or a file that can't be written.
pub fn create(
    config: &Path,
    data: &Path,
    label: &str,
    base: &str,
    agent: &str,
) -> io::Result<Profile> {
    let name = slug(label);
    let refused = |why: String| io::Error::new(io::ErrorKind::InvalidInput, why);
    if name.is_empty() {
        return Err(refused(format!("{label:?} needs a letter or digit")));
    }
    let taken = list(Some(config), data, |_| None)
        .into_iter()
        .any(|p| p.id() == name);
    let mut p = match base {
        "" => Profile {
            name: String::new(),
            label: String::new(),
            agent: agent.to_owned(),
            account: None,
            folder: None,
            instructions: None,
            settings: None,
            env: BTreeMap::new(),
            mcp: Vec::new(),
            policy: None,
            hidden: false,
            why: String::new(),
            avatar: None,
            threads: Vec::new(),
            plugin: None,
        },
        id => find(Some(config), data, id)?,
    };
    (p.name, p.label, p.plugin) = (name, label.trim().to_owned(), None);
    p.threads.clear();
    let dir = config.join("profiles");
    let file = dir.join(format!("{}.toml", p.name));
    if taken || file.exists() {
        return Err(refused(format!("there is an agent {} already", p.name)));
    }
    let q = |v: &str| serde_json::to_string(v).unwrap_or_default();
    let paths = [
        ("folder", &p.folder),
        ("instructions", &p.instructions),
        ("settings", &p.settings),
    ];
    let paths = paths.map(|(k, v)| (k, v.as_ref().map(|v| v.to_string_lossy().into_owned())));
    let keys = [
        ("name", Some(p.name.clone())),
        ("label", Some(p.label.clone())),
        ("agent", Some(p.agent.clone())),
        ("account", p.account.clone()),
        ("policy", p.policy.clone()),
    ];
    let mut lines: Vec<String> = keys
        .into_iter()
        .chain(paths)
        .filter_map(|(k, v)| Some(format!("{k} = {}", q(&v?))))
        .collect();
    if !p.env.is_empty() {
        lines.push("\n[env]".to_owned());
        lines.extend(p.env.iter().map(|(k, v)| format!("{} = {}", q(k), q(v))));
    }
    let text = lines.join("\n") + "\n";
    fs::create_dir_all(&dir)?;
    fs::write(&file, text)?;
    if !p.mcp.is_empty() {
        crate::mcp::save_picks(&file, &p.mcp)?;
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
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
        let (plain, secret) = p.env(&data).unwrap();
        assert_eq!(plain, [("A".to_owned(), "x".to_owned())]);
        assert_eq!(secret, [("B".to_owned(), "s3cret".to_owned())]);
        let e = profile("env = { B = \"secret:NONE\" }")
            .env(&data)
            .unwrap_err();
        assert!(e.contains("secret NONE"), "{e}");
        fs::remove_dir_all(&data).unwrap();
    }
}
