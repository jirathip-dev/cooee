use std::{collections::HashMap, io, sync::Mutex};

use cooee_core::{
    Body, Card, Change, Item, agents,
    mcp::Server,
    shed_proto::ApprovalOption,
    skills::{self, Skill},
};
use serde::Deserialize;

use super::{Hub, HubError, lock, now};

/// An agent's `propose_server` (#56): a server for the registry, and why.
#[derive(Debug, Deserialize)]
pub struct Proposal {
    /// Its name.
    pub name: String,
    /// `stdio`, `http` or `sse`.
    #[serde(rename = "type", default)]
    pub kind: String,
    /// A stdio server's program.
    #[serde(default)]
    pub command: String,
    /// Its arguments.
    #[serde(default)]
    pub args: Vec<String>,
    /// An http or sse server's URL.
    #[serde(default)]
    pub url: String,
    /// What the owner gets from it.
    #[serde(default)]
    pub why: String,
}

pub(super) enum Proposed {
    Server(Server),
    Skill {
        persona: String,
        skill: Skill,
        replaces: Option<String>,
    },
    Later(Box<dyn FnOnce() -> Result<String, String> + Send>),
    Secret(super::secret::Asked),
    Change(Box<cooee_core::chief::Carded>),
}

#[derive(Default)]
pub(super) struct State {
    proposals: Mutex<HashMap<(String, u64), Proposed>>,
}

impl Proposal {
    /// The server it would add: switched off, with no variables or headers (the Mac adds those).
    #[must_use]
    pub fn server(self) -> Server {
        Server {
            name: self.name,
            kind: self.kind,
            command: self.command,
            args: self.args,
            url: self.url,
            enabled: false,
            ..Server::default()
        }
    }
}

impl Hub {
    /// An agent's `propose_server` in thread `id`: a card with the exact command or URL; nothing runs, and the server is added (off) only when the owner approves it with Face ID.
    #[expect(clippy::missing_errors_doc, reason = "#738: moved docs")]
    pub fn propose(&self, id: &str, token: &str, p: Proposal) -> Result<Item, HubError> {
        self.token_of(id, token)?;
        let why: String = p.why.trim().chars().take(300).collect();
        let s = p.server();
        s.check().map_err(HubError::Refused)?;
        if self.mcp()?.iter().any(|o| o.name == s.name) {
            return Err(HubError::Refused(format!(
                "there is an MCP server {} already",
                s.name
            )));
        }
        let tool = format!("add MCP server {} ({}): {why}", s.name, s.kind);
        self.raise(id, (tool, "mcp", s.shown()), Proposed::Server(s))
    }

    pub(super) fn raise(
        &self,
        id: &str,
        (tool, form, detail): (String, &str, String),
        p: Proposed,
    ) -> Result<Item, HubError> {
        let option = |o: &str| ApprovalOption {
            id: o.to_owned(),
            name: o.to_owned(),
            kind: String::new(),
        };
        let body = Body::Approval {
            tool,
            options: vec![option("approve"), option("decline")],
            tapped: None,
            chosen: None,
            plugin: None,
            card: Card {
                form: Some(form.to_owned()),
                detail: Some(detail),
                ..Card::default()
            },
        };
        let live = self.live(id)?;
        let mut live = lock(&live);
        let item = live.chat.push(now(), 0, body);
        live.publish(self, vec![Change::Item(item.clone())])?;
        lock(&self.areas.proposals.proposals).insert((id.to_owned(), item.id), p);
        Ok(item)
    }

    /// A card in thread `id` saying `tool` and `detail`; approving it (with Face ID) runs `then` (#261).
    #[expect(clippy::missing_errors_doc, reason = "#738: moved docs")]
    pub fn propose_later(
        &self,
        id: &str,
        (tool, detail): (String, String),
        then: Box<dyn FnOnce() -> Result<String, String> + Send>,
    ) -> Result<Item, HubError> {
        self.raise(id, (tool, "routine", detail), Proposed::Later(then))
    }

    pub(crate) fn ask_card(
        &self,
        id: &str,
        (what, detail): (String, String),
        allowed: Box<dyn FnOnce() -> Result<String, String> + Send>,
    ) -> Result<Item, HubError> {
        self.raise(id, (what, "tool", detail), Proposed::Later(allowed))
    }

    /// Whether item `item` of thread `id` is an MCP server's, a routine's or a secret's card of cooee's (approving it needs Face ID).
    #[must_use]
    pub fn is_proposal(&self, id: &str, item: u64) -> bool {
        let Ok(live) = self.live(id) else {
            return false;
        };
        let live = lock(&live);
        live.chat
            .items
            .iter()
            .any(|i| i.id == item && i.needs_key())
    }

    pub(super) fn proposal_answer(
        &self,
        id: &str,
        item: u64,
        option: &str,
        value: Option<&str>,
    ) -> Result<Option<String>, String> {
        let p = lock(&self.areas.proposals.proposals).remove(&(id.to_owned(), item));
        if let Some(Proposed::Change(c)) = &p {
            return self
                .change_answer((id, item), c, option == "approve", value)
                .map(Some);
        }
        if option != "approve" {
            return Ok(Some("declined".to_owned()));
        }
        match p.ok_or("this proposal ran out (cooee restarted): ask the agent again")? {
            Proposed::Server(s) => self.add_mcp(s).map(Some).map_err(|e| e.to_string()),
            Proposed::Skill {
                persona,
                skill,
                replaces,
            } => self
                .config_dir()
                .ok_or_else(|| io::Error::other("no config folder"))
                .and_then(|c| agents::home(c, &persona))
                .and_then(|h| skills::save(&h, skill, replaces.as_deref()))
                .map(|s| Some(format!("saved skill {}", s.name)))
                .map_err(|e| e.to_string()),
            Proposed::Later(then) => then().map(Some),
            Proposed::Secret(a) => self.drop_secret((id, item), &a, value).map(Some),
            Proposed::Change(_) => Ok(None),
        }
    }
}
