use std::{path::Path, sync::Mutex};

use cooee_core::{
    Config, Launch, browser,
    mcp::{self, COOEE, Server},
};
use serde::Serialize;
use serde_json::json;

use super::{Hub, HubError, Recipe, lock, now, refused};

#[derive(Default)]
pub(super) struct State {
    registry: Mutex<()>,
}
/// What a checked browser call answers the proxy.
#[derive(Debug, Default, Serialize)]
pub struct Called {
    /// The owner's hand-back note (#162), if they had the browser.
    pub note: Option<String>,
    /// The caption of the replay frame the proxy takes after it (#161), if any.
    pub caption: Option<String>,
}
impl Hub {
    /// Agents' MCP proxies reach cooee at `url` (`cooee serve`, once it listens).
    pub fn listening(&self, url: String) {
        self.url.set(url).ok();
    }

    /// The registry, read from the config file now.
    #[expect(clippy::missing_errors_doc, reason = "#738: moved docs")]
    pub fn mcp(&self) -> Result<Vec<Server>, HubError> {
        let Some(c) = &self.config else {
            return Ok(Vec::new());
        };
        Config::load(c)
            .map(|c| c.mcp)
            .map_err(|e| HubError::Refused(e.to_string()))
    }

    /// Changes the registry with `f` (its sentence on success), checks and saves it.
    #[expect(clippy::missing_errors_doc, reason = "#738: moved docs")]
    pub fn edit_mcp(
        &self,
        f: impl FnOnce(&mut Vec<Server>) -> Result<String, String>,
    ) -> Result<String, HubError> {
        let _held = lock(&self.areas.servers.registry);
        let Some(c) = &self.config else {
            return Err(HubError::Refused(
                "there is no config file to save MCP servers in".to_owned(),
            ));
        };
        let mut list = self.mcp()?;
        let said = f(&mut list).map_err(HubError::Refused)?;
        mcp::check(&list).map_err(HubError::Refused)?;
        mcp::save(c, &list).map_err(|e| refused(c, &e))?;
        Ok(said)
    }

    pub(super) fn config_dir(&self) -> Option<&Path> {
        self.config.as_deref().and_then(Path::parent)
    }

    pub(super) fn servers(&self) -> Result<Vec<Server>, HubError> {
        let mut list = self.mcp()?;
        if !list.iter().any(|s| s.name == browser::SERVER) {
            list.push(browser::server());
        }
        Ok(list)
    }

    pub(super) fn chromium(&self) -> Result<std::path::PathBuf, HubError> {
        let Some(c) = &self.config else {
            return Ok(browser::CHROMIUM.into());
        };
        Config::load(c)
            .map(|c| c.chromium)
            .map_err(|e| HubError::Refused(e.to_string()))
    }

    pub(super) fn mcp_launch(&self, id: &str, recipe: &Recipe) -> Result<Launch, HubError> {
        let mut launch = recipe.launch.clone();
        let (Some(url), Ok(exe)) = (self.url.get(), std::env::current_exe()) else {
            return Ok(launch);
        };
        let servers = self.servers()?;
        let picked: Vec<&Server> = servers
            .iter()
            .filter(|s| recipe.mcp.contains(&s.name))
            .collect();
        for s in &picked {
            if !s.enabled {
                return Err(HubError::Refused(format!(
                    "MCP server {} is off: settings › mcp servers",
                    s.name
                )));
            }
            s.resolved(&self.data).map_err(HubError::Refused)?;
        }
        let var = json!([{ "name": crate::mcp::VAR, "value": format!("{url} {}", crate::mcp::token(&self.areas.grants.mcp_key, id)) }]);
        let exe = exe.to_string_lossy();
        let entry = |name: &str, args: &[&str]| json!({ "name": name, "command": exe, "args": args, "env": var });
        let mut all = vec![entry(COOEE, &["mcp", "serve", "--thread", id])];
        all.extend(
            picked
                .iter()
                .map(|s| entry(&s.name, &["mcp", "proxy", &s.name, "--thread", id])),
        );
        let file = self
            .data
            .join("secrets")
            .join(format!(".mcp-{id}-{:x}.json", now()));
        launch.mcp = Some((file, all));
        Ok(launch)
    }

    pub(super) fn unconnected(&self, recipe: &Recipe) -> Result<String, HubError> {
        let servers = self.servers()?;
        let missing: Vec<&str> = recipe
            .mcp
            .iter()
            .filter(|n| !servers.iter().any(|s| &s.name == *n))
            .map(String::as_str)
            .collect();
        Ok(if missing.is_empty() {
            String::new()
        } else {
            format!(
                "These MCP servers you expect aren't connected yet: {}. Before anything else, your first reply tells \
                 the owner to connect each one by that name in Settings › MCP servers (or `cooee mcp add`); until \
                 then you can't use it.",
                missing.join(", ")
            )
        })
    }

    /// Adds `s` to the registry, off (an agent's proposal, or the phone's `+ server`, both after Face ID).
    #[expect(clippy::missing_errors_doc, reason = "#738: moved docs")]
    pub fn add_mcp(&self, s: Server) -> Result<String, HubError> {
        self.edit_mcp(|list| {
            if list.iter().any(|o| o.name == s.name) {
                return Err(format!(
                    "there is an MCP server {} already: change it on the Mac",
                    s.name
                ));
            }
            let said = format!(
                "added {}, off: turn it on in settings › mcp servers",
                s.name
            );
            list.push(Server {
                enabled: false,
                ..s
            });
            Ok(said)
        })
    }
}
