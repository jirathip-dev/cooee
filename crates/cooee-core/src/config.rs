//! The config file: one TOML file, every key optional, unknown keys an error.

use std::{
    env, fs, io,
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::{Spec, provider};

/// cooee's settings, read from `config.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// The port `cooee serve` listens on (default 4747; 0 picks a free one).
    pub port: u16,
    /// The address `cooee serve` listens on (default 127.0.0.1); any other needs `allowed_hosts`.
    pub bind: IpAddr,
    /// The names requests may come to besides localhost and `bind` (the Tailscale name), matched exactly.
    pub allowed_hosts: Vec<String>,
    /// The folders a thread may work in: each root and the folders right inside it, nothing outside them (`~/x`:
    /// under `$HOME`).
    pub roots: Vec<PathBuf>,
    /// The `shed` binary (default `shed`, found on `PATH`).
    pub shed: PathBuf,
    /// shed's `--dir` (its socket and state); default shed's per-user server.
    pub shed_dir: Option<PathBuf>,
    /// Where threads are stored; default `$XDG_DATA_HOME/cooee/<instance>`, else `~/.local/share/cooee/<instance>`.
    pub data: Option<PathBuf>,
    /// The sign-ins sheet's providers (`[[providers]]`); default every registered provider's.
    pub providers: Vec<Spec>,
    /// The theme a device shows until it picks its own (default farm).
    pub theme: Option<String>,
    /// The config file's folder, where the owner's own `themes/` and `schemes/` live (set by [`Config::load`]).
    #[serde(skip)]
    pub dir: Option<PathBuf>,
    /// The config file itself, where roots added on the phone are saved (set by [`Config::load`]).
    #[serde(skip)]
    pub path: Option<PathBuf>,
    /// The plugins: each one a tab beside Chat (`[[plugins]]`).
    pub plugins: Vec<Plugin>,
    /// The plugin index Settings › Plugins and `cooee plugin add <name>` list; default the built-in one
    /// (`plugins/index.toml` in cooee's repo).
    pub plugin_index: Option<PathBuf>,
    /// The instance's name (`cooee serve --instance`, default `cooee`): its own config, data folder and pairing.
    #[serde(skip)]
    pub instance: String,
    /// Timing (`[perf]`).
    pub perf: Perf,
    /// The MCP servers profiles pick from (`[[mcp]]`, #56).
    pub mcp: Vec<crate::mcp::Server>,
    /// Apple push for the native app (`[apns]`, #130); its key is `<data>/apns.p8`.
    pub apns: Option<Apns>,
    /// GitHub webhooks for routines (`[hooks]`, #261); off when unset.
    pub hooks: Option<Hooks>,
}

/// `[hooks]`: a second listener serving only `POST /hooks/github`, published by Tailscale Funnel.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hooks {
    /// Where it listens (`127.0.0.1:4748`).
    pub bind: std::net::SocketAddr,
    /// Its public URL, as the GitHub webhook has it (`https://<mac>.ts.net:8443/hooks/github`).
    pub url: String,
    /// GitHub's API, for the catch-up (default `https://api.github.com`).
    #[serde(default = "github_api")]
    pub api: String,
}

fn github_api() -> String {
    "https://api.github.com".to_owned()
}

/// `[apns]`: the owner's APNs key's ids and the app it pushes to; none of it is secret.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Apns {
    /// The key's id (Apple Developer › Keys).
    pub key_id: String,
    /// The team id.
    pub team_id: String,
    /// The app's bundle id.
    pub topic: String,
    /// `sandbox` (Xcode and test-flight builds) or `production`: a token registered without one is this.
    pub env: ApnsEnv,
    /// The APNs server; default Apple's for each token's `env`.
    pub url: Option<String>,
}

/// Which APNs server a device token belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ApnsEnv {
    /// `api.sandbox.push.apple.com`.
    Sandbox,
    /// `api.push.apple.com`.
    Production,
}

/// `[perf]`: how cooee times itself (#53).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Perf {
    /// A request taking this many milliseconds or more is logged as slow (default 250).
    pub slow_ms: u64,
}

