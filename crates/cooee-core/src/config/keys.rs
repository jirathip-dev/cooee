use std::{
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
};

use serde::Deserialize;

use super::{Apns, History, Perf, Plugin};

/// cooee's settings, read from `config.toml`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
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
    /// The sign-ins sheet's providers (`[[providers]]`), each row over the provider of its name field by field;
    /// default every registered provider with shed's record of its agent (see [`crate::provider::specs`]).
    pub providers: Option<Vec<toml::Table>>,
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
    /// The skill index `cooee skill add` and the persona panel list (#496); default the built-in one
    /// (`plugins/skills.toml` in cooee's repo).
    pub skill_index: Option<PathBuf>,
    /// The instance's name (`cooee serve --instance`, default `cooee`): its own config, data folder and pairing.
    #[serde(skip)]
    pub instance: String,
    /// Timing (`[perf]`).
    pub perf: Perf,
    /// Log rotation (`[logs]`).
    pub logs: crate::logs::Logs,
    /// The MCP servers profiles pick from (`[[mcp]]`, #56).
    pub mcp: Vec<crate::mcp::Server>,
    /// The Chromium each persona's browser runs (#160; default [`crate::browser::CHROMIUM`]).
    pub chromium: PathBuf,
    /// Apple's `container` CLI the desks run on (#169); unset: no desks are listed.
    pub container: Option<PathBuf>,
    /// Apple push for the native app (`[apns]`, #130); its key is `<data>/apns.p8`.
    pub apns: Option<Apns>,
    /// GitHub webhooks for routines (`[hooks]`, #261); off when unset.
    pub hooks: Option<crate::routines::Hooks>,
    /// Whether `cooee update` (its timer, every 15 minutes) installs main's newest green build (default on, #332).
    pub auto_update: bool,
    /// How many minutes an approval a routine's run raised waits before it counts as Deny (default 30, #505).
    pub approval_expiry_minutes: u64,
    /// How long thread items stay in the hot store in full (`[history]`, #364).
    pub history: History,
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
            providers: None,
            plugins: Vec::new(),
            plugin_index: None,
            skill_index: None,
            theme: None,
            dir: None,
            path: None,
            instance: "cooee".to_owned(),
            perf: Perf::default(),
            logs: crate::logs::Logs::default(),
            mcp: Vec::new(),
            chromium: PathBuf::from(crate::browser::CHROMIUM),
            container: None,
            apns: None,
            hooks: None,
            auto_update: true,
            approval_expiry_minutes: 30,
            history: History::default(),
        }
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
}
