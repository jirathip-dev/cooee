use std::{fs, path::Path};

use cooee_core::{
    Thread,
    agents::{self, Agent},
    browser,
    mcp::{self, Server},
};
use serde_json::{Value, json};

use crate::mcp::{Child, Grant};

use super::{Called, Hub, HubError, Seed, now, refused};

fn mcp_key(key: &Path) -> Result<String, HubError> {
    if let Ok(k) = fs::read_to_string(key) {
        return Ok(k);
    }
    let k = cooee_core::random(32).map(|r| cooee_core::sha256(&r));
    let k = k.and_then(|k| cooee_core::write_private(key, k.as_bytes()).map(|()| k));
    k.map_err(|e| refused(key, &e))
}

pub(super) struct State {
    pub(super) mcp_key: String,
}

impl State {
    pub(super) fn open(seed: &Seed<'_>) -> Result<Self, HubError> {
        Ok(Self {
            mcp_key: mcp_key(&seed.data.join("mcp.key"))?,
        })
    }
}

impl Hub {
    pub(crate) fn persona(&self, id: &str) -> Agent {
        let list = agents::all(self.config_dir(), &self.data, |_| None);
        list.into_iter().find(|a| a.id == id).unwrap_or_default()
    }

    fn grant_of(&self, s: Server, tools: Option<Vec<String>>) -> Result<Grant, String> {
        let (env, _) = s.resolved(&self.data)?;
        if !matches!(s.source()?, mcp::Source::Command { .. }) {
            return Ok(Grant::Pipe);
        }
        Ok(Grant::Run(Child {
            command: s.command,
            args: s.args,
            env,
            tools,
        }))
    }

    /// Thread `id`, if `token` is its MCP token and it is open.
    #[expect(clippy::missing_errors_doc, reason = "#738: moved docs")]
    pub fn token_of(&self, id: &str, token: &str) -> Result<Thread, HubError> {
        let no = || {
            HubError::Refused(format!(
                "this MCP token isn't thread {id}'s, or the thread is closed"
            ))
        };
        if token != crate::mcp::token(&self.areas.grants.mcp_key, id) {
            return Err(no());
        }
        let meta = self.thread(id).map_err(|_| no())?.0;
        if meta.closed {
            return Err(no());
        }
        Ok(meta)
    }

    /// The grant of thread `id`'s proxy for `server` (`token`: the thread's): its pick from its profile, the server on, resolved now.
    #[expect(clippy::missing_errors_doc, reason = "#738: moved docs")]
    pub fn grant(&self, id: &str, token: &str, server: &str) -> Result<Grant, HubError> {
        let (persona, pick, s) = self.granted(id, token, server)?;
        let mut g = self.grant_of(s, pick.tools).map_err(HubError::Refused)?;
        if let (true, Grant::Run(c)) = (server == browser::SERVER, &mut g) {
            let profile = browser::profile(&self.data, &persona.id);
            let cdp = browser::endpoint(&self.chromium()?, &profile)
                .map_err(|e| HubError::Refused(format!("{}'s browser: {e}", persona.id)))?;
            c.args.extend(["--cdp-endpoint".to_owned(), cdp]);
        }
        Ok(g)
    }

    pub(crate) fn granted(
        &self,
        id: &str,
        token: &str,
        server: &str,
    ) -> Result<(Agent, mcp::Pick, Server), HubError> {
        let meta = self.token_of(id, token)?;
        let persona = self.persona(&meta.profile);
        let pick = persona.mcp.iter().find(|p| p.server == server).cloned();
        let pick = pick.ok_or_else(|| {
            HubError::Refused(format!(
                "thread {id}'s profile doesn't pick MCP server {server}"
            ))
        })?;
        let s = self
            .servers()?
            .into_iter()
            .find(|s| s.name == server && s.enabled);
        let s =
            s.ok_or_else(|| HubError::Refused(format!("MCP server {server} is off or gone")))?;
        Ok((persona, pick, s))
    }

    /// Thread `id`'s proxy's `tools/call` of `tool` on `server` with `args`, checked again: the grant still holds, the tool is allowed and, on the browser, a `url` is on the persona's site allowlist; each browser call waits while the owner holds control and is logged (password typing masked).
    #[expect(clippy::missing_errors_doc, reason = "#738: moved docs")]
    pub fn call(
        &self,
        (id, token): (&str, &str),
        (server, tool): (&str, &str),
        args: &Value,
    ) -> Result<Called, HubError> {
        let (persona, pick, _) = self.granted(id, token, server)?;
        if !pick.allows(tool) {
            return Err(HubError::Refused(format!(
                "tool {tool} isn't allowed in this thread (its profile's tools for {server})"
            )));
        }
        let folder = self.token_of(id, token)?.folder.unwrap_or_default();
        self.denied_tool(&persona, &folder, (server, tool))?;
        if server != browser::SERVER {
            return Ok(Called::default());
        }
        let note = self.wait_turn(&persona.id);
        let url = args.get("url").and_then(Value::as_str);
        let off = url.filter(|u| !browser::allows(&persona.sites, u));
        let args = browser::masked(args);
        let line = json!({ "at": now(), "thread": id, "tool": tool, "arguments": args, "refused": off.is_some() });
        let profile = browser::profile(&self.data, &persona.id);
        browser::log(&profile, &line).map_err(|e| refused(&profile, &e))?;
        match off {
            Some(u) => Err(HubError::Refused(format!(
                "{u} isn't on {}'s site allowlist",
                persona.id
            ))),
            None => Ok(Called {
                note,
                caption: browser::caption(tool, &args),
            }),
        }
    }
}