impl Default for Perf {
    fn default() -> Self {
        Self { slow_ms: 250 }
    }
}

/// A plugin: a tab whose page is an ES module served by the plugin's own backend, which cooee reaches with the
/// plugin's token (docs/plugins.md).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Plugin {
    /// Its id in URLs (`/p/<name>`): lowercase letters, digits and `-`.
    pub name: String,
    /// The tab's label.
    pub label: String,
    /// Its backend, `http://host:port` (local or on the tailnet).
    pub url: String,
    /// The path of its UI module on the backend, e.g. `/tab.js`.
    pub module: String,
    /// The secret cooee and the backend share: cooee sends it with every call, and the backend with every push
    /// or choice card it posts. At least 16 characters.
    pub token: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            port: 4747,
            bind: IpAddr::V4(Ipv4Addr::LOCALHOST),
            allowed_hosts: Vec::new(),
            roots: Vec::new(),
            shed: PathBuf::from("shed"),
            shed_dir: None,
            data: None,
            providers: provider::defaults(),
            plugins: Vec::new(),
            plugin_index: None,
            theme: None,
            dir: None,
            path: None,
            instance: "cooee".to_owned(),
            perf: Perf::default(),
            mcp: Vec::new(),
            apns: None,
            hooks: None,
        }
    }
}

/// The fix for an empty `roots`, as the folder picker and `cooee doctor` print it.
pub const ROOTS_FIX: &str = r#"roots = ["~/Projects"]"#;

/// Why the config file couldn't be read.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The file exists but couldn't be read.
    #[error("{path}: {source}")]
    Read {
        /// The config file.
        path: PathBuf,
        /// The underlying error.
        source: io::Error,
    },
    /// An `[[mcp]]` server breaks a rule (a value where only a reference may be): the key, never the value.
    #[error("{path}: {why}")]
    Mcp {
        /// The config file.
        path: PathBuf,
        /// What is wrong.
        why: String,
    },
    /// A key's value breaks a rule (a `base` no provider has).
    #[error("{path}: {why}")]
    Invalid {
        /// The config file.
        path: PathBuf,
        /// What is wrong.
        why: String,
    },
    /// The file isn't valid TOML for [`Config`] (an unknown key lists the known ones).
    #[error("{path}: {source}")]
    Parse {
        /// The config file.
        path: PathBuf,
        /// The underlying error.
        source: toml::de::Error,
    },
}

impl Config {
    /// Instance `instance`'s config: `$XDG_CONFIG_HOME/cooee/<instance>/config.toml`, else under `$HOME/.config`.
    #[must_use]
    pub fn default_path(instance: &str) -> Option<PathBuf> {
        let base = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| Path::new(&home).join(".config")))?;
        Some(base.join("cooee").join(instance).join("config.toml"))
    }

    /// The data folder: `data`, else `$XDG_DATA_HOME/cooee/<instance>`, else `$HOME/.local/share/cooee/<instance>`.
    #[must_use]
    pub fn data_dir(&self) -> Option<PathBuf> {
        self.data.clone().or_else(|| {
            let base = env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| {
                    env::var_os("HOME").map(|home| Path::new(&home).join(".local/share"))
                })?;
            Some(base.join("cooee").join(&self.instance))
        })
    }

    /// Reads `path`; a missing file gives the defaults. Either way [`Config::dir`] is `path`'s folder.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if the file can't be read or parsed.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
            Err(source) => {
                return Err(ConfigError::Read {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        let config: Self = toml::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.to_owned(),
            source,
        })?;
        crate::mcp::check(&config.mcp).map_err(|why| ConfigError::Mcp {
            path: path.to_owned(),
            why,
        })?;
        provider::check(&config.providers).map_err(|why| ConfigError::Invalid {
            path: path.to_owned(),
            why,
        })?;
        let home = env::var_os("HOME").map(PathBuf::from);
        let roots = config
            .roots
            .iter()
            .map(|r| match (r.strip_prefix("~"), &home) {
                (Ok(rest), Some(home)) => home.join(rest),
                _ => r.clone(),
            });
        Ok(Self {
            dir: path.parent().map(Path::to_owned),
            path: Some(path.to_owned()),
            roots: roots.collect(),
            ..config
        })
    }
}

