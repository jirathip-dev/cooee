//! MCP servers (#56) and skills (#218): the registry, a thread's proxies and their grants, and what agents propose.

use std::{collections::BTreeMap, path::Path};

use cooee_core::{
    Body, Card, Change, Config, Item, Launch, Thread,
    mcp::{self, COOEE, Pick, Server},
    personas,
    shed_proto::ApprovalOption,
    skills::{self, Skill},
};
use serde::Deserialize;
use serde_json::json;

use crate::mcp::Grant;

use super::{Hub, HubError, Recipe, lock, now, refused};

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

/// What an agent proposed, until the owner answers its card.
pub(super) enum Proposed {
    /// A server for the registry (#56).
    Server(Server),
    /// A skill for the thread's persona (#218), new or the edit of `replaces`.
    Skill {
        /// The persona.
        persona: String,
        /// The skill.
        skill: Skill,
        /// The skill it edits.
        replaces: Option<String>,
    },
    /// What else approving does (a routine's save, #261): the card's line, or why not.
    Later(Box<dyn FnOnce() -> Result<String, String> + Send>),
}

/// An agent's `skill` proposal (#218): the skill, and the one it edits.
#[derive(Debug, Deserialize)]
pub struct SkillProposal {
    /// The skill.
    #[serde(flatten)]
    pub skill: Skill,
    /// The name of the skill it edits; none: a new one.
    #[serde(default)]
    pub replaces: Option<String>,
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
            env: BTreeMap::new(),
            headers: BTreeMap::new(),
            enabled: false,
        }
    }
}

/// MCP servers (#56): the registry in the config file, a thread's proxies and their grants, and proposals.
impl Hub {
    /// Agents' MCP proxies reach cooee at `url` (`cooee serve`, once it listens).
    pub fn listening(&self, url: String) {
        let _ = self.url.set(url);
    }

    /// The registry, read from the config file now.
    ///
    /// # Errors
    /// [`HubError::Refused`] when the config can't be read.
    pub fn mcp(&self) -> Result<Vec<Server>, HubError> {
        let Some(c) = &self.config else {
            return Ok(Vec::new());
        };
        Config::load(c)
            .map(|c| c.mcp)
            .map_err(|e| HubError::Refused(e.to_string()))
    }

