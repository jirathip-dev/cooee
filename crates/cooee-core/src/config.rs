//! The config file: one TOML file, every key optional, unknown keys an error.

use std::{
    env, fs, io,
    net::{IpAddr, Ipv4Addr},
    path::{Path, PathBuf},
};

use serde::Deserialize;

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
    /// The folders a thread may work in: each root and the folders right inside it, nothing outside them.
    pub roots: Vec<PathBuf>,
    /// The `shed` binary (default `shed`, found on `PATH`).
    pub shed: PathBuf,
    /// shed's `--dir` (its socket and state); default shed's per-user server.
    pub shed_dir: Option<PathBuf>,
    /// Where threads are stored; default `$XDG_DATA_HOME/cooee`, else `~/.local/share/cooee`.
    pub data: Option<PathBuf>,
    /// The plugins: each one a tab beside Chat (`[[plugins]]`).
    pub plugins: Vec<Plugin>,
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
            plugins: Vec::new(),
        }
    }
}

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
    /// `$XDG_CONFIG_HOME/cooee/config.toml`, else `$HOME/.config/cooee/config.toml`.
    #[must_use]
    pub fn default_path() -> Option<PathBuf> {
        let base = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|home| Path::new(&home).join(".config")))?;
        Some(base.join("cooee").join("config.toml"))
    }

    /// The data folder: `data`, else `$XDG_DATA_HOME/cooee`, else `$HOME/.local/share/cooee`.
    #[must_use]
    pub fn data_dir(&self) -> Option<PathBuf> {
        self.data.clone().or_else(|| {
            let base = env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| {
                    env::var_os("HOME").map(|home| Path::new(&home).join(".local/share"))
                })?;
            Some(base.join("cooee"))
        })
    }

    /// Reads `path`; a missing file gives the defaults.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if the file can't be read or parsed.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(source) => {
                return Err(ConfigError::Read {
                    path: path.to_owned(),
                    source,
                });
            }
        };
        toml::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.to_owned(),
            source,
        })
    }
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
    fn missing_file_gives_defaults() {
        let path = env::temp_dir().join(format!("cooee-no-config-{}.toml", std::process::id()));
        assert_eq!(Config::load(&path).unwrap(), Config::default());
    }
}