/// Writes `roots` (`~/x` under `$HOME`) into the config file at `path`, replacing only its `roots` key (or adding
/// it at the top), so every other line and comment stays; atomically.
///
/// # Errors
/// When the file can't be read, parsed or written.
pub fn save_roots(path: &Path, roots: &[PathBuf]) -> io::Result<()> {
    let text = match fs::read_to_string(path) {
        Err(e) if e.kind() == io::ErrorKind::NotFound => String::new(),
        text => text?,
    };
    let home = env::var_os("HOME").map(PathBuf::from);
    let roots: Vec<String> = roots
        .iter()
        .map(
            |r| match home.as_deref().and_then(|h| r.strip_prefix(h).ok()) {
                Some(rest) => Path::new("~").join(rest).display().to_string(),
                None => r.display().to_string(),
            },
        )
        .collect();
    // A JSON array of strings is a TOML inline array too.
    let line = format!("roots = {}", serde_json::to_string(&roots)?);
    let table = toml::de::DeTable::parse(&text).map_err(io::Error::other)?;
    let old = table.get_ref().iter().find(|(k, _)| k.get_ref() == "roots");
    let text = match old {
        Some((k, v)) => format!("{}{line}{}", &text[..k.span().start], &text[v.span().end..]),
        None => format!("{line}\n{text}"),
    };
    crate::write_private(path, text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_from_toml_unknown_key_rejected() {
        assert_eq!(
            toml::from_str::<Config>("port = 8080").unwrap(),
            Config {
                port: 8080,
                ..Config::default()
            }
        );
        assert_eq!(toml::from_str::<Config>("").unwrap(), Config::default());
        let e = toml::from_str::<Config>("prot = 8080")
            .unwrap_err()
            .to_string();
        assert!(
            e.contains("unknown field `prot`") && e.contains("port"),
            "{e}"
        );
    }

    #[test]
    fn roots_under_home_are_expanded() {
        let path = env::temp_dir().join(format!("cooee-roots-{}.toml", std::process::id()));
        fs::write(&path, r#"roots = ["~/Projects", "~", "/srv/~x"]"#).unwrap();
        let roots = Config::load(&path).unwrap().roots;
        fs::remove_file(&path).unwrap();
        let home = PathBuf::from(env::var_os("HOME").unwrap());
        assert_eq!(
            roots,
            [home.join("Projects"), home, PathBuf::from("/srv/~x")]
        );
    }

    #[test]
    fn missing_file_gives_defaults() {
        let path = env::temp_dir().join(format!("cooee-no-config-{}.toml", std::process::id()));
        assert_eq!(
            Config::load(&path).unwrap(),
            Config {
                dir: Some(env::temp_dir()),
                path: Some(path.clone()),
                ..Config::default()
            }
        );
    }

    #[test]
    fn saving_roots_keeps_the_other_lines() {
        let path = env::temp_dir().join(format!("cooee-save-{}.toml", std::process::id()));
        let home = PathBuf::from(env::var_os("HOME").unwrap());
        let text = "# mine\nport = 1\nroots = [\n  \"/a\",\n] # old\n\n[[plugins]]\nname = \"x\"\n";
        fs::write(&path, text).unwrap();
        save_roots(&path, &[home.join("P q"), PathBuf::from("/b\"c")]).unwrap();
        let saved = fs::read_to_string(&path).unwrap();
        assert_eq!(
            saved,
            "# mine\nport = 1\nroots = [\"~/P q\",\"/b\\\"c\"] # old\n\n[[plugins]]\nname = \"x\"\n"
        );
        fs::write(&path, "port = 1\n").unwrap();
        save_roots(&path, std::slice::from_ref(&home)).unwrap();
        assert_eq!(Config::load(&path).unwrap().roots, [home]);
        fs::remove_file(&path).unwrap();
    }
}