    /// Changes the registry with `f` (its sentence on success), checks and saves it.
    ///
    /// # Errors
    /// [`HubError::Refused`] with `f`'s or the check's reason, or no config file.
    pub fn edit_mcp(
        &self,
        f: impl FnOnce(&mut Vec<Server>) -> Result<String, String>,
    ) -> Result<String, HubError> {
        let _held = lock(&self.registry);
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

    /// The config's folder (its `profiles/`).
    pub(super) fn config_dir(&self) -> Option<&Path> {
        self.config.as_deref().and_then(Path::parent)
    }

    /// Profile `id`'s picks (none for Plain or a profile gone).
    fn picks(&self, id: &str) -> Vec<Pick> {
        cooee_core::profiles::find(self.config_dir(), &self.data, id)
            .map_or_else(|_| Vec::new(), |p| p.mcp)
    }

    /// `s` resolved, with `tools` allowed.
    fn grant_of(&self, s: Server, tools: Option<Vec<String>>) -> Result<Grant, String> {
        let (env, headers) = s.resolved(&self.data)?;
        Ok(Grant {
            kind: s.kind,
            command: s.command,
            args: s.args,
            url: s.url,
            env,
            headers,
            tools,
        })
    }

    /// `recipe`'s launch for thread `id`, with its MCP servers: cooee's own, then each its profile picks (on, and
    /// resolving now), each a `cooee mcp proxy` holding the thread's token. Before cooee listens: none.
    pub(super) fn mcp_launch(&self, id: &str, recipe: &Recipe) -> Result<Launch, HubError> {
        let mut launch = recipe.launch.clone();
        let (Some(url), Ok(exe)) = (self.url.get(), std::env::current_exe()) else {
            return Ok(launch);
        };
        let servers = self.mcp()?;
        for name in &recipe.mcp {
            let s = servers.iter().find(|s| &s.name == name && s.enabled);
            let s = s.ok_or_else(|| {
                HubError::Refused(format!(
                    "MCP server {name} is off or gone: settings › mcp servers"
                ))
            })?;
            s.resolved(&self.data).map_err(HubError::Refused)?;
        }
        let var = json!([{ "name": crate::mcp::VAR, "value": format!("{url} {}", crate::mcp::token(&self.mcp_key, id)) }]);
        let exe = exe.to_string_lossy();
        let entry = |name: &str, args: &[&str]| json!({ "name": name, "command": exe, "args": args, "env": var });
        let mut all = vec![entry(COOEE, &["mcp", "serve", "--thread", id])];
        all.extend(
            recipe
                .mcp
                .iter()
                .map(|s| entry(s, &["mcp", "proxy", s, "--thread", id])),
        );
        let file = self
            .data
            .join("secrets")
            .join(format!(".mcp-{id}-{:x}.json", now()));
        launch.mcp = Some((file, all));
        Ok(launch)
    }

    /// Thread `id`, if `token` is its MCP token and it is open.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token or a closed thread.
    pub fn token_of(&self, id: &str, token: &str) -> Result<Thread, HubError> {
        let no = || {
            HubError::Refused(format!(
                "this MCP token isn't thread {id}'s, or the thread is closed"
            ))
        };
        if token != crate::mcp::token(&self.mcp_key, id) {
            return Err(no());
        }
        let meta = self.thread(id).map_err(|_| no())?.0;
        if meta.closed {
            return Err(no());
        }
        Ok(meta)
    }

    /// The grant of thread `id`'s proxy for `server` (`token`: the thread's): its pick from its profile, the server
    /// on, resolved now.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, a server it doesn't pick, one off, or a reference
    /// that doesn't resolve.
    pub fn grant(&self, id: &str, token: &str, server: &str) -> Result<Grant, HubError> {
        let meta = self.token_of(id, token)?;
        let pick = self
            .picks(&meta.profile)
            .into_iter()
            .find(|p| p.server == server);
        let pick = pick.ok_or_else(|| {
            HubError::Refused(format!(
                "thread {id}'s profile doesn't pick MCP server {server}"
            ))
        })?;
        let s = self
            .mcp()?
            .into_iter()
            .find(|s| s.name == server && s.enabled);
        let s =
            s.ok_or_else(|| HubError::Refused(format!("MCP server {server} is off or gone")))?;
        self.grant_of(s, pick.tools).map_err(HubError::Refused)
    }

    /// An agent's `remember` in thread `id` (#126, #216): an edit of its persona's notes, for its next sessions; the
    /// answer names the note's id and the characters used.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, one with no persona, or an edit refused.
    pub fn remember(&self, id: &str, token: &str, e: &personas::Edit) -> Result<String, HubError> {
        let meta = self.token_of(id, token)?;
        let found = personas::list(self.config_dir(), &self.data)
            .into_iter()
            .find(|p| p.id == meta.profile);
        let p = found.ok_or_else(|| HubError::Refused(format!("thread {id} has no persona")))?;
        let kept = personas::remember(&self.data, &p.id, "persona", e, now())
            .map_err(|e| HubError::Refused(e.to_string()))?;
        let used = personas::used(&personas::notes(&self.data, &p.id));
        let what = kept.map_or_else(|| "Forgotten".to_owned(), |n| format!("Kept as [{}]", n.id));
        Ok(format!(
            "{what}; {used} of {} characters used.",
            personas::CAP
        ))
    }

    /// An agent's `history` in thread `id` (#217): a search of its persona's threads (`query`, at most `limit`
    /// hits), or a page of one of them (`thread`, before item `before`).
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, one with no persona, or a thread of another persona.
    pub fn history(
        &self,
        (id, token): (&str, &str),
        (query, limit): (&str, usize),
        thread: &str,
        before: Option<u64>,
    ) -> Result<String, HubError> {
        let meta = self.token_of(id, token)?;
        let persona = personas::find(self.config_dir(), &self.data, &meta.profile)
            .map_err(|_| HubError::Refused(format!("thread {id} has no persona")))?;
        let refused = |e: cooee_core::StoreError| HubError::Refused(e.to_string());
        if thread.is_empty() && query.trim().is_empty() {
            return Err(HubError::Refused("give a query or a thread".to_owned()));
        }
        if thread.is_empty() {
            let mine: Vec<String> = self
                .all()
                .iter()
                .map(|l| lock(l).meta.clone())
                .filter(|m| m.profile == persona.id)
                .map(|m| m.id)
                .collect();
            return cooee_core::history::search(&self.store, &mine, query, limit).map_err(refused);
        }
        match self.thread(thread) {
            Ok((t, _)) if t.profile == persona.id => {
                cooee_core::history::read(&self.store, thread, before).map_err(refused)
            }
            _ => Err(HubError::Refused(format!(
                "thread {thread} isn't one of {}'s",
                persona.id
            ))),
        }
    }

    /// An agent's `propose_server` in thread `id`: a card with the exact command or URL; nothing runs, and the
    /// server is added (off) only when the owner approves it with Face ID.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, a server that breaks a rule or a name taken.
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

    /// An agent's `skill` proposal in thread `id` (#218): a card with the skill's file, or an edit's diff; nothing
    /// is saved until the owner approves it.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, one with no persona, or a skill [`skills::check`]
    /// refuses.
    pub fn propose_skill(
        &self,
        (id, token): (&str, &str),
        p: SkillProposal,
    ) -> Result<Item, HubError> {
        let persona = self.persona_of(id, token)?;
        let old = skills::check(&self.data, &persona, &p.skill, p.replaces.as_deref())
            .map_err(|e| HubError::Refused(e.to_string()))?;
        let verb = if p.replaces.is_some() { "edit" } else { "add" };
        let tool = format!(
            "{verb} {persona}'s skill {}: {}",
            p.skill.name, p.skill.description
        );
        let detail = skills::diff(&old, &p.skill.file());
        let pending = Proposed::Skill {
            persona,
            skill: p.skill,
            replaces: p.replaces,
        };
        self.raise(id, (tool, "skill", detail), pending)
    }

    /// An agent's `skill` read in thread `id` (#218): its persona's skill `name`'s body, at most 8,000 characters.
    ///
    /// # Errors
    /// [`HubError::Refused`] for a wrong token, a closed thread, one with no persona, or no such skill of its.
    pub fn read_skill(&self, (id, token): (&str, &str), name: &str) -> Result<String, HubError> {
        let persona = self.persona_of(id, token)?;
        let s = skills::read(&self.data, &persona, name)
            .map_err(|e| HubError::Refused(format!("{e} for {persona}")))?;
        Ok(s.body.chars().take(8000).collect())
    }

    /// The id of thread `id`'s persona, for its token.
    fn persona_of(&self, id: &str, token: &str) -> Result<String, HubError> {
        let meta = self.token_of(id, token)?;
        personas::find(self.config_dir(), &self.data, &meta.profile)
            .map(|p| p.id)
            .map_err(|_| HubError::Refused(format!("thread {id} has no persona")))
    }

    /// A proposal's card in thread `id`: what it does, its form and its detail; `p` waits for the owner's answer.
    fn raise(
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
        lock(&self.proposals).insert((id.to_owned(), item.id), p);
        Ok(item)
    }

    /// A card in thread `id` saying `tool` and `detail`; approving it (with Face ID) runs `then` (#261).
    ///
    /// # Errors
    /// [`HubError::NoThread`] for no such thread.
    pub fn propose_later(
        &self,
        id: &str,
        (tool, detail): (String, String),
        then: Box<dyn FnOnce() -> Result<String, String> + Send>,
    ) -> Result<Item, HubError> {
        self.raise(id, (tool, "routine", detail), Proposed::Later(then))
    }

    /// Whether item `item` of thread `id` is an MCP server's or a routine's proposal card (approving it needs Face
    /// ID).
    #[must_use]
    pub fn is_proposal(&self, id: &str, item: u64) -> bool {
        let Ok(live) = self.live(id) else {
            return false;
        };
        let live = lock(&live);
        live.chat.items.iter().any(|i| {
            i.id == item && matches!(&i.body, Body::Approval { card, .. } if matches!(card.form.as_deref(), Some("mcp" | "routine")))
        })
    }

    /// The owner's answer to a proposal: `approve` adds a server, off, or saves a skill.
    pub(super) fn proposal_answer(
        &self,
        id: &str,
        item: u64,
        option: &str,
    ) -> Result<Option<String>, String> {
        let p = lock(&self.proposals).remove(&(id.to_owned(), item));
        if option != "approve" {
            return Ok(Some("declined".to_owned()));
        }
        match p.ok_or("this proposal ran out (cooee restarted): ask the agent again")? {
            Proposed::Server(s) => self.add_mcp(s).map(Some).map_err(|e| e.to_string()),
            Proposed::Skill {
                persona,
                skill,
                replaces,
            } => skills::save(&self.data, &persona, skill, replaces.as_deref())
                .map(|s| Some(format!("saved skill {}", s.name)))
                .map_err(|e| e.to_string()),
            Proposed::Later(then) => then().map(Some),
        }
    }

    /// Adds `s` to the registry, off (an agent's proposal, or the phone's `+ server`, both after Face ID).
    ///
    /// # Errors
    /// [`HubError::Refused`] for a name taken or a server that breaks a rule.
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
