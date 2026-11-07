//! Agent profiles: a named recipe for a thread's agent (its agent, account, folder, instructions, settings, env, MCP
//! servers and shed's approval policy), picked when the thread starts and kept for its life. Plain (no profile) is
//! the agent with the owner's own setup. The owner's are `<config>/profiles/*.toml`; a plugin brings its own as
//! `[[profiles]]` in its `cooee-plugin.toml`, its paths in its build. A secret never sits in a profile: an `env`
//! value is plain text or `secret:<NAME>`, the file `<data>/secrets/<NAME>`.

use std::{
    collections::BTreeMap,
    env, fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::plugins::{self, Manifest};

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
    /// Variables for the agent: plain text, or `secret:<NAME>`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env: BTreeMap<String, String>,
    /// An MCP servers file; shed starts ACP sessions without MCP servers, so a profile with one is refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp: Option<PathBuf>,
    /// shed's approval policy: `ask` (shed's default), `allow` or `deny`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>,
    /// The plugin it came with (none: the owner's own).
    #[serde(skip)]
    pub plugin: Option<String>,
}

/// A variable's name: `A-Z a-z 0-9 _`, not starting with a digit.
fn var(name: &str) -> bool {
    name.bytes().next().is_some_and(|b| !b.is_ascii_digit())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
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

    /// Its paths made whole: `~/x` under `$HOME`, others under `base`.
    fn rooted(mut self, base: &Path) -> Self {
        let home = env::var_os("HOME").map(PathBuf::from);
        let whole = |p: &mut PathBuf, base: &Path| {
            *p = match (p.strip_prefix("~"), &home) {
                (Ok(rest), Some(home)) => home.join(rest),
                _ => base.join(&*p),
            };
        };
        let paths = [
            &mut self.instructions,
            &mut self.settings,
            &mut self.mcp,
            &mut self.folder,
        ];
        for p in paths.into_iter().flatten() {
            whole(p, base);
        }
        self
    }

    /// Its env split into plain variables and secret ones, each secret read from `<data>/secrets/<NAME>` (its last
    /// newline dropped).
    ///
    /// # Errors
    /// Names the secret file that can't be read.
    pub fn env(&self, data: &Path) -> Result<(Vars, Vars), String> {
        let (mut plain, mut secret) = (Vec::new(), Vec::new());
        for (k, v) in &self.env {
            match v.strip_prefix("secret:") {
                Some(name) => {
                    let path = data.join("secrets").join(name);
                    let value = fs::read_to_string(&path).map_err(|e| {
                        format!(
                            "profile {} needs secret {name}: {}: {e}",
                            self.name,
                            path.display()
                        )
                    })?;
                    let value = value.strip_suffix('\n').unwrap_or(&value);
                    secret.push((k.clone(), value.to_owned()));
                }
                None => plain.push((k.clone(), v.clone())),
            }
        }
        Ok((plain, secret))
    }
}

/// The profiles to pick from: the owner's (`<config>/profiles/*.toml`, by file name), then each enabled installed
/// plugin's. One that can't be read or breaks a rule is left out and logged by its file, never its text.
#[must_use]
pub fn list(config: Option<&Path>, data: &Path) -> Vec<Profile> {
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
        let read = fs::read_to_string(build.join("cooee-plugin.toml"));
        let Ok(m) = read.and_then(|t| Manifest::parse(&t)) else {
            eprintln!("cooee: {}: no readable cooee-plugin.toml", build.display());
            continue;
        };
        out.extend(m.profiles.into_iter().map(|p| Profile {
            plugin: Some(l.name.clone()),
            ..p.rooted(&build)
        }));
    }
    out
}

/// The profile with id `id` (see [`Profile::id`]).
///
/// # Errors
/// When there is none.
pub fn find(config: Option<&Path>, data: &Path, id: &str) -> io::Result<Profile> {
    let found = list(config, data).into_iter().find(|p| p.id() == id);
    found.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("no profile {id}")))
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
