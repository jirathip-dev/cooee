use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use super::Config;
use crate::provider;

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
        provider::specs(config.providers.as_deref(), &shed_catalog::Catalog::default()).map_err(|why| ConfigError::Invalid {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roots_under_home_are_expanded() {
        let path = crate::scratch::owned(
            env::temp_dir().join(format!("cooee-roots-{}.toml", std::process::id())),
        );
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
        let path = crate::scratch::owned(
            env::temp_dir().join(format!("cooee-no-config-{}.toml", std::process::id())),
        );
        assert_eq!(
            Config::load(&path).unwrap(),
            Config {
                dir: Some(env::temp_dir()),
                path: Some(path.clone()),
                ..Config::default()
            }
        );
    }
}
